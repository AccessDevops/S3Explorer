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
        <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
        <circle cx="12" cy="12" r="3"/>
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

            <div class="grid grid-cols-2 gap-3">
              <button
                @click="changeLanguage('en')"
                :class="[
                  'p-4 rounded-lg border-2 transition-all text-left',
                  settingsStore.language === 'en'
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/50'
                ]"
              >
                <div class="flex items-center gap-3">
                  <div class="text-2xl">🇬🇧</div>
                  <div>
                    <div class="font-medium">English</div>
                    <div class="text-sm text-muted-foreground">English</div>
                  </div>
                </div>
              </button>

              <button
                @click="changeLanguage('fr')"
                :class="[
                  'p-4 rounded-lg border-2 transition-all text-left',
                  settingsStore.language === 'fr'
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/50'
                ]"
              >
                <div class="flex items-center gap-3">
                  <div class="text-2xl">🇫🇷</div>
                  <div>
                    <div class="font-medium">Français</div>
                    <div class="text-sm text-muted-foreground">French</div>
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
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import type { Language } from '../stores/settings'

const settingsStore = useSettingsStore()
const { t } = useI18n()
const showSettingsModal = ref(false)

function changeLanguage(lang: Language) {
  settingsStore.setLanguage(lang)
}
</script>
