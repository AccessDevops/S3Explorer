<template>
  <div
    class="relative flex flex-col h-full bg-background"
    :class="{ 'bg-blue-50': isDraggingOver }"
  >
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
        <Button
          size="sm"
          variant="ghost"
          @click="appStore.goBack()"
          :disabled="!appStore.canGoBack"
          :title="t('goBack')"
          class="mr-2 px-2"
        >
          <PhCaretLeft :size="18" />
        </Button>
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

      <div class="relative flex-1 max-w-md">
        <Input
          ref="searchInputRef"
          v-model="searchQuery"
          :placeholder="t('searchFilesAndFolders')"
          class="w-full pr-20"
        />

        <!-- Index button (left of search mode button) -->
        <div class="absolute right-12 top-1/2 -translate-y-1/2" ref="indexButtonContainerRef">
          <IndexButton
            v-if="appStore.currentProfile && appStore.currentBucket"
            :profileId="appStore.currentProfile.id"
            :bucketName="appStore.currentBucket"
            @indexChanged="handleIndexChanged"
          />
        </div>

        <!-- Settings button -->
        <button
          ref="searchSettingsButtonRef"
          @click="toggleSearchSettingsMenu"
          :title="t('searchMode')"
          class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded-md hover:bg-muted transition-colors"
          :class="showSearchSettings ? 'bg-muted text-primary' : 'text-muted-foreground'"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path
              d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
            />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </button>

        <!-- Search progress bar -->
        <div
          v-if="searchQuery.trim()"
          class="absolute top-full left-0 right-0 mt-1 bg-card border rounded-md shadow-sm overflow-hidden z-10"
        >
          <div class="flex items-center justify-between px-3 py-2 text-sm">
            <!-- INDEX SEARCH MODE -->
            <div v-if="useIndexForSearch" class="flex items-center gap-3">
              <div class="flex items-center gap-2">
                <span class="text-green-600 dark:text-green-400 font-medium">⚡ {{ t('indexSearch') }}</span>
                <span class="font-medium text-primary">{{ searchProgress }} {{ t('found') }}</span>
              </div>
              <div class="text-xs text-muted-foreground">
                {{ t('instant') }} ({{ searchDuration }}ms)
              </div>
            </div>

            <!-- LIVE SEARCH MODE -->
            <div v-else class="flex items-center gap-3">
              <div class="flex items-center gap-2">
                <span v-if="isSearching" class="text-muted-foreground">{{ t('searching') }}...</span>
                <span v-else class="text-green-600 dark:text-green-400 font-medium">✓ {{ t('searchComplete') }}</span>
                <span class="font-medium text-primary">{{ searchProgress }} {{ t('found') }}</span>
              </div>
              <div v-if="isSearching" class="text-xs text-muted-foreground">
                {{ t('pagesScanned', searchPagesScanned) }}
              </div>
              <!-- Search Speed (only during search) -->
              <div v-if="isSearching && searchSpeed > 0" class="text-xs text-muted-foreground">
                {{ searchSpeed.toLocaleString() }} obj/s
              </div>
              <!-- Time Remaining (only during search) -->
              <div
                v-if="isSearching && searchTimeRemaining > 0"
                class="text-xs text-muted-foreground"
              >
                ~{{ formatTime(searchTimeRemaining) }} {{ t('remaining') }}
              </div>
              <!-- Final stats (when search is complete) -->
              <div v-if="!isSearching && searchPagesScanned > 0" class="text-xs text-muted-foreground">
                {{ t('scannedPages', searchPagesScanned) }}
              </div>
            </div>
            <Button v-if="isSearching" size="sm" variant="ghost" @click="stopSearch" :title="t('stopSearch')">
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
                <rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
              </svg>
            </Button>
          </div>
          <!-- Animated progress bar (only during active search) -->
          <div v-if="isSearching" class="h-1 bg-muted overflow-hidden">
            <div class="h-full bg-primary animate-progress-indeterminate"></div>
          </div>
        </div>
      </div>

      <div class="flex gap-2 flex-shrink-0">
        <Button size="sm" variant="outline" @click="uploadFilesHandler">{{
          t('upload')
        }}</Button>
        <Button size="sm" variant="outline" @click="modals.createFolder = true">{{
          t('newFolder')
        }}</Button>
        <Button size="sm" variant="ghost" @click="reloadAllPages()" :title="t('refresh')"
          >⟳</Button
        >
      </div>
    </div>

    <!-- Search Settings Dropdown Menu -->
    <div
      v-if="showSearchSettings"
      :style="searchSettingsMenuStyle"
      class="fixed z-50 min-w-[220px] bg-popover text-popover-foreground rounded-md border shadow-lg"
      @click.stop
    >
      <div class="p-3">
        <div class="text-sm font-medium mb-2 text-foreground">{{ t('searchMode') }}</div>
        <div class="flex flex-col gap-1">
          <button
            @click="selectSearchMode('local')"
            class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer flex items-center gap-2 transition-colors"
            :class="settingsStore.searchMode === 'local' ? 'bg-accent text-accent-foreground' : ''"
          >
            <span>📁</span>
            <span class="flex-1">{{ t('searchLocal') }}</span>
            <span v-if="settingsStore.searchMode === 'local'" class="text-primary">✓</span>
          </button>
          <button
            @click="selectSearchMode('global')"
            class="w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent hover:text-accent-foreground cursor-pointer flex items-center gap-2 transition-colors"
            :class="settingsStore.searchMode === 'global' ? 'bg-accent text-accent-foreground' : ''"
          >
            <span>🌍</span>
            <span class="flex-1">{{ t('searchGlobal') }}</span>
            <span v-if="settingsStore.searchMode === 'global'" class="text-primary">✓</span>
          </button>
        </div>
        <p class="text-xs text-muted-foreground mt-2 px-3">
          {{
            settingsStore.searchMode === 'local'
              ? t('localSearchDescription')
              : t('globalSearchDescription')
          }}
        </p>
      </div>
    </div>

    <!-- Object List -->
    <div
      ref="objectListRef"
      class="flex-1 overflow-y-auto px-4 pb-4 relative"
      @contextmenu.prevent="showEmptyContextMenu($event)"
      @click="clearSelection"
      @mousedown="handleSelectionMouseDown"
      @mousemove="handleSelectionMouseMove"
      @mouseup="handleSelectionMouseUp"
      @mouseleave="handleSelectionMouseUp"
      @scroll="virtualScroll.handleScroll"
    >
      <!-- Selection box -->
      <div :style="selectionBoxStyle"></div>

      <div @click.stop>
        <!-- Table Header -->
        <div
          class="flex items-center gap-3 px-3 py-2 border-b border-border bg-muted/50 rounded-t-md sticky top-0 z-20 backdrop-blur-sm"
        >
          <div class="flex-shrink-0 w-6"></div>
          <!-- Icon space -->
          <button
            @click="handleSort('name')"
            class="flex-1 min-w-0 flex items-center gap-2 hover:text-primary transition-colors cursor-pointer text-left text-[11px] text-muted-foreground font-medium"
          >
            <span>{{ t('fileName') }}</span>
            <span v-if="sortBy === 'name'" class="text-[10px]">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </button>
          <button
            @click="handleSort('size')"
            class="w-24 flex items-center justify-end gap-2 hover:text-primary transition-colors cursor-pointer text-[11px] text-muted-foreground font-medium"
          >
            <span>{{ t('size') }}</span>
            <span v-if="sortBy === 'size'" class="text-[10px]">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </button>
          <button
            @click="handleSort('date')"
            class="w-40 flex items-center justify-end gap-2 hover:text-primary transition-colors cursor-pointer text-[11px] text-muted-foreground font-medium"
          >
            <span>{{ t('lastModified') }}</span>
            <span v-if="sortBy === 'date'" class="text-[10px]">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </button>
          <div class="w-20"></div>
          <!-- Actions space -->
        </div>

        <!-- Virtual scroll container (spacer maintains total scroll height) -->
        <div :style="virtualScroll.spacerStyle.value">
          <!-- Virtual scroll content wrapper (applies offset for visible items) -->
          <div :style="virtualScroll.contentStyle.value">
            <!-- Folders (virtualized) -->
            <div
              v-for="(folder, index) in virtualVisibleFolders"
              :key="folder"
          :class="[
            'flex items-center rounded-md hover:bg-accent transition-colors cursor-pointer group select-none relative z-0',
            rowPadding,
            rowGap,
            selectedItems.has(folder) ? 'bg-primary/20 hover:bg-primary/30' : '',
            isDraggingFile && draggingFolder === folder ? 'opacity-50' : '',
          ]"
          :draggable="true"
          data-object-row
          :data-object-key="folder"
          @click="handleFolderClick($event, folder, index)"
          @dblclick="navigateToFolder(folder)"
          @contextmenu.stop
          @dragstart="handleFolderDragStart($event, folder)"
          @dragend="handleFolderDragEnd($event, folder)"
        >
          <!-- Spacer to align with file version arrow -->
          <div class="flex-shrink-0 w-4"></div>

          <div class="flex-shrink-0">
            <PhFolder :size="iconSize" class="text-yellow-500" weight="duotone" />
          </div>
          <div class="flex-1 min-w-0">
            <div class="font-medium truncate" :class="textSize">{{ getFolderName(folder) }}</div>
          </div>
          <div class="text-sm text-muted-foreground w-24 text-right">
            {{ getFolderSize(folder) }}
          </div>
          <div class="text-sm text-muted-foreground w-40 text-right">-</div>
          <div
            class="flex gap-1 transition-opacity w-20 justify-end"
            :class="selectedItems.has(folder) ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'"
          >
            <Button
              size="sm"
              variant="destructive"
              @click.stop="deleteFolderConfirm(folder)"
              :title="t('delete')"
            >
              <PhTrash :size="16" />
            </Button>
          </div>
        </div>

        <!-- Files (virtualized) -->
        <template v-for="(obj, index) in virtualVisibleObjects" :key="obj.key">
          <div
            :class="[
              'flex items-center rounded-md hover:bg-accent transition-colors group cursor-pointer select-none relative',
              showActionsMenu === obj.key ? 'z-[10000]' : 'z-0',
              rowPadding,
              rowGap,
              selectedItems.has(obj.key) ? 'bg-primary/20 hover:bg-primary/30' : '',
              isDraggingFile && draggingObject?.key === obj.key ? 'opacity-50' : '',
            ]"
            :draggable="true"
            data-object-row
            :data-object-key="obj.key"
            @click="handleFileClick($event, obj, index + filteredFolders.length)"
            @dblclick="viewObject(obj)"
            @contextmenu.stop="showContextMenu($event, obj)"
            @dragstart="handleFileDragStart($event, obj)"
            @dragend="handleFileDragEnd($event, obj)"
          >
            <!-- Version expand/collapse arrow -->
            <button
              @click.stop="toggleInlineVersions(obj, $event)"
              :class="[
                'flex-shrink-0 hover:bg-accent rounded transition-opacity',
                expandedVersions.has(obj.key) || loadingInlineVersions.has(obj.key) || hasMultipleVersions(obj) ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
              ]"
              :title="expandedVersions.has(obj.key) ? t('hideVersions') : t('showVersions')"
            >
              <PhCaretDown
                v-if="expandedVersions.has(obj.key)"
                :size="16"
                :class="hasMultipleVersions(obj) ? 'text-blue-500' : 'text-muted-foreground'"
              />
              <PhCaretRight
                v-else-if="!loadingInlineVersions.has(obj.key)"
                :size="16"
                :class="hasMultipleVersions(obj) ? 'text-blue-500' : 'text-muted-foreground'"
              />
              <svg
                v-else
                class="animate-spin text-muted-foreground"
                :width="16"
                :height="16"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
            </button>

            <div class="flex-shrink-0">
              <component :is="getFileIcon(obj.key).icon" :size="iconSize" :class="getFileIcon(obj.key).colorClass" weight="duotone" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="font-medium truncate" :class="textSize">
                {{ searchQuery.trim() ? obj.key : getFileName(obj.key) }}
              </div>
              <div
                v-if="searchQuery.trim() && obj.key.includes('/')"
                class="text-xs text-muted-foreground truncate"
              >
                {{ obj.key.substring(0, obj.key.lastIndexOf('/')) }}/
              </div>
            </div>
            <div class="text-sm text-muted-foreground w-24 flex-shrink-0 text-right tabular-nums">
              {{ formatSize(obj.size) }}
            </div>
            <div class="text-sm text-muted-foreground w-40 flex-shrink-0 text-right tabular-nums">
              {{ formatDate(obj.last_modified) }}
            </div>
            <div
              class="flex gap-1 transition-opacity w-20 flex-shrink-0 justify-end"
              :class="selectedItems.has(obj.key) ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'"
            >
              <!-- Actions menu button -->
              <div class="relative" @mouseleave="showActionsMenu = null">
                <Button
                  size="sm"
                  variant="secondary"
                  @click.stop="showActionsMenu = showActionsMenu === obj.key ? null : obj.key"
                  @mouseenter="showActionsMenu = obj.key"
                >
                  <PhDotsThree :size="16" weight="bold" />
                </Button>

                <!-- Actions dropdown menu -->
                <Transition name="fade">
                  <div
                    v-if="showActionsMenu === obj.key"
                    @click.stop
                    class="absolute right-full top-0 z-[9999]"
                  >
                    <div class="min-w-[160px] rounded-md border bg-card backdrop-blur-sm p-0.5 text-card-foreground shadow-xl">
                      <!-- Download -->
                      <button
                        @click="downloadObject(obj.key)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-accent text-left"
                      >
                        <PhDownloadSimple :size="14" />
                        {{ t('download') }}
                      </button>

                      <!-- View -->
                      <button
                        @click="viewObject(obj)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-accent text-left"
                      >
                        <PhEye :size="14" />
                        {{ t('view') }}
                      </button>

                      <!-- Divider -->
                      <div class="my-0.5 h-px bg-border"></div>

                      <!-- Copy options -->
                      <button
                        @click="copyFullPath(obj.key)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-accent text-left"
                      >
                        <PhLink :size="14" />
                        {{ t('copyFullPath') }}
                      </button>
                      <button
                        @click="copyPath(obj.key)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-accent text-left"
                      >
                        <PhLink :size="14" />
                        {{ t('copyPath') }}
                      </button>
                      <button
                        @click="copyFileName(obj.key)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-accent text-left"
                      >
                        <PhLink :size="14" />
                        {{ t('copyFileName') }}
                      </button>
                      <button
                        @click="copyObjectUrl(obj.key)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-accent text-left"
                      >
                        <PhLink :size="14" />
                        {{ t('copyUrl') }}
                      </button>

                      <!-- Divider -->
                      <div class="my-0.5 h-px bg-border"></div>

                      <!-- Delete -->
                      <button
                        @click="deleteObjectConfirm(obj.key)"
                        class="flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-xs leading-tight hover:bg-destructive hover:text-destructive-foreground text-left"
                      >
                        <PhTrash :size="14" />
                        {{ t('delete') }}
                      </button>
                    </div>
                  </div>
                </Transition>
              </div>
            </div>
          </div>

          <!-- Versions list (expanded inline) -->
          <div
            v-if="expandedVersions.has(obj.key) && inlineVersions.has(obj.key)"
            class="ml-8 border-l-2 border-muted"
          >
            <div
              v-for="version in inlineVersions.get(obj.key)"
              :key="version.version_id"
              :class="[
                'flex items-center hover:bg-accent/50 transition-colors group select-none',
                rowPadding,
                rowGap,
                'ml-4'
              ]"
            >
              <div class="flex-shrink-0 w-4"></div>
              <div class="flex-shrink-0">
                <PhClock :size="iconSize" class="text-muted-foreground" weight="duotone" />
              </div>
              <div class="flex-1 min-w-0">
                <div class="font-medium truncate" :class="textSize">
                  {{ version.version_id.substring(0, 12) }}...
                  <span
                    v-if="version.is_latest"
                    class="ml-2 text-xs bg-primary/20 text-primary px-2 py-0.5 rounded"
                  >
                    {{ t('latest') }}
                  </span>
                </div>
                <div class="text-xs text-muted-foreground truncate">
                  {{ formatDate(version.last_modified) }}
                </div>
              </div>
              <div class="text-sm text-muted-foreground w-24 flex-shrink-0 text-right tabular-nums">
                {{ formatSize(version.size) }}
              </div>
              <div class="w-40 flex-shrink-0"></div>
              <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity w-36 flex-shrink-0 justify-end">
                <Button
                  size="sm"
                  variant="secondary"
                  @click.stop="downloadObjectVersion(version)"
                  :title="t('download')"
                >
                  <PhDownloadSimple :size="16" />
                </Button>
              </div>
            </div>
          </div>
        </template>
          </div>
          <!-- End virtual scroll content wrapper -->
        </div>
        <!-- End virtual scroll container -->

        <!-- Empty State Hint (Background) -->
        <div
          v-if="showEmptyStateHint"
          class="absolute top-32 left-0 right-0 flex flex-col items-center justify-center py-20 px-6 text-center opacity-30 pointer-events-none select-none"
        >
          <div class="relative mb-6">
            <!-- Upload icon with animation -->
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="120"
              height="120"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="text-muted-foreground animate-pulse"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" x2="12" y1="3" y2="15" />
            </svg>
            <!-- Dashed border circle -->
            <div
              class="absolute inset-0 border-4 border-dashed border-muted-foreground rounded-full opacity-30"
              style="animation: rotate 20s linear infinite"
            ></div>
          </div>
          <div class="text-2xl font-semibold text-muted-foreground mb-2">
            {{ t('dragDropHintTitle') }}
          </div>
          <div class="text-base text-muted-foreground max-w-md">
            {{ t('dragDropHintDescription') }}
          </div>
        </div>

        <!-- Searching indicator -->
        <div
          v-if="isSearching && searchQuery.trim()"
          class="text-center py-8 text-muted-foreground"
        >
          <div class="flex items-center justify-center gap-2">
            <svg
              class="animate-spin h-5 w-5"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
            <span class="text-sm">🔍 {{ t('searchingAllObjects') }}...</span>
          </div>
          <div class="text-sm mt-2">
            {{ t('found') }} {{ globalSearchResults.length }} {{ t('objects') }}
          </div>
        </div>

        <!-- Load More / Load All buttons -->
        <div
          v-if="!searchQuery.trim() && appStore.continuationToken && !appStore.isLoading"
          class="p-4 text-center"
        >
          <div class="flex gap-2 justify-center max-w-md mx-auto">
            <Button variant="outline" @click="appStore.loadObjects(true)" class="flex-1">
              {{ t('loadMore') }}
            </Button>
            <Button
              variant="default"
              @click="loadAllObjects"
              class="flex-1"
              :title="t('loadAllObjectsInFolder')"
            >
              {{ t('loadAll') }}
            </Button>
          </div>
          <div class="text-sm text-muted-foreground mt-2">
            {{ t('showing') }} {{ appStore.objects.length }} {{ t('objects') }}
          </div>
        </div>

        <!-- Loading more indicator -->
        <div
          v-if="appStore.isLoading && appStore.objects.length > 0"
          class="text-center py-4 text-muted-foreground"
        >
          <div class="flex items-center justify-center gap-2">
            <svg
              class="animate-spin h-5 w-5"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
            <span class="text-sm">{{ t('loadingMore') }}</span>
          </div>
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
          {{ selectedItems.size }} {{ selectedItems.size !== 1 ? t('items') : t('item') }}
          {{ t('selected') }}
          <span class="text-muted-foreground ml-2">({{ formatSize(selectedTotalSize) }})</span>
        </span>
        <Button size="sm" variant="ghost" @click="clearSelection">{{ t('clear') }}</Button>
      </div>
      <div class="flex gap-2">
        <Button size="sm" variant="secondary" @click="downloadSelectedItems">{{
          t('download')
        }}</Button>
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
            <path d="M3 6h18" />
            <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
            <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
            <line x1="10" x2="10" y1="11" y2="17" />
            <line x1="14" x2="14" y1="11" y2="17" />
          </svg>
          {{ t('delete') }}
        </Button>
      </div>
    </div>

    <!-- Footer Stats -->
    <div
      class="border-t bg-card px-4 py-2 flex items-center justify-between text-sm text-muted-foreground"
    >
      <div>
        <span v-if="searchQuery.trim()">
          🔍 {{ filteredObjects.length }} {{ t('resultsFound') }}
          <span v-if="isSearching" class="text-primary">({{ t('searching') }}...)</span>
        </span>
        <span v-else>
          {{ totalItemsCount }} {{ totalItemsCount !== 1 ? t('items') : t('item') }} ({{
            filteredFolders.length
          }}
          {{ filteredFolders.length !== 1 ? t('folders') : t('folderName') }},
          {{ filteredObjects.length }}
          {{ filteredObjects.length !== 1 ? t('files') : t('fileName') }})
          <span
            v-if="appStore.continuationToken"
            class="text-primary ml-2 cursor-pointer hover:underline"
            @click="loadAllObjects"
            :title="t('clickToLoadAll')"
            >• {{ t('moreAvailable') }}</span
          >
        </span>
      </div>
      <div>{{ t('totalSize') }}: {{ formatSize(totalSize) }}</div>
    </div>

    <!-- Create Folder Modal -->
    <Dialog v-model:open="modals.createFolder">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('createFolder') }}</DialogTitle>
        </DialogHeader>
        <div class="space-y-2">
          <Input
            v-model="folderCreation.name"
            :placeholder="t('folderName')"
            @keyup.enter="createFolderHandler"
            :class="folderCreation.validationError ? 'border-red-500' : ''"
          />
          <p v-if="folderCreation.validationError" class="text-xs text-red-600">
            {{ folderCreation.validationError }}
          </p>
        </div>
        <DialogFooter>
          <Button
            @click="createFolderHandler"
            :disabled="!folderCreation.name.trim() || !!folderCreation.validationError"
          >
            {{ t('create') }}
          </Button>
          <Button variant="secondary" @click="modals.createFolder = false">{{
            t('cancel')
          }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- View Object Modal -->
    <Dialog v-model:open="showViewModal">
      <DialogContent class="max-w-4xl max-h-[90vh]">
        <DialogHeader>
          <DialogTitle class="flex items-center gap-2">
            <span>{{ viewingObject ? getFileName(viewingObject.key) : '' }}</span>
            <span
              v-if="objectViewerRef?.contentType"
              class="text-xs font-normal text-muted-foreground px-2 py-1 bg-muted rounded"
            >
              {{ objectViewerRef.contentType }}
            </span>
          </DialogTitle>
        </DialogHeader>

        <Tabs default-value="content" class="w-full">
          <TabsList>
            <TabsTrigger value="content">{{ t('content') }}</TabsTrigger>
            <TabsTrigger value="metadata">{{ t('metadata') }}</TabsTrigger>
            <TabsTrigger value="versions">{{ t('versions') }}</TabsTrigger>
            <TabsTrigger value="permissions">{{ t('permissions') }}</TabsTrigger>
            <TabsTrigger value="tags">{{ t('tags') }}</TabsTrigger>
            <TabsTrigger value="events">{{ t('eventLog') }}</TabsTrigger>
          </TabsList>

          <TabsContent value="content">
            <div class="overflow-y-auto max-h-[60vh]">
              <ObjectViewer v-if="viewingObject" ref="objectViewerRef" :object="viewingObject" />
            </div>
            <div class="mt-4 flex gap-2">
              <template v-if="objectViewerRef?.isText">
                <Button v-if="!objectViewerRef?.isEditing" @click="objectViewerRef?.startEditing()">
                  {{ t('edit') }}
                </Button>
                <template v-else>
                  <Button
                    @click="objectViewerRef?.saveChanges()"
                    :disabled="objectViewerRef?.saving"
                  >
                    {{ objectViewerRef?.saving ? t('saving') : t('save') }}
                  </Button>
                  <Button variant="outline" @click="objectViewerRef?.cancelEditing()">
                    {{ t('cancel') }}
                  </Button>
                </template>
              </template>
            </div>
          </TabsContent>

          <TabsContent value="metadata">
            <div class="overflow-y-auto max-h-[60vh]">
              <div v-if="viewingObject" class="space-y-3">
                <div class="grid grid-cols-2 gap-2 text-sm">
                  <div class="font-medium text-muted-foreground">{{ t('key') }}:</div>
                  <div class="font-mono text-xs break-all">{{ viewingObject.key }}</div>

                  <div class="font-medium text-muted-foreground">{{ t('size') }}:</div>
                  <div>{{ formatSize(viewingObject.size) }}</div>

                  <div class="font-medium text-muted-foreground">{{ t('lastModified') }}:</div>
                  <div>{{ formatDate(viewingObject.last_modified) }}</div>

                  <div v-if="viewingObject.storage_class" class="font-medium text-muted-foreground">
                    {{ t('storageClass') }}:
                  </div>
                  <div v-if="viewingObject.storage_class">{{ viewingObject.storage_class }}</div>

                  <div v-if="viewingObject.e_tag" class="font-medium text-muted-foreground">
                    {{ t('etag') }}:
                  </div>
                  <div v-if="viewingObject.e_tag" class="font-mono text-xs break-all">
                    {{ viewingObject.e_tag }}
                  </div>

                  <div
                    v-if="objectViewerRef?.contentType"
                    class="font-medium text-muted-foreground"
                  >
                    {{ t('contentType') }}:
                  </div>
                  <div v-if="objectViewerRef?.contentType">{{ objectViewerRef.contentType }}</div>
                </div>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="versions">
            <div class="overflow-y-auto max-h-[60vh]">
              <div v-if="viewModalVersions.length > 0" class="space-y-2">
                <div
                  v-for="version in viewModalVersions"
                  :key="version.version_id"
                  class="flex items-center justify-between p-3 border rounded-md hover:bg-accent"
                >
                  <div class="flex-1">
                    <div class="flex items-center gap-2">
                      <span class="font-mono text-xs text-muted-foreground">{{
                        version.version_id
                      }}</span>
                      <span
                        v-if="version.is_latest"
                        class="text-xs px-2 py-0.5 bg-primary/20 text-primary rounded"
                      >
                        {{ t('latest') }}
                      </span>
                    </div>
                    <div class="text-sm text-muted-foreground mt-1">
                      {{ formatSize(version.size) }} • {{ formatDate(version.last_modified) }}
                    </div>
                  </div>
                  <Button size="sm" variant="secondary" @click="downloadObjectVersion(version)">
                    {{ t('download') }}
                  </Button>
                </div>
              </div>
              <div v-else class="text-center py-8 text-muted-foreground">
                <p>{{ t('versioningNotEnabled') }}</p>
                <Button size="sm" variant="outline" class="mt-4" @click="loadViewModalVersions">
                  {{ t('refresh') }}
                </Button>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="permissions">
            <div class="overflow-y-auto max-h-[60vh]">
              <div class="text-center py-8 text-muted-foreground">
                <p>{{ t('noPermissions') }}</p>
                <p class="text-xs mt-2">{{ t('acl') }}</p>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="tags">
            <div class="overflow-y-auto max-h-[60vh]">
              <div class="text-center py-8 text-muted-foreground">
                <p>{{ t('noTags') }}</p>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="events">
            <div class="overflow-y-auto max-h-[60vh]">
              <div class="text-center py-8 text-muted-foreground">
                <p>{{ t('noEvents') }}</p>
              </div>
            </div>
          </TabsContent>
        </Tabs>

        <DialogFooter>
          <Button variant="secondary" @click="showViewModal = false">{{ t('close') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Context Menus -->
    <ContextMenu
      :show="contextMenu.show"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :target-object="contextMenu.object"
      :show-empty="emptyContextMenu.show"
      :empty-x="emptyContextMenu.x"
      :empty-y="emptyContextMenu.y"
      :has-copied-file="!!copiedFile"
      :icon-size="iconSize"
      :text-size="textSize"
      :is-compact-view="isCompactView"
      @copy="copyFile"
      @rename="startRename"
      @view-versions="viewObjectVersions"
      @change-content-type="changeContentTypeDirectly"
      @paste="pasteFile"
      @new-file="openCreateFileModal"
      @new-folder="openCreateFolderModalFromContext"
    />

    <!-- Rename Modal -->
    <Dialog v-model:open="modals.rename">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('renameFile') }}</DialogTitle>
        </DialogHeader>
        <div class="space-y-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('newName') }}</label>
            <Input
              v-model="rename.newName"
              :placeholder="t('enterNewFileName')"
              @keyup.enter="renameFileHandler"
              :class="rename.validationError ? 'border-red-500' : ''"
            />
            <p v-if="rename.validationError" class="text-xs text-red-600">
              {{ rename.validationError }}
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="modals.rename = false">{{ t('cancel') }}</Button>
          <Button
            @click="renameFileHandler"
            :disabled="!rename.newName.trim() || !!rename.validationError || rename.isRenaming"
          >
            {{ rename.isRenaming ? t('renaming') : t('rename') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Create File Modal -->
    <Dialog v-model:open="modals.createFile">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ t('createNewFile') }}</DialogTitle>
        </DialogHeader>
        <div class="space-y-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('fileName') }}</label>
            <Input
              v-model="fileCreation.name"
              placeholder="filename.txt"
              @keyup.enter="createFileHandler"
              :class="fileCreation.validationError ? 'border-red-500' : ''"
            />
            <p v-if="fileCreation.validationError" class="text-xs text-red-600">
              {{ fileCreation.validationError }}
            </p>
          </div>
          <div>
            <label class="text-sm font-medium">{{ t('contentOptional') }}</label>
            <textarea
              v-model="fileCreation.content"
              :placeholder="t('fileContentPlaceholder')"
              class="w-full min-h-[120px] p-3 text-sm font-mono border rounded-md resize-y bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            ></textarea>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="modals.createFile = false">{{ t('cancel') }}</Button>
          <Button
            @click="createFileHandler"
            :disabled="!fileCreation.name.trim() || !!fileCreation.validationError || fileCreation.isCreating"
          >
            {{ fileCreation.isCreating ? t('creating') : t('create') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Versions Modal -->
    <Dialog v-model:open="showVersionsModal">
      <DialogContent class="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle
            >{{ t('versions') }} -
            {{ versionsObject ? getFileName(versionsObject.key) : '' }}</DialogTitle
          >
        </DialogHeader>

        <div v-if="loadingVersions" class="flex justify-center py-12">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
        </div>

        <div
          v-else-if="objectVersions.length === 0"
          class="text-center py-12 text-muted-foreground"
        >
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
                <span
                  v-if="version.is_latest"
                  class="px-2 py-0.5 text-xs font-semibold bg-primary text-primary-foreground rounded"
                >
                  {{ t('latest') }}
                </span>
              </div>
              <div class="flex gap-4 text-xs text-muted-foreground">
                <span>{{ formatSize(version.size) }}</span>
                <span v-if="version.last_modified">{{ formatDate(version.last_modified) }}</span>
              </div>
            </div>
            <Button
              size="sm"
              variant="secondary"
              @click="downloadObjectVersion(version)"
              :title="t('download')"
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
                class="mr-1"
              >
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" x2="12" y1="15" y2="3" />
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

    <!-- Loading Progress Bar -->
    <Transition
      enter-active-class="transition-opacity duration-200"
      leave-active-class="transition-opacity duration-300"
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="appStore.loadingProgress.show"
        class="absolute bottom-[38px] left-0 right-0 bg-primary/10 backdrop-blur-sm border-t border-primary/20"
      >
        <div class="relative h-1 bg-primary/20 overflow-hidden">
          <div
            class="absolute inset-0 bg-gradient-to-r from-primary/80 via-primary to-primary/80 animate-progress-bar"
          />
        </div>
        <div class="px-4 py-1.5 text-xs text-center text-primary font-medium">
          {{ appStore.loadingProgress.message }}
        </div>
      </div>
    </Transition>

    <!-- Index Build Prompt Popover (Non-blocking) -->
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-1"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-1"
    >
      <Card
        v-if="showIndexBuildPrompt"
        class="fixed z-50 w-96 shadow-lg border-2 border-primary/20"
        :style="indexPopoverStyle"
      >
        <div class="p-4 space-y-4">
          <div class="flex items-start justify-between gap-2">
            <div class="flex items-start gap-3 flex-1">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-blue-500 flex-shrink-0 mt-0.5"
              >
                <circle cx="12" cy="12" r="10" />
                <path d="M12 16v-4" />
                <path d="M12 8h.01" />
              </svg>
              <div class="flex-1">
                <h4 class="font-semibold text-sm mb-1">{{ t('buildSearchIndex') }}?</h4>
                <p class="text-xs text-muted-foreground">
                  {{
                    indexPromptObjectCount === -1
                      ? t('buildIndexLargeBucket')
                      : t('buildIndexPromptMessage', indexPromptObjectCount.toLocaleString())
                  }}
                </p>
                <p class="text-xs text-muted-foreground mt-2">
                  {{ t('estimatedIndexSize') }}: <span class="font-medium">{{ indexPromptEstimatedSize }}</span>
                </p>
              </div>
            </div>
            <button
              @click="showIndexBuildPrompt = false"
              class="text-muted-foreground hover:text-foreground transition-colors p-1 rounded-md hover:bg-muted"
              :title="t('close')"
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
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
          <div class="flex gap-2">
            <Button variant="outline" size="sm" @click="showIndexBuildPrompt = false" class="flex-1">
              {{ t('skip') }}
            </Button>
            <Button size="sm" @click="handleBuildIndexFromPrompt" class="flex-1">
              {{ t('buildIndex') }}
            </Button>
          </div>
        </div>
      </Card>
    </Transition>

    <!-- Index Update Prompt Popover (Non-blocking) -->
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-1"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-1"
    >
      <Card
        v-if="showIndexUpdatePrompt"
        class="fixed z-50 w-96 shadow-lg border-2 border-orange-500/20"
        :style="indexPopoverStyle"
      >
        <div class="p-4 space-y-4">
          <div class="flex items-start justify-between gap-2">
            <div class="flex items-start gap-3 flex-1">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="text-orange-500 flex-shrink-0 mt-0.5"
              >
                <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                <path d="M3 3v5h5" />
                <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                <path d="M16 16h5v5" />
              </svg>
              <div class="flex-1">
                <h4 class="font-semibold text-sm mb-1">{{ t('updateSearchIndex') }}?</h4>
                <p class="text-xs text-muted-foreground">
                  {{ t('indexExpiredMessage') }}
                </p>
                <div class="text-xs text-muted-foreground mt-2 space-y-1">
                  <p>
                    {{ t('indexedObjects') }}: <span class="font-medium">{{ indexUpdateIndexCount.toLocaleString() }}</span>
                  </p>
                  <p>
                    {{ t('currentObjects') }}:
                    <span class="font-medium">
                      {{ indexUpdateCurrentCount === -1 ? `> ${settingsStore.indexAutoBuildThreshold}` : indexUpdateCurrentCount.toLocaleString() }}
                    </span>
                  </p>
                  <p v-if="indexUpdateObjectDiff !== 0" :class="indexUpdateObjectDiff > 0 ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'">
                    {{ t('difference') }}: <span class="font-medium">{{ indexUpdateObjectDiff > 0 ? '+' : '' }}{{ indexUpdateObjectDiff.toLocaleString() }}</span>
                  </p>
                </div>
              </div>
            </div>
            <button
              @click="showIndexUpdatePrompt = false"
              class="text-muted-foreground hover:text-foreground transition-colors p-1 rounded-md hover:bg-muted"
              :title="t('close')"
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
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
          <div class="flex gap-2">
            <Button variant="outline" size="sm" @click="showIndexUpdatePrompt = false" class="flex-1">
              {{ t('later') }}
            </Button>
            <Button size="sm" @click="handleUpdateIndexFromPrompt" class="flex-1">
              {{ t('updateIndex') }}
            </Button>
          </div>
        </div>
      </Card>
    </Transition>

    <!-- Upload progress popup -->
    <RustUploadProgress />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, reactive } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '../stores/app'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from '../composables/useI18n'
import { useDialog } from '../composables/useDialog'
import { useToast } from '../composables/useToast'
import { useSwipeBack } from '../composables/useSwipeBack'
import { useSearchIndex } from '../composables/useSearchIndex'
import { useVirtualScroll } from '../composables/useVirtualScroll'
import { formatSize, formatDate, formatTime } from '../utils/formatters'
import { logger } from '../utils/logger'
import { validateObjectKey } from '../utils/validators'
import { useRustUploadManager } from '../composables/useRustUploadManager'
import {
  createFolder as createFolderService,
  deleteObject,
  calculateFolderSize,
  deleteFolder,
  putObject,
  listObjects,
  copyObject,
  listObjectVersions,
} from '../services/tauri'
import { save, open } from '@tauri-apps/api/dialog'
import { writeBinaryFile } from '@tauri-apps/api/fs'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { S3Object, ObjectVersion } from '../types'
import ObjectViewer from './ObjectViewer.vue'
import ContextMenu from './ContextMenu.vue'
import IndexButton from './IndexButton.vue'
import RustUploadProgress from './RustUploadProgress.vue'
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
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import {
  PhFolder,
  PhImage,
  PhImages,
  PhFilmSlate,
  PhVideo,
  PhVideoCamera,
  PhMusicNote,
  PhMusicNotes,
  PhSpeakerHigh,
  PhWaveform,
  PhVinylRecord,
  PhFileText,
  PhFile,
  PhFileCode,
  PhFileZip,
  PhMicrosoftExcelLogo,
  PhMicrosoftPowerpointLogo,
  PhFilePdf,
  PhFileJs,
  PhFilePy,
  PhFileJsx,
  PhFileTsx,
  PhFileVue,
  PhFileHtml,
  PhFileCss,
  PhGlobe,
  PhGear,
  PhLightning,
  PhMicrosoftWordLogo,
  PhFileCsv,
  PhFileSql,
  PhTerminal,
  PhPackage,
  PhFileRs,
  PhFileTs,
  PhFileC,
  PhFileCpp,
  PhFilePng,
  PhFileJpg,
  PhImageSquare,
  PhFrameCorners,
  PhArticle,
  PhNote,
  PhBookOpen,
  PhMarkdownLogo,
  PhGitBranch,
  PhCodeBlock,
  PhBracketsCurly,
  PhDatabase,
  PhArchive,
  PhCube,
  PhCubeTransparent,
  PhBinary,
  PhLock,
  PhKey,
  PhShieldCheck,
  PhBug,
  PhChartBar,
  PhChartLine,
  PhFileIni,
  PhFileLock,
  PhFileX,
  PhListBullets,
  PhTable,
  PhFileArrowDown,
  PhAppleLogo,
  PhLinuxLogo,
  PhAndroidLogo,
  PhGithubLogo,
  PhGitlabLogo,
  PhFigmaLogo,
  PhSketchLogo,
  PhAtom,
  PhInfinity,
  PhTrash,
  PhDownloadSimple,
  PhEye,
  PhLink,
  PhCaretRight,
  PhCaretLeft,
  PhCaretDown,
  PhClock,
  PhDotsThree,
} from '@phosphor-icons/vue'

const appStore = useAppStore()
const settingsStore = useSettingsStore()
const { viewMode } = storeToRefs(settingsStore)
const { t } = useI18n()
const dialog = useDialog()
const toast = useToast()
const rustUploadManager = useRustUploadManager()
const searchIndex = useSearchIndex()

// Grouped reactive state - Modals
const modals = reactive({
  upload: false,
  createFolder: false,
  view: false,
  rename: false,
  createFile: false,
  versions: false,
})

// Grouped reactive state - Search
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const search = reactive({
  query: '',
  isSearching: false,
  results: [] as S3Object[],
  abortController: null as AbortController | null,
  progress: 0,
  debounceTimer: null as number | null,
  showSettings: false,
  settingsButtonRef: null as HTMLButtonElement | null,
})

// Grouped reactive state - Selection
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const selection = reactive({
  selectedItems: new Set<string>(),
  lastSelectedIndex: -1,
  isDrawing: false,
  box: { startX: 0, startY: 0, endX: 0, endY: 0 },
  justFinished: false,
})

// Grouped reactive state - Drag & Drop
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const dragDrop = reactive({
  isDraggingOver: false,
  isDraggingFile: false,
  draggingObject: null as S3Object | null,
  draggingFolder: null as string | null,
})

// Grouped reactive state - Context Menus
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const contextMenus = reactive({
  main: { show: false, x: 0, y: 0, object: null as S3Object | null },
  empty: { show: false, x: 0, y: 0 },
  showContentTypeSubmenu: false,
})

// Grouped reactive state - Upload
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const upload = reactive({
  file: null as File | null,
  progress: {
    isUploading: false,
    currentFile: '',
    currentIndex: 0,
    totalFiles: 0,
    successCount: 0,
    failCount: 0,
    startTime: 0,
    totalBytes: 0,
    uploadedBytes: 0,
    estimatedTimeRemaining: '',
  },
})

// Grouped reactive state - Create Folder
const folderCreation = reactive({
  name: '',
  validationError: '',
})

// Validate folder name in real-time
watch(
  () => folderCreation.name,
  (value) => {
    if (!value.trim()) {
      folderCreation.validationError = ''
      return
    }
    const result = validateObjectKey(value)
    folderCreation.validationError = result.valid ? '' : result.error || ''
  }
)

// Grouped reactive state - Create File
const fileCreation = reactive({
  name: '',
  content: '',
  isCreating: false,
  validationError: '',
})

// Validate file name in real-time
watch(
  () => fileCreation.name,
  (value) => {
    if (!value.trim()) {
      fileCreation.validationError = ''
      return
    }
    const result = validateObjectKey(value)
    fileCreation.validationError = result.valid ? '' : result.error || ''
  }
)

// Grouped reactive state - Rename
const rename = reactive({
  newName: '',
  object: null as S3Object | null,
  isRenaming: false,
  changingContentType: false,
  validationError: '',
})

// Validate rename name in real-time
watch(
  () => rename.newName,
  (value) => {
    if (!value.trim()) {
      rename.validationError = ''
      return
    }
    const result = validateObjectKey(value)
    rename.validationError = result.valid ? '' : result.error || ''
  }
)

// Grouped reactive state - View Modal
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const viewModal = reactive({
  object: null as S3Object | null,
  viewerRef: null as InstanceType<typeof ObjectViewer> | null,
  versions: [] as ObjectVersion[],
})

// Grouped reactive state - Versions
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const versions = reactive({
  list: [] as ObjectVersion[],
  isLoading: false,
  object: null as S3Object | null,
  expanded: new Set<string>(),
  inline: new Map<string, ObjectVersion[]>(),
  loadingInline: new Set<string>(),
})

// Grouped reactive state - Folder Sizes
// TODO: Update all template references to use this grouped state (Session 4)
// Renamed to avoid conflict with the ref folderSizes currently in use
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const folderSizesGrouped = reactive({
  sizes: new Map<string, number>(),
  loading: new Set<string>(),
})

// Grouped reactive state - Clipboard
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const clipboard = reactive({
  copiedFile: null as S3Object | null,
  isPasting: false,
})

// Sorting
type SortColumn = 'name' | 'size' | 'date'
type SortOrder = 'asc' | 'desc'
// TODO: Update all template references to use this grouped state (Session 4)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const sorting = reactive({
  by: 'name' as SortColumn,
  order: 'asc' as SortOrder,
})

