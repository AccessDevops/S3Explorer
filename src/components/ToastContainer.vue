<template>
  <div class="fixed top-4 right-4 z-[10001] flex flex-col gap-2 pointer-events-none">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :class="[
          'pointer-events-auto rounded-lg shadow-lg border min-w-[300px] max-w-[500px] overflow-hidden',
          'animate-in slide-in-from-right',
          toastStyles[toast.type],
        ]"
      >
        <div class="px-4 py-3 flex items-center gap-3">
          <div class="text-xl flex-shrink-0">
            <span v-if="toast.type === 'loading'" class="animate-spin inline-block">⏳</span>
            <span v-else>{{ toastIcons[toast.type] }}</span>
          </div>
          <div class="flex-1 text-sm font-medium">{{ toast.message }}</div>
          <button
            v-if="toast.actionText && toast.onAction"
            @click="toast.onAction"
            class="px-3 py-1 text-xs font-semibold rounded bg-white/20 hover:bg-white/30 transition-colors flex-shrink-0"
          >
            {{ toast.actionText }}
          </button>
          <button
            v-if="!toast.persistent"
            @click="removeToast(toast.id)"
            class="flex-shrink-0 hover:opacity-70 transition-opacity"
          >
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
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <!-- Progress bar -->
        <div v-if="toast.progress !== undefined" class="h-1 bg-black/10 dark:bg-white/10">
          <div
            class="h-full transition-all duration-300 ease-out"
            :class="progressBarStyles[toast.type]"
            :style="{ width: `${toast.progress}%` }"
          />
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { useToast } from '../composables/useToast'

const { toasts, removeToast } = useToast()

const toastStyles = {
  success:
    'bg-green-50 dark:bg-green-950 text-green-800 dark:text-green-200 border-green-200 dark:border-green-800',
  error:
    'bg-red-50 dark:bg-red-950 text-red-800 dark:text-red-200 border-red-200 dark:border-red-800',
  warning:
    'bg-yellow-50 dark:bg-yellow-950 text-yellow-800 dark:text-yellow-200 border-yellow-200 dark:border-yellow-800',
  info: 'bg-blue-50 dark:bg-blue-950 text-blue-800 dark:text-blue-200 border-blue-200 dark:border-blue-800',
  loading:
    'bg-blue-50 dark:bg-blue-950 text-blue-800 dark:text-blue-200 border-blue-200 dark:border-blue-800',
}

const progressBarStyles = {
  success: 'bg-green-600 dark:bg-green-400',
  error: 'bg-red-600 dark:bg-red-400',
  warning: 'bg-yellow-600 dark:bg-yellow-400',
  info: 'bg-blue-600 dark:bg-blue-400',
  loading: 'bg-blue-600 dark:bg-blue-400',
}

const toastIcons = {
  success: '✓',
  error: '✕',
  warning: '⚠',
  info: 'ℹ',
  loading: '⏳',
}
</script>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}

@keyframes slide-in-from-right {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.animate-in {
  animation: slide-in-from-right 0.3s ease-out;
}
</style>
