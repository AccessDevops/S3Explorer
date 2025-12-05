use crate::cache_manager::CacheStatus;
use crate::database::{
    clear_all_db_managers, close_db_manager, get_db_cache_status, warmup_db_manager,
    DatabaseManager,
};
use crate::index_manager::{
    clear_all_index_managers, close_index_manager, get_index_cache_status, get_index_manager,
    warmup_index_manager,
};
use crate::metrics::MetricsContext;
use crate::metrics_storage;
use crate::models::*;
use crate::profiles::ProfileStore;
use crate::s3_adapter::S3Adapter;
use crate::validation;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tokio::sync::broadcast;
use tokio::task::{spawn_blocking, JoinHandle};

// Upload task handle with metadata for cancellation
pub struct UploadTask {
    handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    file_name: String,
    file_size: u64,
}

// Download task handle with metadata for cancellation
pub struct DownloadTask {
    handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    file_name: String,
    file_size: u64,
}

// Indexing task handle with metadata for cancellation
pub struct IndexTask {
    #[allow(dead_code)]
    handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    #[allow(dead_code)]
    bucket_name: String,
}

// Global profile store state
pub struct AppState {
    pub profiles: Mutex<ProfileStore>,
    pub active_uploads: Arc<Mutex<HashMap<String, UploadTask>>>,
    pub active_downloads: Arc<Mutex<HashMap<String, DownloadTask>>>,
    pub active_indexing: Arc<Mutex<HashMap<String, IndexTask>>>,
}

