<template>
  <div class="p-4 border-t border-white/10">
    <Button
      variant="ghost"
      class="w-full justify-start text-white/70 hover:text-white hover:bg-white/10"
      @click="showSettingsModal = true"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="mr-2"
      >
        <path
          d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
        />
        <circle cx="12" cy="12" r="3" />
      </svg>
      {{ t('settings') }}
    </Button>

    <!-- Settings Modal -->
    <Dialog v-model:open="showSettingsModal">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('settings') }}</DialogTitle>
        </DialogHeader>

        <div class="space-y-6 py-4">
          <!-- Language Setting -->
          <div class="space-y-3">
            <div>
              <label class="text-sm font-medium">{{ t('language') }}</label>
              <p class="text-sm text-muted-foreground">{{ t('languageDescription') }}</p>
            </div>

            <div class="grid grid-cols-3 gap-3">
              <button
                v-for="lang in languages"
                :key="lang.code"
                @click="changeLanguage(lang.code)"
                :class="[
                  'p-3 rounded-lg border-2 transition-all text-left',
                  settingsStore.language === lang.code
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/50',
                ]"
              >
                <div class="flex items-center gap-2">
                  <div class="text-xl">{{ lang.flag }}</div>
                  <div class="min-w-0">
                    <div class="font-medium truncate text-sm">{{ lang.native }}</div>
                    <div class="text-xs text-muted-foreground truncate">{{ lang.english }}</div>
                  </div>
                </div>
              </button>
            </div>
          </div>

          <!-- Batch Size Setting -->
          <div class="space-y-3">
            <div>
              <label class="text-sm font-medium">{{ t('batchSizeSetting') }}</label>
              <p class="text-sm text-muted-foreground">{{ t('batchSizeDescription') }}</p>
            </div>
            <Input
              type="number"
              :model-value="settingsStore.batchSize"
              @update:model-value="(val) => settingsStore.setBatchSize(Number(val))"
              min="1"
              max="1000"
              class="max-w-xs"
            />
          </div>

          <!-- Max Concurrent Uploads Setting -->
          <div class="space-y-3">
            <div>
              <label class="text-sm font-medium">{{ t('maxConcurrentUploads') }}</label>
              <p class="text-sm text-muted-foreground">{{ t('maxConcurrentUploadsDescription') }}</p>
            </div>
            <Input
              type="number"
              :model-value="settingsStore.maxConcurrentUploads"
              @update:model-value="(val) => settingsStore.setMaxConcurrentUploads(Number(val))"
              min="1"
              max="30"
              class="max-w-xs"
            />
          </div>

          <!-- View Mode Setting -->
          <div class="space-y-3">
            <div>
              <label class="text-sm font-medium">{{ t('viewMode') }}</label>
              <p class="text-sm text-muted-foreground">{{ t('viewModeDescription') }}</p>
            </div>

            <div class="grid grid-cols-2 gap-3 max-w-md">
              <button
                @click="changeViewMode('normal')"
                :class="[
                  'p-3 rounded-lg border-2 transition-all text-left',
                  settingsStore.viewMode === 'normal'
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/50',
                ]"
              >
                <div class="flex items-center gap-2">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <rect width="18" height="18" x="3" y="3" rx="2" />
                    <path d="M3 9h18" />
                    <path d="M3 15h18" />
                  </svg>
                  <div class="min-w-0">
                    <div class="font-medium text-sm">{{ t('normalView') }}</div>
                    <div class="text-xs text-muted-foreground">{{ t('normalViewDesc') }}</div>
                  </div>
                </div>
              </button>

              <button
                @click="changeViewMode('compact')"
                :class="[
                  'p-3 rounded-lg border-2 transition-all text-left',
                  settingsStore.viewMode === 'compact'
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/50',
                ]"
              >
                <div class="flex items-center gap-2">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <rect width="18" height="18" x="3" y="3" rx="2" />
                    <path d="M3 7h18" />
                    <path d="M3 11h18" />
                    <path d="M3 15h18" />
                    <path d="M3 19h18" />
                  </svg>
                  <div class="min-w-0">
                    <div class="font-medium text-sm">{{ t('compactView') }}</div>
                    <div class="text-xs text-muted-foreground">{{ t('compactViewDesc') }}</div>
                  </div>
                </div>
              </button>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" @click="showSettingsModal = false">{{ t('close') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from '../composables/useI18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import type { Language, ViewMode } from '../stores/settings'

const settingsStore = useSettingsStore()
const { t } = useI18n()
const showSettingsModal = ref(false)

const languages = [
  { code: 'en' as Language, flag: '🇬🇧', native: 'English', english: 'English' },
  { code: 'zh' as Language, flag: '🇨🇳', native: '简体中文', english: 'Chinese' },
  { code: 'hi' as Language, flag: '🇮🇳', native: 'हिन्दी', english: 'Hindi' },
  { code: 'es' as Language, flag: '🇪🇸', native: 'Español', english: 'Spanish' },
  { code: 'fr' as Language, flag: '🇫🇷', native: 'Français', english: 'French' },
  { code: 'ar' as Language, flag: '🇸🇦', native: 'العربية', english: 'Arabic' },
  { code: 'bn' as Language, flag: '🇧🇩', native: 'বাংলা', english: 'Bengali' },
  { code: 'pt' as Language, flag: '🇧🇷', native: 'Português', english: 'Portuguese' },
  { code: 'id' as Language, flag: '🇮🇩', native: 'Bahasa Indonesia', english: 'Indonesian' },
  { code: 'ro' as Language, flag: '🇷🇴', native: 'Română', english: 'Romanian' },
]

function changeLanguage(lang: Language) {
  settingsStore.setLanguage(lang)
}

function changeViewMode(mode: ViewMode) {
  settingsStore.setViewMode(mode)
}
</script>
