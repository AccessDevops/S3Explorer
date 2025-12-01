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
        <div class="text-xs text-muted-foreground mb-1 truncate" :title="progress.bucket_name">
          {{ progress.bucket_name }}
        </div>
        <div class="flex items-center gap-2">
          <div class="flex-1 h-1.5 bg-muted rounded-full overflow-hidden">
            <div
              class="h-full bg-blue-500 transition-all duration-300"
              :style="{ width: `${getProgressPercent(progress)}%` }"
            />
          </div>
          <span class="text-xs text-muted-foreground min-w-[60px] text-right">
            {{ formatNumber(progress.objects_indexed) }}
          </span>
        </div>
        <div class="text-xs text-muted-foreground/60 mt-0.5">
          {{ progress.requests_made }}/{{ progress.max_requests }} {{ t('requests') }}
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

const { indexProgress, indexingBuckets } = getIndexManager()
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
</style>
