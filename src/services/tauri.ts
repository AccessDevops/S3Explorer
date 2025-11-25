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

export async function getBucketAcl(profileId: string, bucketName: string): Promise<string> {
  return await invoke('get_bucket_acl', { profileId, bucketName })
}

export async function calculateBucketStats(
  profileId: string,
  bucketName: string
): Promise<[number, number]> {
  return await invoke('calculate_bucket_stats', { profileId, bucketName })
}

export async function estimateBucketStats(
  profileId: string,
  bucketName: string
): Promise<[number, number, boolean]> {
  return await invoke('estimate_bucket_stats', { profileId, bucketName })
}

// Object Operations
export async function listObjects(
  profileId: string,
  bucket: string,
  prefix?: string,
  continuationToken?: string,
  maxKeys?: number,
  useDelimiter?: boolean
): Promise<ListObjectsResponse> {
  return await invoke('list_objects', {
    profileId,
    bucket,
    prefix,
    continuationToken,
    maxKeys,
    useDelimiter,
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
  return await invoke('put_object', {
    profileId,
    bucket,
    key,
    content,
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
  prefix: string
): Promise<number> {
  return await invoke('calculate_folder_size', {
    profileId,
    bucket,
    prefix,
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
