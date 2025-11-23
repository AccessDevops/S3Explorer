import { ref } from 'vue'

/**
 * Global bucket stats invalidation tracker
 * Used to notify BucketList when stats need to be refreshed after uploads/deletes
 */
const invalidationTimestamps = ref<Map<string, number>>(new Map())
const lastRefreshTimestamps = new Map<string, number>() // Track when each bucket was last refreshed
const scheduledTimeouts = new Map<string, number>() // Track scheduled invalidations per bucket

// Minimum time between refreshes for the same bucket (3 seconds)
const REFRESH_COOLDOWN = 3000

export function useBucketStatsInvalidation() {
  /**
   * Mark a bucket's stats as invalid (needs refresh)
   * Smart cooldown: if bucket was refreshed < 3s ago, schedule invalidation after cooldown
   * If an invalidation is already scheduled for this bucket, skip (prevents duplicate refreshes)
   */
  const invalidateBucketStats = (bucketName: string) => {
    // If an invalidation is already scheduled for this bucket, skip
    if (scheduledTimeouts.has(bucketName)) {
      return
    }

    const now = Date.now()
    const lastRefresh = lastRefreshTimestamps.get(bucketName) || 0
    const timeSinceLastRefresh = now - lastRefresh
    const cooldownRemaining = REFRESH_COOLDOWN - timeSinceLastRefresh

    // Calculate delay: wait for cooldown to expire, or invalidate immediately
    const delay = Math.max(0, cooldownRemaining)

    // Schedule invalidation with cooldown
    const timeoutId = window.setTimeout(() => {
      // Mark bucket as invalid
      invalidationTimestamps.value.set(bucketName, Date.now())

      // Clean up scheduled timeout
      scheduledTimeouts.delete(bucketName)
    }, delay)

    // Track the scheduled timeout
    scheduledTimeouts.set(bucketName, timeoutId)
  }

  /**
   * Get the last invalidation timestamp for a bucket
   * Returns 0 if never invalidated
   */
  const getInvalidationTimestamp = (bucketName: string): number => {
    return invalidationTimestamps.value.get(bucketName) || 0
  }

  /**
   * Clear invalidation timestamp (called after refresh)
   * Also updates the last refresh timestamp for cooldown tracking
   */
  const clearInvalidation = (bucketName: string) => {
    invalidationTimestamps.value.delete(bucketName)

    // Update last refresh timestamp for cooldown tracking
    lastRefreshTimestamps.set(bucketName, Date.now())
  }

  /**
   * Immediately invalidate without cooldown (for explicit user actions)
   * Cancels any scheduled invalidation and marks as invalid immediately
   */
  const invalidateImmediately = (bucketName: string) => {
    // Cancel any scheduled timeout for this bucket
    const scheduledTimeout = scheduledTimeouts.get(bucketName)
    if (scheduledTimeout !== undefined) {
      clearTimeout(scheduledTimeout)
      scheduledTimeouts.delete(bucketName)
    }

    // Mark as invalid immediately
    const now = Date.now()
    invalidationTimestamps.value.set(bucketName, now)
  }

  return {
    invalidateBucketStats,
    getInvalidationTimestamp,
    clearInvalidation,
    invalidateImmediately,
  }
}
