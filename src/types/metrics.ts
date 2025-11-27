// S3 Request Metrics Types

/**
 * Request category for cost calculation
 * Based on AWS S3 pricing tiers
 */
export type RequestCategory = 'GET' | 'PUT' | 'LIST' | 'DELETE' | 'LOCAL'

/**
 * All S3 operations tracked by the application
 */
export type S3Operation =
  // Bucket operations
  | 'ListBuckets'
  | 'CreateBucket'
  | 'GetBucketAcl'
  // Object listing
  | 'ListObjectsV2'
  | 'ListObjectVersions'
  // Object CRUD
  | 'GetObject'
  | 'PutObject'
  | 'DeleteObject'
  | 'DeleteObjects'
  | 'CopyObject'
  | 'HeadObject'
  // Multipart upload
  | 'CreateMultipartUpload'
  | 'UploadPart'
  | 'CompleteMultipartUpload'
  | 'AbortMultipartUpload'
  // Tags & Metadata
  | 'GetObjectTagging'
  | 'PutObjectTagging'
  | 'DeleteObjectTagging'
  // Local operations (no S3 API call)
  | 'GeneratePresignedUrl'

/**
 * Error categories for tracking and analysis
 */
export type S3ErrorCategory =
  | 'ConnectionTimeout'
  | 'ReadTimeout'
  | 'NetworkError'
  | 'InvalidCredentials'
  | 'ExpiredCredentials'
  | 'AccessDenied'
  | 'BucketNotFound'
  | 'ObjectNotFound'
  | 'BucketAlreadyExists'
  | 'InvalidBucketName'
  | 'InvalidObjectKey'
  | 'RequestTooLarge'
  | 'ServiceUnavailable'
  | 'InternalError'
  | 'SlowDown'
  | 'MultipartUploadFailed'
  | 'Unknown'

/**
 * Individual S3 request record stored in IndexedDB
 */
export interface S3RequestRecord {
  id: string                      // UUID
  timestamp: number               // Unix timestamp in milliseconds
  date: string                    // YYYY-MM-DD for indexing

  operation: S3Operation          // Type of S3 operation
  category: RequestCategory       // Cost category

  profileId?: string              // Profile ID
  profileName?: string            // Profile name for display
  bucketName?: string             // Target bucket
  objectKey?: string              // Object key (truncated to 200 chars)

  durationMs: number              // Request duration in milliseconds
  bytesTransferred?: number       // Bytes uploaded or downloaded
  objectsAffected?: number        // Number of objects affected (batch operations)

  success: boolean                // Whether request succeeded
  errorCategory?: S3ErrorCategory // Error category if failed
  errorMessage?: string           // Error message (truncated to 500 chars)
}

/**
 * Aggregated daily statistics (cached for performance)
 */
export interface DailyStats {
  date: string                    // YYYY-MM-DD (primary key)

  totalRequests: number
  successfulRequests: number
  failedRequests: number

  getRequests: number
  putRequests: number
  listRequests: number
  deleteRequests: number

  estimatedCostUsd: number

  avgDurationMs: number
  maxDurationMs: number

  bytesDownloaded: number
  bytesUploaded: number

  updatedAt: number               // Last update timestamp
}

/**
 * Statistics grouped by operation type
 */
export interface OperationStats {
  operation: S3Operation
  count: number
  successCount: number
  failedCount: number
  avgDurationMs: number
  totalBytes: number
}

/**
 * Error statistics for analysis
 */
export interface ErrorStats {
  category: S3ErrorCategory
  count: number
  lastOccurrence: number
  exampleMessage?: string
}

/**
 * Bucket usage statistics
 */
export interface BucketUsageStats {
  bucketName: string
  requestCount: number
  bytesTransferred: number
}

/**
 * Hourly breakdown for timeline charts
 */
export interface HourlyStats {
  hour: number                    // 0-23
  count: number
  successCount: number
  failedCount: number
}

/**
 * S3 pricing configuration (per 1000 requests)
 */
export interface S3Pricing {
  getPerThousand: number          // GET, HEAD - $0.0004
  putPerThousand: number          // PUT, COPY, POST - $0.005
  listPerThousand: number         // LIST - $0.005
  deletePerThousand: number       // DELETE - $0.00
}

/**
 * S3 provider type for pricing presets
 */
export type S3Provider = 'aws' | 'gcp' | 'azure' | 'backblaze' | 'wasabi' | 'cloudflare' | 'minio' | 'custom'

/**
 * Provider display info
 */
export interface S3ProviderInfo {
  id: S3Provider
  name: string
  pricing: S3Pricing
  description: string
}

/**
 * Pricing presets for different S3 providers
 * Prices are per 1000 requests in USD
 */
export const PROVIDER_PRICING: Record<S3Provider, S3Pricing> = {
  // AWS S3 Standard (us-east-1)
  aws: {
    getPerThousand: 0.0004,
    putPerThousand: 0.005,
    listPerThousand: 0.005,
    deletePerThousand: 0,
  },
  // Google Cloud Storage
  gcp: {
    getPerThousand: 0.0004,
    putPerThousand: 0.005,
    listPerThousand: 0.005,
    deletePerThousand: 0,
  },
  // Azure Blob Storage
  azure: {
    getPerThousand: 0.0004,
    putPerThousand: 0.005,
    listPerThousand: 0.005,
    deletePerThousand: 0,
  },
  // Backblaze B2
  backblaze: {
    getPerThousand: 0.004,
    putPerThousand: 0.004,
    listPerThousand: 0.004,
    deletePerThousand: 0,
  },
  // Wasabi (no per-request fees, included in storage)
  wasabi: {
    getPerThousand: 0,
    putPerThousand: 0,
    listPerThousand: 0,
    deletePerThousand: 0,
  },
  // Cloudflare R2 (no egress/request fees)
  cloudflare: {
    getPerThousand: 0,
    putPerThousand: 0,
    listPerThousand: 0,
    deletePerThousand: 0,
  },
  // MinIO (self-hosted, no fees)
  minio: {
    getPerThousand: 0,
    putPerThousand: 0,
    listPerThousand: 0,
    deletePerThousand: 0,
  },
  // Custom pricing
  custom: {
    getPerThousand: 0.0004,
    putPerThousand: 0.005,
    listPerThousand: 0.005,
    deletePerThousand: 0,
  },
}