impl AppState {
    pub fn new() -> Self {
        let profiles = ProfileStore::load().unwrap_or_default();
        Self {
            profiles: Mutex::new(profiles),
            active_uploads: Arc::new(Mutex::new(HashMap::new())),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            active_indexing: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// List all saved profiles (without sensitive data)
#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String> {
    let store = state.profiles.lock().map_err(|e| e.to_string())?;
    Ok(store.list())
}

/// Save a new profile or update an existing one
#[tauri::command]
pub async fn save_profile(profile: Profile, state: State<'_, AppState>) -> Result<(), String> {
    let mut store = state.profiles.lock().map_err(|e| e.to_string())?;
    store.upsert(profile).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a profile by ID
#[tauri::command]
pub async fn delete_profile(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut store = state.profiles.lock().map_err(|e| e.to_string())?;
    store.delete(&profile_id).map_err(|e| e.to_string())?;
    Ok(())
}

/// Test a connection profile
#[tauri::command]
pub async fn test_connection(
    app: AppHandle,
    profile: Profile,
) -> Result<TestConnectionResponse, String> {
    let mut ctx = MetricsContext::new(S3Operation::ListBuckets, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name);

    let result = S3Adapter::test_connection_with_profile(&profile).await;

    match &result {
        Ok(response) => {
            if response.success {
                if let Some(count) = response.bucket_count {
                    ctx.set_objects_affected(count as u32);
                }
                ctx.emit_success(&app);
            } else {
                ctx.emit_error(&app, &response.message);
            }
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map_err(|e| e.to_string())
}

/// List buckets for a profile
#[tauri::command]
pub async fn list_buckets(
    app: AppHandle,
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Bucket>, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let mut ctx = MetricsContext::new(S3Operation::ListBuckets, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.list_buckets().await;

    match &result {
        Ok(buckets) => {
            ctx.set_objects_affected(buckets.len() as u32);
            ctx.emit_success(&app);
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map_err(|e| e.to_string())
}

/// Create a new bucket
#[tauri::command]
pub async fn create_bucket(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::validate_bucket_name(&bucket_name).map_err(|e| e.to_string())?;

    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::CreateBucket, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket_name);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.create_bucket(&bucket_name).await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Delete a bucket (must be empty)
#[tauri::command]
pub async fn delete_bucket(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::DeleteBucket, RequestCategory::DELETE)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket_name);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.delete_bucket(&bucket_name).await;
    ctx.emit_result(&app, &result);

    // If deletion successful, also clear the bucket's index
    if result.is_ok() {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            let _ = index_mgr.clear_bucket_index(&bucket_name);
        }
    }

    result.map_err(|e| e.to_string())
}

/// Check if user has permission to delete a bucket
#[tauri::command]
pub async fn can_delete_bucket(
    _app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    Ok(adapter.can_delete_bucket(&bucket_name).await)
}

/// Get bucket ACL (Public/Private)
#[tauri::command]
pub async fn get_bucket_acl(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::GetBucketAcl, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket_name);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.get_bucket_acl(&bucket_name).await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Get complete bucket configuration (policy, ACL, CORS, lifecycle, versioning, encryption)
/// All calls are made in parallel for performance
#[tauri::command]
pub async fn get_bucket_configuration(
    _app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<BucketConfigurationResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    Ok(adapter.get_bucket_configuration(&bucket_name).await)
}

/// Calculate bucket statistics (size and count of all objects)
/// Returns (size, count, is_incomplete) where is_incomplete=true if index is not fully complete
/// Note: Uses ONLY the local SQLite index - no S3 API calls
/// For non-indexed buckets, returns an error (UI should display "-")
#[tauri::command]
pub async fn calculate_bucket_stats(
    _app: AppHandle,
    profile_id: String,
    bucket_name: String,
    _force_refresh: Option<bool>,
    _state: State<'_, AppState>,
) -> Result<(i64, i64, bool), String> {
    // Use ONLY the SQLite index - no S3 fallback
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;

    match index_mgr.get_bucket_stats(&bucket_name) {
        Ok(stats) => {
            // Emit cache hit - estimate saved requests based on object count
            // Each ListObjectsV2 returns max 1000 objects
            let saved_requests = ((stats.total_objects as f64 / 1000.0).ceil() as i32).max(1);
            metrics_storage::emit_cache_hit(
                "BucketStats",
                Some(&profile_id),
                Some(&bucket_name),
                saved_requests,
            );

            // Return stats from index
            // is_incomplete = !is_complete (true if index is partial)
            Ok((stats.total_size, stats.total_objects, !stats.is_complete))
        }
        Err(_) => {
            // Bucket not indexed - return error so UI can display "-"
            Err("Bucket not indexed".to_string())
        }
    }
}

/// List objects in a bucket
///
/// Parameters:
/// - sync_index: If true and this is the first page (no continuation_token),
///   also removes from the index any objects that no longer exist on S3.
///   This cleans up "phantom objects" that were deleted by another client.
#[tauri::command]
pub async fn list_objects(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    prefix: Option<String>,
    continuation_token: Option<String>,
    max_keys: Option<i32>,
    use_delimiter: Option<bool>,
    sync_index: Option<bool>,
    state: State<'_, AppState>,
) -> Result<ListObjectsResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let mut ctx = MetricsContext::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .list_objects(
            &bucket,
            prefix.as_deref(),
            continuation_token.clone(),
            max_keys,
            use_delimiter.unwrap_or(true),
        )
        .await;

    match &result {
        Ok(response) => {
            ctx.set_objects_affected(response.objects.len() as u32);
            ctx.emit_success(&app);

            // Update index with list response (non-blocking, ignore errors)
            if let Ok(index_mgr) = get_index_manager(&profile_id) {
                let prefix_str = prefix.as_deref().unwrap_or("");
                let _ = index_mgr.update_from_list_response(&bucket, prefix_str, response);

                // If sync_index is enabled and this is the first page (complete view of prefix),
                // remove any objects from the index that are no longer on S3
                if sync_index.unwrap_or(false)
                    && continuation_token.is_none()
                    && !response.is_truncated
                {
                    // We have all objects for this prefix - sync the index
                    let current_keys: Vec<String> =
                        response.objects.iter().map(|o| o.key.clone()).collect();
                    if let Err(e) =
                        index_mgr.sync_prefix_objects(&bucket, prefix_str, &current_keys)
                    {
                        eprintln!(
                            "Warning: Failed to sync index for prefix '{}': {}",
                            prefix_str, e
                        );
                    }
                }
            }
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map_err(|e| e.to_string())
}

/// Get an object's content
#[tauri::command]
pub async fn get_object(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<GetObjectResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let mut ctx = MetricsContext::new(S3Operation::GetObject, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.get_object(&bucket, &key).await;

    match &result {
        Ok(response) => {
            ctx.set_bytes(response.size as u64);
            ctx.emit_success(&app);
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map_err(|e| e.to_string())
}

/// Upload an object
#[tauri::command]
pub async fn put_object(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    content: Vec<u8>,
    content_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::validate_bucket_name(&bucket).map_err(|e| e.to_string())?;
    validation::validate_object_key(&key).map_err(|e| e.to_string())?;

    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let content_len = content.len() as u64;
    let mut ctx = MetricsContext::new(S3Operation::PutObject, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);
    ctx.set_bytes(content_len);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .put_object(&bucket, &key, content, content_type.clone())
        .await;
    ctx.emit_result(&app, &result);

    // Add object to index after successful upload
    if result.is_ok() {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            let obj = S3Object {
                key: key.clone(),
                size: content_len as i64,
                last_modified: Some(chrono::Utc::now().to_rfc3339()),
                storage_class: Some("STANDARD".to_string()),
                e_tag: None,
                is_folder: false,
            };
            let _ = index_mgr.add_object(&bucket, &obj);
        }
    }

    result.map_err(|e| e.to_string())
}

/// Delete an object
#[tauri::command]
pub async fn delete_object(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::DeleteObject, RequestCategory::DELETE)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.delete_object(&bucket, &key).await;
    ctx.emit_result(&app, &result);

    // Remove object from index after successful deletion
    if result.is_ok() {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            let _ = index_mgr.remove_object(&bucket, &key);
        }
    }

    result.map_err(|e| e.to_string())
}

/// Delete a specific version of an object
/// This is a PERMANENT deletion - the version cannot be recovered
#[tauri::command]
pub async fn delete_object_version(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::DeleteObject, RequestCategory::DELETE)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .delete_object_version(&bucket, &key, &version_id)
        .await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Copy an object
#[tauri::command]
pub async fn copy_object(
    app: AppHandle,
    profile_id: String,
    source_bucket: String,
    source_key: String,
    dest_bucket: String,
    dest_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::CopyObject, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&dest_bucket)
        .with_object_key(&dest_key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .copy_object(&source_bucket, &source_key, &dest_bucket, &dest_key)
        .await;
    ctx.emit_result(&app, &result);

    // Add new object to index after successful copy
    if result.is_ok() {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            // Try to get source object info from index, use defaults if not found
            let (size, storage_class) = index_mgr
                .db
                .get_object(&source_bucket, &source_key)
                .ok()
                .flatten()
                .map(|o| (o.size, o.storage_class))
                .unwrap_or((0, "STANDARD".to_string()));

            let new_obj = S3Object {
                key: dest_key.clone(),
                size,
                last_modified: Some(chrono::Utc::now().to_rfc3339()),
                storage_class: Some(storage_class),
                e_tag: None,
                is_folder: dest_key.ends_with('/'),
            };
            let _ = index_mgr.add_object(&dest_bucket, &new_obj);
        }
    }

    result.map_err(|e| e.to_string())
}

/// Change the content-type of an object (copies object to itself with new metadata)
#[tauri::command]
pub async fn change_content_type(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    new_content_type: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::CopyObject, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .change_content_type(&bucket, &key, &new_content_type)
        .await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Create a folder (empty object with trailing slash)
#[tauri::command]
pub async fn create_folder(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validation::validate_bucket_name(&bucket).map_err(|e| e.to_string())?;
    let validated_path =
        validation::validate_folder_path(&folder_path).map_err(|e| e.to_string())?;

    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::PutObject, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&validated_path);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.create_folder(&bucket, &validated_path).await;
    ctx.emit_result(&app, &result);

    // Add folder to index after successful creation
    if result.is_ok() {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            let folder_obj = S3Object {
                key: validated_path.clone(),
                size: 0,
                last_modified: Some(chrono::Utc::now().to_rfc3339()),
                storage_class: Some("STANDARD".to_string()),
                e_tag: None,
                is_folder: true,
            };
            let _ = index_mgr.add_object(&bucket, &folder_obj);
        }
    }

    result.map_err(|e| e.to_string())
}

/// Generate a presigned URL (local operation, no S3 API call)
#[tauri::command]
pub async fn generate_presigned_url(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    method: String,
    expires_in_secs: u64,
    state: State<'_, AppState>,
) -> Result<PresignedUrlResponse, String> {
    validation::validate_bucket_name(&bucket).map_err(|e| e.to_string())?;
    validation::validate_object_key(&key).map_err(|e| e.to_string())?;
    validation::validate_presigned_url_expiry(expires_in_secs).map_err(|e| e.to_string())?;

    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::GeneratePresignedUrl, RequestCategory::LOCAL)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .generate_presigned_url(&bucket, &key, &method, expires_in_secs)
        .await;

    ctx.emit_result(&app, &result);

    let url = result.map_err(|e| e.to_string())?;
    Ok(PresignedUrlResponse {
        url,
        expires_in_secs,
    })
}

/// Calculate folder size by summing ALL objects in the prefix (including all subdirectories recursively)
/// Returns (size, is_estimate) where is_estimate=true if from incomplete index
/// Note: Uses index if available, otherwise makes multiple ListObjectsV2 calls
#[tauri::command]
pub async fn calculate_folder_size(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    prefix: String,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(i64, bool), String> {
    // Try index first unless force_refresh is requested
    if !force_refresh.unwrap_or(false) {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            if let Ok((size, is_complete)) = index_mgr.calculate_folder_size(&bucket, &prefix) {
                // Emit cache hit - estimate saved requests based on prefix stats
                let saved_requests = index_mgr
                    .get_prefix_stats(&bucket, &prefix)
                    .map(|stats| ((stats.objects_count as f64 / 1000.0).ceil() as i32).max(1))
                    .unwrap_or(1);
                metrics_storage::emit_cache_hit(
                    "FolderSize",
                    Some(&profile_id),
                    Some(&bucket),
                    saved_requests,
                );

                // Return size from index with is_estimate flag
                return Ok((size, !is_complete));
            }
        }
    }

    // Emit cache miss - falling back to S3
    metrics_storage::emit_cache_miss("FolderSize", Some(&profile_id), Some(&bucket));

    // Fallback to S3 calculation
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let start_time = std::time::Instant::now();

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.calculate_folder_size(&bucket, &prefix).await;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    match &result {
        Ok((_size, request_count)) => {
            // Emit one metric per API request made
            let avg_duration = duration_ms / (*request_count as u64).max(1);

            for _ in 0..*request_count {
                let event = S3MetricsEvent::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
                    .with_duration(avg_duration)
                    .with_profile(&profile.id, &profile.name)
                    .with_bucket(&bucket)
                    .with_object_key(&prefix);
                crate::metrics::emit_metrics(&app, event);
            }
        }
        Err(e) => {
            let ctx = MetricsContext::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket)
                .with_object_key(&prefix);
            ctx.emit_error(&app, &e.to_string());
        }
    }

    // Return (size, is_estimate=false) since we got complete data from S3
    result
        .map(|(size, _)| (size, false))
        .map_err(|e| e.to_string())
}

/// Delete a folder and all its contents
/// Note: This makes multiple ListObjectsV2 and DeleteObjects calls
#[tauri::command]
pub async fn delete_folder(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    prefix: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let start_time = std::time::Instant::now();

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.delete_folder(&bucket, &prefix).await;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    match &result {
        Ok((deleted_count, list_request_count, delete_request_count)) => {
            let total_requests = list_request_count + delete_request_count;
            let avg_duration = duration_ms / (total_requests as u64).max(1);

            // Emit metrics for ListObjectsV2 calls
            for _ in 0..*list_request_count {
                let event = S3MetricsEvent::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
                    .with_duration(avg_duration)
                    .with_profile(&profile.id, &profile.name)
                    .with_bucket(&bucket)
                    .with_object_key(&prefix);
                crate::metrics::emit_metrics(&app, event);
            }

            // Emit metrics for DeleteObjects calls
            let objects_per_delete = (*deleted_count as u32) / (*delete_request_count).max(1);
            for _ in 0..*delete_request_count {
                let event =
                    S3MetricsEvent::new(S3Operation::DeleteObjects, RequestCategory::DELETE)
                        .with_duration(avg_duration)
                        .with_profile(&profile.id, &profile.name)
                        .with_bucket(&bucket)
                        .with_object_key(&prefix)
                        .with_objects_affected(objects_per_delete);
                crate::metrics::emit_metrics(&app, event);
            }
        }
        Err(e) => {
            let ctx = MetricsContext::new(S3Operation::DeleteObjects, RequestCategory::DELETE)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket)
                .with_object_key(&prefix);
            ctx.emit_error(&app, &e.to_string());
        }
    }

    // Remove folder from index after successful deletion
    if let Ok((_, _, _)) = &result {
        if let Ok(index_mgr) = get_index_manager(&profile_id) {
            let _ = index_mgr.remove_folder(&bucket, &prefix);
        }
    }

    result.map(|(count, _, _)| count).map_err(|e| e.to_string())
}

/// List all versions of an object
#[tauri::command]
pub async fn list_object_versions(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<ListObjectVersionsResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let mut ctx = MetricsContext::new(S3Operation::ListObjectVersions, RequestCategory::LIST)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.list_object_versions(&bucket, &key).await;

    match &result {
        Ok(response) => {
            ctx.set_objects_affected(response.versions.len() as u32);
            ctx.emit_success(&app);
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map_err(|e| e.to_string())
}

/// Get file metadata (size) without reading the entire file
#[tauri::command]
pub async fn get_file_size(file_path: String) -> Result<u64, String> {
    use std::fs;

    let metadata =
        fs::metadata(&file_path).map_err(|e| format!("Failed to get file metadata: {}", e))?;

    Ok(metadata.len())
}

/// Upload a file directly from disk using multipart upload with progress events
/// This is a non-blocking command that spawns a background task
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn upload_file(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    file_path: String,
    content_type: Option<String>,
    multipart_threshold_mb: Option<u64>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Generate unique upload ID
    let upload_id = uuid::Uuid::new_v4().to_string();

    // Get file metadata
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;
    let file_size = metadata.len();

    // Extract file name from path
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Get profile
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    // Create cancellation channel
    let (cancel_tx, mut cancel_rx) = broadcast::channel::<()>(1);

    // Clone data for the background task
    let upload_id_clone = upload_id.clone();
    let file_name_clone = file_name.clone();
    let active_uploads = state.active_uploads.clone();

    // Emit initial pending event
    let _ = app.emit_all(
        "upload:progress",
        UploadProgressEvent {
            upload_id: upload_id.clone(),
            file_name: file_name.clone(),
            file_size,
            uploaded_bytes: 0,
            uploaded_parts: 0,
            total_parts: 0,
            percentage: 0.0,
            status: UploadStatus::Pending,
            error: None,
        },
    );

    // Clone values for use after perform_upload
    let profile_id_for_index = profile.id.clone();
    let bucket_for_index = bucket.clone();
    let key_for_index = key.clone();

    // Spawn background upload task
    let handle = tokio::spawn(async move {
        // Emit starting event
        let _ = app.emit_all(
            "upload:progress",
            UploadProgressEvent {
                upload_id: upload_id_clone.clone(),
                file_name: file_name_clone.clone(),
                file_size,
                uploaded_bytes: 0,
                uploaded_parts: 0,
                total_parts: 0,
                percentage: 0.0,
                status: UploadStatus::Starting,
                error: None,
            },
        );

        // Perform the upload
        match perform_upload(
            app.clone(),
            upload_id_clone.clone(),
            file_name_clone.clone(),
            file_path,
            file_size,
            profile,
            bucket,
            key,
            content_type,
            multipart_threshold_mb,
            &mut cancel_rx,
        )
        .await
        {
            Ok(_) => {
                // Add object to index after successful upload
                if let Ok(index_mgr) = get_index_manager(&profile_id_for_index) {
                    let obj = S3Object {
                        key: key_for_index.clone(),
                        size: file_size as i64,
                        last_modified: Some(chrono::Utc::now().to_rfc3339()),
                        storage_class: Some("STANDARD".to_string()),
                        e_tag: None,
                        is_folder: false,
                    };
                    let _ = index_mgr.add_object(&bucket_for_index, &obj);
                }

                // Emit completion event
                let _ = app.emit_all(
                    "upload:progress",
                    UploadProgressEvent {
                        upload_id: upload_id_clone.clone(),
                        file_name: file_name_clone.clone(),
                        file_size,
                        uploaded_bytes: file_size,
                        uploaded_parts: 0,
                        total_parts: 0,
                        percentage: 100.0,
                        status: UploadStatus::Completed,
                        error: None,
                    },
                );
            }
            Err(e) => {
                // Check if it was cancelled
                let is_cancelled = e.contains("cancelled") || e.contains("Cancelled");

                // Emit error/cancelled event
                let _ = app.emit_all(
                    "upload:progress",
                    UploadProgressEvent {
                        upload_id: upload_id_clone.clone(),
                        file_name: file_name_clone.clone(),
                        file_size,
                        uploaded_bytes: 0,
                        uploaded_parts: 0,
                        total_parts: 0,
                        percentage: 0.0,
                        status: if is_cancelled {
                            UploadStatus::Cancelled
                        } else {
                            UploadStatus::Failed
                        },
                        error: Some(e),
                    },
                );
            }
        }

        // Remove from active uploads
        if let Ok(mut uploads) = active_uploads.lock() {
            uploads.remove(&upload_id_clone);
        }
    });

    // Store the task handle and cancellation sender with metadata
    {
        let mut uploads = state.active_uploads.lock().map_err(|e| e.to_string())?;
        uploads.insert(
            upload_id.clone(),
            UploadTask {
                handle,
                cancel_tx,
                file_name,
                file_size,
            },
        );
    }

    Ok(upload_id)
}

/// Cancel an active upload
#[tauri::command]
pub async fn cancel_upload(
    app: AppHandle,
    upload_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut uploads = state.active_uploads.lock().map_err(|e| e.to_string())?;

    if let Some(task) = uploads.remove(&upload_id) {
        // Emit cancelled event immediately (before aborting the task)
        let _ = app.emit_all(
            "upload:progress",
            UploadProgressEvent {
                upload_id: upload_id.clone(),
                file_name: task.file_name.clone(),
                file_size: task.file_size,
                uploaded_bytes: 0,
                uploaded_parts: 0,
                total_parts: 0,
                percentage: 0.0,
                status: UploadStatus::Cancelled,
                error: Some("Upload cancelled by user".to_string()),
            },
        );

        // Send cancellation signal
        let _ = task.cancel_tx.send(());

        // Abort the task
        task.handle.abort();

        Ok(())
    } else {
        Err(format!("Upload {} not found", upload_id))
    }
}

/// Perform the actual upload (helper function)
#[allow(clippy::too_many_arguments)]
async fn perform_upload(
    app: AppHandle,
    upload_id: String,
    file_name: String,
    file_path: String,
    file_size: u64,
    profile: Profile,
    bucket: String,
    key: String,
    content_type: Option<String>,
    multipart_threshold_mb: Option<u64>,
    cancel_rx: &mut broadcast::Receiver<()>,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    // Multipart threshold: configurable, default 50MB
    let multipart_threshold: u64 = multipart_threshold_mb.unwrap_or(50) * 1024 * 1024;
    const PART_SIZE: u64 = 10 * 1024 * 1024; // 10MB parts

    // Create S3 adapter
    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    // Check if we should use multipart upload
    if file_size < multipart_threshold {
        // Simple upload for small files
        let mut file = File::open(&file_path).map_err(|e| format!("Failed to open file: {}", e))?;

        let mut buffer = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Check for cancellation
        if cancel_rx.try_recv().is_ok() {
            return Err("Upload cancelled".to_string());
        }

        // Metrics for simple upload
        let mut ctx = MetricsContext::new(S3Operation::PutObject, RequestCategory::PUT)
            .with_profile(&profile.id, &profile.name)
            .with_bucket(&bucket)
            .with_object_key(&key);
        ctx.set_bytes(file_size);

        // Upload
        let result = adapter
            .put_object(&bucket, &key, buffer, content_type)
            .await
            .map_err(|e| e.to_string());

        ctx.emit_result(&app, &result);
        result?;

        Ok(())
    } else {
        // Multipart upload for large files
        let total_parts = file_size.div_ceil(PART_SIZE) as i32;

        // Start multipart upload with metrics
        let init_ctx =
            MetricsContext::new(S3Operation::CreateMultipartUpload, RequestCategory::PUT)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket)
                .with_object_key(&key);

        let init_result = adapter
            .multipart_upload_start(&bucket, &key, content_type)
            .await
            .map_err(|e| e.to_string());

        init_ctx.emit_result(&app, &init_result);
        let init_response = init_result?;

        let s3_upload_id = init_response.upload_id.clone();

        // Upload parts
        let mut completed_parts = Vec::new();
        let mut uploaded_bytes: u64 = 0;

        // OPTIMIZATION: Open file ONCE before the loop (instead of per-part)
        // This eliminates ~200 syscalls (open/close) for a 1GB file
        let mut file = File::open(&file_path).map_err(|e| format!("Failed to open file: {}", e))?;

        // OPTIMIZATION: Allocate buffer ONCE (instead of per-part)
        // This eliminates ~100 allocations/deallocations of 10MB each
        let mut buffer = vec![0u8; PART_SIZE as usize];

        for part_number in 1..=total_parts {
            // Check for cancellation before each part
            if cancel_rx.try_recv().is_ok() {
                // Abort the multipart upload with metrics
                let abort_ctx =
                    MetricsContext::new(S3Operation::AbortMultipartUpload, RequestCategory::PUT)
                        .with_profile(&profile.id, &profile.name)
                        .with_bucket(&bucket)
                        .with_object_key(&key);

                let abort_result = adapter
                    .multipart_upload_abort(&bucket, &key, &s3_upload_id)
                    .await
                    .map_err(|e| e.to_string());

                abort_ctx.emit_result(&app, &abort_result);
                return Err("Upload cancelled".to_string());
            }

            // Calculate offset and length for this part
            let offset = (part_number - 1) as u64 * PART_SIZE;
            let length = std::cmp::min(PART_SIZE, file_size - offset);

            // Seek and read chunk (reusing file handle and buffer)
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| format!("Failed to seek file: {}", e))?;

            file.read_exact(&mut buffer[..length as usize])
                .map_err(|e| format!("Failed to read file chunk: {}", e))?;

            // Clone the slice for async upload (buffer is reused for next iteration)
            let chunk = buffer[..length as usize].to_vec();

            // Upload part with metrics
            let mut part_ctx = MetricsContext::new(S3Operation::UploadPart, RequestCategory::PUT)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket)
                .with_object_key(&key);
            part_ctx.set_bytes(length);

            let part_result = adapter
                .multipart_upload_part(&bucket, &key, &s3_upload_id, part_number, chunk)
                .await
                .map_err(|e| e.to_string());

            part_ctx.emit_result(&app, &part_result);
            let part_response = part_result?;

            completed_parts.push(CompletedPart {
                part_number,
                e_tag: part_response.e_tag,
            });

            uploaded_bytes += length;

            // Emit progress event
            let percentage = (uploaded_bytes as f64 / file_size as f64) * 100.0;
            let _ = app.emit_all(
                "upload:progress",
                UploadProgressEvent {
                    upload_id: upload_id.clone(),
                    file_name: file_name.clone(),
                    file_size,
                    uploaded_bytes,
                    uploaded_parts: part_number,
                    total_parts,
                    percentage,
                    status: UploadStatus::Uploading,
                    error: None,
                },
            );
        }

        // File handle and buffer are automatically dropped here

        // Complete multipart upload with metrics
        let complete_ctx =
            MetricsContext::new(S3Operation::CompleteMultipartUpload, RequestCategory::PUT)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket)
                .with_object_key(&key);

