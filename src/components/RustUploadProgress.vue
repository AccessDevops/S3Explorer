<template>
  <div
    v-if="hasActiveUploads || uploads.length > 0"
    class="fixed bottom-4 right-4 w-96 bg-background border border-border rounded-lg shadow-lg overflow-hidden z-50"
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 bg-muted border-b border-border">
      <div class="flex flex-col gap-0.5">
        <div class="flex items-center gap-2">
          <PhCloudArrowUp :size="20" class="text-primary" />
          <span class="font-medium text-sm">
            {{ t('upload') }} ({{ uploadCount.active }}/{{ uploadCount.total }})
          </span>
          <span v-if="uploadCount.queued > 0" class="text-xs text-muted-foreground">
            +{{ uploadCount.queued }} {{ t('queued') }}
          </span>
        </div>
        <div v-if="hasActiveUploads && totalTimeRemaining > 0" class="text-xs text-muted-foreground ml-7">
          {{ formatTime(totalTimeRemaining) }} {{ t('remaining') }}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="hasActiveUploads"
          @click="cancelAll"
          class="text-xs px-2 py-1 rounded hover:bg-destructive/10 text-destructive transition-colors"
          v-tooltip="t('cancel')"
        >
          {{ t('cancel') }}
        </button>
        <button
          v-if="!hasActiveUploads"
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

    <!-- Upload List with Virtual Scroll -->
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
            v-for="upload in virtualScroll.visibleItems.value"
            :key="upload.uploadId"
            class="upload-item px-4 py-2 border-b border-border last:border-0 hover:bg-muted/50 transition-colors"
            :style="{ height: `${getItemHeight(upload)}px` }"
          >
            <!-- File name and status -->
            <div class="flex items-start justify-between gap-2 mb-1">
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium truncate" v-tooltip="upload.fileName">
                  {{ upload.fileName }}
                </div>
                <div class="text-xs text-muted-foreground mt-0.5">
                  {{ formatSize(upload.fileSize) }}
                  <span v-if="upload.totalParts > 0" class="ml-1">• Multipart</span>
                </div>
              </div>

              <!-- Status icon and cancel button -->
              <div class="flex items-center gap-2">
                <PhCheckCircle
                  v-if="upload.status === 'completed'"
                  :size="20"
                  class="text-green-500 flex-shrink-0"
                />
                <PhXCircle
                  v-else-if="upload.status === 'failed'"
                  :size="20"
                  class="text-destructive flex-shrink-0"
                />
                <PhProhibit
                  v-else-if="upload.status === 'cancelled'"
                  :size="20"
                  class="text-muted-foreground flex-shrink-0"
                />
                <PhClock
                  v-else-if="upload.status === 'queued'"
                  :size="20"
                  class="text-muted-foreground flex-shrink-0"
                />
                <PhSpinner
                  v-else-if="upload.status === 'uploading' || upload.status === 'starting' || upload.status === 'pending'"
                  :size="20"
                  class="text-primary animate-spin flex-shrink-0"
                />

                <button
                  v-if="upload.status === 'uploading' || upload.status === 'pending' || upload.status === 'starting' || upload.status === 'queued'"
                  @click="cancelUpload(upload.uploadId)"
                  class="p-1 rounded hover:bg-destructive/10 text-destructive transition-colors"
                  v-tooltip="t('cancel')"
                >
                  <PhX :size="16" />
                </button>
              </div>
            </div>

            <!-- Progress bar -->
            <div v-if="upload.status === 'uploading' || upload.status === 'starting'" class="space-y-1">
              <div class="w-full bg-muted rounded-full h-1.5 overflow-hidden">
                <div
                  class="bg-primary h-full transition-all duration-300 ease-out"
                  :style="{ width: `${upload.percentage}%` }"
                />
              </div>
              <div class="flex items-center justify-between text-xs text-muted-foreground">
                <span>
                  {{ Math.round(upload.percentage) }}%
                  <span v-if="upload.totalParts > 0" class="ml-1">
                    ({{ upload.uploadedParts }}/{{ upload.totalParts }} parts)
                  </span>
                </span>
                <span>{{ formatSize(upload.uploadedBytes) }}</span>
              </div>
              <div
                v-if="getTimeRemaining(upload.uploadId) || getUploadSpeed(upload.uploadId)"
                class="flex items-center justify-between text-xs text-muted-foreground"
              >
                <span v-if="getTimeRemaining(upload.uploadId)">
                  {{ formatTime(getTimeRemaining(upload.uploadId)!) }} {{ t('remaining') }}
                </span>
                <span v-if="getUploadSpeed(upload.uploadId)">
                  {{ formatSize(getUploadSpeed(upload.uploadId)!) }}/s
                </span>
              </div>
            </div>

            <!-- Error message -->
            <div v-if="upload.status === 'failed' && upload.error" class="mt-1">
              <div class="text-xs text-destructive bg-destructive/10 px-2 py-0.5 rounded truncate" v-tooltip="upload.error">
                {{ upload.error }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRustUploadManager } from '../composables/useRustUploadManager'
import { useVirtualScroll } from '../composables/useVirtualScroll'
import { useI18n } from '../composables/useI18n'
import { formatSize, formatTime } from '../utils/formatters'
import {
  PhCloudArrowUp,
  PhCaretDown,
  PhCaretUp,
  PhCheckCircle,
  PhXCircle,
  PhProhibit,
  PhClock,
  PhSpinner,
  PhX,
} from '@phosphor-icons/vue'

// ============================================================================
// CORRECTION #3: Virtual scroll for upload list with VARIABLE HEIGHTS
// ============================================================================
// With 900 uploads, rendering all DOM elements causes severe performance issues.
// Virtual scroll renders only visible items (~4-8) instead of all 900.
//
// Height strategy:
// - Active uploads (uploading/starting): 100px (shows progress bar, speed, time)
// - Completed/cancelled/failed: 52px (compact - just filename and status)
// - Queued/pending: 60px (filename, size, cancel button)

const ITEM_HEIGHT_ACTIVE = 100  // For uploading items with progress bar
const ITEM_HEIGHT_QUEUED = 60   // For queued items (no progress bar yet)
const ITEM_HEIGHT_COMPACT = 52  // For completed/cancelled/failed items
const CONTAINER_HEIGHT = 400

// Type for upload items
interface UploadItem {
  uploadId: string
  status: string
  fileName: string
  fileSize: number
  percentage: number
  uploadedBytes: number
  totalParts: number
  uploadedParts: number
  error?: string
}

const { t } = useI18n()
const {
  uploads,
  hasActiveUploads,
  uploadCount,
  totalTimeRemaining,
  getTimeRemaining,
  getUploadSpeed,
  cancelUpload,
  cancelAll,
  clearFinished,
} = useRustUploadManager()

const isExpanded = ref(true)

// Determine item height based on upload status
function getItemHeight(item: UploadItem): number {
  switch (item.status) {
    case 'uploading':
    case 'starting':
      return ITEM_HEIGHT_ACTIVE
    case 'queued':
    case 'pending':
      return ITEM_HEIGHT_QUEUED
    case 'completed':
    case 'cancelled':
    case 'failed':
    default:
      return ITEM_HEIGHT_COMPACT
  }
}

// Configure virtual scroll with variable heights
const virtualScroll = useVirtualScroll({
  items: uploads,
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
/* Dynamic height for upload items - set via inline style for virtual scroll */
.upload-item {
  overflow: hidden;
  box-sizing: border-box;
}
</style>
