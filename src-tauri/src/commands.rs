use crate::metrics::MetricsContext;
use crate::models::*;
use crate::profiles::ProfileStore;
use crate::s3_adapter::S3Adapter;
use crate::validation;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

// Upload task handle with metadata for cancellation
pub struct UploadTask {
    handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    file_name: String,
    file_size: u64,
}

// Global profile store state
pub struct AppState {
    pub profiles: Mutex<ProfileStore>,
    pub active_uploads: Arc<Mutex<HashMap<String, UploadTask>>>,
}

impl AppState {
    pub fn new() -> Self {
        let profiles = ProfileStore::load().unwrap_or_default();
        Self {
            profiles: Mutex::new(profiles),
            active_uploads: Arc::new(Mutex::new(HashMap::new())),
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

/// Calculate bucket statistics (size and count of all objects)
/// Note: This makes multiple ListObjectsV2 calls (one per 1000 objects)
#[tauri::command]
pub async fn calculate_bucket_stats(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(i64, i64), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let start_time = std::time::Instant::now();

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.calculate_bucket_stats(&bucket_name).await;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    match &result {
        Ok((_size, count, request_count)) => {
            // Emit one metric per API request made
            let avg_duration = duration_ms / (*request_count as u64).max(1);
            let objects_per_request = (*count as u32) / (*request_count).max(1);

            for _ in 0..*request_count {
                let event = S3MetricsEvent::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
                    .with_duration(avg_duration)
                    .with_profile(&profile.id, &profile.name)
                    .with_bucket(&bucket_name)
                    .with_objects_affected(objects_per_request);
                crate::metrics::emit_metrics(&app, event);
            }
        }
        Err(e) => {
            let ctx = MetricsContext::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket_name);
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map(|(size, count, _)| (size, count)).map_err(|e| e.to_string())
}

/// Estimate bucket statistics (fast - only first 1000 objects)
/// Returns (size, count, is_estimate)
#[tauri::command]
pub async fn estimate_bucket_stats(
    app: AppHandle,
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(i64, i64, bool), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let mut ctx = MetricsContext::new(S3Operation::ListObjectsV2, RequestCategory::LIST)
        .with_profile(&profile.id, &profile.name)
        .with_bucket(&bucket_name);

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let result = adapter.estimate_bucket_stats(&bucket_name).await;

    match &result {
        Ok((size, count, _)) => {
            ctx.set_bytes(*size as u64);
            ctx.set_objects_affected(*count as u32);
            ctx.emit_success(&app);
        }
        Err(e) => {
            ctx.emit_error(&app, &e.to_string());
        }
    }

    result.map_err(|e| e.to_string())
}

/// List objects in a bucket
#[tauri::command]
pub async fn list_objects(
    app: AppHandle,
    profile_id: String,
    bucket: String,
    prefix: Option<String>,
    continuation_token: Option<String>,
    max_keys: Option<i32>,
    use_delimiter: Option<bool>,
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
            continuation_token,
            max_keys,
            use_delimiter.unwrap_or(true),
        )
        .await;

    match &result {
        Ok(response) => {
            ctx.set_objects_affected(response.objects.len() as u32);
            ctx.emit_success(&app);
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

    let result = adapter.put_object(&bucket, &key, content, content_type).await;
    ctx.emit_result(&app, &result);

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

    let result = adapter.change_content_type(&bucket, &key, &new_content_type).await;
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
    let validated_path = validation::validate_folder_path(&folder_path).map_err(|e| e.to_string())?;

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
/// Note: This makes multiple ListObjectsV2 calls (one per 1000 objects)
#[tauri::command]
pub async fn calculate_folder_size(
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

    result.map(|(size, _)| size).map_err(|e| e.to_string())
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
                let event = S3MetricsEvent::new(S3Operation::DeleteObjects, RequestCategory::DELETE)
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
        let init_ctx = MetricsContext::new(S3Operation::CreateMultipartUpload, RequestCategory::PUT)
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

        for part_number in 1..=total_parts {
            // Check for cancellation before each part
            if cancel_rx.try_recv().is_ok() {
                // Abort the multipart upload with metrics
                let abort_ctx = MetricsContext::new(S3Operation::AbortMultipartUpload, RequestCategory::PUT)
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

            // Read chunk from file
            let mut file =
                File::open(&file_path).map_err(|e| format!("Failed to open file: {}", e))?;

            file.seek(SeekFrom::Start(offset))
                .map_err(|e| format!("Failed to seek file: {}", e))?;

            let mut buffer = vec![0u8; length as usize];
            file.read_exact(&mut buffer)
                .map_err(|e| format!("Failed to read file chunk: {}", e))?;

            // Upload part with metrics
            let mut part_ctx = MetricsContext::new(S3Operation::UploadPart, RequestCategory::PUT)
                .with_profile(&profile.id, &profile.name)
                .with_bucket(&bucket)
                .with_object_key(&key);
            part_ctx.set_bytes(length);

            let part_result = adapter
                .multipart_upload_part(&bucket, &key, &s3_upload_id, part_number, buffer)
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

        // Complete multipart upload with metrics
        let complete_ctx = MetricsContext::new(S3Operation::CompleteMultipartUpload, RequestCategory::PUT)
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
