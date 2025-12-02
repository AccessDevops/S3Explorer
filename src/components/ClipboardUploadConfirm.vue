<template>
  <Dialog :open="show" @update:open="handleClose">
    <DialogContent class="sm:max-w-[500px]">
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2">
          <ClipboardIcon class="w-5 h-5" />
          {{ t('clipboardPasteUpload') }}
        </DialogTitle>
        <DialogDescription>
          {{ t('clipboardDestination') }}: <code class="px-1.5 py-0.5 bg-muted rounded text-xs font-mono">{{ currentBucket }}/{{ currentPrefix || '' }}</code>
        </DialogDescription>
      </DialogHeader>

      <div class="py-2 max-h-[400px] overflow-y-auto">
        <div
          v-for="item in items"
          :key="item.id"
          class="flex flex-col gap-2 p-3 rounded-lg hover:bg-muted/50 transition-colors border-b border-border last:border-0"
        >
          <div class="flex items-center gap-3">
            <!-- Preview / Icon -->
            <div class="flex-shrink-0 w-12 h-12 rounded-md overflow-hidden bg-muted flex items-center justify-center">
              <!-- Image preview -->
              <img
                v-if="item.type === 'image' && item.preview"
                :src="item.preview"
                :alt="item.name"
                class="w-full h-full object-cover"
              />
              <!-- Text icon -->
              <FileTextIcon v-else-if="item.type === 'text'" class="w-6 h-6 text-muted-foreground" />
              <!-- File icon -->
              <FileIcon v-else class="w-6 h-6 text-muted-foreground" />
            </div>

            <!-- Info -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <input
                  v-model="item.name"
                  class="flex-1 text-sm font-medium bg-transparent border-b border-transparent hover:border-muted-foreground/30 focus:border-primary focus:outline-none"
                  :placeholder="t('clipboardFileName')"
                  @input="$emit('updateName', item.id, ($event.target as HTMLInputElement).value)"
                />
              </div>
              <div class="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
                <span class="capitalize">{{ getTypeLabel(item.type) }}</span>
                <span class="text-muted-foreground/50">|</span>
                <span>{{ formatSize(item.size) }}</span>
              </div>
            </div>

            <!-- Remove button -->
            <Button
              variant="ghost"
              size="icon"
              class="flex-shrink-0 h-8 w-8"
              :disabled="isUploading"
              @click="$emit('removeItem', item.id)"
            >
              <XIcon class="w-4 h-4" />
            </Button>
          </div>

          <!-- Text preview (for text type only) -->
          <div
            v-if="item.type === 'text' && item.preview"
            class="ml-15 text-xs text-muted-foreground bg-muted/50 rounded px-2 py-1.5 font-mono whitespace-pre-wrap break-all max-h-20 overflow-hidden"
          >
            {{ item.preview }}
          </div>
        </div>

        <div v-if="items.length === 0" class="text-center py-8 text-muted-foreground">
          {{ t('clipboardNoContent') }}
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" :disabled="isUploading" @click="$emit('cancel')">
          {{ t('cancel') }}
        </Button>
        <Button
          ref="confirmButtonRef"
          :disabled="items.length === 0 || isUploading"
          @click="$emit('confirm')"
        >
          <Loader2Icon v-if="isUploading" class="w-4 h-4 mr-2 animate-spin" />
          <UploadIcon v-else class="w-4 h-4 mr-2" />
          {{ isUploading ? t('uploading') : t('clipboardUploadCount', { count: items.length }) }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import { ref, onUnmounted, watch, nextTick } from 'vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  ClipboardIcon,
  FileTextIcon,
  FileIcon,
  XIcon,
  UploadIcon,
  Loader2Icon,
} from 'lucide-vue-next'
import type { ClipboardItem } from '@/composables/useClipboardUpload'
import { useSettingsStore } from '@/stores/settings'
import { translations } from '@/i18n/translations'

interface Props {
  show: boolean
  items: ClipboardItem[]
  currentBucket: string
  currentPrefix: string
  isUploading?: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  confirm: []
  cancel: []
  updateName: [itemId: string, newName: string]
  removeItem: [itemId: string]
}>()

// Ref to the confirm button for auto-focus
const confirmButtonRef = ref<InstanceType<typeof Button> | null>(null)

// Handle keyboard shortcuts
function handleKeyDown(event: KeyboardEvent) {
  if (!props.show) return

  // Escape or Delete to cancel
  if (event.key === 'Escape' || event.key === 'Delete') {
    if (!props.isUploading) {
      event.preventDefault()
      emit('cancel')
    }
    return
  }

  // Enter to confirm
  if (event.key === 'Enter' && !props.isUploading && props.items.length > 0) {
    // Don't trigger if focus is on input (allow normal input behavior)
    const target = event.target as HTMLElement
    if (target.tagName !== 'INPUT') {
      event.preventDefault()
      emit('confirm')
    }
  }
}

// Setup/cleanup keyboard listener when dialog opens/closes
watch(() => props.show, async (isOpen) => {
  if (isOpen) {
    window.addEventListener('keydown', handleKeyDown)
    // Focus the confirm button after dialog renders
    await nextTick()
    // Small delay to ensure dialog animation completes
    setTimeout(() => {
      const buttonEl = confirmButtonRef.value?.$el as HTMLElement | undefined
      buttonEl?.focus()
    }, 50)
  } else {
    window.removeEventListener('keydown', handleKeyDown)
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})

const settingsStore = useSettingsStore()

const t = (key: string, params?: Record<string, any>) => {
  const lang = settingsStore.language
  const value = (translations as any)[lang]?.[key] || (translations as any)['en']?.[key] || key

  if (params) {
    return value.replace(/\{(\w+)\}/g, (_: string, k: string) => params[k] ?? '')
  }
  return value
}

function getTypeLabel(type: 'image' | 'text' | 'file'): string {
  switch (type) {
    case 'image':
      return t('clipboardTypeImage')
    case 'text':
      return t('clipboardTypeText')
    case 'file':
      return t('clipboardTypeFile')
    default:
      return type
  }
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

function handleClose(open: boolean) {
  if (!open) {
    emit('cancel')
  }
}
</script>
