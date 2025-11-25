<template>
  <!-- File Context Menu -->
  <Transition name="fade">
    <div
      v-if="show && targetObject"
      :style="{ top: `${y}px`, left: `${x}px` }"
      class="fixed z-[100] min-w-[200px] max-h-[80vh] rounded-lg border bg-background p-1 shadow-lg backdrop-blur-sm"
      @click.stop
      style="overflow: visible"
    >
      <!-- Copy -->
      <button
        :class="[
          'flex w-full items-center gap-2 rounded-md hover:bg-accent',
          props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
          props.textSize
        ]"
        @click="$emit('copy')"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          :width="props.iconSize"
          :height="props.iconSize"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
          <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
        </svg>
        {{ t('copy') }}
      </button>

      <!-- Rename -->
      <button
        :class="[
          'flex w-full items-center gap-2 rounded-md hover:bg-accent',
          props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
          props.textSize
        ]"
        @click="$emit('rename')"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          :width="props.iconSize"
          :height="props.iconSize"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
          <path d="m15 5 4 4" />
        </svg>
        {{ t('rename') }}
      </button>

      <!-- Change Content Type (with submenu) -->
      <div
        class="relative"
        @mouseenter="showContentTypeSubmenu = true"
        @mouseleave="showContentTypeSubmenu = false"
      >
        <button
          class="flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent"
        >
          <div class="flex items-center gap-2">
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
              <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            {{ t('changeContentType') }}
          </div>
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
            <path d="m9 18 6-6-6-6" />
          </svg>
        </button>

        <!-- Content Type Submenu -->
        <Transition name="fade">
          <div
            v-if="showContentTypeSubmenu"
            :class="[
              'absolute left-full min-w-[180px] overflow-y-auto rounded-lg border bg-background p-1 shadow-lg backdrop-blur-sm z-[110]',
              isInUpperHalf ? 'top-0' : 'bottom-0'
            ]"
            :style="{ maxHeight: contentTypeSubmenuMaxHeight }"
            @click.stop
            @mouseenter="showContentTypeSubmenu = true"
            @mouseleave="showContentTypeSubmenu = false"
          >
            <!-- Current Content Type (if known) -->
            <button
              v-if="props.currentContentType"
              :class="[
                'flex w-full items-center gap-2 rounded-md hover:bg-accent border-b border-border mb-1',
                props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
                props.textSize
              ]"
              @click="$emit('change-content-type', props.currentContentType)"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                :width="props.iconSize"
                :height="props.iconSize"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-primary"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span class="font-medium">{{ props.currentContentType }}</span>
              <span class="text-xs text-muted-foreground ml-auto">Current</span>
            </button>

            <!-- Recommended Content Type (only if different from current) -->
            <button
              v-if="recommendedContentType && recommendedContentType !== props.currentContentType"
              :class="[
                'flex w-full items-center gap-2 rounded-md hover:bg-accent border-b border-border mb-1',
                props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
                props.textSize
              ]"
              @click="$emit('change-content-type', recommendedContentType)"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                :width="props.iconSize"
                :height="props.iconSize"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-muted-foreground opacity-50"
              >
                <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
              </svg>
              <span class="font-medium">{{ recommendedContentType }}</span>
              <span class="text-xs text-muted-foreground ml-auto">Recommended</span>
            </button>

            <!-- All Content Types -->
            <button
              v-for="ct in contentTypes"
              :key="ct.value"
              :class="[
                'flex w-full items-center gap-2 rounded-md hover:bg-accent',
                props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
                props.textSize
              ]"
              @click="$emit('change-content-type', ct.value)"
            >
              <div :style="{ width: props.iconSize + 'px' }" class="flex-shrink-0">
                <!-- Show check icon if this is the current content type -->
                <svg
                  v-if="ct.value === props.currentContentType"
                  xmlns="http://www.w3.org/2000/svg"
                  :width="props.iconSize"
                  :height="props.iconSize"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="text-primary"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </div>
              {{ ct.label }}
            </button>
          </div>
        </Transition>
      </div>
    </div>
  </Transition>

  <!-- Empty Area Context Menu -->
  <Transition name="fade">
    <div
      v-if="showEmpty"
      :style="{ top: `${emptyY}px`, left: `${emptyX}px`, maxHeight: `calc(100vh - ${emptyY}px - 20px)` }"
      class="fixed z-[100] min-w-[200px] rounded-lg border bg-background p-1 shadow-lg backdrop-blur-sm overflow-y-auto"
      @click.stop
    >
      <!-- Paste -->
      <button
        :class="[
          'flex w-full items-center gap-2 rounded-md hover:bg-accent',
          props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
          props.textSize,
          { 'opacity-50': !hasCopiedFile }
        ]"
        :disabled="!hasCopiedFile"
        @click="$emit('paste')"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          :width="props.iconSize"
          :height="props.iconSize"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M15 2H9a1 1 0 0 0-1 1v2c0 .6.4 1 1 1h6c.6 0 1-.4 1-1V3c0-.6-.4-1-1-1Z" />
          <path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2" />
        </svg>
        {{ t('paste') }}
      </button>

      <div class="my-1 h-px bg-border"></div>

      <!-- New File -->
      <button
        :class="[
          'flex w-full items-center gap-2 rounded-md hover:bg-accent',
          props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
          props.textSize
        ]"
        @click="$emit('new-file')"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          :width="props.iconSize"
          :height="props.iconSize"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
          <polyline points="14 2 14 8 20 8" />
          <line x1="12" x2="12" y1="18" y2="12" />
          <line x1="9" x2="15" y1="15" y2="15" />
        </svg>
        {{ t('newFile') }}
      </button>

      <!-- New Folder -->
      <button
        :class="[
          'flex w-full items-center gap-2 rounded-md hover:bg-accent',
          props.isCompactView ? 'px-2 py-1' : 'px-3 py-2',
          props.textSize
        ]"
        @click="$emit('new-folder')"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          :width="props.iconSize"
          :height="props.iconSize"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path
            d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"
          />
          <line x1="12" x2="12" y1="10" y2="16" />
          <line x1="9" x2="15" y1="13" y2="13" />
        </svg>
        {{ t('newFolder') }}
      </button>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from '../composables/useI18n'
