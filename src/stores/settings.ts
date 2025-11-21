import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Language = 'en' | 'fr'

export const useSettingsStore = defineStore('settings', () => {
  const language = ref<Language>('en')

  // Load settings from localStorage on init
  const loadSettings = () => {
    const savedLanguage = localStorage.getItem('app-language') as Language | null
    if (savedLanguage) {
      language.value = savedLanguage
    }
  }

  // Save language to localStorage
  const setLanguage = (lang: Language) => {
    language.value = lang
    localStorage.setItem('app-language', lang)
  }

  return {
    language,
    loadSettings,
    setLanguage,
  }
})