// Standalone refs that don't fit in groups
const objectListRef = ref<HTMLElement | null>(null)
const objectViewerRef = ref<InstanceType<typeof ObjectViewer> | null>(null)
const pasting = ref(false)

// Context menu refs (will be migrated to contextMenus grouped state)
const contextMenu = ref<{ show: boolean; x: number; y: number; object: S3Object | null }>({
  show: false,
  x: 0,
  y: 0,
  object: null,
})
const emptyContextMenu = ref<{ show: boolean; x: number; y: number }>({
  show: false,
  x: 0,
  y: 0,
})

// Search refs (will be migrated to search grouped state)
const searchQuery = ref('')
const searchInputRef = ref<HTMLInputElement | null>(null)
const indexButtonContainerRef = ref<HTMLDivElement | null>(null)
const isSearching = ref(false)
const searchProgress = ref(0)
const searchPagesScanned = ref(0)
const searchAbortController = ref<AbortController | null>(null)
const showSearchSettings = ref(false)
const searchSettingsButtonRef = ref<HTMLButtonElement | null>(null)
const globalSearchResults = ref<S3Object[]>([])
const searchDebounceTimer = ref<number | null>(null)
const searchStartTime = ref(0)
const searchObjectsProcessed = ref(0)
const searchSpeed = ref(0) // objects per second
const searchTimeRemaining = ref(0) // seconds
const searchDuration = ref(0) // milliseconds - actual time taken for search
const useIndexForSearch = ref(false) // whether current search uses index
const hasSearchIndex = ref(false) // whether an index is available for current bucket
const currentIndexSize = ref(0) // number of objects in the current index

