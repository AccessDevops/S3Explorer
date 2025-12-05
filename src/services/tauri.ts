import { invoke } from '@tauri-apps/api/tauri'
import type {
  Profile,
  TestConnectionResponse,
  Bucket,
  ListObjectsResponse,
  GetObjectResponse,
  PresignedUrlResponse,
  ListObjectVersionsResponse,
  GetObjectTagsResponse,
  ObjectTag,
  GetObjectMetadataResponse,
  ObjectLockStatus,
  BucketIndexStats,
  PrefixStats,
  PrefixStatus,
  InitialIndexResult,
  S3Object,
  BucketIndexMetadata,
  BucketConfigurationResponse,
} from '../types'

// Profile Management
export async function listProfiles(): Promise<Profile[]> {
  return await invoke('list_profiles')
}

export async function saveProfile(profile: Profile): Promise<void> {
  return await invoke('save_profile', { profile })
}

export async function deleteProfile(profileId: string): Promise<void> {
  return await invoke('delete_profile', { profileId })
}

export async function testConnection(profile: Profile): Promise<TestConnectionResponse> {
  return await invoke('test_connection', { profile })
}

// Bucket Operations
export async function listBuckets(profileId: string): Promise<Bucket[]> {
  return await invoke('list_buckets', { profileId })
}

export async function createBucket(profileId: string, bucketName: string): Promise<void> {
  return await invoke('create_bucket', { profileId, bucketName })
}

export async function deleteBucket(profileId: string, bucketName: string): Promise<void> {
  return await invoke('delete_bucket', { profileId, bucketName })
}

export async function canDeleteBucket(profileId: string, bucketName: string): Promise<boolean> {
  return await invoke('can_delete_bucket', { profileId, bucketName })
}

export async function getBucketAcl(profileId: string, bucketName: string): Promise<string> {
  return await invoke('get_bucket_acl', { profileId, bucketName })
}

export async function getBucketConfiguration(
  profileId: string,
  bucketName: string
): Promise<BucketConfigurationResponse> {
  return await invoke('get_bucket_configuration', { profileId, bucketName })
}

export async function calculateBucketStats(
  profileId: string,
  bucketName: string,
  forceRefresh?: boolean
): Promise<[number, number, boolean]> {
  return await invoke('calculate_bucket_stats', { profileId, bucketName, forceRefresh })
}

// Object Operations
export async function listObjects(
  profileId: string,
  bucket: string,
  prefix?: string,
  continuationToken?: string,
  maxKeys?: number,
  useDelimiter?: boolean,
  syncIndex?: boolean
): Promise<ListObjectsResponse> {
  return await invoke('list_objects', {
    profileId,
    bucket,
    prefix,
    continuationToken,
    maxKeys,
    useDelimiter,
    syncIndex,
  })
}

export async function getObject(
  profileId: string,
  bucket: string,
  key: string
): Promise<GetObjectResponse> {
  return await invoke('get_object', {
    profileId,
    bucket,
    key,
  })
}

export async function putObject(
  profileId: string,
  bucket: string,
  key: string,
  content: number[] | Uint8Array,
  contentType?: string
): Promise<void> {
  // Convert Uint8Array to regular array for proper Tauri serialization
  const contentArray = content instanceof Uint8Array ? Array.from(content) : content
  return await invoke('put_object', {
    profileId,
    bucket,
    key,
    content: contentArray,
    contentType,
  })
}

export async function deleteObject(profileId: string, bucket: string, key: string): Promise<void> {
  return await invoke('delete_object', {
    profileId,
    bucket,
    key,
  })
}

export async function deleteObjectVersion(
  profileId: string,
  bucket: string,
  key: string,
  versionId: string
): Promise<void> {
  return await invoke('delete_object_version', {
    profileId,
    bucket,
    key,
    versionId,
  })
}

export async function copyObject(
  profileId: string,
  sourceBucket: string,
  sourceKey: string,
  destBucket: string,
  destKey: string
): Promise<void> {
  return await invoke('copy_object', {
    profileId,
    sourceBucket,
    sourceKey,
    destBucket,
    destKey,
  })
}

export async function changeContentType(
  profileId: string,
  bucket: string,
  key: string,
  newContentType: string
): Promise<void> {
  return await invoke('change_content_type', {
    profileId,
    bucket,
    key,
    newContentType,
  })
}

export async function createFolder(
  profileId: string,
  bucket: string,
  folderPath: string
): Promise<void> {
  return await invoke('create_folder', {
    profileId,
    bucket,
    folderPath,
  })
}

