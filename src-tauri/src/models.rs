use serde::{Deserialize, Serialize};

/// S3 connection profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub endpoint: Option<String>, // Custom endpoint URL (None = use AWS default)
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
    pub path_style: bool, // Force path-style addressing
    pub use_tls: bool,    // Use HTTPS (false for local MinIO)
}

/// Request to test a connection
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub profile: Profile,
}

/// Response from testing a connection
#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub bucket_count: Option<usize>,
    pub suggest_path_style: Option<bool>, // Suggest enabling path_style if connection failed without it
}

/// S3 Bucket information
#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub creation_date: Option<String>,
}

/// S3 Object information
#[derive(Debug, Serialize, Deserialize)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
    pub storage_class: Option<String>,
    pub e_tag: Option<String>,
    pub is_folder: bool,
}

/// Request to list objects
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ListObjectsRequest {
    pub profile_id: String,
    pub bucket: String,
    pub prefix: Option<String>,
    pub continuation_token: Option<String>,
    pub max_keys: Option<i32>,
}

/// Response from listing objects
#[derive(Debug, Serialize)]
pub struct ListObjectsResponse {
    pub objects: Vec<S3Object>,
    pub common_prefixes: Vec<String>, // Folders
    pub continuation_token: Option<String>,
    pub is_truncated: bool,
}

/// Request to get an object
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GetObjectRequest {
    pub profile_id: String,
    pub bucket: String,
    pub key: String,
}

/// Response from getting an object
#[derive(Debug, Serialize)]
pub struct GetObjectResponse {
    pub content: Vec<u8>,
    pub content_type: Option<String>,
    pub size: i64,
}

/// Request to put an object
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PutObjectRequest {
    pub profile_id: String,
    pub bucket: String,
    pub key: String,
    pub content: Vec<u8>,
    pub content_type: Option<String>,
}

/// Request to delete an object
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct DeleteObjectRequest {
    pub profile_id: String,
    pub bucket: String,
    pub key: String,
}

/// Request to copy an object
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CopyObjectRequest {
    pub profile_id: String,
    pub source_bucket: String,
    pub source_key: String,
    pub dest_bucket: String,
    pub dest_key: String,
}

/// Request to create a folder (empty object with trailing slash)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub profile_id: String,
    pub bucket: String,
    pub folder_path: String,
}

/// Request to generate a presigned URL
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PresignedUrlRequest {
    pub profile_id: String,
    pub bucket: String,
    pub key: String,
    pub method: String, // "GET" or "PUT"
    pub expires_in_secs: u64,
}

/// Response with presigned URL
#[derive(Debug, Serialize)]
pub struct PresignedUrlResponse {
    pub url: String,
    pub expires_in_secs: u64,
}

/// S3 Object Version information
#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectVersion {
    pub version_id: String,
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
    pub is_latest: bool,
    pub e_tag: Option<String>,
}

/// Response from listing object versions
#[derive(Debug, Serialize)]
pub struct ListObjectVersionsResponse {
    pub versions: Vec<ObjectVersion>,
}

/// Response from initiating multipart upload
#[derive(Debug, Serialize)]
pub struct MultipartUploadInitResponse {
    pub upload_id: String,
}

/// Completed part information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedPart {
    pub part_number: i32,
    pub e_tag: String,
}

/// Response from uploading a part
#[derive(Debug, Serialize)]
pub struct MultipartUploadPartResponse {
    pub e_tag: String,
}

/// Upload progress event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgressEvent {
    pub upload_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub uploaded_bytes: u64,
    pub uploaded_parts: i32,
    pub total_parts: i32,
    pub percentage: f64,
    pub status: UploadStatus,
    pub error: Option<String>,
}

/// Upload status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadStatus {
    Pending,
    Starting,
    Uploading,
    Completed,
    Failed,
    Cancelled,
}
