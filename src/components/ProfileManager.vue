<template>
  <div class="p-4 border-b border-white/10">
    <div class="flex justify-between items-center mb-4">
      <h3
        class="text-lg font-semibold mr-2 cursor-pointer hover:text-primary transition-colors"
        @click="showConnectionListModal = true"
        v-tooltip="t('manageConnections')"
      >
        {{ t('connections') }}
      </h3>
      <div class="flex items-center gap-3">
        <!-- Search bar for filtering profiles -->
        <div class="relative">
          <Input
            v-model="profileSearchQuery"
            :placeholder="t('searchProfiles')"
            class="h-7 w-24 pr-7 text-sm border-0 bg-white/5 focus:bg-white/10 focus:ring-1 focus:ring-primary/50"
          />
          <button
            v-if="profileSearchQuery"
            @click="profileSearchQuery = ''"
            class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
            v-tooltip="t('clear')"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
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
        <Button size="sm" @click="openAddModal">
          <span class="mr-1">+</span> {{ t('add') }}
        </Button>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <div
        v-for="profile in filteredProfiles"
        :key="profile.id"
        class="flex justify-between items-center p-3 rounded-md cursor-pointer transition-colors group"
        :class="
          appStore.currentProfile?.id === profile.id
            ? 'bg-white/20'
            : 'bg-white/5 hover:bg-white/10'
        "
        @click="selectProfile(profile)"
        @mouseenter="handleProfileHover(profile)"
      >
        <div class="flex-1">
          <div class="font-medium">{{ profile.name }}</div>
        </div>
        <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button
            size="icon"
            variant="ghost"
            class="h-6 w-6 text-white/70 hover:text-white hover:bg-white/10"
            @click.stop="editProfileHandler(profile)"
            v-tooltip="t('edit')"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"
              />
              <path d="m15 5 4 4" />
            </svg>
          </Button>
          <Button
            size="icon"
            variant="ghost"
            class="h-6 w-6 text-white/70 hover:text-red-400 hover:bg-white/10"
            @click.stop="deleteProfileConfirm(profile)"
            v-tooltip="t('delete')"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M3 6h18" />
              <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
              <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
              <line x1="10" x2="10" y1="11" y2="17" />
              <line x1="14" x2="14" y1="11" y2="17" />
            </svg>
          </Button>
        </div>
      </div>
    </div>

    <!-- Connection List Modal -->
    <ConnectionListModal
      v-model:open="showConnectionListModal"
      @edit="editProfileHandler"
      @create="openAddModal"
    />

    <!-- Add/Edit Profile Modal -->
    <Dialog v-model:open="showAddModal">
      <DialogContent class="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle
            >{{ editingProfile ? t('edit') : t('add') }} {{ t('connectionProfile') }}</DialogTitle
          >
          <DialogDescription>
            {{ t('configureS3') }}
          </DialogDescription>
        </DialogHeader>

        <form @submit.prevent="saveProfile" class="space-y-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('profileName') }} *</label>
            <Input v-model="formData.name" required placeholder="My S3 Connection" />
          </div>

          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('endpoint') }} (optional)</label>
            <Input
              v-model="formData.endpoint"
              :placeholder="t('endpointPlaceholder')"
              :class="validationErrors.endpoint ? 'border-red-500' : validationWarnings.endpoint ? 'border-amber-500' : ''"
            />
            <p v-if="validationErrors.endpoint" class="text-xs text-red-600">
              {{ validationErrors.endpoint }}
            </p>
            <p v-else-if="validationWarnings.endpoint" class="text-xs text-amber-600">
              ⚠ {{ validationWarnings.endpoint }}
            </p>
            <p v-else class="text-xs text-muted-foreground">{{ t('endpointDescription') }}</p>
          </div>

          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('region') }} (optional)</label>
            <Input
              v-model="formData.region"
              placeholder="us-east-1"
              :class="validationErrors.region ? 'border-red-500' : validationWarnings.region ? 'border-amber-500' : ''"
            />
            <p v-if="validationErrors.region" class="text-xs text-red-600">
              {{ validationErrors.region }}
            </p>
            <p v-else-if="validationWarnings.region" class="text-xs text-amber-600">
              ⚠ {{ validationWarnings.region }}
            </p>
          </div>

          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('accessKey') }} *</label>
            <Input v-model="formData.access_key" required placeholder="AKIAIOSFODNN7EXAMPLE" />
          </div>

          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('secretKey') }} *</label>
            <Input
              v-model="formData.secret_key"
              type="password"
              required
              placeholder="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
            />
          </div>

          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('sessionToken') }} (optional)</label>
            <Input
              v-model="formData.session_token"
              type="password"
              :placeholder="t('sessionTokenPlaceholder')"
            />
          </div>

          <div class="flex items-center space-x-2">
            <input
              v-model="formData.path_style"
              type="checkbox"
              id="path-style"
              class="rounded border-gray-300"
            />
            <label for="path-style" class="text-sm font-medium">
              {{ t('pathStyle') }}
            </label>
          </div>

          <div
            v-if="testResult"
            class="p-3 rounded-md text-sm"
            :class="
              testResult.success
                ? 'bg-green-50 text-green-800 border border-green-200'
                : 'bg-red-50 text-red-800 border border-red-200'
            "
          >
            {{ testResult.message }}

            <!-- Suggestion to enable path_style -->
            <div
              v-if="testResult.suggest_path_style"
              class="mt-3 flex items-center gap-2 p-2 bg-blue-50 border border-blue-200 rounded"
            >
              <span class="text-blue-800 text-xs flex-1"
                >💡 Path-style addressing is required for this endpoint</span
              >
              <Button
                type="button"
                size="sm"
                @click="enablePathStyle"
                class="text-xs h-7"
                variant="default"
              >
                Enable Path Style
              </Button>
            </div>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" @click="testConnectionHandler" :disabled="isTesting">
              <svg
                v-if="isTesting"
                class="animate-spin mr-2"
                width="16"
                height="16"
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
              {{ isTesting ? t('testing') : t('testConnection') }}
            </Button>
            <Button type="submit" :disabled="!isFormValid">{{ t('save') }}</Button>
            <Button type="button" variant="secondary" @click="showAddModal = false">
              {{ t('cancel') }}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useI18n } from '../composables/useI18n'
