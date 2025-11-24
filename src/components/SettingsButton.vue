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
      <DialogContent class="max-w-2xl max-h-[85vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle>{{ t('settings') }}</DialogTitle>
        </DialogHeader>

        <!-- Tabs -->
        <div class="flex gap-2 border-b pb-2">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            @click="activeTab = tab.id"
            :class="[
              'px-4 py-2 text-sm font-medium rounded-t-md transition-colors',
              activeTab === tab.id
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:text-foreground hover:bg-muted',
            ]"
          >
            {{ tab.label }}
          </button>
        </div>

        <!-- Tab Content with scroll -->
        <div class="flex-1 overflow-y-auto py-4 px-1">
          <!-- APPEARANCE TAB -->
          <div v-if="activeTab === 'appearance'" class="space-y-6">
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

          <!-- PERFORMANCE TAB -->
          <div v-if="activeTab === 'performance'" class="space-y-6">
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
                max="500"
                class="max-w-xs"
              />
            </div>

            <!-- Max Concurrent Uploads Setting -->
            <div class="space-y-3">
              <div>
                <label class="text-sm font-medium">{{ t('maxConcurrentUploads') }}</label>
                <p class="text-sm text-muted-foreground">{{
                  t('maxConcurrentUploadsDescription')
                }}</p>
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

            <!-- Multipart Upload Threshold Setting -->
            <div class="space-y-3">
              <div>
                <label class="text-sm font-medium">{{ t('multipartThreshold') }}</label>
                <p class="text-sm text-muted-foreground">{{
                  t('multipartThresholdDescription')
                }}</p>
              </div>
              <div class="flex items-center gap-2 max-w-xs">
                <Input
                  type="number"
                  :model-value="settingsStore.multipartThresholdMB"
                  @update:model-value="(val) => settingsStore.setMultipartThresholdMB(Number(val))"
                  min="5"
                  max="1000"
                  class="flex-1"
                />
                <span class="text-sm text-muted-foreground">MB</span>
              </div>
            </div>
          </div>

          <!-- SEARCH TAB -->
          <div v-if="activeTab === 'search'" class="space-y-6">
            <!-- Index Settings Section -->
            <div class="space-y-4 pb-4 border-b">
              <div>
                <h3 class="text-sm font-semibold">{{ t('indexSettings') }}</h3>
                <p class="text-sm text-muted-foreground">{{ t('indexSettingsDescription') }}</p>
              </div>

              <!-- Index Validity Hours Setting -->
              <div class="space-y-3">
                <div>
                  <label class="text-sm font-medium">{{ t('indexValidityHours') }}</label>
                  <p class="text-sm text-muted-foreground">{{
                    t('indexValidityHoursDescription')
                  }}</p>
                </div>
                <div class="flex items-center gap-2 max-w-xs">
                  <Input
                    type="number"
                    :model-value="settingsStore.indexValidityHours"
                    @update:model-value="(val) => settingsStore.setIndexValidityHours(Number(val))"
                    min="1"
                    max="48"
                    class="flex-1"
                  />
                  <span class="text-sm text-muted-foreground">{{ t('hours') }}</span>
                </div>
              </div>

              <!-- Index Auto-Build Threshold Setting -->
              <div class="space-y-3">
                <div>
                  <label class="text-sm font-medium">{{ t('indexAutoBuildThreshold') }}</label>
                  <p class="text-sm text-muted-foreground">{{
                    t('indexAutoBuildThresholdDescription')
                  }}</p>
                </div>
                <div class="flex items-center gap-2 max-w-xs">
                  <Input
                    type="number"
                    :model-value="settingsStore.indexAutoBuildThreshold"
                    @update:model-value="(val) => settingsStore.setIndexAutoBuildThreshold(Number(val))"
                    min="100"
                    max="10000"
                    class="flex-1"
                  />
                  <span class="text-sm text-muted-foreground">{{ t('objects') }}</span>
                </div>
              </div>
            </div>

            <!-- All Indexes Section -->
            <div class="space-y-4">
              <div>
                <label class="text-sm font-medium">{{ t('allIndexes') }}</label>
                <p class="text-sm text-muted-foreground">{{ t('allIndexesDescription') }}</p>
              </div>

            <!-- Empty state -->
            <div
              v-if="allIndexes.length === 0"
              class="text-center py-8 text-muted-foreground"
            >
              {{ t('noIndexesFound') }}
            </div>

            <!-- Table of all indexes -->
            <div v-else class="border rounded-lg overflow-hidden">
              <table class="w-full text-sm">
                <thead class="bg-muted/50">
                  <tr>
                    <th class="text-left py-2 px-3 font-medium">{{ t('connection') }}</th>
                    <th class="text-left py-2 px-3 font-medium">{{ t('bucket') }}</th>
                    <th class="text-right py-2 px-3 font-medium">{{ t('objects') }}</th>
                    <th class="text-right py-2 px-3 font-medium">{{ t('size') }}</th>
                    <th class="text-left py-2 px-3 font-medium">{{ t('created') }}</th>
                    <th class="text-center py-2 px-3 font-medium">{{ t('actions') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="index in allIndexes"
                    :key="index.id"
                    class="border-t hover:bg-muted/30 transition-colors"
                  >
                    <td class="py-2 px-3">{{ index.profileName }}</td>
                    <td class="py-2 px-3 font-mono text-xs">{{ index.bucketName }}</td>
                    <td class="text-right py-2 px-3">{{ index.totalObjects.toLocaleString() }}</td>
                    <td class="text-right py-2 px-3">{{ formatBytes(index.sizeInBytes) }}</td>
                    <td class="py-2 px-3 text-xs text-muted-foreground">
                      {{ formatIndexDate(index.lastBuilt) }}
                    </td>
                    <td class="text-center py-2 px-3">
                      <Button
                        size="sm"
                        variant="destructive"
                        @click="handleDeleteIndexFromTable(index.profileId, index.bucketName)"
                      >
                        {{ t('delete') }}
                      </Button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
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
import { ref, watch } from 'vue'

// Component for settings button and modal
import { useSettingsStore } from '../stores/settings'
import { useAppStore } from '../stores/app'
import { useI18n } from '../composables/useI18n'
import { useSearchIndex } from '../composables/useSearchIndex'
import { useToast } from '../composables/useToast'
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
const appStore = useAppStore()
const { t } = useI18n()
const searchIndex = useSearchIndex()
const toast = useToast()
const showSettingsModal = ref(false)

// Tabs
const activeTab = ref('appearance')
const tabs = [
  { id: 'appearance', label: t('appearance') },
  { id: 'performance', label: t('performance') },
  { id: 'search', label: t('search') },
]

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

// Search Index Management - All indexes table
interface IndexTableRow {
  id: string
  profileId: string
  profileName: string
  bucketName: string
  totalObjects: number
  sizeInBytes: number
  lastBuilt: number
}

const allIndexes = ref<IndexTableRow[]>([])

// Load all indexes for the table
async function loadAllIndexes() {
  const indexes = await searchIndex.getAllIndexes()

  // Map indexes with profile names
  allIndexes.value = indexes.map((index) => {
    const profile = appStore.profiles.find((p) => p.id === index.profileId)
    return {
      id: `${index.profileId}-${index.bucketName}`,
      profileId: index.profileId,
      profileName: profile?.name || t('unknownProfile'),
      bucketName: index.bucketName,
      totalObjects: index.totalObjects,
      sizeInBytes: index.sizeInBytes,
      lastBuilt: index.lastBuilt,
    }
  })
}

// Format bytes to human-readable size
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

// Format date to locale string
function formatIndexDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString()
}

// Update indexes when modal opens or tab changes to search
watch(showSettingsModal, (isOpen) => {
  if (isOpen) {
    loadAllIndexes()
  }
})

watch(activeTab, (tab) => {
  if (tab === 'search') {
    loadAllIndexes()
  }
})

// Delete index from table
async function handleDeleteIndexFromTable(profileId: string, bucketName: string) {
  try {
    await searchIndex.deleteIndex(profileId, bucketName)
    // Reload the table
    await loadAllIndexes()
  } catch (error) {
    console.error('Failed to delete index:', error)
    toast.error(t('errorOccurred'))
  }
}
</script>
