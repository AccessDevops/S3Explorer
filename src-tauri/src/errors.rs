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

    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Database connection pool error: {0}")]
    PoolError(String),

    #[error("Database migration error: {0}")]
    MigrationError(String),

    #[error("Index operation error: {0}")]
    IndexError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Conversion depuis rusqlite::Error
impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

// Conversion depuis r2d2::Error
impl From<r2d2::Error> for AppError {
    fn from(err: r2d2::Error) -> Self {
        AppError::PoolError(err.to_string())
    }
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
