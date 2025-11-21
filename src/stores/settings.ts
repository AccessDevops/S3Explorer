import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Language = 'en' | 'zh' | 'hi' | 'es' | 'fr' | 'ar' | 'bn' | 'pt' | 'id' | 'ro'
export type SearchMode = 'local' | 'global'

export const useSettingsStore = defineStore('settings', () => {
  const language = ref<Language>('en')
  const batchSize = ref(250)
  const searchMode = ref<SearchMode>('local')

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

  return {
    language,
    batchSize,
    searchMode,
    loadSettings,
    setLanguage,
    setBatchSize,
    setSearchMode,
  }
})
