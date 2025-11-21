// S3 Profile
export interface Profile {
  id: string
  name: string
  endpoint?: string
  region: string
  access_key: string
  secret_key: string
  session_token?: string
  path_style: boolean
  use_tls: boolean
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

// Error types
export * from './errors'