        let complete_result = adapter
            .multipart_upload_complete(&bucket, &key, &s3_upload_id, completed_parts)
            .await
            .map_err(|e| e.to_string());

        complete_ctx.emit_result(&app, &complete_result);
        complete_result?;

        Ok(())
    }
}

/// Get object tags
#[tauri::command]
pub async fn get_object_tags(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<GetObjectTagsResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let mut ctx = MetricsContext::new(S3Operation::GetObjectTagging, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.get_object_tagging(&bucket, &key).await;

    match &result {
        Ok(tags) => {
            ctx.set_objects_affected(tags.len() as u32);
            ctx.emit_success(&app);
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    let tags = result.map_err(|e| e.to_string())?;
    Ok(GetObjectTagsResponse { tags })
}

/// Put object tags
#[tauri::command]
pub async fn put_object_tags(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    tags: Vec<ObjectTag>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::PutObjectTagging, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.put_object_tagging(&bucket, &key, tags).await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Delete object tags
#[tauri::command]
pub async fn delete_object_tags(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::DeleteObjectTagging, RequestCategory::DELETE)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.delete_object_tagging(&bucket, &key).await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Get object metadata (HTTP headers)
#[tauri::command]
pub async fn get_object_metadata(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<GetObjectMetadataResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::HeadObject, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.get_object_metadata(&bucket, &key).await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

/// Update object metadata (HTTP headers)
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_object_metadata(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    content_type: Option<String>,
    content_encoding: Option<String>,
    content_language: Option<String>,
    content_disposition: Option<String>,
    cache_control: Option<String>,
    expires: Option<String>,
    metadata: std::collections::HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::CopyObject, RequestCategory::PUT)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter
        .update_object_metadata(
            &bucket,
            &key,
            content_type,
            content_encoding,
            content_language,
            content_disposition,
            cache_control,
            expires,
            metadata,
        )
        .await;

    ctx.emit_result(&app, &result);
    result.map_err(|e| e.to_string())
}

/// Get object lock status (retention and legal hold)
#[tauri::command]
pub async fn get_object_lock_status(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<ObjectLockStatus, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let ctx = MetricsContext::new(S3Operation::GetObjectLockStatus, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.get_object_lock_status(&bucket, &key).await;
    ctx.emit_result(&app, &result);

    result.map_err(|e| e.to_string())
}

// ============================================================================
// Index Management Commands
// ============================================================================

/// Start initial indexation of a bucket
/// Makes up to max_requests ListObjectsV2 calls without delimiter
#[tauri::command]
pub async fn start_initial_index(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    max_requests: Option<u32>,
    batch_size: Option<i32>,
    state: State<'_, AppState>,
) -> Result<InitialIndexResult, String> {
    let index_key = format!("{}-{}", profile_id, bucket_name);

    // Check if indexing is already in progress for this bucket
    {
        let indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        if indexing.contains_key(&index_key) {
            return Err("Indexing already in progress for this bucket".to_string());
        }
    }

    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    // Use batch_size from parameter or default to 1000 (S3 max)
    let effective_batch_size = batch_size.unwrap_or(1000).clamp(1, 1000);

    // Create cancellation channel
    let (cancel_tx, cancel_rx) = broadcast::channel::<()>(1);

    // Emit starting event
    let _ = app.emit_all(
        "index:progress",
        IndexProgressEvent {
            profile_id: profile_id.clone(),
            bucket_name: bucket_name.clone(),
            objects_indexed: 0,
            requests_made: 0,
            max_requests: max_requests.unwrap_or(20),
            is_complete: false,
            status: IndexStatus::Starting,
            error: None,
        },
    );

    // Get index manager
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;

    // Create adapter
    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    // Configure indexation
    let config = IndexingConfig {
        max_initial_requests: max_requests.unwrap_or(20),
        batch_size: effective_batch_size,
        stale_ttl_hours: 24,
    };

    // Register the indexing task (with a dummy handle for now)
    // We need to register before starting so cancel_indexing can find it
    {
        let mut indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        // Create a dummy handle that we'll never use (the actual work happens in this function)
        let dummy_handle = tokio::spawn(async {});
        indexing.insert(
            index_key.clone(),
            IndexTask {
                handle: dummy_handle,
                cancel_tx,
                bucket_name: bucket_name.clone(),
            },
        );
    }

    // Emit indexing event
    let _ = app.emit_all(
        "index:progress",
        IndexProgressEvent {
            profile_id: profile_id.clone(),
            bucket_name: bucket_name.clone(),
            objects_indexed: 0,
            requests_made: 0,
            max_requests: config.max_initial_requests,
            is_complete: false,
            status: IndexStatus::Indexing,
            error: None,
        },
    );

    // Clone values for the closure
    let app_clone = app.clone();
    let profile_id_clone = profile_id.clone();
    let bucket_name_clone = bucket_name.clone();

    // Run initial indexation with progress callback and cancellation receiver
    let result = index_mgr
        .initial_index_bucket(
            &adapter,
            &bucket_name,
            &config,
            |objects_indexed, requests_made, max_requests| {
                let _ = app_clone.emit_all(
                    "index:progress",
                    IndexProgressEvent {
                        profile_id: profile_id_clone.clone(),
                        bucket_name: bucket_name_clone.clone(),
                        objects_indexed,
                        requests_made,
                        max_requests,
                        is_complete: false,
                        status: IndexStatus::Indexing,
                        error: None,
                    },
                );
            },
            Some(cancel_rx),
        )
        .await;

    // Remove from active indexing
    {
        let mut indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        indexing.remove(&index_key);
    }

    match &result {
        Ok(index_result) => {
            // Determine status based on result
            let status = if index_result
                .error
                .as_ref()
                .map(|e| e.contains("Cancelled"))
                .unwrap_or(false)
            {
                IndexStatus::Cancelled
            } else if index_result.is_complete {
                IndexStatus::Completed
            } else {
                IndexStatus::Partial
            };

            let _ = app.emit_all(
                "index:progress",
                IndexProgressEvent {
                    profile_id: profile_id.clone(),
                    bucket_name: bucket_name.clone(),
                    objects_indexed: index_result.total_indexed,
                    requests_made: index_result.requests_made,
                    max_requests: config.max_initial_requests,
                    is_complete: index_result.is_complete,
                    status,
                    error: index_result.error.clone(),
                },
            );

            // Emit metrics for each ListObjectsV2 request made during indexation
            for _ in 0..index_result.requests_made {
                let ctx = MetricsContext::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
                    .with_profile(&profile_id, &profile.name)
                    .with_bucket(&bucket_name);
                ctx.emit_success(&app);
            }
        }
        Err(e) => {
            let _ = app.emit_all(
                "index:progress",
                IndexProgressEvent {
                    profile_id: profile_id.clone(),
                    bucket_name: bucket_name.clone(),
                    objects_indexed: 0,
                    requests_made: 0,
                    max_requests: config.max_initial_requests,
                    is_complete: false,
                    status: IndexStatus::Failed,
                    error: Some(e.to_string()),
                },
            );
        }
    }

    result.map_err(|e| e.to_string())
}

/// Cancel an active indexing operation
/// The partial index is preserved and can be resumed later
#[tauri::command]
pub async fn cancel_indexing(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let index_key = format!("{}-{}", profile_id, bucket_name);

    let task = {
        let mut indexing = state.active_indexing.lock().map_err(|e| e.to_string())?;
        indexing.remove(&index_key)
    };

    if let Some(task) = task {
        // Send cancellation signal - the indexing loop will check this
        let _ = task.cancel_tx.send(());

        // Emit cancelled event immediately for responsive UI
        let _ = app.emit_all(
            "index:progress",
            IndexProgressEvent {
                profile_id: profile_id.clone(),
                bucket_name: bucket_name.clone(),
                objects_indexed: 0,
                requests_made: 0,
                max_requests: 0,
                is_complete: false,
                status: IndexStatus::Cancelled,
                error: Some("Indexing cancelled by user".to_string()),
            },
        );

        Ok(())
    } else {
        Err("No active indexing found for this bucket".to_string())
    }
}

/// Get bucket statistics from local index
#[tauri::command]
pub async fn get_bucket_index_stats(
    profile_id: String,
    bucket_name: String,
) -> Result<BucketIndexStats, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    index_mgr
        .get_bucket_stats(&bucket_name)
        .map_err(|e| e.to_string())
}

/// Get prefix (folder) statistics from local index
#[tauri::command]
pub async fn get_prefix_index_stats(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<PrefixStats, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    let stats = index_mgr
        .get_prefix_stats(&bucket_name, &prefix)
        .map_err(|e| e.to_string())?;

    // Emit cache hit - getting prefix stats from index avoids listing from S3
    let saved_requests = ((stats.objects_count as f64 / 1000.0).ceil() as i32).max(1);
    metrics_storage::emit_cache_hit(
        "PrefixStats",
        Some(&profile_id),
        Some(&bucket_name),
        saved_requests,
    );

    Ok(stats)
}

/// Clear bucket index (for re-indexation)
#[tauri::command]
pub async fn clear_bucket_index(profile_id: String, bucket_name: String) -> Result<(), String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    index_mgr
        .clear_bucket_index(&bucket_name)
        .map_err(|e| e.to_string())
}

/// Check if a bucket has been indexed
#[tauri::command]
pub async fn is_bucket_indexed(profile_id: String, bucket_name: String) -> Result<bool, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    index_mgr
        .is_bucket_indexed(&bucket_name)
        .map_err(|e| e.to_string())
}

/// Check if bucket index is complete
#[tauri::command]
pub async fn is_bucket_index_complete(
    profile_id: String,
    bucket_name: String,
) -> Result<bool, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    index_mgr
        .is_bucket_complete(&bucket_name)
        .map_err(|e| e.to_string())
}

/// Check if a prefix has any objects in the index (has been browsed/indexed before)
/// Returns true if the prefix has at least one object indexed, false otherwise
#[tauri::command]
pub async fn is_prefix_known(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<bool, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;

    // Check if we have at least one object with this prefix
    let (count, _) = index_mgr
        .db
        .calculate_prefix_stats(&bucket_name, &prefix)
        .map_err(|e| e.to_string())?;

    Ok(count > 0)
}

/// Get the full status of a prefix from the index
/// Returns None if the prefix has never been seen
#[tauri::command]
pub async fn get_prefix_status(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<Option<PrefixStatus>, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    index_mgr
        .get_prefix_status(&bucket_name, &prefix)
        .map_err(|e| e.to_string())
}

/// Check if a prefix is "discovered only" (seen in common_prefixes but never navigated into)
/// Returns true if the prefix exists in prefix_status but has objects_count = 0 and is not complete
#[tauri::command]
pub async fn is_prefix_discovered_only(
    profile_id: String,
    bucket_name: String,
    prefix: String,
) -> Result<bool, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;

    match index_mgr
        .get_prefix_status(&bucket_name, &prefix)
        .map_err(|e| e.to_string())?
    {
        None => Ok(true), // Not in index at all = discovered only (or never seen)
        Some(status) => {
            // Discovered only if objects_count = 0 and not marked complete
            Ok(status.objects_count == 0 && !status.is_complete)
        }
    }
}

