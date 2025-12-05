use crate::errors::AppError;
use crate::models::{
    Bucket, BucketConfigurationResponse, BucketCorsResponse, BucketEncryptionResponse,
    BucketEncryptionRule, BucketLifecycleResponse, BucketPolicyResponse, BucketVersioningResponse,
    CompletedPart, CorsRule, DeleteObjectError, DeleteObjectsResponse, GetObjectMetadataResponse,
    GetObjectResponse, LifecycleRule, LifecycleTransition, ListObjectVersionsResponse,
    ListObjectsResponse, MultipartUploadInitResponse, MultipartUploadPartResponse,
    ObjectLockStatus, ObjectTag, ObjectVersion, Profile, S3Object, TestConnectionResponse,
};
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{BucketLocationConstraint, CreateBucketConfiguration};
use aws_sdk_s3::Client;
use std::time::Duration;

/// S3 Client Adapter that supports multiple providers
pub struct S3Adapter {
    client: Client,
    region: String,
}

impl S3Adapter {
    /// Create a new S3 adapter from a profile
    pub async fn from_profile(profile: &Profile) -> Result<Self, AppError> {
        let credentials = Credentials::new(
            &profile.access_key,
            &profile.secret_key,
            profile.session_token.clone(),
            None,
            "s3explorer",
        );

        // Use provided region or default to "us-east-1"
        let region_str = profile
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string());
        let region = Region::new(region_str.clone());
        let region_provider = RegionProviderChain::first_try(region);

        // Build base AWS config
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .credentials_provider(credentials)
            .load()
            .await;

        // Build S3-specific config
        let mut s3_config_builder = S3ConfigBuilder::from(&aws_config)
            // Increase timeouts for large file uploads
            .timeout_config(
                aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                    .connect_timeout(Duration::from_secs(10)) // Connection timeout
                    .read_timeout(Duration::from_secs(300)) // Read timeout (5 minutes)
                    .operation_timeout(Duration::from_secs(600)) // Total operation timeout (10 minutes)
                    .build(),
            );

        // Custom endpoint support
        if let Some(endpoint) = &profile.endpoint {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }

