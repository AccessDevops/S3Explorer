import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Profile, Bucket, S3Object } from '../types'
import * as tauriService from '../services/tauri'

export const useAppStore = defineStore('app', () => {
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

  // Computed
  const hasProfile = computed(() => currentProfile.value !== null)
  const hasBucket = computed(() => currentBucket.value !== null)

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

    try {
      isLoading.value = true
      error.value = null
      buckets.value = await tauriService.listBuckets(currentProfile.value.id)
    } catch (e) {
      error.value = String(e)
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
  }

  // Navigate to a folder
  function navigateToFolder(prefix: string) {
    currentPrefix.value = prefix
    objects.value = []
    folders.value = []
    continuationToken.value = undefined
  }

  // Load objects in current bucket/prefix
  async function loadObjects(loadMore = false) {
    if (!currentProfile.value || !currentBucket.value) return

    try {
      isLoading.value = true
      error.value = null

      const response = await tauriService.listObjects(
        currentProfile.value.id,
        currentBucket.value,
        currentPrefix.value || undefined,
        loadMore ? continuationToken.value : undefined,
        100,
        true // Use delimiter for folder navigation
      )

      if (loadMore) {
        objects.value.push(...response.objects)
      } else {
        objects.value = response.objects
        folders.value = response.common_prefixes
      }

      continuationToken.value = response.continuation_token
    } catch (e) {
      error.value = String(e)
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
    // Computed
    hasProfile,
    hasBucket,
    // Actions
    loadProfiles,
    selectProfile,
    saveProfileData,
    removeProfile,
    loadBuckets,
    selectBucket,
    navigateToFolder,
    loadObjects,
    refresh,
    clearError,
  }
})
