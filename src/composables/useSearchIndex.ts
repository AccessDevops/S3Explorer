import { ref } from 'vue'
import { listObjects } from '../services/tauri'
import type { S3Object } from '../types'
import { useToast } from './useToast'
import { useI18n } from './useI18n'
import { useSettingsStore } from '../stores/settings'
import { logger } from '../utils/logger'
import { getCacheMetrics } from './useCacheMetrics'

/**
 * Search index stored in IndexedDB
 */
export interface SearchIndex {
  bucketName: string
  profileId: string
  lastBuilt: number
  totalObjects: number
  sizeInBytes: number // Size of the index in bytes
  objects: Array<{
    key: string
    size: number
    lastModified: number
    searchKey: string // Lowercase key for fast search
  }>
}

/**
 * IndexedDB helper functions
 */
const DB_NAME = 's3explorer-search-index'
const DB_VERSION = 1
const STORE_NAME = 'indexes'

/**
 * Optimal batch size for index building (S3 maximum)
 * Always use 1000 for indexing to minimize number of requests
 */
const INDEX_BATCH_SIZE = 1000

let dbInstance: IDBDatabase | null = null

async function openDB(): Promise<IDBDatabase> {
  if (dbInstance) return dbInstance

  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => {
      dbInstance = request.result
      resolve(request.result)
    }

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'id' })
      }
    }
  })
}

async function saveIndexToDB(id: string, index: SearchIndex): Promise<void> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readwrite')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.put({ id, ...index })

    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve()
  })
}

async function loadIndexFromDB(id: string): Promise<SearchIndex | null> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readonly')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.get(id)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => {
      if (request.result) {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { id: _id, ...index } = request.result

        // Backward compatibility: Add sizeInBytes if missing (old indexes)
        if (!index.sizeInBytes) {
          index.sizeInBytes = 0
        }

        resolve(index as SearchIndex)
      } else {
        resolve(null)
      }
    }
  })
}

async function deleteIndexFromDB(id: string): Promise<void> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readwrite')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.delete(id)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve()
  })
}

async function getAllIndexesFromDB(): Promise<SearchIndex[]> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readonly')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.getAll()

    request.onerror = () => reject(request.error)
    request.onsuccess = () => {
      const results = request.result.map((item: any) => {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { id: _id, ...index } = item
        return index as SearchIndex
      })
      resolve(results)
    }
  })
}

/**
 * Shared state across all instances (singleton pattern)
 */
const isBuilding = ref(false)
const buildProgress = ref(0)
const buildTotal = ref(0)

/**
 * Composable for search index management
 */
