use crate::models::*;
use crate::profiles::ProfileStore;
use crate::s3_adapter::S3Adapter;
use std::sync::Mutex;
use tauri::State;

// Global profile store state
pub struct AppState {
    pub profiles: Mutex<ProfileStore>,
}

impl AppState {
    pub fn new() -> Self {
        let profiles = ProfileStore::load().unwrap_or_default();
        Self {
            profiles: Mutex::new(profiles),
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
pub async fn test_connection(profile: Profile) -> Result<TestConnectionResponse, String> {
    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter.test_connection().await.map_err(|e| e.to_string())
}

/// List buckets for a profile
#[tauri::command]
pub async fn list_buckets(profile_id: String, state: State<'_, AppState>) -> Result<Vec<Bucket>, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    }; // Lock is dropped here

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter.list_buckets().await.map_err(|e| e.to_string())
}

/// Create a new bucket
#[tauri::command]
pub async fn create_bucket(
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .create_bucket(&bucket_name)
        .await
        .map_err(|e| e.to_string())
}

/// Get bucket ACL (Public/Private)
#[tauri::command]
pub async fn get_bucket_acl(
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .get_bucket_acl(&bucket_name)
        .await
        .map_err(|e| e.to_string())
}

/// Calculate bucket statistics (size and count of all objects)
#[tauri::command]
pub async fn calculate_bucket_stats(
    profile_id: String,
    bucket_name: String,
    state: State<'_, AppState>,
) -> Result<(i64, i64), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .calculate_bucket_stats(&bucket_name)
        .await
        .map_err(|e| e.to_string())
}

/// List objects in a bucket
#[tauri::command]
pub async fn list_objects(
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

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .list_objects(&bucket, prefix.as_deref(), continuation_token, max_keys, use_delimiter.unwrap_or(true))
        .await
        .map_err(|e| e.to_string())
}

/// Get an object's content
#[tauri::command]
pub async fn get_object(
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<GetObjectResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .get_object(&bucket, &key)
        .await
        .map_err(|e| e.to_string())
}

/// Upload an object
#[tauri::command]
pub async fn put_object(
    profile_id: String,
    bucket: String,
    key: String,
    content: Vec<u8>,
    content_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .put_object(&bucket, &key, content, content_type)
        .await
        .map_err(|e| e.to_string())
}

/// Delete an object
#[tauri::command]
pub async fn delete_object(
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .delete_object(&bucket, &key)
        .await
        .map_err(|e| e.to_string())
}

/// Copy an object
#[tauri::command]
pub async fn copy_object(
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

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .copy_object(&source_bucket, &source_key, &dest_bucket, &dest_key)
        .await
        .map_err(|e| e.to_string())
}

/// Create a folder (empty object with trailing slash)
#[tauri::command]
pub async fn create_folder(
    profile_id: String,
    bucket: String,
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .create_folder(&bucket, &folder_path)
        .await
        .map_err(|e| e.to_string())
}

/// Generate a presigned URL
#[tauri::command]
pub async fn generate_presigned_url(
    profile_id: String,
    bucket: String,
    key: String,
    method: String,
    expires_in_secs: u64,
    state: State<'_, AppState>,
) -> Result<PresignedUrlResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let url = adapter
        .generate_presigned_url(&bucket, &key, &method, expires_in_secs)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PresignedUrlResponse {
        url,
        expires_in_secs,
    })
}

/// Calculate folder size by summing all objects in the prefix
#[tauri::command]
pub async fn calculate_folder_size(
    profile_id: String,
    bucket: String,
    prefix: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    let mut total_size: i64 = 0;
    let mut continuation_token: Option<String> = None;

    // Paginate through all objects in the folder
    loop {
        let response = adapter
            .list_objects(&bucket, Some(&prefix), continuation_token, Some(1000), false)
            .await
            .map_err(|e| e.to_string())?;

        // Sum up object sizes
        for obj in response.objects {
            total_size += obj.size;
        }

        // Check if there are more pages
        continuation_token = response.continuation_token;
        if continuation_token.is_none() {
            break;
        }
    }

    Ok(total_size)
}

/// Delete a folder and all its contents
#[tauri::command]
pub async fn delete_folder(
    profile_id: String,
    bucket: String,
    prefix: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .delete_folder(&bucket, &prefix)
        .await
        .map_err(|e| e.to_string())
}

/// List all versions of an object
#[tauri::command]
pub async fn list_object_versions(
    profile_id: String,
    bucket: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<ListObjectVersionsResponse, String> {
    let profile = {
        let store = state.profiles.lock().map_err(|e| e.to_string())?;
        store.get(&profile_id).map_err(|e| e.to_string())?
    };

    let adapter = S3Adapter::from_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    adapter
        .list_object_versions(&bucket, &key)
        .await
        .map_err(|e| e.to_string())
}
