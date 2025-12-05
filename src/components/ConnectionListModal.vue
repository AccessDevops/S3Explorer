<template>
  <Dialog :open="open" @update:open="$emit('update:open', $event)">
    <DialogContent class="max-w-4xl max-h-[85vh] overflow-hidden flex flex-col">
      <DialogHeader>
        <DialogTitle>{{ t('connectionList') }}</DialogTitle>
        <DialogDescription>{{ t('connectionListDescription') }}</DialogDescription>
      </DialogHeader>

      <!-- Toolbar -->
      <div class="flex justify-end pb-4">
        <Button size="sm" @click="$emit('create')">
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
            <path d="M5 12h14" />
            <path d="M12 5v14" />
          </svg>
          {{ t('newConnection') }}
        </Button>
      </div>

      <!-- Table -->
      <div class="flex-1 overflow-y-auto">
        <Table v-if="profiles.length > 0">
          <TableHeader>
            <TableRow>
              <TableHead class="w-[200px]">{{ t('profileName') }}</TableHead>
              <TableHead>{{ t('endpoint') }}</TableHead>
              <TableHead class="w-[120px]">{{ t('region') }}</TableHead>
              <TableHead class="w-[150px]">{{ t('accessKey') }}</TableHead>
              <TableHead class="w-[140px] text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow
              v-for="profile in profiles"
              :key="profile.id"
              :class="{ 'opacity-50': profile.enabled === false }"
            >
              <TableCell class="font-medium">
                <div class="flex items-center gap-2">
                  {{ profile.name }}
                  <span
                    v-if="profile.enabled === false"
                    class="inline-flex items-center rounded-full bg-amber-500/20 px-2 py-0.5 text-xs font-medium text-amber-500"
                  >
                    {{ t('connectionDisabled') }}
                  </span>
                </div>
              </TableCell>
              <TableCell class="text-muted-foreground">
                {{ profile.endpoint || t('endpointDefault') }}
              </TableCell>
              <TableCell class="text-muted-foreground">
                {{ profile.region || 'us-east-1' }}
              </TableCell>
              <TableCell class="font-mono text-muted-foreground">
                {{ maskAccessKey(profile.access_key) }}
              </TableCell>
              <TableCell class="text-right">
                <div class="flex justify-end gap-1">
                  <!-- Edit button -->
                  <Button
                    size="icon"
                    variant="ghost"
                    class="h-8 w-8"
                    @click="$emit('edit', profile)"
                    v-tooltip="t('edit')"
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
                      <path
                        d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"
                      />
                      <path d="m15 5 4 4" />
                    </svg>
                  </Button>

                  <!-- Toggle visibility button -->
                  <Button
                    size="icon"
                    variant="ghost"
                    class="h-8 w-8"
                    @click="toggleEnabled(profile)"
                    v-tooltip="profile.enabled === false ? t('enableConnection') : t('disableConnection')"
                  >
                    <!-- Eye icon (visible) -->
                    <svg
                      v-if="profile.enabled !== false"
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
                      <path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0" />
                      <circle cx="12" cy="12" r="3" />
                    </svg>
                    <!-- Eye-off icon (hidden) -->
                    <svg
                      v-else
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
                      <path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49" />
                      <path d="M14.084 14.158a3 3 0 0 1-4.242-4.242" />
                      <path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143" />
                      <path d="m2 2 20 20" />
                    </svg>
                  </Button>

                  <!-- Delete button -->
                  <Button
                    size="icon"
                    variant="ghost"
                    class="h-8 w-8 text-destructive hover:text-destructive"
                    @click="confirmDelete(profile)"
                    v-tooltip="t('delete')"
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
                      <path d="M3 6h18" />
                      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                      <line x1="10" x2="10" y1="11" y2="17" />
                      <line x1="14" x2="14" y1="11" y2="17" />
                    </svg>
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>

        <!-- Empty state -->
        <div v-else class="flex flex-col items-center justify-center py-12 text-center">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-muted-foreground/50 mb-4"
          >
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10" />
            <path d="m14.5 9-5 5" />
            <path d="m9.5 9 5 5" />
          </svg>
          <p class="text-muted-foreground">{{ t('noConnections') }}</p>
          <Button size="sm" class="mt-4" @click="$emit('create')">
            {{ t('newConnection') }}
          </Button>
        </div>
      </div>

      <DialogFooter class="pt-4">
        <Button variant="secondary" @click="$emit('update:open', false)">
          {{ t('close') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '../stores/app'
import { useI18n } from '../composables/useI18n'
import { useDialog } from '../composables/useDialog'
import type { Profile } from '../types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'

defineProps<{
  open: boolean
}>()

defineEmits<{
  'update:open': [value: boolean]
  'edit': [profile: Profile]
  'create': []
}>()

const appStore = useAppStore()
const { t } = useI18n()
const dialog = useDialog()

// All profiles (including disabled ones)
const profiles = computed(() => appStore.profiles)

// Mask access key for display
function maskAccessKey(key: string): string {
  if (key.length <= 8) {
    return '••••••••'
  }
  return `${key.slice(0, 4)}...${key.slice(-4)}`
}

// Toggle profile enabled state
async function toggleEnabled(profile: Profile) {
  const updatedProfile: Profile = {
    ...profile,
    enabled: profile.enabled === false ? true : false,
  }
  await appStore.saveProfileData(updatedProfile)
}

// Confirm and delete profile
async function confirmDelete(profile: Profile) {
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
</script>