// Index build prompt refs
const showIndexBuildPrompt = ref(false)
const indexPromptObjectCount = ref(0)
const indexPromptEstimatedSize = ref('')

// Index update prompt refs (for expired indexes)
const showIndexUpdatePrompt = ref(false)
const indexUpdateObjectDiff = ref(0)
const indexUpdateCurrentCount = ref(0)
const indexUpdateIndexCount = ref(0)

// Selection refs (will be migrated to selection grouped state)
const selectedItems = ref(new Set<string>())
const lastSelectedIndex = ref(-1)
const isDrawingSelection = ref(false)
const selectionBox = ref({ startX: 0, startY: 0, endX: 0, endY: 0 })
const justFinishedSelection = ref(false)

// Drag and drop refs (will be migrated to dragDrop grouped state)
const isDraggingOver = ref(false)
const isDraggingFile = ref(false)
const draggingObject = ref<S3Object | null>(null)
const draggingFolder = ref<string | null>(null)

// View modal refs (will be migrated to viewModal grouped state)
const viewingObject = ref<S3Object | null>(null)
const showViewModal = ref(false)
const viewModalVersions = ref<ObjectVersion[]>([])

// Versions refs (will be migrated to versions grouped state)
const objectVersions = ref<ObjectVersion[]>([])
const versionsObject = ref<S3Object | null>(null)
const showVersionsModal = ref(false)
const loadingVersions = ref(false)
const expandedVersions = ref(new Set<string>())
const inlineVersions = ref(new Map<string, ObjectVersion[]>())
const loadingInlineVersions = ref(new Set<string>())

