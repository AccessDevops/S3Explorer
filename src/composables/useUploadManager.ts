import { ref, computed } from 'vue'
import type { MultipartProgress } from '../types'

export interface UploadTask {
  id: string
  fileName: string
  fileSize: number
  status: 'pending' | 'uploading' | 'completed' | 'failed' | 'cancelled'
  progress: MultipartProgress | null
  error?: string
  abortController: AbortController
  isMultipart: boolean
  startTime: number
}

const uploads = ref<Map<string, UploadTask>>(new Map())
let uploadIdCounter = 0

export function useUploadManager() {
  /**
   * Create a new upload task
   */
  const createUpload = (fileName: string, fileSize: number, isMultipart: boolean): string => {
    const id = `upload-${++uploadIdCounter}-${Date.now()}`
    const task: UploadTask = {
      id,
      fileName,
      fileSize,
      status: 'pending',
      progress: null,
      abortController: new AbortController(),
      isMultipart,
      startTime: Date.now(),
    }

    uploads.value.set(id, task)
    return id
  }

  /**
   * Update upload progress
   */
  const updateProgress = (id: string, progress: MultipartProgress) => {
    const task = uploads.value.get(id)
    if (task) {
      task.progress = progress
      task.status = 'uploading'
    }
  }

  /**
   * Mark upload as completed
   */
  const completeUpload = (id: string) => {
    const task = uploads.value.get(id)
    if (task) {
      task.status = 'completed'
      task.progress = task.progress
        ? { ...task.progress, percentage: 100 }
        : {
            uploadedParts: 1,
            totalParts: 1,
            uploadedBytes: task.fileSize,
            totalBytes: task.fileSize,
            percentage: 100,
          }

      // Auto-remove completed uploads after 5 seconds
      setTimeout(() => {
        removeUpload(id)
      }, 5000)
    }
  }

  /**
   * Mark upload as failed
   */
  const failUpload = (id: string, error: string) => {
    const task = uploads.value.get(id)
    if (task) {
      task.status = 'failed'
      task.error = error
    }
  }

  /**
   * Cancel an upload
   */
  const cancelUpload = (id: string) => {
    const task = uploads.value.get(id)
    if (task) {
      task.abortController.abort()
      task.status = 'cancelled'
    }
  }

  /**
   * Remove an upload from the list
   */
  const removeUpload = (id: string) => {
    uploads.value.delete(id)
  }

  /**
   * Get AbortSignal for an upload
   */
  const getSignal = (id: string): AbortSignal | undefined => {
    return uploads.value.get(id)?.abortController.signal
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
   * Cancel all uploads
   */
  const cancelAll = () => {
    uploads.value.forEach((task) => {
      if (task.status === 'uploading' || task.status === 'pending') {
        task.abortController.abort()
        task.status = 'cancelled'
      }
    })
  }

  // Computed values
  const activeUploads = computed(() => {
    return Array.from(uploads.value.values()).filter(
      (task) => task.status === 'uploading' || task.status === 'pending'
    )
  })

  const hasActiveUploads = computed(() => activeUploads.value.length > 0)

  const allUploads = computed(() => Array.from(uploads.value.values()))

  const uploadCount = computed(() => ({
    total: uploads.value.size,
    active: activeUploads.value.length,
    completed: Array.from(uploads.value.values()).filter((t) => t.status === 'completed').length,
    failed: Array.from(uploads.value.values()).filter((t) => t.status === 'failed').length,
  }))

  /**
   * Calculate estimated time remaining for all active uploads (in seconds)
   */
  const totalTimeRemaining = computed(() => {
    let totalSeconds = 0

    activeUploads.value.forEach((task) => {
      if (task.progress && task.progress.uploadedBytes > 0) {
        const elapsedMs = Date.now() - task.startTime
        const elapsedSeconds = elapsedMs / 1000
        const bytesPerSecond = task.progress.uploadedBytes / elapsedSeconds

        if (bytesPerSecond > 0) {
          const remainingBytes = task.progress.totalBytes - task.progress.uploadedBytes
          totalSeconds += remainingBytes / bytesPerSecond
        }
      }
    })

    return totalSeconds
  })

  /**
   * Calculate estimated time remaining for a specific upload (in seconds)
   */
  const getTimeRemaining = (uploadId: string): number | null => {
    const task = uploads.value.get(uploadId)
    if (!task || !task.progress || task.progress.uploadedBytes === 0) {
      return null
    }

    const elapsedMs = Date.now() - task.startTime
    const elapsedSeconds = elapsedMs / 1000
    const bytesPerSecond = task.progress.uploadedBytes / elapsedSeconds

    if (bytesPerSecond > 0) {
      const remainingBytes = task.progress.totalBytes - task.progress.uploadedBytes
      return remainingBytes / bytesPerSecond
    }

    return null
  }

  return {
    uploads: allUploads,
    activeUploads,
    hasActiveUploads,
    uploadCount,
    totalTimeRemaining,
    createUpload,
    updateProgress,
    completeUpload,
    failUpload,
    cancelUpload,
    removeUpload,
    getSignal,
    getTimeRemaining,
    clearFinished,
    cancelAll,
  }
}
