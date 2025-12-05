import { ref, onMounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  startInitialIndex,
  cancelIndexing as cancelIndexingService,
  getBucketIndexStats,
  getPrefixIndexStats,
  clearBucketIndex,
  isBucketIndexed,
  isBucketIndexComplete,
  searchObjectsInIndex,
  getAllBucketIndexes,
  getIndexFileSize as getIndexFileSizeService,
} from '../services/tauri'
import { useSettingsStore } from '../stores/settings'
import { logger } from '../utils/logger'
import type {
  BucketIndexStats,
  PrefixStats,
  InitialIndexResult,
  IndexProgressEvent,
  S3Object,
  BucketIndexMetadata,
} from '../types'

// Shared state across all instances
const indexingBuckets = ref<Record<string, boolean>>({})
const indexProgress = ref<Record<string, IndexProgressEvent>>({})
const bucketIndexStatus = ref<Record<string, { isComplete: boolean; lastIndexed: number | null }>>({})

// Track if event listener is already set up
let eventListenerSetup = false
let _unlistenProgress: UnlistenFn | null = null // Prefix with _ to indicate intentionally unused

/**
 * Setup the global event listener for index progress events
 * Only called once across all component instances
 */
async function setupGlobalEventListener() {
  if (eventListenerSetup) return
  eventListenerSetup = true

  try {
    _unlistenProgress = await listen<IndexProgressEvent>('index:progress', (event) => {
      const key = `${event.payload.profile_id}-${event.payload.bucket_name}`
      indexProgress.value[key] = event.payload

      if (event.payload.status === 'completed' || event.payload.status === 'partial') {
        indexingBuckets.value[key] = false
        bucketIndexStatus.value[key] = {
          isComplete: event.payload.is_complete,
          lastIndexed: Date.now(),
        }
      } else if (event.payload.status === 'failed' || event.payload.status === 'cancelled') {
        indexingBuckets.value[key] = false
        // For cancelled, we still have partial data indexed
        if (event.payload.status === 'cancelled' && event.payload.objects_indexed > 0) {
          bucketIndexStatus.value[key] = {
            isComplete: false,
            lastIndexed: Date.now(),
          }
        }
      } else {
        indexingBuckets.value[key] = true
      }
    })
  } catch (error) {
    logger.error('Failed to setup index progress listener', error)
    eventListenerSetup = false
  }
}

/**
 * Composable for managing bucket indexation
 */
