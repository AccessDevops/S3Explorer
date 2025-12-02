import { ref, computed, onMounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { DownloadProgressEvent } from '../types'
import { downloadFile as downloadFileService, cancelDownload as cancelDownloadService } from '../services/tauri'

// Download task with metadata
export interface RustDownloadTask {
  downloadId: string
  fileName: string
  fileSize: number
  downloadedBytes: number
  percentage: number
  status: 'pending' | 'starting' | 'downloading' | 'completed' | 'failed' | 'cancelled'
  error?: string
  startTime: number
  bytesPerSecond?: number
}

const downloads = ref<Map<string, RustDownloadTask>>(new Map())
let downloadListenerSetup = false

// ============================================================================
// PERFORMANCE OPTIMIZATION: Throttle progress updates
// ============================================================================
const PROGRESS_THROTTLE_MS = 100
const lastProgressUpdate = new Map<string, number>()

export function useRustDownloadManager() {
  /**
   * Setup global download progress listener (called once, persists for app lifetime)
   */
  const setupDownloadListener = async () => {
    if (downloadListenerSetup) return // Already setup
    downloadListenerSetup = true

    try {
      await listen<DownloadProgressEvent>('download:progress', (event) => {
      const progress = event.payload

      // Throttle non-terminal states
      const isTerminalState = ['completed', 'failed', 'cancelled'].includes(progress.status)

      if (!isTerminalState && progress.status === 'downloading') {
        const now = Date.now()
        const lastUpdate = lastProgressUpdate.get(progress.download_id) || 0

        if (now - lastUpdate < PROGRESS_THROTTLE_MS) {
          // Skip this update - too soon since last one
          return
        }
        lastProgressUpdate.set(progress.download_id, now)
      }

      // Get existing task or create new one
      let task = downloads.value.get(progress.download_id)

      if (!task) {
        // New download, create task
        task = {
          downloadId: progress.download_id,
          fileName: progress.file_name,
          fileSize: progress.file_size,
          downloadedBytes: progress.downloaded_bytes,
          percentage: progress.percentage,
          status: progress.status,
          error: progress.error,
          startTime: Date.now(),
          bytesPerSecond: progress.bytes_per_second,
        }
      } else {
        // Update existing task
        task.downloadedBytes = progress.downloaded_bytes
        task.percentage = progress.percentage
        task.status = progress.status
        task.error = progress.error
        task.bytesPerSecond = progress.bytes_per_second
      }

      downloads.value.set(progress.download_id, task)

      // Auto-remove completed/failed/cancelled downloads after 5 seconds
      if (isTerminalState) {
        setTimeout(() => {
          downloads.value.delete(progress.download_id)
          lastProgressUpdate.delete(progress.download_id)
        }, 5000)
      }
    })
    } catch (error) {
      // Reset flag on error so next mount can retry
      downloadListenerSetup = false
      console.error('Failed to setup download progress listener:', error)
    }
  }

  /**
   * Start a download (streaming to disk)
   */
  const startDownload = async (
    profileId: string,
    bucket: string,
    key: string,
    destPath: string
  ): Promise<string> => {
    return await downloadFileService(profileId, bucket, key, destPath)
  }

  /**
   * Cancel an active download
   */
  const cancelDownload = async (downloadId: string) => {
    await cancelDownloadService(downloadId)
  }

  /**
   * Remove a download from the list (manual cleanup)
   */
  const removeDownload = (downloadId: string) => {
    downloads.value.delete(downloadId)
    lastProgressUpdate.delete(downloadId)
  }

  /**
   * Clear all completed/cancelled/failed downloads
   */
  const clearFinished = () => {
    downloads.value.forEach((task, id) => {
      if (task.status === 'completed' || task.status === 'cancelled' || task.status === 'failed') {
        downloads.value.delete(id)
        lastProgressUpdate.delete(id)
      }
    })
  }

  /**
   * Cancel all active downloads
   */
  const cancelAll = async () => {
    const cancelPromises: Promise<void>[] = []

    downloads.value.forEach((task) => {
      if (
        task.status === 'downloading' ||
        task.status === 'pending' ||
        task.status === 'starting'
      ) {
        cancelPromises.push(cancelDownload(task.downloadId))
      }
    })

    await Promise.all(cancelPromises)
  }

  // ============================================================================
  // Computed properties for download statistics
  // ============================================================================

  const downloadStats = computed(() => {
    const all: RustDownloadTask[] = []
    const active: RustDownloadTask[] = []
    let completed = 0
    let failed = 0
    let cancelled = 0

    // Single iteration over all downloads
    for (const task of downloads.value.values()) {
      all.push(task)

      switch (task.status) {
        case 'downloading':
        case 'pending':
        case 'starting':
          active.push(task)
          break
        case 'completed':
          completed++
          break
        case 'failed':
          failed++
          break
        case 'cancelled':
          cancelled++
          break
      }
    }

    return {
      all,
      active,
      completed,
      failed,
      cancelled,
      total: all.length,
    }
  })

  // Derived computed values
  const activeDownloads = computed(() => downloadStats.value.active)
  const allDownloads = computed(() => downloadStats.value.all)
  const hasActiveDownloads = computed(() => downloadStats.value.active.length > 0)

  const downloadCount = computed(() => ({
    total: downloadStats.value.total,
    active: downloadStats.value.active.length,
    completed: downloadStats.value.completed,
    failed: downloadStats.value.failed,
  }))

  /**
   * Calculate estimated time remaining for all active downloads in seconds
   */
  const totalTimeRemaining = computed(() => {
    let totalSeconds = 0

    activeDownloads.value.forEach((task) => {
      if (task.bytesPerSecond && task.bytesPerSecond > 0) {
        const remainingBytes = task.fileSize - task.downloadedBytes
        totalSeconds += remainingBytes / task.bytesPerSecond
      }
    })

    return totalSeconds
  })

  /**
   * Calculate estimated time remaining for a specific download (in seconds)
   */
  const getTimeRemaining = (downloadId: string): number | null => {
    const task = downloads.value.get(downloadId)
    if (!task || !task.bytesPerSecond || task.bytesPerSecond <= 0) {
      return null
    }

    const remainingBytes = task.fileSize - task.downloadedBytes
    return remainingBytes / task.bytesPerSecond
  }

  /**
   * Get download speed for a specific download (bytes per second)
   */
  const getDownloadSpeed = (downloadId: string): number | null => {
    const task = downloads.value.get(downloadId)
    return task?.bytesPerSecond ?? null
  }

  /**
   * Format bytes per second to human readable string
   */
  const formatSpeed = (bytesPerSecond: number): string => {
    if (bytesPerSecond < 1024) {
      return `${bytesPerSecond.toFixed(0)} B/s`
    } else if (bytesPerSecond < 1024 * 1024) {
      return `${(bytesPerSecond / 1024).toFixed(1)} KB/s`
    } else if (bytesPerSecond < 1024 * 1024 * 1024) {
      return `${(bytesPerSecond / (1024 * 1024)).toFixed(1)} MB/s`
    } else {
      return `${(bytesPerSecond / (1024 * 1024 * 1024)).toFixed(2)} GB/s`
    }
  }

  // Setup global listener on first component mount (persists for app lifetime)
  onMounted(() => {
    setupDownloadListener()
  })

  return {
    downloads: allDownloads,
    activeDownloads,
    hasActiveDownloads,
    downloadCount,
    totalTimeRemaining,
    startDownload,
    cancelDownload,
    removeDownload,
    getTimeRemaining,
    getDownloadSpeed,
    formatSpeed,
    clearFinished,
    cancelAll,
  }
}
