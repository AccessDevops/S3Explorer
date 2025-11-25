//! Input validation utilities for S3 operations.
//!
//! Provides defense-in-depth validation on the backend, mirroring
//! the frontend validators in src/utils/validators.ts.

use crate::errors::AppError;
use std::net::Ipv4Addr;

/// Validate S3 bucket name according to AWS rules.
///
/// Rules:
/// - Must be between 3 and 63 characters
/// - Can only contain lowercase letters, numbers, dots (.), and hyphens (-)
/// - Must begin and end with a letter or number
/// - Must not be formatted as an IP address
/// - Must not contain consecutive periods
/// - Must not contain periods adjacent to hyphens
pub fn validate_bucket_name(name: &str) -> Result<(), AppError> {
    let name = name.trim();

    if name.is_empty() {
        return Err(AppError::ValidationError(
            "Bucket name cannot be empty".into(),
        ));
    }

    if name.len() < 3 {
        return Err(AppError::ValidationError(
            "Bucket name must be at least 3 characters long".into(),
        ));
    }

    if name.len() > 63 {
        return Err(AppError::ValidationError(
            "Bucket name must be no more than 63 characters long".into(),
        ));
    }

    // Check if formatted as IP address
    if name.parse::<Ipv4Addr>().is_ok() {
        return Err(AppError::ValidationError(
            "Bucket name cannot be formatted as an IP address".into(),
        ));
    }

    // Check valid characters
    let valid_chars = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-');

    if !valid_chars {
        return Err(AppError::ValidationError(
            "Bucket name can only contain lowercase letters, numbers, dots, and hyphens".into(),
        ));
    }

    // Must start with alphanumeric
    if let Some(first) = name.chars().next() {
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return Err(AppError::ValidationError(
                "Bucket name must start with a letter or number".into(),
            ));
        }
    }

    // Must end with alphanumeric
    if let Some(last) = name.chars().last() {
        if !last.is_ascii_lowercase() && !last.is_ascii_digit() {
            return Err(AppError::ValidationError(
                "Bucket name must end with a letter or number".into(),
            ));
        }
    }

    // No consecutive periods
    if name.contains("..") {
        return Err(AppError::ValidationError(
            "Bucket name cannot contain consecutive periods".into(),
        ));
    }

    // No period-dash combinations
    if name.contains(".-") || name.contains("-.") {
        return Err(AppError::ValidationError(
            "Bucket name cannot contain periods adjacent to hyphens".into(),
        ));
    }

    Ok(())
}

/// Validate S3 object key.
///
/// Rules:
/// - Cannot be empty
/// - Maximum 1024 characters
/// - Should not contain control characters
pub fn validate_object_key(key: &str) -> Result<(), AppError> {
    let key = key.trim();

    if key.is_empty() {
        return Err(AppError::ValidationError(
            "Object key cannot be empty".into(),
        ));
    }

    if key.len() > 1024 {
        return Err(AppError::ValidationError(
            "Object key must be no more than 1024 characters long".into(),
        ));
    }

    // Check for control characters
    if key.chars().any(|c| c.is_control()) {
        return Err(AppError::ValidationError(
            "Object key contains invalid control characters".into(),
        ));
    }

    Ok(())
}

/// Validate endpoint URL.
///
/// Rules:
/// - Must be a valid URL format
/// - Must use http or https protocol
/// - Warns (but allows) HTTP for non-localhost endpoints
pub fn validate_endpoint(endpoint: &str) -> Result<Option<String>, AppError> {
    let endpoint = endpoint.trim();

    // Empty endpoint is valid (will use AWS default)
    if endpoint.is_empty() {
        return Ok(None);
    }

    // Try to parse as URL
    let url = url::Url::parse(endpoint).map_err(|_| {
        AppError::ValidationError("Invalid endpoint URL format".into())
    })?;

    // Check protocol
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(AppError::ValidationError(
                "Endpoint must use http:// or https:// protocol".into(),
            ));
        }
    }

    // Check hostname exists
    if url.host_str().is_none() {
        return Err(AppError::ValidationError(
            "Endpoint must have a valid hostname".into(),
        ));
    }

    // Return warning for HTTP on non-localhost
    if url.scheme() == "http" {
        if let Some(host) = url.host_str() {
            if host != "localhost" && host != "127.0.0.1" && !host.starts_with("192.168.") && !host.starts_with("10.") {
                return Ok(Some(
                    "Warning: Using HTTP with a non-local endpoint. Credentials will be sent unencrypted.".into()
                ));
            }
        }
    }

    Ok(None)
}

