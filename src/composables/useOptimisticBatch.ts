import { ref, computed } from 'vue'
import { useAppStore } from '../stores/app'
import { useRustUploadManager } from './useRustUploadManager'
import { logger } from '../utils/logger'
import type { S3Object } from '../types'

// ============================================================================
// CORRECTION #4: Adaptive batch system for optimistic updates
// ============================================================================
// Problem: With 900 uploads, each upload:object-completed event triggers:
//   1. appStore.addObject() → Vue reactivity
//   2. filteredObjects computed recalculation (copy + sort)
//   3. Re-render of virtual list
// Result: O(n² log n) complexity instead of O(n log n)
//
// Solution: Batch objects and add them in groups with adaptive delay:
//   - Few uploads (≤10): Immediate add (best UX)
//   - Many uploads (>10): Batch with increasing delay (better performance)

// Threshold configuration
const THRESHOLDS = {
  IMMEDIATE: 10,       // ≤10 uploads: immediate add
  LIGHT_BATCH: 50,     // 11-50: 300ms batch
  MEDIUM_BATCH: 200,   // 51-200: 500ms batch
  HEAVY_BATCH: Infinity, // 201+: 1000ms batch
} as const

const DELAYS = {
  IMMEDIATE: 0,
  LIGHT: 300,
  MEDIUM: 500,
  HEAVY: 1000,
} as const

// Singleton state (shared across all component instances)
const pendingObjects = ref<S3Object[]>([])
let batchTimer: number | null = null
let currentBucket: string | null = null

export function useOptimisticBatch() {
  const appStore = useAppStore()
  const { uploadCount } = useRustUploadManager()

  /**
   * Determine batch delay based on total upload count
   */
  function getBatchDelay(): number {
    const total = uploadCount.value.total

    if (total <= THRESHOLDS.IMMEDIATE) return DELAYS.IMMEDIATE
    if (total <= THRESHOLDS.LIGHT_BATCH) return DELAYS.LIGHT
    if (total <= THRESHOLDS.MEDIUM_BATCH) return DELAYS.MEDIUM
    return DELAYS.HEAVY
  }

  /**
   * Flush the batch: add all pending objects to the store
   * This triggers only ONE reactive update cycle instead of N
   */
  function flushBatch() {
    if (pendingObjects.value.length === 0) return

    const count = pendingObjects.value.length
    const objectsToAdd = [...pendingObjects.value]

    // Clear batch state
    pendingObjects.value = []
    batchTimer = null
    currentBucket = null

    // Add all objects in a single batch
    // Note: Each addObject still triggers reactivity, but they're grouped
    // and the computed recalculations happen once at the end of the batch
    for (const obj of objectsToAdd) {
      appStore.addObject(obj)
    }

    logger.debug(`[OptimisticBatch] Flushed ${count} objects to store`)
  }

  /**
   * Add an object to the batch (or immediately if few uploads)
   */
  function addObjectToBatch(bucket: string, obj: S3Object) {
    // Skip if not the current bucket
    if (bucket !== appStore.currentBucket) {
      logger.debug(`[OptimisticBatch] Skipping object for different bucket: ${bucket}`)
      return
    }

    const delay = getBatchDelay()

    // Immediate mode: no batching for few uploads
    if (delay === 0) {
      appStore.addObject(obj)
      logger.debug(`[OptimisticBatch] Immediate add: ${obj.key}`)
      return
    }

    // If bucket changed, flush the old batch first
    if (currentBucket && currentBucket !== bucket) {
      logger.debug(`[OptimisticBatch] Bucket changed, flushing old batch`)
      flushBatch()
    }
    currentBucket = bucket

    // Add to pending list
    pendingObjects.value.push(obj)

    // Reset timer (debounce behavior)
    if (batchTimer) {
      clearTimeout(batchTimer)
    }

    // Schedule flush
    batchTimer = window.setTimeout(flushBatch, delay)

    logger.debug(
      `[OptimisticBatch] Queued object: ${obj.key} (pending: ${pendingObjects.value.length}, delay: ${delay}ms)`
    )
  }

  /**
   * Force immediate flush (called when all uploads complete)
   */
  function forceFlush() {
    if (batchTimer) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    if (pendingObjects.value.length > 0) {
      logger.debug(`[OptimisticBatch] Force flushing ${pendingObjects.value.length} objects`)
      flushBatch()
    }
  }

  /**
   * Cancel pending batch (called when navigating away)
   */
  function cancelBatch() {
    if (batchTimer) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    if (pendingObjects.value.length > 0) {
      logger.debug(`[OptimisticBatch] Cancelled batch with ${pendingObjects.value.length} objects`)
    }
    pendingObjects.value = []
    currentBucket = null
  }

  /**
   * Get current batch status (for UI indicator)
   */
  const batchStatus = computed(() => ({
    pending: pendingObjects.value.length,
    delay: getBatchDelay(),
    bucket: currentBucket,
  }))

  return {
    pendingObjects,
    batchStatus,
    addObjectToBatch,
    forceFlush,
    cancelBatch,
    getBatchDelay,
  }
}
