<template>
  <Dialog v-model:open="isOpen">
    <DialogContent class="max-w-3xl max-h-[85vh] overflow-hidden flex flex-col">
      <DialogHeader>
        <DialogTitle>{{ bucketName }} - {{ t('bucketSettings') }}</DialogTitle>
      </DialogHeader>

      <!-- Tabs -->
      <div class="flex gap-1 border-b pb-2 overflow-x-auto">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          @click="activeTab = tab.id"
          :class="[
            'px-3 py-1.5 text-sm font-medium rounded-t-md transition-colors whitespace-nowrap',
            activeTab === tab.id
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:text-foreground hover:bg-muted',
          ]"
        >
          {{ tab.label }}
        </button>
      </div>

      <!-- Loading state -->
      <div v-if="loading" class="flex justify-center py-12">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>

      <!-- Tab Content -->
      <div v-else class="flex-1 overflow-y-auto py-4 px-1">
        <!-- POLICY TAB -->
        <div v-if="activeTab === 'policy'" class="space-y-4">
          <div v-if="config?.policy.error" class="p-4 bg-destructive/10 border border-destructive/30 rounded-lg">
            <p class="text-sm text-destructive">{{ t('accessDenied') }}: {{ config.policy.error }}</p>
          </div>
          <div v-else-if="!config?.policy.policy" class="p-4 bg-muted rounded-lg">
            <p class="text-sm text-muted-foreground">{{ t('bucketPolicyNone') }}</p>
          </div>
          <div v-else class="space-y-2">
            <p class="text-sm font-medium">{{ t('bucketPolicy') }}</p>
            <pre class="p-4 bg-muted rounded-lg text-xs overflow-x-auto font-mono">{{ formatJson(config.policy.policy) }}</pre>
          </div>
        </div>

        <!-- ACL TAB -->
        <div v-if="activeTab === 'acl'" class="space-y-4">
          <div class="p-4 bg-muted rounded-lg">
            <div class="flex items-center gap-3">
              <div
                :class="[
                  'w-10 h-10 rounded-full flex items-center justify-center',
                  config?.acl === 'Public' ? 'bg-yellow-500/20' : 'bg-green-500/20',
                ]"
              >
                <svg
                  v-if="config?.acl === 'Public'"
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  class="text-yellow-500"
                >
                  <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
                  <path d="M7 11V7a5 5 0 0 1 9.9-1" />
                </svg>
                <svg
                  v-else
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  class="text-green-500"
                >
                  <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
                  <path d="M7 11V7a5 5 0 0 1 10 0v4" />
                </svg>
              </div>
              <div>
                <p class="font-medium">{{ config?.acl === 'Public' ? t('bucketPublic') : t('bucketPrivate') }}</p>
                <p class="text-sm text-muted-foreground">
                  {{ config?.acl === 'Public' ? t('bucketPublicDesc') : t('bucketPrivateDesc') }}
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- CORS TAB -->
        <div v-if="activeTab === 'cors'" class="space-y-4">
          <div v-if="config?.cors.error" class="p-4 bg-destructive/10 border border-destructive/30 rounded-lg">
            <p class="text-sm text-destructive">{{ t('accessDenied') }}: {{ config.cors.error }}</p>
          </div>
          <div v-else-if="!config?.cors.rules.length" class="p-4 bg-muted rounded-lg">
            <p class="text-sm text-muted-foreground">{{ t('bucketCorsNone') }}</p>
          </div>
          <div v-else class="space-y-4">
            <div
              v-for="(rule, index) in config.cors.rules"
              :key="index"
              class="p-4 border rounded-lg space-y-3"
            >
              <p class="text-sm font-medium">{{ t('corsRule') }} #{{ index + 1 }}</p>
              <div class="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <p class="text-muted-foreground">{{ t('allowedOrigins') }}</p>
                  <p class="font-mono text-xs">{{ rule.allowed_origins.join(', ') || '-' }}</p>
                </div>
                <div>
                  <p class="text-muted-foreground">{{ t('allowedMethods') }}</p>
                  <p class="font-mono text-xs">{{ rule.allowed_methods.join(', ') || '-' }}</p>
                </div>
                <div>
                  <p class="text-muted-foreground">{{ t('allowedHeaders') }}</p>
                  <p class="font-mono text-xs">{{ rule.allowed_headers.join(', ') || '-' }}</p>
                </div>
                <div>
                  <p class="text-muted-foreground">{{ t('maxAgeSeconds') }}</p>
                  <p class="font-mono text-xs">{{ rule.max_age_seconds ?? '-' }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- LIFECYCLE TAB -->
        <div v-if="activeTab === 'lifecycle'" class="space-y-4">
          <div v-if="config?.lifecycle.error" class="p-4 bg-destructive/10 border border-destructive/30 rounded-lg">
            <p class="text-sm text-destructive">{{ t('accessDenied') }}: {{ config.lifecycle.error }}</p>
          </div>
          <div v-else-if="!config?.lifecycle.rules.length" class="p-4 bg-muted rounded-lg">
            <p class="text-sm text-muted-foreground">{{ t('bucketLifecycleNone') }}</p>
          </div>
          <div v-else class="space-y-4">
            <div
              v-for="(rule, index) in config.lifecycle.rules"
              :key="index"
              class="p-4 border rounded-lg space-y-3"
            >
              <div class="flex items-center justify-between">
                <p class="text-sm font-medium">{{ rule.id || `Rule #${index + 1}` }}</p>
                <span
                  :class="[
                    'px-2 py-0.5 text-xs rounded-full',
                    rule.status === 'Enabled' ? 'bg-green-500/20 text-green-500' : 'bg-muted text-muted-foreground',
                  ]"
                >
                  {{ rule.status }}
                </span>
              </div>
              <div class="grid grid-cols-2 gap-4 text-sm">
                <div v-if="rule.filter_prefix">
                  <p class="text-muted-foreground">{{ t('filterPrefix') }}</p>
                  <p class="font-mono text-xs">{{ rule.filter_prefix }}</p>
                </div>
                <div v-if="rule.expiration_days">
                  <p class="text-muted-foreground">{{ t('expirationDays') }}</p>
                  <p>{{ rule.expiration_days }} {{ t('days') }}</p>
                </div>
                <div v-if="rule.noncurrent_version_expiration_days">
                  <p class="text-muted-foreground">{{ t('noncurrentExpiration') }}</p>
                  <p>{{ rule.noncurrent_version_expiration_days }} {{ t('days') }}</p>
                </div>
                <div v-if="rule.abort_incomplete_multipart_days">
                  <p class="text-muted-foreground">{{ t('abortIncompleteMultipart') }}</p>
                  <p>{{ rule.abort_incomplete_multipart_days }} {{ t('days') }}</p>
                </div>
              </div>
              <div v-if="rule.transitions.length" class="pt-2 border-t">
                <p class="text-sm text-muted-foreground mb-2">{{ t('transitions') }}</p>
                <div class="space-y-1">
                  <div v-for="(transition, tIndex) in rule.transitions" :key="tIndex" class="text-sm">
                    <span v-if="transition.days">{{ transition.days }} {{ t('days') }} &rarr; </span>
                    <span class="font-mono text-xs">{{ transition.storage_class }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- VERSIONING TAB -->
        <div v-if="activeTab === 'versioning'" class="space-y-4">
          <div v-if="config?.versioning.error" class="p-4 bg-destructive/10 border border-destructive/30 rounded-lg">
            <p class="text-sm text-destructive">{{ t('accessDenied') }}: {{ config.versioning.error }}</p>
          </div>
          <div v-else class="p-4 bg-muted rounded-lg space-y-4">
            <div class="flex items-center gap-3">
              <div
                :class="[
                  'w-10 h-10 rounded-full flex items-center justify-center',
                  config?.versioning.status === 'Enabled'
                    ? 'bg-green-500/20'
                    : config?.versioning.status === 'Suspended'
                      ? 'bg-yellow-500/20'
                      : 'bg-muted-foreground/20',
                ]"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  :class="[
                    config?.versioning.status === 'Enabled'
                      ? 'text-green-500'
                      : config?.versioning.status === 'Suspended'
                        ? 'text-yellow-500'
                        : 'text-muted-foreground',
                  ]"
                >
                  <path d="M12 3v18" />
                  <path d="M8 6 12 3l4 3" />
                  <path d="M8 18 12 21l4-3" />
                  <path d="M3 12h18" />
                </svg>
              </div>
              <div>
                <p class="font-medium">
                  {{
                    config?.versioning.status === 'Enabled'
                      ? t('versioningEnabled')
                      : config?.versioning.status === 'Suspended'
                        ? t('versioningSuspended')
                        : t('versioningNeverEnabled')
                  }}
                </p>
                <p class="text-sm text-muted-foreground">{{ t('bucketVersioning') }}</p>
              </div>
            </div>
            <div v-if="config?.versioning.mfa_delete" class="pt-4 border-t">
              <p class="text-sm">
                <span class="text-muted-foreground">{{ t('mfaDelete') }}:</span>
                <span class="ml-2">{{ config.versioning.mfa_delete }}</span>
              </p>
            </div>
          </div>
        </div>

        <!-- ENCRYPTION TAB -->
        <div v-if="activeTab === 'encryption'" class="space-y-4">
          <div v-if="config?.encryption.error" class="p-4 bg-destructive/10 border border-destructive/30 rounded-lg">
            <p class="text-sm text-destructive">{{ t('accessDenied') }}: {{ config.encryption.error }}</p>
          </div>
          <div v-else-if="!config?.encryption.rules.length" class="p-4 bg-muted rounded-lg">
            <p class="text-sm text-muted-foreground">{{ t('bucketEncryptionNone') }}</p>
          </div>
          <div v-else class="space-y-4">
            <div
              v-for="(rule, index) in config.encryption.rules"
              :key="index"
              class="p-4 bg-muted rounded-lg space-y-3"
            >
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-full bg-blue-500/20 flex items-center justify-center">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    class="text-blue-500"
                  >
                    <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
                    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
                  </svg>
                </div>
                <div>
                  <p class="font-medium">
                    {{ rule.sse_algorithm === 'AES256' ? t('encryptionAes256') : t('encryptionKms') }}
                  </p>
                  <p class="text-sm text-muted-foreground">{{ t('bucketEncryption') }}</p>
                </div>
              </div>
              <div v-if="rule.kms_master_key_id" class="pt-3 border-t">
                <p class="text-sm">
                  <span class="text-muted-foreground">KMS Key ID:</span>
                  <span class="ml-2 font-mono text-xs">{{ rule.kms_master_key_id }}</span>
                </p>
              </div>
              <div v-if="rule.bucket_key_enabled !== null" class="pt-3 border-t">
                <p class="text-sm">
                  <span class="text-muted-foreground">{{ t('bucketKeyEnabled') }}:</span>
                  <span class="ml-2">{{ rule.bucket_key_enabled ? 'Yes' : 'No' }}</span>
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="isOpen = false">{{ t('close') }}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useI18n } from '../composables/useI18n'
import { getBucketConfiguration } from '../services/tauri'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import type { BucketConfigurationResponse } from '../types'