// Folder size refs (will be migrated to folderSizes grouped state)
const folderSizes = ref(new Map<string, number>())
const loadingFolderSizes = ref(new Set<string>())

// Clipboard refs (will be migrated to clipboard grouped state)
const copiedFile = ref<S3Object | null>(null)

// Sorting refs (will be migrated to sorting grouped state)
const sortBy = ref<SortColumn>('name')
const sortOrder = ref<SortOrder>('asc')

// Modal refs (already using modals grouped state, but some legacy refs)
const showRenameModal = ref(false)

// Rename refs (already using rename grouped state)
const renamingObject = ref<S3Object | null>(null)
const newFileName = ref('')

// Content type changing ref
const changingContentType = ref(false)

// Actions menu ref - tracks which object's actions menu is open
const showActionsMenu = ref<string | null>(null)
const showCopySubmenu = ref<string | null>(null)

// Setup swipe back gesture
useSwipeBack(objectListRef, () => {
  if (appStore.canGoBack) {
    appStore.goBack()
  }
}, {
  threshold: 50,
  velocityThreshold: 0.3
})

// View mode computed properties
const isCompactView = computed(() => viewMode.value === 'compact')
const rowPadding = computed(() => (isCompactView.value ? 'p-0.5' : 'p-1.5'))
const rowGap = computed(() => (isCompactView.value ? 'gap-0.5' : 'gap-1.5'))
const iconSize = computed(() => (isCompactView.value ? 16 : 18))
const textSize = computed(() => 'text-xs')

// Upload progress tracking (handled by RustUploadManager now)

let unlistenFileDrop: UnlistenFn | null = null
let unlistenFileDropHover: UnlistenFn | null = null
let unlistenFileDropCancelled: UnlistenFn | null = null

const pathParts = computed(() => {
  if (!appStore.currentPrefix) return []
  return appStore.currentPrefix.split('/').filter((p) => p)
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
  let folders = []

  if (!searchQuery.value.trim()) {
    folders = [...appStore.folders]
  } else {
    // When searching globally, don't show folders separately
    return []
  }

  // Sort folders
  folders.sort((a, b) => {
    if (sortBy.value === 'name') {
      const nameA = getFolderName(a).toLowerCase()
      const nameB = getFolderName(b).toLowerCase()
      return sortOrder.value === 'asc'
        ? nameA.localeCompare(nameB)
        : nameB.localeCompare(nameA)
    } else if (sortBy.value === 'size') {
      const sizeA = folderSizes.value.get(a) || 0
      const sizeB = folderSizes.value.get(b) || 0
      return sortOrder.value === 'asc' ? sizeA - sizeB : sizeB - sizeA
    }
    // date sorting doesn't apply to folders
    return 0
  })

  return folders
})

const filteredObjects = computed(() => {
  let objects = []

  if (!searchQuery.value.trim()) {
    objects = [...appStore.objects]
  } else {
    const query = searchQuery.value.toLowerCase()
    // Global search: filter all objects in the bucket
    objects = globalSearchResults.value.filter((obj) => {
      const fileName = obj.key.toLowerCase()
      return fileName.includes(query)
    })
  }

  // Sort objects
  objects.sort((a, b) => {
    if (sortBy.value === 'name') {
      const nameA = (searchQuery.value.trim() ? a.key : getFileName(a.key)).toLowerCase()
      const nameB = (searchQuery.value.trim() ? b.key : getFileName(b.key)).toLowerCase()
      return sortOrder.value === 'asc'
        ? nameA.localeCompare(nameB)
        : nameB.localeCompare(nameA)
    } else if (sortBy.value === 'size') {
      return sortOrder.value === 'asc' ? a.size - b.size : b.size - a.size
    } else if (sortBy.value === 'date') {
      const dateA = a.last_modified ? new Date(a.last_modified).getTime() : 0
      const dateB = b.last_modified ? new Date(b.last_modified).getTime() : 0
      return sortOrder.value === 'asc' ? dateA - dateB : dateB - dateA
    }
    return 0
  })

  return objects
})

// Virtual scroll optimization
// Combine folders and objects into a single array for virtualization
type VirtualItem =
  | { type: 'folder'; key: string }
  | { type: 'object'; data: S3Object }

const allVirtualItems = computed<VirtualItem[]>(() => {
  const items: VirtualItem[] = []

  // Add folders first
  filteredFolders.value.forEach(folder => {
    items.push({ type: 'folder', key: folder })
  })

  // Then add objects
  filteredObjects.value.forEach(obj => {
    items.push({ type: 'object', data: obj })
  })

  return items
})

// Item height based on view mode
const virtualItemHeight = computed(() => isCompactView.value ? 40 : 48)

// Initialize virtual scroll
const virtualScroll = useVirtualScroll({
  items: allVirtualItems,
  itemHeight: virtualItemHeight,
  containerHeight: 600, // Fixed container height
  buffer: 10, // Render 10 extra items before/after visible area
})

// Extract visible folders and objects from virtual items
const virtualVisibleFolders = computed(() => {
  return virtualScroll.visibleItems.value
    .filter((item): item is { type: 'folder'; key: string } => item.type === 'folder')
    .map(item => item.key)
})

const virtualVisibleObjects = computed(() => {
  return virtualScroll.visibleItems.value
    .filter((item): item is { type: 'object'; data: S3Object } => item.type === 'object')
    .map(item => item.data)
})

const totalItemsCount = computed(() => {
  return filteredFolders.value.length + filteredObjects.value.length
})

const totalSize = computed(() => {
  // Calculate total size including folders and their recursive contents
  let total = 0

  // Add size of all visible files
  const objects = filteredObjects.value
  total += objects.reduce((sum, obj) => sum + obj.size, 0)

  // Add size of all visible folders (recursive calculation)
  const folders = filteredFolders.value
  for (const folder of folders) {
    const folderSize = folderSizes.value.get(folder)
    if (folderSize !== undefined) {
      total += folderSize
    }
  }

  return total
})

// Show empty state hint when there are few items and not searching
const showEmptyStateHint = computed(() => {
  const totalItems = filteredFolders.value.length + filteredObjects.value.length
  return totalItems < 5 && !searchQuery.value.trim() && !isDraggingOver.value
})

// Calculate popover position relative to index button
const indexPopoverStyle = computed(() => {
  if (!indexButtonContainerRef.value) {
    return { top: '80px', right: '20px' }
  }

  const rect = indexButtonContainerRef.value.getBoundingClientRect()
  return {
    top: `${rect.bottom + 8}px`,
    right: `${window.innerWidth - rect.right}px`,
  }
})

const selectedTotalSize = computed(() => {
  let size = 0
  for (const key of selectedItems.value) {
    // Check if it's a file
    const obj = appStore.objects.find((o) => o.key === key)
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
watch(
  () => appStore.folders,
  async (folders) => {
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
        logger.error(`Failed to calculate size for ${folder}:`, e)
      } finally {
        loadingFolderSizes.value.delete(folder)
      }
    }
  },
  { immediate: true }
)

// Stop search function
function stopSearch() {
  if (searchAbortController.value) {
    searchAbortController.value.abort()
  }
  isSearching.value = false
  searchProgress.value = 0
  searchPagesScanned.value = 0
}

// Search settings menu functions
function toggleSearchSettingsMenu() {
  showSearchSettings.value = !showSearchSettings.value
}

function selectSearchMode(mode: 'local' | 'global') {
  settingsStore.setSearchMode(mode)
  showSearchSettings.value = false
}

// Handle index changed event from IndexButton
async function handleIndexChanged() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  // Reload index status
  const hasIndex = await searchIndex.hasValidIndex(
    appStore.currentProfile.id,
    appStore.currentBucket
  )
  hasSearchIndex.value = hasIndex

  if (hasIndex) {
    const metadata = await searchIndex.getIndexMetadata(
      appStore.currentProfile.id,
      appStore.currentBucket
    )
    if (metadata) {
      currentIndexSize.value = metadata.totalObjects
    }
  } else {
    currentIndexSize.value = 0
  }
}

// Handle build index from prompt
async function handleBuildIndexFromPrompt() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  showIndexBuildPrompt.value = false

  try {
    const index = await searchIndex.buildIndex(
      appStore.currentProfile.id,
      appStore.currentBucket,
      settingsStore.batchSize
    )
    hasSearchIndex.value = true
    currentIndexSize.value = index.totalObjects
  } catch (error) {
    logger.error('Failed to build index:', error)
    toast.error(t('errorOccurred'))
  }
}

// Handle update index from prompt (expired index)
async function handleUpdateIndexFromPrompt() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  showIndexUpdatePrompt.value = false

  try {
    const index = await searchIndex.rebuildIndex(
      appStore.currentProfile.id,
      appStore.currentBucket,
      settingsStore.batchSize
    )
    hasSearchIndex.value = true
    currentIndexSize.value = index.totalObjects
  } catch (error) {
    logger.error('Failed to update index:', error)
    toast.error(t('errorOccurred'))
  }
}

const searchSettingsMenuStyle = computed(() => {
  if (!searchSettingsButtonRef.value) {
    return { top: '0px', left: '0px' }
  }
  const rect = searchSettingsButtonRef.value.getBoundingClientRect()
  return {
    top: `${rect.bottom + 8}px`,
    right: `${window.innerWidth - rect.right}px`,
  }
})

// Watch for search query changes to trigger search
watch(searchQuery, async (query) => {
  // Clear any existing debounce timer
  if (searchDebounceTimer.value !== null) {
    clearTimeout(searchDebounceTimer.value)
    searchDebounceTimer.value = null
  }

  // Stop any ongoing search
  stopSearch()

  if (!query.trim()) {
    globalSearchResults.value = []
    isSearching.value = false
    useIndexForSearch.value = false
    return
  }

  if (!appStore.currentProfile || !appStore.currentBucket) return

  // Capture values before setTimeout to avoid null issues
  const profileId = appStore.currentProfile.id
  const bucket = appStore.currentBucket

  // Debounce search by 500ms
  searchDebounceTimer.value = window.setTimeout(async () => {
    try {
      isSearching.value = true
      globalSearchResults.value = []
      searchProgress.value = 0
      searchPagesScanned.value = 0
      searchObjectsProcessed.value = 0
      searchSpeed.value = 0
      searchTimeRemaining.value = 0
      searchStartTime.value = Date.now()

      const searchPrefix = settingsStore.searchMode === 'local' ? appStore.currentPrefix : ''

      // ═══════════════════════════════════════════════════════════
      // TRY INDEX FIRST (ultra-fast if available and valid)
      // ═══════════════════════════════════════════════════════════

      const hasIndex = await searchIndex.hasValidIndex(profileId, bucket)
      const indexEnabled = searchIndex.isIndexEnabled(profileId, bucket)

      if (hasIndex && indexEnabled) {
        logger.debug('Using search index for instant results')
        useIndexForSearch.value = true

        const index = await searchIndex.loadIndex(profileId, bucket)
        if (index) {
          const results = searchIndex.searchInIndex(index, query, searchPrefix)
          globalSearchResults.value = results
          searchProgress.value = results.length
          searchDuration.value = Date.now() - searchStartTime.value

          logger.debug(`Index search complete: ${results.length} results in ${searchDuration.value}ms`)

          // Mark search as complete (bar stays visible as long as searchQuery exists)
          isSearching.value = false

          return
        }
      }

      // ═══════════════════════════════════════════════════════════
      // FALLBACK: LIVE SEARCH (with improved feedback)
      // ═══════════════════════════════════════════════════════════

      logger.debug('Using live search (no valid index)')
      useIndexForSearch.value = false
      searchAbortController.value = new AbortController()

      let continuationToken: string | undefined = undefined
      const MAX_SEARCH_RESULTS = 10000 // Limit to prevent memory overflow
      const queryLower = query.toLowerCase()

      // If no index exists and it's global search, suggest building one
      // (User can build index manually from settings if desired)
      if (settingsStore.searchMode === 'global' && !hasIndex && !searchIndex.isBuilding.value) {
        logger.debug('No search index found for global search - using live search')
      }

      // Paginate through objects with improved feedback
      do {
        // Check if search was aborted
        if (searchAbortController.value.signal.aborted) {
          break
        }

        // Check if we've reached the maximum limit
        if (globalSearchResults.value.length >= MAX_SEARCH_RESULTS) {
          toast.warning(t('searchLimitReached', MAX_SEARCH_RESULTS))
          break
        }

        const result = await listObjects(
          profileId,
          bucket,
          searchPrefix, // Empty for global, current prefix for local
          continuationToken,
          settingsStore.batchSize,
          false // No delimiter - list all objects recursively
        )

        // Increment counters
        searchPagesScanned.value++
        searchObjectsProcessed.value += result.objects.length

        // ✨ IMPROVEMENT: Filter immediately per page (streaming results)
        const pageMatches = result.objects.filter((obj) =>
          obj.key.toLowerCase().includes(queryLower)
        )

        // Append only matches (not all objects!)
        const remainingSlots = MAX_SEARCH_RESULTS - globalSearchResults.value.length
        const matchesToAdd = pageMatches.slice(0, remainingSlots)
        globalSearchResults.value.push(...matchesToAdd)

        // ✨ IMPROVEMENT: Update progress with matches count (not total objects)
        searchProgress.value = globalSearchResults.value.length

        // ✨ IMPROVEMENT: Calculate and display search speed
        const elapsedSeconds = (Date.now() - searchStartTime.value) / 1000
        if (elapsedSeconds > 0) {
          searchSpeed.value = Math.round(searchObjectsProcessed.value / elapsedSeconds)

          // ✨ IMPROVEMENT: Estimate time remaining (rough estimate)
          // Assume we'll scan similar number of pages as we've scanned so far
          if (continuationToken) {
            const avgObjectsPerPage = searchObjectsProcessed.value / searchPagesScanned.value
            const estimatedTotalPages = searchPagesScanned.value * 2 // Rough estimate
            const remainingPages = estimatedTotalPages - searchPagesScanned.value
            const remainingObjects = remainingPages * avgObjectsPerPage
            searchTimeRemaining.value = Math.max(0, remainingObjects / searchSpeed.value)
          } else {
            searchTimeRemaining.value = 0
          }
        }

        // Stop if we've reached the limit
        if (globalSearchResults.value.length >= MAX_SEARCH_RESULTS) {
          break
        }

        // Check if there are more pages
        continuationToken = result.continuation_token
      } while (continuationToken && !searchAbortController.value.signal.aborted)

    } catch (e: any) {
      if (e.name !== 'AbortError') {
        logger.error('Search failed:', e)
        toast.error(`${t('errorOccurred')}: ${e}`)
      }
      globalSearchResults.value = []
    } finally {
      if (!searchAbortController.value?.signal.aborted) {
        isSearching.value = false
      }
      searchAbortController.value = null
      searchSpeed.value = 0
      searchTimeRemaining.value = 0
    }
  }, 500) // Debounce delay
})

