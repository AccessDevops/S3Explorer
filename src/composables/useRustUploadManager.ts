import { ref, computed, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { UploadProgressEvent } from '../types'
import { uploadFile as uploadFileService, cancelUpload as cancelUploadService } from '../services/tauri'
import { useAppStore } from '../stores/app'

// Upload task with metadata
export interface RustUploadTask {
  uploadId: string
  fileName: string
  fileSize: number
  uploadedBytes: number
  uploadedParts: number
  totalParts: number
  percentage: number
  status: 'pending' | 'starting' | 'uploading' | 'completed' | 'failed' | 'cancelled'
  error?: string
  startTime: number
}

const uploads = ref<Map<string, RustUploadTask>>(new Map())
let unlisten: UnlistenFn | null = null
let reloadTimeout: number | null = null

export function useRustUploadManager() {
  const appStore = useAppStore()

  /**
   * Debounced reload of objects after upload completion
   */
  const scheduleReload = () => {
    // Clear existing timeout
    if (reloadTimeout !== null) {
      clearTimeout(reloadTimeout)
    }

    // Schedule reload after 500ms (debounce multiple completions)
    reloadTimeout = window.setTimeout(() => {
      appStore.loadObjects()
      reloadTimeout = null
    }, 500)
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
      }

      // Auto-remove completed/failed/cancelled uploads after 5 seconds
      if (task.status === 'completed' || task.status === 'failed' || task.status === 'cancelled') {
        setTimeout(() => {
          uploads.value.delete(progress.upload_id)
        }, 5000)
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
   * Start an upload (calls Rust command)
   */
  const startUpload = async (
    profileId: string,
    bucket: string,
    key: string,
    filePath: string,
    contentType?: string
  ): Promise<string> => {
    const uploadId = await uploadFileService(profileId, bucket, key, filePath, contentType)
    return uploadId
  }

  /**
   * Cancel an upload
   */
  const cancelUpload = async (uploadId: string) => {
    await cancelUploadService(uploadId)
    // The status will be updated via the progress event
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
   * Cancel all active uploads
   */
  const cancelAll = async () => {
    const cancelPromises: Promise<void>[] = []

    uploads.value.forEach((task) => {
      if (task.status === 'uploading' || task.status === 'pending' || task.status === 'starting') {
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
      if (task.uploadedBytes > 0 && task.status === 'uploading') {
        const elapsedMs = Date.now() - task.startTime
        const elapsedSeconds = elapsedMs / 1000
        const bytesPerSecond = task.uploadedBytes / elapsedSeconds

        if (bytesPerSecond > 0) {
          const remainingBytes = task.fileSize - task.uploadedBytes
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
