<template>
  <div class="p-4">
    <div class="flex justify-between items-center mb-4">
      <h3 class="text-lg font-semibold">{{ t('buckets') }}</h3>
      <div class="flex gap-1">
        <Button size="icon" variant="ghost" @click="showCreateBucketModal = true" :title="t('createBucket')" class="h-8 w-8">
          +
        </Button>
        <Button size="icon" variant="ghost" @click="refreshBuckets()" :title="t('refresh')" class="h-8 w-8">
          ⟳
        </Button>
      </div>
    </div>

    <div v-if="appStore.isLoading" class="flex justify-center py-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-white"></div>
    </div>

    <div v-else class="flex flex-col gap-2">
      <div
        v-for="bucket in appStore.buckets"
        :key="bucket.name"
        class="flex items-center gap-2 p-3 rounded-md cursor-pointer transition-colors"
        :class="
          appStore.currentBucket === bucket.name
            ? 'bg-white/20'
            : 'bg-white/5 hover:bg-white/10'
        "
        @click="selectBucket(bucket.name)"
      >
        <span class="text-lg">🗄️</span>
        <div class="flex-1 truncate">
          <div class="truncate">{{ bucket.name }}</div>
          <div v-if="bucketStats[bucket.name]" class="text-xs text-muted-foreground/70 mt-0.5">
            {{ formatSize(bucketStats[bucket.name].size) }} · {{ bucketStats[bucket.name].count }} object{{ bucketStats[bucket.name].count !== 1 ? 's' : '' }}
          </div>
          <div v-else-if="loadingStats[bucket.name]" class="text-xs text-muted-foreground/50 mt-0.5">
            {{ t('loading') }}
          </div>
          <div class="text-xs text-muted-foreground/60 mt-0.5 flex items-center gap-2">
            <span v-if="bucket.creation_date">{{ formatDate(bucket.creation_date) }}</span>
            <span v-if="bucketAcls[bucket.name]" class="inline-flex items-center gap-1">
              <span v-if="bucket.creation_date">·</span>
              <span :class="bucketAcls[bucket.name] === 'Public' ? 'text-yellow-500' : 'text-green-500'">
                {{ bucketAcls[bucket.name] === 'Public' ? '🔓' : '🔒' }} {{ bucketAcls[bucket.name] }}
              </span>
            </span>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="appStore.error"
      class="mt-4 p-3 bg-red-500/20 rounded-md text-sm text-red-200 border border-red-500/30"
    >
      {{ appStore.error }}
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
          <div v-if="createError" class="p-3 bg-destructive/10 border border-destructive/30 rounded-lg text-destructive text-sm">
            {{ createError }}
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="showCreateBucketModal = false">{{ t('cancel') }}</Button>
          <Button @click="createBucketHandler" :disabled="!newBucketName.trim() || creating">
            {{ creating ? t('creating') : t('create') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useAppStore } from '../stores/app'
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
import { createBucket, getBucketAcl, calculateBucketStats } from '../services/tauri'

const appStore = useAppStore()
const { t } = useI18n()

interface BucketStats {
  size: number
  count: number
}

interface CachedStats {
  stats: BucketStats
  timestamp: number
}

const bucketStats = ref<Record<string, BucketStats>>({})
const loadingStats = ref<Record<string, boolean>>({})
const bucketAcls = ref<Record<string, string>>({})
const statsCache = ref<Record<string, CachedStats>>({})
const showCreateBucketModal = ref(false)
const newBucketName = ref('')
const creating = ref(false)
const createError = ref<string | null>(null)

// Cache TTL: 30 seconds
const STATS_CACHE_TTL = 30000

async function selectBucket(bucketName: string) {
  appStore.selectBucket(bucketName)
  await appStore.loadObjects()
}

async function refreshBuckets() {
  await appStore.loadBuckets()
  await loadAllBucketStats(true) // Force refresh when explicitly refreshing
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
    await loadAllBucketStats(true) // Force refresh after creating bucket
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

async function loadBucketStats(bucketName: string, forceRefresh = false) {
  if (!appStore.currentProfile) return

  // Check cache first
  const cached = statsCache.value[bucketName]
  const now = Date.now()

  if (!forceRefresh && cached && (now - cached.timestamp) < STATS_CACHE_TTL) {
    // Use cached stats
    bucketStats.value[bucketName] = cached.stats
    return
  }

  try {
    loadingStats.value[bucketName] = true

    // Use the dedicated calculateBucketStats function that lists ALL objects without delimiter
    const [totalSize, totalCount] = await calculateBucketStats(
      appStore.currentProfile.id,
      bucketName
    )

    const stats = {
      size: totalSize,
      count: totalCount
    }

    bucketStats.value[bucketName] = stats

    // Update cache
    statsCache.value[bucketName] = {
      stats,
      timestamp: now
    }
  } catch (e) {
    console.error(`Failed to load stats for bucket ${bucketName}:`, e)
  } finally {
    loadingStats.value[bucketName] = false
  }
}

async function loadBucketAcl(bucketName: string) {
  if (!appStore.currentProfile) return

  try {
    const acl = await getBucketAcl(appStore.currentProfile.id, bucketName)
    bucketAcls.value[bucketName] = acl
  } catch (e) {
    console.error(`Failed to load ACL for bucket ${bucketName}:`, e)
    // Don't set ACL if we can't read it (permission issue)
  }
}

async function loadAllBucketStats(forceRefresh = false) {
  if (!appStore.currentProfile) return

  // Load stats and ACLs for all buckets in parallel
  await Promise.all(
    appStore.buckets.flatMap(bucket => [
      loadBucketStats(bucket.name, forceRefresh),
      loadBucketAcl(bucket.name)
    ])
  )
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString)
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    })
  } catch {
    return dateString
  }
}

// Watch for changes in buckets list
watch(() => appStore.buckets, async (newBuckets) => {
  if (newBuckets.length > 0) {
    await loadAllBucketStats()
  }
})

// Load stats on mount if buckets are already loaded
onMounted(async () => {
  if (appStore.buckets.length > 0) {
    await loadAllBucketStats()
  }
})
</script>
