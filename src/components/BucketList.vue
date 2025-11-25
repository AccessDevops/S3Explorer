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

    <div v-if="appStore.isLoading" class="flex justify-center py-8">
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

          <!-- Stats display (if available) -->
          <div v-if="bucketStats[bucket.name]" class="text-xs text-muted-foreground/70 mt-0.5">
            {{ formatSize(bucketStats[bucket.name].size) }} ·
            {{ bucketStats[bucket.name].count }} object{{
              bucketStats[bucket.name].count !== 1 ? 's' : ''
            }}
            <span v-if="bucketStatsIsEstimate[bucket.name]" class="text-yellow-400 ml-1" v-tooltip="t('estimateTooltip')">
              (~)
            </span>
            <button
              @click.stop="loadBucketStats(bucket.name, true)"
              class="ml-2 text-muted-foreground hover:text-foreground transition-colors"
              v-tooltip="t('refresh')"
            >
              ⟳
            </button>
          </div>

          <!-- Loading state -->
          <div
            v-else-if="loadingStats[bucket.name]"
            class="text-xs text-muted-foreground/50 mt-0.5"
          >
            {{ t('loading') }}{{ statsProgress[bucket.name] ? ` (${statsProgress[bucket.name]} pages)` : '...' }}
          </div>

          <!-- Manual calculate buttons (if stats not available) -->
          <div
            v-else
            class="text-xs mt-0.5 flex items-center gap-2"
          >
            <button
              @click.stop="loadQuickEstimate(bucket.name)"
              class="text-yellow-400 hover:text-yellow-300 transition-colors"
              v-tooltip="t('quickEstimate')"
            >
              ⚡ {{ t('estimate') }}
            </button>
            <span class="text-muted-foreground/30">·</span>
            <button
              @click.stop="loadBucketStats(bucket.name, false)"
              class="text-blue-400 hover:text-blue-300 transition-colors"
              v-tooltip="t('fullCalculation')"
            >
              ⟳ {{ t('calculate') }}
            </button>
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
import { ref, computed, onMounted } from 'vue'
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
import { createBucket, getBucketAcl, estimateBucketStats } from '../services/tauri'
import { formatSize, formatDate } from '../utils/formatters'
import { logger } from '../utils/logger'
import { useBucketStats } from '../composables/useBucketStats'

const appStore = useAppStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()
const bucketStatsComposable = useBucketStats()

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

// Import BucketStats type from composable
import type { BucketStats } from '../composables/useBucketStats'

const bucketStats = ref<Record<string, BucketStats>>({})
const bucketStatsIsEstimate = ref<Record<string, boolean>>({}) // Track which stats are estimates
const { loadingStats, statsProgress } = bucketStatsComposable
const bucketAcls = ref<Record<string, string>>({})
const showCreateBucketModal = ref(false)
const newBucketName = ref('')
const creating = ref(false)
const createError = ref<string | null>(null)

// Cache TTL: configurable via settings (default: 24 hours)
const statsCacheTTL = computed(() => settingsStore.bucketStatsCacheTTLHours * 60 * 60 * 1000)

async function selectBucket(bucketName: string) {
  appStore.selectBucket(bucketName)
  await appStore.loadObjects()
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
 * Load quick estimate (fast - only first 1000 objects)
 * Uses a single S3 request for instant preview
 */
async function loadQuickEstimate(bucketName: string) {
  if (!appStore.currentProfile) return

  try {
    loadingStats.value[bucketName] = true

    const [size, count, isEstimate] = await estimateBucketStats(
      appStore.currentProfile.id,
      bucketName
    )

    // Create stats object
    const stats: BucketStats = {
      profileId: appStore.currentProfile.id,
      bucketName,
      size,
      count,
      lastUpdated: Date.now(),
    }

    bucketStats.value[bucketName] = stats
    bucketStatsIsEstimate.value[bucketName] = isEstimate

    // Cache the estimate for quick display later
    if (!isEstimate) {
      // If not an estimate (bucket has ≤ 1000 objects), save as accurate stats
      await bucketStatsComposable.loadBucketStats(
        appStore.currentProfile.id,
        bucketName,
        false,
        statsCacheTTL.value
      )
    }
  } catch (e) {
    logger.error(`Failed to load estimate for bucket ${bucketName}`, e)
  } finally {
    loadingStats.value[bucketName] = false
  }
}

/**
 * Load bucket stats using IndexedDB cache with 24h TTL
 * This replaces the old 30-second in-memory cache
 */
async function loadBucketStats(bucketName: string, forceRefresh = false) {
  if (!appStore.currentProfile) return

  try {
    // First, try to load from cache (fast)
    if (!forceRefresh) {
      const cached = await bucketStatsComposable.getCachedStats(
        appStore.currentProfile.id,
        bucketName
      )
      if (cached) {
        const age = Date.now() - cached.lastUpdated
        if (age < statsCacheTTL.value) {
          // Cache valid, use it
          bucketStats.value[bucketName] = cached
          bucketStatsIsEstimate.value[bucketName] = false // Cached stats are always accurate
          return
        }
      }
    }

    // Cache miss or force refresh - load from S3
    const stats = await bucketStatsComposable.loadBucketStats(
      appStore.currentProfile.id,
      bucketName,
      forceRefresh,
      statsCacheTTL.value
    )

    if (stats) {
      bucketStats.value[bucketName] = stats
      bucketStatsIsEstimate.value[bucketName] = false // Full calculation is always accurate
    }
  } catch (e) {
    logger.error(`Failed to load stats for bucket ${bucketName}`, e)
  }
}

async function loadBucketAcl(bucketName: string) {
  if (!appStore.currentProfile) return

  try {
    const acl = await getBucketAcl(appStore.currentProfile.id, bucketName)
    bucketAcls.value[bucketName] = acl
  } catch (e) {
    logger.error(`Failed to load ACL for bucket ${bucketName}`, e)
    // Don't set ACL if we can't read it (permission issue)
  }
}

/**
 * Load cached stats from IndexedDB on mount (fast, no S3 calls)
 * This provides instant display of previously calculated stats
 */
async function loadCachedStatsOnMount() {
  if (!appStore.currentProfile) return

  for (const bucket of appStore.buckets) {
    const cached = await bucketStatsComposable.getCachedStats(
      appStore.currentProfile.id,
      bucket.name
    )
    if (cached) {
      const age = Date.now() - cached.lastUpdated
      if (age < statsCacheTTL.value) {
        bucketStats.value[bucket.name] = cached
      }
    }
  }
}

// Load cached stats and ACLs when component mounts
onMounted(async () => {
  await loadCachedStatsOnMount()

  // Load ACLs for all buckets to display lock icons
  for (const bucket of appStore.buckets) {
    await loadBucketAcl(bucket.name)
  }
})

function suggestEnablePathStyle() {
  // Open a dialog to explain and offer to edit the profile
  alert(
    'To enable path-style addressing:\n\n1. Click on the Edit button (pencil icon) next to your connection profile\n2. Check the "Force Path Style" checkbox\n3. Save the profile\n4. Try loading buckets again'
  )
}
</script>