/// Search objects in the index
#[tauri::command]
pub async fn search_objects_in_index(
    profile_id: String,
    bucket_name: String,
    query: String,
    prefix: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<S3Object>, String> {
    let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
    let result = index_mgr
        .search_objects(&bucket_name, &query, prefix.as_deref(), limit)
        .map_err(|e| e.to_string())?;

    // Emit cache hit - search in index avoids listing all objects from S3
    // Estimate saved requests based on bucket stats
    let saved_requests = index_mgr
        .get_bucket_stats(&bucket_name)
        .map(|stats| ((stats.total_objects as f64 / 1000.0).ceil() as i32).max(1))
        .unwrap_or(1);
    metrics_storage::emit_cache_hit(
        "Search",
        Some(&profile_id),
        Some(&bucket_name),
        saved_requests,
    );

    Ok(result)
}

/// Get all bucket indexes for a profile
///
/// Uses spawn_blocking to prevent blocking the main tokio runtime,
/// keeping the UI responsive even with large indexes.
#[tauri::command]
pub async fn get_all_bucket_indexes(
    profile_id: String,
) -> Result<Vec<BucketIndexMetadata>, String> {
    // Run the potentially slow SQLite query in a blocking thread pool
    // to avoid blocking the tokio runtime and keeping UI responsive
    spawn_blocking(move || {
        let index_mgr = get_index_manager(&profile_id).map_err(|e| e.to_string())?;
        index_mgr
            .get_all_bucket_indexes()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Get the index database file size on disk for a profile (in bytes)
#[tauri::command]
pub async fn get_index_file_size(profile_id: String) -> Result<u64, String> {
    DatabaseManager::get_db_file_size(&profile_id).map_err(|e| e.to_string())
}

// ============================================================================
// Download Commands (Streaming to disk with progress)
// ============================================================================

/// Helper function to emit download progress events
fn emit_download_progress(
    app: &AppHandle,
    download_id: &str,
    file_name: &str,
    file_size: u64,
    downloaded_bytes: u64,
    status: DownloadStatus,
    error: Option<String>,
    bytes_per_second: Option<f64>,
) {
    let percentage = if file_size > 0 {
        (downloaded_bytes as f64 / file_size as f64) * 100.0
    } else {
        0.0
    };

    let _ = app.emit_all(
        "download:progress",
        DownloadProgressEvent {
            download_id: download_id.to_string(),
            file_name: file_name.to_string(),
            file_size,
            downloaded_bytes,
            percentage,
            status,
            error,
            bytes_per_second,
        },
    );
}

/// Download a file from S3 directly to disk with streaming (no memory buffering)
/// Returns a download_id that can be used to track progress and cancel the download
/// If version_id is provided, downloads that specific version of the object
#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    version_id: Option<String>,
    dest_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Get profile
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    // Generate download ID
    let download_id = uuid::Uuid::new_v4().to_string();

    // Create S3 adapter
    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    // Get object size first (HEAD request) with metrics
    let head_ctx = MetricsContext::new(S3Operation::HeadObject, RequestCategory::GET)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket)
        .with_object_key(&key);
    let file_size = match adapter
        .get_object_size(&bucket, &key, version_id.as_deref())
        .await
    {
        Ok(size) => {
            head_ctx.emit_success(&app);
            size
        }
        Err(e) => {
            head_ctx.emit_error(&app, &e.to_string());
            0 // Continue with size 0, actual size will be determined from GET response
        }
    };

    // Extract file name from key
    let file_name = key.split('/').next_back().unwrap_or(&key).to_string();

    // Create cancellation channel
    let (cancel_tx, cancel_rx) = broadcast::channel::<()>(1);

    // Emit initial pending event
    emit_download_progress(
        &app,
        &download_id,
        &file_name,
        file_size,
        0,
        DownloadStatus::Pending,
        None,
        None,
    );

    // Clone values for the spawned task
    let download_id_clone = download_id.clone();
    let file_name_clone = file_name.clone();
    let app_clone = app.clone();
    let state_downloads = state.active_downloads.clone();
    let version_id_clone = version_id.clone();
    // Clone for metrics
    let profile_id_for_metrics = profile.id.clone();
    let profile_name_for_metrics = profile.name.clone();
    let bucket_for_metrics = bucket.clone();
    let key_for_metrics = key.clone();

    // Spawn the download task
    let task_handle = tokio::spawn(async move {
        const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks for progress updates
        const PROGRESS_INTERVAL_MS: u128 = 100;

        // Emit starting event
        emit_download_progress(
            &app_clone,
            &download_id_clone,
            &file_name_clone,
            file_size,
            0,
            DownloadStatus::Starting,
            None,
            None,
        );

        // Get S3 object stream
        let (body_stream, actual_size) = match adapter
            .get_object_stream(&bucket, &key, version_id_clone.as_deref())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                emit_download_progress(
                    &app_clone,
                    &download_id_clone,
                    &file_name_clone,
                    file_size,
                    0,
                    DownloadStatus::Failed,
                    Some(format!("S3 error: {}", e)),
                    None,
                );
                // Clean up from active downloads
                if let Ok(mut downloads) = state_downloads.lock() {
                    downloads.remove(&download_id_clone);
                }
                return;
            }
        };

        // Use actual size from GET response if HEAD failed
        let file_size = if file_size > 0 {
            file_size
        } else {
            actual_size
        };

        // Create destination file
        let mut file = match tokio::fs::File::create(&dest_path).await {
            Ok(f) => f,
            Err(e) => {
                emit_download_progress(
                    &app_clone,
                    &download_id_clone,
                    &file_name_clone,
                    file_size,
                    0,
                    DownloadStatus::Failed,
                    Some(format!("Cannot create file: {}", e)),
                    None,
                );
                if let Ok(mut downloads) = state_downloads.lock() {
                    downloads.remove(&download_id_clone);
                }
                return;
            }
        };

        // Stream body to file with progress
        let mut stream = body_stream.into_async_read();
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut downloaded_bytes: u64 = 0;
        let mut last_progress_emit = std::time::Instant::now();
        let start_time = std::time::Instant::now();
        let mut cancel_rx = cancel_rx;

        loop {
            // Check cancellation
            if cancel_rx.try_recv().is_ok() {
                // Clean up partial file
                drop(file);
                let _ = tokio::fs::remove_file(&dest_path).await;
                emit_download_progress(
                    &app_clone,
                    &download_id_clone,
                    &file_name_clone,
                    file_size,
                    downloaded_bytes,
                    DownloadStatus::Cancelled,
                    Some("Download cancelled".to_string()),
                    None,
                );
                if let Ok(mut downloads) = state_downloads.lock() {
                    downloads.remove(&download_id_clone);
                }
                return;
            }

            // Read chunk from stream
            let bytes_read = match stream.read(&mut buffer).await {
                Ok(0) => break, // EOF - download complete
                Ok(n) => n,
                Err(e) => {
                    drop(file);
                    let _ = tokio::fs::remove_file(&dest_path).await;
                    emit_download_progress(
                        &app_clone,
                        &download_id_clone,
                        &file_name_clone,
                        file_size,
                        downloaded_bytes,
                        DownloadStatus::Failed,
                        Some(format!("Read error: {}", e)),
                        None,
                    );
                    if let Ok(mut downloads) = state_downloads.lock() {
                        downloads.remove(&download_id_clone);
                    }
                    return;
                }
            };

            // Write to file
            if let Err(e) = file.write_all(&buffer[..bytes_read]).await {
                drop(file);
                let _ = tokio::fs::remove_file(&dest_path).await;
                emit_download_progress(
                    &app_clone,
                    &download_id_clone,
                    &file_name_clone,
                    file_size,
                    downloaded_bytes,
                    DownloadStatus::Failed,
                    Some(format!("Write error: {}", e)),
                    None,
                );
                if let Ok(mut downloads) = state_downloads.lock() {
                    downloads.remove(&download_id_clone);
                }
                return;
            }

            downloaded_bytes += bytes_read as u64;

            // Emit progress (throttled to avoid flooding)
            if last_progress_emit.elapsed().as_millis() >= PROGRESS_INTERVAL_MS {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    downloaded_bytes as f64 / elapsed
                } else {
                    0.0
                };

                emit_download_progress(
                    &app_clone,
                    &download_id_clone,
                    &file_name_clone,
                    file_size,
                    downloaded_bytes,
                    DownloadStatus::Downloading,
                    None,
                    Some(speed),
                );
                last_progress_emit = std::time::Instant::now();
            }
        }

        // Flush and sync file to ensure data is written
        if let Err(e) = file.flush().await {
            emit_download_progress(
                &app_clone,
                &download_id_clone,
                &file_name_clone,
                file_size,
                downloaded_bytes,
                DownloadStatus::Failed,
                Some(format!("Flush error: {}", e)),
                None,
            );
            if let Ok(mut downloads) = state_downloads.lock() {
                downloads.remove(&download_id_clone);
            }
            return;
        }

        // Calculate final speed
        let elapsed = start_time.elapsed().as_secs_f64();
        let final_speed = if elapsed > 0.0 {
            downloaded_bytes as f64 / elapsed
        } else {
            0.0
        };

        // Emit completed event
        emit_download_progress(
            &app_clone,
            &download_id_clone,
            &file_name_clone,
            file_size,
            downloaded_bytes,
            DownloadStatus::Completed,
            None,
            Some(final_speed),
        );

        // Emit metrics for the download (GetObject)
        let mut ctx = MetricsContext::new(S3Operation::GetObject, RequestCategory::GET)
            .with_profile(&profile_id_for_metrics, &profile_name_for_metrics)
            .with_bucket(&bucket_for_metrics)
            .with_object_key(&key_for_metrics);
        ctx.set_bytes(downloaded_bytes);
        ctx.emit_success(&app_clone);

        // Clean up from active downloads
        if let Ok(mut downloads) = state_downloads.lock() {
            downloads.remove(&download_id_clone);
        }
    });

    // Store the task for cancellation
    {
        let mut downloads = state.active_downloads.lock().map_err(|e| e.to_string())?;
        downloads.insert(
            download_id.clone(),
            DownloadTask {
                handle: task_handle,
                cancel_tx,
                file_name,
                file_size,
            },
        );
    }

    Ok(download_id)
}

