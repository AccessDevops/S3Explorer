use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Profile storage error: {0}")]
    ProfileStorageError(String),

    #[error("S3 operation failed: {0}")]
    S3Error(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Error response sent to frontend
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

impl From<AppError> for ErrorResponse {
    fn from(err: AppError) -> Self {
        ErrorResponse {
            error: err.to_string(),
            details: None,
        }
    }
}

// Convert AppError to String for Tauri command results
impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_string()
    }
}
