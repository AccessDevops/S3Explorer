<template>
  <div ref="editorContainer" class="w-full h-full min-h-[400px] border rounded-lg overflow-hidden"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import * as monaco from 'monaco-editor'

// Configure Monaco Editor theme and options
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

// Setup Monaco environment
self.MonacoEnvironment = {
  getWorker(_: unknown, label: string) {
    if (label === 'json') {
      return new jsonWorker()
    }
    if (label === 'css' || label === 'scss' || label === 'less') {
      return new cssWorker()
    }
    if (label === 'html' || label === 'handlebars' || label === 'razor') {
      return new htmlWorker()
    }
    if (label === 'typescript' || label === 'javascript') {
      return new tsWorker()
    }
    return new editorWorker()
  }
}

const props = withDefaults(
  defineProps<{
    modelValue: string
    language?: string
    readonly?: boolean
    theme?: 'vs' | 'vs-dark' | 'hc-black'
    filename?: string
  }>(),
  {
    language: 'plaintext',
    readonly: false,
    theme: 'vs-dark'
  }
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const editorContainer = ref<HTMLElement | null>(null)
let editor: monaco.editor.IStandaloneCodeEditor | null = null

// Detect language from filename
function detectLanguage(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || ''

  const languageMap: Record<string, string> = {
    // JavaScript/TypeScript
    'js': 'javascript',
    'jsx': 'javascript',
    'ts': 'typescript',
    'tsx': 'typescript',
    'mjs': 'javascript',
    'cjs': 'javascript',

    // Web
    'html': 'html',
    'htm': 'html',
    'css': 'css',
    'scss': 'scss',
    'sass': 'sass',
    'less': 'less',

    // Data formats
    'json': 'json',
    'xml': 'xml',
    'yaml': 'yaml',
    'yml': 'yaml',
    'toml': 'toml',
    'ini': 'ini',
    'conf': 'ini',
    'config': 'ini',

    // Programming languages
    'py': 'python',
    'java': 'java',
    'c': 'c',
    'cpp': 'cpp',
    'cc': 'cpp',
    'cxx': 'cpp',
    'h': 'c',
    'hpp': 'cpp',
    'cs': 'csharp',
    'php': 'php',
    'rb': 'ruby',
    'go': 'go',
    'rs': 'rust',
    'swift': 'swift',
    'kt': 'kotlin',
    'scala': 'scala',
    'r': 'r',

    // Shell/Scripts
    'sh': 'shell',
    'bash': 'shell',
    'zsh': 'shell',
    'fish': 'shell',
    'ps1': 'powershell',
    'bat': 'bat',
    'cmd': 'bat',

    // Database
    'sql': 'sql',

    // Markup/Doc
    'md': 'markdown',
    'markdown': 'markdown',
    'rst': 'restructuredtext',
    'tex': 'latex',

    // Other
    'dockerfile': 'dockerfile',
    'dockerignore': 'ignore',
    'gitignore': 'ignore',
    'env': 'shell',
    'log': 'log',
    'txt': 'plaintext',
  }

  return languageMap[ext] || 'plaintext'
}

onMounted(() => {
  if (!editorContainer.value) return

  // Determine language
  const lang = props.filename ? detectLanguage(props.filename) : props.language

  // Create editor
  editor = monaco.editor.create(editorContainer.value, {
    value: props.modelValue,
    language: lang,
    theme: props.theme,
    readOnly: props.readonly,
    automaticLayout: true,
    minimap: { enabled: true },
    scrollBeyondLastLine: false,
    fontSize: 14,
    lineNumbers: 'on',
    renderWhitespace: 'selection',
    tabSize: 2,
    insertSpaces: true,
    wordWrap: 'on',
    bracketPairColorization: {
      enabled: true
    },
    scrollbar: {
      verticalScrollbarSize: 10,
      horizontalScrollbarSize: 10
    }
  })

  // Listen for content changes
  editor.onDidChangeModelContent(() => {
    if (editor && !props.readonly) {
      emit('update:modelValue', editor.getValue())
    }
  })
})

onUnmounted(() => {
  editor?.dispose()
})

// Watch for external content changes
watch(() => props.modelValue, (newValue) => {
  if (editor && editor.getValue() !== newValue) {
    editor.setValue(newValue)
  }
})

// Watch for language changes
watch(() => props.language, (newLang) => {
  if (editor) {
    const model = editor.getModel()
    if (model) {
      monaco.editor.setModelLanguage(model, newLang)
    }
  }
})

// Watch for readonly changes
watch(() => props.readonly, (newReadonly) => {
  if (editor) {
    editor.updateOptions({ readOnly: newReadonly })
  }
})

// Watch for theme changes
watch(() => props.theme, (newTheme) => {
  if (editor) {
    monaco.editor.setTheme(newTheme)
  }
})
</script>