        // Force path-style addressing (required for MinIO and some S3-compatible services)
        if profile.path_style {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            region: region_str,
        })
    }

    /// Test connection by listing buckets
    /// If connection fails and path_style is not enabled, tries again with path_style
    /// Timeout: 30 seconds
    pub async fn test_connection_with_profile(
        profile: &Profile,
    ) -> Result<TestConnectionResponse, AppError> {
        use tokio::time::{timeout, Duration};

        let adapter = Self::from_profile(profile).await?;

        // Test connection with 30-second timeout
        let timeout_duration = Duration::from_secs(30);
        let result = timeout(timeout_duration, adapter.client.list_buckets().send()).await;

        match result {
            // Timeout elapsed
            Err(_) => {
                Ok(TestConnectionResponse {
                    success: false,
                    message: "Connection timed out after 30 seconds. Please check your endpoint and network connection.".to_string(),
                    bucket_count: None,
                    suggest_path_style: None,
                })
            }
            // Request completed within timeout
            Ok(Ok(output)) => {
                let bucket_count = output.buckets().len();
                Ok(TestConnectionResponse {
                    success: true,
                    message: format!("Successfully connected. Found {} buckets.", bucket_count),
                    bucket_count: Some(bucket_count),
                    suggest_path_style: None,
                })
            }
            // Request completed but failed
            Ok(Err(e)) => {
                let error_str = e.to_string();

                // If path_style is not enabled and we have a custom endpoint, try with path_style
                if !profile.path_style && profile.endpoint.is_some() {
                    // Create a modified profile with path_style enabled
                    let mut path_style_profile = profile.clone();
                    path_style_profile.path_style = true;

                    // Try again with path_style enabled (also with timeout)
                    let path_style_adapter = Self::from_profile(&path_style_profile).await?;
                    let retry_result = timeout(timeout_duration, path_style_adapter.client.list_buckets().send()).await;

                    match retry_result {
                        Ok(Ok(output)) => {
                            // Success with path_style! Suggest enabling it
                            let bucket_count = output.buckets().len();
                            Ok(TestConnectionResponse {
                                success: false,
                                message: format!("Connection failed without path-style, but succeeded with path-style enabled. Found {} buckets. Please enable 'Force Path Style' option.", bucket_count),
                                bucket_count: Some(bucket_count),
                                suggest_path_style: Some(true),
                            })
                        }
                        _ => {
                            // Failed with both, return original error
                            Ok(TestConnectionResponse {
                                success: false,
                                message: format!("Connection failed: {}", error_str),
                                bucket_count: None,
                                suggest_path_style: None,
                            })
                        }
                    }
                } else {
                    // No retry, just return the error
                    Ok(TestConnectionResponse {
                        success: false,
                        message: format!("Connection failed: {}", error_str),
                        bucket_count: None,
                        suggest_path_style: None,
                    })
                }
            }
        }
    }

    /// List all buckets
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>, AppError> {
        let output = self
            .client
            .list_buckets()
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to list buckets: {}", e)))?;

        let buckets = output
            .buckets()
            .iter()
            .map(|b| Bucket {
                name: b.name().unwrap_or("").to_string(),
                creation_date: b.creation_date().map(|d| d.to_string()),
            })
            .collect();

        Ok(buckets)
    }

    /// Create a new bucket
    pub async fn create_bucket(&self, bucket_name: &str) -> Result<(), AppError> {
        let mut request = self.client.create_bucket().bucket(bucket_name);

        // AWS S3 requires CreateBucketConfiguration for regions other than us-east-1
        // MinIO also requires this for proper bucket creation
        if self.region != "us-east-1" {
            let location_constraint = BucketLocationConstraint::from(self.region.as_str());
            let bucket_config = CreateBucketConfiguration::builder()
                .location_constraint(location_constraint)
                .build();
            request = request.create_bucket_configuration(bucket_config);
        }

        request
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to create bucket: {}", e)))?;

        Ok(())
    }

    /// Delete a bucket (must be empty)
    pub async fn delete_bucket(&self, bucket_name: &str) -> Result<(), AppError> {
        self.client
            .delete_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|e| {
                // Try to extract the S3 error code for better error messages
                let error_str = e.to_string();

                // Check for common S3 error codes
                if error_str.contains("BucketNotEmpty") {
                    AppError::S3Error(
                        "BucketNotEmpty: The bucket is not empty. Delete all objects first."
                            .to_string(),
                    )
                } else if error_str.contains("AccessDenied") {
                    AppError::S3Error(
                        "AccessDenied: You don't have permission to delete this bucket."
                            .to_string(),
                    )
                } else if error_str.contains("NoSuchBucket") {
                    AppError::S3Error("NoSuchBucket: The bucket does not exist.".to_string())
                } else {
                    // For other errors, include the full error message
                    AppError::S3Error(format!("Failed to delete bucket: {}", error_str))
                }
            })?;

        Ok(())
    }

    /// Check if user has permission to delete a bucket
    /// Uses HEAD bucket request - if we can access the bucket, we likely have admin access
    /// Note: This is a heuristic - actual delete permission depends on IAM policies
    /// For MinIO admin users, this should always return true
    pub async fn can_delete_bucket(&self, bucket_name: &str) -> bool {
        // Try HEAD bucket - if we can do this, we have at least read access
        // For most S3-compatible systems, if you can list/access the bucket,
        // and you're an admin user, you can delete it
        let head_result = self.client.head_bucket().bucket(bucket_name).send().await;

        match head_result {
            Ok(_) => true, // Bucket exists and we can access it
            Err(e) => {
                let error_str = e.to_string();
                // 404 means bucket doesn't exist - can't delete what doesn't exist
                // 403 means access denied - no permission
                !error_str.contains("404") && !error_str.contains("AccessDenied")
            }
        }
    }

    /// Get bucket ACL to determine if it's public or private
    pub async fn get_bucket_acl(&self, bucket_name: &str) -> Result<String, AppError> {
        let acl = self
            .client
            .get_bucket_acl()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to get bucket ACL: {}", e)))?;

        // Check if bucket has public grants
        let is_public = acl.grants().iter().any(|grant| {
            if let Some(grantee) = grant.grantee() {
                if let Some(uri) = grantee.uri() {
                    // Check for AllUsers or AuthenticatedUsers groups
                    return uri.contains("AllUsers") || uri.contains("AuthenticatedUsers");
                }
            }
            false
        });

        Ok(if is_public {
            "Public".to_string()
        } else {
            "Private".to_string()
        })
    }

    /// Calculate folder size by listing ALL objects with the given prefix (including all subdirectories recursively)
    /// This method skips folder markers (objects ending with '/') as they are zero-byte placeholders
    /// Returns (total_size, request_count) where request_count is the number of API calls made
    pub async fn calculate_folder_size(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<(i64, u32), AppError> {
        let mut total_size: i64 = 0;
        let mut request_count: u32 = 0;
        let mut continuation_token: Option<String> = None;

        // Paginate through ALL objects with the given prefix (no delimiter = recursive)
        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(bucket_name)
                .prefix(prefix)
                .max_keys(1000);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let output = request
                .send()
                .await
                .map_err(|e| AppError::S3Error(format!("Failed to list objects: {}", e)))?;

            request_count += 1;

            // Sum up object sizes (skip folder markers)
            for obj in output.contents() {
                if let Some(key) = obj.key() {
                    if !key.ends_with('/') {
                        total_size += obj.size().unwrap_or(0);
                    }
                }
            }

            // Check if there are more pages
            continuation_token = output.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        }

        Ok((total_size, request_count))
    }

    /// List objects in a bucket with optional prefix
    pub async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        continuation_token: Option<String>,
        max_keys: Option<i32>,
        use_delimiter: bool,
    ) -> Result<ListObjectsResponse, AppError> {
        let mut request = self.client.list_objects_v2().bucket(bucket);

        if let Some(p) = prefix {
            request = request.prefix(p);
        }

        // Only use delimiter if requested (for folder navigation)
        if use_delimiter {
            request = request.delimiter("/");
        }

        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }

        // Set max_keys to respect user's batch size configuration
        // S3's max_keys limits the TOTAL count of objects + common_prefixes (folders)
        // If folders are not visible in the first batch, user can use "Load More"
        if let Some(max) = max_keys {
            request = request.max_keys(max);
        }

        let output = request
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to list objects: {}", e)))?;

        let objects = output
            .contents()
            .iter()
            .map(|obj| S3Object {
                key: obj.key().unwrap_or("").to_string(),
                size: obj.size().unwrap_or(0),
                last_modified: obj.last_modified().map(|d| d.to_string()),
                storage_class: obj.storage_class().map(|s| s.as_str().to_string()),
                e_tag: obj.e_tag().map(|e| e.to_string()),
                is_folder: false,
            })
            .collect();

        let common_prefixes = output
            .common_prefixes()
            .iter()
            .filter_map(|cp| cp.prefix().map(|s| s.to_string()))
            .collect();

        Ok(ListObjectsResponse {
            objects,
            common_prefixes,
            continuation_token: output.next_continuation_token().map(|s| s.to_string()),
            is_truncated: output.is_truncated().unwrap_or(false),
        })
    }

    /// Get an object's content
    pub async fn get_object(&self, bucket: &str, key: &str) -> Result<GetObjectResponse, AppError> {
        let output = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to get object: {}", e)))?;

        // Extract metadata before consuming body
        let content_type = output.content_type().map(|s| s.to_string());
        let size = output.content_length().unwrap_or(0);

        let content = output
            .body
            .collect()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to read object body: {}", e)))?
            .into_bytes()
            .to_vec();

        Ok(GetObjectResponse {
            content,
            content_type,
            size,
        })
    }

    /// Get an object's body as a stream (for streaming downloads without buffering)
    /// Returns the ByteStream and file size
    /// If version_id is provided, downloads that specific version
    pub async fn get_object_stream(
        &self,
        bucket: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<(ByteStream, u64), AppError> {
        let mut request = self.client.get_object().bucket(bucket).key(key);

        if let Some(vid) = version_id {
            request = request.version_id(vid);
        }

        let output = request
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to get object: {}", e)))?;

        let size = output.content_length().unwrap_or(0) as u64;
        Ok((output.body, size))
    }

    /// Get object size without downloading content (uses HEAD request)
    /// If version_id is provided, gets size of that specific version
    pub async fn get_object_size(
        &self,
        bucket: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<u64, AppError> {
        let mut request = self.client.head_object().bucket(bucket).key(key);

        if let Some(vid) = version_id {
            request = request.version_id(vid);
        }

        let output = request
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to head object: {}", e)))?;

        Ok(output.content_length().unwrap_or(0) as u64)
    }

    /// Upload an object
    pub async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        content: Vec<u8>,
        content_type: Option<String>,
    ) -> Result<(), AppError> {
        let mut request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(content));

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        request
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to put object: {}", e)))?;

        Ok(())
    }

    /// Delete an object
    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to delete object: {}", e)))?;

        Ok(())
    }

    /// Delete a specific version of an object
    /// This is a PERMANENT deletion - the version cannot be recovered
    pub async fn delete_object_version(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .version_id(version_id)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to delete object version: {}", e)))?;

        Ok(())
    }

    /// Delete multiple objects in a single request (batch delete)
    /// S3 supports up to 1000 objects per DeleteObjects request
    /// Returns the count of successfully deleted objects and any errors
    pub async fn delete_objects_batch(
        &self,
        bucket: &str,
        keys: Vec<String>,
    ) -> Result<DeleteObjectsResponse, AppError> {
        use aws_sdk_s3::types::{Delete, ObjectIdentifier};

        if keys.is_empty() {
            return Ok(DeleteObjectsResponse {
                deleted_count: 0,
                error_count: 0,
                errors: vec![],
            });
        }

        // Build object identifiers
        let objects: Vec<ObjectIdentifier> = keys
            .into_iter()
            .filter_map(|key| ObjectIdentifier::builder().key(key).build().ok())
            .collect();

        let delete = Delete::builder()
            .set_objects(Some(objects))
            .quiet(false) // Get detailed response with deleted/error info
            .build()
            .map_err(|e| AppError::S3Error(format!("Failed to build delete request: {}", e)))?;

        let result = self
            .client
            .delete_objects()
            .bucket(bucket)
            .delete(delete)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to delete objects: {}", e)))?;

        let deleted_count = result.deleted().len();
        let errors: Vec<DeleteObjectError> = result
            .errors()
            .iter()
            .map(|e| DeleteObjectError {
                key: e.key().unwrap_or("").to_string(),
                code: e.code().map(|s| s.to_string()),
                message: e.message().map(|s| s.to_string()),
            })
            .collect();

        Ok(DeleteObjectsResponse {
            deleted_count,
            error_count: errors.len(),
            errors,
        })
    }

    /// Copy an object
    pub async fn copy_object(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_bucket: &str,
        dest_key: &str,
    ) -> Result<(), AppError> {
        let copy_source = format!("{}/{}", source_bucket, source_key);

        self.client
            .copy_object()
            .copy_source(&copy_source)
            .bucket(dest_bucket)
            .key(dest_key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to copy object: {}", e)))?;

        Ok(())
    }

    /// Change the content-type of an object by copying it to itself with new metadata
    pub async fn change_content_type(
        &self,
        bucket: &str,
        key: &str,
        new_content_type: &str,
    ) -> Result<(), AppError> {
        let copy_source = format!("{}/{}", bucket, key);

        self.client
            .copy_object()
            .copy_source(&copy_source)
            .bucket(bucket)
            .key(key)
            .content_type(new_content_type)
            .metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to change content-type: {}", e)))?;

        Ok(())
    }

    /// Create a folder (zero-byte object with trailing slash)
    pub async fn create_folder(&self, bucket: &str, folder_path: &str) -> Result<(), AppError> {
        let key = if folder_path.ends_with('/') {
            folder_path.to_string()
        } else {
            format!("{}/", folder_path)
        };

        self.put_object(bucket, &key, vec![], None).await
    }

    /// Generate a presigned URL
    pub async fn generate_presigned_url(
        &self,
        bucket: &str,
        key: &str,
        method: &str,
        expires_in_secs: u64,
    ) -> Result<String, AppError> {
        let expires_in = Duration::from_secs(expires_in_secs);
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| AppError::S3Error(format!("Invalid expiration: {}", e)))?;

        let url = match method.to_uppercase().as_str() {
            "GET" => {
                let presigned = self
                    .client
                    .get_object()
                    .bucket(bucket)
                    .key(key)
                    .presigned(presigning_config)
                    .await
                    .map_err(|e| {
                        AppError::S3Error(format!("Failed to generate presigned URL: {}", e))
                    })?;
                presigned.uri().to_string()
            }
            "PUT" => {
                let presigned = self
                    .client
                    .put_object()
                    .bucket(bucket)
                    .key(key)
                    .presigned(presigning_config)
                    .await
                    .map_err(|e| {
                        AppError::S3Error(format!("Failed to generate presigned URL: {}", e))
                    })?;
                presigned.uri().to_string()
            }
            _ => {
                return Err(AppError::ConfigError(format!(
                    "Unsupported method: {}",
                    method
                )))
            }
        };

        Ok(url)
    }

    /// Delete a folder and all its contents using batch delete (DeleteObjects API)
    /// This is ~99% more efficient than deleting objects one by one
    /// S3 DeleteObjects supports up to 1000 objects per request
    /// Returns (deleted_count, list_request_count, delete_request_count)
    pub async fn delete_folder(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<(i64, u32, u32), AppError> {
        let folder_prefix = if prefix.ends_with('/') {
            prefix.to_string()
        } else {
            format!("{}/", prefix)
        };

        let mut deleted_count: i64 = 0;
        let mut total_errors: i64 = 0;
        let mut list_request_count: u32 = 0;
        let mut delete_request_count: u32 = 0;
        let mut continuation_token: Option<String> = None;

        // Paginate through all objects in the folder (without delimiter to get all nested objects)
        loop {
            let response = self
                .list_objects(
                    bucket,
                    Some(&folder_prefix),
                    continuation_token,
                    Some(1000), // Max batch size for DeleteObjects
                    false,
                )
                .await?;

            list_request_count += 1;

            // Collect keys for batch deletion
            let keys: Vec<String> = response.objects.iter().map(|obj| obj.key.clone()).collect();

            if !keys.is_empty() {
                // Use batch delete (1 request instead of N requests)
                let delete_result = self.delete_objects_batch(bucket, keys).await?;
                delete_request_count += 1;
                deleted_count += delete_result.deleted_count as i64;
                total_errors += delete_result.error_count as i64;

                // Log errors if any (but don't fail the entire operation)
                if !delete_result.errors.is_empty() {
                    for err in &delete_result.errors {
                        eprintln!(
                            "Warning: Failed to delete {}: {} - {}",
                            err.key,
                            err.code.as_deref().unwrap_or("Unknown"),
                            err.message.as_deref().unwrap_or("No message")
                        );
                    }
                }
            }

            // Check if there are more pages
            continuation_token = response.continuation_token;
            if continuation_token.is_none() {
                break;
            }
        }

        // Also delete the folder marker itself (if it exists)
        // Use batch delete for consistency (single item batch is fine)
        let _ = self.delete_objects_batch(bucket, vec![folder_prefix]).await;
        delete_request_count += 1;

        // Return total deleted count (errors are logged but don't fail the operation)
        if total_errors > 0 {
            eprintln!(
                "Warning: {} objects could not be deleted out of {} total",
                total_errors,
                deleted_count + total_errors
            );
        }

        Ok((deleted_count, list_request_count, delete_request_count))
    }

    /// List all versions of an object
    pub async fn list_object_versions(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<ListObjectVersionsResponse, AppError> {
        let output = self
            .client
            .list_object_versions()
            .bucket(bucket)
            .prefix(key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to list object versions: {}", e)))?;

        let versions = output
            .versions()
            .iter()
            .filter(|v| v.key() == Some(key)) // Only get versions for the exact key
            .map(|v| ObjectVersion {
                version_id: v.version_id().unwrap_or("null").to_string(),
                key: v.key().unwrap_or("").to_string(),
                size: v.size().unwrap_or(0),
                last_modified: v.last_modified().map(|d| d.to_string()),
                is_latest: v.is_latest().unwrap_or(false),
                e_tag: v.e_tag().map(|e| e.to_string()),
            })
            .collect();

        Ok(ListObjectVersionsResponse { versions })
    }

    /// Initiate a multipart upload
    pub async fn multipart_upload_start(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
    ) -> Result<MultipartUploadInitResponse, AppError> {
        let mut request = self
            .client
            .create_multipart_upload()
            .bucket(bucket)
            .key(key);

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        let output = request.send().await.map_err(|e| {
            AppError::S3Error(format!("Failed to initiate multipart upload: {}", e))
        })?;

        let upload_id = output
            .upload_id()
            .ok_or_else(|| AppError::S3Error("No upload ID returned".to_string()))?
            .to_string();

        Ok(MultipartUploadInitResponse { upload_id })
    }

    /// Upload a single part
    pub async fn multipart_upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Vec<u8>,
    ) -> Result<MultipartUploadPartResponse, AppError> {
        let output = self
            .client
            .upload_part()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| {
                AppError::S3Error(format!("Failed to upload part {}: {}", part_number, e))
            })?;

        let e_tag = output
            .e_tag()
            .ok_or_else(|| AppError::S3Error(format!("No ETag returned for part {}", part_number)))?
            .to_string();

        Ok(MultipartUploadPartResponse { e_tag })
    }

    /// Complete a multipart upload
    pub async fn multipart_upload_complete(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<(), AppError> {
        use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart as S3CompletedPart};

        // Convert our CompletedPart to AWS SDK's CompletedPart
        let completed_parts: Vec<S3CompletedPart> = parts
            .iter()
            .map(|p| {
                S3CompletedPart::builder()
                    .part_number(p.part_number)
                    .e_tag(&p.e_tag)
                    .build()
            })
            .collect();

        let multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(multipart_upload)
            .send()
            .await
            .map_err(|e| {
                AppError::S3Error(format!("Failed to complete multipart upload: {}", e))
            })?;

        Ok(())
    }

    /// Abort a multipart upload
    pub async fn multipart_upload_abort(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<(), AppError> {
        self.client
            .abort_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to abort multipart upload: {}", e)))?;

        Ok(())
    }

    /// Get object tagging
    pub async fn get_object_tagging(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Vec<ObjectTag>, AppError> {
        let output = self
            .client
            .get_object_tagging()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to get object tags: {}", e)))?;

        let tags = output
            .tag_set()
            .iter()
            .map(|tag| ObjectTag {
                key: tag.key().to_string(),
                value: tag.value().to_string(),
            })
            .collect();

        Ok(tags)
    }

    /// Put object tagging
    pub async fn put_object_tagging(
        &self,
        bucket: &str,
        key: &str,
        tags: Vec<ObjectTag>,
    ) -> Result<(), AppError> {
        use aws_sdk_s3::types::{Tag, Tagging};

        let s3_tags: Result<Vec<Tag>, _> = tags
            .iter()
            .map(|tag| {
                Tag::builder()
                    .key(&tag.key)
                    .value(&tag.value)
                    .build()
                    .map_err(|e| AppError::S3Error(format!("Failed to build tag: {}", e)))
            })
            .collect();
        let s3_tags = s3_tags?;

        let tagging = Tagging::builder()
            .set_tag_set(Some(s3_tags))
            .build()
            .map_err(|e| AppError::S3Error(format!("Failed to build tagging: {}", e)))?;

        self.client
            .put_object_tagging()
            .bucket(bucket)
            .key(key)
            .tagging(tagging)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to put object tags: {}", e)))?;

        Ok(())
    }

    /// Delete object tagging
    pub async fn delete_object_tagging(&self, bucket: &str, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object_tagging()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to delete object tags: {}", e)))?;

        Ok(())
    }

    /// Get object metadata (HTTP headers)
    pub async fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<GetObjectMetadataResponse, AppError> {
        use std::collections::HashMap;

        let output = self
            .client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to get object metadata: {}", e)))?;

        // Extract custom metadata (x-amz-meta-* headers)
        let metadata: HashMap<String, String> = output.metadata().cloned().unwrap_or_default();

        Ok(GetObjectMetadataResponse {
            content_type: output.content_type().map(|s| s.to_string()),
            content_encoding: output.content_encoding().map(|s| s.to_string()),
            content_language: output.content_language().map(|s| s.to_string()),
            content_disposition: output.content_disposition().map(|s| s.to_string()),
            cache_control: output.cache_control().map(|s| s.to_string()),
            expires: output.expires_string().map(|s| s.to_string()),
            metadata,
        })
    }

    /// Update object metadata (HTTP headers) by copying the object to itself
    #[allow(clippy::too_many_arguments)]
    pub async fn update_object_metadata(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
        content_encoding: Option<String>,
        content_language: Option<String>,
        content_disposition: Option<String>,
        cache_control: Option<String>,
        expires: Option<String>,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<(), AppError> {
        use aws_sdk_s3::primitives::DateTime;

        let copy_source = format!("{}/{}", bucket, key);

        let mut request = self
            .client
            .copy_object()
            .bucket(bucket)
            .key(key)
            .copy_source(&copy_source)
            .metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace);

        // Set standard headers
        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }
        if let Some(ce) = content_encoding {
            request = request.content_encoding(ce);
        }
        if let Some(cl) = content_language {
            request = request.content_language(cl);
        }
        if let Some(cd) = content_disposition {
            request = request.content_disposition(cd);
        }
        if let Some(cc) = cache_control {
            request = request.cache_control(cc);
        }
        if let Some(exp) = expires {
            // Try to parse the expires string as DateTime
            if let Ok(dt) =
                DateTime::from_str(&exp, aws_sdk_s3::primitives::DateTimeFormat::HttpDate)
            {
                request = request.expires(dt);
            }
        }

        // Set custom metadata
        for (k, v) in metadata {
            request = request.metadata(k, v);
        }

        request
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to update object metadata: {}", e)))?;

        Ok(())
    }

    // ========================================================================
    // Bucket Configuration Methods (Read-Only)
    // ========================================================================

    /// Get bucket policy (JSON document)
    pub async fn get_bucket_policy(&self, bucket_name: &str) -> BucketPolicyResponse {
        match self
            .client
            .get_bucket_policy()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(output) => BucketPolicyResponse {
                policy: output.policy,
                error: None,
            },
            Err(e) => {
                let err_str = e.to_string();
                // NoSuchBucketPolicy means no policy is set - not an error
                if err_str.contains("NoSuchBucketPolicy") || err_str.contains("NoSuchPolicy") {
                    BucketPolicyResponse {
                        policy: None,
                        error: None,
                    }
                } else {
                    BucketPolicyResponse {
                        policy: None,
                        error: Some(err_str),
                    }
                }
            }
        }
    }

    /// Get bucket CORS configuration
    pub async fn get_bucket_cors(&self, bucket_name: &str) -> BucketCorsResponse {
        match self
            .client
            .get_bucket_cors()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(output) => {
                let rules = output
                    .cors_rules()
                    .iter()
                    .map(|rule| CorsRule {
                        allowed_headers: rule
                            .allowed_headers()
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        allowed_methods: rule
                            .allowed_methods()
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        allowed_origins: rule
                            .allowed_origins()
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        expose_headers: rule
                            .expose_headers()
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        max_age_seconds: rule.max_age_seconds(),
                    })
                    .collect();

                BucketCorsResponse { rules, error: None }
            }
            Err(e) => {
                let err_str = e.to_string();
                // NoSuchCORSConfiguration means no CORS is set - not an error
                if err_str.contains("NoSuchCORSConfiguration")
                    || err_str.contains("NoSuchCors")
                    || err_str.contains("The CORS configuration does not exist")
                {
                    BucketCorsResponse {
                        rules: vec![],
                        error: None,
                    }
                } else {
                    BucketCorsResponse {
                        rules: vec![],
                        error: Some(err_str),
                    }
                }
            }
        }
    }

    /// Get bucket lifecycle configuration
    pub async fn get_bucket_lifecycle(&self, bucket_name: &str) -> BucketLifecycleResponse {
        match self
            .client
            .get_bucket_lifecycle_configuration()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(output) => {
                let rules = output
                    .rules()
                    .iter()
                    .map(|rule| {
                        // Extract transitions
                        let transitions = rule
                            .transitions()
                            .iter()
                            .map(|t| LifecycleTransition {
                                days: t.days(),
                                date: t.date().map(|d| d.to_string()),
                                storage_class: t
                                    .storage_class()
                                    .map(|sc| sc.as_str().to_string())
                                    .unwrap_or_default(),
                            })
                            .collect();

                        // Extract filter prefix (simplified - could be more complex)
                        let filter_prefix = rule
                            .filter()
                            .and_then(|f| f.prefix().map(|p| p.to_string()));

                        LifecycleRule {
                            id: rule.id().map(|s| s.to_string()),
                            status: rule.status().as_str().to_string(),
                            filter_prefix,
                            expiration_days: rule.expiration().and_then(|e| e.days()),
                            expiration_date: rule
                                .expiration()
                                .and_then(|e| e.date().map(|d| d.to_string())),
                            transitions,
                            noncurrent_version_expiration_days: rule
                                .noncurrent_version_expiration()
                                .and_then(|e| e.noncurrent_days()),
                            abort_incomplete_multipart_days: rule
                                .abort_incomplete_multipart_upload()
                                .and_then(|a| a.days_after_initiation()),
                        }
                    })
                    .collect();

                BucketLifecycleResponse { rules, error: None }
            }
            Err(e) => {
                let err_str = e.to_string();
                // NoSuchLifecycleConfiguration means no lifecycle is set - not an error
                if err_str.contains("NoSuchLifecycleConfiguration")
                    || err_str.contains("NoSuchLifecycle")
                    || err_str.contains("The lifecycle configuration does not exist")
                {
                    BucketLifecycleResponse {
                        rules: vec![],
                        error: None,
                    }
                } else {
                    BucketLifecycleResponse {
                        rules: vec![],
                        error: Some(err_str),
                    }
                }
            }
        }
    }

    /// Get bucket versioning status
    pub async fn get_bucket_versioning(&self, bucket_name: &str) -> BucketVersioningResponse {
        match self
            .client
            .get_bucket_versioning()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(output) => BucketVersioningResponse {
                status: output.status().map(|s| s.as_str().to_string()),
                mfa_delete: output.mfa_delete().map(|m| m.as_str().to_string()),
                error: None,
            },
            Err(e) => BucketVersioningResponse {
                status: None,
                mfa_delete: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Get bucket encryption configuration
    pub async fn get_bucket_encryption(&self, bucket_name: &str) -> BucketEncryptionResponse {
        match self
            .client
            .get_bucket_encryption()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(output) => {
                let rules = output
                    .server_side_encryption_configuration()
                    .map(|config| {
                        config
                            .rules()
                            .iter()
                            .filter_map(|rule| {
                                rule.apply_server_side_encryption_by_default().map(|sse| {
                                    BucketEncryptionRule {
                                        sse_algorithm: sse.sse_algorithm().as_str().to_string(),
                                        kms_master_key_id: sse
                                            .kms_master_key_id()
                                            .map(|k| k.to_string()),
                                        bucket_key_enabled: rule.bucket_key_enabled(),
                                    }
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                BucketEncryptionResponse { rules, error: None }
            }
            Err(e) => {
                let err_str = e.to_string();
                // ServerSideEncryptionConfigurationNotFoundError means no encryption is set
                if err_str.contains("ServerSideEncryptionConfigurationNotFoundError")
                    || err_str.contains("NoSuchEncryption")
                    || err_str.contains("encryption configuration was not found")
                {
                    BucketEncryptionResponse {
                        rules: vec![],
                        error: None,
                    }
                } else {
                    BucketEncryptionResponse {
                        rules: vec![],
                        error: Some(err_str),
                    }
                }
            }
        }
    }

    /// Get all bucket configuration at once (parallel calls)
    pub async fn get_bucket_configuration(&self, bucket_name: &str) -> BucketConfigurationResponse {
        // Execute all configuration fetches in parallel
        let (policy, acl_result, cors, lifecycle, versioning, encryption) = tokio::join!(
            self.get_bucket_policy(bucket_name),
            self.get_bucket_acl(bucket_name),
            self.get_bucket_cors(bucket_name),
            self.get_bucket_lifecycle(bucket_name),
            self.get_bucket_versioning(bucket_name),
            self.get_bucket_encryption(bucket_name),
        );

        // ACL returns Result, convert to string
        let acl = acl_result.unwrap_or_else(|_| "Unknown".to_string());

        BucketConfigurationResponse {
            policy,
            acl,
            cors,
            lifecycle,
            versioning,
            encryption,
        }
    }

    // ========================================================================
    // Object Lock Methods
    // ========================================================================

    /// Get object lock status (retention + legal hold)
    /// Returns ObjectLockStatus with is_locked=false if Object Lock is not configured
    /// or if the bucket doesn't support Object Lock
    pub async fn get_object_lock_status(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<ObjectLockStatus, AppError> {
        // Fetch retention and legal hold in parallel
        let (retention_result, legal_hold_result) = tokio::join!(
            self.client
                .get_object_retention()
                .bucket(bucket)
                .key(key)
                .send(),
            self.client
                .get_object_legal_hold()
                .bucket(bucket)
                .key(key)
                .send()
        );

        // Parse retention result
        let (retention_mode, retain_until_date) = match retention_result {
            Ok(output) => {
                let retention = output.retention();
                let mode = retention.and_then(|r| r.mode().map(|m| m.as_str().to_string()));
                let until_date =
                    retention.and_then(|r| r.retain_until_date().map(|d| d.to_string()));
                (mode, until_date)
            }
            Err(e) => {
                let err_str = e.to_string();
                // These errors mean no retention is set - not an error condition
                if err_str.contains("ObjectLockConfigurationNotFoundError")
                    || err_str.contains("NoSuchObjectLockConfiguration")
                    || err_str.contains("InvalidRequest")
                    || err_str.contains("AccessDenied")
                {
                    (None, None)
                } else {
                    // Log unexpected errors but continue
                    eprintln!("Warning: Failed to get object retention: {}", err_str);
                    (None, None)
                }
            }
        };

        // Parse legal hold result
        let legal_hold = match legal_hold_result {
            Ok(output) => output
                .legal_hold()
                .and_then(|lh| lh.status())
                .map(|s| s.as_str() == "ON")
                .unwrap_or(false),
            Err(e) => {
                let err_str = e.to_string();
                // These errors mean no legal hold is set - not an error condition
                if err_str.contains("ObjectLockConfigurationNotFoundError")
                    || err_str.contains("NoSuchObjectLockConfiguration")
                    || err_str.contains("InvalidRequest")
                    || err_str.contains("AccessDenied")
                {
                    false
                } else {
                    // Log unexpected errors but continue
                    eprintln!("Warning: Failed to get object legal hold: {}", err_str);
                    false
                }
            }
        };

        // Object is locked if it has retention OR legal hold
        let is_locked = retention_mode.is_some() || legal_hold;

        Ok(ObjectLockStatus {
            is_locked,
            retention_mode,
            retain_until_date,
            legal_hold,
        })
    }
}
