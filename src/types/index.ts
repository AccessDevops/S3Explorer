// S3 Profile
export interface Profile {
  id: string
  name: string
  endpoint?: string
  region?: string
  access_key: string
  secret_key: string
  session_token?: string
  path_style: boolean
  enabled?: boolean // Whether this profile is visible in the sidebar (default: true)
}

// Test Connection
export interface TestConnectionResponse {
  success: boolean
  message: string
  bucket_count?: number
  suggest_path_style?: boolean // Suggest enabling path_style if connection failed without it
}

// Bucket
export interface Bucket {
  name: string
  creation_date?: string
}

// S3 Object
export interface S3Object {
  key: string
  size: number
  last_modified?: string
  storage_class?: string
  e_tag?: string
  is_folder: boolean
}

// List Objects Response
export interface ListObjectsResponse {
  objects: S3Object[]
  common_prefixes: string[]
  continuation_token?: string
  is_truncated: boolean
}

// Get Object Response
export interface GetObjectResponse {
  content: number[]
  content_type?: string
  size: number
}

// Presigned URL Response
export interface PresignedUrlResponse {
  url: string
  expires_in_secs: number
}

// Object Version
export interface ObjectVersion {
  version_id: string
  key: string
  size: number
  last_modified?: string
  is_latest: boolean
  e_tag?: string
}

// List Object Versions Response
export interface ListObjectVersionsResponse {
  versions: ObjectVersion[]
}

// Multipart Upload
export interface MultipartUploadInitResponse {
  upload_id: string
}

export interface CompletedPart {
  part_number: number
  e_tag: string
}

export interface MultipartUploadPartResponse {
  e_tag: string
}

export interface MultipartProgress {
  uploadedParts: number
  totalParts: number
  uploadedBytes: number
  totalBytes: number
  percentage: number
}

// Upload Progress Event (from Rust)
export interface UploadProgressEvent {
  upload_id: string
  file_name: string
  file_size: number
  uploaded_bytes: number
  uploaded_parts: number
  total_parts: number
  percentage: number
  status: 'pending' | 'starting' | 'uploading' | 'completed' | 'failed' | 'cancelled'
  error?: string
}

// Download Progress Event (from Rust)
export interface DownloadProgressEvent {
  download_id: string
  file_name: string
  file_size: number
  downloaded_bytes: number
  percentage: number
  status: 'pending' | 'starting' | 'downloading' | 'completed' | 'failed' | 'cancelled'
  error?: string
  bytes_per_second?: number
}

// Object Tag
export interface ObjectTag {
  key: string
  value: string
}

// Get Object Tags Response
export interface GetObjectTagsResponse {
  tags: ObjectTag[]
}

// Object Metadata (HTTP Headers)
export interface GetObjectMetadataResponse {
  content_type?: string
  content_encoding?: string
  content_language?: string
  content_disposition?: string
  cache_control?: string
  expires?: string
  metadata: Record<string, string> // Custom x-amz-meta-* headers
}

// Error types
export * from './errors'

// ============================================================================
// Index Types (from Rust SQLite backend)
// ============================================================================

// Storage class statistics
export interface StorageClassStats {
  storage_class: string
  object_count: number
  total_size: number
}

// Bucket index statistics
export interface BucketIndexStats {
  bucket_name: string
  total_objects: number
  total_size: number
  is_complete: boolean
  storage_class_breakdown: StorageClassStats[]
  last_indexed_at: number | null
  /** Estimated size of the index data for this bucket (in bytes) */
  estimated_index_size: number
}

// Prefix (folder) statistics
export interface PrefixStats {
  prefix: string
  objects_count: number
  total_size: number
  is_complete: boolean
  last_sync_at: number | null
}

// Full prefix status from the index database
export interface PrefixStatus {
  profile_id: string
  bucket_name: string
  prefix: string
  is_complete: boolean
  objects_count: number
  total_size: number
  continuation_token: string | null
  last_indexed_key: string | null
  last_sync_started_at: number | null
  last_sync_completed_at: number | null
}

// Initial indexation result
export interface InitialIndexResult {
  total_indexed: number
  is_complete: boolean
  requests_made: number
  continuation_token: string | null
  last_key: string | null
  total_size: number
  error: string | null
}

// Index status enum
export type IndexStatus = 'starting' | 'indexing' | 'completed' | 'partial' | 'failed' | 'cancelled'

// Index progress event (from Rust via Tauri events)
export interface IndexProgressEvent {
  profile_id: string
  bucket_name: string
  objects_indexed: number
  requests_made: number
  max_requests: number
  is_complete: boolean
  status: IndexStatus
  error: string | null
}

// Bucket index metadata (for listing all indexes)
export interface BucketIndexMetadata {
  bucket_name: string
  total_objects: number
  total_size: number
  is_complete: boolean
  last_indexed_at: number | null
  estimated_index_size: number // Estimated size of the index data for this bucket
}

// ============================================================================
// Bucket Configuration Types (Read-Only)
// ============================================================================

export interface BucketPolicyResponse {
  policy: string | null
  error: string | null
}

export interface CorsRule {
  allowed_headers: string[]
  allowed_methods: string[]
  allowed_origins: string[]
  expose_headers: string[]
  max_age_seconds: number | null
}

export interface BucketCorsResponse {
  rules: CorsRule[]
  error: string | null
}

export interface LifecycleTransition {
  days: number | null
  date: string | null
  storage_class: string
}

export interface LifecycleRule {
  id: string | null
  status: string
  filter_prefix: string | null
  expiration_days: number | null
  expiration_date: string | null
  transitions: LifecycleTransition[]
  noncurrent_version_expiration_days: number | null
  abort_incomplete_multipart_days: number | null
}

export interface BucketLifecycleResponse {
  rules: LifecycleRule[]
  error: string | null
}

export interface BucketVersioningResponse {
  status: string | null // "Enabled" | "Suspended" | null
  mfa_delete: string | null
  error: string | null
}

export interface BucketEncryptionRule {
  sse_algorithm: string
  kms_master_key_id: string | null
  bucket_key_enabled: boolean | null
}

export interface BucketEncryptionResponse {
  rules: BucketEncryptionRule[]
  error: string | null
}

export interface BucketConfigurationResponse {
  policy: BucketPolicyResponse
  acl: string
  cors: BucketCorsResponse
  lifecycle: BucketLifecycleResponse
  versioning: BucketVersioningResponse
  encryption: BucketEncryptionResponse
}

// ============================================================================
// Object Lock Types
// ============================================================================

export interface ObjectLockStatus {
  is_locked: boolean
  retention_mode: 'GOVERNANCE' | 'COMPLIANCE' | null
  retain_until_date: string | null
  legal_hold: boolean
}
