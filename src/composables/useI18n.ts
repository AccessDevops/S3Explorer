import { computed } from 'vue'
import { useSettingsStore } from '../stores/settings'
import { translations, type TranslationKey } from '../i18n/translations'

export function useI18n() {
  const settingsStore = useSettingsStore()

  const t = computed(() => {
    return (key: TranslationKey): string => {
      return translations[settingsStore.language][key] || key
    }
  })

  return {
    t: t.value,
    language: computed(() => settingsStore.language),
  }
}
