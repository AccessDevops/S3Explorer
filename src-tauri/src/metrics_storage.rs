//! Metrics Storage Module
//!
//! Handles storage and retrieval of S3 request metrics in a dedicated SQLite database.
//! This replaces the frontend IndexedDB storage for better performance and consistency.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

use crate::errors::AppError;
use crate::models::{RequestCategory, S3ErrorCategory, S3MetricsEvent, S3Operation};

// ============================================================================
// Types
// ============================================================================

/// Daily statistics (aggregated from requests)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub get_requests: i64,
    pub put_requests: i64,
    pub list_requests: i64,
    pub delete_requests: i64,
    pub estimated_cost_usd: f64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: i64,
    pub bytes_downloaded: i64,
    pub bytes_uploaded: i64,
    pub updated_at: i64,
}

/// Hourly statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HourlyStats {
    pub hour: i32,
    pub count: i64,
    pub success_count: i64,
    pub failed_count: i64,
}

/// Daily distribution statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyDistribution {
    pub date: String,
    pub day_label: String,
    pub count: i64,
    pub success_count: i64,
    pub failed_count: i64,
}

/// Weekly distribution statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeeklyDistribution {
    pub week_start: String,
    pub week_label: String,
    pub count: i64,
    pub success_count: i64,
    pub failed_count: i64,
}

/// Statistics by operation type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperationStats {
    pub operation: String,
    pub count: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub avg_duration_ms: f64,
    pub total_bytes: i64,
}

/// Error statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorStats {
    pub category: String,
    pub count: i64,
    pub last_occurrence: i64,
    pub example_message: Option<String>,
}

/// Bucket usage statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BucketUsageStats {
    pub bucket_name: String,
    pub request_count: i64,
    pub bytes_transferred: i64,
}

/// Individual request record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestRecord {
    pub id: String,
    pub timestamp: i64,
    pub date: String,
    pub operation: String,
    pub category: String,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub bucket_name: Option<String>,
    pub object_key: Option<String>,
    pub duration_ms: i64,
    pub bytes_transferred: Option<i64>,
    pub objects_affected: Option<i32>,
    pub success: bool,
    pub error_category: Option<String>,
    pub error_message: Option<String>,
}

/// Cache event record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEvent {
    pub id: String,
    pub timestamp: i64,
    pub date: String,
    pub operation: String,
    pub hit: bool,
    pub profile_id: Option<String>,
    pub bucket_name: Option<String>,
    pub saved_requests: Option<i32>,
}

/// Daily cache statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyCacheStats {
    pub date: String,
    pub total_lookups: i64,
    pub hits: i64,
    pub misses: i64,
    pub hit_rate: f64,
    pub estimated_requests_saved: i64,
    pub estimated_cost_saved: f64,
    pub updated_at: i64,
}

/// Cache summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheSummary {
    pub hit_rate: f64,
    pub total_hits: i64,
    pub total_misses: i64,
    pub requests_saved: i64,
    pub cost_saved: f64,
}

/// Storage info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageInfo {
    pub request_count: i64,
    pub oldest_date: Option<String>,
}

/// S3 Pricing configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3Pricing {
    pub get_per_thousand: f64,
    pub put_per_thousand: f64,
    pub list_per_thousand: f64,
    pub delete_per_thousand: f64,
}

impl Default for S3Pricing {
    fn default() -> Self {
        // AWS S3 Standard pricing (us-east-1)
        Self {
            get_per_thousand: 0.0004,
            put_per_thousand: 0.005,
            list_per_thousand: 0.005,
            delete_per_thousand: 0.0,
        }
    }
}

// ============================================================================
// Database Manager
// ============================================================================

