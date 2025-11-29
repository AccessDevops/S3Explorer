/**
 * Metrics Storage Service using IndexedDB
 *
 * Stores S3 request metrics for the dashboard.
 * Provides CRUD operations and aggregation queries.
 */

import type {
  S3RequestRecord,
  DailyStats,
  OperationStats,
  ErrorStats,
  BucketUsageStats,
  HourlyStats,
  S3Operation,
  S3ErrorCategory,
  RequestCategory,
  S3Pricing,
  CacheEvent,
  DailyCacheStats,
  CacheSummary,
} from '@/types/metrics'
import { DEFAULT_S3_PRICING, calculateCost } from '@/types/metrics'

const DB_NAME = 's3explorer-metrics'
const DB_VERSION = 2 // Upgraded for cache store
const REQUESTS_STORE = 'requests'
const DAILY_STATS_STORE = 'daily_stats'
const CACHE_EVENTS_STORE = 'cache_events'

let dbInstance: IDBDatabase | null = null

/**
 * Open or create the IndexedDB database
 */
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

      // Create requests store
      if (!db.objectStoreNames.contains(REQUESTS_STORE)) {
        const requestsStore = db.createObjectStore(REQUESTS_STORE, { keyPath: 'id' })
        requestsStore.createIndex('by-timestamp', 'timestamp', { unique: false })
        requestsStore.createIndex('by-date', 'date', { unique: false })
        requestsStore.createIndex('by-operation', 'operation', { unique: false })
        requestsStore.createIndex('by-bucket', 'bucketName', { unique: false })
        requestsStore.createIndex('by-success', 'success', { unique: false })
      }

      // Create daily stats store
      if (!db.objectStoreNames.contains(DAILY_STATS_STORE)) {
        db.createObjectStore(DAILY_STATS_STORE, { keyPath: 'date' })
      }

      // Create cache events store (v2)
      if (!db.objectStoreNames.contains(CACHE_EVENTS_STORE)) {
        const cacheStore = db.createObjectStore(CACHE_EVENTS_STORE, { keyPath: 'id' })
        cacheStore.createIndex('by-timestamp', 'timestamp', { unique: false })
        cacheStore.createIndex('by-date', 'date', { unique: false })
        cacheStore.createIndex('by-operation', 'operation', { unique: false })
      }
    }
  })
}

/**
 * Get today's date as YYYY-MM-DD
 */
function getTodayDate(): string {
  return new Date().toISOString().split('T')[0]
}

/**
 * Get date N days ago as YYYY-MM-DD
 */
function getDateDaysAgo(days: number): string {
  const date = new Date()
  date.setDate(date.getDate() - days)
  return date.toISOString().split('T')[0]
}

/**
 * Metrics Storage class
 */
class MetricsStorage {
  private initialized = false

  /**
   * Initialize the database (call once at app startup)
   */
  async init(): Promise<void> {
    if (this.initialized) return
    await openDB()
    this.initialized = true
  }

