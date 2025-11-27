import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { S3Provider, S3Pricing } from '@/types/metrics'
import { PROVIDER_PRICING } from '@/types/metrics'

export type Language = 'en' | 'zh' | 'hi' | 'es' | 'fr' | 'ar' | 'bn' | 'pt' | 'id' | 'ro'
export type SearchMode = 'local' | 'global'
export type ViewMode = 'normal' | 'compact'
export type EditorTheme = 'dark' | 'light' | 'high-contrast' | 'system'
export type MonacoTheme = 'vs-dark' | 'vs' | 'hc-black'

export const useSettingsStore = defineStore('settings', () => {
  const language = ref<Language>('en')

  // Batch size for object navigation (smaller = faster UI response, less data per request)
  // Note: Index building always uses 1000 (S3 maximum) for optimal performance
  const batchSize = ref(250)

  const searchMode = ref<SearchMode>('local')
  const viewMode = ref<ViewMode>('normal')
  const maxConcurrentUploads = ref(10)
  const multipartThresholdMB = ref(20) // MB - files larger than this use multipart upload
  const indexValidityHours = ref(8) // hours - index older than this is considered expired
  const indexAutoBuildThreshold = ref(12000) // objects - buckets with fewer objects auto-build index
  const bucketStatsCacheTTLHours = ref(24) // hours - bucket stats cache TTL (default: 24h)
  const previewWarningLimitMB = ref(10) // MB - files larger than this show warning before loading
  const previewMaxLimitMB = ref(80) // MB - files larger than this cannot be previewed
  const editorTheme = ref<EditorTheme>('system') // editor theme - dark, light, high-contrast, or system

  // Metrics pricing settings
  const metricsProvider = ref<S3Provider>('aws')
  const customPricing = ref<S3Pricing>({ ...PROVIDER_PRICING.custom })

  // Track system theme preference reactively
  const systemPrefersDark = ref(false)

  // Initialize system theme detection
  if (typeof window !== 'undefined' && window.matchMedia) {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    systemPrefersDark.value = mediaQuery.matches

    // Listen for system theme changes
    const updateSystemTheme = (e: MediaQueryListEvent) => {
      systemPrefersDark.value = e.matches
    }

    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener('change', updateSystemTheme)
    } else {
      // Fallback for older browsers
      mediaQuery.addListener(updateSystemTheme)
    }
  }

  // Load settings from localStorage on init
  const loadSettings = () => {
    const savedLanguage = localStorage.getItem('app-language') as Language | null
    if (savedLanguage) {
      language.value = savedLanguage
    }

    const savedBatchSize = localStorage.getItem('app-batchSize')
    if (savedBatchSize) {
      const size = parseInt(savedBatchSize, 10)
      // Enforce max 500 to prevent UI performance issues with large lists
      if (!isNaN(size) && size >= 1 && size <= 500) {
        batchSize.value = size
      }
    }

    const savedSearchMode = localStorage.getItem('app-searchMode') as SearchMode | null
    if (savedSearchMode === 'local' || savedSearchMode === 'global') {
      searchMode.value = savedSearchMode
    }

    const savedViewMode = localStorage.getItem('app-viewMode') as ViewMode | null
    if (savedViewMode === 'normal' || savedViewMode === 'compact') {
      viewMode.value = savedViewMode
    }

    const savedMaxConcurrentUploads = localStorage.getItem('app-maxConcurrentUploads')
    if (savedMaxConcurrentUploads) {
      const max = parseInt(savedMaxConcurrentUploads, 10)
      if (!isNaN(max) && max >= 1 && max <= 30) {
        maxConcurrentUploads.value = max
      }
    }

    const savedMultipartThreshold = localStorage.getItem('app-multipartThresholdMB')
    if (savedMultipartThreshold) {
      const threshold = parseInt(savedMultipartThreshold, 10)
      if (!isNaN(threshold) && threshold >= 5 && threshold <= 1000) {
        multipartThresholdMB.value = threshold
      }
    }

    const savedIndexValidityHours = localStorage.getItem('app-indexValidityHours')
    if (savedIndexValidityHours) {
      const hours = parseInt(savedIndexValidityHours, 10)
      if (!isNaN(hours) && hours >= 1 && hours <= 48) {
        indexValidityHours.value = hours
      }
    }

    const savedIndexAutoBuildThreshold = localStorage.getItem('app-indexAutoBuildThreshold')
    if (savedIndexAutoBuildThreshold) {
      const threshold = parseInt(savedIndexAutoBuildThreshold, 10)
      if (!isNaN(threshold) && threshold >= 100 && threshold <= 100000) {
        indexAutoBuildThreshold.value = threshold
      }
    }

    const savedBucketStatsCacheTTL = localStorage.getItem('app-bucketStatsCacheTTLHours')
    if (savedBucketStatsCacheTTL) {
      const hours = parseInt(savedBucketStatsCacheTTL, 10)
      if (!isNaN(hours) && hours >= 1 && hours <= 168) {
        bucketStatsCacheTTLHours.value = hours
      }
    }

    const savedPreviewWarningLimit = localStorage.getItem('app-previewWarningLimitMB')
    if (savedPreviewWarningLimit) {
      const limit = parseInt(savedPreviewWarningLimit, 10)
      if (!isNaN(limit) && limit >= 10 && limit <= 500) {
        previewWarningLimitMB.value = limit
      }
    }

    const savedPreviewMaxLimit = localStorage.getItem('app-previewMaxLimitMB')
    if (savedPreviewMaxLimit) {
      const limit = parseInt(savedPreviewMaxLimit, 10)
      if (!isNaN(limit) && limit >= 100 && limit <= 5000) {
        previewMaxLimitMB.value = limit
      }
    }

    const savedEditorTheme = localStorage.getItem('app-editorTheme') as EditorTheme | null
    if (savedEditorTheme === 'dark' || savedEditorTheme === 'light' || savedEditorTheme === 'high-contrast' || savedEditorTheme === 'system') {
      editorTheme.value = savedEditorTheme
    }

    // Load metrics pricing settings
    const savedMetricsProvider = localStorage.getItem('app-metricsProvider') as S3Provider | null
    if (savedMetricsProvider && savedMetricsProvider in PROVIDER_PRICING) {
      metricsProvider.value = savedMetricsProvider
    }

    const savedCustomPricing = localStorage.getItem('app-customPricing')
    if (savedCustomPricing) {
      try {
        const parsed = JSON.parse(savedCustomPricing)
        if (typeof parsed.getPerThousand === 'number' &&
            typeof parsed.putPerThousand === 'number' &&
            typeof parsed.listPerThousand === 'number' &&
            typeof parsed.deletePerThousand === 'number') {
          customPricing.value = parsed
        }
      } catch {
        // Invalid JSON, use default
      }
    }
  }

  // Save language to localStorage
  const setLanguage = (lang: Language) => {
    language.value = lang
    localStorage.setItem('app-language', lang)
  }

  // Save batch size to localStorage
  const setBatchSize = (size: number) => {
    // Enforce max 500 to prevent UI performance issues with large lists
    if (size < 1 || size > 500) {
      throw new Error('Batch size must be between 1 and 500')
    }
    batchSize.value = size
    localStorage.setItem('app-batchSize', String(size))
  }

  // Save search mode to localStorage
  const setSearchMode = (mode: SearchMode) => {
    searchMode.value = mode
    localStorage.setItem('app-searchMode', mode)
  }

  // Save view mode to localStorage
  const setViewMode = (mode: ViewMode) => {
    viewMode.value = mode
    localStorage.setItem('app-viewMode', mode)
  }

  // Save max concurrent uploads to localStorage
  const setMaxConcurrentUploads = (max: number) => {
    if (max < 1 || max > 30) {
      throw new Error('Max concurrent uploads must be between 1 and 30')
    }
    maxConcurrentUploads.value = max
    localStorage.setItem('app-maxConcurrentUploads', String(max))
  }

  // Save multipart threshold to localStorage
  const setMultipartThresholdMB = (threshold: number) => {
    if (threshold < 5 || threshold > 1000) {
      throw new Error('Multipart threshold must be between 5 and 1000 MB')
    }
    multipartThresholdMB.value = threshold
    localStorage.setItem('app-multipartThresholdMB', String(threshold))
  }

  // Save index validity hours to localStorage
  const setIndexValidityHours = (hours: number) => {
    if (hours < 1 || hours > 48) {
      throw new Error('Index validity must be between 1 and 48 hours')
    }
    indexValidityHours.value = hours
    localStorage.setItem('app-indexValidityHours', String(hours))
  }

  // Save index auto-build threshold to localStorage
  const setIndexAutoBuildThreshold = (threshold: number) => {
    if (threshold < 100 || threshold > 100000) {
      throw new Error('Index auto-build threshold must be between 100 and 100000 objects')
    }
    indexAutoBuildThreshold.value = threshold
    localStorage.setItem('app-indexAutoBuildThreshold', String(threshold))
  }

  // Save bucket stats cache TTL to localStorage
  const setBucketStatsCacheTTLHours = (hours: number) => {
    if (hours < 1 || hours > 168) {
      throw new Error('Bucket stats cache TTL must be between 1 and 168 hours (1 week)')
    }
    bucketStatsCacheTTLHours.value = hours
    localStorage.setItem('app-bucketStatsCacheTTLHours', String(hours))
  }

  // Save preview warning limit to localStorage
  const setPreviewWarningLimitMB = (limit: number) => {
    if (isNaN(limit) || limit < 10 || limit > 500) {
      return // Silently ignore invalid values during typing
    }
    previewWarningLimitMB.value = limit
    localStorage.setItem('app-previewWarningLimitMB', String(limit))
  }

  // Save preview max limit to localStorage
  const setPreviewMaxLimitMB = (limit: number) => {
    if (isNaN(limit) || limit < 10 || limit > 5000) {
      return // Silently ignore invalid values during typing
    }
    previewMaxLimitMB.value = limit
    localStorage.setItem('app-previewMaxLimitMB', String(limit))
  }

  // Save editor theme to localStorage
  const setEditorTheme = (theme: EditorTheme) => {
    editorTheme.value = theme
    localStorage.setItem('app-editorTheme', theme)
  }

  // Save metrics provider to localStorage
  const setMetricsProvider = (provider: S3Provider) => {
    metricsProvider.value = provider
    localStorage.setItem('app-metricsProvider', provider)
  }

  // Save custom pricing to localStorage
  const setCustomPricing = (pricing: S3Pricing) => {
    customPricing.value = { ...pricing }
    localStorage.setItem('app-customPricing', JSON.stringify(pricing))
  }

  // Get current pricing based on provider selection
  const getCurrentPricing = computed<S3Pricing>(() => {
    if (metricsProvider.value === 'custom') {
      return customPricing.value
    }
    return PROVIDER_PRICING[metricsProvider.value]
  })

  // Computed property to get Monaco theme based on editor theme setting
  // This reactively detects system theme when 'system' is selected
  const getMonacoTheme = computed<MonacoTheme>(() => {
    if (editorTheme.value === 'dark') {
      return 'vs-dark'
    } else if (editorTheme.value === 'light') {
      return 'vs'
    } else if (editorTheme.value === 'high-contrast') {
      return 'hc-black'
    } else {
      // system - use reactive system preference
      return systemPrefersDark.value ? 'vs-dark' : 'vs'
    }
  })

  return {
    language,
    batchSize,
    searchMode,
    viewMode,
    maxConcurrentUploads,
    multipartThresholdMB,
    indexValidityHours,
    indexAutoBuildThreshold,
    bucketStatsCacheTTLHours,
    previewWarningLimitMB,
    previewMaxLimitMB,
    editorTheme,
    loadSettings,
    setLanguage,
    setBatchSize,
    setSearchMode,
    setViewMode,
    setMaxConcurrentUploads,
    setMultipartThresholdMB,
    setIndexValidityHours,
    setIndexAutoBuildThreshold,
    setBucketStatsCacheTTLHours,
    setPreviewWarningLimitMB,
    setPreviewMaxLimitMB,
    setEditorTheme,
    getMonacoTheme,
    // Metrics pricing
    metricsProvider,
    customPricing,
    setMetricsProvider,
    setCustomPricing,
    getCurrentPricing,
  }
})
