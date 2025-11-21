<template>
  <div class="min-h-[400px] max-h-[600px] overflow-auto">
    <!-- File Too Large (>900MB) -->
    <div
      v-if="isTooLarge"
      class="flex flex-col items-center justify-center py-12 text-center space-y-4"
    >
      <div class="text-6xl">⚠️</div>
      <div>
        <p class="font-semibold text-lg">{{ t('fileTooLarge') }}</p>
        <p class="text-muted-foreground">
          {{ formatSize(object.size) }} ({{ t('fileTooLargeLimit') }}: 900 MB)
        </p>
        <p class="text-sm text-muted-foreground mt-2">{{ t('pleaseDownload') }}</p>
      </div>
    </div>

    <!-- File Large (>100MB) - Not loaded yet -->
    <div
      v-else-if="isLarge && !forceLoad"
      class="flex flex-col items-center justify-center py-12 text-center space-y-4"
    >
      <div class="text-6xl">📦</div>
      <div>
        <p class="font-semibold text-lg">{{ t('largeFileDetected') }}</p>
        <p class="text-muted-foreground">{{ formatSize(object.size) }}</p>
        <p class="text-sm text-muted-foreground mt-2">{{ t('loadingMayTakeTime') }}</p>
      </div>
      <button
        @click="loadLargeFile"
        class="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
      >
        {{ t('loadFile') }}
      </button>
    </div>

    <!-- Loading State -->
    <div v-else-if="loading" class="flex flex-col items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mb-4"></div>
      <p class="text-muted-foreground">Loading...</p>
    </div>

    <!-- Error State -->
    <div
      v-else-if="error"
      class="p-6 bg-destructive/10 border border-destructive/30 rounded-lg text-destructive"
    >
      {{ error }}
    </div>

    <!-- Content Display Area -->
    <div v-else>
      <!-- Content-Type Mismatch Warning -->
      <div
        v-if="contentTypeMismatch.mismatch"
        class="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/30 rounded-lg text-yellow-700 dark:text-yellow-400 flex items-start gap-3"
      >
        <div class="text-2xl flex-shrink-0">⚠️</div>
        <div class="flex-1">
          <p class="font-semibold">Content-Type Mismatch Detected</p>
          <p class="text-sm mt-1">{{ contentTypeMismatch.message }}</p>
        </div>
      </div>

      <!-- Text Viewer/Editor -->
      <div v-if="isText" class="flex flex-col gap-2">
        <!-- Editor Toolbar -->
        <div class="flex items-center justify-between gap-2 px-2">
          <div class="flex items-center gap-2">
            <span class="text-sm text-muted-foreground">{{ t('theme') }}:</span>
            <select
              v-model="editorTheme"
              class="px-2 py-1 text-sm border rounded-md bg-background"
            >
              <option value="vs-dark">{{ t('themeDark') }}</option>
              <option value="vs">{{ t('themeLight') }}</option>
              <option value="hc-black">{{ t('themeHighContrast') }}</option>
            </select>
          </div>
          <button
            @click="showFullscreenEditor = true"
            class="px-3 py-1 text-sm border rounded-md hover:bg-accent transition-colors flex items-center gap-2"
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
              <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
            </svg>
            {{ t('fullscreen') }}
          </button>
        </div>

        <!-- Monaco Editor (View or Edit Mode) -->
        <CodeEditor
          v-model="editedContent"
          :filename="object.key"
          :readonly="!isEditing"
          :theme="editorTheme"
        />
      </div>

      <!-- Image Viewer -->
      <div v-else-if="isImage" class="flex justify-center p-6">
        <img
          :src="imageUrl"
          :alt="object.key"
          class="max-w-full max-h-[500px] object-contain rounded-lg shadow-lg"
        />
      </div>

      <!-- PDF Viewer -->
      <div v-else-if="isPdf" class="h-full">
        <object :data="pdfUrl" type="application/pdf" class="w-full h-[600px] border rounded-lg">
          <div class="text-center py-12 text-muted-foreground space-y-2">
            <div class="text-5xl mb-4">📄</div>
            <p class="font-semibold">PDF file ({{ formatSize(object.size) }})</p>
            <p>Your browser doesn't support PDF preview.</p>
            <p>Please use the Download button to view this file.</p>
          </div>
        </object>
      </div>

      <!-- Binary File -->
      <div v-else class="text-center py-12 text-muted-foreground space-y-4">
        <div class="text-5xl mb-4">📦</div>
        <p class="font-semibold">Binary file ({{ formatSize(object.size) }})</p>
        <p>Cannot preview this file type. Please download to view.</p>

        <!-- Option to convert to text -->
        <div class="mt-6 p-4 border border-yellow-500/30 bg-yellow-500/10 rounded-lg inline-block">
          <p class="text-sm text-yellow-700 dark:text-yellow-400 mb-3">
            {{ t('tryViewAsText') }}
          </p>
          <button
            @click="showChangeContentTypeConfirm"
            class="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-md transition-colors text-sm font-medium"
          >
            {{ t('changeToTextType') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Fullscreen Editor Modal -->
    <div
      v-if="showFullscreenEditor && isText"
      class="fixed inset-0 z-50 bg-background flex flex-col"
    >
      <!-- Fullscreen Toolbar -->
      <div class="flex items-center justify-between gap-4 p-4 border-b bg-card">
        <div class="flex items-center gap-4">
          <h3 class="font-semibold">{{ object.key }}</h3>
          <div class="flex items-center gap-2">
            <span class="text-sm text-muted-foreground">{{ t('theme') }}:</span>
            <select
              v-model="editorTheme"
              class="px-2 py-1 text-sm border rounded-md bg-background"
            >
              <option value="vs-dark">{{ t('themeDark') }}</option>
              <option value="vs">{{ t('themeLight') }}</option>
              <option value="hc-black">{{ t('themeHighContrast') }}</option>
            </select>
          </div>
        </div>
        <button
          @click="showFullscreenEditor = false"
          class="px-3 py-1.5 text-sm border rounded-md hover:bg-accent transition-colors flex items-center gap-2"
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
            <path d="M8 3v3a2 2 0 0 1-2 2H3m18 0h-3a2 2 0 0 1-2-2V3m0 18v-3a2 2 0 0 1 2-2h3M3 16h3a2 2 0 0 1 2 2v3" />
          </svg>
          {{ t('exitFullscreen') }}
        </button>
      </div>

      <!-- Fullscreen Editor -->
      <div class="flex-1 overflow-hidden">
        <CodeEditor
          v-model="editedContent"
          :filename="object.key"
          :readonly="!isEditing"
          :theme="editorTheme"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useToast } from '../composables/useToast'
import { useI18n } from '../composables/useI18n'
import { getObject, putObject, changeContentType } from '../services/tauri'
import { formatSize } from '../utils/formatters'
import type { S3Object } from '../types'
import CodeEditor from './CodeEditor.vue'

const props = defineProps<{
  object: S3Object
}>()

const emit = defineEmits<{
  saved: []
}>()

const appStore = useAppStore()
const toast = useToast()
const { t } = useI18n()
const loading = ref(true)
const error = ref<string | null>(null)
const content = ref<Uint8Array | null>(null)
const contentType = ref<string | null>(null)
const isEditing = ref(false)
const editedContent = ref('')
const saving = ref(false)
const forceLoad = ref(false)
const editorTheme = ref<'vs' | 'vs-dark' | 'hc-black'>('vs-dark')
const showFullscreenEditor = ref(false)

// File size limits
const SIZE_100MB = 100 * 1024 * 1024 // 104857600 bytes
const SIZE_900MB = 900 * 1024 * 1024 // 943718400 bytes

const isTooLarge = computed(() => props.object.size > SIZE_900MB)
const isLarge = computed(() => props.object.size > SIZE_100MB && props.object.size <= SIZE_900MB)

// Utility functions for content-type detection
function getFileExtension(filename: string): string {
  const parts = filename.split('.')
  return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : ''
}

function getExpectedContentType(filename: string): string | null {
  const ext = getFileExtension(filename)

  const contentTypeMap: Record<string, string> = {
    // Images
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    gif: 'image/gif',
    bmp: 'image/bmp',
    webp: 'image/webp',
    svg: 'image/svg+xml',
    ico: 'image/x-icon',

    // Videos
    mp4: 'video/mp4',
    avi: 'video/x-msvideo',
    mov: 'video/quicktime',
    wmv: 'video/x-ms-wmv',
    flv: 'video/x-flv',
    webm: 'video/webm',
    mkv: 'video/x-matroska',

    // Audio
    mp3: 'audio/mpeg',
    wav: 'audio/wav',
    ogg: 'audio/ogg',
    flac: 'audio/flac',
    aac: 'audio/aac',
    m4a: 'audio/mp4',

    // Documents
    pdf: 'application/pdf',
    doc: 'application/msword',
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    xls: 'application/vnd.ms-excel',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    ppt: 'application/vnd.ms-powerpoint',
    pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',

    // Text
    txt: 'text/plain',
    md: 'text/markdown',
    html: 'text/html',
    css: 'text/css',
    js: 'text/javascript',
    json: 'application/json',
    xml: 'application/xml',

    // Archives
    zip: 'application/zip',
    rar: 'application/x-rar-compressed',
    '7z': 'application/x-7z-compressed',
    tar: 'application/x-tar',
    gz: 'application/gzip',
  }

  return contentTypeMap[ext] || null
}

const contentTypeMismatch = computed(() => {
  if (!contentType.value) {
    return { mismatch: false, message: '' }
  }

  const filename = props.object.key.split('/').pop() || ''
  const expectedContentType = getExpectedContentType(filename)

  if (!expectedContentType) {
    return { mismatch: false, message: '' }
  }

  // Normalize content types for comparison
  const normalizedActual = contentType.value.toLowerCase().split(';')[0].trim()
  const normalizedExpected = expectedContentType.toLowerCase()

  if (normalizedActual !== normalizedExpected) {
    return {
      mismatch: true,
      message: `Content-Type mismatch: File extension suggests "${normalizedExpected}" but actual type is "${normalizedActual}"`,
    }
  }

  return { mismatch: false, message: '' }
})

const isText = computed(() => {
  // Check by content type
  if (contentType.value) {
    if (
      contentType.value.startsWith('text/') ||
      contentType.value === 'application/json' ||
      contentType.value === 'application/xml' ||
      contentType.value === 'application/javascript'
    ) {
      return true
    }
  }

  // Fallback: check by file extension
  const key = props.object.key.toLowerCase()
  const textExtensions = [
    '.txt',
    '.md',
    '.markdown',
    '.json',
    '.xml',
    '.yaml',
    '.yml',
    '.js',
    '.ts',
    '.jsx',
    '.tsx',
    '.css',
    '.scss',
    '.html',
    '.htm',
    '.csv',
    '.log',
    '.sh',
    '.bash',
    '.py',
    '.java',
    '.c',
    '.cpp',
    '.h',
    '.rs',
    '.go',
    '.php',
    '.rb',
    '.sql',
    '.env',
    '.gitignore',
    '.dockerfile',
    '.config',
    '.conf',
    '.ini',
    '.toml',
  ]

  return textExtensions.some((ext) => key.endsWith(ext))
})

const isImage = computed(() => {
  if (!contentType.value) return false
  return contentType.value.startsWith('image/')
})

const isPdf = computed(() => {
  // Check by content type
  if (contentType.value === 'application/pdf') {
    return true
  }

  // Fallback: check by file extension
  const key = props.object.key.toLowerCase()
  return key.endsWith('.pdf')
})

const textContent = computed(() => {
  if (!content.value || !isText.value) return ''
  const decoder = new TextDecoder()
  return decoder.decode(content.value)
})

const imageUrl = computed(() => {
  if (!content.value || !isImage.value) return ''
  const blob = new Blob([content.value], { type: contentType.value || undefined })
  return URL.createObjectURL(blob)
})

const pdfUrl = computed(() => {
  if (!content.value || !isPdf.value) return ''
  const blob = new Blob([content.value], { type: 'application/pdf' })
  return URL.createObjectURL(blob)
})

function startEditing() {
  editedContent.value = textContent.value
  isEditing.value = true
}

function cancelEditing() {
  isEditing.value = false
  editedContent.value = textContent.value
}

// Keep editedContent in sync with textContent
watch(textContent, (newContent) => {
  if (!isEditing.value) {
    editedContent.value = newContent
  }
}, { immediate: true })

async function saveChanges() {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    toast.error('No profile or bucket selected')
    return
  }

  try {
    saving.value = true
    const encoder = new TextEncoder()
    const bytes = Array.from(encoder.encode(editedContent.value))

    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      props.object.key,
      bytes,
      contentType.value || undefined
    )

    // Update local content
    content.value = new Uint8Array(bytes)
    isEditing.value = false
    editedContent.value = ''

    emit('saved')
    toast.success('File saved successfully!')
  } catch (e) {
    toast.error(`Failed to save: ${e}`)
  } finally {
    saving.value = false
  }
}

