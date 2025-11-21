<template>
  <div class="min-h-[400px] max-h-[600px] overflow-auto">
    <!-- Loading State -->
    <div v-if="loading" class="flex flex-col items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mb-4"></div>
      <p class="text-muted-foreground">Loading...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="p-6 bg-destructive/10 border border-destructive/30 rounded-lg text-destructive">
      {{ error }}
    </div>

    <!-- Text Viewer/Editor -->
    <div v-else-if="isText" class="flex flex-col">
      <!-- View Mode -->
      <pre
        v-if="!isEditing"
        class="bg-muted/50 p-4 rounded-lg overflow-x-auto font-mono text-sm leading-relaxed border"
      >{{ textContent }}</pre>

      <!-- Edit Mode -->
      <textarea
        v-else
        v-model="editedContent"
        spellcheck="false"
        class="w-full min-h-[400px] p-4 font-mono text-sm leading-relaxed border rounded-lg resize-y bg-background focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
      ></textarea>
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
      <object
        :data="pdfUrl"
        type="application/pdf"
        class="w-full h-[600px] border rounded-lg"
      >
        <div class="text-center py-12 text-muted-foreground space-y-2">
          <div class="text-5xl mb-4">📄</div>
          <p class="font-semibold">PDF file ({{ formatSize(object.size) }})</p>
          <p>Your browser doesn't support PDF preview.</p>
          <p>Please use the Download button to view this file.</p>
        </div>
      </object>
    </div>

    <!-- Binary File -->
    <div v-else class="text-center py-12 text-muted-foreground space-y-2">
      <div class="text-5xl mb-4">📦</div>
      <p class="font-semibold">Binary file ({{ formatSize(object.size) }})</p>
      <p>Cannot preview this file type. Please download to view.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '../stores/app'
import { useToast } from '../composables/useToast'
import { getObject, putObject } from '../services/tauri'
import type { S3Object } from '../types'

const props = defineProps<{
  object: S3Object
}>()

const emit = defineEmits<{
  saved: []
}>()

const appStore = useAppStore()
const toast = useToast()
const loading = ref(true)
const error = ref<string | null>(null)
const content = ref<Uint8Array | null>(null)
const contentType = ref<string | null>(null)
const isEditing = ref(false)
const editedContent = ref('')
const saving = ref(false)

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
    '.txt', '.md', '.markdown', '.json', '.xml', '.yaml', '.yml',
    '.js', '.ts', '.jsx', '.tsx', '.css', '.scss', '.html', '.htm',
    '.csv', '.log', '.sh', '.bash', '.py', '.java', '.c', '.cpp',
    '.h', '.rs', '.go', '.php', '.rb', '.sql', '.env', '.gitignore',
    '.dockerfile', '.config', '.conf', '.ini', '.toml'
  ]

  return textExtensions.some(ext => key.endsWith(ext))
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

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function startEditing() {
  editedContent.value = textContent.value
  isEditing.value = true
}

function cancelEditing() {
  isEditing.value = false
  editedContent.value = ''
}

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
  cancelEditing
})

onMounted(async () => {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    error.value = 'No profile or bucket selected'
    loading.value = false
    return
  }

  try {
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
})
</script>