// Clear selection and cached data when navigating to a different folder
watch(
  () => appStore.currentPrefix,
  () => {
    clearSelection()
    // Clear inline versions to free memory
    inlineVersions.value.clear()
    expandedVersions.value.clear()
    // Clear search results to free memory
    globalSearchResults.value = []
    searchQuery.value = ''
    isSearching.value = false
  }
)

// Clear all cached data when switching buckets and auto-build index if needed
watch(
  () => appStore.currentBucket,
  async (newBucket) => {
    clearSelection()
    inlineVersions.value.clear()
    expandedVersions.value.clear()
    globalSearchResults.value = []
    searchQuery.value = ''
    isSearching.value = false
    hasSearchIndex.value = false
    showIndexBuildPrompt.value = false
    showIndexUpdatePrompt.value = false

    // Auto-build index for new bucket (if no valid index exists)
    if (newBucket && appStore.currentProfile) {
      const profileId = appStore.currentProfile.id
      const indexStatus = await searchIndex.getIndexStatus(profileId, newBucket)

      logger.debug(`[Index] Bucket ${newBucket}: exists=${indexStatus.exists}, isValid=${indexStatus.isValid}, age=${Math.round(indexStatus.age / 1000 / 60)}min, objects=${indexStatus.totalObjects}`)

      if (indexStatus.exists && indexStatus.isValid) {
        // Valid index exists (< 8h old)
        hasSearchIndex.value = true
        currentIndexSize.value = indexStatus.totalObjects
        logger.debug(`[Index] Valid index with ${indexStatus.totalObjects} objects`)
      } else if (indexStatus.exists && !indexStatus.isValid) {
        // Expired index exists (> 8h old) - check if we should update
        hasSearchIndex.value = true // Keep using the old index until updated
        currentIndexSize.value = indexStatus.totalObjects

        logger.debug(`[Index] Expired index found (age: ${Math.round(indexStatus.age / 1000 / 60 / 60)}h)`)

        if (!searchIndex.isBuilding.value) {
          // Estimate current bucket size
          const estimatedCount = await searchIndex.estimateBucketSize(profileId, newBucket)
          logger.debug(`[Index] Current bucket size estimate: ${estimatedCount}`)

          if (estimatedCount !== -1 && estimatedCount < settingsStore.indexAutoBuildThreshold) {
            // Auto-rebuild for small buckets (< threshold objects)
            logger.debug(`[Index] Auto-rebuilding expired index for small bucket (${estimatedCount} objects, threshold=${settingsStore.indexAutoBuildThreshold})`)
            searchIndex.rebuildIndex(profileId, newBucket, settingsStore.batchSize)
              .then((index) => {
                hasSearchIndex.value = true
                currentIndexSize.value = index.totalObjects
              })
              .catch(error => {
                logger.error('Failed to auto-rebuild index:', error)
              })
          } else {
            // Show update prompt for large buckets
            const diff = estimatedCount === -1 ? 0 : estimatedCount - indexStatus.totalObjects
            logger.debug(`[Index] Showing update prompt. Diff: ${diff > 0 ? '+' : ''}${diff}`)

            showIndexUpdatePrompt.value = true
            indexUpdateObjectDiff.value = diff
            indexUpdateCurrentCount.value = estimatedCount
            indexUpdateIndexCount.value = indexStatus.totalObjects
          }
        }
      } else if (!searchIndex.isBuilding.value) {
        // No index exists - estimate bucket size to decide if we should auto-build
        const estimatedCount = await searchIndex.estimateBucketSize(profileId, newBucket)
        logger.debug(`[Index] Estimated bucket size: ${estimatedCount}`)

        if (estimatedCount !== -1 && estimatedCount < settingsStore.indexAutoBuildThreshold) {
          // Auto-build for small buckets (< threshold objects)
          logger.debug(`[Index] Auto-building index for bucket with ${estimatedCount} objects (threshold=${settingsStore.indexAutoBuildThreshold})`)
          searchIndex.buildIndex(profileId, newBucket, settingsStore.batchSize)
            .then((index) => {
              hasSearchIndex.value = true
              currentIndexSize.value = index.totalObjects
            })
            .catch(error => {
              logger.error('Failed to auto-build index:', error)
            })
        } else if (estimatedCount === -1 || estimatedCount >= settingsStore.indexAutoBuildThreshold) {
          // Show prompt for large buckets (>= threshold objects)
          logger.debug(`[Index] Showing build prompt for large bucket (${estimatedCount} objects, threshold=${settingsStore.indexAutoBuildThreshold})`)
          showIndexBuildPrompt.value = true
          indexPromptObjectCount.value = estimatedCount

          // Estimate index size
          if (estimatedCount !== -1) {
            const estimatedBytes = searchIndex.estimateIndexSize(estimatedCount)
            indexPromptEstimatedSize.value = formatSize(estimatedBytes)
          } else {
            // When truncated, we know there are at least threshold objects
            const minEstimatedBytes = searchIndex.estimateIndexSize(settingsStore.indexAutoBuildThreshold)
            indexPromptEstimatedSize.value = `≈ ${formatSize(minEstimatedBytes)}`
          }
        }
      } else {
        logger.debug(`[Index] Build already in progress, skipping prompt`)
      }
    }
  }
)

function getFolderSize(folder: string): string {
  if (loadingFolderSizes.value.has(folder)) {
    return t('calculating')
  }
  const size = folderSizes.value.get(folder)
  if (size === undefined) return '-'
  return formatSize(size)
}

/**
 * Navigate to a folder and load the first page
 * Consistent with reloadCurrentView() - loads only the first batch of objects
 * User can click "Load More" to load additional pages
 */
async function navigateAndLoad(prefix?: string) {
  // Navigate to prefix (or stay current if undefined)
  if (prefix !== undefined) {
    appStore.navigateToFolder(prefix)
  }

  // Load first page only (consistent with batch size setting)
  await appStore.loadObjects()
}

/**
 * Reload ALL pages from S3 (used for explicit Refresh button only)
 *
 * This function loads the first page, then continues loading all subsequent pages
 * until no more data exists. It's useful for the explicit refresh button where
 * the user wants to see all previously loaded data refreshed from S3.
 *
 * ⚠️ NOTE: For CRUD operations (create/delete/upload), use appStore.reloadCurrentView()
 * instead, which is faster and resets pagination to the first page.
 */
async function reloadAllPages() {
  // Capture the number of pages the user had loaded
  const targetPages = Math.max(1, appStore.loadedPagesCount)

  logger.debug(`[Refresh] Reloading ${targetPages} page(s)...`)

  // Reset and load first page
  await appStore.reloadCurrentView()

  // If the user had loaded more than one page, load the remaining ones
  if (targetPages > 1) {
    const toastId = toast.loading(`${t('loading')} ${targetPages} pages...`)

    try {
      // Load remaining pages (targetPages - 1 since we already have the first)
      for (let i = 1; i < targetPages && appStore.continuationToken; i++) {
        await appStore.loadObjects(true)

        // Update toast with progress
        toast.updateToast(toastId, {
          message: `${t('loaded')} ${i + 1}/${targetPages} pages (${appStore.objects.length} ${t('objects')})`,
        })
      }

      // Success
      toast.completeToast(
        toastId,
        `Refreshed ${targetPages} pages (${appStore.objects.length} ${t('objects')})`,
        'success',
        2000
      )
    } catch (e) {
      toast.completeToast(toastId, `${t('errorOccurred')}: ${e}`, 'error')
      logger.error('Failed to reload pages:', e)
    }
  }

  logger.debug(`[Refresh] Reloaded ${appStore.objects.length} objects`)
}

function navigateToRoot() {
  folderSizes.value.clear()
  navigateAndLoad('')
}

function navigateToPath(index: number) {
  folderSizes.value.clear()
  const parts = pathParts.value.slice(0, index + 1)
  const prefix = parts.join('/') + '/'
  navigateAndLoad(prefix)
}

function navigateToFolder(folder: string) {
  folderSizes.value.clear()
  navigateAndLoad(folder)
}

function handleSort(column: SortColumn) {
  if (sortBy.value === column) {
    // Toggle sort order
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    // Change column and reset to ascending
    sortBy.value = column
    sortOrder.value = 'asc'
  }
}

function getFolderName(folder: string): string {
  const parts = folder.split('/').filter((p) => p)
  return parts[parts.length - 1] || folder
}

function getFileName(key: string): string {
  const parts = key.split('/')
  return parts[parts.length - 1] || key
}

function getFileExtension(filename: string): string {
  const parts = filename.split('.')
  return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : ''
}

