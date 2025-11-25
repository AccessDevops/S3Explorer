<template>
  <div class="image-editor-container">
    <!-- Editor Container -->
    <div ref="editorContainer" class="tui-image-editor"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import ImageEditor from 'tui-image-editor'
import { useI18n } from '../composables/useI18n'

const props = defineProps<{
  imageUrl: string
  imageName?: string
  theme?: 'dark' | 'light'
}>()

const emit = defineEmits<{
  loaded: []
  error: [error: string]
  modified: []
}>()

const { t: _t, language } = useI18n()
const editorContainer = ref<HTMLElement | null>(null)
let editorInstance: ImageEditor | null = null

// Dark theme configuration
const darkTheme = {
  'common.bi.image': '', // No default watermark
  'common.bisize.width': '0px',
  'common.bisize.height': '0px',
  'common.backgroundColor': '#1e1e1e',

  // Header
  'header.backgroundImage': 'none',
  'header.backgroundColor': '#1e1e1e',
  'header.border': '0px',

  // Main
  'loadButton.backgroundColor': '#fff',
  'loadButton.border': '1px solid #ddd',
  'loadButton.color': '#222',
  'loadButton.fontFamily': 'NotoSans, sans-serif',
  'loadButton.fontSize': '12px',
  'downloadButton.backgroundColor': '#4169e1',
  'downloadButton.border': '1px solid #4169e1',
  'downloadButton.color': '#fff',
  'downloadButton.fontFamily': 'NotoSans, sans-serif',
  'downloadButton.fontSize': '12px',

  // Submenu
  'submenu.backgroundColor': '#2a2a2a',
  'submenu.partition.color': '#3c3c3c',
  'submenu.normalLabel.color': '#c9c9c9',
  'submenu.normalLabel.fontWeight': 'lighter',
  'submenu.activeLabel.color': '#fff',
  'submenu.activeLabel.fontWeight': 'lighter',
  'submenu.iconSize.width': '24px',
  'submenu.iconSize.height': '24px',

  // Checkbox
  'checkbox.border': '1px solid #ccc',
  'checkbox.backgroundColor': '#fff',

  // Range slider
  'range.pointer.color': '#4169e1',
  'range.bar.color': '#666',
  'range.subbar.color': '#4169e1',
  'range.value.color': '#fff',
  'range.value.fontWeight': 'lighter',
  'range.value.fontSize': '11px',
  'range.value.border': '1px solid #353535',
  'range.value.backgroundColor': '#151515',
  'range.title.color': '#fff',
  'range.title.fontWeight': 'lighter',

  // Colorpicker
  'colorpicker.button.border': '1px solid #1e1e1e',
  'colorpicker.title.color': '#fff',
}

// Light theme configuration
const lightTheme = {
  'common.bi.image': '', // No default watermark
  'common.bisize.width': '0px',
  'common.bisize.height': '0px',
  'common.backgroundColor': '#f5f5f5',

  // Header
  'header.backgroundImage': 'none',
  'header.backgroundColor': '#ffffff',
  'header.border': '1px solid #e0e0e0',

  // Main
  'loadButton.backgroundColor': '#fff',
  'loadButton.border': '1px solid #ddd',
  'loadButton.color': '#222',
  'loadButton.fontFamily': 'NotoSans, sans-serif',
  'loadButton.fontSize': '12px',
  'downloadButton.backgroundColor': '#4169e1',
  'downloadButton.border': '1px solid #4169e1',
  'downloadButton.color': '#fff',
  'downloadButton.fontFamily': 'NotoSans, sans-serif',
  'downloadButton.fontSize': '12px',

  // Submenu
  'submenu.backgroundColor': '#ffffff',
  'submenu.partition.color': '#e0e0e0',
  'submenu.normalLabel.color': '#666',
  'submenu.normalLabel.fontWeight': 'normal',
  'submenu.activeLabel.color': '#222',
  'submenu.activeLabel.fontWeight': 'normal',
  'submenu.iconSize.width': '24px',
  'submenu.iconSize.height': '24px',

  // Checkbox
  'checkbox.border': '1px solid #ccc',
  'checkbox.backgroundColor': '#fff',

  // Range slider
  'range.pointer.color': '#4169e1',
  'range.bar.color': '#ccc',
  'range.subbar.color': '#4169e1',
  'range.value.color': '#222',
  'range.value.fontWeight': 'normal',
  'range.value.fontSize': '11px',
  'range.value.border': '1px solid #d5d5d5',
  'range.value.backgroundColor': '#fff',
  'range.title.color': '#222',
  'range.title.fontWeight': 'normal',

  // Colorpicker
  'colorpicker.button.border': '1px solid #d5d5d5',
  'colorpicker.title.color': '#222',
}

// Get the appropriate theme based on prop (default to dark)
const editorTheme = computed(() => {
  return props.theme === 'light' ? lightTheme : darkTheme
})

// i18n locale for Tui.Image-Editor (mapping our languages to TUI locales)
function _getTuiLocale() {
  const langMap: Record<string, string> = {
    en: 'en-US',
    fr: 'fr-FR',
    es: 'es-ES',
    de: 'de-DE',
    it: 'it-IT',
    pt: 'pt-PT',
    zh: 'zh-CN',
    ja: 'ja-JP',
    ko: 'ko-KR',
    ru: 'ru-RU',
  }

  const currentLang = language.value?.toLowerCase() || 'en'
  return langMap[currentLang] || 'en-US'
}

