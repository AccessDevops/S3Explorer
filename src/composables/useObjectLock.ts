import { ref } from 'vue'
import { getObjectLockStatus } from '../services/tauri'
import type { ObjectLockStatus } from '../types'

/**
 * Composable for fetching S3 Object Lock status
 *
 * Fetches fresh data from S3 each time (no caching).
 * Used in the ObjectViewer modal to display lock status.
 */
export function useObjectLock() {
  const lockStatus = ref<ObjectLockStatus | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  /**
   * Fetch object lock status from S3
   * Makes 2 API calls: GetObjectRetention + GetObjectLegalHold
   */
  async function fetchLockStatus(
    profileId: string,
    bucket: string,
    key: string
  ): Promise<ObjectLockStatus | null> {
    isLoading.value = true
    error.value = null
    lockStatus.value = null

    try {
      const status = await getObjectLockStatus(profileId, bucket, key)
      lockStatus.value = status
      return status
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      error.value = message
      // Return unlocked status on error (bucket may not support Object Lock)
      lockStatus.value = {
        is_locked: false,
        retention_mode: null,
        retain_until_date: null,
        legal_hold: false,
      }
      return lockStatus.value
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Reset the lock status state
   */
  function reset() {
    lockStatus.value = null
    isLoading.value = false
    error.value = null
  }

  return {
    lockStatus,
    isLoading,
    error,
    fetchLockStatus,
    reset,
  }
}
