<template>
  <div class="flex h-screen overflow-hidden">
    <div class="w-80 bg-slate-900 text-white flex flex-col flex-shrink-0">
      <div class="flex-1 overflow-y-auto">
        <ProfileManager />
        <BucketList v-if="appStore.hasProfile" />
      </div>
      <SettingsButton />
    </div>
    <div class="flex-1 overflow-hidden bg-background">
      <ObjectBrowser v-if="appStore.hasBucket" />
      <div v-else class="flex flex-col items-center justify-center h-full p-8 text-center">
        <Card class="max-w-md">
          <CardHeader>
            <CardTitle>{{ t('welcomeTitle') }}</CardTitle>
            <CardDescription v-if="!appStore.hasProfile">
              {{ t('welcomeMessageNoProfile') }}
            </CardDescription>
            <CardDescription v-else>
              {{ t('welcomeMessageNoBucket') }}
            </CardDescription>
          </CardHeader>
        </Card>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '../stores/app'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from '../composables/useI18n'
import ProfileManager from '../components/ProfileManager.vue'
import BucketList from '../components/BucketList.vue'
import ObjectBrowser from '../components/ObjectBrowser.vue'
import SettingsButton from '../components/SettingsButton.vue'
import { Card, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'

const appStore = useAppStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

onMounted(async () => {
  settingsStore.loadSettings()
  await appStore.loadProfiles()
})
</script>
