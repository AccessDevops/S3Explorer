<template>
  <div
    v-if="hasActiveDownloads || downloads.length > 0"
    class="fixed bottom-4 w-96 bg-background border border-border rounded-lg shadow-lg overflow-hidden z-50 transition-all duration-300"
    :class="isUploadModalVisible ? 'right-[26rem]' : 'right-4'"
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 bg-muted border-b border-border">
      <div class="flex flex-col gap-0.5">
        <div class="flex items-center gap-2">
          <PhCloudArrowDown :size="20" class="text-primary" />
          <span class="font-medium text-sm">
            {{ t('download') }} ({{ downloadCount.active }}/{{ downloadCount.total }})
          </span>
        </div>
        <div v-if="hasActiveDownloads && totalTimeRemaining > 0" class="text-xs text-muted-foreground ml-7">
          {{ formatTime(totalTimeRemaining) }} {{ t('remaining') }}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="hasActiveDownloads"
          @click="cancelAll"
          class="text-xs px-2 py-1 rounded hover:bg-destructive/10 text-destructive transition-colors"
          v-tooltip="t('cancel')"
        >
          {{ t('cancel') }}
        </button>
        <button
          v-if="!hasActiveDownloads"
          @click="clearFinished"
          class="text-xs px-2 py-1 rounded hover:bg-muted-foreground/10 transition-colors"
          v-tooltip="t('clear')"
        >
          {{ t('clear') }}
        </button>
        <button
          @click="isExpanded = !isExpanded"
          class="p-1 rounded hover:bg-muted-foreground/10 transition-colors"
        >
          <PhCaretDown v-if="isExpanded" :size="16" />
          <PhCaretUp v-else :size="16" />
        </button>
      </div>
    </div>

    <!-- Download List with Virtual Scroll -->
    <div
      v-if="isExpanded"
      class="overflow-y-auto"
      :style="{ maxHeight: `${CONTAINER_HEIGHT}px` }"
      @scroll="virtualScroll.handleScroll"
    >
      <!-- Spacer maintains total scroll height -->
      <div :style="virtualScroll.spacerStyle.value">
        <!-- Content wrapper positioned with transform -->
        <div :style="virtualScroll.contentStyle.value">
          <div
            v-for="download in virtualScroll.visibleItems.value"
            :key="download.downloadId"
            class="download-item px-4 py-2 border-b border-border last:border-0 hover:bg-muted/50 transition-colors"
            :style="{ height: `${getItemHeight(download)}px` }"
          >
            <!-- File name and status -->
            <div class="flex items-start justify-between gap-2 mb-1">
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium truncate" v-tooltip="download.fileName">
                  {{ download.fileName }}
                </div>
                <div class="text-xs text-muted-foreground mt-0.5">
                  {{ formatSize(download.fileSize) }}
                </div>
              </div>

              <!-- Status icon and cancel button -->
              <div class="flex items-center gap-2">
                <PhCheckCircle
                  v-if="download.status === 'completed'"
                  :size="20"
                  class="text-green-500 flex-shrink-0"
                />
                <PhXCircle
                  v-else-if="download.status === 'failed'"
                  :size="20"
                  class="text-destructive flex-shrink-0"
                />
                <PhProhibit
                  v-else-if="download.status === 'cancelled'"
                  :size="20"
                  class="text-muted-foreground flex-shrink-0"
                />
                <PhSpinner
                  v-else-if="download.status === 'downloading' || download.status === 'starting' || download.status === 'pending'"
                  :size="20"
                  class="text-primary animate-spin flex-shrink-0"
                />

                <button
                  v-if="download.status === 'downloading' || download.status === 'pending' || download.status === 'starting'"
                  @click="cancelDownload(download.downloadId)"
                  class="p-1 rounded hover:bg-destructive/10 text-destructive transition-colors"
                  v-tooltip="t('cancel')"
                >
                  <PhX :size="16" />
                </button>
              </div>
            </div>

            <!-- Progress bar -->
            <div v-if="download.status === 'downloading' || download.status === 'starting'" class="space-y-1">
              <div class="w-full bg-muted rounded-full h-1.5 overflow-hidden">
                <div
                  class="bg-primary h-full transition-all duration-300 ease-out"
                  :style="{ width: `${download.percentage}%` }"
                />
              </div>
              <div class="flex items-center justify-between text-xs text-muted-foreground">
                <span>{{ Math.round(download.percentage) }}%</span>
                <span>{{ formatSize(download.downloadedBytes) }}</span>
              </div>
              <div
                v-if="getTimeRemaining(download.downloadId) || (download.bytesPerSecond && download.bytesPerSecond > 0)"
                class="flex items-center justify-between text-xs text-muted-foreground"
              >
                <span v-if="getTimeRemaining(download.downloadId)">
                  {{ formatTime(getTimeRemaining(download.downloadId)!) }} {{ t('remaining') }}
                </span>
                <span v-if="download.bytesPerSecond && download.bytesPerSecond > 0">
                  {{ formatSpeed(download.bytesPerSecond) }}
                </span>
              </div>
            </div>

            <!-- Error message -->
            <div v-if="download.status === 'failed' && download.error" class="mt-1">
              <div class="text-xs text-destructive bg-destructive/10 px-2 py-0.5 rounded truncate" v-tooltip="download.error">
                {{ download.error }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useRustDownloadManager } from '../composables/useRustDownloadManager'