/// Cancel an active download
#[tauri::command]
pub async fn cancel_download(
    app: AppHandle,
    download_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let task = {
        let mut downloads = state.active_downloads.lock().map_err(|e| e.to_string())?;
        downloads.remove(&download_id)
    };

    if let Some(task) = task {
        // Send cancellation signal
        let _ = task.cancel_tx.send(());

        // Emit cancelled event immediately
        emit_download_progress(
            &app,
            &download_id,
            &task.file_name,
            task.file_size,
            0,
            DownloadStatus::Cancelled,
            Some("Download cancelled by user".to_string()),
            None,
        );

        // Abort the task
        task.handle.abort();
    }

    Ok(())
}

// ============================================================================
// Cache Management Commands
// ============================================================================

/// Structure contenant le statut de tous les caches
#[derive(serde::Serialize)]
pub struct AllCachesStatus {
    pub database_managers: CacheStatus,
    pub index_managers: CacheStatus,
}

/// Obtenir le statut de tous les caches (pour monitoring/debug)
///
/// Retourne les metriques (hits, misses, evictions) et la configuration
/// pour les caches DatabaseManager et IndexManager.
#[tauri::command]
pub async fn get_cache_status() -> Result<AllCachesStatus, String> {
    Ok(AllCachesStatus {
        database_managers: get_db_cache_status(),
        index_managers: get_index_cache_status(),
    })
}

