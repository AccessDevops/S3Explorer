import { ref } from 'vue'
import { calculateBucketStats } from '../services/tauri'
import { logger } from '../utils/logger'
import { getCacheMetrics } from './useCacheMetrics'

/**
 * Bucket statistics stored in IndexedDB
 */
export interface BucketStats {
  profileId: string
  bucketName: string
  size: number
  count: number
  lastUpdated: number
}

/**
 * IndexedDB helper functions
 */
const DB_NAME = 's3browser-bucket-stats'
const DB_VERSION = 1
const STORE_NAME = 'stats'

/**
 * Cache TTL: 24 hours (configurable)
 */
const STATS_CACHE_TTL_MS = 24 * 60 * 60 * 1000

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
        // Create object store with composite key [profileId, bucketName]
        const store = db.createObjectStore(STORE_NAME, { keyPath: 'id' })
        // Create index for querying by profile
        store.createIndex('profileId', 'profileId', { unique: false })
      }
    }
  })
}

/**
 * Generate unique ID for profile+bucket combination
 */
function getStatsId(profileId: string, bucketName: string): string {
  return `${profileId}-${bucketName}`
}

async function saveStatsToDB(stats: BucketStats): Promise<void> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readwrite')
    const store = transaction.objectStore(STORE_NAME)
    const id = getStatsId(stats.profileId, stats.bucketName)
    const request = store.put({ id, ...stats })

    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve()
  })
}

async function loadStatsFromDB(profileId: string, bucketName: string): Promise<BucketStats | null> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readonly')
    const store = transaction.objectStore(STORE_NAME)
    const id = getStatsId(profileId, bucketName)
    const request = store.get(id)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => {
      if (request.result) {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { id: _id, ...stats } = request.result
        resolve(stats as BucketStats)
      } else {
        resolve(null)
      }
    }
  })
}

async function deleteStatsFromDB(profileId: string, bucketName: string): Promise<void> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readwrite')
    const store = transaction.objectStore(STORE_NAME)
    const id = getStatsId(profileId, bucketName)
    const request = store.delete(id)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve()
  })
}

async function getAllStatsFromDB(): Promise<BucketStats[]> {
  const db = await openDB()
  return new Promise((resolve, reject) => {
    const transaction = db.transaction([STORE_NAME], 'readonly')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.getAll()

    request.onerror = () => reject(request.error)
    request.onsuccess = () => {
      const results = request.result.map((item: any) => {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { id: _id, ...stats } = item
        return stats as BucketStats
      })
      resolve(results)
    }
  })
}

/**
 * Shared state for loading progress
 */
const loadingStats = ref<Record<string, boolean>>({})
const statsProgress = ref<Record<string, number>>({}) // Current page count

/**
 * Composable for bucket stats management
 */
export function useBucketStats() {
  /**
   * Load bucket stats with cache support
   * @param profileId Profile ID
   * @param bucketName Bucket name
   * @param forceRefresh Force refresh from S3 (ignore cache)
   * @param cacheTTL Cache TTL in milliseconds (default: 24h)
   * @returns Bucket stats or null if not available
   */
  async function loadBucketStats(
    profileId: string,
    bucketName: string,
    forceRefresh: boolean = false,
    cacheTTL: number = STATS_CACHE_TTL_MS
  ): Promise<BucketStats | null> {
    const cacheKey = `${profileId}-${bucketName}`

    try {
      // Try to load from cache first
      if (!forceRefresh) {
        const cached = await loadStatsFromDB(profileId, bucketName)
        if (cached) {
          const age = Date.now() - cached.lastUpdated
          if (age < cacheTTL) {
            // Cache HIT - record metrics
            getCacheMetrics().recordCacheHit('bucketStats', {
              profileId,
              bucketName,
              savedRequests: 1, // Avoids 1 full bucket scan
            }).catch((e) => logger.error('Failed to record cache hit', e))
            return cached
          }
        }
      }

      // Cache MISS or expired - fetch from S3
      getCacheMetrics().recordCacheMiss('bucketStats', {
        profileId,
        bucketName,
      }).catch((e) => logger.error('Failed to record cache miss', e))
      loadingStats.value[cacheKey] = true
      statsProgress.value[cacheKey] = 0

      const [totalSize, totalCount] = await calculateBucketStats(profileId, bucketName)

      const stats: BucketStats = {
        profileId,
        bucketName,
        size: totalSize,
        count: totalCount,
        lastUpdated: Date.now(),
      }

      // Save to cache
      await saveStatsToDB(stats)

      return stats
    } catch (error) {
      logger.error(`Failed to load stats for bucket ${bucketName}`, error)
      // Error is logged, no need to show toast for background operation
      return null
    } finally {
      loadingStats.value[cacheKey] = false
      delete statsProgress.value[cacheKey]
    }
  }

  /**
   * Check if stats are cached and valid
   */
  async function hasValidStats(
    profileId: string,
    bucketName: string,
    cacheTTL: number = STATS_CACHE_TTL_MS
  ): Promise<boolean> {
    try {
      const cached = await loadStatsFromDB(profileId, bucketName)
      if (!cached) return false

      const age = Date.now() - cached.lastUpdated
      return age < cacheTTL
    } catch (error) {
      logger.error('Error checking stats validity', error)
      return false
    }
  }

  /**
   * Get cached stats without fetching from S3
   */
  async function getCachedStats(
    profileId: string,
    bucketName: string
  ): Promise<BucketStats | null> {
    try {
      return await loadStatsFromDB(profileId, bucketName)
    } catch (error) {
      logger.error('Error loading cached stats', error)
      return null
    }
  }

  /**
   * Invalidate (delete) cached stats for a bucket
   */
  async function invalidateStats(profileId: string, bucketName: string): Promise<void> {
    try {
      await deleteStatsFromDB(profileId, bucketName)
    } catch (error) {
      logger.error('Error invalidating stats', error)
    }
  }

  /**
   * Get all cached stats from IndexedDB
   */
  async function getAllCachedStats(): Promise<BucketStats[]> {
    try {
      return await getAllStatsFromDB()
    } catch (error) {
      logger.error('Error loading all cached stats', error)
      return []
    }
  }

  /**
   * Clear all cached stats from IndexedDB
   */
  async function clearAllStats(): Promise<void> {
    try {
      const db = await openDB()
      const transaction = db.transaction([STORE_NAME], 'readwrite')
      const store = transaction.objectStore(STORE_NAME)
      await new Promise<void>((resolve, reject) => {
        const request = store.clear()
        request.onerror = () => reject(request.error)
        request.onsuccess = () => resolve()
      })
      // Success logged, operation complete
    } catch (error) {
      logger.error('Error clearing all stats', error)
    }
  }

  return {
    loadBucketStats,
    hasValidStats,
    getCachedStats,
    invalidateStats,
    getAllCachedStats,
    clearAllStats,
    loadingStats,
    statsProgress,
  }
}
