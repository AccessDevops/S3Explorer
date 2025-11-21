/**
 * Memory monitoring utilities to track and alert on RAM usage
 */

// Extend Performance interface to include memory property (Chrome/Edge only)
interface PerformanceMemory {
  usedJSHeapSize: number
  totalJSHeapSize: number
  jsHeapSizeLimit: number
}

interface PerformanceWithMemory extends Performance {
  memory?: PerformanceMemory
}

export interface MemoryInfo {
  usedJSHeapSize: number // Bytes used by JavaScript heap
  totalJSHeapSize: number // Total bytes allocated for JavaScript heap
  jsHeapSizeLimit: number // Maximum heap size available
  usagePercentage: number // Percentage of heap used
}

export interface MemoryThresholds {
  warning: number // Percentage (default: 70%)
  critical: number // Percentage (default: 85%)
}

const DEFAULT_THRESHOLDS: MemoryThresholds = {
  warning: 70,
  critical: 85,
}

/**
 * Get current memory usage information
 * Returns null if memory API is not available
 */
export function getMemoryInfo(): MemoryInfo | null {
  // Check if performance.memory is available (Chrome/Edge only)
  const perf = performance as PerformanceWithMemory
  if (!perf.memory) {
    return null
  }

  const memory = perf.memory
  const usagePercentage = (memory.usedJSHeapSize / memory.jsHeapSizeLimit) * 100

  return {
    usedJSHeapSize: memory.usedJSHeapSize,
    totalJSHeapSize: memory.totalJSHeapSize,
    jsHeapSizeLimit: memory.jsHeapSizeLimit,
    usagePercentage,
  }
}

/**
 * Format memory size in human-readable format
 */
export function formatMemorySize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

/**
 * Check if memory usage exceeds thresholds
 * Returns 'ok', 'warning', or 'critical'
 */
export function checkMemoryThreshold(
  thresholds: MemoryThresholds = DEFAULT_THRESHOLDS
): 'ok' | 'warning' | 'critical' | 'unavailable' {
  const memInfo = getMemoryInfo()

  if (!memInfo) {
    return 'unavailable'
  }

  if (memInfo.usagePercentage >= thresholds.critical) {
    return 'critical'
  }

  if (memInfo.usagePercentage >= thresholds.warning) {
    return 'warning'
  }

  return 'ok'
}

/**
 * Monitor memory usage and call callbacks when thresholds are exceeded
 */
export class MemoryMonitor {
  private intervalId: number | null = null
  private thresholds: MemoryThresholds
  private checkInterval: number
  private onWarning?: (info: MemoryInfo) => void
  private onCritical?: (info: MemoryInfo) => void
  private lastStatus: 'ok' | 'warning' | 'critical' | 'unavailable' = 'ok'

  constructor(
    thresholds: MemoryThresholds = DEFAULT_THRESHOLDS,
    checkInterval: number = 5000 // Check every 5 seconds
  ) {
    this.thresholds = thresholds
    this.checkInterval = checkInterval
  }

  /**
   * Start monitoring memory usage
   */
  start(callbacks: {
    onWarning?: (info: MemoryInfo) => void
    onCritical?: (info: MemoryInfo) => void
  }): void {
    this.onWarning = callbacks.onWarning
    this.onCritical = callbacks.onCritical

    // Initial check
    this.check()

    // Set up interval
    this.intervalId = window.setInterval(() => {
      this.check()
    }, this.checkInterval)
  }

  /**
   * Stop monitoring
   */
  stop(): void {
    if (this.intervalId !== null) {
      clearInterval(this.intervalId)
      this.intervalId = null
    }
  }

  /**
   * Check current memory status
   */
  private check(): void {
    const status = checkMemoryThreshold(this.thresholds)
    const memInfo = getMemoryInfo()

    if (!memInfo) {
      return
    }

    // Only trigger callbacks when status changes (to avoid spam)
    if (status !== this.lastStatus) {
      if (status === 'critical' && this.onCritical) {
        this.onCritical(memInfo)
      } else if (status === 'warning' && this.onWarning) {
        this.onWarning(memInfo)
      }

      this.lastStatus = status
    }
  }

  /**
   * Get current memory info
   */
  getCurrentInfo(): MemoryInfo | null {
    return getMemoryInfo()
  }
}

/**
 * Attempt to trigger garbage collection (non-standard, only works with --expose-gc flag)
 * This is a hint to the browser, not guaranteed to work
 */
export function requestGarbageCollection(): void {
  // Check if gc is exposed (requires --js-flags=--expose-gc in Chrome)
  if (typeof (globalThis as any).gc === 'function') {
    ;(globalThis as any).gc()
  }
}