function getFileIcon(key: string): { icon: any; colorClass: string } {
  const ext = getFileExtension(getFileName(key))

  // Exact extension matching with unique icons for each
  switch (ext) {
    // Images - Different icons for each format
    case 'png':
      return { icon: PhFilePng, colorClass: 'text-blue-500' }
    case 'jpg':
      return { icon: PhFileJpg, colorClass: 'text-blue-600' }
    case 'jpeg':
      return { icon: PhImage, colorClass: 'text-blue-400' }
    case 'gif':
      return { icon: PhImages, colorClass: 'text-cyan-500' }
    case 'svg':
      return { icon: PhFrameCorners, colorClass: 'text-purple-400' }
    case 'webp':
      return { icon: PhImageSquare, colorClass: 'text-teal-500' }
    case 'bmp':
      return { icon: PhImage, colorClass: 'text-blue-300' }
    case 'ico':
      return { icon: PhCube, colorClass: 'text-indigo-500' }

    // Videos - Different icons for each format
    case 'mp4':
      return { icon: PhVideo, colorClass: 'text-purple-500' }
    case 'avi':
      return { icon: PhFilmSlate, colorClass: 'text-purple-600' }
    case 'mov':
      return { icon: PhVideoCamera, colorClass: 'text-purple-400' }
    case 'wmv':
      return { icon: PhVideo, colorClass: 'text-purple-700' }
    case 'flv':
      return { icon: PhFilmSlate, colorClass: 'text-fuchsia-500' }
    case 'webm':
      return { icon: PhVideoCamera, colorClass: 'text-violet-500' }
    case 'mkv':
      return { icon: PhVideo, colorClass: 'text-indigo-600' }
    case 'm4v':
      return { icon: PhFilmSlate, colorClass: 'text-purple-500' }

    // Audio - Different icons for each format
    case 'mp3':
      return { icon: PhMusicNote, colorClass: 'text-pink-500' }
    case 'wav':
      return { icon: PhWaveform, colorClass: 'text-pink-600' }
    case 'ogg':
      return { icon: PhMusicNotes, colorClass: 'text-pink-400' }
    case 'flac':
      return { icon: PhVinylRecord, colorClass: 'text-rose-500' }
    case 'aac':
      return { icon: PhSpeakerHigh, colorClass: 'text-pink-600' }
    case 'm4a':
      return { icon: PhMusicNote, colorClass: 'text-fuchsia-500' }
    case 'wma':
      return { icon: PhMusicNotes, colorClass: 'text-rose-600' }

    // Documents
    case 'pdf':
      return { icon: PhFilePdf, colorClass: 'text-red-500' }
    case 'doc':
      return { icon: PhMicrosoftWordLogo, colorClass: 'text-blue-600' }
    case 'docx':
      return { icon: PhMicrosoftWordLogo, colorClass: 'text-blue-700' }
    case 'xls':
      return { icon: PhMicrosoftExcelLogo, colorClass: 'text-green-600' }
    case 'xlsx':
      return { icon: PhMicrosoftExcelLogo, colorClass: 'text-green-700' }
    case 'csv':
      return { icon: PhFileCsv, colorClass: 'text-green-500' }
    case 'ppt':
      return { icon: PhMicrosoftPowerpointLogo, colorClass: 'text-orange-500' }
    case 'pptx':
      return { icon: PhMicrosoftPowerpointLogo, colorClass: 'text-orange-600' }

    // Text files - Different icons
    case 'txt':
      return { icon: PhFileText, colorClass: 'text-gray-400' }
    case 'log':
      return { icon: PhNote, colorClass: 'text-gray-500' }
    case 'md':
      return { icon: PhMarkdownLogo, colorClass: 'text-slate-600' }
    case 'rtf':
      return { icon: PhArticle, colorClass: 'text-gray-600' }
    case 'readme':
      return { icon: PhBookOpen, colorClass: 'text-blue-500' }

    // JavaScript - Unique icons
    case 'js':
      return { icon: PhFileJs, colorClass: 'text-yellow-500' }
    case 'mjs':
      return { icon: PhFileJs, colorClass: 'text-yellow-600' }
    case 'cjs':
      return { icon: PhFileJs, colorClass: 'text-yellow-400' }
    case 'jsx':
      return { icon: PhFileJsx, colorClass: 'text-cyan-500' }

    // TypeScript
    case 'ts':
      return { icon: PhFileTs, colorClass: 'text-blue-500' }
    case 'mts':
      return { icon: PhFileTs, colorClass: 'text-blue-600' }
    case 'cts':
      return { icon: PhFileTs, colorClass: 'text-blue-400' }
    case 'tsx':
      return { icon: PhFileTsx, colorClass: 'text-cyan-600' }

    // Vue
    case 'vue':
      return { icon: PhFileVue, colorClass: 'text-green-500' }

    // Python
    case 'py':
      return { icon: PhFilePy, colorClass: 'text-blue-400' }
    case 'pyw':
      return { icon: PhFilePy, colorClass: 'text-blue-500' }
    case 'pyc':
      return { icon: PhBinary, colorClass: 'text-blue-300' }

    // Rust
    case 'rs':
      return { icon: PhFileRs, colorClass: 'text-orange-600' }

    // C/C++
    case 'c':
      return { icon: PhFileC, colorClass: 'text-purple-500' }
    case 'h':
      return { icon: PhFileCode, colorClass: 'text-purple-400' }
    case 'cpp':
      return { icon: PhFileCpp, colorClass: 'text-purple-600' }
    case 'cc':
      return { icon: PhFileCpp, colorClass: 'text-purple-700' }
    case 'cxx':
      return { icon: PhFileCpp, colorClass: 'text-indigo-600' }
    case 'hpp':
      return { icon: PhFileCode, colorClass: 'text-purple-500' }

    // Other programming languages
    case 'java':
      return { icon: PhFileCode, colorClass: 'text-red-600' }
    case 'cs':
      return { icon: PhFileCode, colorClass: 'text-purple-600' }
    case 'go':
      return { icon: PhFileCode, colorClass: 'text-cyan-600' }
    case 'php':
      return { icon: PhFileCode, colorClass: 'text-indigo-500' }
    case 'rb':
      return { icon: PhFileCode, colorClass: 'text-red-500' }
    case 'swift':
      return { icon: PhFileCode, colorClass: 'text-orange-500' }
    case 'kt':
      return { icon: PhFileCode, colorClass: 'text-purple-500' }
    case 'scala':
      return { icon: PhFileCode, colorClass: 'text-red-700' }

    // Web files
    case 'html':
      return { icon: PhFileHtml, colorClass: 'text-orange-500' }
    case 'htm':
      return { icon: PhFileHtml, colorClass: 'text-orange-400' }
    case 'css':
      return { icon: PhFileCss, colorClass: 'text-blue-400' }
    case 'scss':
      return { icon: PhFileCss, colorClass: 'text-pink-500' }
    case 'sass':
      return { icon: PhFileCss, colorClass: 'text-pink-400' }
    case 'less':
      return { icon: PhFileCss, colorClass: 'text-blue-500' }
    case 'wasm':
      return { icon: PhCubeTransparent, colorClass: 'text-purple-600' }
    case 'webmanifest':
      return { icon: PhGlobe, colorClass: 'text-cyan-500' }

    // Config files - Different icons
    case 'json':
      return { icon: PhBracketsCurly, colorClass: 'text-yellow-500' }
    case 'xml':
      return { icon: PhCodeBlock, colorClass: 'text-orange-500' }
    case 'yaml':
      return { icon: PhListBullets, colorClass: 'text-purple-500' }
    case 'yml':
      return { icon: PhListBullets, colorClass: 'text-purple-400' }
    case 'toml':
      return { icon: PhGear, colorClass: 'text-orange-600' }
    case 'ini':
      return { icon: PhFileIni, colorClass: 'text-gray-500' }
    case 'conf':
      return { icon: PhGear, colorClass: 'text-gray-600' }
    case 'config':
      return { icon: PhGear, colorClass: 'text-gray-500' }
    case 'env':
      return { icon: PhFileLock, colorClass: 'text-green-600' }

    // Database
    case 'sql':
      return { icon: PhFileSql, colorClass: 'text-indigo-500' }
    case 'db':
      return { icon: PhDatabase, colorClass: 'text-indigo-600' }
    case 'sqlite':
      return { icon: PhDatabase, colorClass: 'text-blue-600' }
    case 'sqlite3':
      return { icon: PhDatabase, colorClass: 'text-blue-500' }

    // Archives - Different icons
    case 'zip':
      return { icon: PhFileZip, colorClass: 'text-amber-600' }
    case 'rar':
      return { icon: PhArchive, colorClass: 'text-amber-700' }
    case '7z':
      return { icon: PhFileZip, colorClass: 'text-amber-500' }
    case 'tar':
      return { icon: PhArchive, colorClass: 'text-orange-700' }
    case 'gz':
      return { icon: PhFileZip, colorClass: 'text-orange-600' }
    case 'bz2':
      return { icon: PhArchive, colorClass: 'text-red-700' }
    case 'xz':
      return { icon: PhFileZip, colorClass: 'text-yellow-700' }
    case 'tgz':
      return { icon: PhArchive, colorClass: 'text-amber-600' }

    // Executables & binaries
    case 'exe':
      return { icon: PhLightning, colorClass: 'text-red-600' }
    case 'msi':
      return { icon: PhLightning, colorClass: 'text-red-500' }
    case 'app':
      return { icon: PhAppleLogo, colorClass: 'text-gray-700' }
    case 'dmg':
      return { icon: PhAppleLogo, colorClass: 'text-blue-600' }
    case 'deb':
      return { icon: PhLinuxLogo, colorClass: 'text-red-600' }
    case 'rpm':
      return { icon: PhLinuxLogo, colorClass: 'text-red-700' }
    case 'apk':
      return { icon: PhAndroidLogo, colorClass: 'text-green-600' }
    case 'bin':
      return { icon: PhBinary, colorClass: 'text-gray-700' }

    // Shell scripts
    case 'sh':
      return { icon: PhTerminal, colorClass: 'text-green-500' }
    case 'bash':
      return { icon: PhTerminal, colorClass: 'text-green-600' }
    case 'zsh':
      return { icon: PhTerminal, colorClass: 'text-cyan-600' }
    case 'fish':
      return { icon: PhTerminal, colorClass: 'text-blue-500' }
    case 'ps1':
      return { icon: PhTerminal, colorClass: 'text-blue-600' }
    case 'bat':
      return { icon: PhTerminal, colorClass: 'text-gray-600' }
    case 'cmd':
      return { icon: PhTerminal, colorClass: 'text-gray-700' }

    // Package & lock files
    case 'pkg':
      return { icon: PhPackage, colorClass: 'text-amber-700' }
    case 'snap':
      return { icon: PhPackage, colorClass: 'text-orange-700' }
    case 'lock':
      return { icon: PhLock, colorClass: 'text-red-500' }
    case 'key':
      return { icon: PhKey, colorClass: 'text-yellow-600' }
    case 'pem':
      return { icon: PhShieldCheck, colorClass: 'text-green-700' }
    case 'crt':
      return { icon: PhShieldCheck, colorClass: 'text-blue-700' }
    case 'cert':
      return { icon: PhShieldCheck, colorClass: 'text-indigo-700' }

    // Git & version control
    case 'git':
      return { icon: PhGitBranch, colorClass: 'text-orange-600' }
    case 'gitignore':
      return { icon: PhGithubLogo, colorClass: 'text-gray-700' }
    case 'gitlab-ci':
      return { icon: PhGitlabLogo, colorClass: 'text-orange-600' }

    // Design files
    case 'fig':
      return { icon: PhFigmaLogo, colorClass: 'text-purple-600' }
    case 'sketch':
      return { icon: PhSketchLogo, colorClass: 'text-orange-500' }
    case 'ai':
      return { icon: PhBug, colorClass: 'text-orange-600' }
    case 'psd':
      return { icon: PhImage, colorClass: 'text-blue-700' }

    // Data files
    case 'tsv':
      return { icon: PhTable, colorClass: 'text-green-600' }
    case 'parquet':
      return { icon: PhChartBar, colorClass: 'text-purple-600' }
    case 'avro':
      return { icon: PhChartLine, colorClass: 'text-blue-600' }

    // Mobile
    case 'ipa':
      return { icon: PhAppleLogo, colorClass: 'text-blue-500' }
    case 'aab':
      return { icon: PhAndroidLogo, colorClass: 'text-green-500' }

    // Other
    case 'dockerfile':
      return { icon: PhCube, colorClass: 'text-blue-600' }
    case 'iso':
      return { icon: PhCubeTransparent, colorClass: 'text-purple-700' }
    case 'img':
      return { icon: PhCube, colorClass: 'text-indigo-600' }
    case 'bak':
      return { icon: PhFileArrowDown, colorClass: 'text-gray-500' }
    case 'tmp':
      return { icon: PhFileX, colorClass: 'text-red-400' }
    case 'cache':
      return { icon: PhInfinity, colorClass: 'text-cyan-600' }
    case 'atom':
      return { icon: PhAtom, colorClass: 'text-green-600' }

    // Default
    default:
      return { icon: PhFile, colorClass: 'text-gray-400' }
  }
}

async function uploadFilesHandler() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  // Capture values to prevent null reference errors during async operations
  const profileId = appStore.currentProfile.id
  const bucket = appStore.currentBucket
  const prefix = appStore.currentPrefix

  // Use Tauri dialog to select files
  const selected = await open({
    multiple: true,
    title: t('upload'),
  })

  if (!selected) return // User cancelled

  const filePaths = Array.isArray(selected) ? selected : [selected]

  // Helper to detect content type from file extension
  const getContentType = (fileName: string): string | undefined => {
    const ext = fileName.split('.').pop()?.toLowerCase()
    if (!ext) return undefined

    const contentTypes: Record<string, string> = {
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      png: 'image/png',
      gif: 'image/gif',
      pdf: 'application/pdf',
      txt: 'text/plain',
      json: 'application/json',
      xml: 'application/xml',
      zip: 'application/zip',
      mp4: 'video/mp4',
    }

    return contentTypes[ext]
  }

  // Queue all uploads immediately - Rust upload manager handles concurrency
  // Don't wait for uploads to start, just queue them all
  filePaths.forEach((filePath) => {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file'
    const key = prefix + fileName
    const contentType = getContentType(fileName)

    // Fire and forget - just queue the upload
    rustUploadManager.startUpload(
      profileId,
      bucket,
      key,
      filePath,
      contentType
    ).catch((e) => {
      // Ignore cancellation errors (user clicked cancel)
      if (e && e.message && e.message.includes('cancelled')) {
        return
      }
      logger.error(`✗ Failed to queue upload ${fileName}:`, e)
      toast.error(`Failed to queue: ${fileName}`)
    })
  })

  // Show immediate feedback
  logger.debug(`Queued ${filePaths.length} file(s) for upload`)
  toast.success(`Queued ${filePaths.length} file(s) for upload`)

  // Objects will be reloaded automatically when uploads complete
}

async function createFolderHandler() {
  // Validate before creating
  if (!folderCreation.name.trim() || folderCreation.validationError) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const toastId = toast.loading(`${t('creating')} ${folderCreation.name}`)

  try {
    const folderPath = appStore.currentPrefix + folderCreation.name

    // Create folder on S3
    await createFolderService(appStore.currentProfile.id, appStore.currentBucket, folderPath)

    // Optimistic update: add folder immediately to local store
    // S3 folder paths end with '/'
    appStore.addFolder(folderPath + '/')

    toast.completeToast(toastId, `Folder "${folderCreation.name}" created successfully!`, 'success')
    modals.createFolder = false
    folderCreation.name = ''
    folderCreation.validationError = ''
  } catch (e) {
    toast.completeToast(toastId, `${t('createFailed')}: ${e}`, 'error')

    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('createFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // On error, reload view to resynchronize with S3
    await appStore.reloadCurrentView()
  }
}

/**
 * Load all remaining objects in the current folder
 * Continues loading pages until there are no more objects
 */
async function loadAllObjects() {
  if (!appStore.currentProfile || !appStore.currentBucket) return
  if (!appStore.continuationToken) return

  // Show loading toast
  const toastId = toast.loading(t('loadingAllObjects'))
  let totalLoaded = appStore.objects.length

  try {
    // Keep loading until no more continuation token
    while (appStore.continuationToken) {
      await appStore.loadObjects(true) // Load next page
      totalLoaded = appStore.objects.length

      // Update toast with progress
      toast.updateToast(toastId, {
        message: `${t('loaded')} ${totalLoaded} ${t('objects')}...`,
      })
    }

    // Success toast
    toast.completeToast(
      toastId,
      `${t('loadedAllObjects')}: ${totalLoaded} ${t('objects')}`,
      'success',
      3000
    )
  } catch (e) {
    toast.completeToast(toastId, `${t('errorLoadingObjects')}: ${e}`, 'error', 5000)
    logger.error('Failed to load all objects:', e)
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
      variant: 'destructive',
    })
  }
}

// Copy object URL to clipboard
async function copyObjectUrl(key: string) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    const profile = appStore.currentProfile
    const bucket = appStore.currentBucket

    let fullUrl = ''

    if (profile.endpoint) {
      // Custom endpoint (MinIO, S3-compatible storage)
      const endpoint = profile.endpoint.replace(/\/$/, '') // Remove trailing slash

      if (profile.path_style) {
        // Path-style URL: https://endpoint/bucket/key
        fullUrl = `${endpoint}/${bucket}/${key}`
      } else {
        // Virtual-hosted-style URL: https://bucket.endpoint/key
        const endpointHost = endpoint.replace(/^https?:\/\//, '')
        const protocol = endpoint.startsWith('https') ? 'https' : 'http'
        fullUrl = `${protocol}://${bucket}.${endpointHost}/${key}`
      }
    } else {
      // AWS S3 URL format
      if (profile.region === 'us-east-1') {
        // Special case for us-east-1
        fullUrl = `https://${bucket}.s3.amazonaws.com/${key}`
      } else {
        fullUrl = `https://${bucket}.s3.${profile.region}.amazonaws.com/${key}`
      }
    }

    // Copy to clipboard
    await navigator.clipboard.writeText(fullUrl)
    showCopySubmenu.value = null
    toast.success(t('urlCopied'))
  } catch (e) {
    toast.error(`${t('copyFailed')}: ${e}`)
  }
}

// Copy full path (bucket + key) to clipboard
async function copyFullPath(key: string) {
  if (!appStore.currentBucket) return

  try {
    const fullPath = `${appStore.currentBucket}/${key}`
    await navigator.clipboard.writeText(fullPath)
    showCopySubmenu.value = null
    toast.success(t('fullPathCopied'))
  } catch (e) {
    toast.error(`${t('copyFailed')}: ${e}`)
  }
}

// Copy path (without bucket) to clipboard
async function copyPath(key: string) {
  try {
    await navigator.clipboard.writeText(key)
    showCopySubmenu.value = null
    toast.success(t('pathCopied'))
  } catch (e) {
    toast.error(`${t('copyFailed')}: ${e}`)
  }
}