/// Prechauffer le cache pour un profil (warmup)
///
/// Cree les managers (DatabaseManager et IndexManager) pour un profil
/// en avance, evitant ainsi la latence lors de l'utilisation reelle.
/// Ideal a appeler lors du survol d'un profil dans l'UI.
#[tauri::command]
pub async fn warmup_profile_cache(profile_id: String) -> Result<(), String> {
    // Warmup en parallele avec spawn_blocking pour ne pas bloquer
    let profile_id_clone = profile_id.clone();

    spawn_blocking(move || {
        // Warmup DatabaseManager (cree aussi le pool SQLite)
        if let Err(e) = warmup_db_manager(&profile_id_clone) {
            eprintln!(
                "[CacheWarmup] Failed to warmup DB manager for {}: {}",
                profile_id_clone, e
            );
        }

        // Warmup IndexManager
        if let Err(e) = warmup_index_manager(&profile_id_clone) {
            eprintln!(
                "[CacheWarmup] Failed to warmup index manager for {}: {}",
                profile_id_clone, e
            );
        }
    })
    .await
    .map_err(|e| format!("Warmup task failed: {}", e))?;

    Ok(())
}

/// Nettoyer le cache pour un profil specifique
///
/// A appeler lors de la suppression d'un profil pour liberer les ressources
/// immediatement au lieu d'attendre l'eviction LRU/TTL.
#[tauri::command]
pub async fn cleanup_profile_cache(profile_id: String) -> Result<(), String> {
    close_index_manager(&profile_id);
    close_db_manager(&profile_id);
    Ok(())
}

