<template>
  <div class="p-4 border-b border-white/10">
    <div class="flex justify-between items-center mb-4">
      <h3 class="text-lg font-semibold">{{ t('connections') }}</h3>
      <Button size="sm" @click="showAddModal = true">
        <span class="mr-1">+</span> {{ t('add') }}
      </Button>
    </div>

    <div class="flex flex-col gap-2">
      <div
        v-for="profile in appStore.profiles"
        :key="profile.id"
        class="flex justify-between items-center p-3 rounded-md cursor-pointer transition-colors group"
        :class="
          appStore.currentProfile?.id === profile.id
            ? 'bg-white/20'
            : 'bg-white/5 hover:bg-white/10'
        "
        @click="selectProfile(profile)"
      >
        <div class="flex-1">
          <div class="font-medium">{{ profile.name }}</div>
          <Badge variant="secondary" class="mt-1 text-xs">{{ profile.region }}</Badge>
        </div>
        <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button
            size="icon"
            variant="ghost"
            class="h-6 w-6 text-white/70 hover:text-white hover:bg-white/10"
            @click.stop="editProfileHandler(profile)"
            :title="t('edit')"
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
            :title="t('delete')"
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
            <Input v-model="formData.endpoint" :placeholder="t('endpointPlaceholder')" />
            <p class="text-xs text-muted-foreground">{{ t('endpointDescription') }}</p>
          </div>

          <div class="space-y-2">
            <label class="text-sm font-medium">{{ t('region') }} *</label>
            <Input v-model="formData.region" required placeholder="us-east-1" />
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

          <div class="flex items-center space-x-2">
            <input
              v-model="formData.use_tls"
              type="checkbox"
              id="use-tls"
              class="rounded border-gray-300"
            />
            <label for="use-tls" class="text-sm font-medium">{{ t('useTls') }}</label>
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
            <Button type="button" variant="outline" @click="testConnectionHandler">
              {{ t('testConnection') }}
            </Button>
            <Button type="submit">{{ t('save') }}</Button>
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
import { ref, reactive } from 'vue'
import { useAppStore } from '../stores/app'
import { useI18n } from '../composables/useI18n'
import { useDialog } from '../composables/useDialog'
import { testConnection } from '../services/tauri'
import type { Profile, TestConnectionResponse } from '../types'
import { v4 as uuidv4 } from 'uuid'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

const appStore = useAppStore()
const { t } = useI18n()
const dialog = useDialog()
const showAddModal = ref(false)
const editingProfile = ref<Profile | null>(null)
const testResult = ref<TestConnectionResponse | null>(null)

const formData = reactive({
  name: '',
  endpoint: '',
  region: 'us-east-1',
  access_key: '',
  secret_key: '',
  session_token: '',
  path_style: false,
  use_tls: true,
})

function resetForm() {
  formData.name = ''
  formData.endpoint = ''
  formData.region = 'us-east-1'
  formData.access_key = ''
  formData.secret_key = ''
  formData.session_token = ''
  formData.path_style = false
  formData.use_tls = true
  testResult.value = null
  editingProfile.value = null
}

async function saveProfile() {
  const profile: Profile = {
    id: editingProfile.value?.id || uuidv4(),
    name: formData.name,
    endpoint: formData.endpoint || undefined,
    region: formData.region,
    access_key: formData.access_key,
    secret_key: formData.secret_key,
    session_token: formData.session_token || undefined,
    path_style: formData.path_style,
    use_tls: formData.use_tls,
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
    region: formData.region,
    access_key: formData.access_key,
    secret_key: formData.secret_key,
    session_token: formData.session_token || undefined,
    path_style: formData.path_style,
    use_tls: formData.use_tls,
  }

  try {
    testResult.value = await testConnection(profile)
  } catch (e) {
    testResult.value = {
      success: false,
      message: String(e),
    }
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
  formData.region = profile.region
  formData.access_key = profile.access_key
  formData.secret_key = profile.secret_key
  formData.session_token = profile.session_token || ''
  formData.path_style = profile.path_style
  formData.use_tls = profile.use_tls
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
</script>