import { useRustUploadManager } from '../composables/useRustUploadManager'
import { useVirtualScroll } from '../composables/useVirtualScroll'
import { useI18n } from '../composables/useI18n'
import { formatSize, formatTime } from '../utils/formatters'
import {
  PhCloudArrowDown,
  PhCaretDown,
  PhCaretUp,
  PhCheckCircle,
  PhXCircle,
  PhProhibit,
  PhSpinner,
  PhX,
} from '@phosphor-icons/vue'

// ============================================================================
// Virtual scroll with VARIABLE HEIGHTS (same as RustUploadProgress)
// ============================================================================
// Height strategy:
// - Active downloads (downloading/starting): 100px (shows progress bar, speed, time)
// - Completed/cancelled/failed: 52px (compact - just filename and status)
// - Pending: 60px (filename, size, cancel button)

const ITEM_HEIGHT_ACTIVE = 100  // For downloading items with progress bar
const ITEM_HEIGHT_PENDING = 60  // For pending items (no progress bar yet)
const ITEM_HEIGHT_COMPACT = 52  // For completed/cancelled/failed items
const CONTAINER_HEIGHT = 400

// Type for download items
interface DownloadItem {
  downloadId: string
  status: string
  fileName: string
  fileSize: number
  percentage: number
  downloadedBytes: number
  bytesPerSecond?: number
  error?: string
}

const { t } = useI18n()
const {
  downloads,
  hasActiveDownloads,
  downloadCount,
  totalTimeRemaining,
  getTimeRemaining,
  formatSpeed,
  cancelDownload,
  cancelAll,
  clearFinished,
} = useRustDownloadManager()

// Detect if upload modal is visible to adjust position
const { hasActiveUploads, uploads: uploadsList } = useRustUploadManager()
const isUploadModalVisible = computed(() => {
  return hasActiveUploads.value || uploadsList.value.length > 0
})

const isExpanded = ref(true)

// Determine item height based on download status
function getItemHeight(item: DownloadItem): number {
  switch (item.status) {
    case 'downloading':
    case 'starting':
      return ITEM_HEIGHT_ACTIVE
    case 'pending':
      return ITEM_HEIGHT_PENDING
    case 'completed':
    case 'cancelled':
    case 'failed':
    default:
      return ITEM_HEIGHT_COMPACT
  }
}

// Configure virtual scroll with variable heights
const virtualScroll = useVirtualScroll({
  items: downloads,
  getItemHeight: getItemHeight as (item: unknown, index: number) => number,
  containerHeight: CONTAINER_HEIGHT,
  buffer: 3, // Render 3 extra items above/below viewport
})

// Reset scroll position when collapsing/expanding
watch(isExpanded, (expanded) => {
  if (expanded) {
    virtualScroll.scrollTop.value = 0
  }
})
</script>

<style scoped>
/* Dynamic height for download items - set via inline style for virtual scroll */
.download-item {
  overflow: hidden;
  box-sizing: border-box;
}
</style>
