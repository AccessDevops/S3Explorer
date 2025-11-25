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
let resizeObserver: ResizeObserver | null = null
let resizeTimeout: ReturnType<typeof setTimeout> | null = null

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

// Calculate max dimensions based on available container space
function calculateDimensions(): { cssMaxWidth: number; cssMaxHeight: number } {
  const parent = editorContainer.value?.parentElement
  const availableWidth = parent?.clientWidth || window.innerWidth
  const availableHeight = parent?.clientHeight || window.innerHeight

  // Reserve space for toolbar (~120px) and menu bar (~50px)
  return {
    cssMaxWidth: Math.max(availableWidth - 40, 400),
    cssMaxHeight: Math.max(availableHeight - 170, 300),
  }
}

// Store current dimensions to detect significant changes
let currentDimensions = { width: 0, height: 0 }

// Handle resize with debounce
function handleResize() {
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
  }

  resizeTimeout = setTimeout(() => {
    const { cssMaxWidth, cssMaxHeight } = calculateDimensions()

    // Only reinitialize if dimensions changed significantly (>50px)
    const widthDiff = Math.abs(cssMaxWidth - currentDimensions.width)
    const heightDiff = Math.abs(cssMaxHeight - currentDimensions.height)

    if (widthDiff > 50 || heightDiff > 50) {
      currentDimensions = { width: cssMaxWidth, height: cssMaxHeight }

      // Store current image data before reinitializing
      if (editorInstance) {
        const imageData = editorInstance.toDataURL()
        destroyEditor()

        // Reinitialize with new dimensions and reload image
        initializeEditorWithImage(imageData)
      }
    }
  }, 300) // 300ms debounce
}

// Initialize editor with optional image data URL
function initializeEditorWithImage(imageDataUrl?: string) {
  if (!editorContainer.value) return

  const { cssMaxWidth, cssMaxHeight } = calculateDimensions()
  currentDimensions = { width: cssMaxWidth, height: cssMaxHeight }

  try {
    editorInstance = new ImageEditor(editorContainer.value, {
      includeUI: {
        loadImage: {
          path: imageDataUrl || props.imageUrl,
          name: props.imageName || 'image',
        },
        theme: editorTheme.value,
        menu: ['crop', 'flip', 'rotate', 'draw', 'shape', 'icon', 'text', 'mask', 'filter'],
        initMenu: 'filter',
        uiSize: {
          width: '100%',
          height: '100%',
        },
        menuBarPosition: 'bottom',
      },
      cssMaxWidth,
      cssMaxHeight,
      usageStatistics: false,
    })

    editorInstance.on('load', () => {
      emit('loaded')
    })

    editorInstance.on('error', (error: any) => {
      console.error('Image editor error:', error)
      emit('error', error.message || 'Failed to load image')
    })

    editorInstance.on('undoStackChanged', () => {
      emit('modified')
    })

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

// Initialize editor
function initializeEditor() {
  if (!editorContainer.value) return

  const { cssMaxWidth, cssMaxHeight } = calculateDimensions()
  currentDimensions = { width: cssMaxWidth, height: cssMaxHeight }

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
          height: '100%',
        },
        menuBarPosition: 'bottom',
      },
      cssMaxWidth,
      cssMaxHeight,
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

    // Setup ResizeObserver to handle window/container resizing
    if (editorContainer.value?.parentElement) {
      resizeObserver = new ResizeObserver(() => {
        handleResize()
      })
      resizeObserver.observe(editorContainer.value.parentElement)
    }
  }, 100)
})

onBeforeUnmount(() => {
  // Cleanup ResizeObserver
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }

  // Cleanup resize timeout
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
    resizeTimeout = null
  }

  destroyEditor()
})
</script>

<style scoped>
.image-editor-container {
  width: 100%;
  height: 100%;
  overflow: visible;
}

/* Force the tui-image-editor to take full height - override inline style */
:deep(.tui-image-editor-container) {
  height: 100% !important;
}

/* Ensure the editor is responsive */
:deep(.tui-image-editor-canvas-container > canvas) {
  max-width: 100% !important;
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

/* Fix color picker visibility - ensure it's not clipped by parent overflow */
:deep(.tui-image-editor-submenu) {
  overflow: visible !important;
}

:deep(.tui-image-editor-menu) {
  overflow: visible !important;
}

/* Ensure color picker popup is visible and has high z-index */
:deep(.color-picker-control),
:deep(.tui-colorpicker-container) {
  z-index: 10000 !important;
}

/* Fix the submenu container to allow color picker to overflow */
:deep(.tui-image-editor-submenu-item) {
  overflow: visible !important;
}

/* Ensure the main container doesn't clip the color picker */
:deep(.tui-image-editor-main-container) {
  overflow: visible !important;
}

/* Fix color picker position if needed */
:deep(.color-picker-control .triangle) {
  z-index: 10001 !important;
}

/* Fix filter color items - the color picker is positioned with negative top */
:deep(.filter-color-item) {
  overflow: visible !important;
}

:deep(.tui-image-editor-button) {
  overflow: visible !important;
}

/* Fix the submenu content area that contains the filter options */
:deep(.tui-image-editor-submenu-style) {
  overflow: visible !important;
}

/* Fix the range wrapper and filter options containers */
:deep(.tui-image-editor-range-wrap),
:deep(.tui-image-editor-filter-options) {
  overflow: visible !important;
}

/* Ensure all parent elements of color picker allow overflow */
:deep(.tie-filter-tint-color),
:deep(.tie-filter-multiply-color),
:deep(.tie-filter-blend-color) {
  overflow: visible !important;
  position: relative;
}

/* Make sure the colorpicker palette is fully visible */
:deep(.tui-colorpicker-palette-container) {
  overflow: visible !important;
}

:deep(.tui-colorpicker-clearfix) {
  overflow: visible !important;
}

/* Fix the main wrap and container elements */
:deep(.tui-image-editor-wrap) {
  overflow: visible !important;
}

:deep(.tui-image-editor-main) {
  overflow: visible !important;
}

:deep(.tui-image-editor-controls) {
  overflow: visible !important;
}

/* Ensure the sub-menu element itself allows overflow */
:deep(.tui-image-editor-submenu > div) {
  overflow: visible !important;
}

/* Fix any potential clipping from the header area */
:deep(.tui-image-editor-header) {
  overflow: visible !important;
}

/* Fix color palette buttons being crushed - ensure proper dimensions */
:deep(.tui-colorpicker-palette-button) {
  width: 16px !important;
  height: 16px !important;
  min-width: 16px !important;
  min-height: 16px !important;
  display: inline-block !important;
  margin: 2px !important;
  padding: 0 !important;
  border: 1px solid #ccc !important;
  border-radius: 2px !important;
  cursor: pointer !important;
}

:deep(.tui-colorpicker-clearfix li) {
  display: inline-block !important;
  height: auto !important;
  min-height: 20px !important;
}

:deep(.tui-colorpicker-palette-container ul) {
  display: flex !important;
  flex-wrap: wrap !important;
  padding: 5px !important;
  margin: 0 !important;
  list-style: none !important;
}
</style>
