/**
 * Cache Metrics Composable
 *
 * Provides methods to track cache hit/miss events and query cache statistics.
 * Used to measure the effectiveness of local caching (search index, bucket stats, etc.)
 */

import { ref, computed } from 'vue'
import { metricsStorage } from '@/services/metricsStorage'
import { useSettingsStore } from '@/stores/settings'
import type { CacheEvent, CacheOperation, CacheSummary, DailyCacheStats } from '@/types/metrics'

// Reactive state
const todayCacheStats = ref<DailyCacheStats | null>(null)
const isInitialized = ref(false)

/**
 * Generate a unique ID for cache events
 */
function generateId(): string {
  return `cache-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
}

/**
 * Get today's date as YYYY-MM-DD
 */
function getTodayDate(): string {
  return new Date().toISOString().split('T')[0]
}

/**
 * Cache metrics composable
 */
export function useCacheMetrics() {
  const settingsStore = useSettingsStore()

  /**
   * Initialize cache metrics (call once at app startup)
   */
  async function init(): Promise<void> {
    if (isInitialized.value) return

    await metricsStorage.init()
    await refreshTodayStats()
    isInitialized.value = true
  }

  /**
   * Record a cache hit event
   */
  async function recordCacheHit(
    operation: CacheOperation,
    options?: {
      profileId?: string
      bucketName?: string
      savedRequests?: number // estimated S3 requests saved
    }
  ): Promise<void> {
    const event: CacheEvent = {
      id: generateId(),
      timestamp: Date.now(),
      date: getTodayDate(),
      operation,
      hit: true,
      profileId: options?.profileId,
      bucketName: options?.bucketName,
      savedRequests: options?.savedRequests || 1,
    }

    await metricsStorage.recordCacheEvent(event)

    // Update today's stats
    if (todayCacheStats.value) {
      todayCacheStats.value.hits++
      todayCacheStats.value.totalLookups++
      todayCacheStats.value.estimatedRequestsSaved += options?.savedRequests || 1
      todayCacheStats.value.hitRate =
        (todayCacheStats.value.hits / todayCacheStats.value.totalLookups) * 100
    }
  }

  /**
   * Record a cache miss event
   */
  async function recordCacheMiss(
    operation: CacheOperation,
    options?: {
      profileId?: string
      bucketName?: string
    }
  ): Promise<void> {
    const event: CacheEvent = {
      id: generateId(),
      timestamp: Date.now(),
      date: getTodayDate(),
      operation,
      hit: false,
      profileId: options?.profileId,
      bucketName: options?.bucketName,
    }

    await metricsStorage.recordCacheEvent(event)

    // Update today's stats
    if (todayCacheStats.value) {
      todayCacheStats.value.misses++
      todayCacheStats.value.totalLookups++
      todayCacheStats.value.hitRate =
        (todayCacheStats.value.hits / todayCacheStats.value.totalLookups) * 100
    }
  }

  /**
   * Refresh today's cache statistics
   */
  async function refreshTodayStats(): Promise<void> {
    const pricing = settingsStore.getCurrentPricing
    todayCacheStats.value = await metricsStorage.getTodayCacheStats(pricing)
  }

  /**
   * Get cache summary for a period
   */
  async function getCacheSummary(days: number): Promise<CacheSummary> {
    const pricing = settingsStore.getCurrentPricing
    return metricsStorage.getCacheSummary(days, pricing)
  }

  /**
   * Get today's hit rate (0-100)
   */
  const hitRateToday = computed(() => {
    return todayCacheStats.value?.hitRate ?? 0
  })

  /**
   * Get today's total cache lookups
   */
  const lookupsToday = computed(() => {
    return todayCacheStats.value?.totalLookups ?? 0
  })

  /**
   * Get today's hits
   */
  const hitsToday = computed(() => {
    return todayCacheStats.value?.hits ?? 0
  })

  /**
   * Get today's requests saved
   */
  const requestsSavedToday = computed(() => {
    return todayCacheStats.value?.estimatedRequestsSaved ?? 0
  })

  /**
   * Get today's cost saved
   */
  const costSavedToday = computed(() => {
    return todayCacheStats.value?.estimatedCostSaved ?? 0
  })

  return {
    // State
    todayCacheStats,
    isInitialized,

    // Computed
    hitRateToday,
    lookupsToday,
    hitsToday,
    requestsSavedToday,
    costSavedToday,

    // Methods
    init,
    recordCacheHit,
    recordCacheMiss,
    refreshTodayStats,
    getCacheSummary,
  }
}

// Export a singleton for easy access across components
let singletonInstance: ReturnType<typeof useCacheMetrics> | null = null

export function getCacheMetrics(): ReturnType<typeof useCacheMetrics> {
  if (!singletonInstance) {
    singletonInstance = useCacheMetrics()
  }
  return singletonInstance
}
