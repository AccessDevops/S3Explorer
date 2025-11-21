<template>
  <div class="relative flex flex-col h-full bg-background" :class="{ 'bg-blue-50': isDraggingOver }">
    <!-- Drop Zone Overlay -->
    <div
      v-if="isDraggingOver"
      class="absolute inset-0 bg-primary/10 backdrop-blur-sm flex items-center justify-center z-50 pointer-events-none border-4 border-dashed border-primary m-4 rounded-lg"
    >
      <Card class="p-8">
        <div class="text-center">
          <div class="text-6xl mb-4 animate-bounce">📁</div>
          <div class="text-xl font-semibold text-primary">{{ t('dropFilesHere') }}</div>
        </div>
      </Card>
    </div>

    <!-- Toolbar -->
    <div class="flex justify-between items-center gap-4 p-4 border-b bg-card">
      <div class="flex items-center gap-1 text-sm flex-shrink min-w-0">
        <Button variant="link" class="p-0 h-auto font-medium" @click="navigateToRoot">
          {{ appStore.currentBucket }}
        </Button>
        <template v-if="displayPathParts.length > 0">
          <span v-for="(part, index) in displayPathParts" :key="index" class="flex items-center">
            <span class="mx-1 text-muted-foreground">/</span>
            <Button
              v-if="part.text !== '...'"
              variant="link"
              class="p-0 h-auto font-medium"
              @click="navigateToPath(part.index)"
            >
              {{ part.text }}
            </Button>
            <span v-else class="text-muted-foreground px-1">...</span>
          </span>
        </template>
      </div>

      <div class="flex-1 max-w-md">
        <Input
          v-model="searchQuery"
          :placeholder="t('searchFilesAndFolders')"
          class="w-full"
        />
      </div>

      <div class="flex gap-2 flex-shrink-0">
        <Button size="sm" variant="outline" @click="showUploadModal = true">{{ t('upload') }}</Button>
        <Button size="sm" variant="outline" @click="showCreateFolderModal = true">{{ t('newFolder') }}</Button>
        <Button size="sm" variant="ghost" @click="appStore.loadObjects()" :title="t('refresh')">⟳</Button>
      </div>
    </div>

    <!-- Object List -->
    <div class="flex-1 overflow-y-auto p-4" @contextmenu.prevent="showEmptyContextMenu($event)" @click="clearSelection">
      <div class="space-y-1" @click.stop>
        <!-- Folders -->
        <div
          v-for="(folder, index) in filteredFolders"
          :key="folder"
          :class="[
            'flex items-center gap-3 p-3 rounded-md hover:bg-accent transition-colors cursor-pointer group select-none',
            selectedItems.has(folder) ? 'bg-primary/20 hover:bg-primary/30' : ''
          ]"
          @click="handleFolderClick($event, folder, index)"
          @dblclick="navigateToFolder(folder)"
          @contextmenu.stop
        >
          <div class="text-2xl">📁</div>
          <div class="flex-1 min-w-0">
            <div class="font-medium truncate">{{ getFolderName(folder) }}</div>
          </div>
          <div class="text-sm text-muted-foreground w-24 text-right">
            {{ getFolderSize(folder) }}
          </div>
          <div class="text-sm text-muted-foreground w-40 text-right">-</div>
          <div v-if="!selectedItems.has(folder)" class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity w-28 justify-end">
            <Button size="sm" variant="destructive" @click.stop="deleteFolderConfirm(folder)" :title="t('delete')">
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
                <path d="M3 6h18"/>
                <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/>
                <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
                <line x1="10" x2="10" y1="11" y2="17"/>
                <line x1="14" x2="14" y1="11" y2="17"/>
              </svg>
            </Button>
          </div>
          <div v-else class="w-28"></div>
        </div>

        <!-- Files -->
        <div
          v-for="(obj, index) in filteredObjects"
          :key="obj.key"
          :class="[
            'flex items-center gap-3 p-3 rounded-md hover:bg-accent transition-colors group cursor-pointer select-none',
            selectedItems.has(obj.key) ? 'bg-primary/20 hover:bg-primary/30' : ''
          ]"
          @click="handleFileClick($event, obj, index + filteredFolders.length)"
          @dblclick="viewObject(obj)"
          @contextmenu.stop="showContextMenu($event, obj)"
        >
          <div class="text-2xl">📄</div>
          <div class="flex-1 min-w-0">
            <div class="font-medium truncate">
              {{ searchQuery.trim() ? obj.key : getFileName(obj.key) }}
            </div>
            <div v-if="searchQuery.trim() && obj.key.includes('/')" class="text-xs text-muted-foreground truncate">
              {{ obj.key.substring(0, obj.key.lastIndexOf('/')) }}/
            </div>
          </div>
          <div class="text-sm text-muted-foreground w-24 text-right">{{ formatSize(obj.size) }}</div>
          <div class="text-sm text-muted-foreground w-40 text-right">{{ formatDate(obj.last_modified) }}</div>
          <div v-if="!selectedItems.has(obj.key)" class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity w-28 justify-end">
            <Button size="sm" variant="secondary" @click.stop="downloadObject(obj.key)" :title="t('download')">
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
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" x2="12" y1="15" y2="3"/>
              </svg>
            </Button>
            <Button size="sm" variant="secondary" @click.stop="viewObject(obj)" :title="t('view')">
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
                <path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/>
                <circle cx="12" cy="12" r="3"/>
              </svg>
            </Button>
            <Button size="sm" variant="destructive" @click.stop="deleteObjectConfirm(obj.key)" :title="t('delete')">
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
                <path d="M3 6h18"/>
                <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/>
                <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
                <line x1="10" x2="10" y1="11" y2="17"/>
                <line x1="14" x2="14" y1="11" y2="17"/>
              </svg>
            </Button>
          </div>
          <div v-else class="w-28"></div>
        </div>
      </div>
    </div>

    <!-- Selection Action Bar -->
    <div
      v-if="selectedItems.size > 0"
      class="border-t bg-primary/10 px-4 py-3 flex items-center justify-between"
    >
      <div class="flex items-center gap-3">
        <span class="font-medium">
          {{ selectedItems.size }} {{ selectedItems.size !== 1 ? t('items') : t('item') }} {{ t('selected') }}
          <span class="text-muted-foreground ml-2">({{ formatSize(selectedTotalSize) }})</span>
        </span>
        <Button size="sm" variant="ghost" @click="clearSelection">{{ t('clear') }}</Button>
      </div>
      <div class="flex gap-2">
        <Button size="sm" variant="secondary" @click="downloadSelectedItems">{{ t('download') }}</Button>
        <Button size="sm" variant="secondary" @click="copySelectedItems">{{ t('copy') }}</Button>
        <Button size="sm" variant="destructive" @click="deleteSelectedItems" :title="t('delete')">
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
            class="mr-1"
          >
            <path d="M3 6h18"/>
            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/>
            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
            <line x1="10" x2="10" y1="11" y2="17"/>
            <line x1="14" x2="14" y1="11" y2="17"/>
          </svg>
          {{ t('delete') }}
        </Button>
      </div>
    </div>

    <!-- Footer Stats -->
    <div class="border-t bg-card px-4 py-2 flex items-center justify-between text-sm text-muted-foreground">
      <div>
        {{ totalItemsCount }} {{ totalItemsCount !== 1 ? t('items') : t('item') }}
        ({{ filteredFolders.length }} {{ filteredFolders.length !== 1 ? t('folders') : t('folderName') }},
        {{ filteredObjects.length }} {{ filteredObjects.length !== 1 ? t('files') : t('fileName') }})
      </div>
      <div>
        {{ t('totalSize') }}: {{ formatSize(totalSize) }}
      </div>
    </div>

    <!-- Upload Modal -->
    <Dialog v-model:open="showUploadModal">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('uploadFile') }}</DialogTitle>
        </DialogHeader>
        <input type="file" @change="handleFileSelect" class="mb-4" />
        <div v-if="uploadFile" class="p-3 bg-muted rounded-md mb-4">
          <p class="text-sm"><strong>{{ t('selectedFile') }}:</strong> {{ uploadFile.name }}</p>
          <p class="text-sm text-muted-foreground"><strong>{{ t('size') }}:</strong> {{ formatSize(uploadFile.size) }}</p>
        </div>
        <DialogFooter>
          <Button @click="uploadFileHandler" :disabled="!uploadFile">{{ t('upload') }}</Button>
          <Button variant="secondary" @click="showUploadModal = false">{{ t('cancel') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Create Folder Modal -->
    <Dialog v-model:open="showCreateFolderModal">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('createFolder') }}</DialogTitle>
        </DialogHeader>
        <Input
          v-model="newFolderName"
          :placeholder="t('folderName')"
          @keyup.enter="createFolderHandler"
          class="mb-4"
        />
        <DialogFooter>
          <Button @click="createFolderHandler">{{ t('create') }}</Button>
          <Button variant="secondary" @click="showCreateFolderModal = false">{{ t('cancel') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- View Object Modal -->
    <Dialog v-model:open="showViewModal">
      <DialogContent class="max-w-4xl max-h-[90vh]">
        <DialogHeader>
          <DialogTitle class="flex items-center gap-2">
            <span>{{ viewingObject ? getFileName(viewingObject.key) : '' }}</span>
            <span v-if="objectViewerRef?.contentType" class="text-xs font-normal text-muted-foreground px-2 py-1 bg-muted rounded">
              {{ objectViewerRef.contentType }}
            </span>
          </DialogTitle>
        </DialogHeader>
        <div class="overflow-y-auto max-h-[70vh]">
          <ObjectViewer v-if="viewingObject" ref="objectViewerRef" :object="viewingObject" />
        </div>
        <DialogFooter>
          <template v-if="objectViewerRef?.isText">
            <Button
              v-if="!objectViewerRef?.isEditing"
              @click="objectViewerRef?.startEditing()"
            >
              {{ t('edit') }}
            </Button>
            <template v-else>
              <Button
                @click="objectViewerRef?.saveChanges()"
                :disabled="objectViewerRef?.saving"
              >
                {{ objectViewerRef?.saving ? t('saving') : t('save') }}
              </Button>
              <Button
                variant="outline"
                @click="objectViewerRef?.cancelEditing()"
              >
                {{ t('cancel') }}
              </Button>
            </template>
          </template>
          <Button variant="secondary" @click="showViewModal = false">{{ t('close') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Context Menu (File) -->
    <div
      v-if="contextMenu.show"
      :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
      class="fixed z-50 min-w-[180px] bg-popover text-popover-foreground rounded-md border shadow-md p-1"
      @click="closeContextMenu"
    >
      <button
        @click="copyFile"
        class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer"
      >
        📋 {{ t('copy') }}
      </button>
      <button
        @click="startRename"
        class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer"
      >
        ✏️ {{ t('rename') }}
      </button>
      <button
        @click="viewObjectVersions"
        class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer"
      >
        📚 {{ t('viewVersions') }}
      </button>
      <div class="relative">
        <button
          @click.stop="showContentTypeSubmenu = !showContentTypeSubmenu"
          @mouseenter="showContentTypeSubmenu = true"
          class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer flex items-center justify-between"
        >
          <span>🏷️ {{ t('changeContentType') }}</span>
          <span class="text-xs">▶</span>
        </button>

        <!-- Content Type Submenu -->
        <div
          v-if="showContentTypeSubmenu"
          @click.stop
          @mouseleave="showContentTypeSubmenu = false"
          class="absolute left-full top-0 ml-1 min-w-[280px] max-h-[400px] overflow-y-auto bg-popover text-popover-foreground rounded-md border shadow-md p-1 z-50"
        >
          <button
            v-for="(type, index) in commonContentTypes"
            :key="index"
            @click="changeContentTypeDirectly(type.value)"
            :class="[
              'w-full text-left px-3 py-2 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer',
              type.isRecommended ? 'font-bold border-b border-border mb-1' : ''
            ]"
          >
            {{ type.label }}
          </button>
        </div>
      </div>
    </div>

    <!-- Context Menu (Empty Area) -->
    <div
      v-if="emptyContextMenu.show"
      :style="{ top: emptyContextMenu.y + 'px', left: emptyContextMenu.x + 'px' }"
      class="fixed z-50 min-w-[180px] bg-popover text-popover-foreground rounded-md border shadow-md p-1"
      @click="closeEmptyContextMenu"
    >
      <button
        v-if="copiedFile"
        @click="pasteFile"
        class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer"
      >
        📋 {{ t('paste') }} "{{ getFileName(copiedFile.key) }}"
      </button>
      <div v-if="copiedFile" class="h-px bg-border my-1"></div>
      <button
        @click="openCreateFileModal"
        class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer"
      >
        📄 {{ t('newFile') }}
      </button>
      <button
        @click="openCreateFolderModalFromContext"
        class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer"
      >
        📁 {{ t('newFolder') }}
      </button>
    </div>

    <!-- Rename Modal -->
    <Dialog v-model:open="showRenameModal">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('renameFile') }}</DialogTitle>
        </DialogHeader>
        <div class="space-y-4">
          <div>
            <label class="text-sm font-medium">{{ t('newName') }}</label>
            <Input
              v-model="newFileName"
              :placeholder="t('enterNewFileName')"
              class="mt-1"
              @keyup.enter="renameFileHandler"
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="showRenameModal = false">{{ t('cancel') }}</Button>
          <Button @click="renameFileHandler" :disabled="!newFileName.trim() || renaming">
            {{ renaming ? t('renaming') : t('rename') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Create File Modal -->
    <Dialog v-model:open="showCreateFileModal">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('createNewFile') }}</DialogTitle>
        </DialogHeader>
        <div class="space-y-4">
          <div>
            <label class="text-sm font-medium">{{ t('fileName') }}</label>
            <Input
              v-model="newFileName"
              placeholder="filename.txt"
              class="mt-1"
              @keyup.enter="createFileHandler"
            />
          </div>
          <div>
            <label class="text-sm font-medium">{{ t('contentOptional') }}</label>
            <textarea
              v-model="newFileContent"
              :placeholder="t('fileContentPlaceholder')"
              class="w-full min-h-[120px] p-3 text-sm font-mono border rounded-md resize-y bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            ></textarea>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="showCreateFileModal = false">{{ t('cancel') }}</Button>
          <Button @click="createFileHandler" :disabled="!newFileName.trim() || creatingFile">
            {{ creatingFile ? t('creating') : t('create') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Versions Modal -->
    <Dialog v-model:open="showVersionsModal">
      <DialogContent class="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{{ t('versions') }} - {{ versionsObject ? getFileName(versionsObject.key) : '' }}</DialogTitle>
        </DialogHeader>

        <div v-if="loadingVersions" class="flex justify-center py-12">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
        </div>

        <div v-else-if="objectVersions.length === 0" class="text-center py-12 text-muted-foreground">
          <div class="text-5xl mb-4">📚</div>
          <p>{{ t('noVersions') }}</p>
          <p class="text-sm mt-2">{{ t('versioningNotEnabled') }}</p>
        </div>

        <div v-else class="space-y-2">
          <div
            v-for="version in objectVersions"
            :key="version.version_id"
            class="flex items-center gap-3 p-4 rounded-md border hover:bg-accent transition-colors"
          >
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="text-sm font-medium font-mono truncate">{{ version.version_id }}</span>
                <span v-if="version.is_latest" class="px-2 py-0.5 text-xs font-semibold bg-primary text-primary-foreground rounded">
                  {{ t('latest') }}
                </span>
              </div>
              <div class="flex gap-4 text-xs text-muted-foreground">
                <span>{{ formatSize(version.size) }}</span>
                <span v-if="version.last_modified">{{ formatDate(version.last_modified) }}</span>
              </div>
            </div>
            <Button size="sm" variant="secondary" @click="downloadObjectVersion(version)" :title="t('download')">
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
                class="mr-1"
              >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" x2="12" y1="15" y2="3"/>
              </svg>
              {{ t('download') }}
            </Button>
          </div>
        </div>

        <DialogFooter>
          <Button variant="secondary" @click="showVersionsModal = false">{{ t('close') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Overlay to close context menus -->
    <div
      v-if="contextMenu.show || emptyContextMenu.show"
      @click="closeAllContextMenus"
      @contextmenu.prevent="closeAllContextMenus"
      class="fixed inset-0 z-40"
    ></div>

    <!-- Upload Progress Bar -->
    <div
      v-if="uploadProgress.isUploading"
      class="fixed bottom-4 right-4 z-50 bg-card border border-border rounded-lg shadow-lg p-4 min-w-[320px]"
    >
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm font-medium">{{ t('uploading') }}</span>
        <span class="text-sm text-muted-foreground">{{ uploadProgress.currentIndex }}/{{ uploadProgress.totalFiles }}</span>
      </div>
      <div class="text-xs text-muted-foreground mb-2 truncate">{{ uploadProgress.currentFile }}</div>
      <div class="w-full bg-muted rounded-full h-2 overflow-hidden mb-2">
        <div
          class="bg-primary h-full transition-all duration-300"
          :style="{ width: `${(uploadProgress.uploadedBytes / uploadProgress.totalBytes) * 100}%` }"
        ></div>
      </div>
      <div class="flex items-center justify-between text-xs text-muted-foreground">
        <span>{{ formatSize(uploadProgress.uploadedBytes) }} / {{ formatSize(uploadProgress.totalBytes) }}</span>
        <span>
          {{ uploadProgress.estimatedTimeRemaining === '--' ? t('estimating') : `${uploadProgress.estimatedTimeRemaining} ${t('remaining')}` }}
        </span>
      </div>
      <div v-if="uploadProgress.failCount > 0" class="text-xs text-destructive mt-2">
        {{ uploadProgress.failCount }} {{ uploadProgress.failCount > 1 ? 'errors' : 'error' }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '../stores/app'
import { useI18n } from '../composables/useI18n'
import { useDialog } from '../composables/useDialog'
import { useToast } from '../composables/useToast'
import { createFolder, deleteObject, calculateFolderSize, deleteFolder, putObject, listObjects, copyObject, listObjectVersions } from '../services/tauri'
import { save } from '@tauri-apps/api/dialog'
import { writeBinaryFile, readBinaryFile } from '@tauri-apps/api/fs'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { S3Object, ObjectVersion } from '../types'
import ObjectViewer from './ObjectViewer.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

const appStore = useAppStore()
const { t } = useI18n()
const dialog = useDialog()
const toast = useToast()
const showUploadModal = ref(false)
const showCreateFolderModal = ref(false)
const showViewModal = ref(false)
const uploadFile = ref<File | null>(null)
const newFolderName = ref('')
const viewingObject = ref<S3Object | null>(null)
const objectViewerRef = ref<InstanceType<typeof ObjectViewer> | null>(null)
const folderSizes = ref<Map<string, number>>(new Map())
const loadingFolderSizes = ref<Set<string>>(new Set())
const isDraggingOver = ref(false)
const searchQuery = ref('')
const isSearching = ref(false)
const globalSearchResults = ref<S3Object[]>([])
const contextMenu = ref({ show: false, x: 0, y: 0, object: null as S3Object | null })
const showRenameModal = ref(false)
const newFileName = ref('')
const renamingObject = ref<S3Object | null>(null)
const renaming = ref(false)
const showContentTypeSubmenu = ref(false)
const changingContentType = ref(false)
const emptyContextMenu = ref({ show: false, x: 0, y: 0 })
const showCreateFileModal = ref(false)
const newFileContent = ref('')
const creatingFile = ref(false)
const copiedFile = ref<S3Object | null>(null)
const pasting = ref(false)
const selectedItems = ref<Set<string>>(new Set())
const lastSelectedIndex = ref<number>(-1)
const showVersionsModal = ref(false)
const objectVersions = ref<ObjectVersion[]>([])
const loadingVersions = ref(false)
const versionsObject = ref<S3Object | null>(null)

// Upload progress tracking
const uploadProgress = ref({
  isUploading: false,
  currentFile: '',
  currentIndex: 0,
  totalFiles: 0,
  successCount: 0,
  failCount: 0,
  startTime: 0,
  totalBytes: 0,
  uploadedBytes: 0,
  estimatedTimeRemaining: ''
})

let unlistenFileDrop: UnlistenFn | null = null
let unlistenFileDropHover: UnlistenFn | null = null
let unlistenFileDropCancelled: UnlistenFn | null = null

const pathParts = computed(() => {
  if (!appStore.currentPrefix) return []
  return appStore.currentPrefix.split('/').filter((p) => p)
})

// Common content types list
const commonContentTypes = computed(() => {
  if (!contextMenu.value.object) return []

  const fileName = getFileName(contextMenu.value.object.key)
  const recommended = getContentTypeFromExtension(fileName)

  const types = [
    // Recommended (will be styled differently)
    { value: recommended, label: `${recommended} (recommandé)`, isRecommended: true },

    // Images
    { value: 'image/jpeg', label: 'image/jpeg', category: 'Images' },
    { value: 'image/png', label: 'image/png', category: 'Images' },
    { value: 'image/gif', label: 'image/gif', category: 'Images' },
    { value: 'image/webp', label: 'image/webp', category: 'Images' },
    { value: 'image/svg+xml', label: 'image/svg+xml', category: 'Images' },

    // Documents
    { value: 'application/pdf', label: 'application/pdf', category: 'Documents' },
    { value: 'application/msword', label: 'application/msword', category: 'Documents' },
    { value: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', label: 'application/vnd...docx', category: 'Documents' },
    { value: 'application/vnd.ms-excel', label: 'application/vnd.ms-excel', category: 'Documents' },
    { value: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', label: 'application/vnd...xlsx', category: 'Documents' },

    // Text
    { value: 'text/plain', label: 'text/plain', category: 'Text' },
    { value: 'text/html', label: 'text/html', category: 'Text' },
    { value: 'text/css', label: 'text/css', category: 'Text' },
    { value: 'text/javascript', label: 'text/javascript', category: 'Text' },

    // Application
    { value: 'application/json', label: 'application/json', category: 'Application' },
    { value: 'application/xml', label: 'application/xml', category: 'Application' },
    { value: 'application/zip', label: 'application/zip', category: 'Application' },
    { value: 'application/octet-stream', label: 'application/octet-stream', category: 'Application' },

    // Video
    { value: 'video/mp4', label: 'video/mp4', category: 'Video' },
    { value: 'video/webm', label: 'video/webm', category: 'Video' },

    // Audio
    { value: 'audio/mpeg', label: 'audio/mpeg', category: 'Audio' },
    { value: 'audio/wav', label: 'audio/wav', category: 'Audio' },
  ]

  // Remove duplicates (if recommended is already in the list)
  return types.filter((type, index) =>
    index === 0 || type.value !== recommended
  )
})

const displayPathParts = computed(() => {
  const parts = pathParts.value
  const maxVisibleParts = 4

  if (parts.length <= maxVisibleParts) {
    // Display all parts
    return parts.map((text, index) => ({ text, index }))
  }

  // Too many parts: show first, "...", and last 2
  const result = []

  // First part
  result.push({ text: parts[0], index: 0 })

  // Ellipsis
  result.push({ text: '...', index: -1 })

  // Last 2 parts
  const lastTwo = parts.length - 2
  for (let i = lastTwo; i < parts.length; i++) {
    result.push({ text: parts[i], index: i })
  }

  return result
})

const filteredFolders = computed(() => {
  if (!searchQuery.value.trim()) return appStore.folders

  // When searching globally, don't show folders separately
  return []
})

const filteredObjects = computed(() => {
  if (!searchQuery.value.trim()) return appStore.objects

  const query = searchQuery.value.toLowerCase()

  // Global search: filter all objects in the bucket
  return globalSearchResults.value.filter((obj) => {
    const fileName = obj.key.toLowerCase()
    return fileName.includes(query)
  })
})

const totalItemsCount = computed(() => {
  return filteredFolders.value.length + filteredObjects.value.length
})

const totalSize = computed(() => {
  return filteredObjects.value.reduce((sum, obj) => sum + obj.size, 0)
})

const selectedTotalSize = computed(() => {
  let size = 0
  for (const key of selectedItems.value) {
    // Check if it's a file
    const obj = appStore.objects.find(o => o.key === key)
    if (obj) {
      size += obj.size
    } else {
      // It's a folder
      const folderSize = folderSizes.value.get(key)
      if (folderSize !== undefined) {
        size += folderSize
      }
    }
  }
  return size
})

// Watch for folder changes and calculate sizes
watch(() => appStore.folders, async (folders) => {
  if (!folders || folders.length === 0) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  for (const folder of folders) {
    if (folderSizes.value.has(folder)) continue

    loadingFolderSizes.value.add(folder)
    try {
      const size = await calculateFolderSize(
        appStore.currentProfile.id,
        appStore.currentBucket,
        folder
      )
      folderSizes.value.set(folder, size)
    } catch (e) {
      console.error(`Failed to calculate size for ${folder}:`, e)
    } finally {
      loadingFolderSizes.value.delete(folder)
    }
  }
}, { immediate: true })

// Watch for search query changes to trigger global search
watch(searchQuery, async (query) => {
  if (!query.trim()) {
    globalSearchResults.value = []
    isSearching.value = false
    return
  }

  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    isSearching.value = true
    globalSearchResults.value = []

    let continuationToken: string | undefined = undefined

    // Paginate through ALL objects in the bucket (no delimiter for recursive listing)
    do {
      const result = await listObjects(
        appStore.currentProfile.id,
        appStore.currentBucket,
        '', // Empty prefix to search entire bucket
        continuationToken,
        1000, // Max keys per request
        false // No delimiter - list all objects recursively
      )

      // Append results
      globalSearchResults.value.push(...result.objects)

      // Check if there are more pages
      continuationToken = result.continuation_token
    } while (continuationToken)

  } catch (e) {
    console.error('Global search failed:', e)
    globalSearchResults.value = []
  } finally {
    isSearching.value = false
  }
})

// Clear selection when navigating to a different folder
watch(() => appStore.currentPrefix, () => {
  clearSelection()
})

function getFolderSize(folder: string): string {
  if (loadingFolderSizes.value.has(folder)) {
    return t('calculating')
  }
  const size = folderSizes.value.get(folder)
  if (size === undefined) return '-'
  return formatSize(size)
}

function getContentTypeFromExtension(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase()

  const contentTypes: Record<string, string> = {
    // Images
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'png': 'image/png',
    'gif': 'image/gif',
    'webp': 'image/webp',
    'svg': 'image/svg+xml',
    'ico': 'image/x-icon',

    // Documents
    'pdf': 'application/pdf',
    'doc': 'application/msword',
    'docx': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'xls': 'application/vnd.ms-excel',
    'xlsx': 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    'ppt': 'application/vnd.ms-powerpoint',
    'pptx': 'application/vnd.openxmlformats-officedocument.presentationml.presentation',

    // Text
    'txt': 'text/plain',
    'html': 'text/html',
    'htm': 'text/html',
    'css': 'text/css',
    'js': 'text/javascript',
    'json': 'application/json',
    'xml': 'application/xml',
    'csv': 'text/csv',

    // Archives
    'zip': 'application/zip',
    'rar': 'application/x-rar-compressed',
    '7z': 'application/x-7z-compressed',
    'tar': 'application/x-tar',
    'gz': 'application/gzip',

    // Video
    'mp4': 'video/mp4',
    'avi': 'video/x-msvideo',
    'mov': 'video/quicktime',
    'wmv': 'video/x-ms-wmv',
    'flv': 'video/x-flv',
    'webm': 'video/webm',

    // Audio
    'mp3': 'audio/mpeg',
    'wav': 'audio/wav',
    'ogg': 'audio/ogg',
    'flac': 'audio/flac',
  }

  return contentTypes[ext || ''] || 'application/octet-stream'
}

function formatTimeRemaining(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return '--'

  if (seconds < 60) {
    return `${Math.round(seconds)}s`
  } else if (seconds < 3600) {
    const mins = Math.floor(seconds / 60)
    const secs = Math.round(seconds % 60)
    return `${mins}m ${secs}s`
  } else {
    const hours = Math.floor(seconds / 3600)
    const mins = Math.floor((seconds % 3600) / 60)
    return `${hours}h ${mins}m`
  }
}

function navigateToRoot() {
  folderSizes.value.clear()
  appStore.navigateToFolder('')
  appStore.loadObjects()
}

function navigateToPath(index: number) {
  folderSizes.value.clear()
  const parts = pathParts.value.slice(0, index + 1)
  const prefix = parts.join('/') + '/'
  appStore.navigateToFolder(prefix)
  appStore.loadObjects()
}

function navigateToFolder(folder: string) {
  folderSizes.value.clear()
  appStore.navigateToFolder(folder)
  appStore.loadObjects()
}

function getFolderName(folder: string): string {
  const parts = folder.split('/').filter((p) => p)
  return parts[parts.length - 1] || folder
}

function getFileName(key: string): string {
  const parts = key.split('/')
  return parts[parts.length - 1] || key
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return '-'
  const date = new Date(dateStr)
  return date.toLocaleString()
}

function handleFileSelect(event: Event) {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    uploadFile.value = target.files[0]
  }
}

async function uploadFileHandler() {
  if (!uploadFile.value || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    const arrayBuffer = await uploadFile.value.arrayBuffer()
    const bytes = Array.from(new Uint8Array(arrayBuffer))
    const key = appStore.currentPrefix + uploadFile.value.name

    // Detect content type by extension as fallback
    const ext = uploadFile.value.name.split('.').pop()?.toLowerCase()
    let contentType = uploadFile.value.type || undefined

    if (!contentType && ext) {
      if (ext === 'jpg' || ext === 'jpeg') contentType = 'image/jpeg'
      else if (ext === 'png') contentType = 'image/png'
      else if (ext === 'gif') contentType = 'image/gif'
      else if (ext === 'pdf') contentType = 'application/pdf'
      else if (ext === 'txt') contentType = 'text/plain'
      else if (ext === 'json') contentType = 'application/json'
      else if (ext === 'xml') contentType = 'application/xml'
      else if (ext === 'zip') contentType = 'application/zip'
    }

    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      key,
      bytes,
      contentType
    )

    showUploadModal.value = false
    uploadFile.value = null
    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('uploadFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

async function createFolderHandler() {
  if (!newFolderName.value || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    const folderPath = appStore.currentPrefix + newFolderName.value
    await createFolder(appStore.currentProfile.id, appStore.currentBucket, folderPath)

    showCreateFolderModal.value = false
    newFolderName.value = ''
    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('createFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

async function downloadObject(key: string) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    const fileName = getFileName(key)
    const filePath = await save({
      defaultPath: fileName,
    })

    if (!filePath) return

    const { getObject } = await import('../services/tauri')
    const response = await getObject(appStore.currentProfile.id, appStore.currentBucket, key)

    await writeBinaryFile(filePath, new Uint8Array(response.content))
    toast.success(t('fileDownloadedSuccess'))
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('downloadFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

function viewObject(obj: S3Object) {
  viewingObject.value = obj
  showViewModal.value = true
}

async function deleteObjectConfirm(key: string) {
  const confirmed = await dialog.confirm({
    title: t('delete'),
    message: t('deleteFileConfirm').replace('{0}', getFileName(key)),
    confirmText: t('delete'),
    cancelText: t('cancel'),
    variant: 'destructive'
  })

  if (!confirmed) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    await deleteObject(appStore.currentProfile.id, appStore.currentBucket, key)
    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('deleteFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

async function deleteFolderConfirm(folder: string) {
  const folderName = getFolderName(folder)
  const confirmed = await dialog.confirm({
    title: t('delete'),
    message: t('deleteFolderConfirm').replace('{0}', folderName),
    confirmText: t('delete'),
    cancelText: t('cancel'),
    variant: 'destructive'
  })

  if (!confirmed) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    const deletedCount = await deleteFolder(appStore.currentProfile.id, appStore.currentBucket, folder)
    toast.success(t('folderDeletedSuccess').replace('{0}', String(deletedCount)))
    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('deleteFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

// Handle file drop using Tauri's event system
async function handleFileDrop(paths: string[]) {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    console.error('No profile or bucket selected')
    return
  }

  console.log(`Starting upload of ${paths.length} file(s)...`)

  // Initialize progress card immediately
  const startTime = Date.now()
  uploadProgress.value = {
    isUploading: true,
    currentFile: '',
    currentIndex: 0,
    totalFiles: paths.length,
    successCount: 0,
    failCount: 0,
    startTime,
    totalBytes: 0, // Will be calculated as we go
    uploadedBytes: 0,
    estimatedTimeRemaining: '--'
  }

  try {
    // Get file sizes first (now happening after progress card is shown)
    const fileSizes: number[] = []
    let totalBytes = 0

    for (const filePath of paths) {
      try {
        const fileData = await readBinaryFile(filePath)
        const size = fileData.length
        fileSizes.push(size)
        totalBytes += size
      } catch (e) {
        console.error(`Failed to get size for ${filePath}:`, e)
        fileSizes.push(0)
      }
    }

    // Update total bytes after calculation
    uploadProgress.value.totalBytes = totalBytes

    let successCount = 0
    let failCount = 0
    let uploadedBytes = 0

    for (let i = 0; i < paths.length; i++) {
      const filePath = paths[i]
      const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file'
      const fileSize = fileSizes[i]

      // Update progress
      uploadProgress.value.currentFile = fileName
      uploadProgress.value.currentIndex = i + 1

      try {
        console.log(`Uploading: ${filePath}`)
        const fileContent = await readBinaryFile(filePath)
        const key = appStore.currentPrefix + fileName

        const contentType = getContentTypeFromExtension(fileName)

        await putObject(
          appStore.currentProfile.id,
          appStore.currentBucket,
          key,
          Array.from(fileContent),
          contentType
        )

        successCount++
        uploadedBytes += fileSize
        uploadProgress.value.successCount = successCount
        uploadProgress.value.uploadedBytes = uploadedBytes

        // Calculate time estimate (only if we have totalBytes calculated)
        if (totalBytes > 0) {
          const elapsedSeconds = (Date.now() - startTime) / 1000
          const bytesRemaining = totalBytes - uploadedBytes
          const bytesPerSecond = uploadedBytes / elapsedSeconds
          const secondsRemaining = bytesRemaining / bytesPerSecond
          uploadProgress.value.estimatedTimeRemaining = formatTimeRemaining(secondsRemaining)
        }

        console.log(`✓ Successfully uploaded: ${fileName}`)
      } catch (e) {
        failCount++
        uploadedBytes += fileSize // Count failed files too for progress calculation
        uploadProgress.value.failCount = failCount
        uploadProgress.value.uploadedBytes = uploadedBytes
        console.error(`✗ Failed to upload ${filePath}:`, e)
      }
    }

    await appStore.loadObjects()
    console.log(`Upload complete: ${successCount} succeeded, ${failCount} failed`)

    // Keep progress visible for a short time to show completion
    setTimeout(() => {
      uploadProgress.value.isUploading = false
    }, 1500)
  } catch (e) {
    console.error('Upload failed:', e)
    uploadProgress.value.isUploading = false
  }
}

// Context menu functions
function showContextMenu(event: MouseEvent, obj: S3Object) {
  event.preventDefault()
  contextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
    object: obj
  }
}

function closeContextMenu() {
  contextMenu.value.show = false
  showContentTypeSubmenu.value = false
}

function startRename() {
  if (contextMenu.value.object) {
    renamingObject.value = contextMenu.value.object
    newFileName.value = getFileName(contextMenu.value.object.key)
    showRenameModal.value = true
    closeContextMenu()
  }
}

async function renameFileHandler() {
  if (!renamingObject.value || !newFileName.value.trim() || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    renaming.value = true

    const oldKey = renamingObject.value.key
    const path = oldKey.substring(0, oldKey.lastIndexOf('/') + 1)
    const newKey = path + newFileName.value.trim()

    // S3 doesn't have rename, so we copy then delete
    // First, get the object
    const { getObject } = await import('../services/tauri')
    const objectData = await getObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      oldKey
    )

    // Copy to new key
    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      newKey,
      objectData.content,
      objectData.content_type
    )

    // Delete old key
    await deleteObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      oldKey
    )

    // Close modal and refresh
    showRenameModal.value = false
    newFileName.value = ''
    renamingObject.value = null
    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('renameFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  } finally {
    renaming.value = false
  }
}

async function changeContentTypeDirectly(contentType: string) {
  if (!contextMenu.value.object || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    changingContentType.value = true
    const key = contextMenu.value.object.key

    // Get the object content
    const { getObject } = await import('../services/tauri')
    const objectData = await getObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      key
    )

    // Put it back with new content type
    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      key,
      objectData.content,
      contentType
    )

    // Close menus and refresh
    closeContextMenu()
    showContentTypeSubmenu.value = false
    await appStore.loadObjects()

    // Show success message
    toast.success(t('contentTypeUpdated'))
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `Failed to change content type: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  } finally {
    changingContentType.value = false
  }
}

// View object versions
async function viewObjectVersions() {
  if (!contextMenu.value.object || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    versionsObject.value = contextMenu.value.object
    showVersionsModal.value = true
    loadingVersions.value = true
    objectVersions.value = []
    closeContextMenu()

    const response = await listObjectVersions(
      appStore.currentProfile.id,
      appStore.currentBucket,
      contextMenu.value.object.key
    )

    objectVersions.value = response.versions
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `Failed to load versions: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  } finally {
    loadingVersions.value = false
  }
}

// Download a specific version of an object
async function downloadObjectVersion(version: ObjectVersion) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    const fileName = getFileName(version.key)
    const filePath = await save({
      defaultPath: fileName,
      filters: []
    })

    if (!filePath) return

    // Note: You may need to update the backend to support downloading specific versions
    // For now, this downloads the object with the version ID
    // The backend's getObject function would need to accept an optional versionId parameter
    const { getObject } = await import('../services/tauri')
    const response = await getObject(appStore.currentProfile.id, appStore.currentBucket, version.key)

    await writeBinaryFile(filePath, new Uint8Array(response.content))
    toast.success(t('fileDownloadedSuccess'))
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('downloadFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

// Empty area context menu functions
function showEmptyContextMenu(event: MouseEvent) {
  emptyContextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY
  }
}

function closeEmptyContextMenu() {
  emptyContextMenu.value.show = false
}

function closeAllContextMenus() {
  closeContextMenu()
  closeEmptyContextMenu()
}

function openCreateFileModal() {
  showCreateFileModal.value = true
  closeEmptyContextMenu()
}

function openCreateFolderModalFromContext() {
  showCreateFolderModal.value = true
  closeEmptyContextMenu()
}

async function createFileHandler() {
  if (!newFileName.value.trim() || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    creatingFile.value = true

    const filePath = appStore.currentPrefix + newFileName.value.trim()
    const content = newFileContent.value || ''
    const encoder = new TextEncoder()
    const bytes = Array.from(encoder.encode(content))

    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      filePath,
      bytes,
      'text/plain'
    )

    // Close modal and reset
    showCreateFileModal.value = false
    newFileName.value = ''
    newFileContent.value = ''
    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('createFileFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  } finally {
    creatingFile.value = false
  }
}

// Copy/Paste functions
function copyFile() {
  if (contextMenu.value.object) {
    copiedFile.value = contextMenu.value.object
    closeContextMenu()
  }
}

async function pasteFile() {
  if (!copiedFile.value || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    pasting.value = true
    closeEmptyContextMenu()

    const sourceKey = copiedFile.value.key
    const fileName = getFileName(sourceKey)
    const destKey = appStore.currentPrefix + fileName

    // Check if destination already exists
    const existingFile = appStore.objects.find(obj => obj.key === destKey)
    if (existingFile) {
      const confirmed = await dialog.confirm({
        title: t('paste'),
        message: t('fileExistsConfirm').replace('{0}', fileName),
        confirmText: t('paste'),
        cancelText: t('cancel'),
        variant: 'destructive'
      })

      if (!confirmed) {
        return
      }
    }

    // Copy the file to the current location
    await copyObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      sourceKey,
      appStore.currentBucket,
      destKey
    )

    await appStore.loadObjects()
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('pasteFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  } finally {
    pasting.value = false
  }
}

// Selection functions
function handleFileClick(event: MouseEvent, obj: S3Object, index: number) {
  if (event.ctrlKey || event.metaKey) {
    // Ctrl/Cmd+Click: Toggle selection
    if (selectedItems.value.has(obj.key)) {
      selectedItems.value.delete(obj.key)
    } else {
      selectedItems.value.add(obj.key)
    }
    lastSelectedIndex.value = index
  } else if (event.shiftKey && lastSelectedIndex.value !== -1) {
    // Shift+Click: Select range
    const start = Math.min(lastSelectedIndex.value, index)
    const end = Math.max(lastSelectedIndex.value, index)

    const allItems = [...filteredFolders.value, ...filteredObjects.value.map(o => o.key)]
    for (let i = start; i <= end; i++) {
      selectedItems.value.add(allItems[i])
    }
  } else {
    // Normal click: Clear selection
    selectedItems.value.clear()
    lastSelectedIndex.value = index
  }
}

function handleFolderClick(event: MouseEvent, folder: string, index: number) {
  if (event.ctrlKey || event.metaKey) {
    // Ctrl/Cmd+Click: Toggle selection
    if (selectedItems.value.has(folder)) {
      selectedItems.value.delete(folder)
    } else {
      selectedItems.value.add(folder)
    }
    lastSelectedIndex.value = index
  } else if (event.shiftKey && lastSelectedIndex.value !== -1) {
    // Shift+Click: Select range
    const start = Math.min(lastSelectedIndex.value, index)
    const end = Math.max(lastSelectedIndex.value, index)

    const allItems = [...filteredFolders.value, ...filteredObjects.value.map(o => o.key)]
    for (let i = start; i <= end; i++) {
      selectedItems.value.add(allItems[i])
    }
  } else {
    // Normal click: Clear selection
    selectedItems.value.clear()
    lastSelectedIndex.value = index
  }
}

function clearSelection() {
  selectedItems.value.clear()
  lastSelectedIndex.value = -1
}

async function copySelectedItems() {
  // For now, only copy the first selected file (folders cannot be copied individually)
  const selectedFiles = Array.from(selectedItems.value).filter(key =>
    appStore.objects.some(obj => obj.key === key)
  )

  if (selectedFiles.length === 0) {
    await dialog.confirm({
      title: t('copy'),
      message: t('cannotCopyFolders'),
      confirmText: t('close')
    })
    return
  }

  if (selectedFiles.length > 1) {
    await dialog.confirm({
      title: t('copy'),
      message: t('multiCopyNotSupported'),
      confirmText: t('close')
    })
    return
  }

  const fileObj = appStore.objects.find(obj => obj.key === selectedFiles[0])
  if (fileObj) {
    copiedFile.value = fileObj
    clearSelection()
  }
}

async function deleteSelectedItems() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const selectedCount = selectedItems.value.size
  const confirmed = await dialog.confirm({
    title: t('delete'),
    message: t('deleteItemsConfirm')
      .replace('{0}', String(selectedCount))
      .replace('{1}', selectedCount !== 1 ? 's' : ''),
    confirmText: t('delete'),
    cancelText: t('cancel'),
    variant: 'destructive'
  })

  if (!confirmed) return

  try {
    let successCount = 0
    let failCount = 0

    for (const key of selectedItems.value) {
      try {
        // Check if it's a folder or a file
        const isFolder = filteredFolders.value.includes(key)

        if (isFolder) {
          await deleteFolder(appStore.currentProfile.id, appStore.currentBucket, key)
        } else {
          await deleteObject(appStore.currentProfile.id, appStore.currentBucket, key)
        }
        successCount++
      } catch (e) {
        console.error(`Failed to delete ${key}:`, e)
        failCount++
      }
    }

    clearSelection()
    await appStore.loadObjects()

    if (failCount === 0) {
      toast.success(t('deleteSuccess')
        .replace('{0}', String(successCount))
        .replace('{1}', successCount > 1 ? 's' : ''))
    } else {
      toast.warning(t('deletePartialSuccess')
        .replace('{0}', String(successCount))
        .replace('{1}', String(failCount)))
    }
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('deleteOperationFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

async function downloadSelectedItems() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  // Filter to get only files (exclude folders)
  const selectedFiles = Array.from(selectedItems.value).filter(key =>
    appStore.objects.some(obj => obj.key === key)
  )

  if (selectedFiles.length === 0) {
    toast.warning(t('noFilesToDownload'))
    return
  }

  try {
    // Ask user to select a folder to save all files
    const folderPath = await save({
      defaultPath: 'downloads',
      title: t('selectDownloadFolder')
    })

    if (!folderPath) return

    // Get the directory path (remove the filename if any)
    const directory = folderPath.includes('/')
      ? folderPath.substring(0, folderPath.lastIndexOf('/'))
      : folderPath.includes('\\')
      ? folderPath.substring(0, folderPath.lastIndexOf('\\'))
      : folderPath

    const { getObject } = await import('../services/tauri')
    let successCount = 0
    let failCount = 0

    // Create a persistent progress toast (duration = 0 means it won't auto-dismiss)
    const progressToastId = toast.info(
      `${t('downloading')} 0/${selectedFiles.length}`,
      0
    )

    for (const key of selectedFiles) {
      try {
        const fileName = getFileName(key)
        const filePath = `${directory}/${fileName}`

        const response = await getObject(appStore.currentProfile.id, appStore.currentBucket, key)
        await writeBinaryFile(filePath, new Uint8Array(response.content))
        successCount++

        // Update progress toast
        const totalProcessed = successCount + failCount
        toast.updateToast(
          progressToastId,
          `${t('downloading')} ${totalProcessed}/${selectedFiles.length}`
        )
      } catch (e) {
        console.error(`Failed to download ${key}:`, e)
        failCount++

        // Update progress toast even on failure
        const totalProcessed = successCount + failCount
        toast.updateToast(
          progressToastId,
          `${t('downloading')} ${totalProcessed}/${selectedFiles.length}`
        )
      }
    }

    // Remove the progress toast
    toast.removeToast(progressToastId)

    clearSelection()

    // Show final result toast
    if (failCount === 0) {
      toast.success(t('filesDownloadedSuccess')
        .replace('{0}', String(successCount)))
    } else {
      toast.warning(t('downloadPartialSuccess')
        .replace('{0}', String(successCount))
        .replace('{1}', String(failCount)))
    }
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('downloadFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive'
    })
  }
}

// Select all items
function selectAllItems() {
  selectedItems.value.clear()

  // Add all folders
  filteredFolders.value.forEach(folder => {
    selectedItems.value.add(folder)
  })

  // Add all files
  filteredObjects.value.forEach(obj => {
    selectedItems.value.add(obj.key)
  })
}

// Keyboard event handler
function handleKeyDown(event: KeyboardEvent) {
  // Check if the active element is an input, textarea, or contentEditable
  const activeElement = document.activeElement
  const isInputField = activeElement instanceof HTMLInputElement ||
                       activeElement instanceof HTMLTextAreaElement ||
                       (activeElement as HTMLElement)?.isContentEditable

  // Check for Cmd+A (macOS) or Ctrl+A (other OS)
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
    // Only intercept if not in an input field
    if (!isInputField) {
      // Prevent default browser behavior
      event.preventDefault()
      selectAllItems()
    }
  }
  // Check for Escape key to clear selection
  else if (event.key === 'Escape') {
    // Allow Escape to work in input fields (they handle it themselves)
    // But also clear selection
    if (!isInputField) {
      clearSelection()
    }
  }
}

// Setup event listeners on mount
onMounted(async () => {
  // Add keyboard event listener
  window.addEventListener('keydown', handleKeyDown)
  unlistenFileDrop = await listen('tauri://file-drop', (event) => {
    isDraggingOver.value = false
    const paths = event.payload as string[]
    handleFileDrop(paths)
  })

  unlistenFileDropHover = await listen('tauri://file-drop-hover', () => {
    isDraggingOver.value = true
  })

  unlistenFileDropCancelled = await listen('tauri://file-drop-cancelled', () => {
    isDraggingOver.value = false
  })
})

// Cleanup event listeners on unmount
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
  if (unlistenFileDrop) unlistenFileDrop()
  if (unlistenFileDropHover) unlistenFileDropHover()
  if (unlistenFileDropCancelled) unlistenFileDropCancelled()
})
</script>