// Copy file name to clipboard
async function copyFileName(key: string) {
  try {
    const fileName = getFileName(key)
    await navigator.clipboard.writeText(fileName)
    showCopySubmenu.value = null
    toast.success(t('fileNameCopied'))
  } catch (e) {
    toast.error(`${t('copyFailed')}: ${e}`)
  }
}

// Drag & drop to download
function handleFileDragStart(event: DragEvent, obj: S3Object) {
  isDraggingFile.value = true
  draggingObject.value = obj

  // Set drag data
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'copy'
    event.dataTransfer.setData('text/plain', getFileName(obj.key))
  }
}

async function handleFileDragEnd(event: DragEvent, obj: S3Object) {
  isDraggingFile.value = false
  draggingObject.value = null

  // Check if the file was dropped outside the app window (to filesystem)
  if (event.dataTransfer && event.dataTransfer.dropEffect !== 'none') {
    // User dropped the file somewhere, trigger download
    await downloadObjectDragDrop(obj.key)
  }
}

async function downloadObjectDragDrop(key: string) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const fileName = getFileName(key)
  const toastId = toast.loading(`${t('downloading')} ${fileName}`)

  try {
    // Ask user where to save
    const filePath = await save({
      defaultPath: fileName,
    })

    if (!filePath) {
      toast.removeToast(toastId)
      return
    }

    // Download the file
    const { getObject } = await import('../services/tauri')
    const response = await getObject(appStore.currentProfile.id, appStore.currentBucket, key)

    await writeBinaryFile(filePath, new Uint8Array(response.content))
    toast.completeToast(toastId, `${fileName} ${t('fileDownloadedSuccess')}`, 'success')
  } catch (e) {
    toast.completeToast(toastId, `${t('downloadFailed')}: ${e}`, 'error')
  }
}

// Drag & drop folders to download all contents
function handleFolderDragStart(event: DragEvent, folder: string) {
  isDraggingFile.value = true
  draggingFolder.value = folder

  // Set drag data
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'copy'
    event.dataTransfer.setData('text/plain', getFolderName(folder))
  }
}

async function handleFolderDragEnd(event: DragEvent, folder: string) {
  isDraggingFile.value = false
  draggingFolder.value = null

  // Check if the folder was dropped outside the app window
  if (event.dataTransfer && event.dataTransfer.dropEffect !== 'none') {
    // User dropped the folder, trigger download of all contents
    await downloadFolderDragDrop(folder)
  }
}

async function downloadFolderDragDrop(folder: string) {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const folderName = getFolderName(folder)

  // Show info that folder download will download all files
  const confirmed = await dialog.confirm({
    title: t('download'),
    message: `Download all files from "${folderName}"?`,
    confirmText: t('download'),
    cancelText: t('cancel'),
  })

  if (!confirmed) return

  try {
    // Get all objects in the folder
    const { listObjects, getObject } = await import('../services/tauri')
    const allObjects: S3Object[] = []
    let continuationToken: string | undefined = undefined

    // Create toast with progress
    const toastId = toast.loading(`${t('downloading')} ${folderName}... 0 files`)

    do {
      const result = await listObjects(
        appStore.currentProfile.id,
        appStore.currentBucket,
        folder,
        continuationToken,
        settingsStore.batchSize,
        false // No delimiter - get all files recursively
      )

      allObjects.push(...result.objects)

      // Update toast
      toast.updateToast(toastId, {
        message: `${t('downloading')} ${folderName}... ${allObjects.length} files found`,
      })

      continuationToken = result.continuation_token
    } while (continuationToken)

    if (allObjects.length === 0) {
      toast.completeToast(toastId, 'No files to download', 'error')
      return
    }

    // Ask for download directory
    const folderPath = await save({
      defaultPath: folderName,
    })

    if (!folderPath) {
      toast.removeToast(toastId)
      return
    }

    // Download all files
    let successCount = 0
    let failCount = 0

    for (const obj of allObjects) {
      try {
        const fileName = getFileName(obj.key)
        const response = await getObject(
          appStore.currentProfile.id,
          appStore.currentBucket,
          obj.key
        )

        // Construct file path (base directory + filename)
        const baseDir = folderPath.substring(0, folderPath.lastIndexOf('/'))
        const filePath = `${baseDir}/${fileName}`

        await writeBinaryFile(filePath, new Uint8Array(response.content))
        successCount++

        // Update progress
        const progress = Math.round(((successCount + failCount) / allObjects.length) * 100)
        toast.updateToast(toastId, {
          message: `${t('downloading')} ${successCount}/${allObjects.length} files`,
          progress,
        })
      } catch (e) {
        logger.error(`Failed to download ${obj.key}:`, e)
        failCount++
      }
    }

    // Complete
    if (failCount === 0) {
      toast.completeToast(toastId, `Downloaded ${successCount} file(s) successfully!`, 'success')
    } else {
      toast.completeToast(
        toastId,
        `Downloaded ${successCount} file(s), ${failCount} failed`,
        failCount < allObjects.length ? 'success' : 'error'
      )
    }
  } catch (e) {
    logger.error('Folder download failed:', e)
  }
}

async function viewObject(obj: S3Object) {
  viewingObject.value = obj
  showViewModal.value = true
  // Load versions for the object
  await loadViewModalVersions()
}

async function loadViewModalVersions() {
  if (!viewingObject.value || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    const response = await listObjectVersions(
      appStore.currentProfile.id,
      appStore.currentBucket,
      viewingObject.value.key
    )
    viewModalVersions.value = response.versions
  } catch (e) {
    logger.error('Failed to load versions:', e)
    viewModalVersions.value = []
  }
}

