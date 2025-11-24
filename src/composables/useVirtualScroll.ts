import { ref, computed, type Ref, type ComputedRef } from 'vue'

/**
 * Virtual scroll composable for optimizing large lists
 * Renders only visible items + buffer to improve performance
 *
 * Usage:
 * const virtualScroll = useVirtualScroll({
 *   items: allItems,
 *   itemHeight: 48,
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

  /** Height of each item in pixels (must be fixed) */
  itemHeight: number | Ref<number> | ComputedRef<number>

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
}

export function useVirtualScroll<T>(
  options: VirtualScrollOptions<T>
): VirtualScrollReturn<T> {
  const {
    items,
    itemHeight: itemHeightOption,
    containerHeight: containerHeightOption,
    buffer = 10,
    enabled = ref(true),
  } = options

  // Reactive state
  const scrollTop = ref(0)

  // Normalize inputs to refs
  const itemHeight = computed(() =>
    typeof itemHeightOption === 'number'
      ? itemHeightOption
      : itemHeightOption.value
  )

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

  // Calculate total height of all items
  const totalHeight = computed(() => {
    const allItems = Array.isArray(items.value) ? items.value : items.value
    return allItems.length * itemHeight.value
  })

  // Calculate how many items fit in the visible area
  const visibleCount = computed(() =>
    Math.ceil(containerHeight.value / itemHeight.value)
  )

  // Calculate start index (with buffer)
  const startIndex = computed(() => {
    if (!isEnabled.value) return 0

    const index = Math.floor(scrollTop.value / itemHeight.value)
    return Math.max(0, index - buffer)
  })

  // Calculate end index (with buffer)
  const endIndex = computed(() => {
    if (!isEnabled.value) {
      const allItems = Array.isArray(items.value) ? items.value : items.value
      return allItems.length
    }

    const allItems = Array.isArray(items.value) ? items.value : items.value
    const index = startIndex.value + visibleCount.value + buffer * 2
    return Math.min(allItems.length, index)
  })

  // Get visible items slice
  const visibleItems = computed(() => {
    const allItems = Array.isArray(items.value) ? items.value : items.value

    if (!isEnabled.value) {
      return allItems
    }

    return allItems.slice(startIndex.value, endIndex.value)
  })

  // Calculate vertical offset for positioning
  const offsetY = computed(() => {
    if (!isEnabled.value) return 0
    return startIndex.value * itemHeight.value
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
  }
}
