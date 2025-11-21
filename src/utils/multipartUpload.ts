import {
  multipartUploadStart,
  multipartUploadPart,
  multipartUploadPartFromFile,
  multipartUploadComplete,
  multipartUploadAbort,
} from '../services/tauri'
import type { MultipartProgress, CompletedPart } from '../types'
import { throttleAnimationFrame } from './throttle'
import { requestGarbageCollection, checkMemoryThreshold } from './memoryMonitor'

// Taille de chaque part : 10MB (minimum S3 = 5MB, sauf la dernière)
const PART_SIZE = 10 * 1024 * 1024 // 10MB

// Limite maximale de concurrence par fichier
export const MAX_CONCURRENT_PER_FILE = 10

// Limite maximale totale pour tous les fichiers uploadés simultanément
export const MAX_TOTAL_CONCURRENT = 20

/**
 * Calculate optimal concurrency based on file size to balance speed and memory usage
 * Large files use fewer concurrent uploads to avoid excessive RAM consumption
 *
 * @param fileSize - Size of the file in bytes
 * @param maxLimit - Optional maximum limit (useful when uploading multiple files)
 */
function getConcurrency(fileSize: number, maxLimit?: number): number {
  // Calculate ideal concurrency based on file size
  let ideal: number
  if (fileSize > 5 * 1024 * 1024 * 1024) {
    ideal = 2 // 5GB: 2 threads (~20MB RAM)
  } else if (fileSize > 1 * 1024 * 1024 * 1024) {
    ideal = 5 // 1GB: 5 threads (~50MB RAM)
  } else if (fileSize > 100 * 1024 * 1024) {
    ideal = 10 // 100MB: 10 threads (~100MB RAM)
  } else {
    ideal = 20 // Small files: 20 threads (~200MB RAM)
  }

  // Apply per-file limit
  ideal = Math.min(ideal, MAX_CONCURRENT_PER_FILE)

  // Apply custom limit if provided (for multi-file uploads)
  if (maxLimit !== undefined) {
    ideal = Math.min(ideal, maxLimit)
  }

  return Math.max(1, ideal) // Ensure at least 1 thread
}

/**
 * Calculate the maximum concurrency per file when uploading multiple files
 * Distributes MAX_TOTAL_CONCURRENT across active uploads
 *
 * @param activeUploadsCount - Number of files currently being uploaded
 * @returns Maximum concurrency to use per file
 */
export function getConcurrencyForMultipleFiles(activeUploadsCount: number): number {
  if (activeUploadsCount <= 0) return MAX_CONCURRENT_PER_FILE

  // Distribute total concurrency across active uploads
  const perFile = Math.floor(MAX_TOTAL_CONCURRENT / activeUploadsCount)

  // Ensure at least 1 thread per file, but respect MAX_CONCURRENT_PER_FILE
  return Math.max(1, Math.min(perFile, MAX_CONCURRENT_PER_FILE))
}

export interface UploadLargeFileOptions {
  profileId: string
  bucket: string
  key: string
  file: File
  contentType?: string
  onProgress?: (progress: MultipartProgress) => void
  partSize?: number
  concurrentUploads?: number
  signal?: AbortSignal // Signal pour annuler l'upload
}

/**
 * Upload a large file using S3 multipart upload
 * Automatically splits the file into parts and uploads them in parallel
 */