// Expose methods and state to parent component
defineExpose({
  isText,
  isEditing,
  saving,
  contentType,
  startEditing,
  saveChanges,
  cancelEditing,
})

async function loadContent() {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    error.value = 'No profile or bucket selected'
    loading.value = false
    return
  }

  try {
    loading.value = true
    const response = await getObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      props.object.key
    )
    content.value = new Uint8Array(response.content)
    contentType.value = response.content_type || null
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

function loadLargeFile() {
  forceLoad.value = true
  loadContent()
}

// Change content-type to text/plain
async function showChangeContentTypeConfirm() {
  const fileName = props.object.key.split('/').pop() || props.object.key

  // Use browser's confirm dialog with warning
  const confirmed = confirm(
    t('changeContentTypeWarning', fileName)
  )

  if (!confirmed) return

  if (!appStore.currentProfile || !appStore.currentBucket) {
    toast.error('No profile or bucket selected')
    return
  }

  try {
    loading.value = true

    await changeContentType(
      appStore.currentProfile.id,
      appStore.currentBucket,
      props.object.key,
      'text/plain'
    )

    toast.success(t('contentTypeChangeSuccess'))

    // Reload the object to show it as text
    await loadContent()
  } catch (e) {
    toast.error(`${t('contentTypeChangeFailed')}: ${e}`)
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  // Don't auto-load if file is too large (>900MB)
  if (isTooLarge.value) {
    loading.value = false
    return
  }

  // Don't auto-load if file is large (>100MB) unless forced
  if (isLarge.value && !forceLoad.value) {
    loading.value = false
    return
  }

  // Auto-load for files <= 100MB
  await loadContent()
})
</script>
