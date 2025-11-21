use crate::errors::AppError;
use crate::models::*;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::time::Duration;

/// S3 Client Adapter that supports multiple providers
pub struct S3Adapter {
    client: Client,
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

        let region = Region::new(profile.region.clone());
        let region_provider = RegionProviderChain::first_try(region);

        // Build base AWS config
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .credentials_provider(credentials)
            .load()
            .await;

        // Build S3-specific config
        let mut s3_config_builder = S3ConfigBuilder::from(&aws_config);

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

        Ok(Self { client })
    }

    /// Test connection by listing buckets
    pub async fn test_connection(&self) -> Result<TestConnectionResponse, AppError> {
        match self.client.list_buckets().send().await {
            Ok(output) => {
                let bucket_count = output.buckets().len();
                Ok(TestConnectionResponse {
                    success: true,
                    message: format!("Successfully connected. Found {} buckets.", bucket_count),
                    bucket_count: Some(bucket_count),
                })
            }
            Err(e) => Ok(TestConnectionResponse {
                success: false,
                message: format!("Connection failed: {}", e),
                bucket_count: None,
            }),
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
        self.client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to create bucket: {}", e)))?;

        Ok(())
    }

    /// Get bucket ACL to determine if it's public or private
    pub async fn get_bucket_acl(&self, bucket_name: &str) -> Result<String, AppError> {
        let acl = self.client
            .get_bucket_acl()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|e| AppError::S3Error(format!("Failed to get bucket ACL: {}", e)))?;

        // Check if bucket has public grants
        let is_public = acl.grants()
            .iter()
            .any(|grant| {
                if let Some(grantee) = grant.grantee() {
                    if let Some(uri) = grantee.uri() {
                        // Check for AllUsers or AuthenticatedUsers groups
                        return uri.contains("AllUsers") || uri.contains("AuthenticatedUsers");
                    }
                }
                false
            });

        Ok(if is_public { "Public".to_string() } else { "Private".to_string() })
    }

    /// Calculate bucket statistics (total size and count) by listing ALL objects without delimiter
    pub async fn calculate_bucket_stats(&self, bucket_name: &str) -> Result<(i64, i64), AppError> {
        let mut total_size: i64 = 0;
        let mut total_count: i64 = 0;
        let mut continuation_token: Option<String> = None;

        // Paginate through ALL objects in the bucket (no delimiter)
        loop {
            let mut request = self.client
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
                    .map_err(|e| AppError::S3Error(format!("Failed to generate presigned URL: {}", e)))?;
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
                    .map_err(|e| AppError::S3Error(format!("Failed to generate presigned URL: {}", e)))?;
                presigned.uri().to_string()
            }
            _ => return Err(AppError::ConfigError(format!("Unsupported method: {}", method))),
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
                .list_objects(bucket, Some(&folder_prefix), continuation_token, Some(1000), false)
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
        if let Err(_) = self.delete_object(bucket, &folder_prefix).await {
            // Ignore error if the folder marker doesn't exist
        }

        Ok(deleted_count)
    }

    /// List all versions of an object
    pub async fn list_object_versions(&self, bucket: &str, key: &str) -> Result<ListObjectVersionsResponse, AppError> {
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
}
