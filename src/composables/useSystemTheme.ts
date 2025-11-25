import { ref, onMounted, onUnmounted } from 'vue'

/**
 * Composable to detect and track system theme preference
 * Returns a reactive ref that updates when the system theme changes
 */
export function useSystemTheme() {
  const isDark = ref(false)
  let mediaQuery: MediaQueryList | null = null

  const updateTheme = (e: MediaQueryListEvent | MediaQueryList) => {
    isDark.value = e.matches
  }

  onMounted(() => {
    // Check if window.matchMedia is available
    if (typeof window !== 'undefined' && window.matchMedia) {
      mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

      // Set initial value
      isDark.value = mediaQuery.matches

      // Listen for changes
      // Use addEventListener for modern browsers
      if (mediaQuery.addEventListener) {
        mediaQuery.addEventListener('change', updateTheme)
      } else {
        // Fallback for older browsers
        mediaQuery.addListener(updateTheme)
      }
    }
  })

  onUnmounted(() => {
    // Clean up listener
    if (mediaQuery) {
      if (mediaQuery.removeEventListener) {
        mediaQuery.removeEventListener('change', updateTheme)
      } else {
        // Fallback for older browsers
        mediaQuery.removeListener(updateTheme)
      }
    }
  })

  return {
    isDark,
  }
}