import type { S3Object } from '../types'

interface Props {
  // File context menu
  show: boolean
  x: number
  y: number
  targetObject: S3Object | null
  currentContentType?: string | null
  // Empty area context menu
  showEmpty: boolean
  emptyX: number
  emptyY: number
  hasCopiedFile: boolean
  // View mode sizing
  iconSize?: number
  textSize?: string
  isCompactView?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  iconSize: 16,
  textSize: 'text-sm',
  isCompactView: false,
  currentContentType: null
})

defineEmits<{
  copy: []
  rename: []
  'view-versions': []
  'change-content-type': [contentType: string]
  paste: []
  'new-file': []
  'new-folder': []
}>()

const { t } = useI18n()

// Local state for content type submenu
const showContentTypeSubmenu = ref(false)

// Detect if menu is in upper or lower half of screen
const isInUpperHalf = computed(() => {
  const menuY = props.show ? props.y : props.emptyY
  return menuY < window.innerHeight / 2
})

// Calculate max height for content type submenu based on available space
const contentTypeSubmenuMaxHeight = computed(() => {
  const menuY = props.show ? props.y : props.emptyY
  const defaultMaxHeight = window.innerHeight * 0.6 // 60vh
  const padding = 80 // Safety padding to avoid touching screen edge

  // Calculate available space depending on whether menu is in upper or lower half
  let availableHeight: number
  if (isInUpperHalf.value) {
    // Menu is in upper half, submenu grows downward
    availableHeight = window.innerHeight - menuY
  } else {
    // Menu is in lower half, submenu grows upward
    availableHeight = menuY
  }

  // Use the smaller of available height or default max height
  return `${Math.min(availableHeight - padding, defaultMaxHeight)}px`
})

