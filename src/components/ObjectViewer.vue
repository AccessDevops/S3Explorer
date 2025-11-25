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
          {{ formatSize(object.size) }} ({{ t('fileTooLargeLimit') }}: {{ settingsStore.previewMaxLimitMB }} MB)
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
    <div v-else-if="loading" class="flex items-center justify-center py-12">
      <div class="w-full max-w-md space-y-3">
        <p class="text-center text-muted-foreground font-medium">Loading file...</p>
        <div class="h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
          <div class="h-full bg-primary rounded-full animate-progress-indeterminate"></div>
        </div>
      </div>
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
        class="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/30 rounded-lg text-yellow-700 dark:text-yellow-400"
      >
        <div class="flex items-start gap-3">
          <div class="text-2xl flex-shrink-0">⚠️</div>
          <div class="flex-1">
            <p class="font-semibold">Content-Type Mismatch Detected</p>
            <p class="text-sm mt-1">{{ contentTypeMismatch.message }}</p>
          </div>
        </div>
        <div class="mt-3 flex justify-end">
          <button
            @click="fixContentTypeMismatch"
            :disabled="changingContentType"
            class="px-3 py-1.5 bg-yellow-600 hover:bg-yellow-700 disabled:bg-yellow-600/50 text-white rounded-md transition-colors text-sm font-medium flex items-center gap-2"
          >
            <svg
              v-if="changingContentType"
              class="animate-spin h-4 w-4"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <span v-if="changingContentType">Changing...</span>
            <span v-else>Change to {{ contentTypeMismatch.expectedType }}</span>
          </button>
        </div>
      </div>

      <!-- Text Viewer/Editor -->
      <div v-if="isText" class="flex flex-col gap-2">
        <!-- Editor Toolbar -->
        <div class="flex items-center justify-end gap-2 px-2">
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
          :theme="monacoTheme"
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

      <!-- Audio Player -->
      <div v-else-if="isAudio" class="flex flex-col items-center justify-center py-12 space-y-6">
        <div class="text-6xl">🎵</div>
        <div class="text-center space-y-2">
          <p class="font-semibold text-lg">{{ object.key.split('/').pop() }}</p>
          <p class="text-sm text-muted-foreground">{{ formatSize(object.size) }}</p>
        </div>
        <audio
          v-if="audioUrl"
          :src="audioUrl"
          controls
          class="w-full max-w-2xl rounded-lg shadow-lg"
          preload="metadata"
        >
          <div class="text-center py-6 text-muted-foreground">
            <p>Your browser doesn't support audio playback.</p>
            <p class="text-sm mt-2">Please use the Download button to play this file.</p>
          </div>
        </audio>
      </div>

      <!-- Video Player -->
      <div v-else-if="isVideo" class="flex flex-col items-center justify-center py-12 space-y-6">
        <div class="text-6xl">🎬</div>
        <div class="text-center space-y-2">
          <p class="font-semibold text-lg">{{ object.key.split('/').pop() }}</p>
          <p class="text-sm text-muted-foreground">{{ formatSize(object.size) }}</p>
        </div>
        <video
          v-if="videoUrl"
          :src="videoUrl"
          controls
          class="w-full max-w-2xl max-h-[500px] rounded-lg shadow-lg object-contain"
          preload="metadata"
        >
          <div class="text-center py-6 text-muted-foreground">
            <p>Your browser doesn't support video playback.</p>
            <p class="text-sm mt-2">Please use the Download button to view this file.</p>
          </div>
        </video>
      </div>

      <!-- Binary File -->
      <div v-else class="text-center py-12 text-muted-foreground space-y-4">
        <div class="text-5xl mb-4">📦</div>
        <p class="font-semibold">Binary file ({{ formatSize(object.size) }})</p>
        <p v-if="!suggestedContentType">Cannot preview this file type. Please download to view.</p>

        <!-- Option to try loading with suggested content type -->
        <div v-if="suggestedContentType" class="mt-6 p-4 border border-blue-500/30 bg-blue-500/10 rounded-lg inline-block">
          <p class="text-sm text-blue-700 dark:text-blue-400 mb-3">
            Try to load it as <strong>{{ suggestedContentType.label }}</strong>
          </p>
          <button
            @click="tryLoadWithSuggestedType"
            class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md transition-colors text-sm font-medium"
          >
            Load as {{ suggestedContentType.label }}
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
        <h3 class="font-semibold">{{ object.key }}</h3>
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
          :theme="monacoTheme"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useSettingsStore } from '../stores/settings'
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
const settingsStore = useSettingsStore()
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
const showFullscreenEditor = ref(false)
const changingContentType = ref(false)

// Get Monaco theme from settings store
const monacoTheme = computed(() => settingsStore.getMonacoTheme)

// File size limits from settings
const SIZE_WARNING_LIMIT = computed(() => settingsStore.previewWarningLimitMB * 1024 * 1024)
const SIZE_MAX_LIMIT = computed(() => settingsStore.previewMaxLimitMB * 1024 * 1024)

const isTooLarge = computed(() => props.object.size > SIZE_MAX_LIMIT.value)
const isLarge = computed(() => props.object.size > SIZE_WARNING_LIMIT.value && props.object.size <= SIZE_MAX_LIMIT.value)

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
    return { mismatch: false, message: '', expectedType: '' }
  }

  const filename = props.object.key.split('/').pop() || ''
  const expectedContentType = getExpectedContentType(filename)

  if (!expectedContentType) {
    return { mismatch: false, message: '', expectedType: '' }
  }

  // Normalize content types for comparison
  const normalizedActual = contentType.value.toLowerCase().split(';')[0].trim()
  const normalizedExpected = expectedContentType.toLowerCase()

  if (normalizedActual !== normalizedExpected) {
    return {
      mismatch: true,
      message: `Content-Type mismatch: File extension suggests "${normalizedExpected}" but actual type is "${normalizedActual}"`,
      expectedType: normalizedExpected,
    }
  }

  return { mismatch: false, message: '', expectedType: '' }
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