export function useIndexManager() {
  // Setup event listener on first use
  onMounted(() => {
    setupGlobalEventListener()
  })

  /**
   * Start initial indexation for a bucket
   * @param profileId Profile ID
   * @param bucketName Bucket name
   * @param maxRequests Maximum number of ListObjectsV2 requests (default from settings, 0 = unlimited)
   */
  async function startIndexing(
    profileId: string,
    bucketName: string,
    maxRequests?: number
  ): Promise<InitialIndexResult | null> {
    const key = `${profileId}-${bucketName}`
    const settingsStore = useSettingsStore()

    try {
      indexingBuckets.value[key] = true

      // Use maxRequests from parameter or from settings
      // Special case: if maxRequests is explicitly 0, pass 0 for unlimited
      const effectiveMaxRequests = maxRequests !== undefined ? maxRequests : settingsStore.maxInitialIndexRequests

      // Pass batchSize from settings to backend
      const result = await startInitialIndex(
        profileId,
        bucketName,
        effectiveMaxRequests,
        settingsStore.batchSize
      )

      bucketIndexStatus.value[key] = {
        isComplete: result.is_complete,
        lastIndexed: Date.now(),
      }

      return result
    } catch (error) {
      logger.error('Failed to start indexing', error)
      return null
    } finally {
      indexingBuckets.value[key] = false
    }
  }

  /**
   * Start full indexation for a bucket (no request limit)
   * This will index ALL objects in the bucket regardless of size
   * @param profileId Profile ID
   * @param bucketName Bucket name
   */
  async function startFullIndexing(
    profileId: string,
    bucketName: string
  ): Promise<InitialIndexResult | null> {
    // Pass 0 as maxRequests to indicate no limit
    return startIndexing(profileId, bucketName, 0)
  }

  /**
   * Cancel an active indexing operation
   * The partial index is preserved and can be resumed later
   * @param profileId Profile ID
   * @param bucketName Bucket name
   * @returns true if cancellation was successful
   */
  async function cancelIndexing(
    profileId: string,
    bucketName: string
  ): Promise<boolean> {
    try {
      await cancelIndexingService(profileId, bucketName)
      return true
    } catch (error) {
      logger.error('Failed to cancel indexing', error)
      return false
    }
  }

  /**
   * Get bucket stats from index
   */
  async function getIndexStats(
    profileId: string,
    bucketName: string
  ): Promise<BucketIndexStats | null> {
    try {
      return await getBucketIndexStats(profileId, bucketName)
    } catch (error) {
      logger.error('Failed to get index stats', error)
      return null
    }
  }

  /**
   * Get prefix/folder stats from index
   */
  async function getFolderStats(
    profileId: string,
    bucketName: string,
    prefix: string
  ): Promise<PrefixStats | null> {
    try {
      return await getPrefixIndexStats(profileId, bucketName, prefix)
    } catch (error) {
      logger.error('Failed to get prefix stats', error)
      return null
    }
  }

  /**
   * Check if bucket is indexed
   */
  async function isIndexed(profileId: string, bucketName: string): Promise<boolean> {
    try {
      return await isBucketIndexed(profileId, bucketName)
    } catch {
      return false
    }
  }

  /**
   * Check if index is complete
   */
  async function isComplete(profileId: string, bucketName: string): Promise<boolean> {
    try {
      return await isBucketIndexComplete(profileId, bucketName)
    } catch {
      return false
    }
  }

  /**
   * Clear bucket index
   */
  async function clearIndex(profileId: string, bucketName: string): Promise<void> {
    try {
      await clearBucketIndex(profileId, bucketName)
      const key = `${profileId}-${bucketName}`
      delete bucketIndexStatus.value[key]
      delete indexProgress.value[key]
    } catch (error) {
      logger.error('Failed to clear index', error)
    }
  }

  /**
   * Check if bucket is currently being indexed
   */
  function isIndexing(profileId: string, bucketName: string): boolean {
    const key = `${profileId}-${bucketName}`
    return indexingBuckets.value[key] ?? false
  }

  /**
   * Get current progress for a bucket
   */
  function getProgress(profileId: string, bucketName: string): IndexProgressEvent | null {
    const key = `${profileId}-${bucketName}`
    return indexProgress.value[key] ?? null
  }

  /**
   * Get cached index status
   */
  function getIndexStatus(profileId: string, bucketName: string) {
    const key = `${profileId}-${bucketName}`
    return bucketIndexStatus.value[key] ?? null
  }

  /**
   * Search objects in the index
   * @param profileId Profile ID
   * @param bucketName Bucket name
   * @param query Search query (case-insensitive)
   * @param prefix Optional prefix to filter results
   * @param limit Maximum number of results (default: 10000)
   */
  async function searchObjects(
    profileId: string,
    bucketName: string,
    query: string,
    prefix?: string,
    limit?: number
  ): Promise<S3Object[]> {
    try {
      return await searchObjectsInIndex(profileId, bucketName, query, prefix, limit)
    } catch (error) {
      logger.error('Failed to search in index', error)
      return []
    }
  }

  /**
   * Get all bucket indexes for a profile
   * @param profileId Profile ID
   */
  async function getAllIndexes(profileId: string): Promise<BucketIndexMetadata[]> {
    try {
      return await getAllBucketIndexes(profileId)
    } catch (error) {
      logger.error('Failed to get all indexes', error)
      return []
    }
  }

  /**
   * Check if index is valid (not too old)
   * Uses indexValidityHours from settings store
   * @param profileId Profile ID
   * @param bucketName Bucket name
   */
  async function isIndexValid(
    profileId: string,
    bucketName: string
  ): Promise<boolean> {
    try {
      const settingsStore = useSettingsStore()
      const stats = await getIndexStats(profileId, bucketName)
      if (!stats || !stats.last_indexed_at) {
        return false
      }
      const age = Date.now() - stats.last_indexed_at
      const maxAgeMs = settingsStore.indexValidityHours * 60 * 60 * 1000
      return age < maxAgeMs
    } catch {
      return false
    }
  }

  /**
   * Get the index database file size on disk for a profile
   * @param profileId Profile ID
   * @returns File size in bytes
   */
  async function getIndexFileSize(profileId: string): Promise<number> {
    try {
      return await getIndexFileSizeService(profileId)
    } catch (error) {
      logger.error('Failed to get index file size', error)
      return 0
    }
  }

  return {
    // Methods
    startIndexing,
    startFullIndexing,
    cancelIndexing,
    getIndexStats,
    getFolderStats,
    isIndexed,
    isComplete,
    clearIndex,
    isIndexing,
    getProgress,
    getIndexStatus,
    searchObjects,
    getAllIndexes,
    isIndexValid,
    getIndexFileSize,

    // Reactive state
    indexingBuckets,
    indexProgress,
    bucketIndexStatus,
  }
}

// Singleton instance
let instance: ReturnType<typeof useIndexManager> | null = null

/**
 * Get singleton instance of index manager
 * Use this to share state across components
 */
export function getIndexManager(): ReturnType<typeof useIndexManager> {
  if (!instance) {
    instance = useIndexManager()
  }
  return instance
}
