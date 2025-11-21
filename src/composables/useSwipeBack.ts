import { onMounted, onUnmounted, type Ref } from 'vue'

/**
 * Composable to detect trackpad/mouse swipe gestures for back navigation
 * Detects horizontal swipe gestures (similar to browser back gesture)
 */
export function useSwipeBack(
  elementRef: Ref<HTMLElement | null>,
  onSwipeBack: () => void,
  options: {
    threshold?: number // Minimum horizontal distance to trigger
    velocityThreshold?: number // Minimum velocity to trigger
  } = {}
) {
  const { threshold = 100, velocityThreshold = 0.5 } = options

  let startX = 0
  let startY = 0
  let startTime = 0
  let isTracking = false

  function handleTouchStart(e: TouchEvent) {
    if (e.touches.length !== 1) return

    startX = e.touches[0].clientX
    startY = e.touches[0].clientY
    startTime = Date.now()
    isTracking = true
  }

  function handleTouchEnd(e: TouchEvent) {
    if (!isTracking || e.changedTouches.length !== 1) return

    const endX = e.changedTouches[0].clientX
    const endY = e.changedTouches[0].clientY
    const endTime = Date.now()

    const deltaX = endX - startX
    const deltaY = endY - startY
    const deltaTime = endTime - startTime

    // Calculate velocity (pixels per millisecond)
    const velocity = Math.abs(deltaX) / deltaTime

    // Check if it's a horizontal swipe to the right
    // and not too much vertical movement
    const isHorizontalSwipe = Math.abs(deltaX) > Math.abs(deltaY) * 2
    const isRightSwipe = deltaX > threshold
    const isFastEnough = velocity > velocityThreshold

    if (isHorizontalSwipe && isRightSwipe && isFastEnough) {
      e.preventDefault()
      onSwipeBack()
    }

    isTracking = false
  }

  function handleWheel(e: WheelEvent) {
    // Detect horizontal scroll (trackpad swipe)
    // deltaX > 0 means swipe left (which triggers back navigation)
    // deltaX < 0 means swipe right

    // On macOS, a back swipe gesture produces negative deltaX
    // We want to trigger on significant negative deltaX (swipe right gesture)
    if (e.deltaX < -threshold && Math.abs(e.deltaX) > Math.abs(e.deltaY)) {
      e.preventDefault()
      onSwipeBack()
    }
  }

  onMounted(() => {
    const element = elementRef.value
    if (!element) return

    // Touch events for touch screens
    element.addEventListener('touchstart', handleTouchStart, { passive: false })
    element.addEventListener('touchend', handleTouchEnd, { passive: false })

    // Wheel events for trackpad gestures
    element.addEventListener('wheel', handleWheel, { passive: false })
  })

  onUnmounted(() => {
    const element = elementRef.value
    if (!element) return

    element.removeEventListener('touchstart', handleTouchStart)
    element.removeEventListener('touchend', handleTouchEnd)
    element.removeEventListener('wheel', handleWheel)
  })
}