export async function uploadLargeFile(options: UploadLargeFileOptions): Promise<void> {
  const {
    profileId,
    bucket,
    key,
    file,
    contentType,
    onProgress,
    partSize = PART_SIZE,
    concurrentUploads = getConcurrency(options.file.size),
    signal,
  } = options

  const totalParts = Math.ceil(file.size / partSize)
  let uploadedBytes = 0
  let uploadId: string | null = null

  // Throttle progress updates to prevent UI lag
  const throttledProgress = onProgress ? throttleAnimationFrame(onProgress) : null

  // Check if already aborted
  if (signal?.aborted) {
    if (throttledProgress) throttledProgress.cancel()
    throw new Error('Upload cancelled')
  }

  try {
    // Step 1: Initiate multipart upload
    const initResponse = await multipartUploadStart(profileId, bucket, key, contentType)
    uploadId = initResponse.upload_id

    // Check for cancellation after init
    if (signal?.aborted) {
      throw new Error('Upload cancelled')
    }

    // Step 2: Upload parts with concurrency control
    const completedParts: CompletedPart[] = []
    const partNumbers = Array.from({ length: totalParts }, (_, i) => i + 1)

    // Upload parts in batches to control concurrency
    for (let i = 0; i < partNumbers.length; i += concurrentUploads) {
      // Check for cancellation before each batch
      if (signal?.aborted) {
        throw new Error('Upload cancelled')
      }

      const batch = partNumbers.slice(i, i + concurrentUploads)

      // Upload this batch in parallel
      const batchPromises = batch.map(async (partNumber) => {
        // Check for cancellation before uploading each part
        if (signal?.aborted) {
          throw new Error('Upload cancelled')
        }

        const start = (partNumber - 1) * partSize
        const end = Math.min(start + partSize, file.size)
        const chunk = file.slice(start, end)
        let arrayBuffer: ArrayBuffer | null = await chunk.arrayBuffer()
        let bytes: Uint8Array | null = new Uint8Array(arrayBuffer)

        const partResponse = await multipartUploadPart(
          profileId,
          bucket,
          key,
          uploadId!,
          partNumber,
          bytes
        )

        // Explicit memory cleanup to help GC
        bytes = null
        arrayBuffer = null

        // Update progress (throttled to prevent UI lag)
        uploadedBytes += end - start
        const progress: MultipartProgress = {
          uploadedParts: completedParts.length + 1,
          totalParts,
          uploadedBytes,
          totalBytes: file.size,
          percentage: (uploadedBytes / file.size) * 100,
        }

        throttledProgress?.(progress)

        return {
          part_number: partNumber,
          e_tag: partResponse.e_tag,
        }
      })

      // Wait for this batch to complete
      const batchResults = await Promise.all(batchPromises)
      completedParts.push(...batchResults)

      // Request garbage collection after each batch to free memory
      requestGarbageCollection()

      // Check memory usage and warn if getting high
      const memStatus = checkMemoryThreshold()
      if (memStatus === 'critical') {
        console.warn('⚠️ Critical memory usage during upload. Consider reducing concurrency.')
      }
    }

    // Check for cancellation before completing
    if (signal?.aborted) {
      if (throttledProgress) throttledProgress.cancel()
      throw new Error('Upload cancelled')
    }

    // Flush any pending progress update
    if (throttledProgress) throttledProgress.flush()

    // Step 3: Complete the multipart upload
    // Sort parts by part number (S3 requires this)
    completedParts.sort((a, b) => a.part_number - b.part_number)

    await multipartUploadComplete(profileId, bucket, key, uploadId, completedParts)
  } catch (error) {
    // Cancel any pending progress updates
    if (throttledProgress) throttledProgress.cancel()

    // Abort the upload on error (including cancellation)
    if (uploadId) {
      try {
        await multipartUploadAbort(profileId, bucket, key, uploadId)
      } catch (abortError) {
        console.error('Failed to abort multipart upload:', abortError)
      }
    }
    throw error
  }
}

/**
 * Upload a large file using S3 multipart upload (optimized version using file path)
 * Reads file chunks directly from disk without loading entire file in memory
 */
export interface UploadLargeFileFromPathOptions {
  profileId: string
  bucket: string
  key: string
  filePath: string
  fileSize: number
  contentType?: string
  onProgress?: (progress: MultipartProgress) => void
  partSize?: number
  concurrentUploads?: number
  signal?: AbortSignal
}

