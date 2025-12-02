import { ref, computed } from 'vue'
import { useAppStore } from '../stores/app'
import { readClipboardFiles, getFileSize } from '../services/tauri'
import { useRustUploadManager } from './useRustUploadManager'

/**
 * Represents an item from the clipboard ready to be uploaded
 */
export interface ClipboardItem {
  id: string
  type: 'image' | 'text' | 'file'
  name: string
  data: Uint8Array | null // For image/text (in-memory data)
  path: string | null // For files from OS (file path)
  contentType: string
  size: number
  preview?: string // Data URL for image preview
}

// ============================================================================
// MODULE-LEVEL STATE
// ============================================================================
// These must be module-level to ensure only one paste handler exists
// and isProcessing flag is shared across all component instances.
// Without this, component remounts create duplicate handlers.

let moduleIsProcessing = false
let moduleProcessingStartTime = 0
let modulePasteListenerSetup = false
let moduleHandlePaste: ((event: ClipboardEvent) => Promise<void>) | null = null

// Auto-reset processing flag if it's been stuck for more than 5 seconds
const PROCESSING_TIMEOUT_MS = 5000

function checkAndResetStuckProcessing(): boolean {
  if (moduleIsProcessing && moduleProcessingStartTime > 0) {
    const elapsed = Date.now() - moduleProcessingStartTime
    if (elapsed > PROCESSING_TIMEOUT_MS) {
      console.log('[clipboard] Auto-resetting stuck processing flag after', elapsed, 'ms')
      moduleIsProcessing = false
      moduleProcessingStartTime = 0
      return true
    }
  }
  return false
}

/**
 * Composable for handling clipboard paste uploads
 *
 * Features:
 * - Listen to paste events (CTRL+V / CMD+V)
 * - Detect clipboard content type (image, text, files)
 * - Generate filenames with timestamps
 * - Resolve name conflicts
 * - Show confirmation dialog
 * - Upload via existing upload infrastructure
 */
