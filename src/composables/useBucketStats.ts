import { ref } from 'vue'
import { calculateBucketStats } from '../services/tauri'
import { logger } from '../utils/logger'

/**
 * Bucket statistics (now from Rust SQLite index)
 */
export interface BucketStats {
  profileId: string
  bucketName: string
  size: number
  count: number
  lastUpdated: number
  isEstimate: boolean // true if from incomplete index
}

/**
 * In-memory cache for stats (quick display, Rust handles persistence)
 */
const statsCache = ref<Record<string, BucketStats>>({})

/**
 * Shared state for loading progress
 */
const loadingStats = ref<Record<string, boolean>>({})
const statsProgress = ref<Record<string, number>>({})

/**
 * Generate cache key for profile+bucket combination
 */
function getCacheKey(profileId: string, bucketName: string): string {
  return `${profileId}-${bucketName}`
}

/**
 * Composable for bucket stats management
 * Stats now come from Rust SQLite index with automatic fallback to S3
 */
export function useBucketStats() {
  /**
   * Load bucket stats from Rust backend
   * The backend automatically uses SQLite index if available, otherwise falls back to S3
   * @param profileId Profile ID
   * @param bucketName Bucket name
   * @param forceRefresh Force refresh from S3 (ignore index)
   * @returns Bucket stats or null if not available
   */
  async function loadBucketStats(
    profileId: string,
    bucketName: string,
    forceRefresh: boolean = false
  ): Promise<BucketStats | null> {
    const cacheKey = getCacheKey(profileId, bucketName)

    try {
      // Check in-memory cache first (for quick UI updates)
      if (!forceRefresh && statsCache.value[cacheKey]) {
        return statsCache.value[cacheKey]
      }

      loadingStats.value[cacheKey] = true
      statsProgress.value[cacheKey] = 0

      // Call Rust backend - it handles index lookup and S3 fallback
      const [totalSize, totalCount, isEstimate] = await calculateBucketStats(
        profileId,
        bucketName,
        forceRefresh
      )

      const stats: BucketStats = {
        profileId,
        bucketName,
        size: totalSize,
        count: totalCount,
        lastUpdated: Date.now(),
        isEstimate,
      }

      // Update in-memory cache
      statsCache.value[cacheKey] = stats

      return stats
    } catch (error) {
      logger.error(`Failed to load stats for bucket ${bucketName}`, error)
      return null
    } finally {
      loadingStats.value[cacheKey] = false
      delete statsProgress.value[cacheKey]
    }
  }

  /**
   * Get cached stats from memory (no backend call)
   */
  function getCachedStats(
    profileId: string,
    bucketName: string
  ): BucketStats | null {
    const cacheKey = getCacheKey(profileId, bucketName)
    return statsCache.value[cacheKey] ?? null
  }

  /**
   * Invalidate cached stats for a bucket
   */
  function invalidateStats(profileId: string, bucketName: string): void {
    const cacheKey = getCacheKey(profileId, bucketName)
    delete statsCache.value[cacheKey]
  }

  /**
   * Clear all cached stats from memory
   */
  function clearAllStats(): void {
    statsCache.value = {}
  }

  /**
   * Update stats in cache (for optimistic updates)
   */
  function updateCachedStats(
    profileId: string,
    bucketName: string,
    update: Partial<Pick<BucketStats, 'size' | 'count'>>
  ): void {
    const cacheKey = getCacheKey(profileId, bucketName)
    const existing = statsCache.value[cacheKey]
    if (existing) {
      statsCache.value[cacheKey] = {
        ...existing,
        ...update,
        lastUpdated: Date.now(),
      }
    }
  }

  return {
    loadBucketStats,
    getCachedStats,
    invalidateStats,
    clearAllStats,
    updateCachedStats,
    loadingStats,
    statsProgress,
    statsCache,
  }
}