import { useDialog } from '../composables/useDialog'
import { testConnection, warmupProfileCache } from '../services/tauri'
import { logger } from '../utils/logger'
import type { Profile, TestConnectionResponse } from '../types'
import { v4 as uuidv4 } from 'uuid'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { validateEndpoint, validateRegion } from '../utils/validators'
import ConnectionListModal from './ConnectionListModal.vue'

const appStore = useAppStore()
const { t } = useI18n()
const dialog = useDialog()
const showAddModal = ref(false)
const showConnectionListModal = ref(false)
const editingProfile = ref<Profile | null>(null)
const testResult = ref<TestConnectionResponse | null>(null)
const isTesting = ref(false)

// Search query for filtering profiles
const profileSearchQuery = ref('')

// Filtered profiles based on search query (excluding disabled profiles)
const filteredProfiles = computed(() => {
  // First filter out disabled profiles
  const enabledProfiles = appStore.profiles.filter(p => p.enabled !== false)

  if (!profileSearchQuery.value.trim()) {
    return enabledProfiles
  }

  const query = profileSearchQuery.value.toLowerCase()
  return enabledProfiles.filter(profile =>
    profile.name.toLowerCase().includes(query)
  )
})

const formData = reactive({
  name: '',
  endpoint: '',
  region: 'us-east-1',
  access_key: '',
  secret_key: '',
  session_token: '',
  path_style: false,
})

// Validation errors and warnings
const validationErrors = reactive({
  endpoint: '',
  region: '',
})

const validationWarnings = reactive({
  endpoint: '',
  region: '',
})

// Validate endpoint
watch(
  () => formData.endpoint,
  (value) => {
    const result = validateEndpoint(value)
    validationErrors.endpoint = result.error || ''
    validationWarnings.endpoint = result.warning || ''
  }
)

// Validate region
watch(
  () => formData.region,
  (value) => {
    const result = validateRegion(value)
    validationErrors.region = result.error || ''
    validationWarnings.region = result.warning || ''
  }
)

// Reset form when modal closes (cleanup)
watch(showAddModal, (isOpen) => {
  if (!isOpen) {
    // Modal closed, reset form for next use
    resetForm()
  }
})

