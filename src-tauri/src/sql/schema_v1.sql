-- =====================================================
-- S3Explorer Index Database Schema v1
-- =====================================================

-- Table de version du schema
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

INSERT INTO schema_version (version, updated_at) VALUES (1, strftime('%s', 'now') * 1000);

-- =====================================================
-- TABLE OBJETS
-- =====================================================

CREATE TABLE IF NOT EXISTS objects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    bucket_name TEXT NOT NULL,
    key TEXT NOT NULL,
    version_id TEXT DEFAULT NULL,

    -- Proprietes S3
    size INTEGER NOT NULL,
    last_modified TEXT,
    e_tag TEXT,
    storage_class TEXT DEFAULT 'STANDARD',

    -- Proprietes optionnelles
    owner_id TEXT,
    owner_display_name TEXT,
    checksum_algorithm TEXT,

    -- Glacier/Archive
    restore_status TEXT,
    restore_expiry_date TEXT,

    -- Metadonnees enrichies (HeadObject)
    content_type TEXT,
    server_side_encryption TEXT,
    sse_kms_key_id TEXT,

    -- Colonnes pre-calculees
    parent_prefix TEXT NOT NULL,
    basename TEXT NOT NULL,
    extension TEXT,
    depth INTEGER NOT NULL DEFAULT 0,
    is_folder BOOLEAN DEFAULT FALSE,

    -- Tracking
    indexed_at INTEGER NOT NULL,
    metadata_loaded BOOLEAN DEFAULT FALSE
);

-- Index unique pour les objets (profile_id, bucket_name, key)
-- Pour le versioning, on utilise la table object_versions separement
CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_unique
    ON objects(profile_id, bucket_name, key)
    WHERE version_id IS NULL;

-- Index unique pour les objets versionnes
CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_versioned_unique
    ON objects(profile_id, bucket_name, key, version_id)
    WHERE version_id IS NOT NULL;

-- Index optimises pour la navigation
CREATE INDEX IF NOT EXISTS idx_objects_parent
    ON objects(profile_id, bucket_name, parent_prefix);

CREATE INDEX IF NOT EXISTS idx_objects_depth
    ON objects(profile_id, bucket_name, depth);

CREATE INDEX IF NOT EXISTS idx_objects_ext
    ON objects(profile_id, bucket_name, extension)
    WHERE extension IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_objects_storage
    ON objects(profile_id, bucket_name, storage_class);

CREATE INDEX IF NOT EXISTS idx_objects_indexed
    ON objects(indexed_at);

-- Index pour calcul de taille par prefix
CREATE INDEX IF NOT EXISTS idx_objects_size_calc
    ON objects(profile_id, bucket_name, parent_prefix, size);

-- Index pour recherche par cle
CREATE INDEX IF NOT EXISTS idx_objects_key
    ON objects(profile_id, bucket_name, key);

-- =====================================================
-- TABLE VERSIONS (pour buckets versionnes)
-- =====================================================

CREATE TABLE IF NOT EXISTS object_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    bucket_name TEXT NOT NULL,
    key TEXT NOT NULL,

    version_id TEXT NOT NULL,
    size INTEGER NOT NULL,
    last_modified TEXT,
    e_tag TEXT,
    storage_class TEXT,
    is_latest BOOLEAN DEFAULT FALSE,
    is_delete_marker BOOLEAN DEFAULT FALSE,

    owner_id TEXT,
    owner_display_name TEXT,
    checksum_algorithm TEXT,

    indexed_at INTEGER NOT NULL,

    UNIQUE(profile_id, bucket_name, key, version_id)
);

CREATE INDEX IF NOT EXISTS idx_versions_key
    ON object_versions(profile_id, bucket_name, key);

CREATE INDEX IF NOT EXISTS idx_versions_latest
    ON object_versions(profile_id, bucket_name, is_latest)
    WHERE is_latest = TRUE;

-- =====================================================
-- TABLE STATUS DES PREFIX
-- =====================================================

CREATE TABLE IF NOT EXISTS prefix_status (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    bucket_name TEXT NOT NULL,
    prefix TEXT NOT NULL DEFAULT '',

    is_complete BOOLEAN DEFAULT FALSE,
    objects_count INTEGER DEFAULT 0,
    total_size INTEGER DEFAULT 0,

    continuation_token TEXT,
    last_indexed_key TEXT,

    last_sync_started_at INTEGER,
    last_sync_completed_at INTEGER,

    UNIQUE(profile_id, bucket_name, prefix)
);

CREATE INDEX IF NOT EXISTS idx_prefix_status
    ON prefix_status(profile_id, bucket_name, is_complete);

CREATE INDEX IF NOT EXISTS idx_prefix_parent
    ON prefix_status(profile_id, bucket_name, prefix);

-- =====================================================
-- TABLE INFO BUCKET
-- =====================================================

CREATE TABLE IF NOT EXISTS bucket_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    bucket_name TEXT NOT NULL,

    versioning_enabled BOOLEAN,
    encryption_enabled BOOLEAN,
    default_encryption TEXT,

    acl TEXT,
    acl_cached_at INTEGER,

    region TEXT,

    initial_index_requests INTEGER DEFAULT 0,
    initial_index_completed BOOLEAN DEFAULT FALSE,

    last_checked_at INTEGER,

    UNIQUE(profile_id, bucket_name)
);

-- =====================================================
-- TABLE METRICS (migration depuis IndexedDB)
-- =====================================================

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

CREATE INDEX IF NOT EXISTS idx_metrics_timestamp
    ON metrics_requests(timestamp);

CREATE INDEX IF NOT EXISTS idx_metrics_date
    ON metrics_requests(date);

CREATE INDEX IF NOT EXISTS idx_metrics_operation
    ON metrics_requests(operation);

CREATE INDEX IF NOT EXISTS idx_metrics_bucket
    ON metrics_requests(bucket_name);

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

CREATE INDEX IF NOT EXISTS idx_cache_date
    ON metrics_cache_events(date);

-- =====================================================
-- VUES POUR STATISTIQUES
-- =====================================================

CREATE VIEW IF NOT EXISTS v_bucket_stats AS
SELECT
    o.profile_id,
    o.bucket_name,
    COUNT(*) as total_objects,
    COALESCE(SUM(o.size), 0) as total_size,
    COUNT(DISTINCT o.storage_class) as storage_class_count,
    MAX(o.indexed_at) as last_indexed,
    (SELECT is_complete FROM prefix_status ps
     WHERE ps.profile_id = o.profile_id
     AND ps.bucket_name = o.bucket_name
     AND ps.prefix = ''
     LIMIT 1) as is_complete,
    bi.versioning_enabled,
    bi.encryption_enabled
FROM objects o
LEFT JOIN bucket_info bi
    ON bi.profile_id = o.profile_id
    AND bi.bucket_name = o.bucket_name
WHERE o.is_folder = FALSE
GROUP BY o.profile_id, o.bucket_name;

CREATE VIEW IF NOT EXISTS v_storage_stats AS
SELECT
    profile_id,
    bucket_name,
    storage_class,
    COUNT(*) as object_count,
    SUM(size) as total_size
FROM objects
WHERE is_folder = FALSE
GROUP BY profile_id, bucket_name, storage_class;
