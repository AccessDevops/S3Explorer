/**
 * Composable for S3 request metrics tracking
 *
 * Listens to metrics events from the Rust backend for real-time UI updates.
 * Metrics are stored in SQLite on the backend side.
 */

import { ref, computed, onMounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/tauri'
import { useSettingsStore } from '@/stores/settings'
import type {
  S3MetricsEvent,
  DailyStats,
  OperationStats,
  ErrorStats,
  BucketUsageStats,
  HourlyStats,
  DailyDistribution,
  WeeklyDistribution,
  S3Pricing,
} from '@/types/metrics'
import { DEFAULT_S3_PRICING, calculateCost } from '@/types/metrics'

// Types from backend (snake_case)
interface BackendDailyStats {
  date: string
  total_requests: number
  successful_requests: number
  failed_requests: number
  get_requests: number
  put_requests: number
  list_requests: number
  delete_requests: number
  estimated_cost_usd: number
  avg_duration_ms: number
  max_duration_ms: number
  bytes_downloaded: number
  bytes_uploaded: number
  updated_at: number
}

interface BackendHourlyStats {
  hour: number
  count: number
  success_count: number
  failed_count: number
}

interface BackendOperationStats {
  operation: string
  count: number
  success_count: number
  failed_count: number
  avg_duration_ms: number
  total_bytes: number
}

interface BackendErrorStats {
  category: string
  count: number
  last_occurrence: number
  example_message: string | null
}

interface BackendBucketUsageStats {
  bucket_name: string
  request_count: number
  bytes_transferred: number
}

interface BackendDailyDistribution {
  date: string
  day_label: string
  count: number
  success_count: number
  failed_count: number
}

interface BackendWeeklyDistribution {
  week_start: string
  week_label: string
  count: number
  success_count: number
  failed_count: number
}

interface StorageInfo {
  request_count: number
  oldest_date: string | null
}

// Shared state across all instances
const initialized = ref(false)
const todayStats = ref<DailyStats | null>(null)
const isLoading = ref(false)
const lastUpdate = ref<number>(0)
const realtimeRequestCount = ref(0)

// Unlisten function
let unlistenFn: UnlistenFn | null = null

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
 * Initialize metrics listener (call once at app startup)
 */
export async function initMetricsListener(): Promise<void> {
  if (initialized.value) return

  try {
    // Listen for metrics events from the Rust backend (for real-time counter updates)
    unlistenFn = await listen<S3MetricsEvent>('metrics:s3-request', () => {
      // Just update the counter - storage is handled by the backend
      realtimeRequestCount.value++
      lastUpdate.value = Date.now()
      // Invalidate today's stats cache
      todayStats.value = null
    })

    initialized.value = true

    // Load initial today stats
    await refreshTodayStats()
  } catch (error) {
    console.error('Failed to initialize metrics listener:', error)
  }
}

/**
 * Stop metrics listener
 */
export function stopMetricsListener(): void {
  if (unlistenFn) {
    unlistenFn()
    unlistenFn = null
  }
  initialized.value = false
}

/**
 * Refresh today's stats from backend
 */
async function refreshTodayStats(): Promise<void> {
  if (isLoading.value) return

  isLoading.value = true
  try {
    const pricing = getCurrentPricing()
    const stats = await invoke<BackendDailyStats>('get_metrics_today', {
      getPerThousand: pricing.get_per_thousand,
      putPerThousand: pricing.put_per_thousand,
      listPerThousand: pricing.list_per_thousand,
    })

    // Convert snake_case from backend to camelCase
    todayStats.value = {
      date: stats.date,
      totalRequests: stats.total_requests,
      successfulRequests: stats.successful_requests,
      failedRequests: stats.failed_requests,
      getRequests: stats.get_requests,
      putRequests: stats.put_requests,
      listRequests: stats.list_requests,
      deleteRequests: stats.delete_requests,
      estimatedCostUsd: stats.estimated_cost_usd,
      avgDurationMs: stats.avg_duration_ms,
      maxDurationMs: stats.max_duration_ms,
      bytesDownloaded: stats.bytes_downloaded,
      bytesUploaded: stats.bytes_uploaded,
      updatedAt: stats.updated_at,
    }

    lastUpdate.value = Date.now()
  } catch (error) {
    console.error('Failed to refresh today stats:', error)
  } finally {
    isLoading.value = false
  }
}

/**
 * Composable for metrics access
 */
export function useMetrics() {
  // Initialize on first use
  onMounted(() => {
    initMetricsListener()
  })

  // Computed properties
  const totalRequestsToday = computed(() => todayStats.value?.totalRequests ?? 0)

  const errorRateToday = computed(() => {
    if (!todayStats.value || todayStats.value.totalRequests === 0) return 0
    return (todayStats.value.failedRequests / todayStats.value.totalRequests) * 100
  })

  const estimatedCostToday = computed(() => todayStats.value?.estimatedCostUsd ?? 0)

  const estimatedCostMonth = computed(() => {
    // Project based on today's average
    const today = new Date()
    const daysInMonth = new Date(today.getFullYear(), today.getMonth() + 1, 0).getDate()
    const dayOfMonth = today.getDate()
    const dailyCost = todayStats.value?.estimatedCostUsd ?? 0
    return (dailyCost / dayOfMonth) * daysInMonth
  })

  const requestsPerHour = computed(() => {
    if (!todayStats.value || todayStats.value.totalRequests === 0) return 0
    const now = new Date()
    const hoursElapsed = now.getHours() + now.getMinutes() / 60 || 1
    return Math.round(todayStats.value.totalRequests / hoursElapsed)
  })

  const mostFrequentCategory = computed(() => {
    if (!todayStats.value) return null

    const categories = [
      { name: 'LIST', count: todayStats.value.listRequests },
      { name: 'GET', count: todayStats.value.getRequests },
      { name: 'PUT', count: todayStats.value.putRequests },
      { name: 'DELETE', count: todayStats.value.deleteRequests },
    ]

    const sorted = categories.sort((a, b) => b.count - a.count)
    if (sorted[0].count === 0) return null

    const percentage = Math.round((sorted[0].count / todayStats.value.totalRequests) * 100)
    return { category: sorted[0].name, percentage }
  })

  // Methods
  async function getStatsHistory(days: number): Promise<DailyStats[]> {
    const pricing = getCurrentPricing()
    const stats = await invoke<BackendDailyStats[]>('get_metrics_history', {
      days,
      getPerThousand: pricing.get_per_thousand,
      putPerThousand: pricing.put_per_thousand,
      listPerThousand: pricing.list_per_thousand,
    })

    // Convert snake_case to camelCase
    return stats.map(s => ({
      date: s.date,
      totalRequests: s.total_requests,
      successfulRequests: s.successful_requests,
      failedRequests: s.failed_requests,
      getRequests: s.get_requests,
      putRequests: s.put_requests,
      listRequests: s.list_requests,
      deleteRequests: s.delete_requests,
      estimatedCostUsd: s.estimated_cost_usd,
      avgDurationMs: s.avg_duration_ms,
      maxDurationMs: s.max_duration_ms,
      bytesDownloaded: s.bytes_downloaded,
      bytesUploaded: s.bytes_uploaded,
      updatedAt: s.updated_at,
    }))
  }

  async function getHourlyStats(): Promise<HourlyStats[]> {
    const stats = await invoke<BackendHourlyStats[]>('get_metrics_hourly', { date: null })
    // Convert snake_case to camelCase
    return stats.map(s => ({
      hour: s.hour,
      count: s.count,
      successCount: s.success_count,
      failedCount: s.failed_count,
    }))
  }

  async function getHourlyStatsPeriod(days: number): Promise<HourlyStats[]> {
    const stats = await invoke<BackendHourlyStats[]>('get_metrics_hourly_period', { days })
    // Convert snake_case to camelCase
    return stats.map(s => ({
      hour: s.hour,
      count: s.count,
      successCount: s.success_count,
      failedCount: s.failed_count,
    }))
  }

  async function getDailyDistribution(days: number): Promise<DailyDistribution[]> {
    const stats = await invoke<BackendDailyDistribution[]>('get_metrics_daily_distribution', { days })
    // Convert snake_case to camelCase
    return stats.map(s => ({
      date: s.date,
      dayLabel: s.day_label,
      count: s.count,
      successCount: s.success_count,
      failedCount: s.failed_count,
    }))
  }

  async function getWeeklyDistribution(days: number): Promise<WeeklyDistribution[]> {
    const stats = await invoke<BackendWeeklyDistribution[]>('get_metrics_weekly_distribution', { days })
    // Convert snake_case to camelCase
    return stats.map(s => ({
      weekStart: s.week_start,
      weekLabel: s.week_label,
      count: s.count,
      successCount: s.success_count,
      failedCount: s.failed_count,
    }))
  }

  async function getPeriodStats(days: number): Promise<DailyStats> {
    const pricing = getCurrentPricing()
    const stats = await invoke<BackendDailyStats>('get_metrics_period', {
      days,
      getPerThousand: pricing.get_per_thousand,
      putPerThousand: pricing.put_per_thousand,
      listPerThousand: pricing.list_per_thousand,
    })

    // Convert snake_case from backend to camelCase
    return {
      date: stats.date,
      totalRequests: stats.total_requests,
      successfulRequests: stats.successful_requests,
      failedRequests: stats.failed_requests,
      getRequests: stats.get_requests,
      putRequests: stats.put_requests,
      listRequests: stats.list_requests,
      deleteRequests: stats.delete_requests,
      estimatedCostUsd: stats.estimated_cost_usd,
      avgDurationMs: stats.avg_duration_ms,
      maxDurationMs: stats.max_duration_ms,
      bytesDownloaded: stats.bytes_downloaded,
      bytesUploaded: stats.bytes_uploaded,
      updatedAt: stats.updated_at,
    }
  }

  async function getOperationStats(days: number): Promise<OperationStats[]> {
    const stats = await invoke<BackendOperationStats[]>('get_metrics_by_operation', { days })
    // Convert snake_case to camelCase and cast string to enum
    return stats.map(s => ({
      operation: s.operation as unknown as OperationStats['operation'],
      count: s.count,
      successCount: s.success_count,
      failedCount: s.failed_count,
      avgDurationMs: s.avg_duration_ms,
      totalBytes: s.total_bytes,
    }))
  }

  async function getErrorStats(days: number): Promise<ErrorStats[]> {
    const stats = await invoke<BackendErrorStats[]>('get_metrics_errors', { days })
    // Convert snake_case to camelCase and cast string to enum
    return stats.map(s => ({
      category: s.category as unknown as ErrorStats['category'],
      count: s.count,
      lastOccurrence: s.last_occurrence,
      exampleMessage: s.example_message ?? undefined,
    }))
  }

  async function getTopBuckets(days: number, limit: number = 10): Promise<BucketUsageStats[]> {
    const stats = await invoke<BackendBucketUsageStats[]>('get_metrics_top_buckets', { days, limit })
    // Convert snake_case to camelCase
    return stats.map(s => ({
      bucketName: s.bucket_name,
      requestCount: s.request_count,
      bytesTransferred: s.bytes_transferred,
    }))
  }

  async function getRecentRequests(limit: number = 100) {
    return invoke('get_metrics_recent', { limit })
  }

  async function getFailedRequests(days: number, limit: number = 50) {
    return invoke('get_metrics_failed', { days, limit })
  }

  async function purgeData(retentionDays: number): Promise<number> {
    const count = await invoke<number>('purge_metrics', { retentionDays })
    await refreshTodayStats()
    return count
  }

  async function getStorageInfo(): Promise<{ requestCount: number; oldestDate: string | null }> {
    const info = await invoke<StorageInfo>('get_metrics_storage_info')
    return {
      requestCount: info.request_count,
      oldestDate: info.oldest_date,
    }
  }

  async function clearAllData(): Promise<void> {
    await invoke('clear_metrics')
    todayStats.value = null
    realtimeRequestCount.value = 0
  }

  function calculateCustomCost(
    getCount: number,
    putCount: number,
    listCount: number,
    deleteCount: number,
    pricing: S3Pricing = DEFAULT_S3_PRICING
  ): number {
    return calculateCost(getCount, putCount, listCount, deleteCount, pricing)
  }

  return {
    // State
    initialized,
    todayStats,
    isLoading,
    lastUpdate,
    realtimeRequestCount,

    // Computed
    totalRequestsToday,
    errorRateToday,
    estimatedCostToday,
    estimatedCostMonth,
    requestsPerHour,
    mostFrequentCategory,

    // Methods
    refreshTodayStats,
    getStatsHistory,
    getHourlyStats,
    getHourlyStatsPeriod,
    getDailyDistribution,
    getWeeklyDistribution,
    getPeriodStats,
    getOperationStats,
    getErrorStats,
    getTopBuckets,
    getRecentRequests,
    getFailedRequests,
    purgeData,
    getStorageInfo,
    clearAllData,
    calculateCustomCost,
  }
}
