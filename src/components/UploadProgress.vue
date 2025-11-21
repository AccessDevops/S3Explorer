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
          :title="t('cancel')"
        >
          {{ t('cancel') }}
        </button>
        <button
          v-if="!hasActiveUploads"
          @click="clearFinished"
          class="text-xs px-2 py-1 rounded hover:bg-muted-foreground/10 transition-colors"
          :title="t('clear')"
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

    <!-- Upload List -->
    <div v-if="isExpanded" class="max-h-96 overflow-y-auto">
      <div
        v-for="upload in uploads"
        :key="upload.id"
        class="px-4 py-3 border-b border-border last:border-0 hover:bg-muted/50 transition-colors"
      >
        <!-- File name and status -->
        <div class="flex items-start justify-between gap-2 mb-2">
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium truncate" :title="upload.fileName">
              {{ upload.fileName }}
            </div>
            <div class="text-xs text-muted-foreground mt-0.5">
              {{ formatSize(upload.fileSize) }}
              <span v-if="upload.isMultipart" class="ml-1">" Multipart</span>
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
            <PhSpinner
              v-else-if="upload.status === 'uploading'"
              :size="20"
              class="text-primary animate-spin flex-shrink-0"
            />
            <PhClock
              v-else
              :size="20"
              class="text-muted-foreground flex-shrink-0"
            />

            <button
              v-if="upload.status === 'uploading' || upload.status === 'pending'"
              @click="cancelUpload(upload.id)"
              class="p-1 rounded hover:bg-destructive/10 text-destructive transition-colors"
              :title="t('cancel')"
            >
              <PhX :size="16" />
            </button>
          </div>
        </div>

        <!-- Progress bar -->
        <div v-if="upload.progress && upload.status === 'uploading'" class="space-y-1">
          <div class="w-full bg-muted rounded-full h-1.5 overflow-hidden">
            <div
              class="bg-primary h-full transition-all duration-300 ease-out"
              :style="{ width: `${upload.progress.percentage}%` }"
            />
          </div>
          <div class="flex items-center justify-between text-xs text-muted-foreground">
            <span>
              {{ Math.round(upload.progress.percentage) }}%
              <span v-if="upload.isMultipart" class="ml-1">
                ({{ upload.progress.uploadedParts }}/{{ upload.progress.totalParts }} parts)
              </span>
            </span>
            <span>{{ formatSize(upload.progress.uploadedBytes) }}</span>
          </div>
          <div
            v-if="getTimeRemaining(upload.id)"
            class="text-xs text-muted-foreground"
          >
            {{ formatTime(getTimeRemaining(upload.id)!) }} {{ t('remaining') }}
          </div>
        </div>

        <!-- Error message -->
        <div v-if="upload.status === 'failed' && upload.error" class="mt-2">
          <div class="text-xs text-destructive bg-destructive/10 px-2 py-1 rounded">
            {{ upload.error }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useUploadManager } from '../composables/useUploadManager'
import { useI18n } from '../composables/useI18n'
import { formatSize, formatTime } from '../utils/formatters'
import {
  PhCloudArrowUp,
  PhCaretDown,
  PhCaretUp,
  PhCheckCircle,
  PhXCircle,
  PhProhibit,
  PhSpinner,
  PhClock,
  PhX,
} from '@phosphor-icons/vue'

const { t } = useI18n()
const {
  uploads,
  hasActiveUploads,
  uploadCount,
  totalTimeRemaining,
  getTimeRemaining,
  cancelUpload,
  cancelAll,
  clearFinished,
} = useUploadManager()

const isExpanded = ref(true)
</script>