export async function generatePresignedUrl(
  profileId: string,
  bucket: string,
  key: string,
  method: string,
  expiresInSecs: number
): Promise<PresignedUrlResponse> {
  return await invoke('generate_presigned_url', {
    profileId,
    bucket,
    key,
    method,
    expiresInSecs,
  })
}

export async function calculateFolderSize(
  profileId: string,
  bucket: string,
  prefix: string,
  forceRefresh?: boolean
): Promise<[number, boolean]> {
  return await invoke('calculate_folder_size', {
    profileId,
    bucket,
    prefix,
    forceRefresh,
  })
}

export async function deleteFolder(
  profileId: string,
  bucket: string,
  prefix: string
): Promise<number> {
  return await invoke('delete_folder', {
    profileId,
    bucket,
    prefix,
  })
}

export async function listObjectVersions(
  profileId: string,
  bucket: string,
  key: string
): Promise<ListObjectVersionsResponse> {
  return await invoke('list_object_versions', {
    profileId,
    bucket,
    key,
  })
}

// File Operations
export async function getFileSize(filePath: string): Promise<number> {
  return await invoke('get_file_size', { filePath })
}

// Upload Operations (Rust-managed with progress events)
export async function uploadFile(
  profileId: string,
  bucket: string,
  key: string,
  filePath: string,
  contentType?: string,
  multipartThresholdMb?: number
): Promise<string> {
  return await invoke('upload_file', {
    profileId,
    bucket,
    key,
    filePath,
    contentType,
    multipartThresholdMb,
  })
}

export async function cancelUpload(uploadId: string): Promise<void> {
  return await invoke('cancel_upload', {
    uploadId,
  })
}

// Download Operations (Rust-managed streaming to disk with progress events)
export async function downloadFile(
  profileId: string,
  bucket: string,
  key: string,
  destPath: string,
  versionId?: string
): Promise<string> {
  return await invoke('download_file', {
    profileId,
    bucket,
    key,
    versionId: versionId || null,
    destPath,
  })
}

export async function cancelDownload(downloadId: string): Promise<void> {
  return await invoke('cancel_download', {
    downloadId,
  })
}

// Tag Operations
export async function getObjectTags(
  profileId: string,
  bucket: string,
  key: string
): Promise<GetObjectTagsResponse> {
  return await invoke('get_object_tags', {
    profileId,
    bucket,
    key,
  })
}

export async function putObjectTags(
  profileId: string,
  bucket: string,
  key: string,
  tags: ObjectTag[]
): Promise<void> {
  return await invoke('put_object_tags', {
    profileId,
    bucket,
    key,
    tags,
  })
}

export async function deleteObjectTags(
  profileId: string,
  bucket: string,
  key: string
): Promise<void> {
  return await invoke('delete_object_tags', {
    profileId,
    bucket,
    key,
  })
}

// Metadata Operations (HTTP Headers)
export async function getObjectMetadata(
  profileId: string,
  bucket: string,
  key: string
): Promise<GetObjectMetadataResponse> {
  return await invoke('get_object_metadata', {
    profileId,
    bucket,
    key,
  })
}

export async function updateObjectMetadata(
  profileId: string,
  bucket: string,
  key: string,
  metadata: GetObjectMetadataResponse
): Promise<void> {
  return await invoke('update_object_metadata', {
    profileId,
    bucket,
    key,
    contentType: metadata.content_type,
    contentEncoding: metadata.content_encoding,
    contentLanguage: metadata.content_language,
    contentDisposition: metadata.content_disposition,
    cacheControl: metadata.cache_control,
    expires: metadata.expires,
    metadata: metadata.metadata,
  })
}

export async function getObjectLockStatus(
  profileId: string,
  bucket: string,
  key: string
): Promise<ObjectLockStatus> {
  return await invoke('get_object_lock_status', {
    profileId,
    bucket,
    key,
  })
}

// ============================================================================
// Index Management
// ============================================================================

export async function startInitialIndex(
  profileId: string,
  bucketName: string,
  maxRequests?: number,
  batchSize?: number
): Promise<InitialIndexResult> {
  return await invoke('start_initial_index', { profileId, bucketName, maxRequests, batchSize })
}

export async function cancelIndexing(
  profileId: string,
  bucketName: string
): Promise<void> {
  return await invoke('cancel_indexing', { profileId, bucketName })
}

