<template>
  <div class="p-4">
    <div class="flex justify-between items-center mb-4">
      <h3 class="text-lg font-semibold mr-2">{{ t('buckets') }}</h3>
      <div class="flex items-center gap-3">
        <!-- Search bar for filtering buckets -->
        <div class="relative">
          <Input
            v-model="bucketSearchQuery"
            :placeholder="t('searchBuckets')"
            class="h-7 w-32 pr-7 text-sm border-0 bg-white/5 focus:bg-white/10 focus:ring-1 focus:ring-primary/50"
          />
          <button
            v-if="bucketSearchQuery"
            @click="bucketSearchQuery = ''"
            class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
            v-tooltip="t('clear')"
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
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <div class="flex gap-1">
          <Button
            size="icon"
            variant="ghost"
            @click="showCreateBucketModal = true"
            v-tooltip="t('createBucket')"
            class="h-8 w-8"
          >
            +
          </Button>
          <Button
            size="icon"
            variant="ghost"
            @click="refreshBuckets()"
            v-tooltip="t('refresh')"
            class="h-8 w-8"
          >
            ⟳
          </Button>
        </div>
      </div>
    </div>

    <!-- Only show loading spinner if loading AND no buckets yet (loading buckets, not objects) -->
    <div v-if="appStore.isLoading && appStore.buckets.length === 0" class="flex justify-center py-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-white"></div>
    </div>

    <div v-else class="flex flex-col gap-2">
      <div
        v-for="bucket in filteredBuckets"
        :key="bucket.name"
        class="flex items-center gap-2 p-3 rounded-md cursor-pointer transition-colors"
        :class="
          appStore.currentBucket === bucket.name ? 'bg-white/20' : 'bg-white/5 hover:bg-white/10'
        "
        @click="selectBucket(bucket.name)"
      >
        <span class="text-lg">🗄️</span>
        <div class="flex-1 truncate">
          <div class="flex items-center gap-2">
            <div class="truncate flex-1">{{ bucket.name }}</div>
            <!-- ACL Lock Icon -->
            <div
              v-if="bucketAcls[bucket.name]"
              class="flex-shrink-0"
              v-tooltip="bucketAcls[bucket.name] === 'Public' ? t('bucketPublic') : t('bucketPrivate')"
            >
              <svg
                v-if="bucketAcls[bucket.name] === 'Public'"
                xmlns="http://www.w3.org/2000/svg"
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-yellow-500"
              >
                <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 9.9-1" />
              </svg>
              <svg
                v-else
                xmlns="http://www.w3.org/2000/svg"
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-green-500"
              >
                <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
            </div>
          </div>

          <!-- Stats display from index (only if we have meaningful stats or bucket is complete) -->
          <div v-if="bucketStats[bucket.name] && (bucketStats[bucket.name].total_objects > 0 || bucketStats[bucket.name].is_complete)" class="text-xs text-muted-foreground/70 mt-0.5">
            <span
              :class="{ 'text-amber-400/70': !bucketStats[bucket.name].is_complete }"
              v-tooltip="!bucketStats[bucket.name].is_complete ? t('indexIncompleteTooltip') : undefined"
            >
              {{ formatSize(bucketStats[bucket.name].total_size) }} ·
              {{ bucketStats[bucket.name].total_objects }} object{{
                bucketStats[bucket.name].total_objects !== 1 ? 's' : ''
              }}
            </span>
            <!-- Indexing indicator -->
            <span
              v-if="appStore.currentProfile && indexManager.isIndexing(appStore.currentProfile.id, bucket.name)"
              class="text-blue-400 ml-1 animate-pulse"
              v-tooltip="t('indexingInProgress')"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="inline-block"
              >
                <path d="M21 12a9 9 0 1 1-6.219-8.56" />
              </svg>
            </span>
            <button
              @click.stop="refreshBucketStats(bucket.name)"
              class="ml-2 text-muted-foreground hover:text-foreground transition-colors"
              v-tooltip="t('refresh')"
              :disabled="!!(appStore.currentProfile && indexManager.isIndexing(appStore.currentProfile.id, bucket.name))"
            >
              ⟳
            </button>
          </div>

          <!-- Loading state -->
          <div
            v-else-if="loadingStats[bucket.name]"
            class="text-xs text-muted-foreground/50 mt-0.5"
          >
            {{ t('loading') }}...
          </div>

          <!-- Not indexed yet - show indexing indicator if running -->
          <div
            v-else-if="appStore.currentProfile && indexManager.isIndexing(appStore.currentProfile.id, bucket.name)"
            class="text-xs text-muted-foreground/50 mt-0.5"
          >
            <span class="text-blue-400 animate-pulse">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="inline-block mr-1"
              >
                <path d="M21 12a9 9 0 1 1-6.219-8.56" />
              </svg>
              {{ t('indexingInProgress') }}
            </span>
          </div>

          <!-- Not indexed yet - show placeholder -->
          <div v-else class="text-xs text-muted-foreground/50 mt-0.5">
            <span>- · -</span>
          </div>

          <div class="text-xs text-muted-foreground/60 mt-0.5">
            <span v-if="bucket.creation_date">{{ formatDate(bucket.creation_date) }}</span>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="appStore.error"
      class="mt-4 p-3 bg-red-500/20 rounded-md text-sm text-red-200 border border-red-500/30"
    >
      <div>{{ appStore.error }}</div>

      <!-- Suggestion to enable path_style if not already enabled -->
      <div
        v-if="
          appStore.currentProfile &&
          !appStore.currentProfile.path_style &&
          appStore.currentProfile.endpoint
        "
        class="mt-3 flex items-center gap-2 p-2 bg-blue-500/20 border border-blue-500/30 rounded"
      >
        <span class="text-blue-200 text-xs flex-1"
          >💡 This might be a path-style addressing issue. Try enabling 'Force Path Style' in your
          profile settings.</span
        >
        <Button
          size="sm"
          @click="suggestEnablePathStyle"
          class="text-xs h-7 bg-blue-500 hover:bg-blue-600"
        >
          Open Settings
        </Button>
      </div>
    </div>

    <!-- Create Bucket Modal -->
    <Dialog v-model:open="showCreateBucketModal">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('createBucket') }}</DialogTitle>
        </DialogHeader>
        <div class="space-y-4">
          <div>
            <label class="text-sm font-medium">{{ t('bucketName') }}</label>
            <Input
              v-model="newBucketName"
              :placeholder="t('bucketNamePlaceholder')"
              class="mt-1"
              @keyup.enter="createBucketHandler"
            />
            <p class="text-xs text-muted-foreground mt-1">
              Bucket names must be unique and follow DNS naming conventions
            </p>
          </div>
          <div
            v-if="createError"
            class="p-3 bg-destructive/10 border border-destructive/30 rounded-lg text-destructive text-sm"
          >
            {{ createError }}
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="showCreateBucketModal = false">{{
            t('cancel')
          }}</Button>
          <Button @click="createBucketHandler" :disabled="!newBucketName.trim() || creating">
            {{ creating ? t('creating') : t('create') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from '../composables/useI18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { createBucket } from '../services/tauri'
import { formatSize, formatDate } from '../utils/formatters'
import { logger } from '../utils/logger'
import { getIndexManager } from '../composables/useIndexManager'
import type { BucketIndexStats } from '../types'

const appStore = useAppStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()
const indexManager = getIndexManager()

// Search query for filtering buckets
const bucketSearchQuery = ref('')

// Filtered buckets based on search query
const filteredBuckets = computed(() => {
  if (!bucketSearchQuery.value.trim()) {
    return appStore.buckets
  }

  const query = bucketSearchQuery.value.toLowerCase()
  return appStore.buckets.filter(bucket =>
    bucket.name.toLowerCase().includes(query)
  )
})

const bucketStats = ref<Record<string, BucketIndexStats>>({})
const loadingStats = ref<Record<string, boolean>>({})
const bucketAcls = ref<Record<string, string>>({})
const lastProcessedIndexStatus = ref<Record<string, string>>({}) // Track processed index status to avoid redundant refreshes
const showCreateBucketModal = ref(false)
const newBucketName = ref('')
const creating = ref(false)
const createError = ref<string | null>(null)

async function selectBucket(bucketName: string) {
  appStore.selectBucket(bucketName)
  await appStore.loadObjects()

  // Start background indexing if bucket is not indexed yet
  if (appStore.currentProfile) {
    const isIndexed = await indexManager.isIndexed(appStore.currentProfile.id, bucketName)
    if (!isIndexed) {
      // Start indexing in background (non-blocking) with configured max requests
      indexManager.startIndexing(
        appStore.currentProfile.id,
        bucketName,
        settingsStore.maxInitialIndexRequests
      )
    }
  }
}

async function refreshBuckets() {
  await appStore.loadBuckets()
  // Don't auto-refresh stats anymore (lazy loading)

  // Load ACLs for all buckets to display lock icons
  for (const bucket of appStore.buckets) {
    await loadBucketAcl(bucket.name)
  }
}

async function createBucketHandler() {
  if (!newBucketName.value.trim() || !appStore.currentProfile) return

  try {
    creating.value = true
    createError.value = null

    await createBucket(appStore.currentProfile.id, newBucketName.value.trim())

    // Success: close modal and refresh bucket list
    showCreateBucketModal.value = false
    newBucketName.value = ''
    await appStore.loadBuckets()
    // Don't auto-refresh stats anymore (lazy loading)
  } catch (e: any) {
    // Handle permission errors and other errors
    if (e.toString().includes('AccessDenied') || e.toString().includes('permission')) {
      createError.value = 'Permission denied: You do not have permission to create buckets'
    } else if (e.toString().includes('BucketAlreadyExists')) {
      createError.value = 'This bucket name is already taken'
    } else if (e.toString().includes('InvalidBucketName')) {
      createError.value = 'Invalid bucket name. Must follow DNS naming conventions'
    } else {
      createError.value = `Failed to create bucket: ${e}`
    }
  } finally {
    creating.value = false
  }
}

/**
 * Load bucket stats from the SQLite index
 */
async function loadBucketStatsFromIndex(bucketName: string) {
  if (!appStore.currentProfile) return

  try {
    const stats = await indexManager.getIndexStats(appStore.currentProfile.id, bucketName)
    if (stats) {
      bucketStats.value[bucketName] = stats
    }
  } catch (e) {
    logger.error(`Failed to load stats for bucket ${bucketName}`, e)
  }
}

/**
 * Refresh bucket stats by re-indexing and reloading from index
 */
async function refreshBucketStats(bucketName: string) {
  if (!appStore.currentProfile) return

  try {
    loadingStats.value[bucketName] = true

    // Re-index the bucket to get fresh stats
    await indexManager.startIndexing(
      appStore.currentProfile.id,
      bucketName,
      settingsStore.maxInitialIndexRequests
    )

    // Load the updated stats from the index
    await loadBucketStatsFromIndex(bucketName)
  } catch (e) {
    logger.error(`Failed to refresh stats for bucket ${bucketName}`, e)
  } finally {
    loadingStats.value[bucketName] = false
  }
}

async function loadBucketAcl(bucketName: string) {
  if (!appStore.currentProfile) return

  // Use cached ACL from store (reduces requests by ~90%)
  const acl = await appStore.getCachedBucketAcl(appStore.currentProfile.id, bucketName)
  if (acl) {
    bucketAcls.value[bucketName] = acl
  }
}

/**
 * Load stats from index for all buckets on mount
 */
async function loadStatsFromIndexOnMount() {
  if (!appStore.currentProfile) return

  for (const bucket of appStore.buckets) {
    await loadBucketStatsFromIndex(bucket.name)
  }
}

// Load stats and ACLs when component mounts
onMounted(async () => {
  await loadStatsFromIndexOnMount()

  // Load ACLs for all buckets to display lock icons
  for (const bucket of appStore.buckets) {
    await loadBucketAcl(bucket.name)
  }
})

// Watch for profile changes to clear cached stats (prevent cross-profile contamination)
// Without this, if two profiles have buckets with the same name, stats from profile A
// would incorrectly be shown for profile B
// Note: We watch the profile ID (primitive) instead of the object to avoid
// unnecessary clears when the object reference changes but the profile is the same
watch(
  () => appStore.currentProfile?.id,
  (newId, oldId) => {
    // Only clear if the profile actually changed
    if (newId !== oldId) {
      bucketStats.value = {}
      bucketAcls.value = {}
      lastProcessedIndexStatus.value = {}
    }
  }
)

// Watch for bucket list changes to load ACLs and stats from index
watch(
  () => appStore.buckets,
  async (newBuckets) => {
    if (newBuckets.length > 0 && appStore.currentProfile) {
      // Load stats and ACLs for all buckets when the list changes
      for (const bucket of newBuckets) {
        // Load stats from index if not already loaded
        if (!bucketStats.value[bucket.name]) {
          await loadBucketStatsFromIndex(bucket.name)
        }

        // Load ACL if not already cached locally
        if (!bucketAcls.value[bucket.name]) {
          await loadBucketAcl(bucket.name)
        }
      }
    }
  },
  { immediate: true }
)

// Watch for index completion to auto-refresh bucket stats
watch(
  () => indexManager.indexProgress.value,
  async (progressMap) => {
    if (!appStore.currentProfile) return

    for (const progress of Object.values(progressMap)) {
      // Only react to 'completed' or 'partial' status
      if (progress.status !== 'completed' && progress.status !== 'partial') continue

      // Only update if it's for the current profile
      if (progress.profile_id !== appStore.currentProfile.id) continue

      // Avoid redundant refreshes - check if we already processed this status
      const key = `${progress.profile_id}-${progress.bucket_name}`
      const statusKey = `${progress.status}-${progress.objects_indexed}`
      if (lastProcessedIndexStatus.value[key] === statusKey) continue
      lastProcessedIndexStatus.value[key] = statusKey

      // Reload stats from index for this bucket
      const bucketName = progress.bucket_name
      logger.debug(`[BucketList] Index completed for ${bucketName}, refreshing stats`)
      await loadBucketStatsFromIndex(bucketName)
    }
  },
  { deep: true }
)

function suggestEnablePathStyle() {
  // Open a dialog to explain and offer to edit the profile
  alert(
    'To enable path-style addressing:\n\n1. Click on the Edit button (pencil icon) next to your connection profile\n2. Check the "Force Path Style" checkbox\n3. Save the profile\n4. Try loading buckets again'
  )
}
</script>