// Common content types (sorted alphabetically)
const contentTypes = computed(() => {
  const types = [
    // Text
    { label: 'text/plain', value: 'text/plain' },
    { label: 'text/html', value: 'text/html' },
    { label: 'text/css', value: 'text/css' },
    { label: 'text/csv', value: 'text/csv' },
    { label: 'text/xml', value: 'text/xml' },
    { label: 'text/markdown', value: 'text/markdown' },
    { label: 'text/yaml', value: 'text/yaml' },
    { label: 'text/rtf', value: 'text/rtf' },
    { label: 'text/calendar', value: 'text/calendar' },
    { label: 'text/x-python', value: 'text/x-python' },
    { label: 'text/x-typescript', value: 'text/x-typescript' },
    { label: 'text/x-java', value: 'text/x-java' },
    { label: 'text/x-c', value: 'text/x-c' },
    { label: 'text/x-go', value: 'text/x-go' },
    { label: 'text/x-rust', value: 'text/x-rust' },
    { label: 'text/x-ruby', value: 'text/x-ruby' },
    { label: 'text/x-php', value: 'text/x-php' },
    { label: 'text/x-shellscript', value: 'text/x-shellscript' },
    { label: 'text/x-sql', value: 'text/x-sql' },
    // Application - General
    { label: 'application/json', value: 'application/json' },
    { label: 'application/ld+json', value: 'application/ld+json' },
    { label: 'application/x-ndjson', value: 'application/x-ndjson' },
    { label: 'application/javascript', value: 'application/javascript' },
    { label: 'application/xml', value: 'application/xml' },
    { label: 'application/pdf', value: 'application/pdf' },
    { label: 'application/wasm', value: 'application/wasm' },
    { label: 'application/x-httpd-php', value: 'application/x-httpd-php' },
    { label: 'application/x-sh', value: 'application/x-sh' },
    { label: 'application/sql', value: 'application/sql' },
    { label: 'application/x-www-form-urlencoded', value: 'application/x-www-form-urlencoded' },
    { label: 'multipart/form-data', value: 'multipart/form-data' },
    // Application - Archives
    { label: 'application/zip', value: 'application/zip' },
    { label: 'application/gzip', value: 'application/gzip' },
    { label: 'application/x-tar', value: 'application/x-tar' },
    { label: 'application/x-7z-compressed', value: 'application/x-7z-compressed' },
    { label: 'application/vnd.rar', value: 'application/vnd.rar' },
    { label: 'application/x-bzip', value: 'application/x-bzip' },
    { label: 'application/x-bzip2', value: 'application/x-bzip2' },
    { label: 'application/x-xz', value: 'application/x-xz' },
    // Application - Java/Mobile
    { label: 'application/java-archive', value: 'application/java-archive' },
    { label: 'application/vnd.android.package-archive', value: 'application/vnd.android.package-archive' },
    // Application - Data formats
    { label: 'application/vnd.apache.parquet', value: 'application/vnd.apache.parquet' },
    { label: 'application/vnd.apache.avro+json', value: 'application/vnd.apache.avro+json' },
    { label: 'application/protobuf', value: 'application/protobuf' },
    // Images
    { label: 'image/jpeg', value: 'image/jpeg' },
    { label: 'image/png', value: 'image/png' },
    { label: 'image/gif', value: 'image/gif' },
    { label: 'image/svg+xml', value: 'image/svg+xml' },
    { label: 'image/webp', value: 'image/webp' },
    { label: 'image/avif', value: 'image/avif' },
    { label: 'image/heic', value: 'image/heic' },
    { label: 'image/heif', value: 'image/heif' },
    { label: 'image/bmp', value: 'image/bmp' },
    { label: 'image/tiff', value: 'image/tiff' },
    { label: 'image/x-icon', value: 'image/x-icon' },
    // Video
    { label: 'video/mp4', value: 'video/mp4' },
    { label: 'video/mpeg', value: 'video/mpeg' },
    { label: 'video/webm', value: 'video/webm' },
    { label: 'video/ogg', value: 'video/ogg' },
    { label: 'video/quicktime', value: 'video/quicktime' },
    { label: 'video/x-msvideo', value: 'video/x-msvideo' },
    { label: 'video/x-matroska', value: 'video/x-matroska' },
    { label: 'video/x-flv', value: 'video/x-flv' },
    // Audio
    { label: 'audio/mpeg', value: 'audio/mpeg' },
    { label: 'audio/mp4', value: 'audio/mp4' },
    { label: 'audio/aac', value: 'audio/aac' },
    { label: 'audio/x-m4a', value: 'audio/x-m4a' },
    { label: 'audio/wav', value: 'audio/wav' },
    { label: 'audio/flac', value: 'audio/flac' },
    { label: 'audio/ogg', value: 'audio/ogg' },
    { label: 'audio/opus', value: 'audio/opus' },
    { label: 'audio/webm', value: 'audio/webm' },
    // Documents - Microsoft Office
    { label: 'application/msword', value: 'application/msword' },
    { label: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', value: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' },
    { label: 'application/vnd.ms-excel', value: 'application/vnd.ms-excel' },
    { label: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', value: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' },
    { label: 'application/vnd.ms-powerpoint', value: 'application/vnd.ms-powerpoint' },
    { label: 'application/vnd.openxmlformats-officedocument.presentationml.presentation', value: 'application/vnd.openxmlformats-officedocument.presentationml.presentation' },
    // Documents - eBooks
    { label: 'application/epub+zip', value: 'application/epub+zip' },
    { label: 'application/vnd.amazon.ebook', value: 'application/vnd.amazon.ebook' },
    // Fonts
    { label: 'font/woff', value: 'font/woff' },
    { label: 'font/woff2', value: 'font/woff2' },
    { label: 'font/ttf', value: 'font/ttf' },
    { label: 'font/otf', value: 'font/otf' },
    // Other
    { label: 'application/octet-stream', value: 'application/octet-stream' },
  ]

  // Sort alphabetically by label
  return types.sort((a, b) => a.label.localeCompare(b.label))
})

