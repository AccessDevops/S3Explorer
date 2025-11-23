import { ref, computed, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { UploadProgressEvent } from '../types'
import { uploadFile as uploadFileService, cancelUpload as cancelUploadService } from '../services/tauri'
import { useAppStore } from '../stores/app'
import { useSettingsStore } from '../stores/settings'
import { useBucketStatsInvalidation } from './useBucketStatsInvalidation'

// Upload task with metadata
export interface RustUploadTask {
  uploadId: string
  fileName: string
  fileSize: number
  uploadedBytes: number
  uploadedParts: number
  totalParts: number
  percentage: number
  status: 'pending' | 'starting' | 'uploading' | 'completed' | 'failed' | 'cancelled' | 'queued'
  error?: string
  startTime: number
  bucket?: string // Bucket name for stats invalidation
}

// Queued upload request
interface QueuedUpload {
  queueId: string // Unique ID for queue item (before upload starts)
  fileName: string
  fileSize: number
  profileId: string
  bucket: string
  key: string
  filePath: string
  contentType?: string
  resolve: (uploadId: string) => void
  reject: (error: any) => void
}

const uploads = ref<Map<string, RustUploadTask>>(new Map())
const uploadQueue = ref<QueuedUpload[]>([])
const uploadBuckets = new Map<string, string>() // Maps uploadId -> bucket name
let unlisten: UnlistenFn | null = null
let reloadTimeout: number | null = null
let lastReloadTime: number = 0
let isProcessingQueue = false
let queueIdCounter = 0

// Minimum time between reloads (3 seconds)
const RELOAD_COOLDOWN = 3000

export function useRustUploadManager() {
  const appStore = useAppStore()
  const settingsStore = useSettingsStore()
  const { invalidateBucketStats } = useBucketStatsInvalidation()

  /**
   * Smart reload with 3-second cooldown
   * - If a reload is already scheduled, do nothing (prevents unnecessary reloads)
   * - If last reload was < 3s ago, schedule reload for 3s after last reload
   * - Otherwise, reload immediately
   */
  const scheduleReload = () => {
    // If a reload is already scheduled, skip (prevents duplicate reloads)
    if (reloadTimeout !== null) {
      return
    }

    const now = Date.now()
    const timeSinceLastReload = now - lastReloadTime
    const cooldownRemaining = RELOAD_COOLDOWN - timeSinceLastReload

    // Calculate delay: wait for cooldown to expire, or reload immediately
    const delay = Math.max(0, cooldownRemaining)

    reloadTimeout = window.setTimeout(() => {
      appStore.loadObjects()
      lastReloadTime = Date.now()
      reloadTimeout = null
    }, delay)
  }

  /**
   * Start listening to upload progress events from Rust
   */
  const startListening = async () => {
    if (unlisten) return // Already listening

    unlisten = await listen<UploadProgressEvent>('upload:progress', (event) => {
      const progress = event.payload

      // Get existing task or create new one
      let task = uploads.value.get(progress.upload_id)

      if (!task) {
        // New upload, create task
        task = {
          uploadId: progress.upload_id,
          fileName: progress.file_name,
          fileSize: progress.file_size,
          uploadedBytes: progress.uploaded_bytes,
          uploadedParts: progress.uploaded_parts,
          totalParts: progress.total_parts,
          percentage: progress.percentage,
          status: progress.status,
          error: progress.error,
          startTime: Date.now(),
          bucket: uploadBuckets.get(progress.upload_id), // Get bucket from our map
        }
      } else {
        // Update existing task
        task.uploadedBytes = progress.uploaded_bytes
        task.uploadedParts = progress.uploaded_parts
        task.totalParts = progress.total_parts
        task.percentage = progress.percentage
        task.status = progress.status
        task.error = progress.error
      }

      uploads.value.set(progress.upload_id, task)

      // Reload objects when upload completes successfully
      if (task.status === 'completed') {
        scheduleReload()

        // Invalidate bucket stats to trigger refresh
        if (task.bucket) {
          invalidateBucketStats(task.bucket)
        }
      }

      // Auto-remove completed/failed/cancelled uploads after 5 seconds
      if (task.status === 'completed' || task.status === 'failed' || task.status === 'cancelled') {
        setTimeout(() => {
          uploads.value.delete(progress.upload_id)
          // Clean up bucket mapping
          uploadBuckets.delete(progress.upload_id)
        }, 5000)

        // Process queue when an upload finishes (freed up a slot)
        processQueue()
      }
    })
  }

  /**
   * Stop listening to upload progress events
   */
  const stopListening = () => {
    if (unlisten) {
      unlisten()
      unlisten = null
    }

    // Clear pending reload timeout
    if (reloadTimeout !== null) {
      clearTimeout(reloadTimeout)
      reloadTimeout = null
    }
  }

  /**
   * Process the upload queue - start uploads if under max concurrent limit
   */
  const processQueue = async () => {
    // Prevent concurrent queue processing
    if (isProcessingQueue) return
    if (uploadQueue.value.length === 0) return

    isProcessingQueue = true

    try {
      // Count active uploads (uploading, pending, starting - NOT queued or completed)
      const activeCount = Array.from(uploads.value.values()).filter(
        (task) =>
          task.status === 'uploading' ||
          task.status === 'pending' ||
          task.status === 'starting'
      ).length

      const maxConcurrent = settingsStore.maxConcurrentUploads
      const availableSlots = maxConcurrent - activeCount

      if (availableSlots <= 0) {
        isProcessingQueue = false
        return
      }

      // Start as many uploads as we have available slots
      const toStart = uploadQueue.value.slice(0, availableSlots)

      // Remove all from queue first
      toStart.forEach((queuedUpload) => {
        const index = uploadQueue.value.findIndex((q) => q.queueId === queuedUpload.queueId)
        if (index !== -1) {
          uploadQueue.value.splice(index, 1)
        }

        // Change phantom status to 'starting'
        const phantomTask = uploads.value.get(queuedUpload.queueId)
        if (phantomTask) {
          phantomTask.status = 'starting'
          uploads.value.set(queuedUpload.queueId, phantomTask)
        }
      })

      // Start all uploads in parallel (not sequential)
      const uploadPromises = toStart.map(async (queuedUpload) => {
        try {
          // Start the actual upload via Rust
          const uploadId = await uploadFileService(
            queuedUpload.profileId,
            queuedUpload.bucket,
            queuedUpload.key,
            queuedUpload.filePath,
            queuedUpload.contentType,
            settingsStore.multipartThresholdMB
          )

          // Store bucket mapping for this upload
          uploadBuckets.set(uploadId, queuedUpload.bucket)

          // Now remove the phantom - the real upload will appear via events
          uploads.value.delete(queuedUpload.queueId)

          // Resolve the promise with the upload ID
          queuedUpload.resolve(uploadId)
        } catch (error) {
          // Remove the phantom upload on error
          uploads.value.delete(queuedUpload.queueId)

          // Reject the promise if upload fails to start
          queuedUpload.reject(error)
        }
      })

      // Wait for all uploads to start
      await Promise.all(uploadPromises)
    } finally {
      isProcessingQueue = false

      // If there are still items in queue, try to process again
      // (in case some uploads failed immediately)
      if (uploadQueue.value.length > 0) {
        setTimeout(() => processQueue(), 100)
      }
    }
  }

  /**
   * Start an upload (adds to queue, respects max concurrent limit)
   */
  const startUpload = async (
    profileId: string,
    bucket: string,
    key: string,
    filePath: string,
    contentType?: string
  ): Promise<string> => {
    // Generate unique queue ID
    const queueId = `queue_${++queueIdCounter}_${Date.now()}`

    // Extract file name and get file size
    const fileName = filePath.split(/[\\/]/).pop() || 'unknown'

    // Get file size (we'll use a placeholder for now, the real size will come from Rust events)
    let fileSize = 0
    try {
      const { getFileSize } = await import('../services/tauri')
      fileSize = await getFileSize(filePath)
    } catch {
      // If we can't get file size, use 0 (will be updated by Rust events)
      fileSize = 0
    }

    // Create a promise that will be resolved when the upload actually starts
    return new Promise<string>((resolve, reject) => {
      // Add to queue
      const queuedUpload: QueuedUpload = {
        queueId,
        fileName,
        fileSize,
        profileId,
        bucket,
        key,
        filePath,
        contentType,
        resolve,
        reject,
      }

      uploadQueue.value.push(queuedUpload)

      // Create a "phantom" upload task to show in UI with 'queued' status
      const task: RustUploadTask = {
        uploadId: queueId, // Use queue ID as temporary upload ID
        fileName,
        fileSize,
        uploadedBytes: 0,
        uploadedParts: 0,
        totalParts: 0,
        percentage: 0,
        status: 'queued',
        startTime: Date.now(),
        bucket, // Store bucket name for stats invalidation
      }
      uploads.value.set(queueId, task)

      // Process queue (will start upload if slot available)
      processQueue()
    })
  }

  /**
   * Cancel an upload (handles both queued and active uploads)
   */
  const cancelUpload = async (uploadId: string) => {
    // Check if this is a queued upload (starts with "queue_")
    if (uploadId.startsWith('queue_')) {
      // Remove from queue
      const index = uploadQueue.value.findIndex((q) => q.queueId === uploadId)
      if (index !== -1) {
        const queuedUpload = uploadQueue.value[index]
        uploadQueue.value.splice(index, 1)

        // Reject the promise
        queuedUpload.reject(new Error('Upload cancelled'))

        // Remove from UI
        uploads.value.delete(uploadId)
      }
    } else {
      // Active upload - cancel via Rust
      await cancelUploadService(uploadId)
      // The status will be updated via the progress event
    }
  }

  /**
   * Remove an upload from the list (manual cleanup)
   */
  const removeUpload = (uploadId: string) => {
    uploads.value.delete(uploadId)
  }

  /**
   * Clear all completed/cancelled/failed uploads
   */
  const clearFinished = () => {
    uploads.value.forEach((task, id) => {
      if (task.status === 'completed' || task.status === 'cancelled' || task.status === 'failed') {
        uploads.value.delete(id)
      }
    })
  }

  /**
   * Cancel all active and queued uploads
   */
  const cancelAll = async () => {
    const cancelPromises: Promise<void>[] = []

    // Cancel all active and queued uploads
    uploads.value.forEach((task) => {
      if (
        task.status === 'uploading' ||
        task.status === 'pending' ||
        task.status === 'starting' ||
        task.status === 'queued'
      ) {
        cancelPromises.push(cancelUpload(task.uploadId))
      }
    })

    await Promise.all(cancelPromises)
  }

  // Computed values
  const activeUploads = computed(() => {
    return Array.from(uploads.value.values()).filter(
      (task) =>
        task.status === 'uploading' ||
        task.status === 'pending' ||
        task.status === 'starting'
    )
  })

  const queuedUploads = computed(() => {
    return Array.from(uploads.value.values()).filter((task) => task.status === 'queued')
  })

  const hasActiveUploads = computed(
    () => activeUploads.value.length > 0 || queuedUploads.value.length > 0
  )

  const allUploads = computed(() => Array.from(uploads.value.values()))

  const uploadCount = computed(() => ({
    total: uploads.value.size,
    active: activeUploads.value.length,
    queued: queuedUploads.value.length,
    completed: Array.from(uploads.value.values()).filter((t) => t.status === 'completed').length,
    failed: Array.from(uploads.value.values()).filter((t) => t.status === 'failed').length,
  }))

  /**
   * Calculate estimated time remaining for ALL uploads (active + queued) in seconds
   * Uses average upload speed from active uploads to estimate time for queued uploads
   * Handles small files (<50MB) that don't emit progress events
   */
  const totalTimeRemaining = computed(() => {
    let totalSeconds = 0
    let totalBytesPerSecond = 0
    let activeUploadingCount = 0
    let uploadsWithoutProgressBytes = 0

    // 1. Calculate time remaining for active uploads + collect speeds
    activeUploads.value.forEach((task) => {
      if (task.uploadedBytes > 0 && task.status === 'uploading') {
        // Upload with progress (multipart, ≥50MB)
        const elapsedMs = Date.now() - task.startTime
        const elapsedSeconds = elapsedMs / 1000
        const bytesPerSecond = task.uploadedBytes / elapsedSeconds

        if (bytesPerSecond > 0) {
          const remainingBytes = task.fileSize - task.uploadedBytes
          totalSeconds += remainingBytes / bytesPerSecond
          totalBytesPerSecond += bytesPerSecond
          activeUploadingCount++
        }
      } else if (task.status === 'uploading' || task.status === 'starting') {
        // Upload without progress (simple upload, <50MB)
        // Will estimate time using average speed from other uploads
        uploadsWithoutProgressBytes += task.fileSize
      }
    })

    // 2. Calculate average speed from uploads with progress
    const avgBytesPerSecond = activeUploadingCount > 0
      ? totalBytesPerSecond / activeUploadingCount
      : 0

    // 3. Estimate time for active uploads without progress (small files)
    if (avgBytesPerSecond > 0 && uploadsWithoutProgressBytes > 0) {
      totalSeconds += uploadsWithoutProgressBytes / avgBytesPerSecond
    }

    // 4. Estimate time for queued uploads based on average speed
    if (avgBytesPerSecond > 0 && queuedUploads.value.length > 0) {
      const maxConcurrent = settingsStore.maxConcurrentUploads

      // Calculate total size of queued uploads
      let queuedTotalBytes = 0
      queuedUploads.value.forEach((task) => {
        queuedTotalBytes += task.fileSize
      })

      // Estimated time = total size / (average speed × concurrency)
      // This assumes uploads will maintain current average speed
      const effectiveSpeed = avgBytesPerSecond * maxConcurrent
      if (effectiveSpeed > 0) {
        totalSeconds += queuedTotalBytes / effectiveSpeed
      }
    }

    return totalSeconds
  })

  /**
   * Calculate estimated time remaining for a specific upload (in seconds)
   */
  const getTimeRemaining = (uploadId: string): number | null => {
    const task = uploads.value.get(uploadId)
    if (!task || task.uploadedBytes === 0 || task.status !== 'uploading') {
      return null
    }

    const elapsedMs = Date.now() - task.startTime
    const elapsedSeconds = elapsedMs / 1000
    const bytesPerSecond = task.uploadedBytes / elapsedSeconds

    if (bytesPerSecond > 0) {
      const remainingBytes = task.fileSize - task.uploadedBytes
      return remainingBytes / bytesPerSecond
    }

    return null
  }

  /**
   * Calculate upload speed for a specific upload (bytes per second)
   */
  const getUploadSpeed = (uploadId: string): number | null => {
    const task = uploads.value.get(uploadId)
    if (!task || task.uploadedBytes === 0) {
      return null
    }

    const elapsedMs = Date.now() - task.startTime
    const elapsedSeconds = elapsedMs / 1000

    if (elapsedSeconds > 0) {
      return task.uploadedBytes / elapsedSeconds
    }

    return null
  }

  // Auto-start listening on mount
  onMounted(() => {
    startListening()
  })

  // Stop listening on unmount
  onUnmounted(() => {
    stopListening()
  })

  return {
    uploads: allUploads,
    activeUploads,
    queuedUploads,
    hasActiveUploads,
    uploadCount,
    totalTimeRemaining,
    startUpload,
    cancelUpload,
    removeUpload,
    getTimeRemaining,
    getUploadSpeed,
    clearFinished,
    cancelAll,
    startListening,
    stopListening,
  }
}