/// Schema SQL for metrics database
const METRICS_SCHEMA: &str = r#"
-- Metrics requests table
CREATE TABLE IF NOT EXISTS metrics_requests (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    date TEXT NOT NULL,
    operation TEXT NOT NULL,
    category TEXT NOT NULL CHECK(category IN ('GET', 'PUT', 'LIST', 'DELETE', 'LOCAL')),
    profile_id TEXT,
    profile_name TEXT,
    bucket_name TEXT,
    object_key TEXT,
    duration_ms INTEGER NOT NULL,
    bytes_transferred INTEGER,
    objects_affected INTEGER,
    success BOOLEAN NOT NULL,
    error_category TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics_requests(timestamp);
CREATE INDEX IF NOT EXISTS idx_metrics_date ON metrics_requests(date);
CREATE INDEX IF NOT EXISTS idx_metrics_operation ON metrics_requests(operation);
CREATE INDEX IF NOT EXISTS idx_metrics_bucket ON metrics_requests(bucket_name);
CREATE INDEX IF NOT EXISTS idx_metrics_success ON metrics_requests(success);

-- Daily stats cache table
CREATE TABLE IF NOT EXISTS metrics_daily_stats (
    date TEXT PRIMARY KEY,
    total_requests INTEGER DEFAULT 0,
    successful_requests INTEGER DEFAULT 0,
    failed_requests INTEGER DEFAULT 0,
    get_requests INTEGER DEFAULT 0,
    put_requests INTEGER DEFAULT 0,
    list_requests INTEGER DEFAULT 0,
    delete_requests INTEGER DEFAULT 0,
    estimated_cost_usd REAL DEFAULT 0,
    avg_duration_ms REAL DEFAULT 0,
    max_duration_ms INTEGER DEFAULT 0,
    bytes_downloaded INTEGER DEFAULT 0,
    bytes_uploaded INTEGER DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- Cache events table
CREATE TABLE IF NOT EXISTS metrics_cache_events (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    date TEXT NOT NULL,
    operation TEXT NOT NULL,
    hit BOOLEAN NOT NULL,
    profile_id TEXT,
    bucket_name TEXT,
    saved_requests INTEGER DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_cache_timestamp ON metrics_cache_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_cache_date ON metrics_cache_events(date);
"#;

/// Get the metrics database path
fn get_metrics_db_path() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::ConfigError("Could not determine home directory".to_string()))?;
    let app_dir = home.join(".s3explorer");
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| AppError::DatabaseError(format!("Failed to create app directory: {}", e)))?;
    Ok(app_dir.join("metrics.db"))
}