// Check if form is valid (warnings don't block submission)
const isFormValid = computed(() => {
  return (
    formData.name.trim() !== '' &&
    formData.access_key.trim() !== '' &&
    formData.secret_key.trim() !== '' &&
    validationErrors.endpoint === '' &&
    validationErrors.region === ''
  )
})

function resetForm() {
  formData.name = ''
  formData.endpoint = ''
  formData.region = ''
  formData.access_key = ''
  formData.secret_key = ''
  formData.session_token = ''
  formData.path_style = false
  validationErrors.endpoint = ''
  validationErrors.region = ''
  validationWarnings.endpoint = ''
  validationWarnings.region = ''
  testResult.value = null
  editingProfile.value = null
}

function openAddModal() {
  resetForm()
  showAddModal.value = true
}

async function saveProfile() {
  // Validate before saving
  if (!isFormValid.value) {
    return
  }

  const profile: Profile = {
    id: editingProfile.value?.id || uuidv4(),
    name: formData.name,
    endpoint: formData.endpoint || undefined,
    region: formData.region || undefined,
    access_key: formData.access_key,
    secret_key: formData.secret_key,
    session_token: formData.session_token || undefined,
    path_style: formData.path_style,
  }

  try {
    await appStore.saveProfileData(profile)
    showAddModal.value = false
    resetForm()
  } catch (e) {
    alert(`Failed to save profile: ${e}`)
  }
}

async function testConnectionHandler() {
  const profile: Profile = {
    id: 'test',
    name: formData.name,
    endpoint: formData.endpoint || undefined,
    region: formData.region || undefined,
    access_key: formData.access_key,
    secret_key: formData.secret_key,
    session_token: formData.session_token || undefined,
    path_style: formData.path_style,
  }

  try {
    isTesting.value = true
    testResult.value = null // Clear previous result
    testResult.value = await testConnection(profile)
  } catch (e) {
    testResult.value = {
      success: false,
      message: String(e),
    }
  } finally {
    isTesting.value = false
  }
}

async function selectProfile(profile: Profile) {
  appStore.selectProfile(profile)
  await appStore.loadBuckets()
}

function editProfileHandler(profile: Profile) {
  editingProfile.value = profile
  formData.name = profile.name
  formData.endpoint = profile.endpoint || ''
  formData.region = profile.region || ''
  formData.access_key = profile.access_key
  formData.secret_key = profile.secret_key
  formData.session_token = profile.session_token || ''
  formData.path_style = profile.path_style
  testResult.value = null
  showAddModal.value = true
}

async function deleteProfileConfirm(profile: Profile) {
  const confirmed = await dialog.confirm({
    title: t('deleteConnection'),
    message: `${t('deleteConfirm')} "${profile.name}"?`,
    confirmText: t('delete'),
    cancelText: t('cancel'),
    variant: 'destructive',
  })

  if (confirmed) {
    try {
      await appStore.removeProfile(profile.id)
    } catch (e) {
      await dialog.confirm({
        title: t('errorOccurred'),
        message: `Failed to delete profile: ${e}`,
        confirmText: t('close'),
        variant: 'destructive',
      })
    }
  }
}

function enablePathStyle() {
  formData.path_style = true
  testResult.value = null
}

// Cache warmup on profile hover - debounced to avoid spam
let warmupTimeout: ReturnType<typeof setTimeout> | null = null
const warmedUpProfiles = new Set<string>()

function handleProfileHover(profile: Profile) {
  // Skip if already current profile or already warmed up in this session
  if (appStore.currentProfile?.id === profile.id || warmedUpProfiles.has(profile.id)) {
    return
  }

  // Clear previous timeout
  if (warmupTimeout) {
    clearTimeout(warmupTimeout)
  }

  // Debounce: only warmup if user hovers for 150ms
  warmupTimeout = setTimeout(async () => {
    try {
      await warmupProfileCache(profile.id)
      warmedUpProfiles.add(profile.id)
      logger.debug(`[CacheWarmup] Preloaded cache for profile: ${profile.name}`)
    } catch (e) {
      // Non-critical - silent fail
      logger.warn(`[CacheWarmup] Failed to warmup profile ${profile.name}`, e)
    }
  }, 150)
}
</script>
