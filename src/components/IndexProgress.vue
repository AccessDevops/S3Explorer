<template>
  <Transition name="slide-up">
    <div
      v-if="hasActiveIndexing"
      class="fixed bottom-4 right-4 bg-background border border-border rounded-lg shadow-lg p-3 min-w-[280px] max-w-[350px] z-50"
    >
      <div class="flex items-center gap-2 mb-2">
        <div class="animate-spin text-blue-400">
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
            <path d="M21 12a9 9 0 1 1-6.219-8.56" />
          </svg>
        </div>
        <span class="text-sm font-medium">{{ t('indexing') }}</span>
      </div>

      <div v-for="(progress, key) in activeProgress" :key="key" class="mb-2 last:mb-0">
        <div class="flex items-center justify-between mb-1">
          <div class="text-xs text-muted-foreground truncate flex-1" :title="progress.bucket_name">
            {{ progress.bucket_name }}
          </div>
          <!-- Stop button -->
          <button
            @click="handleCancel(progress.profile_id, progress.bucket_name)"
            class="ml-2 p-1 text-muted-foreground hover:text-red-500 hover:bg-red-500/10 rounded transition-colors"
            :title="t('cancelIndex')"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="currentColor"
              stroke="none"
            >
              <rect x="4" y="4" width="16" height="16" rx="2" ry="2" />
            </svg>
          </button>
        </div>
        <div class="flex items-center gap-2">
          <div class="flex-1 h-1.5 bg-muted rounded-full overflow-hidden">
            <!-- Indeterminate progress bar when max_requests = 0 (full indexing) -->
            <div
              v-if="progress.max_requests === 0"
              class="h-full bg-blue-500 animate-indeterminate"
              style="width: 30%"
            />
            <!-- Determinate progress bar -->
            <div
              v-else
              class="h-full bg-blue-500 transition-all duration-300"
              :style="{ width: `${getProgressPercent(progress)}%` }"
            />
          </div>
          <span class="text-xs text-muted-foreground min-w-[60px] text-right">
            {{ formatNumber(progress.objects_indexed) }}
          </span>
        </div>
        <div class="text-xs text-muted-foreground/60 mt-0.5">
          <!-- Show different text for full indexing vs limited -->
          <span v-if="progress.max_requests === 0">
            {{ progress.requests_made }} {{ t('requests') }}
          </span>
          <span v-else>
            {{ progress.requests_made }}/{{ progress.max_requests }} {{ t('requests') }}
          </span>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { getIndexManager } from '../composables/useIndexManager'
import { useI18n } from '../composables/useI18n'
import type { IndexProgressEvent } from '../types'

const { indexProgress, indexingBuckets, cancelIndexing } = getIndexManager()
const { t } = useI18n()

const hasActiveIndexing = computed(() =>
  Object.values(indexingBuckets.value).some((v) => v)
)

const activeProgress = computed(() =>
  Object.fromEntries(
    Object.entries(indexProgress.value).filter(([key]) => indexingBuckets.value[key])
  )
)

function getProgressPercent(progress: IndexProgressEvent): number {
  if (progress.max_requests === 0) return 0
  return Math.min(100, (progress.requests_made / progress.max_requests) * 100)
}

function formatNumber(num: number): string {
  return num.toLocaleString()
}

async function handleCancel(profileId: string, bucketName: string) {
  await cancelIndexing(profileId, bucketName)
}
</script>

<style scoped>
.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.3s ease;
}

.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

/* Indeterminate progress bar animation */
.animate-indeterminate {
  animation: indeterminate 1.5s infinite ease-in-out;
}

@keyframes indeterminate {
  0% {
    transform: translateX(-100%);
  }
  50% {
    transform: translateX(200%);
  }
  100% {
    transform: translateX(-100%);
  }
}
</style>