const props = defineProps<{
  modelValue: boolean
  bucketName: string
  profileId: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const { t } = useI18n()

const isOpen = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
})

const loading = ref(false)
const config = ref<BucketConfigurationResponse | null>(null)
const activeTab = ref('policy')

const tabs = computed(() => [
  { id: 'policy', label: t('bucketPolicy') },
  { id: 'acl', label: t('bucketAcl') },
  { id: 'cors', label: t('bucketCors') },
  { id: 'lifecycle', label: t('bucketLifecycle') },
  { id: 'versioning', label: t('bucketVersioning') },
  { id: 'encryption', label: t('bucketEncryption') },
])

function formatJson(jsonStr: string): string {
  try {
    return JSON.stringify(JSON.parse(jsonStr), null, 2)
  } catch {
    return jsonStr
  }
}

async function loadConfiguration() {
  if (!props.profileId || !props.bucketName) return

  loading.value = true
  try {
    config.value = await getBucketConfiguration(props.profileId, props.bucketName)
  } catch (e) {
    console.error('Failed to load bucket configuration:', e)
  } finally {
    loading.value = false
  }
}

// Load configuration when modal opens
watch(
  () => props.modelValue,
  (isVisible) => {
    if (isVisible) {
      activeTab.value = 'policy'
      loadConfiguration()
    }
  }
)
</script>