/// Open a new metrics database connection
/// Each call creates a new connection - connections are not pooled.
pub fn get_metrics_db() -> Result<Connection, AppError> {
    let db_path = get_metrics_db_path()?;
    let conn = Connection::open(&db_path)
        .map_err(|e| AppError::DatabaseError(format!("Failed to open metrics database: {}", e)))?;

    // Enable WAL mode for better concurrency
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(|e| AppError::DatabaseError(format!("Failed to set pragmas: {}", e)))?;

    // Apply schema (CREATE IF NOT EXISTS is idempotent)
    conn.execute_batch(METRICS_SCHEMA)
        .map_err(|e| AppError::DatabaseError(format!("Failed to apply metrics schema: {}", e)))?;

    Ok(conn)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get today's date as YYYY-MM-DD
fn get_today_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Get date N days ago as YYYY-MM-DD
fn get_date_days_ago(days: u32) -> String {
    let date = chrono::Utc::now() - chrono::Duration::days(days as i64);
    date.format("%Y-%m-%d").to_string()
}

/// Helper to emit a cache hit event (index used successfully instead of S3)
/// `operation` should describe what operation was served from cache (e.g., "BucketStats", "FolderSize", "Search")
/// `saved_requests` is the estimated number of S3 requests that were avoided
pub fn emit_cache_hit(
    operation: &str,
    profile_id: Option<&str>,
    bucket_name: Option<&str>,
    saved_requests: i32,
) {
    if let Ok(db) = get_metrics_db() {
        let event = CacheEvent {
            id: format!(
                "cache-{}-{}",
                chrono::Utc::now().timestamp_millis(),
                uuid::Uuid::new_v4()
            ),
            timestamp: chrono::Utc::now().timestamp_millis(),
            date: get_today_date(),
            operation: operation.to_string(),
            hit: true,
            profile_id: profile_id.map(|s| s.to_string()),
            bucket_name: bucket_name.map(|s| s.to_string()),
            saved_requests: Some(saved_requests),
        };
        let _ = record_cache_event(&db, &event);
    }
}

/// Helper to emit a cache miss event (index didn't have data, had to use S3)
pub fn emit_cache_miss(operation: &str, profile_id: Option<&str>, bucket_name: Option<&str>) {
    if let Ok(db) = get_metrics_db() {
        let event = CacheEvent {
            id: format!(
                "cache-{}-{}",
                chrono::Utc::now().timestamp_millis(),
                uuid::Uuid::new_v4()
            ),
            timestamp: chrono::Utc::now().timestamp_millis(),
            date: get_today_date(),
            operation: operation.to_string(),
            hit: false,
            profile_id: profile_id.map(|s| s.to_string()),
            bucket_name: bucket_name.map(|s| s.to_string()),
            saved_requests: None,
        };
        let _ = record_cache_event(&db, &event);
    }
}

/// Convert S3Operation to string
fn operation_to_string(op: &S3Operation) -> String {
    format!("{:?}", op)
}

/// Convert RequestCategory to string
fn category_to_string(cat: &RequestCategory) -> String {
    match cat {
        RequestCategory::GET => "GET".to_string(),
        RequestCategory::PUT => "PUT".to_string(),
        RequestCategory::LIST => "LIST".to_string(),
        RequestCategory::DELETE => "DELETE".to_string(),
        RequestCategory::LOCAL => "LOCAL".to_string(),
    }
}

/// Convert S3ErrorCategory to string
fn error_category_to_string(cat: &S3ErrorCategory) -> Option<String> {
    Some(format!("{:?}", cat))
}

/// Calculate cost from request counts
fn calculate_cost(
    get_count: i64,
    put_count: i64,
    list_count: i64,
    _delete_count: i64,
    pricing: &S3Pricing,
) -> f64 {
    (get_count as f64 / 1000.0) * pricing.get_per_thousand
        + (put_count as f64 / 1000.0) * pricing.put_per_thousand
        + (list_count as f64 / 1000.0) * pricing.list_per_thousand
    // DELETE is typically free
}

// ============================================================================
// Write Operations
// ============================================================================

/// Record a new S3 request metric
pub fn record_request(conn: &Connection, event: &S3MetricsEvent) -> Result<(), AppError> {
    let date = chrono::DateTime::from_timestamp_millis(event.timestamp)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(get_today_date);

    conn.execute(
        r#"
        INSERT OR REPLACE INTO metrics_requests (
            id, timestamp, date, operation, category,
            profile_id, profile_name, bucket_name, object_key,
            duration_ms, bytes_transferred, objects_affected,
            success, error_category, error_message
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#,
        params![
            event.id,
            event.timestamp,
            date,
            operation_to_string(&event.operation),
            category_to_string(&event.category),
            event.profile_id,
            event.profile_name,
            event.bucket_name,
            event.object_key,
            event.duration_ms as i64,
            event.bytes_transferred.map(|b| b as i64),
            event.objects_affected.map(|o| o as i32),
            event.success,
            event
                .error_category
                .as_ref()
                .and_then(error_category_to_string),
            event.error_message,
        ],
    )
    .map_err(|e| AppError::DatabaseError(format!("Failed to record request: {}", e)))?;

    // Invalidate daily stats cache for this date
    conn.execute(
        "DELETE FROM metrics_daily_stats WHERE date = ?1",
        params![date],
    )
    .ok();

    Ok(())
}

/// Record a cache event
pub fn record_cache_event(conn: &Connection, event: &CacheEvent) -> Result<(), AppError> {
    conn.execute(
        r#"
        INSERT OR REPLACE INTO metrics_cache_events (
            id, timestamp, date, operation, hit,
            profile_id, bucket_name, saved_requests
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            event.id,
            event.timestamp,
            event.date,
            event.operation,
            event.hit,
            event.profile_id,
            event.bucket_name,
            event.saved_requests.unwrap_or(1),
        ],
    )
    .map_err(|e| AppError::DatabaseError(format!("Failed to record cache event: {}", e)))?;

    Ok(())
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get today's statistics
pub fn get_today_stats(conn: &Connection, pricing: &S3Pricing) -> Result<DailyStats, AppError> {
    let today = get_today_date();
    get_daily_stats(conn, &today, pricing)
}

/// Get statistics for a specific date
pub fn get_daily_stats(
    conn: &Connection,
    date: &str,
    pricing: &S3Pricing,
) -> Result<DailyStats, AppError> {
    // Try to get from cache first (only for past dates)
    let today = get_today_date();
    if date != today {
        if let Some(cached) = get_cached_daily_stats(conn, date)? {
            return Ok(cached);
        }
    }

    // Calculate fresh stats
    let stats = calculate_daily_stats(conn, date, pricing)?;

    // Cache if not today
    if date != today {
        cache_daily_stats(conn, &stats).ok();
    }

    Ok(stats)
}

/// Calculate daily stats from raw requests
fn calculate_daily_stats(
    conn: &Connection,
    date: &str,
    pricing: &S3Pricing,
) -> Result<DailyStats, AppError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN category = 'GET' THEN 1 ELSE 0 END) as get_count,
                SUM(CASE WHEN category = 'PUT' THEN 1 ELSE 0 END) as put_count,
                SUM(CASE WHEN category = 'LIST' THEN 1 ELSE 0 END) as list_count,
                SUM(CASE WHEN category = 'DELETE' THEN 1 ELSE 0 END) as delete_count,
                AVG(duration_ms) as avg_duration,
                MAX(duration_ms) as max_duration,
                SUM(CASE WHEN operation = 'GetObject' THEN COALESCE(bytes_transferred, 0) ELSE 0 END) as bytes_down,
                SUM(CASE WHEN operation IN ('PutObject', 'UploadPart') THEN COALESCE(bytes_transferred, 0) ELSE 0 END) as bytes_up
            FROM metrics_requests
            WHERE date = ?1
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let stats = stmt
        .query_row(params![date], |row| {
            let total: i64 = row.get(0)?;
            let successful: i64 = row.get(1)?;
            let failed: i64 = row.get(2)?;
            let get_count: i64 = row.get(3)?;
            let put_count: i64 = row.get(4)?;
            let list_count: i64 = row.get(5)?;
            let delete_count: i64 = row.get(6)?;
            let avg_duration: f64 = row.get::<_, Option<f64>>(7)?.unwrap_or(0.0);
            let max_duration: i64 = row.get::<_, Option<i64>>(8)?.unwrap_or(0);
            let bytes_down: i64 = row.get::<_, Option<i64>>(9)?.unwrap_or(0);
            let bytes_up: i64 = row.get::<_, Option<i64>>(10)?.unwrap_or(0);

            let cost = calculate_cost(get_count, put_count, list_count, delete_count, pricing);

            Ok(DailyStats {
                date: date.to_string(),
                total_requests: total,
                successful_requests: successful,
                failed_requests: failed,
                get_requests: get_count,
                put_requests: put_count,
                list_requests: list_count,
                delete_requests: delete_count,
                estimated_cost_usd: cost,
                avg_duration_ms: avg_duration,
                max_duration_ms: max_duration,
                bytes_downloaded: bytes_down,
                bytes_uploaded: bytes_up,
                updated_at: chrono::Utc::now().timestamp_millis(),
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(stats)
}

/// Get cached daily stats
fn get_cached_daily_stats(conn: &Connection, date: &str) -> Result<Option<DailyStats>, AppError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT date, total_requests, successful_requests, failed_requests,
                   get_requests, put_requests, list_requests, delete_requests,
                   estimated_cost_usd, avg_duration_ms, max_duration_ms,
                   bytes_downloaded, bytes_uploaded, updated_at
            FROM metrics_daily_stats
            WHERE date = ?1
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let result = stmt
        .query_row(params![date], |row| {
            Ok(DailyStats {
                date: row.get(0)?,
                total_requests: row.get(1)?,
                successful_requests: row.get(2)?,
                failed_requests: row.get(3)?,
                get_requests: row.get(4)?,
                put_requests: row.get(5)?,
                list_requests: row.get(6)?,
                delete_requests: row.get(7)?,
                estimated_cost_usd: row.get(8)?,
                avg_duration_ms: row.get(9)?,
                max_duration_ms: row.get(10)?,
                bytes_downloaded: row.get(11)?,
                bytes_uploaded: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })
        .optional()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(result)
}

/// Cache daily stats
fn cache_daily_stats(conn: &Connection, stats: &DailyStats) -> Result<(), AppError> {
    conn.execute(
        r#"
        INSERT OR REPLACE INTO metrics_daily_stats (
            date, total_requests, successful_requests, failed_requests,
            get_requests, put_requests, list_requests, delete_requests,
            estimated_cost_usd, avg_duration_ms, max_duration_ms,
            bytes_downloaded, bytes_uploaded, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        params![
            stats.date,
            stats.total_requests,
            stats.successful_requests,
            stats.failed_requests,
            stats.get_requests,
            stats.put_requests,
            stats.list_requests,
            stats.delete_requests,
            stats.estimated_cost_usd,
            stats.avg_duration_ms,
            stats.max_duration_ms,
            stats.bytes_downloaded,
            stats.bytes_uploaded,
            stats.updated_at,
        ],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Get stats history for last N days
pub fn get_stats_history(
    conn: &Connection,
    days: u32,
    pricing: &S3Pricing,
) -> Result<Vec<DailyStats>, AppError> {
    let mut stats = Vec::with_capacity(days as usize);

    for i in 0..days {
        let date = get_date_days_ago(days - 1 - i);
        let daily = get_daily_stats(conn, &date, pricing)?;
        stats.push(daily);
    }

    Ok(stats)
}

/// Get aggregated stats for a period (sum of all days)
pub fn get_period_stats(
    conn: &Connection,
    days: u32,
    pricing: &S3Pricing,
) -> Result<DailyStats, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN category = 'GET' THEN 1 ELSE 0 END) as get_count,
                SUM(CASE WHEN category = 'PUT' THEN 1 ELSE 0 END) as put_count,
                SUM(CASE WHEN category = 'LIST' THEN 1 ELSE 0 END) as list_count,
                SUM(CASE WHEN category = 'DELETE' THEN 1 ELSE 0 END) as delete_count,
                AVG(duration_ms) as avg_duration,
                MAX(duration_ms) as max_duration,
                SUM(CASE WHEN operation = 'GetObject' THEN COALESCE(bytes_transferred, 0) ELSE 0 END) as bytes_down,
                SUM(CASE WHEN operation IN ('PutObject', 'UploadPart') THEN COALESCE(bytes_transferred, 0) ELSE 0 END) as bytes_up
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let stats = stmt
        .query_row(params![from_date, to_date], |row| {
            let total: i64 = row.get(0)?;
            let successful: i64 = row.get(1)?;
            let failed: i64 = row.get(2)?;
            let get_count: i64 = row.get(3)?;
            let put_count: i64 = row.get(4)?;
            let list_count: i64 = row.get(5)?;
            let delete_count: i64 = row.get(6)?;
            let avg_duration: f64 = row.get::<_, Option<f64>>(7)?.unwrap_or(0.0);
            let max_duration: i64 = row.get::<_, Option<i64>>(8)?.unwrap_or(0);
            let bytes_down: i64 = row.get::<_, Option<i64>>(9)?.unwrap_or(0);
            let bytes_up: i64 = row.get::<_, Option<i64>>(10)?.unwrap_or(0);

            let cost = calculate_cost(get_count, put_count, list_count, delete_count, pricing);

            Ok(DailyStats {
                date: format!("{} - {}", from_date, to_date),
                total_requests: total,
                successful_requests: successful,
                failed_requests: failed,
                get_requests: get_count,
                put_requests: put_count,
                list_requests: list_count,
                delete_requests: delete_count,
                estimated_cost_usd: cost,
                avg_duration_ms: avg_duration,
                max_duration_ms: max_duration,
                bytes_downloaded: bytes_down,
                bytes_uploaded: bytes_up,
                updated_at: chrono::Utc::now().timestamp_millis(),
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(stats)
}

/// Get hourly breakdown aggregated over multiple days
pub fn get_hourly_stats_period(conn: &Connection, days: u32) -> Result<Vec<HourlyStats>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                CAST(strftime('%H', datetime(timestamp/1000, 'unixepoch')) AS INTEGER) as hour,
                COUNT(*) as count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed_count
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2
            GROUP BY hour
            ORDER BY hour
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut hourly_map: std::collections::HashMap<i32, HourlyStats> = (0..24)
        .map(|h| {
            (
                h,
                HourlyStats {
                    hour: h,
                    count: 0,
                    success_count: 0,
                    failed_count: 0,
                },
            )
        })
        .collect();

    let rows = stmt
        .query_map(params![from_date, to_date], |row| {
            Ok(HourlyStats {
                hour: row.get(0)?,
                count: row.get(1)?,
                success_count: row.get(2)?,
                failed_count: row.get(3)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    for row in rows {
        let stats = row.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        hourly_map.insert(stats.hour, stats);
    }

    let mut result: Vec<HourlyStats> = hourly_map.into_values().collect();
    result.sort_by_key(|s| s.hour);
    Ok(result)
}

/// Get daily distribution for a period (used for 7-day view)
pub fn get_daily_distribution(
    conn: &Connection,
    days: u32,
) -> Result<Vec<DailyDistribution>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                date,
                COUNT(*) as count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed_count
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2
            GROUP BY date
            ORDER BY date
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![from_date, to_date], |row| {
            let date_str: String = row.get(0)?;
            // Parse the date to get day name
            let day_label =
                if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    parsed.format("%a").to_string() // Mon, Tue, etc.
                } else {
                    date_str.clone()
                };
            Ok(DailyDistribution {
                date: date_str,
                day_label,
                count: row.get(1)?,
                success_count: row.get(2)?,
                failed_count: row.get(3)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Create a map with all days in the range
    let mut daily_map: std::collections::HashMap<String, DailyDistribution> =
        std::collections::HashMap::new();

    // Initialize all days in range with zero counts
    for i in 0..days {
        let date = get_date_days_ago(days - 1 - i);
        let day_label = if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
            parsed.format("%a").to_string()
        } else {
            date.clone()
        };
        daily_map.insert(
            date.clone(),
            DailyDistribution {
                date: date.clone(),
                day_label,
                count: 0,
                success_count: 0,
                failed_count: 0,
            },
        );
    }

    // Fill in actual data
    for row in rows {
        let stats = row.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        daily_map.insert(stats.date.clone(), stats);
    }

    let mut result: Vec<DailyDistribution> = daily_map.into_values().collect();
    result.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(result)
}

/// Get weekly distribution for a period (used for 30-day view)
pub fn get_weekly_distribution(
    conn: &Connection,
    days: u32,
) -> Result<Vec<WeeklyDistribution>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                strftime('%Y-%W', date) as week,
                MIN(date) as week_start,
                COUNT(*) as count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed_count
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2
            GROUP BY week
            ORDER BY week
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![from_date, to_date], |row| {
            let week_start: String = row.get(1)?;
            // Create week label like "Dec 2" from the week start date
            let week_label =
                if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&week_start, "%Y-%m-%d") {
                    parsed.format("%b %d").to_string()
                } else {
                    week_start.clone()
                };
            Ok(WeeklyDistribution {
                week_start,
                week_label,
                count: row.get(2)?,
                success_count: row.get(3)?,
                failed_count: row.get(4)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Get hourly breakdown for a specific date
pub fn get_hourly_stats(conn: &Connection, date: &str) -> Result<Vec<HourlyStats>, AppError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                CAST(strftime('%H', datetime(timestamp/1000, 'unixepoch')) AS INTEGER) as hour,
                COUNT(*) as count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed_count
            FROM metrics_requests
            WHERE date = ?1
            GROUP BY hour
            ORDER BY hour
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut hourly_map: std::collections::HashMap<i32, HourlyStats> = (0..24)
        .map(|h| {
            (
                h,
                HourlyStats {
                    hour: h,
                    count: 0,
                    success_count: 0,
                    failed_count: 0,
                },
            )
        })
        .collect();

    let rows = stmt
        .query_map(params![date], |row| {
            Ok(HourlyStats {
                hour: row.get(0)?,
                count: row.get(1)?,
                success_count: row.get(2)?,
                failed_count: row.get(3)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    for row in rows {
        let stats = row.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        hourly_map.insert(stats.hour, stats);
    }

    let mut result: Vec<HourlyStats> = hourly_map.into_values().collect();
    result.sort_by_key(|s| s.hour);
    Ok(result)
}

/// Get stats grouped by operation type
pub fn get_operation_stats(conn: &Connection, days: u32) -> Result<Vec<OperationStats>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                operation,
                COUNT(*) as count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed_count,
                AVG(duration_ms) as avg_duration,
                COALESCE(SUM(bytes_transferred), 0) as total_bytes
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2
            GROUP BY operation
            ORDER BY count DESC
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![from_date, to_date], |row| {
            Ok(OperationStats {
                operation: row.get(0)?,
                count: row.get(1)?,
                success_count: row.get(2)?,
                failed_count: row.get(3)?,
                avg_duration_ms: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                total_bytes: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Get error statistics
pub fn get_error_stats(conn: &Connection, days: u32) -> Result<Vec<ErrorStats>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                error_category,
                COUNT(*) as count,
                MAX(timestamp) as last_occurrence,
                (SELECT error_message FROM metrics_requests m2
                 WHERE m2.error_category = metrics_requests.error_category
                 ORDER BY timestamp DESC LIMIT 1) as example_message
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2 AND success = 0 AND error_category IS NOT NULL
            GROUP BY error_category
            ORDER BY count DESC
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![from_date, to_date], |row| {
            Ok(ErrorStats {
                category: row.get(0)?,
                count: row.get(1)?,
                last_occurrence: row.get(2)?,
                example_message: row.get(3)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Get top buckets by request count
pub fn get_top_buckets(
    conn: &Connection,
    days: u32,
    limit: u32,
) -> Result<Vec<BucketUsageStats>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                bucket_name,
                COUNT(*) as request_count,
                COALESCE(SUM(bytes_transferred), 0) as bytes_transferred
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2 AND bucket_name IS NOT NULL
            GROUP BY bucket_name
            ORDER BY request_count DESC
            LIMIT ?3
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![from_date, to_date, limit], |row| {
            Ok(BucketUsageStats {
                bucket_name: row.get(0)?,
                request_count: row.get(1)?,
                bytes_transferred: row.get(2)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Get recent requests
pub fn get_recent_requests(conn: &Connection, limit: u32) -> Result<Vec<RequestRecord>, AppError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, timestamp, date, operation, category,
                   profile_id, profile_name, bucket_name, object_key,
                   duration_ms, bytes_transferred, objects_affected,
                   success, error_category, error_message
            FROM metrics_requests
            ORDER BY timestamp DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(RequestRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                date: row.get(2)?,
                operation: row.get(3)?,
                category: row.get(4)?,
                profile_id: row.get(5)?,
                profile_name: row.get(6)?,
                bucket_name: row.get(7)?,
                object_key: row.get(8)?,
                duration_ms: row.get(9)?,
                bytes_transferred: row.get(10)?,
                objects_affected: row.get(11)?,
                success: row.get(12)?,
                error_category: row.get(13)?,
                error_message: row.get(14)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

/// Get failed requests
pub fn get_failed_requests(
    conn: &Connection,
    days: u32,
    limit: u32,
) -> Result<Vec<RequestRecord>, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, timestamp, date, operation, category,
                   profile_id, profile_name, bucket_name, object_key,
                   duration_ms, bytes_transferred, objects_affected,
                   success, error_category, error_message
            FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2 AND success = 0
            ORDER BY timestamp DESC
            LIMIT ?3
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map(params![from_date, to_date, limit], |row| {
            Ok(RequestRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                date: row.get(2)?,
                operation: row.get(3)?,
                category: row.get(4)?,
                profile_id: row.get(5)?,
                profile_name: row.get(6)?,
                bucket_name: row.get(7)?,
                object_key: row.get(8)?,
                duration_ms: row.get(9)?,
                bytes_transferred: row.get(10)?,
                objects_affected: row.get(11)?,
                success: row.get(12)?,
                error_category: row.get(13)?,
                error_message: row.get(14)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))
}

// ============================================================================
// Cache Metrics
// ============================================================================

/// Get cache summary for a period
/// Cache efficiency is calculated as: saved_requests / (saved_requests + actual_list_requests) * 100
/// This shows what percentage of LIST-type operations were avoided thanks to the cache
pub fn get_cache_summary(
    conn: &Connection,
    days: u32,
    pricing: &S3Pricing,
) -> Result<CacheSummary, AppError> {
    let from_date = get_date_days_ago(days - 1);
    let to_date = get_today_date();

    // Get cache events stats
    let mut cache_stmt = conn
        .prepare(
            r#"
            SELECT
                SUM(CASE WHEN hit = 1 THEN 1 ELSE 0 END) as total_hits,
                SUM(CASE WHEN hit = 0 THEN 1 ELSE 0 END) as total_misses,
                COALESCE(SUM(CASE WHEN hit = 1 THEN saved_requests ELSE 0 END), 0) as requests_saved
            FROM metrics_cache_events
            WHERE date >= ?1 AND date <= ?2
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let (total_hits, total_misses, requests_saved) = cache_stmt
        .query_row(params![&from_date, &to_date], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            ))
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Get actual LIST requests from S3 metrics
    let mut list_stmt = conn
        .prepare(
            r#"
            SELECT COUNT(*) FROM metrics_requests
            WHERE date >= ?1 AND date <= ?2 AND category = 'LIST'
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let actual_list_requests: i64 = list_stmt
        .query_row(params![&from_date, &to_date], |row| row.get(0))
        .unwrap_or(0);

    // Calculate efficiency: saved_requests / (saved_requests + actual_list_requests)
    // This shows what percentage of potential LIST operations were served from cache
    let total_potential_requests = requests_saved + actual_list_requests;
    let hit_rate = if total_potential_requests > 0 {
        (requests_saved as f64 / total_potential_requests as f64) * 100.0
    } else {
        0.0
    };

    let cost_saved = (requests_saved as f64 / 1000.0) * pricing.list_per_thousand;

    Ok(CacheSummary {
        hit_rate,
        total_hits,
        total_misses,
        requests_saved,
        cost_saved,
    })
}

/// Get today's cache statistics
/// Cache efficiency is calculated as: saved_requests / (saved_requests + actual_list_requests) * 100
pub fn get_today_cache_stats(
    conn: &Connection,
    pricing: &S3Pricing,
) -> Result<DailyCacheStats, AppError> {
    let today = get_today_date();

    // Get cache events stats
    let mut cache_stmt = conn
        .prepare(
            r#"
            SELECT
                SUM(CASE WHEN hit = 1 THEN 1 ELSE 0 END) as hits,
                SUM(CASE WHEN hit = 0 THEN 1 ELSE 0 END) as misses,
                COALESCE(SUM(CASE WHEN hit = 1 THEN saved_requests ELSE 0 END), 0) as requests_saved
            FROM metrics_cache_events
            WHERE date = ?1
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let (hits, misses, requests_saved) = cache_stmt
        .query_row(params![&today], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            ))
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Get actual LIST requests from S3 metrics for today
    let mut list_stmt = conn
        .prepare(
            r#"
            SELECT COUNT(*) FROM metrics_requests
            WHERE date = ?1 AND category = 'LIST'
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let actual_list_requests: i64 = list_stmt
        .query_row(params![&today], |row| row.get(0))
        .unwrap_or(0);

    // Calculate efficiency: saved_requests / (saved_requests + actual_list_requests)
    let total_potential_requests = requests_saved + actual_list_requests;
    let hit_rate = if total_potential_requests > 0 {
        (requests_saved as f64 / total_potential_requests as f64) * 100.0
    } else {
        0.0
    };

    let cost_saved = (requests_saved as f64 / 1000.0) * pricing.list_per_thousand;

    Ok(DailyCacheStats {
        date: today,
        total_lookups: hits + misses,
        hits,
        misses,
        hit_rate,
        estimated_requests_saved: requests_saved,
        estimated_cost_saved: cost_saved,
        updated_at: chrono::Utc::now().timestamp_millis(),
    })
}

// ============================================================================
// Maintenance Operations
// ============================================================================

/// Get storage information
pub fn get_storage_info(conn: &Connection) -> Result<StorageInfo, AppError> {
    let request_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM metrics_requests", [], |row| {
            row.get(0)
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let oldest_date: Option<String> = conn
        .query_row("SELECT MIN(date) FROM metrics_requests", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .flatten();

    Ok(StorageInfo {
        request_count,
        oldest_date,
    })
}

/// Purge old data
pub fn purge_old_data(conn: &Connection, retention_days: u32) -> Result<u64, AppError> {
    let cutoff_date = get_date_days_ago(retention_days);

    let deleted_requests = conn
        .execute(
            "DELETE FROM metrics_requests WHERE date < ?1",
            params![cutoff_date],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))? as u64;

    conn.execute(
        "DELETE FROM metrics_daily_stats WHERE date < ?1",
        params![cutoff_date],
    )
    .ok();

    Ok(deleted_requests)
}

/// Purge old cache events
pub fn purge_cache_events(conn: &Connection, retention_days: u32) -> Result<u64, AppError> {
    let cutoff_date = get_date_days_ago(retention_days);

    let deleted = conn
        .execute(
            "DELETE FROM metrics_cache_events WHERE date < ?1",
            params![cutoff_date],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))? as u64;

    Ok(deleted)
}

/// Clear all metrics data
pub fn clear_all(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        DELETE FROM metrics_requests;
        DELETE FROM metrics_daily_stats;
        DELETE FROM metrics_cache_events;
        "#,
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Default retention period in days
pub const DEFAULT_RETENTION_DAYS: u32 = 30;

/// Auto-purge old metrics data at application startup.
/// Removes all data older than DEFAULT_RETENTION_DAYS (30 days).
/// This function is designed to be called once at app startup.
pub fn auto_purge_on_startup() {
    match get_metrics_db() {
        Ok(conn) => {
            let requests_deleted = purge_old_data(&conn, DEFAULT_RETENTION_DAYS).unwrap_or(0);
            let cache_events_deleted =
                purge_cache_events(&conn, DEFAULT_RETENTION_DAYS).unwrap_or(0);

            if requests_deleted > 0 || cache_events_deleted > 0 {
                println!(
                    "[Metrics] Auto-purge completed: {} requests and {} cache events older than {} days removed",
                    requests_deleted, cache_events_deleted, DEFAULT_RETENTION_DAYS
                );
            }
        }
        Err(e) => {
            eprintln!("[Metrics] Failed to auto-purge metrics: {}", e);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(METRICS_SCHEMA).unwrap();
        conn
    }

    #[test]
    fn test_record_and_get_request() {
        let conn = create_test_db();
        let event = S3MetricsEvent::new(S3Operation::ListBuckets, RequestCategory::GET)
            .with_duration(100)
            .with_profile("test-profile", "Test Profile")
            .with_bucket("test-bucket");

        record_request(&conn, &event).unwrap();

        let info = get_storage_info(&conn).unwrap();
        assert_eq!(info.request_count, 1);
    }

    #[test]
    fn test_daily_stats() {
        let conn = create_test_db();
        let pricing = S3Pricing::default();

        // Record some requests
        for i in 0..5 {
            let event = S3MetricsEvent::new(S3Operation::GetObject, RequestCategory::GET)
                .with_duration(100 + i * 10)
                .with_bucket("bucket");
            record_request(&conn, &event).unwrap();
        }

        let stats = get_today_stats(&conn, &pricing).unwrap();
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.get_requests, 5);
    }

    #[test]
    fn test_cache_events() {
        let conn = create_test_db();
        let pricing = S3Pricing::default();

        let hit_event = CacheEvent {
            id: "1".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            date: get_today_date(),
            operation: "search".to_string(),
            hit: true,
            profile_id: None,
            bucket_name: Some("bucket".to_string()),
            saved_requests: Some(5),
        };

        let miss_event = CacheEvent {
            id: "2".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            date: get_today_date(),
            operation: "search".to_string(),
            hit: false,
            profile_id: None,
            bucket_name: Some("bucket".to_string()),
            saved_requests: None,
        };

        record_cache_event(&conn, &hit_event).unwrap();
        record_cache_event(&conn, &miss_event).unwrap();

        let summary = get_cache_summary(&conn, 1, &pricing).unwrap();
        assert_eq!(summary.total_hits, 1);
        assert_eq!(summary.total_misses, 1);
        assert_eq!(summary.requests_saved, 5);
    }

    #[test]
    fn test_purge_old_data() {
        let conn = create_test_db();

        // Insert old request manually
        conn.execute(
            "INSERT INTO metrics_requests (id, timestamp, date, operation, category, duration_ms, success)
             VALUES ('old', 0, '2020-01-01', 'GetObject', 'GET', 100, 1)",
            [],
        ).unwrap();

        let info_before = get_storage_info(&conn).unwrap();
        assert_eq!(info_before.request_count, 1);

        let deleted = purge_old_data(&conn, 30).unwrap();
        assert_eq!(deleted, 1);

        let info_after = get_storage_info(&conn).unwrap();
        assert_eq!(info_after.request_count, 0);
    }
}
