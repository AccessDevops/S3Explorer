import { ref, computed, type Ref, type ComputedRef } from 'vue'

/**
 * Virtual scroll composable for optimizing large lists
 * Renders only visible items + buffer to improve performance
 *
 * Supports both fixed and variable item heights:
 * - Fixed: itemHeight: 48
 * - Variable: getItemHeight: (item) => item.isCompact ? 48 : 100
 *
 * Usage:
 * const virtualScroll = useVirtualScroll({
 *   items: allItems,
 *   itemHeight: 48,  // OR getItemHeight for variable heights
 *   containerHeight: 600,
 *   buffer: 10
 * })
 *
 * Then use:
 * - virtualScroll.visibleItems for rendering
 * - virtualScroll.containerStyle for container
 * - virtualScroll.contentStyle for content wrapper
 * - virtualScroll.handleScroll for scroll handler
 */

export interface VirtualScrollOptions<T> {
  /** All items to virtualize */
  items: Ref<T[]> | ComputedRef<T[]>

  /** Fixed height of each item in pixels */
  itemHeight?: number | Ref<number> | ComputedRef<number>

  /** Function to get height of each item (for variable heights) */
  getItemHeight?: (item: T, index: number) => number

  /** Height of the visible container in pixels */
  containerHeight: number | Ref<number> | ComputedRef<number>

  /** Number of items to render before/after visible area (buffer) */
  buffer?: number

  /** Enable/disable virtualization (useful for debugging) */
  enabled?: Ref<boolean> | ComputedRef<boolean>
}

export interface VirtualScrollReturn<T> {
  /** Items that should be rendered (visible + buffer) */
  visibleItems: ComputedRef<T[]>

  /** Total height of all items combined */
  totalHeight: ComputedRef<number>

  /** Vertical offset for positioning visible items */
  offsetY: ComputedRef<number>

  /** Start index of visible items */
  startIndex: ComputedRef<number>

  /** End index of visible items */
  endIndex: ComputedRef<number>

  /** Container style object */
  containerStyle: ComputedRef<{ height: string; overflow: string }>

  /** Spacer style object (maintains total scroll height) */
  spacerStyle: ComputedRef<{ height: string; position: 'relative' }>

  /** Content wrapper style object (applies offset) */
  contentStyle: ComputedRef<{ transform: string; willChange: string }>

  /** Scroll event handler */
  handleScroll: (event: Event) => void

  /** Current scroll position */
  scrollTop: Ref<number>

  /** Get height for a specific item (useful for inline styles) */
  getHeightForItem: (item: T, index: number) => number

  /** Whether variable heights are being used */
  isVariableHeight: boolean
}