const isAudio = computed(() => {
  // Check by content type
  if (contentType.value?.startsWith('audio/')) {
    return true
  }

  // Fallback: check by file extension
  const key = props.object.key.toLowerCase()
  const audioExtensions = ['.mp3', '.wav', '.ogg', '.flac', '.aac', '.m4a', '.wma', '.opus']
  return audioExtensions.some((ext) => key.endsWith(ext))
})

const isVideo = computed(() => {
  // Check by content type
  if (contentType.value?.startsWith('video/')) {
    return true
  }

  // Fallback: check by file extension
  const key = props.object.key.toLowerCase()
  const videoExtensions = ['.mp4', '.avi', '.mov', '.wmv', '.flv', '.webm', '.mkv', '.m4v', '.mpeg', '.mpg']
  return videoExtensions.some((ext) => key.endsWith(ext))
})

const textContent = computed(() => {
  if (!content.value || !isText.value) return ''
  const decoder = new TextDecoder()
  return decoder.decode(content.value)
})

// Suggested content type based on file extension for binary files
const suggestedContentType = computed(() => {
  // Only suggest if current detection shows it as binary (not text, image, pdf, audio, video)
  if (isText.value || isImage.value || isPdf.value || isAudio.value || isVideo.value) {
    return null
  }

  const filename = props.object.key.split('/').pop() || ''
  const expectedType = getExpectedContentType(filename)

  if (!expectedType) {
    return null
  }

  // Determine the label based on the content type
  let label = 'Unknown'
  if (expectedType.startsWith('image/')) {
    label = 'Image'
  } else if (expectedType.startsWith('video/')) {
    label = 'Video'
  } else if (expectedType.startsWith('audio/')) {
    label = 'Audio'
  } else if (expectedType === 'application/pdf') {
    label = 'PDF'
  } else if (expectedType.startsWith('text/') ||
             expectedType === 'application/json' ||
             expectedType === 'application/xml' ||
             expectedType === 'application/javascript') {
    label = 'Text'
  } else {
    label = expectedType.split('/')[1] || 'File'
  }

  return {
    type: expectedType,
    label: label
  }
})

// Blob URLs stored as refs to allow proper cleanup
const imageUrl = ref<string>('')
const pdfUrl = ref<string>('')
const audioUrl = ref<string>('')
const videoUrl = ref<string>('')

// Function to revoke Blob URLs and free memory
function revokeObjectUrls() {
  if (imageUrl.value) {
    URL.revokeObjectURL(imageUrl.value)
    imageUrl.value = ''
  }
  if (pdfUrl.value) {
    URL.revokeObjectURL(pdfUrl.value)
    pdfUrl.value = ''
  }
  if (audioUrl.value) {
    URL.revokeObjectURL(audioUrl.value)
    audioUrl.value = ''
  }
  if (videoUrl.value) {
    URL.revokeObjectURL(videoUrl.value)
    videoUrl.value = ''
  }
}

// Watch for content changes and create/revoke Blob URLs accordingly
watch([content, contentType, isImage, isPdf, isAudio, isVideo], () => {
  // Revoke old URLs first to free memory
  revokeObjectUrls()

  // Create new Blob URLs if needed
  if (content.value && isImage.value) {
    const blob = new Blob([content.value as BlobPart], { type: contentType.value || undefined })
    imageUrl.value = URL.createObjectURL(blob)
  }

  if (content.value && isPdf.value) {
    const blob = new Blob([content.value as BlobPart], { type: 'application/pdf' })
    pdfUrl.value = URL.createObjectURL(blob)
  }

  if (content.value && isAudio.value) {
    const blob = new Blob([content.value as BlobPart], { type: contentType.value || 'audio/mpeg' })
    audioUrl.value = URL.createObjectURL(blob)
  }

  if (content.value && isVideo.value) {
    const blob = new Blob([content.value as BlobPart], { type: contentType.value || 'video/mp4' })
    videoUrl.value = URL.createObjectURL(blob)
  }
}, { immediate: true })

// Cleanup on component unmount to prevent memory leaks
onUnmounted(() => {
  revokeObjectUrls()
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

// Try loading the file with the suggested content type based on extension
function tryLoadWithSuggestedType() {
  if (!suggestedContentType.value) {
    return
  }

  // Override the content type locally to trigger the appropriate viewer
  contentType.value = suggestedContentType.value.type

  // The watch on [content, contentType, isImage, isPdf, isAudio, isVideo] will automatically
  // recreate the Blob URLs with the new content type, triggering the appropriate viewer
}

// Change content-type in S3 to the expected type based on file extension
async function fixContentTypeMismatch() {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    toast.error('No profile or bucket selected')
    return
  }

  const filename = props.object.key.split('/').pop() || ''
  const expectedType = getExpectedContentType(filename)

  if (!expectedType) {
    toast.error('Cannot determine expected content type')
    return
  }

  try {
    changingContentType.value = true

    await changeContentType(
      appStore.currentProfile.id,
      appStore.currentBucket,
      props.object.key,
      expectedType
    )

    toast.success(`Content-Type changed to ${expectedType}`)

    // Reload the object to show it with the correct content type
    await loadContent()
  } catch (e) {
    toast.error(`Failed to change content type: ${e}`)
  } finally {
    changingContentType.value = false
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

  // Auto-load for files <= warning limit (default 10MB)
  await loadContent()
})
</script>