// Determine recommended content type based on file extension
const recommendedContentType = computed(() => {
  if (!props.targetObject) return null

  const ext = props.targetObject.key.split('.').pop()?.toLowerCase()

  const extensionMap: Record<string, string> = {
    // Text - General
    'txt': 'text/plain',
    'html': 'text/html',
    'htm': 'text/html',
    'css': 'text/css',
    'csv': 'text/csv',
    'xml': 'text/xml',
    'md': 'text/markdown',
    'markdown': 'text/markdown',
    'yaml': 'text/yaml',
    'yml': 'text/yaml',
    'rtf': 'text/rtf',
    'ics': 'text/calendar',
    // Text - Programming
    'py': 'text/x-python',
    'ts': 'text/x-typescript',
    'java': 'text/x-java',
    'c': 'text/x-c',
    'h': 'text/x-c',
    'cpp': 'text/x-c',
    'go': 'text/x-go',
    'rs': 'text/x-rust',
    'rb': 'text/x-ruby',
    'php': 'application/x-httpd-php',
    'sh': 'text/x-shellscript',
    'bash': 'text/x-shellscript',
    'sql': 'text/x-sql',
    // Application - General
    'json': 'application/json',
    'jsonld': 'application/ld+json',
    'jsonl': 'application/x-ndjson',
    'ndjson': 'application/x-ndjson',
    'js': 'application/javascript',
    'mjs': 'application/javascript',
    'pdf': 'application/pdf',
    'wasm': 'application/wasm',
    // Application - Archives
    'zip': 'application/zip',
    'gz': 'application/gzip',
    'gzip': 'application/gzip',
    'tar': 'application/x-tar',
    '7z': 'application/x-7z-compressed',
    'rar': 'application/vnd.rar',
    'bz': 'application/x-bzip',
    'bz2': 'application/x-bzip2',
    'bzip2': 'application/x-bzip2',
    'xz': 'application/x-xz',
    // Application - Java/Mobile
    'jar': 'application/java-archive',
    'apk': 'application/vnd.android.package-archive',
    // Application - Data formats
    'parquet': 'application/vnd.apache.parquet',
    'avro': 'application/vnd.apache.avro+json',
    'proto': 'application/protobuf',
    'protobuf': 'application/protobuf',
    // Images
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'png': 'image/png',
    'gif': 'image/gif',
    'svg': 'image/svg+xml',
    'webp': 'image/webp',
    'avif': 'image/avif',
    'heic': 'image/heic',
    'heif': 'image/heif',
    'bmp': 'image/bmp',
    'tif': 'image/tiff',
    'tiff': 'image/tiff',
    'ico': 'image/x-icon',
    // Video
    'mp4': 'video/mp4',
    'mpeg': 'video/mpeg',
    'mpg': 'video/mpeg',
    'webm': 'video/webm',
    'ogv': 'video/ogg',
    'mov': 'video/quicktime',
    'avi': 'video/x-msvideo',
    'mkv': 'video/x-matroska',
    'flv': 'video/x-flv',
    // Audio
    'mp3': 'audio/mpeg',
    'aac': 'audio/aac',
    'm4a': 'audio/x-m4a',
    'wav': 'audio/wav',
    'flac': 'audio/flac',
    'ogg': 'audio/ogg',
    'oga': 'audio/ogg',
    'opus': 'audio/opus',
    // Documents - Microsoft Office
    'doc': 'application/msword',
    'docx': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'xls': 'application/vnd.ms-excel',
    'xlsx': 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    'ppt': 'application/vnd.ms-powerpoint',
    'pptx': 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    // Documents - eBooks
    'epub': 'application/epub+zip',
    'azw': 'application/vnd.amazon.ebook',
    // Fonts
    'woff': 'font/woff',
    'woff2': 'font/woff2',
    'ttf': 'font/ttf',
    'otf': 'font/otf',
  }

  return ext ? extensionMap[ext] || null : null
})
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