async function deleteObjectConfirm(key: string) {
  const confirmed = await dialog.confirm({
    title: t('delete'),
    message: t('deleteFileConfirm').replace('{0}', getFileName(key)),
    confirmText: t('delete'),
    cancelText: t('cancel'),
    variant: 'destructive',
  })

  if (!confirmed) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const toastId = toast.loading(`${t('deleting')} ${getFileName(key)}`)

  try {
    // Delete from S3
    await deleteObject(appStore.currentProfile.id, appStore.currentBucket, key)

    // Optimistic update: remove from local store immediately
    appStore.removeObject(key)

    toast.completeToast(toastId, `${getFileName(key)} deleted successfully!`, 'success')
  } catch (e) {
    // On error, reload view to resynchronize with server
    toast.completeToast(toastId, `${t('deleteFailed')}: ${e}`, 'error')

    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('deleteFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // Resynchronize with S3 to ensure consistency
    await appStore.reloadCurrentView()
  }
}

async function deleteFolderConfirm(folder: string) {
  const folderName = getFolderName(folder)
  const confirmed = await dialog.confirm({
    title: t('delete'),
    message: t('deleteFolderConfirm').replace('{0}', folderName),
    confirmText: t('delete'),
    cancelText: t('cancel'),
    variant: 'destructive',
  })

  if (!confirmed) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  const toastId = toast.loading(`${t('deleting')} ${folderName}`)

  try {
    // Delete folder from S3 (recursively deletes all objects inside)
    const deletedCount = await deleteFolder(
      appStore.currentProfile.id,
      appStore.currentBucket,
      folder
    )

    // Optimistic update: remove folder from local store immediately
    appStore.removeFolder(folder)

    toast.completeToast(
      toastId,
      t('folderDeletedSuccess').replace('{0}', String(deletedCount)),
      'success'
    )
  } catch (e) {
    // On error, reload view to resynchronize with server
    toast.completeToast(toastId, `${t('deleteFailed')}: ${e}`, 'error')

    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('deleteFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // Resynchronize with S3 to ensure consistency
    await appStore.reloadCurrentView()
  }
}

// Handle file drop using Tauri's event system (NEW: Rust-managed uploads)
async function handleFileDrop(paths: string[]) {
  if (!appStore.currentProfile || !appStore.currentBucket) {
    logger.error('No profile or bucket selected')
    return
  }

  // Capture values to prevent null reference errors during async operations
  const profileId = appStore.currentProfile.id
  const bucket = appStore.currentBucket
  const prefix = appStore.currentPrefix

  logger.debug(`Starting upload of ${paths.length} file(s) via drag & drop...`)

  // Helper to detect content type from file extension
  const getContentType = (fileName: string): string | undefined => {
    const ext = fileName.split('.').pop()?.toLowerCase()
    if (!ext) return undefined

    const contentTypes: Record<string, string> = {
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      png: 'image/png',
      gif: 'image/gif',
      pdf: 'application/pdf',
      txt: 'text/plain',
      json: 'application/json',
      xml: 'application/xml',
      zip: 'application/zip',
      mp4: 'video/mp4',
    }

    return contentTypes[ext]
  }

  // Queue all uploads immediately - Rust upload manager handles concurrency
  // Don't wait for uploads to start, just queue them all
  paths.forEach((filePath) => {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file'
    const key = prefix + fileName
    const contentType = getContentType(fileName)

    // Fire and forget - just queue the upload
    rustUploadManager.startUpload(
      profileId,
      bucket,
      key,
      filePath,
      contentType
    ).catch((e) => {
      // Ignore cancellation errors (user clicked cancel)
      if (e && e.message && e.message.includes('cancelled')) {
        return
      }
      logger.error(`✗ Failed to queue upload ${fileName}:`, e)
      toast.error(`Failed to queue: ${fileName}`)
    })
  })

  // Show immediate feedback
  logger.debug(`Queued ${paths.length} file(s) for drag & drop upload`)
  toast.success(`Queued ${paths.length} file(s) for upload`)

  // Objects will be reloaded automatically when uploads complete
}

// Context menu functions
function showContextMenu(event: MouseEvent, obj: S3Object) {
  event.preventDefault()
  contextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
    object: obj,
  }
}

function closeContextMenu() {
  contextMenu.value.show = false
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
  // Validate before renaming
  if (!rename.newName.trim() || rename.validationError) return
  if (!rename.object || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    rename.isRenaming = true

    const oldKey = rename.object.key
    const path = oldKey.substring(0, oldKey.lastIndexOf('/') + 1)
    const newKey = path + rename.newName.trim()

    // S3 doesn't have rename, so we copy then delete
    // First, get the object
    const { getObject } = await import('../services/tauri')
    const objectData = await getObject(appStore.currentProfile.id, appStore.currentBucket, oldKey)

    // Copy to new key
    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      newKey,
      objectData.content,
      objectData.content_type
    )

    // Delete old key
    await deleteObject(appStore.currentProfile.id, appStore.currentBucket, oldKey)

    // Optimistic update: remove old object and add new one
    appStore.removeObject(oldKey)

    const newObj: S3Object = {
      key: newKey,
      size: rename.object.size,
      last_modified: new Date().toISOString(),
      e_tag: '',
      storage_class: rename.object.storage_class || 'STANDARD',
      is_folder: false,
    }
    appStore.addObject(newObj)

    // Close modal and reset
    modals.rename = false
    rename.newName = ''
    rename.object = null
    rename.validationError = ''
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('renameFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // On error, reload view to resynchronize with S3
    await appStore.reloadCurrentView()
  } finally {
    rename.isRenaming = false
  }
}

async function changeContentTypeDirectly(contentType: string) {
  if (!contextMenu.value.object || !appStore.currentProfile || !appStore.currentBucket) return

  try {
    changingContentType.value = true
    const key = contextMenu.value.object.key

    // Get the object content
    const { getObject } = await import('../services/tauri')
    const objectData = await getObject(appStore.currentProfile.id, appStore.currentBucket, key)

    // Put it back with new content type
    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      key,
      objectData.content,
      contentType
    )

    // Note: S3Object doesn't store content_type, so we can't update it optimistically
    // The change is effective on S3, user will see it on next navigation/refresh

    // Close menus
    closeContextMenu()

    // Show success message
    toast.success(t('contentTypeUpdated'))
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `Failed to change content type: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // On error, reload view to resynchronize with S3
    await appStore.reloadCurrentView()
  } finally {
    changingContentType.value = false
  }
}

// Check if an object has multiple versions loaded
function hasMultipleVersions(obj: S3Object): boolean {
  const versions = inlineVersions.value.get(obj.key)
  return versions !== undefined && versions.length > 1
}

// Toggle inline version expansion for a file
async function toggleInlineVersions(obj: S3Object, event?: Event) {
  if (event) {
    event.stopPropagation()
  }

  const key = obj.key

  // If already expanded, collapse it
  if (expandedVersions.value.has(key)) {
    expandedVersions.value.delete(key)
    expandedVersions.value = new Set(expandedVersions.value) // Trigger reactivity
    return
  }

  // If not expanded, expand and load versions if not already loaded
  expandedVersions.value.add(key)
  expandedVersions.value = new Set(expandedVersions.value) // Trigger reactivity

  // If versions are already loaded, no need to fetch again
  if (inlineVersions.value.has(key)) {
    return
  }

  // Load versions
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    loadingInlineVersions.value.add(key)
    loadingInlineVersions.value = new Set(loadingInlineVersions.value) // Trigger reactivity

    const response = await listObjectVersions(
      appStore.currentProfile.id,
      appStore.currentBucket,
      key
    )

    // Check if there's only one version
    if (response.versions.length <= 1) {
      // Don't expand if there's only one version
      expandedVersions.value.delete(key)
      expandedVersions.value = new Set(expandedVersions.value)
      toast.info(t('onlyOneVersion'))
      return
    }

    inlineVersions.value.set(key, response.versions)
    inlineVersions.value = new Map(inlineVersions.value) // Trigger reactivity
  } catch (e) {
    toast.error(`${t('errorLoadingVersions')}: ${e}`)
    // Remove from expanded if loading failed
    expandedVersions.value.delete(key)
    expandedVersions.value = new Set(expandedVersions.value)
  } finally {
    loadingInlineVersions.value.delete(key)
    loadingInlineVersions.value = new Set(loadingInlineVersions.value)
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
      variant: 'destructive',
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
      filters: [],
    })

    if (!filePath) return

    // Note: You may need to update the backend to support downloading specific versions
    // For now, this downloads the object with the version ID
    // The backend's getObject function would need to accept an optional versionId parameter
    const { getObject } = await import('../services/tauri')
    const response = await getObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      version.key
    )

    await writeBinaryFile(filePath, new Uint8Array(response.content))
    toast.success(t('fileDownloadedSuccess'))
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('downloadFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })
  }
}

// Empty area context menu functions
function showEmptyContextMenu(event: MouseEvent) {
  emptyContextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
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
  modals.createFile = true
  closeEmptyContextMenu()
}

function openCreateFolderModalFromContext() {
  modals.createFolder = true
  closeEmptyContextMenu()
}

async function createFileHandler() {
  // Validate before creating
  if (!fileCreation.name.trim() || fileCreation.validationError) return
  if (!appStore.currentProfile || !appStore.currentBucket) return

  try {
    fileCreation.isCreating = true

    const filePath = appStore.currentPrefix + fileCreation.name.trim()
    const content = fileCreation.content || ''
    const encoder = new TextEncoder()
    const bytes = Array.from(encoder.encode(content))

    // Create file on S3
    await putObject(
      appStore.currentProfile.id,
      appStore.currentBucket,
      filePath,
      bytes,
      'text/plain'
    )

    // Optimistic update: add file immediately to local store
    const newObj: S3Object = {
      key: filePath,
      size: bytes.length,
      last_modified: new Date().toISOString(),
      e_tag: '',
      storage_class: 'STANDARD',
      is_folder: false,
    }
    appStore.addObject(newObj)

    // Close modal and reset
    modals.createFile = false
    fileCreation.name = ''
    fileCreation.content = ''
    fileCreation.validationError = ''
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('createFileFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // On error, reload view to resynchronize with S3
    await appStore.reloadCurrentView()
  } finally {
    fileCreation.isCreating = false
  }
}

/**
 * Handle upload completion optimistically
 * Adds uploaded files to local store without reloading from S3
 */
function handleUploadCompleted(event: Event) {
  const customEvent = event as CustomEvent<{
    bucket: string
    key: string
    size: number
    contentType?: string
  }>

  const { bucket, key, size, contentType: _contentType } = customEvent.detail

  logger.debug('[Upload] Object uploaded, adding optimistically:', key)

  // Only add if it's in the current bucket and current prefix
  if (bucket !== appStore.currentBucket) {
    logger.debug('[Upload] Skipping - different bucket')
    return
  }

  if (!key.startsWith(appStore.currentPrefix)) {
    logger.debug('[Upload] Skipping - different prefix')
    return
  }

  // Add object optimistically
  const newObj: S3Object = {
    key,
    size,
    last_modified: new Date().toISOString(),
    e_tag: '',
    storage_class: 'STANDARD',
    is_folder: false,
  }

  appStore.addObject(newObj)
  logger.debug('[Upload] Object added optimistically:', key)
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
    const existingFile = appStore.objects.find((obj) => obj.key === destKey)
    if (existingFile) {
      const confirmed = await dialog.confirm({
        title: t('paste'),
        message: t('fileExistsConfirm').replace('{0}', fileName),
        confirmText: t('paste'),
        cancelText: t('cancel'),
        variant: 'destructive',
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

    // Optimistic update: add copied file immediately to local store
    const sourceObj = copiedFile.value
    const newObj: S3Object = {
      key: destKey,
      size: sourceObj.size,
      last_modified: new Date().toISOString(),
      e_tag: '',
      storage_class: sourceObj.storage_class || 'STANDARD',
      is_folder: false,
    }
    appStore.addObject(newObj)
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('pasteFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })

    // On error, reload view to resynchronize with S3
    await appStore.reloadCurrentView()
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

    const allItems = [...filteredFolders.value, ...filteredObjects.value.map((o) => o.key)]
    for (let i = start; i <= end; i++) {
      selectedItems.value.add(allItems[i])
    }
  } else {
    // Normal click: Select this item only
    selectedItems.value.clear()
    selectedItems.value.add(obj.key)
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

    const allItems = [...filteredFolders.value, ...filteredObjects.value.map((o) => o.key)]
    for (let i = start; i <= end; i++) {
      selectedItems.value.add(allItems[i])
    }
  } else {
    // Normal click: Select this folder only
    selectedItems.value.clear()
    selectedItems.value.add(folder)
    lastSelectedIndex.value = index
  }
}

function clearSelection() {
  // Don't clear if we just finished a selection box drag
  if (justFinishedSelection.value) return

  selectedItems.value.clear()
  lastSelectedIndex.value = -1
}

// Selection box functions
function handleSelectionMouseDown(event: MouseEvent) {
  // Only start selection on left click in empty area
  if (event.button !== 0) return

  // Check if clicking on an empty area (not on an object)
  const target = event.target as HTMLElement
  if (target.closest('[data-object-row]')) return

  isDrawingSelection.value = true
  const rect = objectListRef.value?.getBoundingClientRect()
  if (!rect) return

  selectionBox.value = {
    startX: event.clientX - rect.left + (objectListRef.value?.scrollLeft || 0),
    startY: event.clientY - rect.top + (objectListRef.value?.scrollTop || 0),
    endX: event.clientX - rect.left + (objectListRef.value?.scrollLeft || 0),
    endY: event.clientY - rect.top + (objectListRef.value?.scrollTop || 0),
  }

  // Clear previous selection if not holding Ctrl/Cmd
  if (!event.ctrlKey && !event.metaKey) {
    selectedItems.value.clear()
  }
}

function handleSelectionMouseMove(event: MouseEvent) {
  if (!isDrawingSelection.value) return

  const rect = objectListRef.value?.getBoundingClientRect()
  if (!rect) return

  selectionBox.value.endX = event.clientX - rect.left + (objectListRef.value?.scrollLeft || 0)
  selectionBox.value.endY = event.clientY - rect.top + (objectListRef.value?.scrollTop || 0)

  // Update selected items based on selection box
  updateSelectionFromBox()
}

function handleSelectionMouseUp() {
  if (isDrawingSelection.value) {
    // Mark that we just finished a selection to prevent clearSelection from firing
    const box = selectionBox.value
    const moved = Math.abs(box.endX - box.startX) > 5 || Math.abs(box.endY - box.startY) > 5
    if (moved) {
      justFinishedSelection.value = true
      setTimeout(() => {
        justFinishedSelection.value = false
      }, 100)
    }
  }
  isDrawingSelection.value = false
}

function updateSelectionFromBox() {
  if (!objectListRef.value) return

  const box = selectionBox.value
  const minX = Math.min(box.startX, box.endX)
  const maxX = Math.max(box.startX, box.endX)
  const minY = Math.min(box.startY, box.endY)
  const maxY = Math.max(box.startY, box.endY)

  // Get all object rows
  const rows = objectListRef.value.querySelectorAll('[data-object-row]')
  const newSelection = new Set<string>()

  rows.forEach((row) => {
    const rect = row.getBoundingClientRect()
    const containerRect = objectListRef.value!.getBoundingClientRect()

    const rowLeft = rect.left - containerRect.left + (objectListRef.value?.scrollLeft || 0)
    const rowTop = rect.top - containerRect.top + (objectListRef.value?.scrollTop || 0)
    const rowRight = rowLeft + rect.width
    const rowBottom = rowTop + rect.height

    // Check if row intersects with selection box
    if (!(rowRight < minX || rowLeft > maxX || rowBottom < minY || rowTop > maxY)) {
      const key = row.getAttribute('data-object-key')
      if (key) newSelection.add(key)
    }
  })

  selectedItems.value = newSelection
}

const selectionBoxStyle = computed(() => {
  if (!isDrawingSelection.value) return { display: 'none' }

  const box = selectionBox.value
  const left = Math.min(box.startX, box.endX)
  const top = Math.min(box.startY, box.endY)
  const width = Math.abs(box.endX - box.startX)
  const height = Math.abs(box.endY - box.startY)

  return {
    display: 'block',
    position: 'absolute' as const,
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
    height: `${height}px`,
    border: '2px solid #3b82f6',
    backgroundColor: 'rgba(59, 130, 246, 0.1)',
    pointerEvents: 'none' as const,
    zIndex: 1000,
  }
})

async function copySelectedItems() {
  // For now, only copy the first selected file (folders cannot be copied individually)
  const selectedFiles = Array.from(selectedItems.value).filter((key) =>
    appStore.objects.some((obj) => obj.key === key)
  )

  if (selectedFiles.length === 0) {
    await dialog.confirm({
      title: t('copy'),
      message: t('cannotCopyFolders'),
      confirmText: t('close'),
    })
    return
  }

  if (selectedFiles.length > 1) {
    await dialog.confirm({
      title: t('copy'),
      message: t('multiCopyNotSupported'),
      confirmText: t('close'),
    })
    return
  }

  const fileObj = appStore.objects.find((obj) => obj.key === selectedFiles[0])
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
    variant: 'destructive',
  })

  if (!confirmed) return

  const toastId = toast.loading(`${t('deleting')} 0/${selectedCount}`)

  try {
    let successCount = 0
    let failCount = 0

    for (const key of selectedItems.value) {
      try {
        // Check if it's a folder or a file
        const isFolder = filteredFolders.value.includes(key)

        if (isFolder) {
          await deleteFolder(appStore.currentProfile.id, appStore.currentBucket, key)
          // Optimistic update: remove folder immediately
          appStore.removeFolder(key)
        } else {
          await deleteObject(appStore.currentProfile.id, appStore.currentBucket, key)
          // Optimistic update: remove object immediately
          appStore.removeObject(key)
        }
        successCount++

        // Update progress
        const totalProcessed = successCount + failCount
        const progress = Math.round((totalProcessed / selectedCount) * 100)
        toast.updateToast(toastId, {
          message: `${t('deleting')} ${totalProcessed}/${selectedCount}`,
          progress,
        })
      } catch (e) {
        logger.error(`Failed to delete ${key}:`, e)
        failCount++

        // Update progress even on failure
        const totalProcessed = successCount + failCount
        const progress = Math.round((totalProcessed / selectedCount) * 100)
        toast.updateToast(toastId, {
          message: `${t('deleting')} ${totalProcessed}/${selectedCount}`,
          progress,
        })
      }
    }

    clearSelection()

    // If some deletions failed, reload view to resynchronize with S3
    if (failCount > 0) {
      await appStore.reloadCurrentView()
    }

    if (failCount === 0) {
      toast.completeToast(
        toastId,
        t('deleteSuccess')
          .replace('{0}', String(successCount))
          .replace('{1}', successCount > 1 ? 's' : ''),
        'success'
      )
    } else {
      toast.completeToast(
        toastId,
        t('deletePartialSuccess')
          .replace('{0}', String(successCount))
          .replace('{1}', String(failCount)),
        failCount < selectedCount ? 'success' : 'error'
      )
    }
  } catch (e) {
    toast.completeToast(toastId, `${t('deleteOperationFailed')}: ${e}`, 'error')
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('deleteOperationFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })
  }
}

async function downloadSelectedItems() {
  if (!appStore.currentProfile || !appStore.currentBucket) return

  // Filter to get only files (exclude folders)
  const selectedFiles = Array.from(selectedItems.value).filter((key) =>
    appStore.objects.some((obj) => obj.key === key)
  )

  if (selectedFiles.length === 0) {
    toast.warning(t('noFilesToDownload'))
    return
  }

  try {
    // Ask user to select a folder to save all files
    const folderPath = await save({
      defaultPath: 'downloads',
      title: t('selectDownloadFolder'),
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

    // Create a persistent progress toast
    const progressToastId = toast.loading(`${t('downloading')} 0/${selectedFiles.length}`)

    for (const key of selectedFiles) {
      try {
        const fileName = getFileName(key)
        const filePath = `${directory}/${fileName}`

        const response = await getObject(appStore.currentProfile.id, appStore.currentBucket, key)
        await writeBinaryFile(filePath, new Uint8Array(response.content))
        successCount++

        // Update progress toast
        const totalProcessed = successCount + failCount
        const progress = Math.round((totalProcessed / selectedFiles.length) * 100)
        toast.updateToast(progressToastId, {
          message: `${t('downloading')} ${totalProcessed}/${selectedFiles.length}`,
          progress,
        })
      } catch (e) {
        logger.error(`Failed to download ${key}:`, e)
        failCount++

        // Update progress toast even on failure
        const totalProcessed = successCount + failCount
        const progress = Math.round((totalProcessed / selectedFiles.length) * 100)
        toast.updateToast(progressToastId, {
          message: `${t('downloading')} ${totalProcessed}/${selectedFiles.length}`,
          progress,
        })
      }
    }

    // Complete the progress toast
    clearSelection()

    if (failCount === 0) {
      toast.completeToast(
        progressToastId,
        t('filesDownloadedSuccess').replace('{0}', String(successCount)),
        'success'
      )
    } else {
      toast.completeToast(
        progressToastId,
        t('downloadPartialSuccess')
          .replace('{0}', String(successCount))
          .replace('{1}', String(failCount)),
        failCount < selectedFiles.length ? 'success' : 'error'
      )
    }
  } catch (e) {
    await dialog.confirm({
      title: t('errorOccurred'),
      message: `${t('downloadFailed')}: ${e}`,
      confirmText: t('close'),
      variant: 'destructive',
    })
  }
}

// Select all items
function selectAllItems() {
  selectedItems.value.clear()

  // Add all folders
  filteredFolders.value.forEach((folder) => {
    selectedItems.value.add(folder)
  })

  // Add all files
  filteredObjects.value.forEach((obj) => {
    selectedItems.value.add(obj.key)
  })
}

// Keyboard event handler
function handleKeyDown(event: KeyboardEvent) {
  // Check if the active element is an input, textarea, or contentEditable
  const activeElement = document.activeElement
  const isInputField =
    activeElement instanceof HTMLInputElement ||
    activeElement instanceof HTMLTextAreaElement ||
    (activeElement as HTMLElement)?.isContentEditable

  // Check for Cmd+F (macOS) or Ctrl+F (other OS) - Focus search bar
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
    // Prevent default browser search
    event.preventDefault()
    // Focus the search input (need to access $el because Input is a Vue component)
    if (searchInputRef.value) {
      const inputElement = (searchInputRef.value as any).$el as HTMLInputElement
      if (inputElement) {
        inputElement.focus()
        // Select all text in the search input for easy replacement
        inputElement.select()
      }
    }
  }
  // Check for Cmd+A (macOS) or Ctrl+A (other OS)
  else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
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

// Close search settings menu when clicking outside
function handleClickOutside(event: MouseEvent) {
  if (
    showSearchSettings.value &&
    searchSettingsButtonRef.value &&
    !searchSettingsButtonRef.value.contains(event.target as Node)
  ) {
    const menu = document.querySelector('.fixed.z-50.min-w-\\[220px\\]')
    if (menu && !menu.contains(event.target as Node)) {
      showSearchSettings.value = false
    }
  }
}

// Setup event listeners on mount
onMounted(async () => {
  // Add keyboard event listener
  window.addEventListener('keydown', handleKeyDown)
  // Add click outside listener for search settings menu
  window.addEventListener('click', handleClickOutside)
  // Add upload completion listener for optimistic updates
  window.addEventListener('upload:object-completed', handleUploadCompleted)

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
  window.removeEventListener('click', handleClickOutside)
  window.removeEventListener('upload:object-completed', handleUploadCompleted)
  if (unlistenFileDrop) unlistenFileDrop()
  if (unlistenFileDropHover) unlistenFileDropHover()
  if (unlistenFileDropCancelled) unlistenFileDropCancelled()
})
</script>

<style scoped>
@keyframes progress-bar {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
}

@keyframes rotate {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.animate-progress-bar {
  animation: progress-bar 1.5s ease-in-out infinite;
}
</style>