/**
 * Provider information for UI
 */
export const PROVIDER_INFO: S3ProviderInfo[] = [
  { id: 'aws', name: 'AWS S3', pricing: PROVIDER_PRICING.aws, description: 'Amazon Web Services S3' },
  { id: 'gcp', name: 'Google Cloud', pricing: PROVIDER_PRICING.gcp, description: 'Google Cloud Storage' },
  { id: 'azure', name: 'Azure Blob', pricing: PROVIDER_PRICING.azure, description: 'Microsoft Azure Blob Storage' },
  { id: 'backblaze', name: 'Backblaze B2', pricing: PROVIDER_PRICING.backblaze, description: 'Backblaze B2 Cloud Storage' },
  { id: 'wasabi', name: 'Wasabi', pricing: PROVIDER_PRICING.wasabi, description: 'Wasabi Hot Cloud Storage (no request fees)' },
  { id: 'cloudflare', name: 'Cloudflare R2', pricing: PROVIDER_PRICING.cloudflare, description: 'Cloudflare R2 (no egress fees)' },
  { id: 'minio', name: 'MinIO', pricing: PROVIDER_PRICING.minio, description: 'Self-hosted MinIO (no fees)' },
  { id: 'custom', name: 'Custom', pricing: PROVIDER_PRICING.custom, description: 'Custom pricing configuration' },
]

/**
 * Default AWS S3 pricing (us-east-1)
 */
export const DEFAULT_S3_PRICING: S3Pricing = PROVIDER_PRICING.aws

// ============================================
// Cache Metrics Types
// ============================================

/**
 * Type of cache operation
 */
export type CacheOperation = 'search' | 'listObjects' | 'bucketStats'

/**
 * Individual cache event record
 */
export interface CacheEvent {
  id: string
  timestamp: number
  date: string                    // YYYY-MM-DD for indexing
  operation: CacheOperation
  hit: boolean                    // true = served from cache
  profileId?: string
  bucketName?: string
  savedRequests?: number          // estimated S3 requests saved
}

/**
 * Daily cache statistics
 */
export interface DailyCacheStats {
  date: string                    // YYYY-MM-DD
  totalLookups: number
  hits: number
  misses: number
  hitRate: number                 // 0-100 percentage
  estimatedRequestsSaved: number
  estimatedCostSaved: number      // based on current pricing
  updatedAt: number
}

/**
 * Cache summary for UI display
 */
export interface CacheSummary {
  hitRate: number                 // 0-100
  totalHits: number
  totalMisses: number
  requestsSaved: number
  costSaved: number
}

/**
 * Maps operations to their cost category
 */
export const OPERATION_CATEGORY_MAP: Record<S3Operation, RequestCategory> = {
  // GET category
  ListBuckets: 'GET',
  GetBucketAcl: 'GET',
  GetObject: 'GET',
  HeadObject: 'GET',
  GetObjectTagging: 'GET',
  // PUT category
  CreateBucket: 'PUT',
  PutObject: 'PUT',
  CopyObject: 'PUT',
  CreateMultipartUpload: 'PUT',
  UploadPart: 'PUT',
  CompleteMultipartUpload: 'PUT',
  AbortMultipartUpload: 'PUT',
  PutObjectTagging: 'PUT',
  // LIST category
  ListObjectsV2: 'LIST',
  ListObjectVersions: 'LIST',
  // DELETE category
  DeleteObject: 'DELETE',
  DeleteObjects: 'DELETE',
  DeleteObjectTagging: 'DELETE',
  // LOCAL category (no S3 API call)
  GeneratePresignedUrl: 'LOCAL',
}

/**
 * Calculate cost based on request counts and pricing
 */
export function calculateCost(
  getCount: number,
  putCount: number,
  listCount: number,
  deleteCount: number,
  pricing: S3Pricing = DEFAULT_S3_PRICING
): number {
  return (
    (getCount / 1000) * pricing.getPerThousand +
    (putCount / 1000) * pricing.putPerThousand +
    (listCount / 1000) * pricing.listPerThousand +
    (deleteCount / 1000) * pricing.deletePerThousand
  )
}

/**
 * Event payload from Rust backend
 */
export interface S3MetricsEvent {
  id: string
  timestamp: number
  operation: S3Operation
  category: RequestCategory

  profile_id?: string
  profile_name?: string
  bucket_name?: string
  object_key?: string

  duration_ms: number
  bytes_transferred?: number
  objects_affected?: number

  success: boolean
  error_category?: S3ErrorCategory
  error_message?: string
}

/**
 * Convert backend event to storage record
 */
export function eventToRecord(event: S3MetricsEvent): S3RequestRecord {
  return {
    id: event.id,
    timestamp: event.timestamp,
    date: new Date(event.timestamp).toISOString().split('T')[0],
    operation: event.operation,
    category: event.category,
    profileId: event.profile_id,
    profileName: event.profile_name,
    bucketName: event.bucket_name,
    objectKey: event.object_key,
    durationMs: event.duration_ms,
    bytesTransferred: event.bytes_transferred,
    objectsAffected: event.objects_affected,
    success: event.success,
    errorCategory: event.error_category,
    errorMessage: event.error_message,
  }
}
