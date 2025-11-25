use crate::errors::AppError;
use crate::models::*;
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
            "s3browser",
        );

        // Use provided region or default to "us-east-1"
        let region_str = profile.region.clone().unwrap_or_else(|| "us-east-1".to_string());
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

    /// Calculate bucket statistics (total size and count) by listing ALL objects without delimiter
    pub async fn calculate_bucket_stats(&self, bucket_name: &str) -> Result<(i64, i64), AppError> {
        let mut total_size: i64 = 0;
        let mut total_count: i64 = 0;
        let mut continuation_token: Option<String> = None;

        // Paginate through ALL objects in the bucket (no delimiter)
        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(bucket_name)
                .max_keys(1000);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let output = request
                .send()
                .await
                .map_err(|e| AppError::S3Error(format!("Failed to list objects: {}", e)))?;

            // Sum up object sizes (skip folder markers)
            for obj in output.contents() {
                if let Some(key) = obj.key() {
                    if !key.ends_with('/') {
                        total_size += obj.size().unwrap_or(0);
                        total_count += 1;
                    }
                }
            }

            // Check if there are more pages
            continuation_token = output.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        }

        Ok((total_size, total_count))
    }

    /// Estimate bucket statistics by listing only the first 1000 objects
    /// Returns (total_size, total_count, is_estimate)
    /// - If bucket has <= 1000 objects: returns exact stats with is_estimate=false
    /// - If bucket has > 1000 objects: returns partial stats with is_estimate=true
    pub async fn estimate_bucket_stats(&self, bucket_name: &str) -> Result<(i64, i64, bool), AppError> {
        let mut total_size: i64 = 0;
        let mut total_count: i64 = 0;

        // List only the first 1000 objects (single request)
        let output = self
            .client
            .list_objects_v2()
            .bucket(bucket_name)
            .max_keys(1000)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to list objects: {}", e)))?;

        // Sum up object sizes (skip folder markers)
        for obj in output.contents() {
            if let Some(key) = obj.key() {
                if !key.ends_with('/') {
                    total_size += obj.size().unwrap_or(0);
                    total_count += 1;
                }
            }
        }

        // Check if truncated (more than 1000 objects)
        let is_estimate = output.is_truncated().unwrap_or(false);

        Ok((total_size, total_count, is_estimate))
    }

    /// Calculate folder size by listing ALL objects with the given prefix (including all subdirectories recursively)
    /// This method skips folder markers (objects ending with '/') as they are zero-byte placeholders
    pub async fn calculate_folder_size(
        &self,
        bucket_name: &str,
        prefix: &str,
    ) -> Result<i64, AppError> {
        let mut total_size: i64 = 0;
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

        Ok(total_size)
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

    /// Delete a folder and all its contents
    pub async fn delete_folder(&self, bucket: &str, prefix: &str) -> Result<i64, AppError> {
        let folder_prefix = if prefix.ends_with('/') {
            prefix.to_string()
        } else {
            format!("{}/", prefix)
        };

        let mut deleted_count: i64 = 0;
        let mut continuation_token: Option<String> = None;

        // Paginate through all objects in the folder (without delimiter to get all nested objects)
        loop {
            let response = self
                .list_objects(
                    bucket,
                    Some(&folder_prefix),
                    continuation_token,
                    Some(1000),
                    false,
                )
                .await?;

            // Delete all objects in this batch
            for obj in response.objects {
                self.delete_object(bucket, &obj.key).await?;
                deleted_count += 1;
            }

            // Check if there are more pages
            continuation_token = response.continuation_token;
            if continuation_token.is_none() {
                break;
            }
        }

        // Also delete the folder marker itself (if it exists)
        if self.delete_object(bucket, &folder_prefix).await.is_err() {
            // Ignore error if the folder marker doesn't exist
        }

        Ok(deleted_count)
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

        let s3_tags: Vec<Tag> = tags
            .iter()
            .map(|tag| {
                Tag::builder()
                    .key(&tag.key)
                    .value(&tag.value)
                    .build()
                    .expect("Failed to build tag")
            })
            .collect();

        let tagging = Tagging::builder()
            .set_tag_set(Some(s3_tags))
            .build()
            .expect("Failed to build tagging");

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
        let metadata: HashMap<String, String> = output
            .metadata()
            .map(|m| m.clone())
            .unwrap_or_default();

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
            if let Ok(dt) = DateTime::from_str(&exp, aws_sdk_s3::primitives::DateTimeFormat::HttpDate) {
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
}
