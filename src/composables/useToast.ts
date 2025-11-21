import { ref } from 'vue'

export interface Toast {
  id: string
  message: string
  type: 'success' | 'error' | 'info' | 'warning' | 'loading'
  duration?: number
  progress?: number // 0-100 for progress bar
  persistent?: boolean // Don't auto-dismiss
  actionText?: string // Text for action button
  onAction?: () => void // Callback for action button
}

const toasts = ref<Toast[]>([])
let toastId = 0

export function useToast() {
  const addToast = (
    message: string,
    type: Toast['type'] = 'info',
    duration = 3000,
    persistent = false
  ) => {
    const id = `toast-${++toastId}`

    // Adjust duration for info toasts based on number of active toasts
    let adjustedDuration = duration
    if (type === 'info') {
      const activeToastsCount = toasts.value.length

      // Base duration for info toasts is shorter (2 seconds instead of 3)
      if (duration === 3000) {
        adjustedDuration = 2000
      }

      // If there are many toasts, reduce duration even more
      if (activeToastsCount >= 5) {
        adjustedDuration = Math.max(1000, adjustedDuration * 0.5) // 50% faster, min 1 second
      } else if (activeToastsCount >= 3) {
        adjustedDuration = Math.max(1500, adjustedDuration * 0.75) // 25% faster, min 1.5 seconds
      }
    }

    const toast: Toast = {
      id,
      message,
      type,
      duration: adjustedDuration,
      persistent,
      progress: type === 'loading' ? 0 : undefined,
    }

    toasts.value.push(toast)

    if (adjustedDuration > 0 && !persistent) {
      setTimeout(() => {
        removeToast(id)
      }, adjustedDuration)
    }

    return id
  }

  const removeToast = (id: string) => {
    const index = toasts.value.findIndex((t) => t.id === id)
    if (index > -1) {
      toasts.value.splice(index, 1)
    }
  }

  const updateToast = (
    id: string,
    updates: { message?: string; progress?: number; type?: Toast['type'] }
  ) => {
    const toast = toasts.value.find((t) => t.id === id)
    if (toast) {
      if (updates.message !== undefined) toast.message = updates.message
      if (updates.progress !== undefined) toast.progress = updates.progress
      if (updates.type !== undefined) toast.type = updates.type
    }
  }

  const completeToast = (
    id: string,
    message?: string,
    type: 'success' | 'error' = 'success',
    duration = 3000
  ) => {
    const toast = toasts.value.find((t) => t.id === id)
    if (toast) {
      toast.type = type
      if (message) toast.message = message
      toast.progress = undefined
      toast.persistent = false

      setTimeout(() => {
        removeToast(id)
      }, duration)
    }
  }

  const success = (message: string, duration?: number) => {
    return addToast(message, 'success', duration)
  }

  const error = (message: string, duration?: number) => {
    return addToast(message, 'error', duration)
  }

  const info = (message: string, duration?: number) => {
    return addToast(message, 'info', duration)
  }

  const warning = (message: string, duration?: number) => {
    return addToast(message, 'warning', duration)
  }

  const loading = (message: string, actionText?: string, onAction?: () => void) => {
    const id = addToast(message, 'loading', 0, true)

    if (actionText && onAction) {
      const toast = toasts.value.find((t) => t.id === id)
      if (toast) {
        toast.actionText = actionText
        toast.onAction = onAction
      }
    }

    return id
  }

  return {
    toasts,
    addToast,
    removeToast,
    updateToast,
    completeToast,
    success,
    error,
    info,
    warning,
    loading,
  }
}
