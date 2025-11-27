/**
 * Composable for S3 request metrics tracking
 *
 * Listens to metrics events from the Rust backend and stores them in IndexedDB.
 * Provides reactive access to metrics data for the dashboard.
 */

import { ref, computed, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { metricsStorage } from '@/services/metricsStorage'
import type {
  S3MetricsEvent,
  DailyStats,
  OperationStats,
  ErrorStats,
  BucketUsageStats,
  HourlyStats,
  S3Pricing,
} from '@/types/metrics'
import { eventToRecord, DEFAULT_S3_PRICING, calculateCost } from '@/types/metrics'

// Shared state across all instances
const initialized = ref(false)
const todayStats = ref<DailyStats | null>(null)
const isLoading = ref(false)
const lastUpdate = ref<number>(0)
const realtimeRequestCount = ref(0)

// Unlisten function
let unlistenFn: UnlistenFn | null = null

/**
 * Initialize metrics listener (call once at app startup)
 */
export async function initMetricsListener(): Promise<void> {
  if (initialized.value) return

  try {
    await metricsStorage.init()

    // Listen for metrics events from the Rust backend
    unlistenFn = await listen<S3MetricsEvent>('metrics:s3-request', async (event) => {
      const record = eventToRecord(event.payload)

      try {
        await metricsStorage.recordRequest(record)
        realtimeRequestCount.value++
        lastUpdate.value = Date.now()

        // Invalidate today's stats cache
        todayStats.value = null
      } catch (error) {
        console.error('Failed to record metrics:', error)
      }
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
 * Refresh today's stats from storage
 */
async function refreshTodayStats(): Promise<void> {
  if (isLoading.value) return

  isLoading.value = true
  try {
    todayStats.value = await metricsStorage.getTodayStats()
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
    return metricsStorage.getStatsHistory(days)
  }

  async function getHourlyStats(): Promise<HourlyStats[]> {
    return metricsStorage.getTodayHourlyStats()
  }

  async function getOperationStats(days: number): Promise<OperationStats[]> {
    return metricsStorage.getOperationStats(days)
  }

  async function getErrorStats(days: number): Promise<ErrorStats[]> {
    return metricsStorage.getErrorStats(days)
  }

  async function getTopBuckets(days: number, limit: number = 10): Promise<BucketUsageStats[]> {
    return metricsStorage.getTopBuckets(days, limit)
  }

  async function getRecentRequests(limit: number = 100) {
    return metricsStorage.getRecentRequests(limit)
  }

  async function getFailedRequests(days: number, limit: number = 50) {
    return metricsStorage.getFailedRequests(days, limit)
  }

  async function purgeData(retentionDays: number): Promise<number> {
    const count = await metricsStorage.purgeOldData(retentionDays)
    await refreshTodayStats()
    return count
  }

  async function exportCSV(fromDate: string, toDate: string): Promise<string> {
    return metricsStorage.exportToCSV(fromDate, toDate)
  }

  async function getStorageInfo() {
    return metricsStorage.getStorageInfo()
  }

  async function clearAllData(): Promise<void> {
    await metricsStorage.clearAll()
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
    getOperationStats,
    getErrorStats,
    getTopBuckets,
    getRecentRequests,
    getFailedRequests,
    purgeData,
    exportCSV,
    getStorageInfo,
    clearAllData,
    calculateCustomCost,
  }
}
