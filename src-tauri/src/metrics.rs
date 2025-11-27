//! Metrics emission module
//!
//! Provides helper functions to emit S3 metrics events to the frontend.

use crate::models::{
    categorize_s3_error, RequestCategory, S3MetricsEvent, S3Operation,
};
use tauri::{AppHandle, Manager};

/// Emit a metrics event to the frontend
pub fn emit_metrics(app: &AppHandle, event: S3MetricsEvent) {
    let _ = app.emit_all("metrics:s3-request", event);
}

/// Helper to create and emit a successful metrics event
#[allow(dead_code)]
pub fn emit_success(
    app: &AppHandle,
    operation: S3Operation,
    category: RequestCategory,
    duration_ms: u64,
    profile_id: Option<&str>,
    profile_name: Option<&str>,
    bucket_name: Option<&str>,
    object_key: Option<&str>,
    bytes_transferred: Option<u64>,
    objects_affected: Option<u32>,
) {
    let mut event = S3MetricsEvent::new(operation, category).with_duration(duration_ms);

    if let (Some(pid), Some(pname)) = (profile_id, profile_name) {
        event = event.with_profile(pid, pname);
    }
    if let Some(bucket) = bucket_name {
        event = event.with_bucket(bucket);
    }
    if let Some(key) = object_key {
        event = event.with_object_key(key);
    }
    if let Some(bytes) = bytes_transferred {
        event = event.with_bytes(bytes);
    }
    if let Some(count) = objects_affected {
        event = event.with_objects_affected(count);
    }

    emit_metrics(app, event);
}

/// Helper to create and emit a failed metrics event
#[allow(dead_code)]
pub fn emit_error(
    app: &AppHandle,
    operation: S3Operation,
    category: RequestCategory,
    duration_ms: u64,
    error_message: &str,
    profile_id: Option<&str>,
    profile_name: Option<&str>,
    bucket_name: Option<&str>,
    object_key: Option<&str>,
) {
    let error_category = categorize_s3_error(error_message);
    let mut event = S3MetricsEvent::new(operation, category)
        .with_duration(duration_ms)
        .with_error(error_category, error_message);

    if let (Some(pid), Some(pname)) = (profile_id, profile_name) {
        event = event.with_profile(pid, pname);
    }
    if let Some(bucket) = bucket_name {
        event = event.with_bucket(bucket);
    }
    if let Some(key) = object_key {
        event = event.with_object_key(key);
    }

    emit_metrics(app, event);
}

/// Context for tracking a metrics event
pub struct MetricsContext {
    pub operation: S3Operation,
    pub category: RequestCategory,
    pub start_time: std::time::Instant,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub bucket_name: Option<String>,
    pub object_key: Option<String>,
    pub bytes_transferred: Option<u64>,
    pub objects_affected: Option<u32>,
}

impl MetricsContext {
    /// Create a new metrics context
    pub fn new(operation: S3Operation, category: RequestCategory) -> Self {
        Self {
            operation,
            category,
            start_time: std::time::Instant::now(),
            profile_id: None,
            profile_name: None,
            bucket_name: None,
            object_key: None,
            bytes_transferred: None,
            objects_affected: None,
        }
    }

    /// Set profile info
    pub fn with_profile(mut self, profile_id: &str, profile_name: &str) -> Self {
        self.profile_id = Some(profile_id.to_string());
        self.profile_name = Some(profile_name.to_string());
        self
    }

    /// Set bucket name
    pub fn with_bucket(mut self, bucket: &str) -> Self {
        self.bucket_name = Some(bucket.to_string());
        self
    }

    /// Set object key
    pub fn with_object_key(mut self, key: &str) -> Self {
        self.object_key = Some(key.to_string());
        self
    }

    /// Set bytes transferred
    pub fn set_bytes(&mut self, bytes: u64) {
        self.bytes_transferred = Some(bytes);
    }

    /// Set objects affected
    pub fn set_objects_affected(&mut self, count: u32) {
        self.objects_affected = Some(count);
    }

    /// Emit a success event
    pub fn emit_success(self, app: &AppHandle) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        let mut event = S3MetricsEvent::new(self.operation, self.category).with_duration(duration_ms);

        if let (Some(ref pid), Some(ref pname)) = (&self.profile_id, &self.profile_name) {
            event = event.with_profile(pid, pname);
        }
        if let Some(ref bucket) = self.bucket_name {
            event = event.with_bucket(bucket);
        }
        if let Some(ref key) = self.object_key {
            event = event.with_object_key(key);
        }
        if let Some(bytes) = self.bytes_transferred {
            event = event.with_bytes(bytes);
        }
        if let Some(count) = self.objects_affected {
            event = event.with_objects_affected(count);
        }

        emit_metrics(app, event);
    }

    /// Emit an error event
    pub fn emit_error(self, app: &AppHandle, error_message: &str) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        let error_category = categorize_s3_error(error_message);
        let mut event = S3MetricsEvent::new(self.operation, self.category)
            .with_duration(duration_ms)
            .with_error(error_category, error_message);

        if let (Some(ref pid), Some(ref pname)) = (&self.profile_id, &self.profile_name) {
            event = event.with_profile(pid, pname);
        }
        if let Some(ref bucket) = self.bucket_name {
            event = event.with_bucket(bucket);
        }
        if let Some(ref key) = self.object_key {
            event = event.with_object_key(key);
        }

        emit_metrics(app, event);
    }

    /// Emit based on result
    pub fn emit_result<T, E: std::fmt::Display>(self, app: &AppHandle, result: &Result<T, E>) {
        match result {
            Ok(_) => self.emit_success(app),
            Err(e) => self.emit_error(app, &e.to_string()),
        }
    }
}

/// Macro to simplify metrics instrumentation
#[macro_export]
macro_rules! with_metrics {
    ($app:expr, $operation:expr, $category:expr, $ctx_setup:expr, $body:expr) => {{
        let mut ctx = $crate::metrics::MetricsContext::new($operation, $category);
        ctx = $ctx_setup(ctx);
        let result = $body;
        ctx.emit_result($app, &result);
        result
    }};
}
