import type { Directive, DirectiveBinding } from 'vue'

interface TooltipOptions {
  text: string
  side?: 'top' | 'right' | 'bottom' | 'left'
  sideOffset?: number
  delayDuration?: number
  showArrow?: boolean
}

interface TooltipElement extends HTMLElement {
  _tooltipCleanup?: () => void
  _tooltipEl?: HTMLElement
  _showTimeout?: ReturnType<typeof setTimeout>
  _hideTimeout?: ReturnType<typeof setTimeout>
}

function getOptions(binding: DirectiveBinding): TooltipOptions | null {
  let text: string
  let side: TooltipOptions['side'] = 'top'
  let sideOffset = 8
  let delayDuration = 150
  let showArrow = true

  // Handle modifiers for side
  if (binding.modifiers.top) side = 'top'
  if (binding.modifiers.bottom) side = 'bottom'
  if (binding.modifiers.left) side = 'left'
  if (binding.modifiers.right) side = 'right'

  // Handle modifiers for arrow
  if (binding.modifiers.noArrow) showArrow = false

  // Handle value
  if (typeof binding.value === 'string') {
    text = binding.value
  } else if (binding.value && typeof binding.value === 'object') {
    text = binding.value.text || ''
    if (binding.value.side) side = binding.value.side
    if (binding.value.sideOffset !== undefined) sideOffset = binding.value.sideOffset
    if (binding.value.delayDuration !== undefined) delayDuration = binding.value.delayDuration
    if (binding.value.showArrow !== undefined) showArrow = binding.value.showArrow
  } else {
    return null
  }

  if (!text) return null

  return { text, side, sideOffset, delayDuration, showArrow }
}

function createTooltipElement(options: TooltipOptions): HTMLElement {
  const tooltip = document.createElement('div')
  tooltip.className = 'v-tooltip'
  tooltip.setAttribute('role', 'tooltip')
  tooltip.innerHTML = `
    <div class="v-tooltip-content">${escapeHtml(options.text)}</div>
    ${options.showArrow ? '<div class="v-tooltip-arrow"></div>' : ''}
  `
  tooltip.dataset.side = options.side || 'top'
  return tooltip
}

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

function positionTooltip(el: HTMLElement, tooltip: HTMLElement, options: TooltipOptions) {
  const rect = el.getBoundingClientRect()
  const tooltipRect = tooltip.getBoundingClientRect()
  const offset = options.sideOffset || 8
  const scrollX = window.scrollX
  const scrollY = window.scrollY

  let top = 0
  let left = 0

  switch (options.side) {
    case 'top':
      top = rect.top + scrollY - tooltipRect.height - offset
      left = rect.left + scrollX + (rect.width - tooltipRect.width) / 2
      break
    case 'bottom':
      top = rect.bottom + scrollY + offset
      left = rect.left + scrollX + (rect.width - tooltipRect.width) / 2
      break
    case 'left':
      top = rect.top + scrollY + (rect.height - tooltipRect.height) / 2
      left = rect.left + scrollX - tooltipRect.width - offset
      break
    case 'right':
      top = rect.top + scrollY + (rect.height - tooltipRect.height) / 2
      left = rect.right + scrollX + offset
      break
  }

  // Keep tooltip within viewport
  const padding = 8
  const viewportWidth = window.innerWidth
  const viewportHeight = window.innerHeight

  // Horizontal bounds
  if (left < padding) {
    left = padding
  } else if (left + tooltipRect.width > viewportWidth - padding) {
    left = viewportWidth - tooltipRect.width - padding
  }

  // Vertical bounds
  if (top < padding + scrollY) {
    top = padding + scrollY
  } else if (top + tooltipRect.height > viewportHeight + scrollY - padding) {
    top = viewportHeight + scrollY - tooltipRect.height - padding
  }

  tooltip.style.top = `${top}px`
  tooltip.style.left = `${left}px`
}

function showTooltip(el: TooltipElement, options: TooltipOptions) {
  if (el._tooltipEl) return

  const tooltip = createTooltipElement(options)
  tooltip.style.visibility = 'hidden'
  document.body.appendChild(tooltip)

  // Position after adding to DOM (need dimensions)
  requestAnimationFrame(() => {
    positionTooltip(el, tooltip, options)
    tooltip.style.visibility = 'visible'
    tooltip.classList.add('v-tooltip-visible')
  })

  el._tooltipEl = tooltip
}

function hideTooltip(el: TooltipElement) {
  if (el._tooltipEl) {
    el._tooltipEl.classList.remove('v-tooltip-visible')
    const tooltipEl = el._tooltipEl
    setTimeout(() => {
      tooltipEl.remove()
    }, 150) // Match animation duration
    el._tooltipEl = undefined
  }
}

function setupTooltip(el: TooltipElement, options: TooltipOptions) {
  cleanupTooltip(el)

  const onMouseEnter = () => {
    if (el._hideTimeout) {
      clearTimeout(el._hideTimeout)
      el._hideTimeout = undefined
    }
    el._showTimeout = setTimeout(() => {
      showTooltip(el, options)
    }, options.delayDuration)
  }

  const onMouseLeave = () => {
    if (el._showTimeout) {
      clearTimeout(el._showTimeout)
      el._showTimeout = undefined
    }
    el._hideTimeout = setTimeout(() => {
      hideTooltip(el)
    }, 50)
  }

  const onFocus = () => {
    showTooltip(el, options)
  }

  const onBlur = () => {
    hideTooltip(el)
  }

  el.addEventListener('mouseenter', onMouseEnter)
  el.addEventListener('mouseleave', onMouseLeave)
  el.addEventListener('focus', onFocus)
  el.addEventListener('blur', onBlur)

  el._tooltipCleanup = () => {
    if (el._showTimeout) clearTimeout(el._showTimeout)
    if (el._hideTimeout) clearTimeout(el._hideTimeout)
    el.removeEventListener('mouseenter', onMouseEnter)
    el.removeEventListener('mouseleave', onMouseLeave)
    el.removeEventListener('focus', onFocus)
    el.removeEventListener('blur', onBlur)
    hideTooltip(el)
  }
}

function cleanupTooltip(el: TooltipElement) {
  if (el._tooltipCleanup) {
    el._tooltipCleanup()
    el._tooltipCleanup = undefined
  }
}

export const vTooltip: Directive<TooltipElement> = {
  mounted(el, binding) {
    const options = getOptions(binding)
    if (options) {
      setupTooltip(el, options)
    }
  },
  updated(el, binding) {
    const options = getOptions(binding)
    if (options) {
      setupTooltip(el, options)
    } else {
      cleanupTooltip(el)
    }
  },
  unmounted(el) {
    cleanupTooltip(el)
  },
}

export default vTooltip