export async function uploadLargeFileFromPath(
  options: UploadLargeFileFromPathOptions
): Promise<void> {
  const {
    profileId,
    bucket,
    key,
    filePath,
    fileSize,
    contentType,
    onProgress,
    partSize = PART_SIZE,
    concurrentUploads = getConcurrency(options.fileSize),
    signal,
  } = options

  const totalParts = Math.ceil(fileSize / partSize)
  let uploadedBytes = 0
  let uploadId: string | null = null

  // Throttle progress updates to prevent UI lag
  const throttledProgress = onProgress ? throttleAnimationFrame(onProgress) : null

  // Check if already aborted
  if (signal?.aborted) {
    if (throttledProgress) throttledProgress.cancel()
    throw new Error('Upload cancelled')
  }

  try {
    // Step 1: Initiate multipart upload
    const initResponse = await multipartUploadStart(profileId, bucket, key, contentType)
    uploadId = initResponse.upload_id

    // Check for cancellation after init
    if (signal?.aborted) {
      if (throttledProgress) throttledProgress.cancel()
      throw new Error('Upload cancelled')
    }

    // Step 2: Upload parts with concurrency control
    const completedParts: CompletedPart[] = []
    const partNumbers = Array.from({ length: totalParts }, (_, i) => i + 1)

    // Upload parts in batches to control concurrency
    for (let i = 0; i < partNumbers.length; i += concurrentUploads) {
      // Check for cancellation before each batch
      if (signal?.aborted) {
        if (throttledProgress) throttledProgress.cancel()
        throw new Error('Upload cancelled')
      }

      const batch = partNumbers.slice(i, i + concurrentUploads)

      // Upload this batch in parallel
      const batchPromises = batch.map(async (partNumber) => {
        // Check for cancellation before uploading each part
        if (signal?.aborted) {
          if (throttledProgress) throttledProgress.cancel()
          throw new Error('Upload cancelled')
        }

        const offset = (partNumber - 1) * partSize
        const length = Math.min(partSize, fileSize - offset)

        // Read and upload part directly from file (no Array conversion!)
        const partResponse = await multipartUploadPartFromFile(
          profileId,
          bucket,
          key,
          uploadId!,
          partNumber,
          filePath,
          offset,
          length
        )

        // Update progress (throttled to prevent UI lag)
        uploadedBytes += length
        const progress: MultipartProgress = {
          uploadedParts: completedParts.length + 1,
          totalParts,
          uploadedBytes,
          totalBytes: fileSize,
          percentage: (uploadedBytes / fileSize) * 100,
        }

        throttledProgress?.(progress)

        return {
          part_number: partNumber,
          e_tag: partResponse.e_tag,
        }
      })

      // Wait for this batch to complete
      const batchResults = await Promise.all(batchPromises)
      completedParts.push(...batchResults)

      // Request garbage collection after each batch to free memory
      requestGarbageCollection()

      // Check memory usage and warn if getting high
      const memStatus = checkMemoryThreshold()
      if (memStatus === 'critical') {
        console.warn('⚠️ Critical memory usage during upload. Consider reducing concurrency.')
      }
    }

    // Check for cancellation before completing
    if (signal?.aborted) {
      if (throttledProgress) throttledProgress.cancel()
      throw new Error('Upload cancelled')
    }

    // Flush any pending progress update
    if (throttledProgress) throttledProgress.flush()

    // Step 3: Complete the multipart upload
    // Sort parts by part number (S3 requires this)
    completedParts.sort((a, b) => a.part_number - b.part_number)

    await multipartUploadComplete(profileId, bucket, key, uploadId, completedParts)
  } catch (error) {
    // Cancel any pending progress updates
    if (throttledProgress) throttledProgress.cancel()

    // Abort the upload on error (including cancellation)
    if (uploadId) {
      try {
        await multipartUploadAbort(profileId, bucket, key, uploadId)
      } catch (abortError) {
        console.error('Failed to abort multipart upload:', abortError)
      }
    }
    throw error
  }
}

/**
 * Check if a file should use multipart upload
 * Files larger than 50MB should use multipart
 */
export function shouldUseMultipartUpload(fileSize: number): boolean {
  const MULTIPART_THRESHOLD = 50 * 1024 * 1024 // 50MB
  return fileSize >= MULTIPART_THRESHOLD
}
