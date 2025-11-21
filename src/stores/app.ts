import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Profile, Bucket, S3Object } from '../types'
import * as tauriService from '../services/tauri'
import { useToast } from '../composables/useToast'
import { useSettingsStore } from './settings'

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

  // Navigation history for back button
  const navigationHistory = ref<Array<{ bucket: string; prefix: string }>>([])

  // Preload cache for next page (performance optimization)
  const preloadedNextPage = ref<{
    objects: S3Object[]
    continuationToken?: string
    forToken?: string // The continuation token this preload is for
  } | null>(null)

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
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  // Select a profile
  function selectProfile(profile: Profile) {
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
      error.value = String(e)
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
      if (currentProfile.value?.id === profileId) {
        currentProfile.value = null
        buckets.value = []
        currentBucket.value = null
      }
      await loadProfiles()
    } catch (e) {
      error.value = String(e)
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
      error.value = String(e)
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
  }

  // Navigate to a folder
  function navigateToFolder(prefix: string) {
    // Save current state to history before navigating
    if (currentBucket.value) {
      navigationHistory.value.push({
        bucket: currentBucket.value,
        prefix: currentPrefix.value,
      })
    }

    currentPrefix.value = prefix
    objects.value = []
    folders.value = []
    continuationToken.value = undefined
    preloadedNextPage.value = null // Clear preload cache on navigation
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

    // Load the previous folder's content
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
        true
      )

      preloadedNextPage.value = {
        objects: response.objects,
        continuationToken: response.continuation_token,
        forToken: continuationToken.value,
      }
    } catch (e) {
      // Silent fail for preload - not critical
      console.warn('Failed to preload next page:', e)
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
        continuationToken.value = cachedPage!.continuationToken
        preloadedNextPage.value = null
        isLoading.value = false

        // Preload the next page in background
        if (continuationToken.value) {
          preloadNextPage()
        }
        return
      }

      // Normal load (first page or cache miss)
      const response = await tauriService.listObjects(
        currentProfile.value.id,
        currentBucket.value,
        currentPrefix.value || undefined,
        loadMore ? continuationToken.value : undefined,
        settingsStore.batchSize, // Use batch size from settings
        true // Use delimiter for folder navigation
      )

      if (loadMore) {
        objects.value.push(...response.objects)
      } else {
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
    } catch (e) {
      error.value = String(e)
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

  // Refresh current view
  async function refresh() {
    if (currentBucket.value) {
      await loadObjects()
    } else if (currentProfile.value) {
      await loadBuckets()
    }
  }

  // Clear error
  function clearError() {
    error.value = null
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
    refresh,
    clearError,
  }
})
