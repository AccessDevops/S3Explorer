<template>
  <div class="relative">
    <button
      ref="buttonRef"
      @click="toggleMenu"
      :class="[
        'p-1.5 rounded-md transition-all',
        iconColorClass,
        isBuilding ? 'animate-pulse' : 'hover:bg-muted',
      ]"
      v-tooltip="t('searchIndex')"
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
          <div v-if="indexMetadata" class="space-y-2 text-sm">
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('lastUpdated') }}:</span>
              <span>{{ formatRelativeTime(indexMetadata.lastBuilt) }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('objects') }}:</span>
              <span>{{ indexMetadata.totalObjects.toLocaleString() }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">{{ t('size') }}:</span>
              <span>{{ formatBytes(indexMetadata.sizeInBytes) }}</span>
            </div>
          </div>

          <!-- No index message -->
          <div v-else class="text-sm text-muted-foreground">
            {{ t('noIndexAvailable') }}
          </div>

          <!-- Building message -->
          <div
            v-if="isBuilding"
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
            <span>{{ t('buildingIndex') }}... {{ searchIndex.buildProgress.value.toLocaleString() }}</span>
          </div>
        </div>

        <!-- Toggle to enable/disable index -->
        <div
          v-if="indexMetadata && !isBuilding"
          class="flex items-center justify-between pt-3 border-t"
        >
          <label class="text-sm font-medium cursor-pointer" :for="`index-toggle-${profileId}-${bucketName}`">
            {{ t('useIndexForSearch') }}
          </label>
          <input
            :id="`index-toggle-${profileId}-${bucketName}`"
            type="checkbox"
            v-model="indexEnabled"
            @change="handleToggleIndex(indexEnabled)"
            class="w-11 h-6 bg-gray-200 dark:bg-gray-700 rounded-full peer appearance-none cursor-pointer checked:bg-primary relative
                   after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-5 after:w-5
                   after:transition-all checked:after:translate-x-full"
          />
        </div>

        <!-- Action buttons -->
        <div class="space-y-2 pt-3 border-t">
          <Button
            v-if="!isBuilding"
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
            {{ indexMetadata ? t('rebuildIndex') : t('buildIndex') }}
          </Button>

          <Button
            v-if="indexMetadata && !isBuilding"
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
      </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useSearchIndex } from '../composables/useSearchIndex'
import { useI18n } from '../composables/useI18n'
import { useSettingsStore } from '../stores/settings'
import { Button } from './ui/button'
import { logger } from '../utils/logger'

const props = defineProps<{
  profileId: string
  bucketName: string
  totalObjectsInBucket?: number
}>()

const emit = defineEmits<{
  indexChanged: []
}>()

const { t } = useI18n()
const searchIndex = useSearchIndex()
const _settingsStore = useSettingsStore()

const buttonRef = ref<HTMLButtonElement | null>(null)
const showMenu = ref(false)
const menuPosition = ref({ top: 0, left: 0 })
const indexMetadata = ref<{
  lastBuilt: number
  totalObjects: number
  sizeInBytes: number
} | null>(null)
const indexEnabled = ref(false)

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
  if (searchIndex.isBuilding.value) return 'building' // blue blinking
  if (!indexMetadata.value) return 'missing' // orange
  if (indexEnabled.value) return 'active' // green
  return 'inactive' // gray
})

// Icon color classes based on state
const iconColorClass = computed(() => {
  switch (iconState.value) {
    case 'building':
      return 'text-blue-500 dark:text-blue-400'
    case 'missing':
      return 'text-orange-500 dark:text-orange-400'
    case 'active':
      return 'text-green-600 dark:text-green-400'
    case 'inactive':
      return 'text-muted-foreground'
    default:
      return 'text-muted-foreground'
  }
})

const isBuilding = computed(() => searchIndex.isBuilding.value)

// Format bytes to human-readable size
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
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

// Load index metadata
async function loadIndexMetadata() {
  if (!props.profileId || !props.bucketName) return

  const metadata = await searchIndex.getIndexMetadata(props.profileId, props.bucketName)
  indexMetadata.value = metadata

  // Load enabled state
  indexEnabled.value = searchIndex.isIndexEnabled(props.profileId, props.bucketName)
}

// Handle toggle index enabled/disabled
function handleToggleIndex(enabled: boolean) {
  searchIndex.setIndexEnabled(props.profileId, props.bucketName, enabled)
  emit('indexChanged')
}

// Handle rebuild index
async function handleRebuildIndex() {
  if (!props.profileId || !props.bucketName) return

  showMenu.value = false

  try {
    await searchIndex.rebuildIndex(
      props.profileId,
      props.bucketName
    )
    await loadIndexMetadata()
    emit('indexChanged')
  } catch (error) {
    logger.error('Error rebuilding index', error)
  }
}

// Handle delete index
async function handleDeleteIndex() {
  if (!props.profileId || !props.bucketName) return

  showMenu.value = false

  await searchIndex.deleteIndex(props.profileId, props.bucketName)
  indexMetadata.value = null
  emit('indexChanged')
}

// Watch for bucket/profile changes
watch(
  () => [props.profileId, props.bucketName],
  () => {
    loadIndexMetadata()
  },
  { immediate: true }
)

// Watch for build completion
watch(
  () => searchIndex.isBuilding.value,
  (newVal, oldVal) => {
    // When building finishes, reload metadata
    if (oldVal && !newVal) {
      loadIndexMetadata()
    }
  }
)

onMounted(() => {
  loadIndexMetadata()
})
</script>