export function useVirtualScroll<T>(
  options: VirtualScrollOptions<T>
): VirtualScrollReturn<T> {
  const {
    items,
    itemHeight: itemHeightOption,
    getItemHeight: getItemHeightOption,
    containerHeight: containerHeightOption,
    buffer = 10,
    enabled = ref(true),
  } = options

  // Determine if we're using variable heights
  const isVariableHeight = !!getItemHeightOption

  // Reactive state
  const scrollTop = ref(0)

  // Default fixed height (used when getItemHeight is not provided)
  const fixedItemHeight = computed(() => {
    if (itemHeightOption === undefined) return 48 // Default fallback
    return typeof itemHeightOption === 'number'
      ? itemHeightOption
      : itemHeightOption.value
  })

  const containerHeight = computed(() =>
    typeof containerHeightOption === 'number'
      ? containerHeightOption
      : containerHeightOption.value
  )

  const isEnabled = computed(() =>
    typeof enabled === 'boolean'
      ? enabled
      : enabled.value
  )

  // Get height for a specific item
  function getHeightForItem(item: T, index: number): number {
    if (getItemHeightOption) {
      return getItemHeightOption(item, index)
    }
    return fixedItemHeight.value
  }

  // Cache for cumulative heights (for variable height mode)
  // This avoids recalculating from scratch on each scroll
  const itemPositions = computed(() => {
    const allItems = items.value
    if (!isVariableHeight) {
      // For fixed heights, we don't need to calculate positions
      return null
    }

    const positions: { top: number; height: number }[] = []
    let cumulativeHeight = 0

    for (let i = 0; i < allItems.length; i++) {
      const height = getHeightForItem(allItems[i], i)
      positions.push({ top: cumulativeHeight, height })
      cumulativeHeight += height
    }

    return positions
  })

  // Calculate total height of all items
  const totalHeight = computed(() => {
    const allItems = items.value

    if (!isVariableHeight) {
      return allItems.length * fixedItemHeight.value
    }

    // With variable heights, sum all positions
    const positions = itemPositions.value
    if (!positions || positions.length === 0) return 0

    const lastItem = positions[positions.length - 1]
    return lastItem.top + lastItem.height
  })

  // Calculate start index (with buffer)
  const startIndex = computed(() => {
    if (!isEnabled.value) return 0
    const _allItems = items.value

    if (!isVariableHeight) {
      // Fixed height: simple calculation
      const index = Math.floor(scrollTop.value / fixedItemHeight.value)
      return Math.max(0, index - buffer)
    }

    // Variable height: binary search for the first visible item
    const positions = itemPositions.value
    if (!positions || positions.length === 0) return 0

    const targetScroll = scrollTop.value
    let low = 0
    let high = positions.length - 1

    while (low < high) {
      const mid = Math.floor((low + high) / 2)
      if (positions[mid].top + positions[mid].height < targetScroll) {
        low = mid + 1
      } else {
        high = mid
      }
    }

    return Math.max(0, low - buffer)
  })

  // Calculate how many items fit in the visible area (approximate for variable heights)
  const visibleCount = computed(() => {
    if (!isVariableHeight) {
      return Math.ceil(containerHeight.value / fixedItemHeight.value)
    }

    // For variable heights, estimate based on average height
    const positions = itemPositions.value
    if (!positions || positions.length === 0) return 10

    const avgHeight = totalHeight.value / positions.length
    return Math.ceil(containerHeight.value / avgHeight) + buffer
  })

  // Calculate end index (with buffer)
  const endIndex = computed(() => {
    const allItems = items.value

    if (!isEnabled.value) {
      return allItems.length
    }

    if (!isVariableHeight) {
      const index = startIndex.value + visibleCount.value + buffer * 2
      return Math.min(allItems.length, index)
    }

    // For variable heights, find items until we exceed container + buffer
    const positions = itemPositions.value
    if (!positions || positions.length === 0) return 0

    const targetBottom = scrollTop.value + containerHeight.value
    let endIdx = startIndex.value

    while (endIdx < positions.length && positions[endIdx].top < targetBottom) {
      endIdx++
    }

    // Add buffer
    return Math.min(allItems.length, endIdx + buffer)
  })

  // Get visible items slice
  const visibleItems = computed(() => {
    const allItems = items.value

    if (!isEnabled.value) {
      return allItems
    }

    return allItems.slice(startIndex.value, endIndex.value)
  })

  // Calculate vertical offset for positioning
  const offsetY = computed(() => {
    if (!isEnabled.value) return 0

    if (!isVariableHeight) {
      return startIndex.value * fixedItemHeight.value
    }

    // For variable heights, get the position of the start item
    const positions = itemPositions.value
    if (!positions || startIndex.value >= positions.length) return 0

    return positions[startIndex.value].top
  })

  // Style objects for template
  const containerStyle = computed(() => ({
    height: `${containerHeight.value}px`,
    overflow: 'auto',
  }))

  const spacerStyle = computed(() => ({
    height: `${totalHeight.value}px`,
    position: 'relative' as const,
  }))

  const contentStyle = computed(() => ({
    transform: `translateY(${offsetY.value}px)`,
    willChange: 'transform',
  }))

  // Scroll event handler
  function handleScroll(event: Event) {
    const target = event.target as HTMLElement
    scrollTop.value = target.scrollTop
  }

  return {
    visibleItems,
    totalHeight,
    offsetY,
    startIndex,
    endIndex,
    containerStyle,
    spacerStyle,
    contentStyle,
    handleScroll,
    scrollTop,
    getHeightForItem,
    isVariableHeight,
  }
}
