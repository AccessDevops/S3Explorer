/**
 * Creates a throttled version of a function that only executes at most once per specified interval.
 * The last call is always executed after the interval.
 *
 * @param fn - The function to throttle
 * @param interval - Minimum time in milliseconds between executions
 * @returns Throttled function with flush and cancel methods
 */
export function throttle<T extends (...args: any[]) => void>(
  fn: T,
  interval: number
): T & { flush: () => void; cancel: () => void } {
  let timeoutId: number | null = null
  let lastArgs: any[] | null = null
  let lastCallTime = 0

  const throttled = function (this: any, ...args: any[]) {
    const now = Date.now()
    lastArgs = args

    // If enough time has passed since last execution, execute immediately
    if (now - lastCallTime >= interval) {
      lastCallTime = now
      fn.apply(this, args)
      lastArgs = null
    } else {
      // Schedule execution for later (debounce the trailing call)
      if (timeoutId !== null) {
        clearTimeout(timeoutId)
      }
      timeoutId = window.setTimeout(() => {
        lastCallTime = Date.now()
        if (lastArgs !== null) {
          fn.apply(this, lastArgs)
          lastArgs = null
        }
        timeoutId = null
      }, interval - (now - lastCallTime))
    }
  } as T & { flush: () => void; cancel: () => void }

  // Flush: Execute immediately with last arguments
  throttled.flush = function () {
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    if (lastArgs !== null) {
      fn(...lastArgs)
      lastArgs = null
      lastCallTime = Date.now()
    }
  }

  // Cancel: Clear pending execution
  throttled.cancel = function () {
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    lastArgs = null
  }

  return throttled
}

/**
 * Creates a throttled function using requestAnimationFrame for UI updates.
 * Ensures updates happen at most once per frame (~16ms for 60fps).
 *
 * @param fn - The function to throttle
 * @returns Throttled function with flush and cancel methods
 */
export function throttleAnimationFrame<T extends (...args: any[]) => void>(
  fn: T
): T & { flush: () => void; cancel: () => void } {
  let rafId: number | null = null
  let lastArgs: any[] | null = null

  const throttled = function (this: any, ...args: any[]) {
    lastArgs = args

    if (rafId === null) {
      rafId = requestAnimationFrame(() => {
        if (lastArgs !== null) {
          fn.apply(this, lastArgs)
          lastArgs = null
        }
        rafId = null
      })
    }
  } as T & { flush: () => void; cancel: () => void }

  // Flush: Execute immediately with last arguments
  throttled.flush = function () {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
      rafId = null
    }
    if (lastArgs !== null) {
      fn(...lastArgs)
      lastArgs = null
    }
  }

  // Cancel: Clear pending execution
  throttled.cancel = function () {
    if (rafId !== null) {
      cancelAnimationFrame(rafId)
      rafId = null
    }
    lastArgs = null
  }

  return throttled
}
