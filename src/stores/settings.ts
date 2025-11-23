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

  // Load settings from localStorage on init
  const loadSettings = () => {
    const savedLanguage = localStorage.getItem('app-language') as Language | null
    if (savedLanguage) {
      language.value = savedLanguage
    }

    const savedBatchSize = localStorage.getItem('app-batchSize')
    if (savedBatchSize) {
      const size = parseInt(savedBatchSize, 10)
      if (!isNaN(size) && size >= 1 && size <= 1000) {
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
  }

  // Save language to localStorage
  const setLanguage = (lang: Language) => {
    language.value = lang
    localStorage.setItem('app-language', lang)
  }

  // Save batch size to localStorage
  const setBatchSize = (size: number) => {
    if (size < 1 || size > 1000) {
      throw new Error('Batch size must be between 1 and 1000')
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

  return {
    language,
    batchSize,
    searchMode,
    viewMode,
    maxConcurrentUploads,
    multipartThresholdMB,
    loadSettings,
    setLanguage,
    setBatchSize,
    setSearchMode,
    setViewMode,
    setMaxConcurrentUploads,
    setMultipartThresholdMB,
  }
})
