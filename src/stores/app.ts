import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Profile, Bucket, S3Object } from '../types'
import { AppError } from '../types/errors'
import * as tauriService from '../services/tauri'
import { useToast } from '../composables/useToast'
import { useSettingsStore } from './settings'
import { logger } from '../utils/logger'

export const useAppStore = defineStore('app', () => {
  const toast = useToast()
  const settingsStore = useSettingsStore()

  // State
  const profiles = ref<Profile[]>([])
  const currentProfile = ref<Profile | null>(null)
  const buckets = ref<Bucket[]>([])
  const currentBucket = ref<string | null>(null)
  const currentPrefix = ref<string>('')
  const objects = ref<S3Object[]>([])
  const folders = ref<string[]>([])
  const continuationToken = ref<string | undefined>(undefined)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const loadingProgress = ref({ show: false, message: '' })
  const loadedPagesCount = ref(1) // Track number of loaded pages for smart refresh

  // Navigation history for back button (includes objects/folders state)
  const navigationHistory = ref<Array<{
    bucket: string
    prefix: string
    objects: S3Object[]
    folders: string[]
    continuationToken?: string
  }>>([])

  // Preload cache for next page (performance optimization)
  const preloadedNextPage = ref<{
    objects: S3Object[]
    folders: string[] // Common prefixes (folders)
    continuationToken?: string
    forToken?: string // The continuation token this preload is for
  } | null>(null)

  // ACL cache with TTL (5 minutes) - reduces GetBucketAcl requests by ~90%
  const ACL_CACHE_TTL = 5 * 60 * 1000 // 5 minutes in milliseconds
  interface AclCacheEntry {
    acl: string
    timestamp: number
  }
  const aclCache = ref<Map<string, AclCacheEntry>>(new Map())

  // Current operation toast ID
  let currentLoadToastId: string | null = null

  // Computed
  const hasProfile = computed(() => currentProfile.value !== null)
  const hasBucket = computed(() => currentBucket.value !== null)
  const canGoBack = computed(() => navigationHistory.value.length > 0)

  // Actions

  // Load all profiles
  async function loadProfiles() {
    try {
      isLoading.value = true
      error.value = null
      profiles.value = await tauriService.listProfiles()
    } catch (e) {
      error.value = AppError.fromUnknown(e).message
      throw e
    } finally {
      isLoading.value = false
    }
  }

  // Select a profile
  function selectProfile(profile: Profile) {
    // Invalidate ACL cache for previous profile (if any)
    if (currentProfile.value && currentProfile.value.id !== profile.id) {
      invalidateAclCache(currentProfile.value.id)
    }
    currentProfile.value = profile
    buckets.value = []
    currentBucket.value = null
    objects.value = []
    folders.value = []
  }

  // Add or update a profile
  async function saveProfileData(profile: Profile) {
    try {
      isLoading.value = true
      error.value = null
      await tauriService.saveProfile(profile)
      await loadProfiles()
    } catch (e) {
      error.value = AppError.fromUnknown(e).message
      throw e
    } finally {
      isLoading.value = false
    }
  }

  // Delete a profile
  async function removeProfile(profileId: string) {
    try {
      isLoading.value = true
      error.value = null
      await tauriService.deleteProfile(profileId)

      // Cleanup backend cache for this profile (DatabaseManager + IndexManager)
      // This frees SQLite connections and memory immediately instead of waiting for LRU eviction
      try {
        await tauriService.cleanupProfileCache(profileId)
        logger.debug(`[Cache] Cleaned up cache for deleted profile: ${profileId}`)
      } catch (cacheError) {
        // Non-critical - cache will eventually be evicted by TTL
        logger.warn(`[Cache] Failed to cleanup cache for profile ${profileId}`, cacheError)
      }

      if (currentProfile.value?.id === profileId) {
        currentProfile.value = null
        buckets.value = []
        currentBucket.value = null
      }
      await loadProfiles()
    } catch (e) {
      error.value = AppError.fromUnknown(e).message
      throw e
    } finally {
      isLoading.value = false
    }
  }

  // Load buckets for current profile
  async function loadBuckets() {
    if (!currentProfile.value) return

    const toastId = toast.loading('Loading buckets...')

    try {
      isLoading.value = true
      error.value = null
      buckets.value = await tauriService.listBuckets(currentProfile.value.id)
      toast.completeToast(toastId, `Loaded ${buckets.value.length} bucket(s)`, 'success')
    } catch (e) {
      error.value = AppError.fromUnknown(e).message
      toast.completeToast(toastId, 'Failed to load buckets', 'error')
      throw e
    } finally {
      isLoading.value = false
    }
  }

  // Select a bucket
  function selectBucket(bucketName: string) {
    currentBucket.value = bucketName
    currentPrefix.value = ''
    objects.value = []
    folders.value = []
    continuationToken.value = undefined
    preloadedNextPage.value = null // Clear preload cache on bucket change
    navigationHistory.value = [] // Clear history on bucket change
    loadedPagesCount.value = 1 // Reset page counter
  }

  // Navigate to a folder
  function navigateToFolder(prefix: string) {
    // Save current state to history before navigating (including objects/folders)
    if (currentBucket.value) {
      navigationHistory.value.push({
        bucket: currentBucket.value,
        prefix: currentPrefix.value,
        objects: [...objects.value], // Save current objects
        folders: [...folders.value], // Save current folders
        continuationToken: continuationToken.value,
      })

      // Limit history to 50 entries to prevent memory overflow
      const MAX_HISTORY = 50
      if (navigationHistory.value.length > MAX_HISTORY) {
        navigationHistory.value = navigationHistory.value.slice(-MAX_HISTORY)
      }
    }

    currentPrefix.value = prefix
    objects.value = []
    folders.value = []
    continuationToken.value = undefined
    preloadedNextPage.value = null // Clear preload cache on navigation
    loadedPagesCount.value = 1 // Reset page counter
  }

  // Go back to previous folder
  async function goBack() {
    if (navigationHistory.value.length === 0) return

    const previous = navigationHistory.value.pop()!

    // Restore previous state without adding to history
    currentBucket.value = previous.bucket
    currentPrefix.value = previous.prefix
    objects.value = []
    folders.value = []
    continuationToken.value = undefined
    preloadedNextPage.value = null
    loadedPagesCount.value = 1 // Reset page counter

    // Reload from S3 to ensure fresh data (no cache for sync with server)
    await loadObjects()
  }

  // Preload next page in background for instant pagination
  async function preloadNextPage() {
    if (!currentProfile.value || !currentBucket.value || !continuationToken.value) {
      preloadedNextPage.value = null
      return
    }

    try {
      const response = await tauriService.listObjects(
        currentProfile.value.id,
        currentBucket.value,
        currentPrefix.value || undefined,
        continuationToken.value,
        settingsStore.batchSize,
        true, // useDelimiter
        false // syncIndex = false for preload (don't trigger cleanup)
      )

      preloadedNextPage.value = {
        objects: response.objects,
        folders: response.common_prefixes,
        continuationToken: response.continuation_token,
        forToken: continuationToken.value,
      }
    } catch (e) {
      // Silent fail for preload - not critical
      logger.warn('Failed to preload next page', e)
      preloadedNextPage.value = null
    }
  }

  // Load objects in current bucket/prefix
  async function loadObjects(loadMore = false) {
    if (!currentProfile.value || !currentBucket.value) return

    // Don't show progress for cached loads
    const cachedPage = preloadedNextPage.value
    const isUsingCache = loadMore && cachedPage?.forToken === continuationToken.value

    if (!isUsingCache && !loadMore) {
      // Show progress bar for initial loads
      loadingProgress.value = {
        show: true,
        message: `Loading objects from ${currentBucket.value}...`,
      }
    }

    try {
      isLoading.value = true
      error.value = null

      // Check if we have a preloaded page for this continuation token
      if (isUsingCache) {
        // Use preloaded data - instant!
        objects.value.push(...cachedPage!.objects)

        // Merge folders from cache
        if (cachedPage!.folders && cachedPage!.folders.length > 0) {
          const newFolders = cachedPage!.folders.filter(
            folder => !folders.value.includes(folder)
          )
          if (newFolders.length > 0) {
            folders.value.push(...newFolders)
          }
        }

        continuationToken.value = cachedPage!.continuationToken
        preloadedNextPage.value = null
        isLoading.value = false

        // Preload the next page in background
        if (continuationToken.value) {
          preloadNextPage()
        }

        // Track loaded pages for smart refresh (cached load is always loadMore=true)
        loadedPagesCount.value++

        return
      }

      // Normal load (first page or cache miss)
      const response = await tauriService.listObjects(
        currentProfile.value.id,
        currentBucket.value,
        currentPrefix.value || undefined,
        loadMore ? continuationToken.value : undefined,
        settingsStore.batchSize, // Use batch size from settings
        true, // Use delimiter for folder navigation
        !loadMore // syncIndex = true only for first page to cleanup phantom objects
      )

      if (loadMore) {
        objects.value.push(...response.objects)
        logger.debug(`[DEBUG] loadObjects(loadMore=true): Got ${response.objects.length} objects, ${response.common_prefixes.length} folders`)
        // Update folders on each page - some S3 implementations may return different folders per page
        if (response.common_prefixes && response.common_prefixes.length > 0) {
          // Merge folders: add new ones that aren't already in the list
          const newFolders = response.common_prefixes.filter(
            folder => !folders.value.includes(folder)
          )
          if (newFolders.length > 0) {
            logger.debug(`[DEBUG] Adding ${newFolders.length} new folders:`, newFolders)
            folders.value.push(...newFolders)
          }
        }
      } else {
        logger.debug(`[DEBUG] loadObjects(loadMore=false): Got ${response.objects.length} objects, ${response.common_prefixes.length} folders`)
        logger.debug(`[DEBUG] Folders from API:`, response.common_prefixes)
        objects.value = response.objects
        folders.value = response.common_prefixes
        // Clear preload cache when navigating to new folder
        preloadedNextPage.value = null
      }

      continuationToken.value = response.continuation_token

      // Hide progress bar after successful load
      if (!loadMore) {
        // Show briefly the count, then hide
        const totalCount = objects.value.length
        const hasMore = continuationToken.value ? ' (more available)' : ''
        loadingProgress.value.message = `Loaded ${totalCount} object(s)${hasMore}`

        setTimeout(() => {
          loadingProgress.value.show = false
        }, 1500)
      }

      // Preload next page in background if there's more data
      if (continuationToken.value) {
        preloadNextPage()
      }

      // Track loaded pages for smart refresh
      if (loadMore) {
        loadedPagesCount.value++
      } else {
        loadedPagesCount.value = 1
      }
    } catch (e) {
      error.value = AppError.fromUnknown(e).message
      loadingProgress.value.show = false // Hide progress bar on error
      if (currentLoadToastId) {
        toast.completeToast(currentLoadToastId, 'Failed to load objects', 'error')
        currentLoadToastId = null
      }
      throw e
    } finally {
      isLoading.value = false
    }
  }

  // Reload current view (reset pagination and load first page from S3)
  async function reloadCurrentView() {
    if (!currentProfile.value || !currentBucket.value) return

    // Reset pagination state
    continuationToken.value = undefined
    objects.value = []
    folders.value = []
    preloadedNextPage.value = null
    loadedPagesCount.value = 1 // Reset page counter

    // Load first page from S3 to ensure synchronization with server
    await loadObjects()
  }

  // Remove object from store (optimistic deletion)
  function removeObject(key: string) {
    const index = objects.value.findIndex(obj => obj.key === key)
    if (index !== -1) {
      objects.value.splice(index, 1)
      logger.debug(`[Optimistic] Removed object: ${key}`)
    }
  }

  // Remove folder from store (optimistic deletion)
  function removeFolder(folder: string) {
    const index = folders.value.indexOf(folder)
    if (index !== -1) {
      folders.value.splice(index, 1)
      logger.debug(`[Optimistic] Removed folder: ${folder}`)
    }
  }

  // Add object to store (optimistic creation)
  function addObject(obj: S3Object) {
    // Check if object already exists (avoid duplicates)
    const exists = objects.value.find(o => o.key === obj.key)
    if (!exists) {
      objects.value.push(obj)
      logger.debug(`[Optimistic] Added object: ${obj.key}`)
    } else {
      logger.debug(`[Optimistic] Object already exists: ${obj.key}`)
    }
  }

  // Add folder to store (optimistic creation)
  function addFolder(folder: string) {
    // Check if folder already exists (avoid duplicates)
    if (!folders.value.includes(folder)) {
      folders.value.push(folder)
      logger.debug(`[Optimistic] Added folder: ${folder}`)
    } else {
      logger.debug(`[Optimistic] Folder already exists: ${folder}`)
    }
  }

  // Update object in store (for metadata changes)
  function updateObject(key: string, updates: Partial<S3Object>) {
    const index = objects.value.findIndex(obj => obj.key === key)
    if (index !== -1) {
      objects.value[index] = { ...objects.value[index], ...updates }
      logger.debug(`[Optimistic] Updated object: ${key}`)
    } else {
      logger.debug(`[Optimistic] Object not found for update: ${key}`)
    }
  }

  // Clear error
  function clearError() {
    error.value = null
  }

  // Get bucket ACL with caching (reduces requests by ~90%)
  async function getCachedBucketAcl(profileId: string, bucketName: string): Promise<string | null> {
    const cacheKey = `${profileId}:${bucketName}`
    const cached = aclCache.value.get(cacheKey)

    // Return cached value if valid (within TTL)
    if (cached && Date.now() - cached.timestamp < ACL_CACHE_TTL) {
      return cached.acl
    }

    // Fetch from S3 and cache
    try {
      const acl = await tauriService.getBucketAcl(profileId, bucketName)
      aclCache.value.set(cacheKey, { acl, timestamp: Date.now() })
      return acl
    } catch (e) {
      logger.warn(`Failed to get ACL for bucket ${bucketName}`, e)
      return null
    }
  }

  // Invalidate ACL cache (call when profile changes or on manual refresh)
  function invalidateAclCache(profileId?: string) {
    if (profileId) {
      // Invalidate only entries for this profile
      for (const key of aclCache.value.keys()) {
        if (key.startsWith(`${profileId}:`)) {
          aclCache.value.delete(key)
        }
      }
    } else {
      // Invalidate all entries
      aclCache.value.clear()
    }
    logger.debug(`ACL cache invalidated${profileId ? ` for profile ${profileId}` : ' (all)'}`)
  }

  return {
    // State
    profiles,
    currentProfile,
    buckets,
    currentBucket,
    currentPrefix,
    objects,
    folders,
    continuationToken,
    isLoading,
    error,
    loadingProgress,
    navigationHistory,
    loadedPagesCount,
    // Computed
    hasProfile,
    hasBucket,
    canGoBack,
    // Actions
    loadProfiles,
    selectProfile,
    saveProfileData,
    removeProfile,
    loadBuckets,
    selectBucket,
    navigateToFolder,
    goBack,
    loadObjects,
    reloadCurrentView,
    removeObject,
    removeFolder,
    addObject,
    addFolder,
    updateObject,
    clearError,
    getCachedBucketAcl,
    invalidateAclCache,
  }
})