export function useSearchIndex() {
  const toast = useToast()
  const { t } = useI18n()
  const settingsStore = useSettingsStore()

  /**
   * Generate unique index ID for profile+bucket combination
   */
  function getIndexId(profileId: string, bucketName: string): string {
    return `${profileId}-${bucketName}`
  }

  /**
   * Check if an index exists and is valid (based on indexValidityHours setting)
   */
  async function hasValidIndex(profileId: string, bucketName: string): Promise<boolean> {
    try {
      const indexId = getIndexId(profileId, bucketName)
      const index = await loadIndexFromDB(indexId)

      if (!index) return false

      const age = Date.now() - index.lastBuilt
      const validityMs = settingsStore.indexValidityHours * 60 * 60 * 1000
      return age < validityMs
    } catch (error) {
      logger.error('Error checking index validity', error)
      return false
    }
  }

  /**
   * Check index status (exists, valid, age, object count)
   */
  async function getIndexStatus(
    profileId: string,
    bucketName: string
  ): Promise<{
    exists: boolean
    isValid: boolean
    age: number
    totalObjects: number
  }> {
    try {
      const indexId = getIndexId(profileId, bucketName)
      const index = await loadIndexFromDB(indexId)

      if (!index) {
        return { exists: false, isValid: false, age: 0, totalObjects: 0 }
      }

      const age = Date.now() - index.lastBuilt
      const validityMs = settingsStore.indexValidityHours * 60 * 60 * 1000
      const isValid = age < validityMs

      return {
        exists: true,
        isValid,
        age,
        totalObjects: index.totalObjects,
      }
    } catch (error) {
      logger.error('Error checking index status', error)
      return { exists: false, isValid: false, age: 0, totalObjects: 0 }
    }
  }

  /**
   * Load index from IndexedDB
   */
  async function loadIndex(
    profileId: string,
    bucketName: string
  ): Promise<SearchIndex | null> {
    try {
      const indexId = getIndexId(profileId, bucketName)
      return await loadIndexFromDB(indexId)
    } catch (error) {
      logger.error('Error loading index', error)
      return null
    }
  }

  /**
   * Build index using Web Worker (optimal for large buckets)
   * Always uses INDEX_BATCH_SIZE (1000) for optimal S3 performance
   */
  async function buildIndexWithWorker(
    profileId: string,
    bucketName: string,
    batchSize: number = INDEX_BATCH_SIZE,
    onProgress?: (current: number, total: number) => void
  ): Promise<SearchIndex> {
    // eslint-disable-next-line no-async-promise-executor
    return new Promise(async (resolve, reject) => {
      // Create worker
      const worker = new Worker(
        new URL('../workers/searchIndexWorker.ts', import.meta.url),
        { type: 'module' }
      )

      let toastId: string | null = null

      // Handle messages from worker
      worker.onmessage = (e) => {
        const message = e.data

        switch (message.type) {
          case 'progress': {
            buildProgress.value = message.current
            buildTotal.value = message.total

            // Update toast every 3 updates to avoid spam
            if (toastId && message.current % (batchSize * 3) === 0) {
              toast.updateToast(toastId, {
                message: `${t('indexing')}... ${message.current.toLocaleString()} ${t('objects')}`,
              })
            }

            // Call progress callback
            if (onProgress) {
              onProgress(message.current, message.total)
            }
            break
          }

          case 'complete': {
            // Complete toast
            if (toastId) {
              toast.completeToast(
                toastId,
                `${t('indexBuilt')}: ${message.index.totalObjects.toLocaleString()} ${t('objects')}`,
                'success',
                5000
              )
            }

            // Cleanup worker
            worker.terminate()

            // Resolve with index
            resolve(message.index)
            break
          }

          case 'error': {
            // Error toast
            if (toastId) {
              toast.completeToast(toastId, `Error: ${message.error}`, 'error', 5000)
            }

            // Cleanup worker
            worker.terminate()

            // Reject with error
            reject(new Error(message.error))
            break
          }
        }
      }

      worker.onerror = (error) => {
        if (toastId) {
          toast.completeToast(toastId, `Worker error: ${error.message}`, 'error', 5000)
        }
        worker.terminate()
        reject(error)
      }

      try {
        // Initialize worker
        worker.postMessage({ type: 'init', bucketName, profileId })

        // Show initial loading toast
        toastId = toast.loading(t('buildingSearchIndex', bucketName))

        // Fetch and send batches to worker
        let continuationToken: string | undefined = undefined
        do {
          const result = await listObjects(
            profileId,
            bucketName,
            '', // Empty prefix - index entire bucket
            continuationToken,
            batchSize,
            false // No delimiter - get all objects
          )

          // Send batch to worker for processing
          worker.postMessage({
            type: 'processBatch',
            objects: result.objects,
          })

          continuationToken = result.continuation_token
        } while (continuationToken)

        // Finalize index in worker
        worker.postMessage({ type: 'finalize' })
      } catch (error) {
        if (toastId) {
          toast.completeToast(toastId, `Error: ${error}`, 'error', 5000)
        }
        worker.terminate()
        reject(error)
      }
    })
  }

  /**
   * Build search index for a bucket
   * Uses Web Worker for optimal performance with large buckets
   * Always uses INDEX_BATCH_SIZE (1000) for optimal S3 performance
   */
  async function buildIndex(
    profileId: string,
    bucketName: string,
    batchSize: number = INDEX_BATCH_SIZE,
    onProgress?: (current: number, total: number) => void
  ): Promise<SearchIndex> {
    isBuilding.value = true
    buildProgress.value = 0
    buildTotal.value = 0

    try {
      // Use Web Worker for indexing (runs on separate thread)
      const index = await buildIndexWithWorker(profileId, bucketName, batchSize, onProgress)

      // Save to IndexedDB
      const indexId = getIndexId(profileId, bucketName)
      await saveIndexToDB(indexId, index)

      return index
    } finally {
      isBuilding.value = false
    }
  }

  /**
   * Search in index (ultra-fast)
   * Records a cache hit event since this avoids S3 LIST requests
   */
  function searchInIndex(
    index: SearchIndex,
    query: string,
    prefix: string = '',
    options?: { profileId?: string }
  ): S3Object[] {
    const queryLower = query.toLowerCase()
    const prefixLower = prefix.toLowerCase()

    const results = index.objects
      .filter((obj) => {
        // Filter by prefix (local vs global mode)
        if (prefixLower && !obj.searchKey.startsWith(prefixLower)) {
          return false
        }

        // Filter by query
        return obj.searchKey.includes(queryLower)
      })
      .map((obj) => ({
        key: obj.key,
        size: obj.size,
        last_modified: new Date(obj.lastModified).toISOString(),
        storage_class: 'STANDARD',
        e_tag: '',
        is_folder: false,
      }))

    // Track cache hit - estimate S3 requests saved
    // A full search would require ~1 LIST per 1000 objects
    const savedRequests = Math.ceil(index.totalObjects / 1000)
    getCacheMetrics().recordCacheHit('search', {
      profileId: options?.profileId,
      bucketName: index.bucketName,
      savedRequests,
    }).catch((e) => logger.error('Failed to record cache hit', e))

    return results
  }

  /**
   * Record a cache miss event (called when search falls back to S3)
   */
  function recordSearchCacheMiss(profileId?: string, bucketName?: string): void {
    getCacheMetrics().recordCacheMiss('search', {
      profileId,
      bucketName,
    }).catch((e) => logger.error('Failed to record cache miss', e))
  }

  /**
   * Delete index for a bucket
   */
  async function deleteIndex(profileId: string, bucketName: string): Promise<void> {
    try {
      const indexId = getIndexId(profileId, bucketName)
      await deleteIndexFromDB(indexId)
      toast.success(t('indexDeleted'))
    } catch (error) {
      logger.error('Error deleting index', error)
      toast.error(t('errorDeletingIndex'))
    }
  }

  /**
   * Rebuild index (delete + build)
   * Always uses INDEX_BATCH_SIZE (1000) for optimal S3 performance
   */
  async function rebuildIndex(
    profileId: string,
    bucketName: string,
    batchSize: number = INDEX_BATCH_SIZE,
    onProgress?: (current: number, total: number) => void
  ): Promise<SearchIndex> {
    await deleteIndex(profileId, bucketName)
    return buildIndex(profileId, bucketName, batchSize, onProgress)
  }

  /**
   * Estimate total number of objects in bucket (fast check)
   */
  async function estimateBucketSize(profileId: string, bucketName: string): Promise<number> {
    try {
      const result = await listObjects(profileId, bucketName, '', undefined, 1000, false)

      if (!result.is_truncated) {
        return result.objects.length
      }

      // If truncated, we don't know exact total
      // Return -1 to indicate "large bucket"
      return -1
    } catch (error) {
      logger.error('Error estimating bucket size', error)
      return -1
    }
  }

  /**
   * Get all indexes from IndexedDB
   */
  async function getAllIndexes(): Promise<SearchIndex[]> {
    try {
      return await getAllIndexesFromDB()
    } catch (error) {
      logger.error('Error loading all indexes', error)
      return []
    }
  }

  /**
   * Get only index metadata (without loading all objects)
   */
  async function getIndexMetadata(
    profileId: string,
    bucketName: string
  ): Promise<{
    lastBuilt: number
    totalObjects: number
    sizeInBytes: number
  } | null> {
    try {
      const index = await loadIndex(profileId, bucketName)
      if (!index) return null

      return {
        lastBuilt: index.lastBuilt,
        totalObjects: index.totalObjects,
        sizeInBytes: index.sizeInBytes,
      }
    } catch (error) {
      logger.error('Error loading index metadata', error)
      return null
    }
  }

  /**
   * Check if index is enabled for search
   */
  function isIndexEnabled(profileId: string, bucketName: string): boolean {
    const key = `index-enabled-${profileId}-${bucketName}`
    const value = localStorage.getItem(key)
    return value !== 'false' // Enabled by default
  }

  /**
   * Set index enabled/disabled state
   */
  function setIndexEnabled(profileId: string, bucketName: string, enabled: boolean): void {
    const key = `index-enabled-${profileId}-${bucketName}`
    localStorage.setItem(key, enabled ? 'true' : 'false')
  }

  /**
   * Estimate index size in bytes based on object count
   */
  function estimateIndexSize(objectCount: number): number {
    // Average: ~150 bytes per object (key + metadata)
    return objectCount * 150
  }

  return {
    isBuilding,
    buildProgress,
    buildTotal,
    hasValidIndex,
    getIndexStatus,
    loadIndex,
    buildIndex,
    searchInIndex,
    recordSearchCacheMiss,
    deleteIndex,
    rebuildIndex,
    estimateBucketSize,
    getAllIndexes,
    getIndexMetadata,
    isIndexEnabled,
    setIndexEnabled,
    estimateIndexSize,
  }
}