export function useClipboardUpload() {
  const appStore = useAppStore()
  const { startUpload, startUploadFromBytes } = useRustUploadManager()

  // State - these are reactive for the UI
  const isProcessing = ref(false)
  const isUploading = ref(false)
  const pendingItems = ref<ClipboardItem[]>([])
  const showConfirmDialog = ref(false)

  // Computed
  const canPaste = computed(() => {
    return appStore.currentProfile !== null && appStore.currentBucket !== null
  })

  /**
   * Generate a timestamp-based filename
   */
  function generateTimestamp(): string {
    const now = new Date()
    const year = now.getFullYear()
    const month = String(now.getMonth() + 1).padStart(2, '0')
    const day = String(now.getDate()).padStart(2, '0')
    const hours = String(now.getHours()).padStart(2, '0')
    const minutes = String(now.getMinutes()).padStart(2, '0')
    const seconds = String(now.getSeconds()).padStart(2, '0')
    return `${year}-${month}-${day}_${hours}-${minutes}-${seconds}`
  }

  /**
   * Generate filename based on type
   */
  function generateFileName(type: 'image' | 'text', extension: string): string {
    const timestamp = generateTimestamp()
    if (type === 'image') {
      return `screenshot_${timestamp}.${extension}`
    } else {
      return `pasted_text_${timestamp}.${extension}`
    }
  }

  /**
   * Resolve name conflicts by adding (1), (2), etc.
   */
  function resolveNameConflict(baseName: string): string {
    const prefix = appStore.currentPrefix
    const existingKeys = appStore.objects.map((obj) => obj.key)

    let finalName = baseName
    let counter = 1

    while (existingKeys.includes(prefix + finalName)) {
      const lastDot = baseName.lastIndexOf('.')
      if (lastDot > 0) {
        finalName = `${baseName.slice(0, lastDot)} (${counter})${baseName.slice(lastDot)}`
      } else {
        finalName = `${baseName} (${counter})`
      }
      counter++
    }

    return finalName
  }

  /**
   * Check if an element is an input or editable
   */
  function isInputElement(element: Element | null): boolean {
    if (!element) return false

    const tagName = element.tagName.toLowerCase()

    // Check for form inputs
    if (tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
      return true
    }

    // Check for contenteditable
    if (element.getAttribute('contenteditable') === 'true') {
      return true
    }

    // Check for Monaco editor or other code editors
    if (element.classList.contains('monaco-editor') ||
        element.closest('.monaco-editor') !== null) {
      return true
    }

    return false
  }

  /**
   * Get image extension from MIME type
   */
  function getImageExtension(mimeType: string): string {
    const mimeToExt: Record<string, string> = {
      'image/png': 'png',
      'image/jpeg': 'jpg',
      'image/gif': 'gif',
      'image/webp': 'webp',
      'image/bmp': 'bmp',
      'image/svg+xml': 'svg',
    }
    return mimeToExt[mimeType] || 'png'
  }

  /**
   * Detect text format and suggest appropriate extension
   */
  function detectTextFormat(text: string): { fileName: string; extension: string } {
    const trimmed = text.trim()

    // JSON detection
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) ||
        (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      try {
        JSON.parse(trimmed)
        return { fileName: 'pasted.json', extension: 'json' }
      } catch {
        // Not valid JSON, continue
      }
    }

    // XML detection
    if (trimmed.startsWith('<?xml') || (trimmed.startsWith('<') && trimmed.includes('</') && trimmed.endsWith('>'))) {
      return { fileName: 'pasted.xml', extension: 'xml' }
    }

    // HTML detection
    if (trimmed.toLowerCase().includes('<!doctype html') ||
        trimmed.toLowerCase().includes('<html') ||
        (trimmed.startsWith('<') && /<\/?(div|span|p|a|body|head|script|style|table|form)\b/i.test(trimmed))) {
      return { fileName: 'pasted.html', extension: 'html' }
    }

    // CSS detection
    if (/^[\s\S]*\{[\s\S]*:[\s\S]*;[\s\S]*\}/.test(trimmed) &&
        (trimmed.includes('color:') || trimmed.includes('margin:') || trimmed.includes('padding:') ||
         trimmed.includes('display:') || trimmed.includes('font-'))) {
      return { fileName: 'pasted.css', extension: 'css' }
    }

    // JavaScript/TypeScript detection (basic)
    if (/^(import |export |const |let |var |function |class |async |=>\s*\{)/.test(trimmed) ||
        /\b(function|const|let|var)\s+\w+\s*[=\(]/.test(trimmed)) {
      return { fileName: 'pasted.js', extension: 'js' }
    }

    // SQL detection
    if (/^(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER|WITH)\s/i.test(trimmed)) {
      return { fileName: 'pasted.sql', extension: 'sql' }
    }

    // Markdown detection (headers, lists, links)
    if (/^#{1,6}\s/.test(trimmed) || /^\s*[-*+]\s/.test(trimmed) || /\[.+\]\(.+\)/.test(trimmed)) {
      return { fileName: 'pasted.md', extension: 'md' }
    }

    // Default to plain text
    return { fileName: 'pasted.txt', extension: 'txt' }
  }

  /**
   * Get content type from file extension
   */
  function getContentTypeFromExtension(fileName: string): string {
    const ext = fileName.split('.').pop()?.toLowerCase() || ''
    const extToMime: Record<string, string> = {
      // Images
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      png: 'image/png',
      gif: 'image/gif',
      webp: 'image/webp',
      svg: 'image/svg+xml',
      bmp: 'image/bmp',
      ico: 'image/x-icon',
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
      html: 'text/html',
      htm: 'text/html',
      css: 'text/css',
      js: 'application/javascript',
      ts: 'text/typescript',
      json: 'application/json',
      xml: 'application/xml',
      yaml: 'text/yaml',
      yml: 'text/yaml',
      md: 'text/markdown',
      csv: 'text/csv',
      // Archives
      zip: 'application/zip',
      rar: 'application/x-rar-compressed',
      '7z': 'application/x-7z-compressed',
      tar: 'application/x-tar',
      gz: 'application/gzip',
      // Media
      mp3: 'audio/mpeg',
      mp4: 'video/mp4',
      avi: 'video/x-msvideo',
      mov: 'video/quicktime',
      wav: 'audio/wav',
    }
    return extToMime[ext] || 'application/octet-stream'
  }

  /**
   * Handle paste event
   */
  async function handlePaste(event: ClipboardEvent) {
    console.log('[clipboard] handlePaste called, moduleIsProcessing:', moduleIsProcessing)

    // Check and auto-reset stuck processing flag
    checkAndResetStuckProcessing()

    // Check if we're in a context where paste should work for uploads
    if (!canPaste.value) {
      console.log('[clipboard] canPaste is false, returning')
      return // No bucket selected, let normal paste behavior work
    }

    // Check if focus is on an input element
    if (isInputElement(document.activeElement)) {
      console.log('[clipboard] focus is on input element, returning')
      return // Let normal paste behavior work
    }

    // =========================================================================
    // IMPORTANT: Read clipboard data SYNCHRONOUSLY before any async operations
    // =========================================================================
    // In WebKit (Safari/Tauri on macOS), clipboard data becomes unavailable
    // after preventDefault() or async operations. We must capture it NOW.

    // Capture text synchronously
    let capturedText = ''
    try {
      capturedText = event.clipboardData?.getData('text/plain') || ''
      console.log('[clipboard] captured text (sync):', capturedText ? `"${capturedText.substring(0, 50)}..."` : '(empty)')
    } catch (e) {
      console.warn('[clipboard] failed to capture text:', e)
    }

    // Capture image files synchronously
    const capturedImages: { file: File; type: string }[] = []
    if (event.clipboardData?.items) {
      for (const item of event.clipboardData.items) {
        if (item.type.startsWith('image/')) {
          const file = item.getAsFile()
          if (file) {
            capturedImages.push({ file, type: item.type })
          }
        }
      }
    }
    console.log('[clipboard] captured images (sync):', capturedImages.length)

    // Prevent default paste behavior
    event.preventDefault()

    // Use module-level flag to prevent duplicate processing
    if (moduleIsProcessing) {
      console.log('[clipboard] already processing (module flag), returning')
      return // Already processing a paste
    }

    moduleIsProcessing = true
    moduleProcessingStartTime = Date.now()
    isProcessing.value = true
    pendingItems.value = []

    try {
      const clipboardItems: ClipboardItem[] = []
      let idCounter = 0

      console.log('[clipboard] processing captured data...')

      // 1. Process captured images (already captured synchronously)
      for (const { file, type } of capturedImages) {
        const arrayBuffer = await file.arrayBuffer()
        const data = new Uint8Array(arrayBuffer)
        const extension = getImageExtension(type)

        // Use original filename if available (image copied from disk)
        // Otherwise generate screenshot_timestamp name (pure screenshot)
        let baseName: string
        const fileName = file.name?.toLowerCase() || ''
        const isGenericName = !file.name ||
          fileName === 'image.png' ||
          fileName === 'image.jpg' ||
          fileName === 'image.jpeg' ||
          fileName === 'blob' ||
          fileName.startsWith('blob')

        if (isGenericName) {
          // Pure screenshot - generate timestamp name
          baseName = generateFileName('image', extension)
        } else {
          // Image from disk - keep original name
          baseName = file.name
        }
        const finalName = resolveNameConflict(baseName)

        // Create preview URL
        const preview = URL.createObjectURL(file)

        clipboardItems.push({
          id: `clipboard_${++idCounter}`,
          type: 'image',
          name: finalName,
          data,
          path: null,
          contentType: type,
          size: data.length,
          preview,
        })
      }
      console.log('[clipboard] processed images:', clipboardItems.length)

      // 2. If no images, try to get files from OS clipboard (via Rust)
      // This must come BEFORE text, because copying a file from Finder
      // also puts text (the path/name) in the clipboard
      console.log('[clipboard] clipboardItems after images:', clipboardItems.length)
      if (clipboardItems.length === 0) {
        console.log('[clipboard] no images found, trying readClipboardFiles...')
        try {
          const filePaths = await readClipboardFiles()
          console.log('[clipboard] readClipboardFiles returned:', filePaths)
          for (const filePath of filePaths) {
            // Extract filename from path
            const pathParts = filePath.split(/[\\/]/)
            const fileName = pathParts[pathParts.length - 1]
            const baseName = resolveNameConflict(fileName)
            const contentType = getContentTypeFromExtension(fileName)

            // Get file size
            let fileSize = 0
            try {
              fileSize = await getFileSize(filePath)
            } catch {
              // Ignore errors, use 0
            }

            clipboardItems.push({
              id: `clipboard_${++idCounter}`,
              type: 'file',
              name: baseName,
              data: null,
              path: filePath,
              contentType,
              size: fileSize,
            })
          }
        } catch (error) {
          console.warn('Failed to read clipboard files:', error)
        }
      }

      // 3. If no images and no files, use captured text
      console.log('[clipboard] checking for text, clipboardItems.length:', clipboardItems.length)
      if (clipboardItems.length === 0 && capturedText && capturedText.trim().length > 0) {
        console.log('[clipboard] using captured text:', `"${capturedText.substring(0, 50)}..."`)
        const data = new TextEncoder().encode(capturedText)
        const { fileName, extension } = detectTextFormat(capturedText)
        const baseName = generateFileName('text', extension)
        const finalName = resolveNameConflict(baseName)

        clipboardItems.push({
          id: `clipboard_${++idCounter}`,
          type: 'text',
          name: finalName,
          data,
          path: null,
          contentType: getContentTypeFromExtension(fileName),
          size: data.length,
          preview: capturedText.length > 200 ? capturedText.substring(0, 200) + '...' : capturedText,
        })
        console.log('[clipboard] text item created:', finalName)
      } else if (clipboardItems.length === 0) {
        console.log('[clipboard] no text found or text is empty')
      }

      // If we found any items, show confirmation dialog
      console.log('[clipboard] final clipboardItems count:', clipboardItems.length)
      if (clipboardItems.length > 0) {
        console.log('[clipboard] showing confirmation dialog')
        pendingItems.value = clipboardItems
        showConfirmDialog.value = true
      } else {
        console.log('[clipboard] no items found, not showing dialog')
      }
    } catch (error) {
      console.error('[clipboard] error in handlePaste:', error)
    } finally {
      moduleIsProcessing = false
      moduleProcessingStartTime = 0
      isProcessing.value = false
    }
  }

  /**
   * Confirm and start uploads
   */
  async function confirmUpload() {
    if (pendingItems.value.length === 0) return
    if (isUploading.value) return

    isUploading.value = true
    const profileId = appStore.currentProfile!.id
    const bucket = appStore.currentBucket!
    const prefix = appStore.currentPrefix

    let successCount = 0
    let errorCount = 0

    for (const item of pendingItems.value) {
      const key = prefix + item.name

      try {
        if (item.type === 'file' && item.path) {
          // File from OS - use existing file upload (shows in RustUploadProgress)
          await startUpload(
            profileId,
            bucket,
            key,
            item.path,
            item.contentType
          )
          successCount++
        } else if (item.data) {
          // Image or text from clipboard - use startUploadFromBytes which:
          // 1. Registers bucket/key metadata for the upload:object-completed event
          // 2. Shows progress in RustUploadProgress modal
          await startUploadFromBytes(
            profileId,
            bucket,
            key,
            item.data,
            item.contentType
          )
          successCount++
        }
      } catch (error) {
        console.error(`Failed to upload ${item.name}:`, error)
        errorCount++
      }
    }

    // Cleanup
    isUploading.value = false
    cleanupPendingItems()
    showConfirmDialog.value = false

    // Return result for toast feedback
    return { successCount, errorCount }
  }

  /**
   * Cancel upload
   */
  function cancelUpload() {
    cleanupPendingItems()
    showConfirmDialog.value = false
  }

  /**
   * Cleanup preview URLs and reset state
   */
  function cleanupPendingItems() {
    // Revoke preview URLs to free memory
    for (const item of pendingItems.value) {
      if (item.preview) {
        URL.revokeObjectURL(item.preview)
      }
    }
    pendingItems.value = []
  }

  /**
   * Update item name (for manual editing)
   */
  function updateItemName(itemId: string, newName: string) {
    const item = pendingItems.value.find((i) => i.id === itemId)
    if (item) {
      item.name = newName
    }
  }

  /**
   * Remove item from pending list
   */
  function removeItem(itemId: string) {
    const index = pendingItems.value.findIndex((i) => i.id === itemId)
    if (index !== -1) {
      const item = pendingItems.value[index]
      if (item.preview) {
        URL.revokeObjectURL(item.preview)
      }
      pendingItems.value.splice(index, 1)

      // If no items left, close dialog
      if (pendingItems.value.length === 0) {
        showConfirmDialog.value = false
      }
    }
  }

  /**
   * Setup paste event listener
   * Uses module-level tracking to ensure only one listener exists globally
   */
  function setupPasteListener() {
    console.log('[clipboard] setupPasteListener called, already setup:', modulePasteListenerSetup)

    // Remove any existing listener first to prevent duplicates
    if (moduleHandlePaste) {
      document.removeEventListener('paste', moduleHandlePaste as unknown as EventListener)
    }

    // Store and add the new handler
    moduleHandlePaste = handlePaste
    document.addEventListener('paste', handlePaste as unknown as EventListener)
    modulePasteListenerSetup = true

    // Reset processing flag in case it got stuck
    moduleIsProcessing = false
    moduleProcessingStartTime = 0

    console.log('[clipboard] paste listener added to document, flags reset')
  }

  /**
   * Cleanup paste event listener
   */
  function cleanupPasteListener() {
    if (!modulePasteListenerSetup) return

    if (moduleHandlePaste) {
      document.removeEventListener('paste', moduleHandlePaste as unknown as EventListener)
      moduleHandlePaste = null
    }
    modulePasteListenerSetup = false
    moduleIsProcessing = false
    moduleProcessingStartTime = 0
    cleanupPendingItems()
  }

  return {
    // State
    isProcessing,
    isUploading,
    pendingItems,
    showConfirmDialog,
    canPaste,

    // Actions
    handlePaste,
    confirmUpload,
    cancelUpload,
    updateItemName,
    removeItem,
    setupPasteListener,
    cleanupPasteListener,
  }
}
