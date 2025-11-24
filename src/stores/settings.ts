import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Language = 'en' | 'zh' | 'hi' | 'es' | 'fr' | 'ar' | 'bn' | 'pt' | 'id' | 'ro'
export type SearchMode = 'local' | 'global'
export type ViewMode = 'normal' | 'compact'

export const useSettingsStore = defineStore('settings', () => {
  const language = ref<Language>('en')
  const batchSize = ref(250)
  const searchMode = ref<SearchMode>('local')
  const viewMode = ref<ViewMode>('normal')
  const maxConcurrentUploads = ref(10)
  const multipartThresholdMB = ref(50) // MB - files larger than this use multipart upload
  const indexValidityHours = ref(8) // hours - index older than this is considered expired
  const indexAutoBuildThreshold = ref(12000) // objects - buckets with fewer objects auto-build index

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

  return {
    language,
    batchSize,
    searchMode,
    viewMode,
    maxConcurrentUploads,
    multipartThresholdMB,
    indexValidityHours,
    indexAutoBuildThreshold,
    loadSettings,
    setLanguage,
    setBatchSize,
    setSearchMode,
    setViewMode,
    setMaxConcurrentUploads,
    setMultipartThresholdMB,
    setIndexValidityHours,
    setIndexAutoBuildThreshold,
  }
})