export async function getBucketIndexStats(
  profileId: string,
  bucketName: string
): Promise<BucketIndexStats> {
  return await invoke('get_bucket_index_stats', { profileId, bucketName })
}

export async function getPrefixIndexStats(
  profileId: string,
  bucketName: string,
  prefix: string
): Promise<PrefixStats> {
  return await invoke('get_prefix_index_stats', { profileId, bucketName, prefix })
}

export async function clearBucketIndex(
  profileId: string,
  bucketName: string
): Promise<void> {
  return await invoke('clear_bucket_index', { profileId, bucketName })
}

export async function isBucketIndexed(
  profileId: string,
  bucketName: string
): Promise<boolean> {
  return await invoke('is_bucket_indexed', { profileId, bucketName })
}

export async function isBucketIndexComplete(
  profileId: string,
  bucketName: string
): Promise<boolean> {
  return await invoke('is_bucket_index_complete', { profileId, bucketName })
}

export async function isPrefixKnown(
  profileId: string,
  bucketName: string,
  prefix: string
): Promise<boolean> {
  return await invoke('is_prefix_known', { profileId, bucketName, prefix })
}

export async function getPrefixStatus(
  profileId: string,
  bucketName: string,
  prefix: string
): Promise<PrefixStatus | null> {
  return await invoke('get_prefix_status', { profileId, bucketName, prefix })
}

export async function isPrefixDiscoveredOnly(
  profileId: string,
  bucketName: string,
  prefix: string
): Promise<boolean> {
  return await invoke('is_prefix_discovered_only', { profileId, bucketName, prefix })
}

export async function searchObjectsInIndex(
  profileId: string,
  bucketName: string,
  query: string,
  prefix?: string,
  limit?: number
): Promise<S3Object[]> {
  return await invoke('search_objects_in_index', { profileId, bucketName, query, prefix, limit })
}

export async function getAllBucketIndexes(
  profileId: string
): Promise<BucketIndexMetadata[]> {
  return await invoke('get_all_bucket_indexes', { profileId })
}

export async function getIndexFileSize(
  profileId: string
): Promise<number> {
  return await invoke('get_index_file_size', { profileId })
}

// ============================================================================
// Cache Management
// ============================================================================

/** Metrics snapshot for a single cache */
export interface CacheMetricsSnapshot {
  hits: number
  misses: number
  evictions: number
  insertions: number
  hit_rate: number
}

/** Status of a single cache */
export interface CacheStatus {
  name: string
  entries: number
  max_entries: number
  idle_timeout_secs: number
  ttl_secs: number | null
  metrics: CacheMetricsSnapshot
}

/** Status of all caches */
export interface AllCachesStatus {
  database_managers: CacheStatus
  index_managers: CacheStatus
}

/**
 * Get status of all caches (for monitoring/debug)
 * Returns metrics (hits, misses, evictions) and configuration
 */
export async function getCacheStatus(): Promise<AllCachesStatus> {
  return await invoke('get_cache_status')
}

/**
 * Warmup cache for a profile (preload managers)
 * Creates DatabaseManager and IndexManager in advance to avoid latency
 * Ideal to call on profile hover in UI
 */
export async function warmupProfileCache(profileId: string): Promise<void> {
  return await invoke('warmup_profile_cache', { profileId })
}

/**
 * Cleanup cache for a specific profile
 * Call when deleting a profile to free resources immediately
 */
export async function cleanupProfileCache(profileId: string): Promise<void> {
  return await invoke('cleanup_profile_cache', { profileId })
}

/**
 * Clear all caches (maintenance)
 * Frees all cached resources. Managers will be recreated on demand.
 */
export async function clearAllCaches(): Promise<void> {
  return await invoke('clear_all_caches')
}

// ============================================================================
// Clipboard Upload Operations
// ============================================================================

/**
 * Upload data directly from bytes (for clipboard images/text)
 * Writes data to a temp file and uses existing upload infrastructure
 * @returns Upload ID for tracking progress
 */
export async function uploadFromBytes(
  profileId: string,
  bucket: string,
  key: string,
  data: number[] | Uint8Array,
  contentType: string
): Promise<string> {
  return await invoke('upload_from_bytes', {
    profileId,
    bucket,
    key,
    data: Array.from(data),
    contentType,
  })
}

/**
 * Read file paths from the system clipboard
 * Returns a list of file paths if the clipboard contains copied files
 */
export async function readClipboardFiles(): Promise<string[]> {
  return await invoke('read_clipboard_files')
}
