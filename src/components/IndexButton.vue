<template>
  <div class="relative">
    <button
      ref="buttonRef"
      @click="toggleMenu"
      :class="[
        'p-1.5 rounded-md transition-all',
        settingsStore.forceS3Search ? 'text-orange-500 dark:text-orange-400 ring-2 ring-orange-500 dark:ring-orange-400' : iconColorClass,
        isIndexing ? 'animate-pulse' : 'hover:bg-muted',
      ]"
      v-tooltip="settingsStore.forceS3Search ? t('forceS3Search') : t('searchIndex')"
    >
      <!-- Database/Index icon -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <ellipse cx="12" cy="5" rx="9" ry="3" />
        <path d="M3 5V19A9 3 0 0 0 21 19V5" />
        <path d="M3 12A9 3 0 0 0 21 12" />
      </svg>
    </button>

    <!-- Backdrop and dropdown menu teleported to body -->
    <Teleport to="body">
      <!-- Backdrop to close menu when clicking outside -->
      <div
        v-if="showMenu"
        @click="showMenu = false"
        class="fixed inset-0 z-[10000]"
      ></div>

      <!-- Dropdown menu -->
      <div
        v-if="showMenu"
        @click.stop
        :style="{ top: `${menuPosition.top}px`, left: `${menuPosition.left}px` }"
        class="fixed w-80 bg-popover text-popover-foreground rounded-md border shadow-lg z-[10001] p-4"
      >
      <div class="space-y-4">
        <div class="space-y-2">
          <h3 class="font-medium text-sm">{{ t('searchIndex') }}</h3>

          <!-- Index metadata (if exists) -->
          <div v-if="indexStats" class="space-y-2 text-sm">
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('lastUpdated') }}:</span>
              <span>{{ indexStats.last_indexed_at ? formatRelativeTime(indexStats.last_indexed_at) : '-' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('objects') }}:</span>
              <span>{{ indexStats.total_objects.toLocaleString() }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('indexSize') }}:</span>
              <span>{{ formatBytes(indexStats.estimated_index_size) }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('status') }}:</span>
              <span :class="indexStats.is_complete ? 'text-green-600 dark:text-green-400' : 'text-yellow-600 dark:text-yellow-400'">
                {{ indexStats.is_complete ? t('complete') : t('partial') }}
              </span>
            </div>
          </div>

          <!-- No index message -->
          <div v-else class="text-sm text-muted-foreground">
            {{ t('noIndexAvailable') }}
          </div>

          <!-- Building message -->
          <div
            v-if="isIndexing"
            class="text-sm text-blue-600 dark:text-blue-400 flex items-center gap-2"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="animate-spin"
            >
              <path d="M21 12a9 9 0 1 1-6.219-8.56" />
            </svg>
            <span>{{ t('buildingIndex') }}...</span>
          </div>
        </div>

        <!-- Force S3 Search Toggle -->
        <div class="pt-3 border-t">
          <button
            @click="settingsStore.forceS3Search = !settingsStore.forceS3Search"
            class="w-full text-left px-3 py-2 text-sm rounded-md flex items-center justify-between gap-2 transition-colors"
            :class="settingsStore.forceS3Search
              ? 'bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300'
              : 'hover:bg-muted'"
          >
            <div class="flex items-center gap-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="11" cy="11" r="8" />
                <path d="m21 21-4.3-4.3" />
              </svg>
              <span>{{ t('forceS3Search') }}</span>
            </div>
            <div
              class="w-9 h-5 rounded-full transition-colors relative"
              :class="settingsStore.forceS3Search ? 'bg-orange-500' : 'bg-muted-foreground/30'"
            >
              <div
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
                :class="settingsStore.forceS3Search ? 'translate-x-4' : 'translate-x-0.5'"
              ></div>
            </div>
          </button>
          <p class="text-xs text-muted-foreground mt-1 px-3">
            {{ t('forceS3SearchDescription') }}
          </p>
        </div>

        <!-- Action buttons -->
        <div class="space-y-2 pt-3 border-t">
          <!-- Rebuild Index button (limited) -->
          <Button
            v-if="!isIndexing"
            @click="handleRebuildIndex"
            variant="outline"
            size="sm"
            class="w-full"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="mr-2"
            >
              <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
              <path d="M3 3v5h5" />
              <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
              <path d="M16 16h5v5" />
            </svg>
            {{ indexStats ? t('rebuildIndex') : t('buildIndex') }}
          </Button>

          <!-- Build Full Index button (only shown when index is partial) -->
          <Button
            v-if="indexStats && !indexStats.is_complete && !isIndexing"
            @click="showFullIndexWarning = true"
            variant="default"
            size="sm"
            class="w-full"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="mr-2"
            >
              <ellipse cx="12" cy="5" rx="9" ry="3" />
              <path d="M3 5V19A9 3 0 0 0 21 19V5" />
              <path d="M3 12A9 3 0 0 0 21 12" />
              <path d="M12 12v6" />
              <path d="M9 15l3 3 3-3" />
            </svg>
            {{ t('buildFullIndex') }}
          </Button>

          <Button
            v-if="indexStats && !isIndexing"
            @click="handleDeleteIndex"
            variant="destructive"
            size="sm"
            class="w-full"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="mr-2"
            >
              <path d="M3 6h18" />
              <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
              <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
            </svg>
            {{ t('deleteIndex') }}
          </Button>
        </div>

        <!-- Full Index Warning Dialog -->
        <div v-if="showFullIndexWarning" class="mt-4 p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-md">
          <div class="flex items-start gap-2 mb-2">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="text-yellow-500 mt-0.5 flex-shrink-0"
            >
              <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
              <path d="M12 9v4" />
              <path d="M12 17h.01" />
            </svg>
            <div class="text-sm">
              <p class="font-medium text-yellow-600 dark:text-yellow-400 mb-1">{{ t('buildFullIndexWarningTitle') }}</p>
              <p class="text-muted-foreground text-xs">{{ t('buildFullIndexWarning') }}</p>
              <p class="text-muted-foreground text-xs mt-1">
                {{ t('currentlyIndexed') }}: <span class="font-medium">{{ indexStats?.total_objects.toLocaleString() }}</span> {{ t('objects').toLowerCase() }}
              </p>
              <p class="text-muted-foreground text-xs mt-1">{{ t('canCancelAnytime') }}</p>
            </div>
          </div>
          <div class="flex gap-2 mt-3">
            <Button
              size="sm"
              variant="outline"
              class="flex-1"
              @click="showFullIndexWarning = false"
            >
              {{ t('cancel') }}
            </Button>
            <Button
              size="sm"
              variant="default"
              class="flex-1"
              @click="handleBuildFullIndex"
            >
              {{ t('startFullIndex') }}
            </Button>
          </div>
        </div>
      </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { getIndexManager } from '../composables/useIndexManager'
