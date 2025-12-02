/**
 * Cache Metrics Composable
 *
 * Provides methods to track cache hit/miss events and query cache statistics.
 * Uses SQLite backend via Tauri for storage.
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useSettingsStore } from '@/stores/settings'
import type { CacheOperation, CacheSummary, DailyCacheStats } from '@/types/metrics'
import { DEFAULT_S3_PRICING } from '@/types/metrics'

// Backend types
interface BackendCacheEvent {
  id: string
  timestamp: number
  date: string
  operation: string
  hit: boolean
  profile_id?: string
  bucket_name?: string
  saved_requests?: number
}

interface BackendCacheSummary {
  hit_rate: number
  total_hits: number
  total_misses: number
  requests_saved: number
  cost_saved: number
}

interface BackendDailyCacheStats {
  date: string
  total_lookups: number
  hits: number
  misses: number
  hit_rate: number
  estimated_requests_saved: number
  estimated_cost_saved: number
  updated_at: number
}

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
 * Get current pricing from settings store
 */
function getCurrentPricing(): { get_per_thousand: number; put_per_thousand: number; list_per_thousand: number } {
  try {
    const settingsStore = useSettingsStore()
    const pricing = settingsStore.getCurrentPricing
    return {
      get_per_thousand: pricing.getPerThousand,
      put_per_thousand: pricing.putPerThousand,
      list_per_thousand: pricing.listPerThousand,
    }
  } catch {
    return {
      get_per_thousand: DEFAULT_S3_PRICING.getPerThousand,
      put_per_thousand: DEFAULT_S3_PRICING.putPerThousand,
      list_per_thousand: DEFAULT_S3_PRICING.listPerThousand,
    }
  }
}

/**
 * Cache metrics composable
 */
export function useCacheMetrics() {
  /**
   * Initialize cache metrics (call once at app startup)
   */
  async function init(): Promise<void> {
    if (isInitialized.value) return
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
    const event: BackendCacheEvent = {
      id: generateId(),
      timestamp: Date.now(),
      date: getTodayDate(),
      operation,
      hit: true,
      profile_id: options?.profileId,
      bucket_name: options?.bucketName,
      saved_requests: options?.savedRequests || 1,
    }

    await invoke('record_cache_event', { event })

    // Update today's stats locally
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
    const event: BackendCacheEvent = {
      id: generateId(),
      timestamp: Date.now(),
      date: getTodayDate(),
      operation,
      hit: false,
      profile_id: options?.profileId,
      bucket_name: options?.bucketName,
    }

    await invoke('record_cache_event', { event })

    // Update today's stats locally
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
    try {
      const pricing = getCurrentPricing()
      const stats = await invoke<BackendDailyCacheStats>('get_today_cache_stats', {
        getPerThousand: pricing.get_per_thousand,
        putPerThousand: pricing.put_per_thousand,
        listPerThousand: pricing.list_per_thousand,
      })

      todayCacheStats.value = {
        date: stats.date,
        totalLookups: stats.total_lookups,
        hits: stats.hits,
        misses: stats.misses,
        hitRate: stats.hit_rate,
        estimatedRequestsSaved: stats.estimated_requests_saved,
        estimatedCostSaved: stats.estimated_cost_saved,
        updatedAt: stats.updated_at,
      }
    } catch (error) {
      console.warn('Failed to refresh today cache stats:', error)
    }
  }

  /**
   * Get cache summary for a period
   */
  async function getCacheSummary(days: number): Promise<CacheSummary> {
    try {
      const pricing = getCurrentPricing()
      const summary = await invoke<BackendCacheSummary>('get_cache_summary', {
        days,
        getPerThousand: pricing.get_per_thousand,
        putPerThousand: pricing.put_per_thousand,
        listPerThousand: pricing.list_per_thousand,
      })

      return {
        hitRate: summary.hit_rate,
        totalHits: summary.total_hits,
        totalMisses: summary.total_misses,
        requestsSaved: summary.requests_saved,
        costSaved: summary.cost_saved,
      }
    } catch (error) {
      console.warn('Failed to get cache summary:', error)
      return {
        hitRate: 0,
        totalHits: 0,
        totalMisses: 0,
        requestsSaved: 0,
        costSaved: 0,
      }
    }
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