/// Validate presigned URL expiration time.
///
/// Rules:
/// - Must be at least 1 second
/// - Must not exceed 7 days (604800 seconds)
pub fn validate_presigned_url_expiry(seconds: u64) -> Result<(), AppError> {
    const MIN_EXPIRY: u64 = 1;
    const MAX_EXPIRY: u64 = 604800; // 7 days

    if seconds < MIN_EXPIRY {
        return Err(AppError::ValidationError(
            "Presigned URL expiry must be at least 1 second".into(),
        ));
    }

    if seconds > MAX_EXPIRY {
        return Err(AppError::ValidationError(
            format!("Presigned URL expiry cannot exceed {} seconds (7 days)", MAX_EXPIRY),
        ));
    }

    Ok(())
}

/// Validate folder path for S3.
///
/// Rules:
/// - Cannot be empty
/// - Must not start with /
/// - Must end with / (will be added if missing)
pub fn validate_folder_path(path: &str) -> Result<String, AppError> {
    let path = path.trim();

    if path.is_empty() {
        return Err(AppError::ValidationError(
            "Folder path cannot be empty".into(),
        ));
    }

    // Remove leading slash if present
    let path = path.strip_prefix('/').unwrap_or(path);

    // Validate as object key
    validate_object_key(path)?;

    // Ensure trailing slash
    let path = if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{}/", path)
    };

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bucket_names() {
        assert!(validate_bucket_name("my-bucket").is_ok());
        assert!(validate_bucket_name("my.bucket.name").is_ok());
        assert!(validate_bucket_name("bucket123").is_ok());
        assert!(validate_bucket_name("123bucket").is_ok());
        assert!(validate_bucket_name("a-b").is_ok()); // Minimum 3 chars
    }

    #[test]
    fn test_invalid_bucket_names() {
        // Too short
        assert!(validate_bucket_name("ab").is_err());
        assert!(validate_bucket_name("").is_err());

        // Too long
        assert!(validate_bucket_name(&"a".repeat(64)).is_err());

        // Invalid characters
        assert!(validate_bucket_name("My-Bucket").is_err()); // uppercase
        assert!(validate_bucket_name("my_bucket").is_err()); // underscore
        assert!(validate_bucket_name("my bucket").is_err()); // space

        // Invalid start/end
        assert!(validate_bucket_name("-bucket").is_err());
        assert!(validate_bucket_name("bucket-").is_err());
        assert!(validate_bucket_name(".bucket").is_err());

        // IP address format
        assert!(validate_bucket_name("192.168.1.1").is_err());

        // Consecutive periods
        assert!(validate_bucket_name("my..bucket").is_err());

        // Period-dash combinations
        assert!(validate_bucket_name("my.-bucket").is_err());
        assert!(validate_bucket_name("my-.bucket").is_err());
    }

    #[test]
    fn test_valid_object_keys() {
        assert!(validate_object_key("file.txt").is_ok());
        assert!(validate_object_key("folder/file.txt").is_ok());
        assert!(validate_object_key("a").is_ok());
        assert!(validate_object_key("file with spaces.txt").is_ok());
        assert!(validate_object_key("文件.txt").is_ok()); // Unicode
    }

    #[test]
    fn test_invalid_object_keys() {
        assert!(validate_object_key("").is_err());
        assert!(validate_object_key(&"a".repeat(1025)).is_err());
        assert!(validate_object_key("file\x00.txt").is_err()); // null char
        assert!(validate_object_key("file\n.txt").is_err()); // newline
    }

    #[test]
    fn test_valid_endpoints() {
        assert!(validate_endpoint("").is_ok());
        assert!(validate_endpoint("https://s3.amazonaws.com").is_ok());
        assert!(validate_endpoint("http://localhost:9000").is_ok());
        assert!(validate_endpoint("http://127.0.0.1:9000").is_ok());
    }

    #[test]
    fn test_invalid_endpoints() {
        assert!(validate_endpoint("not-a-url").is_err());
        assert!(validate_endpoint("ftp://server.com").is_err());
    }

    #[test]
    fn test_presigned_url_expiry() {
        assert!(validate_presigned_url_expiry(1).is_ok());
        assert!(validate_presigned_url_expiry(3600).is_ok());
        assert!(validate_presigned_url_expiry(604800).is_ok());

        assert!(validate_presigned_url_expiry(0).is_err());
        assert!(validate_presigned_url_expiry(604801).is_err());
    }

    #[test]
    fn test_folder_path() {
        assert_eq!(validate_folder_path("folder").unwrap(), "folder/");
        assert_eq!(validate_folder_path("folder/").unwrap(), "folder/");
        assert_eq!(validate_folder_path("/folder").unwrap(), "folder/");
        assert_eq!(
            validate_folder_path("path/to/folder").unwrap(),
            "path/to/folder/"
        );

        assert!(validate_folder_path("").is_err());
    }
}
