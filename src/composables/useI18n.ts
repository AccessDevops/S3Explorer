import { computed } from 'vue'
import { useSettingsStore } from '../stores/settings'
import { translations, type TranslationKey } from '../i18n/translations'

export function useI18n() {
  const settingsStore = useSettingsStore()

  const t = computed(() => {
    return (key: TranslationKey, ...args: (string | number)[]): string => {
      let translation = translations[settingsStore.language][key] || key

      // Replace placeholders {0}, {1}, etc. with provided arguments
      args.forEach((arg, index) => {
        translation = translation.replace(`{${index}}`, String(arg))
      })

      return translation
    }
  })

  return {
    t: t.value,
    language: computed(() => settingsStore.language),
  }
}
