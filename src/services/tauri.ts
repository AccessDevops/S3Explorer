import { invoke } from '@tauri-apps/api/tauri'
import type {
  Profile,
  TestConnectionResponse,
  Bucket,
  ListObjectsResponse,
  GetObjectResponse,
  PresignedUrlResponse,
  ListObjectVersionsResponse,
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

// Multipart Upload Operations
export async function multipartUploadStart(
  profileId: string,
  bucket: string,
  key: string,
  contentType?: string
): Promise<{ upload_id: string }> {
  return await invoke('multipart_upload_start', {
    profileId,
    bucket,
    key,
    contentType,
  })
}

export async function multipartUploadPart(
  profileId: string,
  bucket: string,
  key: string,
  uploadId: string,
  partNumber: number,
  data: number[] | Uint8Array
): Promise<{ e_tag: string }> {
  return await invoke('multipart_upload_part', {
    profileId,
    bucket,
    key,
    uploadId,
    partNumber,
    data,
  })
}

export async function multipartUploadPartFromFile(
  profileId: string,
  bucket: string,
  key: string,
  uploadId: string,
  partNumber: number,
  filePath: string,
  offset: number,
  length: number
): Promise<{ e_tag: string }> {
  return await invoke('multipart_upload_part_from_file', {
    profileId,
    bucket,
    key,
    uploadId,
    partNumber,
    filePath,
    offset,
    length,
  })
}

export async function multipartUploadComplete(
  profileId: string,
  bucket: string,
  key: string,
  uploadId: string,
  parts: Array<{ part_number: number; e_tag: string }>
): Promise<void> {
  return await invoke('multipart_upload_complete', {
    profileId,
    bucket,
    key,
    uploadId,
    parts,
  })
}

export async function multipartUploadAbort(
  profileId: string,
  bucket: string,
  key: string,
  uploadId: string
): Promise<void> {
  return await invoke('multipart_upload_abort', {
    profileId,
    bucket,
    key,
    uploadId,
  })
}

// New unified upload command (Rust-managed with progress events)
export async function uploadFile(
  profileId: string,
  bucket: string,
  key: string,
  filePath: string,
  contentType?: string
): Promise<string> {
  return await invoke('upload_file', {
    profileId,
    bucket,
    key,
    filePath,
    contentType,
  })
}

export async function cancelUpload(uploadId: string): Promise<void> {
  return await invoke('cancel_upload', {
    uploadId,
  })
}