import { useI18n } from '../composables/useI18n'
import { useSettingsStore } from '../stores/settings'
import { Button } from './ui/button'
import { logger } from '../utils/logger'
import type { BucketIndexStats } from '../types'

const props = defineProps<{
  profileId: string
  bucketName: string
  totalObjectsInBucket?: number
}>()

const emit = defineEmits<{
  indexChanged: []
}>()

const { t } = useI18n()
const settingsStore = useSettingsStore()
const indexManager = getIndexManager()

const buttonRef = ref<HTMLButtonElement | null>(null)
const showMenu = ref(false)
const showFullIndexWarning = ref(false)
const menuPosition = ref({ top: 0, left: 0 })
const indexStats = ref<BucketIndexStats | null>(null)

// Check if currently indexing
const isIndexing = computed(() => {
  return indexManager.isIndexing(props.profileId, props.bucketName)
})

// Calculate menu position based on button position
function calculateMenuPosition() {
  if (!buttonRef.value) return

  const rect = buttonRef.value.getBoundingClientRect()
  const menuWidth = 320 // w-80 = 320px
  const spacing = 8 // mt-2 = 8px

  // Position below the button, aligned to the right
  menuPosition.value = {
    top: rect.bottom + spacing,
    left: rect.right - menuWidth,
  }
}

// Toggle menu and calculate position
function toggleMenu() {
  showMenu.value = !showMenu.value
  if (showMenu.value) {
    calculateMenuPosition()
  }
}

// Icon state computed property
const iconState = computed(() => {
  if (isIndexing.value) return 'building' // blue blinking
  if (!indexStats.value) return 'missing' // orange
  if (!indexStats.value.is_complete) return 'partial' // yellow
  return 'active' // green
})

// Icon color classes based on state
const iconColorClass = computed(() => {
  switch (iconState.value) {
    case 'building':
      return 'text-blue-500 dark:text-blue-400'
    case 'missing':
      return 'text-orange-500 dark:text-orange-400'
    case 'partial':
      return 'text-yellow-500 dark:text-yellow-400'
    case 'active':
      return 'text-green-600 dark:text-green-400'
    default:
      return 'text-muted-foreground'
  }
})

// Format bytes to human-readable size
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

// Format relative time
function formatRelativeTime(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes < 1) return t('justNow')
  if (minutes < 60) return t('minutesAgo', minutes)
  if (hours < 24) return t('hoursAgo', hours)
  return t('daysAgo', days)
}

// Load index stats
async function loadIndexStats() {
  if (!props.profileId || !props.bucketName) return

  const stats = await indexManager.getIndexStats(props.profileId, props.bucketName)
  indexStats.value = stats
}

// Handle rebuild index (limited by settings)
async function handleRebuildIndex() {
  if (!props.profileId || !props.bucketName) return

  showMenu.value = false
  showFullIndexWarning.value = false

  try {
    // Clear existing index first if it exists
    if (indexStats.value) {
      await indexManager.clearIndex(props.profileId, props.bucketName)
    }
    // Start new indexing (limited by settings)
    await indexManager.startIndexing(props.profileId, props.bucketName)
    await loadIndexStats()
    emit('indexChanged')
  } catch (error) {
    logger.error('Error rebuilding index', error)
  }
}

// Handle build full index (no limit)
async function handleBuildFullIndex() {
  if (!props.profileId || !props.bucketName) return

  showMenu.value = false
  showFullIndexWarning.value = false

  try {
    // Clear existing index first if it exists
    if (indexStats.value) {
      await indexManager.clearIndex(props.profileId, props.bucketName)
    }
    // Start full indexing (no request limit)
    await indexManager.startFullIndexing(props.profileId, props.bucketName)
    await loadIndexStats()
    emit('indexChanged')
  } catch (error) {
    logger.error('Error building full index', error)
  }
}

// Handle delete index
async function handleDeleteIndex() {
  if (!props.profileId || !props.bucketName) return

  showMenu.value = false

  await indexManager.clearIndex(props.profileId, props.bucketName)
  indexStats.value = null
  emit('indexChanged')
}

// Watch for bucket/profile changes
watch(
  () => [props.profileId, props.bucketName],
  () => {
    loadIndexStats()
  },
  { immediate: true }
)

// Watch for indexing completion
watch(
  () => indexManager.indexingBuckets.value,
  () => {
    // When indexing finishes, reload stats
    if (!isIndexing.value) {
      loadIndexStats()
    }
  },
  { deep: true }
)

onMounted(() => {
  loadIndexStats()
})
</script>