/// Vider tous les caches (maintenance)
///
/// Libere toutes les ressources en cache. Les managers seront recrees
/// a la demande lors des prochains acces.
#[tauri::command]
pub async fn clear_all_caches() -> Result<(), String> {
    clear_all_index_managers();
    clear_all_db_managers();
    Ok(())
}

// ============================================================================
// Metrics Commands
// ============================================================================

/// Get today's metrics statistics
#[tauri::command]
pub async fn get_metrics_today(
    get_per_thousand: Option<f64>,
    put_per_thousand: Option<f64>,
    list_per_thousand: Option<f64>,
) -> Result<metrics_storage::DailyStats, String> {
    let pricing = metrics_storage::S3Pricing {
        get_per_thousand: get_per_thousand.unwrap_or(0.0004),
        put_per_thousand: put_per_thousand.unwrap_or(0.005),
        list_per_thousand: list_per_thousand.unwrap_or(0.005),
        delete_per_thousand: 0.0,
    };

    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_today_stats(&db, &pricing).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get metrics history for last N days
#[tauri::command]
pub async fn get_metrics_history(
    days: u32,
    get_per_thousand: Option<f64>,
    put_per_thousand: Option<f64>,
    list_per_thousand: Option<f64>,
) -> Result<Vec<metrics_storage::DailyStats>, String> {
    let pricing = metrics_storage::S3Pricing {
        get_per_thousand: get_per_thousand.unwrap_or(0.0004),
        put_per_thousand: put_per_thousand.unwrap_or(0.005),
        list_per_thousand: list_per_thousand.unwrap_or(0.005),
        delete_per_thousand: 0.0,
    };

    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_stats_history(&db, days, &pricing).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get hourly breakdown for today (or specified date)
#[tauri::command]
pub async fn get_metrics_hourly(
    date: Option<String>,
) -> Result<Vec<metrics_storage::HourlyStats>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        let target_date = date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
        metrics_storage::get_hourly_stats(&db, &target_date).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get hourly breakdown aggregated over a period
#[tauri::command]
pub async fn get_metrics_hourly_period(
    days: u32,
) -> Result<Vec<metrics_storage::HourlyStats>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_hourly_stats_period(&db, days).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get daily distribution for a period (for 7-day chart view)
#[tauri::command]
pub async fn get_metrics_daily_distribution(
    days: u32,
) -> Result<Vec<metrics_storage::DailyDistribution>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_daily_distribution(&db, days).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get weekly distribution for a period (for 30-day chart view)
#[tauri::command]
pub async fn get_metrics_weekly_distribution(
    days: u32,
) -> Result<Vec<metrics_storage::WeeklyDistribution>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_weekly_distribution(&db, days).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get aggregated stats for a period
#[tauri::command]
pub async fn get_metrics_period(
    days: u32,
    get_per_thousand: f64,
    put_per_thousand: f64,
    list_per_thousand: f64,
) -> Result<metrics_storage::DailyStats, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        let pricing = metrics_storage::S3Pricing {
            get_per_thousand,
            put_per_thousand,
            list_per_thousand,
            delete_per_thousand: 0.0,
        };
        metrics_storage::get_period_stats(&db, days, &pricing).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get metrics grouped by operation type
#[tauri::command]
pub async fn get_metrics_by_operation(
    days: u32,
) -> Result<Vec<metrics_storage::OperationStats>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_operation_stats(&db, days).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get error statistics
#[tauri::command]
pub async fn get_metrics_errors(days: u32) -> Result<Vec<metrics_storage::ErrorStats>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_error_stats(&db, days).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get top buckets by request count
#[tauri::command]
pub async fn get_metrics_top_buckets(
    days: u32,
    limit: u32,
) -> Result<Vec<metrics_storage::BucketUsageStats>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_top_buckets(&db, days, limit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get recent requests
#[tauri::command]
pub async fn get_metrics_recent(limit: u32) -> Result<Vec<metrics_storage::RequestRecord>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_recent_requests(&db, limit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get failed requests
#[tauri::command]
pub async fn get_metrics_failed(
    days: u32,
    limit: u32,
) -> Result<Vec<metrics_storage::RequestRecord>, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_failed_requests(&db, days, limit).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get storage info for metrics database
#[tauri::command]
pub async fn get_metrics_storage_info() -> Result<metrics_storage::StorageInfo, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_storage_info(&db).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Purge old metrics data
#[tauri::command]
pub async fn purge_metrics(retention_days: u32) -> Result<u64, String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        let requests_deleted =
            metrics_storage::purge_old_data(&db, retention_days).map_err(|e| e.to_string())?;
        let _ = metrics_storage::purge_cache_events(&db, retention_days);
        Ok(requests_deleted)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Clear all metrics data
#[tauri::command]
pub async fn clear_metrics() -> Result<(), String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::clear_all(&db).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Record a cache event
#[tauri::command]
pub async fn record_cache_event(event: metrics_storage::CacheEvent) -> Result<(), String> {
    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::record_cache_event(&db, &event).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get cache summary for a period
#[tauri::command]
pub async fn get_cache_summary(
    days: u32,
    get_per_thousand: Option<f64>,
    put_per_thousand: Option<f64>,
    list_per_thousand: Option<f64>,
) -> Result<metrics_storage::CacheSummary, String> {
    let pricing = metrics_storage::S3Pricing {
        get_per_thousand: get_per_thousand.unwrap_or(0.0004),
        put_per_thousand: put_per_thousand.unwrap_or(0.005),
        list_per_thousand: list_per_thousand.unwrap_or(0.005),
        delete_per_thousand: 0.0,
    };

    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_cache_summary(&db, days, &pricing).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get today's cache statistics
#[tauri::command]
pub async fn get_today_cache_stats(
    get_per_thousand: Option<f64>,
    put_per_thousand: Option<f64>,
    list_per_thousand: Option<f64>,
) -> Result<metrics_storage::DailyCacheStats, String> {
    let pricing = metrics_storage::S3Pricing {
        get_per_thousand: get_per_thousand.unwrap_or(0.0004),
        put_per_thousand: put_per_thousand.unwrap_or(0.005),
        list_per_thousand: list_per_thousand.unwrap_or(0.005),
        delete_per_thousand: 0.0,
    };

    spawn_blocking(move || {
        let db = metrics_storage::get_metrics_db().map_err(|e| e.to_string())?;
        metrics_storage::get_today_cache_stats(&db, &pricing).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ============================================================================
// Clipboard Upload Commands
// ============================================================================

/// Upload data directly from bytes (for clipboard images/text)
/// This writes the data to a temp file and uses the existing upload infrastructure
#[tauri::command]
pub async fn upload_from_bytes(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    key: String,
    data: Vec<u8>,
    content_type: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use std::io::Write;

    // Create a temp file with the data
    // Use UUID to ensure unique filename (upload_file spawns a background task
    // that reads the file asynchronously, so we can't delete it immediately)
    let temp_dir = std::env::temp_dir();
    let file_name = key.split('/').next_back().unwrap_or("clipboard_data");
    let unique_id = uuid::Uuid::new_v4();
    let temp_path = temp_dir.join(format!("s3explorer_clipboard_{}_{}", unique_id, file_name));

    // Write data to temp file
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    file.write_all(&data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    drop(file);

    let temp_path_str = temp_path.to_string_lossy().to_string();

    // Use existing upload_file command
    // Note: upload_file spawns a background task, so we cannot delete the temp file here.
    // The OS will clean up temp files eventually. Each file has a unique UUID to avoid conflicts.
    upload_file(
        app,
        profile_id,
        bucket,
        key,
        temp_path_str,
        Some(content_type),
        None, // Use default multipart threshold
        state,
    )
    .await
}

/// Read file paths from the system clipboard
/// Returns a list of file paths if the clipboard contains files
#[tauri::command]
pub async fn read_clipboard_files() -> Result<Vec<String>, String> {
    spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        {
            read_clipboard_files_macos()
        }
        #[cfg(target_os = "windows")]
        {
            read_clipboard_files_windows()
        }
        #[cfg(target_os = "linux")]
        {
            read_clipboard_files_linux()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            read_clipboard_files_fallback()
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// macOS-specific implementation using NSPasteboard
#[cfg(target_os = "macos")]
fn read_clipboard_files_macos() -> Result<Vec<String>, String> {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSString;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CStr;

    unsafe {
        // Get the general pasteboard
        let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
        if pasteboard == nil {
            println!("[clipboard] pasteboard is nil");
            return Ok(vec![]);
        }

        // Get available types for debugging
        let types: id = msg_send![pasteboard, types];
        if types != nil {
            let types_count: usize = msg_send![types, count];
            println!("[clipboard] Available types count: {}", types_count);
            for i in 0..types_count {
                let type_str: id = msg_send![types, objectAtIndex: i];
                if type_str != nil {
                    let c_str: *const i8 = msg_send![type_str, UTF8String];
                    if !c_str.is_null() {
                        if let Ok(s) = CStr::from_ptr(c_str).to_str() {
                            println!("[clipboard] Type {}: {}", i, s);
                        }
                    }
                }
            }
        }

        // Try NSFilenamesPboardType - this is what Finder uses when you Copy files
        let filenames_type: id = NSString::alloc(nil).init_str("NSFilenamesPboardType");
        let filenames: id = msg_send![pasteboard, propertyListForType: filenames_type];
        println!(
            "[clipboard] NSFilenamesPboardType filenames: {:?}",
            filenames != nil
        );

        if filenames != nil {
            let count: usize = msg_send![filenames, count];
            println!("[clipboard] NSFilenamesPboardType count: {}", count);
            if count > 0 {
                let mut paths = Vec::with_capacity(count);
                for i in 0..count {
                    let path: id = msg_send![filenames, objectAtIndex: i];
                    if path != nil {
                        let c_str: *const i8 = msg_send![path, UTF8String];
                        if !c_str.is_null() {
                            if let Ok(s) = CStr::from_ptr(c_str).to_str() {
                                let path_str = s.to_string();
                                println!(
                                    "[clipboard] Found path: {} (exists: {})",
                                    path_str,
                                    std::path::Path::new(&path_str).exists()
                                );
                                if std::path::Path::new(&path_str).exists() {
                                    paths.push(path_str);
                                }
                            }
                        }
                    }
                }
                if !paths.is_empty() {
                    println!(
                        "[clipboard] Returning {} paths from NSFilenamesPboardType",
                        paths.len()
                    );
                    return Ok(paths);
                }
            }
        }

        // Try public.file-url format as fallback
        let file_url_type: id = NSString::alloc(nil).init_str("public.file-url");
        let file_urls: id = msg_send![pasteboard, stringForType: file_url_type];
        println!("[clipboard] public.file-url: {:?}", file_urls != nil);

        if file_urls != nil {
            let c_str: *const i8 = msg_send![file_urls, UTF8String];
            if !c_str.is_null() {
                if let Ok(url_str) = CStr::from_ptr(c_str).to_str() {
                    println!("[clipboard] file-url value: {}", url_str);
                    // Parse file:// URL to path
                    if let Some(path) = url_str.strip_prefix("file://") {
                        // URL decode the path
                        let decoded = urlencoding_decode(path);
                        println!(
                            "[clipboard] Decoded path: {} (exists: {})",
                            decoded,
                            std::path::Path::new(&decoded).exists()
                        );
                        if std::path::Path::new(&decoded).exists() {
                            return Ok(vec![decoded]);
                        }
                    }
                }
            }
        }

        println!("[clipboard] No files found in clipboard");
        Ok(vec![])
    }
}

/// Windows-specific implementation using CF_HDROP format
#[cfg(target_os = "windows")]
fn read_clipboard_files_windows() -> Result<Vec<String>, String> {
    use clipboard_win::{formats, get_clipboard};

    // Try to get file list from clipboard (CF_HDROP format)
    match get_clipboard::<Vec<String>, _>(formats::FileList) {
        Ok(files) => {
            // Filter to only existing files
            let valid_files: Vec<String> = files
                .into_iter()
                .filter(|path| std::path::Path::new(path).exists())
                .collect();

            println!("[clipboard-win] Found {} files", valid_files.len());
            Ok(valid_files)
        }
        Err(e) => {
            println!("[clipboard-win] No files in clipboard: {}", e);
            // Fallback to text parsing
            read_clipboard_files_fallback()
        }
    }
}

/// Linux-specific implementation using text/uri-list format
#[cfg(target_os = "linux")]
fn read_clipboard_files_linux() -> Result<Vec<String>, String> {
    use std::process::Command;

    // Try xclip first (X11)
    let xclip_result = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "text/uri-list", "-o"])
        .output();

    if let Ok(output) = xclip_result {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                println!("[clipboard-linux] Found {} files via xclip", paths.len());
                return Ok(paths);
            }
        }
    }

    // Try xsel as fallback (X11)
    let xsel_result = Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output();

    if let Ok(output) = xsel_result {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                println!("[clipboard-linux] Found {} files via xsel", paths.len());
                return Ok(paths);
            }
        }
    }

    // Try wl-paste for Wayland
    let wl_result = Command::new("wl-paste")
        .args(["--type", "text/uri-list"])
        .output();

    if let Ok(output) = wl_result {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                println!("[clipboard-linux] Found {} files via wl-paste", paths.len());
                return Ok(paths);
            }
        }
    }

    // Fallback to arboard text
    read_clipboard_files_fallback()
}

/// Parse text/uri-list format into file paths
#[cfg(target_os = "linux")]
fn parse_uri_list(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.starts_with('#')) // Skip comments
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("file://") {
                // Remove file:// prefix and decode URL
                let path = line.strip_prefix("file://").unwrap();
                let decoded = urlencoding_decode(path);
                if std::path::Path::new(&decoded).exists() {
                    return Some(decoded);
                }
            }
            None
        })
        .collect()
}

/// Fallback implementation using arboard text
#[cfg(any(
    target_os = "linux",
    target_os = "windows",
    not(any(target_os = "macos", target_os = "windows", target_os = "linux"))
))]
fn read_clipboard_files_fallback() -> Result<Vec<String>, String> {
    use arboard::Clipboard;

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;

    // Try to get file list from clipboard (platform-specific)
    match clipboard.get_text() {
        Ok(text) => {
            let lines: Vec<String> = text
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    // Handle file:// URIs
                    let path = if line.starts_with("file://") {
                        line.strip_prefix("file://").map(|s| urlencoding_decode(s))
                    } else {
                        Some(line.to_string())
                    };

                    // Check if it's a valid file path
                    if let Some(p) = path {
                        if std::path::Path::new(&p).exists() && !p.is_empty() {
                            return Some(p);
                        }
                    }
                    None
                })
                .collect();

            Ok(lines)
        }
        Err(_) => Ok(vec![]),
    }
}

/// Simple URL decoding for file paths
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}