  /**
   * Record a new S3 request
   * Uses put() instead of add() to handle duplicate IDs gracefully
   */
  async recordRequest(record: S3RequestRecord): Promise<void> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readwrite')
      const store = transaction.objectStore(REQUESTS_STORE)
      // Use put() to insert or update - more resilient than add()
      const request = store.put(record)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => {
        // Invalidate daily stats cache for this date
        this.invalidateDailyStats(record.date).catch(console.error)
        resolve()
      }
    })
  }

  /**
   * Invalidate cached daily stats for a specific date
   */
  private async invalidateDailyStats(date: string): Promise<void> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([DAILY_STATS_STORE], 'readwrite')
      const store = transaction.objectStore(DAILY_STATS_STORE)
      const request = store.delete(date)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve()
    })
  }

  /**
   * Get all requests for a specific date
   */
  async getRequestsByDate(date: string): Promise<S3RequestRecord[]> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readonly')
      const store = transaction.objectStore(REQUESTS_STORE)
      const index = store.index('by-date')
      const request = index.getAll(date)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve(request.result)
    })
  }

  /**
   * Get requests within a date range
   */
  async getRequestsInRange(fromDate: string, toDate: string): Promise<S3RequestRecord[]> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readonly')
      const store = transaction.objectStore(REQUESTS_STORE)
      const index = store.index('by-date')
      const range = IDBKeyRange.bound(fromDate, toDate)
      const request = index.getAll(range)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve(request.result)
    })
  }

  /**
   * Calculate daily stats from raw requests
   */
  private calculateDailyStats(date: string, requests: S3RequestRecord[]): DailyStats {
    let getRequests = 0
    let putRequests = 0
    let listRequests = 0
    let deleteRequests = 0
    let successfulRequests = 0
    let failedRequests = 0
    let totalDurationMs = 0
    let maxDurationMs = 0
    let bytesDownloaded = 0
    let bytesUploaded = 0

    for (const req of requests) {
      // Count by category
      switch (req.category) {
        case 'GET':
          getRequests++
          break
        case 'PUT':
          putRequests++
          break
        case 'LIST':
          listRequests++
          break
        case 'DELETE':
          deleteRequests++
          break
      }

      // Count success/failure
      if (req.success) {
        successfulRequests++
      } else {
        failedRequests++
      }

      // Duration stats
      totalDurationMs += req.durationMs
      if (req.durationMs > maxDurationMs) {
        maxDurationMs = req.durationMs
      }

      // Bytes transferred
      if (req.bytesTransferred) {
        if (req.operation === 'GetObject') {
          bytesDownloaded += req.bytesTransferred
        } else if (req.operation === 'PutObject' || req.operation === 'UploadPart') {
          bytesUploaded += req.bytesTransferred
        }
      }
    }

    const totalRequests = requests.length
    const avgDurationMs = totalRequests > 0 ? totalDurationMs / totalRequests : 0
    const estimatedCostUsd = calculateCost(getRequests, putRequests, listRequests, deleteRequests)

    return {
      date,
      totalRequests,
      successfulRequests,
      failedRequests,
      getRequests,
      putRequests,
      listRequests,
      deleteRequests,
      estimatedCostUsd,
      avgDurationMs,
      maxDurationMs,
      bytesDownloaded,
      bytesUploaded,
      updatedAt: Date.now(),
    }
  }

  /**
   * Get or calculate daily stats for a specific date
   */
  async getDailyStats(date: string): Promise<DailyStats> {
    const db = await openDB()

    // Try to get cached stats first
    const cached = await new Promise<DailyStats | null>((resolve, reject) => {
      const transaction = db.transaction([DAILY_STATS_STORE], 'readonly')
      const store = transaction.objectStore(DAILY_STATS_STORE)
      const request = store.get(date)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve(request.result || null)
    })

    // Return cached if available and not stale (less than 5 minutes old)
    if (cached && Date.now() - cached.updatedAt < 5 * 60 * 1000) {
      return cached
    }

    // Calculate fresh stats
    const requests = await this.getRequestsByDate(date)
    const stats = this.calculateDailyStats(date, requests)

    // Cache the result (but not for today as it's still changing)
    if (date !== getTodayDate()) {
      await new Promise<void>((resolve, reject) => {
        const transaction = db.transaction([DAILY_STATS_STORE], 'readwrite')
        const store = transaction.objectStore(DAILY_STATS_STORE)
        const request = store.put(stats)

        request.onerror = () => reject(request.error)
        request.onsuccess = () => resolve()
      })
    }

    return stats
  }

  /**
   * Get today's stats (real-time, not cached)
   */
  async getTodayStats(): Promise<DailyStats> {
    const today = getTodayDate()
    const requests = await this.getRequestsByDate(today)
    return this.calculateDailyStats(today, requests)
  }

  /**
   * Get stats for the last N days
   */
  async getStatsHistory(days: number): Promise<DailyStats[]> {
    const stats: DailyStats[] = []

    for (let i = 0; i < days; i++) {
      const date = getDateDaysAgo(i)
      const dailyStats = await this.getDailyStats(date)
      stats.push(dailyStats)
    }

    // Return in chronological order
    return stats.reverse()
  }

  /**
   * Get hourly breakdown for today
   */
  async getTodayHourlyStats(): Promise<HourlyStats[]> {
    const today = getTodayDate()
    const requests = await this.getRequestsByDate(today)

    // Initialize hourly buckets
    const hourlyMap: Map<number, HourlyStats> = new Map()
    for (let h = 0; h < 24; h++) {
      hourlyMap.set(h, { hour: h, count: 0, successCount: 0, failedCount: 0 })
    }

    // Aggregate by hour
    for (const req of requests) {
      const hour = new Date(req.timestamp).getHours()
      const stats = hourlyMap.get(hour)!
      stats.count++
      if (req.success) {
        stats.successCount++
      } else {
        stats.failedCount++
      }
    }

    return Array.from(hourlyMap.values())
  }

  /**
   * Get stats grouped by operation type
   */
  async getOperationStats(days: number): Promise<OperationStats[]> {
    const fromDate = getDateDaysAgo(days - 1)
    const toDate = getTodayDate()
    const requests = await this.getRequestsInRange(fromDate, toDate)

    // Group by operation
    const opMap: Map<S3Operation, OperationStats> = new Map()

    for (const req of requests) {
      let stats = opMap.get(req.operation)
      if (!stats) {
        stats = {
          operation: req.operation,
          count: 0,
          successCount: 0,
          failedCount: 0,
          avgDurationMs: 0,
          totalBytes: 0,
        }
        opMap.set(req.operation, stats)
      }

      stats.count++
      if (req.success) {
        stats.successCount++
      } else {
        stats.failedCount++
      }
      stats.avgDurationMs += req.durationMs
      if (req.bytesTransferred) {
        stats.totalBytes += req.bytesTransferred
      }
    }

    // Calculate averages
    for (const stats of opMap.values()) {
      stats.avgDurationMs = stats.count > 0 ? stats.avgDurationMs / stats.count : 0
    }

    // Sort by count descending
    return Array.from(opMap.values()).sort((a, b) => b.count - a.count)
  }

  /**
   * Get error statistics
   */
  async getErrorStats(days: number): Promise<ErrorStats[]> {
    const fromDate = getDateDaysAgo(days - 1)
    const toDate = getTodayDate()
    const requests = await this.getRequestsInRange(fromDate, toDate)

    // Group errors by category
    const errorMap: Map<S3ErrorCategory, ErrorStats> = new Map()

    for (const req of requests) {
      if (!req.success && req.errorCategory) {
        let stats = errorMap.get(req.errorCategory)
        if (!stats) {
          stats = {
            category: req.errorCategory,
            count: 0,
            lastOccurrence: 0,
            exampleMessage: undefined,
          }
          errorMap.set(req.errorCategory, stats)
        }

        stats.count++
        if (req.timestamp > stats.lastOccurrence) {
          stats.lastOccurrence = req.timestamp
          stats.exampleMessage = req.errorMessage
        }
      }
    }

    // Sort by count descending
    return Array.from(errorMap.values()).sort((a, b) => b.count - a.count)
  }

  /**
   * Get top buckets by request count
   */
  async getTopBuckets(days: number, limit: number = 10): Promise<BucketUsageStats[]> {
    const fromDate = getDateDaysAgo(days - 1)
    const toDate = getTodayDate()
    const requests = await this.getRequestsInRange(fromDate, toDate)

    // Group by bucket
    const bucketMap: Map<string, BucketUsageStats> = new Map()

    for (const req of requests) {
      if (!req.bucketName) continue

      let stats = bucketMap.get(req.bucketName)
      if (!stats) {
        stats = {
          bucketName: req.bucketName,
          requestCount: 0,
          bytesTransferred: 0,
        }
        bucketMap.set(req.bucketName, stats)
      }

      stats.requestCount++
      if (req.bytesTransferred) {
        stats.bytesTransferred += req.bytesTransferred
      }
    }

    // Sort by request count and take top N
    return Array.from(bucketMap.values())
      .sort((a, b) => b.requestCount - a.requestCount)
      .slice(0, limit)
  }

  /**
   * Get recent requests (for detailed view)
   */
  async getRecentRequests(limit: number = 100): Promise<S3RequestRecord[]> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readonly')
      const store = transaction.objectStore(REQUESTS_STORE)
      const index = store.index('by-timestamp')

      const results: S3RequestRecord[] = []
      const cursorRequest = index.openCursor(null, 'prev') // Descending order

      cursorRequest.onerror = () => reject(cursorRequest.error)
      cursorRequest.onsuccess = () => {
        const cursor = cursorRequest.result
        if (cursor && results.length < limit) {
          results.push(cursor.value)
          cursor.continue()
        } else {
          resolve(results)
        }
      }
    })
  }

  /**
   * Get failed requests (for error analysis)
   */
  async getFailedRequests(days: number, limit: number = 50): Promise<S3RequestRecord[]> {
    const fromDate = getDateDaysAgo(days - 1)
    const toDate = getTodayDate()
    const requests = await this.getRequestsInRange(fromDate, toDate)

    return requests
      .filter((r) => !r.success)
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(0, limit)
  }

  /**
   * Purge old data beyond retention period
   */
  async purgeOldData(retentionDays: number): Promise<number> {
    const cutoffDate = getDateDaysAgo(retentionDays)
    const cutoffTimestamp = new Date(cutoffDate).getTime()
    const db = await openDB()

    let deletedCount = 0

    // Delete old requests
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readwrite')
      const store = transaction.objectStore(REQUESTS_STORE)
      const index = store.index('by-timestamp')
      const range = IDBKeyRange.upperBound(cutoffTimestamp)
      const cursorRequest = index.openCursor(range)

      cursorRequest.onerror = () => reject(cursorRequest.error)
      cursorRequest.onsuccess = () => {
        const cursor = cursorRequest.result
        if (cursor) {
          cursor.delete()
          deletedCount++
          cursor.continue()
        } else {
          resolve()
        }
      }
    })

    // Delete old daily stats
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction([DAILY_STATS_STORE], 'readwrite')
      const store = transaction.objectStore(DAILY_STATS_STORE)
      const range = IDBKeyRange.upperBound(cutoffDate)
      const cursorRequest = store.openCursor(range)

      cursorRequest.onerror = () => reject(cursorRequest.error)
      cursorRequest.onsuccess = () => {
        const cursor = cursorRequest.result
        if (cursor) {
          cursor.delete()
          cursor.continue()
        } else {
          resolve()
        }
      }
    })

    return deletedCount
  }

  /**
   * Export data to CSV format
   */
  async exportToCSV(fromDate: string, toDate: string): Promise<string> {
    const requests = await this.getRequestsInRange(fromDate, toDate)

    const headers = [
      'id',
      'timestamp',
      'date',
      'operation',
      'category',
      'profileId',
      'profileName',
      'bucketName',
      'objectKey',
      'durationMs',
      'bytesTransferred',
      'objectsAffected',
      'success',
      'errorCategory',
      'errorMessage',
    ]

    const rows = requests.map((r) =>
      [
        r.id,
        new Date(r.timestamp).toISOString(),
        r.date,
        r.operation,
        r.category,
        r.profileId || '',
        r.profileName || '',
        r.bucketName || '',
        r.objectKey || '',
        r.durationMs,
        r.bytesTransferred || '',
        r.objectsAffected || '',
        r.success,
        r.errorCategory || '',
        (r.errorMessage || '').replace(/"/g, '""'),
      ]
        .map((v) => `"${v}"`)
        .join(',')
    )

    return [headers.join(','), ...rows].join('\n')
  }

  /**
   * Get total storage size estimate
   */
  async getStorageInfo(): Promise<{ requestCount: number; oldestDate: string | null }> {
    const db = await openDB()

    const requestCount = await new Promise<number>((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readonly')
      const store = transaction.objectStore(REQUESTS_STORE)
      const request = store.count()

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve(request.result)
    })

    const oldestDate = await new Promise<string | null>((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE], 'readonly')
      const store = transaction.objectStore(REQUESTS_STORE)
      const index = store.index('by-timestamp')
      const cursorRequest = index.openCursor(null, 'next')

      cursorRequest.onerror = () => reject(cursorRequest.error)
      cursorRequest.onsuccess = () => {
        const cursor = cursorRequest.result
        if (cursor) {
          resolve(cursor.value.date)
        } else {
          resolve(null)
        }
      }
    })

    return { requestCount, oldestDate }
  }

  /**
   * Clear all metrics data
   */
  async clearAll(): Promise<void> {
    const db = await openDB()

    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction([REQUESTS_STORE, DAILY_STATS_STORE, CACHE_EVENTS_STORE], 'readwrite')

      transaction.onerror = () => reject(transaction.error)
      transaction.oncomplete = () => resolve()

      transaction.objectStore(REQUESTS_STORE).clear()
      transaction.objectStore(DAILY_STATS_STORE).clear()
      transaction.objectStore(CACHE_EVENTS_STORE).clear()
    })
  }

  // ============================================
  // Cache Metrics Methods
  // ============================================

  /**
   * Record a cache hit or miss event
   */
  async recordCacheEvent(event: CacheEvent): Promise<void> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([CACHE_EVENTS_STORE], 'readwrite')
      const store = transaction.objectStore(CACHE_EVENTS_STORE)
      // Use put() to insert or update - more resilient than add()
      const request = store.put(event)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve()
    })
  }

  /**
   * Get cache events for a specific date
   */
  async getCacheEventsByDate(date: string): Promise<CacheEvent[]> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([CACHE_EVENTS_STORE], 'readonly')
      const store = transaction.objectStore(CACHE_EVENTS_STORE)
      const index = store.index('by-date')
      const request = index.getAll(date)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve(request.result)
    })
  }

  /**
   * Get cache events within a date range
   */
  async getCacheEventsInRange(fromDate: string, toDate: string): Promise<CacheEvent[]> {
    const db = await openDB()

    return new Promise((resolve, reject) => {
      const transaction = db.transaction([CACHE_EVENTS_STORE], 'readonly')
      const store = transaction.objectStore(CACHE_EVENTS_STORE)
      const index = store.index('by-date')
      const range = IDBKeyRange.bound(fromDate, toDate)
      const request = index.getAll(range)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => resolve(request.result)
    })
  }

  /**
   * Get today's cache statistics
   */
  async getTodayCacheStats(pricing: S3Pricing = DEFAULT_S3_PRICING): Promise<DailyCacheStats> {
    const today = getTodayDate()
    const events = await this.getCacheEventsByDate(today)

    return this.calculateCacheStats(today, events, pricing)
  }

  /**
   * Calculate cache statistics from events
   */
  private calculateCacheStats(date: string, events: CacheEvent[], pricing: S3Pricing): DailyCacheStats {
    let hits = 0
    let misses = 0
    let estimatedRequestsSaved = 0

    for (const event of events) {
      if (event.hit) {
        hits++
        estimatedRequestsSaved += event.savedRequests || 1
      } else {
        misses++
      }
    }

    const totalLookups = hits + misses
    const hitRate = totalLookups > 0 ? (hits / totalLookups) * 100 : 0

    // Estimate cost saved based on LIST requests (most common for search)
    const estimatedCostSaved = (estimatedRequestsSaved / 1000) * pricing.listPerThousand

    return {
      date,
      totalLookups,
      hits,
      misses,
      hitRate,
      estimatedRequestsSaved,
      estimatedCostSaved,
      updatedAt: Date.now(),
    }
  }

  /**
   * Get cache summary for a period
   */
  async getCacheSummary(days: number, pricing: S3Pricing = DEFAULT_S3_PRICING): Promise<CacheSummary> {
    const fromDate = getDateDaysAgo(days - 1)
    const toDate = getTodayDate()
    const events = await this.getCacheEventsInRange(fromDate, toDate)

    let totalHits = 0
    let totalMisses = 0
    let requestsSaved = 0

    for (const event of events) {
      if (event.hit) {
        totalHits++
        requestsSaved += event.savedRequests || 1
      } else {
        totalMisses++
      }
    }

    const total = totalHits + totalMisses
    const hitRate = total > 0 ? (totalHits / total) * 100 : 0
    const costSaved = (requestsSaved / 1000) * pricing.listPerThousand

    return {
      hitRate,
      totalHits,
      totalMisses,
      requestsSaved,
      costSaved,
    }
  }

  /**
   * Purge old cache events
   */
  async purgeCacheEvents(retentionDays: number): Promise<number> {
    const cutoffDate = getDateDaysAgo(retentionDays)
    const cutoffTimestamp = new Date(cutoffDate).getTime()
    const db = await openDB()

    let deletedCount = 0

    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction([CACHE_EVENTS_STORE], 'readwrite')
      const store = transaction.objectStore(CACHE_EVENTS_STORE)
      const index = store.index('by-timestamp')
      const range = IDBKeyRange.upperBound(cutoffTimestamp)
      const cursorRequest = index.openCursor(range)

      cursorRequest.onerror = () => reject(cursorRequest.error)
      cursorRequest.onsuccess = () => {
        const cursor = cursorRequest.result
        if (cursor) {
          cursor.delete()
          deletedCount++
          cursor.continue()
        } else {
          resolve()
        }
      }
    })

    return deletedCount
  }
}

// Export singleton instance
export const metricsStorage = new MetricsStorage()