// Initialize editor
function initializeEditor() {
  if (!editorContainer.value) return

  try {
    editorInstance = new ImageEditor(editorContainer.value, {
      includeUI: {
        loadImage: {
          path: props.imageUrl,
          name: props.imageName || 'image',
        },
        theme: editorTheme.value,
        menu: ['crop', 'flip', 'rotate', 'draw', 'shape', 'icon', 'text', 'mask', 'filter'],
        initMenu: 'filter',
        uiSize: {
          width: '100%',
          height: '600px',
        },
        menuBarPosition: 'bottom',
      },
      cssMaxWidth: 1000,
      cssMaxHeight: 600,
      usageStatistics: false, // Disable analytics
    })

    // Wait for image to load
    editorInstance.on('load', () => {
      emit('loaded')
    })

    // Handle errors
    editorInstance.on('error', (error: any) => {
      console.error('Image editor error:', error)
      emit('error', error.message || 'Failed to load image')
    })

    // Listen for any changes to the image (undo stack changes indicate modifications)
    editorInstance.on('undoStackChanged', () => {
      emit('modified')
    })

    // Listen for object modifications (shapes, text, etc.)
    editorInstance.on('objectActivated', () => {
      emit('modified')
    })

    editorInstance.on('objectMoved', () => {
      emit('modified')
    })

    editorInstance.on('objectScaled', () => {
      emit('modified')
    })

    editorInstance.on('objectRotated', () => {
      emit('modified')
    })
  } catch (error: any) {
    console.error('Failed to initialize image editor:', error)
    emit('error', error.message || 'Failed to initialize editor')
  }
}

// Get edited image as Blob
async function getEditedImage(format: 'png' | 'jpeg' = 'png', quality = 0.92): Promise<Blob | null> {
  if (!editorInstance) {
    console.error('Editor not initialized')
    return null
  }

  try {
    const dataUrl = editorInstance.toDataURL({ format, quality })

    // Convert data URL to Blob
    const response = await fetch(dataUrl)
    const blob = await response.blob()

    return blob
  } catch (error) {
    console.error('Failed to get edited image:', error)
    return null
  }
}

// Get edited image as Uint8Array for S3 upload
async function getEditedImageBytes(format: 'png' | 'jpeg' = 'png', quality = 0.92): Promise<Uint8Array | null> {
  const blob = await getEditedImage(format, quality)
  if (!blob) return null

  const arrayBuffer = await blob.arrayBuffer()
  return new Uint8Array(arrayBuffer)
}

// Destroy editor instance
function destroyEditor() {
  if (editorInstance) {
    try {
      editorInstance.destroy()
      editorInstance = null

      // Clean up any body modifications that tui-image-editor might have made
      // Remove any classes that might have been added
      document.body.classList.remove('tui-image-editor-container')
      document.body.style.overflow = ''
      document.body.style.height = ''
      document.body.style.margin = ''
      document.body.style.padding = ''

      // Also clean html element
      document.documentElement.style.overflow = ''
      document.documentElement.style.height = ''
    } catch (error) {
      console.error('Error destroying editor:', error)
    }
  }
}

// Watch for image URL changes and reload
watch(() => props.imageUrl, (newUrl, oldUrl) => {
  if (newUrl !== oldUrl && editorInstance) {
    editorInstance.loadImageFromURL(newUrl, props.imageName || 'image').then(() => {
      emit('loaded')
    }).catch((error: any) => {
      emit('error', error.message || 'Failed to load new image')
    })
  }
})

// Expose methods to parent component
defineExpose({
  getEditedImage,
  getEditedImageBytes,
  editorInstance: () => editorInstance,
})

onMounted(() => {
  // Small delay to ensure DOM is ready
  setTimeout(() => {
    initializeEditor()
  }, 100)
})

onBeforeUnmount(() => {
  destroyEditor()
})
</script>

<style scoped>
.image-editor-container {
  width: 100%;
  height: 100%;
  min-height: 600px;
}

/* Ensure the editor is responsive */
:deep(.tui-image-editor-canvas-container > canvas) {
  max-width: 100% !important;
  height: auto !important;
}

/* Hide the Load and Download buttons (not needed since we load from S3 and save directly) */
:deep(.tui-image-editor-load-btn),
:deep(.tui-image-editor-download-btn) {
  display: none !important;
}

/* Hide the div parents containing "Load" and "Download" text */
:deep(.tui-image-editor-header-buttons div:has(.tui-image-editor-load-btn)),
:deep(.tui-image-editor-controls-buttons div:has(.tui-image-editor-load-btn)),
:deep(.tui-image-editor-header-buttons div:has(.tui-image-editor-download-btn)),
:deep(.tui-image-editor-controls-buttons div:has(.tui-image-editor-download-btn)) {
  display: none !important;
}

/* Alternative: hide all buttons in header/controls (both Load and Download) */
:deep(.tui-image-editor-header-buttons),
:deep(.tui-image-editor-controls-buttons) {
  display: none !important;
}
</style>
