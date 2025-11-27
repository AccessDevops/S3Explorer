<template>
  <div>
    <Tooltip :text="t('metrics')" side="top">
      <Button
        variant="ghost"
        class="w-full justify-center text-white/70 hover:text-white hover:bg-white/10"
        @click="showMetricsModal = true"
      >
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
        >
          <path d="M3 3v18h18" />
          <path d="m19 9-5 5-4-4-3 3" />
        </svg>
      </Button>
    </Tooltip>

    <!-- Metrics Dashboard Modal -->
    <Dialog v-model:open="showMetricsModal">
      <DialogContent class="max-w-5xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle class="flex items-center gap-2">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M3 3v18h18" />
              <path d="m19 9-5 5-4-4-3 3" />
            </svg>
            {{ t('metricsTitle') }}
          </DialogTitle>
        </DialogHeader>

        <!-- Period selector and Provider -->
        <div class="flex items-center gap-4 pb-2 border-b flex-wrap">
          <div class="flex items-center gap-2">
            <span class="text-sm text-muted-foreground">{{ t('metricsPeriod') }}:</span>
            <div class="flex gap-1">
              <Button
                v-for="period in periods"
                :key="period.days"
                size="sm"
                :variant="selectedPeriod === period.days ? 'default' : 'outline'"
                @click="selectedPeriod = period.days"
              >
                {{ period.label }}
              </Button>
            </div>
          </div>

          <!-- Provider selector dropdown -->
          <div class="flex items-center gap-2 relative">
            <span class="text-sm text-muted-foreground">{{ t('metricsProvider') }}:</span>
            <div class="relative">
              <Button
                size="sm"
                variant="outline"
                @click="showProviderDropdown = !showProviderDropdown"
                class="min-w-[120px] justify-between"
              >
                {{ currentProviderName }}
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
                  class="ml-2"
                  :class="{ 'rotate-180': showProviderDropdown }"
                >
                  <path d="m6 9 6 6 6-6" />
                </svg>
              </Button>
              <Transition
                enter-active-class="transition duration-100 ease-out"
                enter-from-class="opacity-0 scale-95"
                enter-to-class="opacity-100 scale-100"
                leave-active-class="transition duration-75 ease-in"
                leave-from-class="opacity-100 scale-100"
                leave-to-class="opacity-0 scale-95"
              >
                <div
                  v-if="showProviderDropdown"
                  class="absolute top-full left-0 mt-1 z-50 min-w-[180px] bg-popover border rounded-md shadow-md py-1"
                >
                  <button
                    v-for="provider in PROVIDER_INFO"
                    :key="provider.id"
                    @click="selectProvider(provider.id)"
                    class="w-full px-3 py-2 text-sm text-left hover:bg-accent hover:text-accent-foreground flex items-center justify-between"
                    :class="{ 'bg-accent/50': selectedProvider === provider.id }"
                  >
                    <span>{{ provider.name }}</span>
                    <svg
                      v-if="selectedProvider === provider.id"
                      xmlns="http://www.w3.org/2000/svg"
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      class="text-primary"
                    >
                      <path d="M20 6 9 17l-5-5" />
                    </svg>
                  </button>
                </div>
              </Transition>
            </div>
          </div>

          <!-- Custom pricing inputs (shown when custom provider is selected) -->
          <div v-if="selectedProvider === 'custom'" class="flex items-center gap-2 text-xs">
            <div class="flex items-center gap-1">
              <span class="text-xs text-muted-foreground">GET:</span>
              <Input
                v-model.number="customPricingGet"
                type="number"
                step="0.0001"
                min="0"
                class="w-20 h-6 text-xs"
              />
            </div>
            <div class="flex items-center gap-1">
              <span class="text-xs text-muted-foreground">PUT:</span>
              <Input
                v-model.number="customPricingPut"
                type="number"
                step="0.0001"
                min="0"
                class="w-20 h-6 text-xs"
              />
            </div>
            <div class="flex items-center gap-1">
              <span class="text-xs text-muted-foreground">LIST:</span>
              <Input
                v-model.number="customPricingList"
                type="number"
                step="0.0001"
                min="0"
                class="w-20 h-6 text-xs"
              />
            </div>
          </div>

          <div class="ml-auto flex gap-2">
            <Button size="sm" variant="outline" @click="refreshData">
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
                <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                <path d="M3 3v5h5" />
                <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                <path d="M16 16h5v5" />
              </svg>
              {{ t('refresh') }}
            </Button>
          </div>
        </div>

        <!-- Main content with scroll -->
        <div class="flex-1 overflow-y-auto py-4 space-y-6">
          <!-- KPI Cards -->
          <div class="grid grid-cols-5 gap-4">
            <!-- Today's Requests -->
            <div class="bg-muted/50 rounded-lg p-4">
              <div class="text-2xl font-bold">{{ totalRequestsToday.toLocaleString() }}</div>
              <div class="text-sm text-muted-foreground">{{ t('metricsToday') }}</div>
            </div>

            <!-- Cost Today -->
            <div class="bg-muted/50 rounded-lg p-4">
              <div class="text-2xl font-bold">${{ estimatedCostToday.toFixed(4) }}</div>
              <div class="text-sm text-muted-foreground">{{ t('metricsCostToday') }}</div>
            </div>

            <!-- Cost Month Projection -->
            <div class="bg-muted/50 rounded-lg p-4">
              <div class="text-2xl font-bold">${{ estimatedCostMonth.toFixed(2) }}</div>
              <div class="text-sm text-muted-foreground">{{ t('metricsCostMonth') }}</div>
            </div>

            <!-- Requests per Hour -->
            <div class="bg-muted/50 rounded-lg p-4">
              <div class="text-2xl font-bold">{{ requestsPerHour }}/h</div>
              <div class="text-sm text-muted-foreground">{{ t('metricsPerHour') }}</div>
            </div>

            <!-- Error Rate -->
            <div class="bg-muted/50 rounded-lg p-4">
              <div
                class="text-2xl font-bold"
                :class="{
                  'text-green-500': errorRateToday < 1,
                  'text-yellow-500': errorRateToday >= 1 && errorRateToday < 5,
                  'text-red-500': errorRateToday >= 5,
                }"
              >
                {{ errorRateToday.toFixed(1) }}%
              </div>
              <div class="text-sm text-muted-foreground">{{ t('metricsErrorRate') }}</div>
            </div>
          </div>

          <!-- Charts Row -->
          <div class="grid grid-cols-3 gap-4">
            <!-- Category Distribution -->
            <div class="border rounded-lg p-4">
              <h3 class="font-medium mb-4">{{ t('metricsByCategory') }}</h3>
              <div class="space-y-3">
                <div v-for="cat in categoryData" :key="cat.name" class="space-y-1">
                  <div class="flex justify-between text-sm">
                    <span>{{ cat.name }}</span>
                    <span class="text-muted-foreground"
                      >{{ cat.count.toLocaleString() }} ({{ cat.percentage.toFixed(1) }}%)</span
                    >
                  </div>
                  <div class="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      class="h-full rounded-full transition-all"
                      :class="cat.color"
                      :style="{ width: `${cat.percentage}%` }"
                    ></div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Cache Hit Rate Gauge -->
            <div class="border rounded-lg p-4">
              <h3 class="font-medium mb-4">{{ t('metricsCacheEfficiency') }}</h3>
              <div class="flex flex-col items-center justify-center h-32">
                <!-- Circular gauge -->
                <div class="relative w-24 h-24">
                  <svg class="w-24 h-24 transform -rotate-90" viewBox="0 0 36 36">
                    <!-- Background circle -->
                    <path
                      class="text-muted"
                      stroke="currentColor"
                      stroke-width="3"
                      fill="none"
                      d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                    />
                    <!-- Progress circle -->
                    <path
                      :class="{
                        'text-green-500': cacheHitRate >= 70,
                        'text-yellow-500': cacheHitRate >= 40 && cacheHitRate < 70,
                        'text-red-500': cacheHitRate < 40,
                      }"
                      stroke="currentColor"
                      stroke-width="3"
                      stroke-linecap="round"
                      fill="none"
                      :stroke-dasharray="`${cacheHitRate}, 100`"
                      d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                    />
                  </svg>
                  <div class="absolute inset-0 flex items-center justify-center">
                    <span class="text-xl font-bold">{{ cacheHitRate.toFixed(0) }}%</span>
                  </div>
                </div>
                <div class="mt-2 text-center text-sm text-muted-foreground">
                  <div>{{ t('metricsCacheHits') }}: {{ cacheRequestsSaved.toLocaleString() }}</div>
                  <div class="text-green-600">${{ cacheCostSaved.toFixed(4) }} {{ t('metricsSaved') }}</div>
                </div>
              </div>
            </div>

            <!-- Hourly Distribution -->
            <div class="border rounded-lg p-4">
              <h3 class="font-medium mb-4">{{ t('metricsHourly') }}</h3>
              <div class="flex items-end h-28 gap-0.5">
                <Tooltip
                  v-for="hour in hourlyData"
                  :key="hour.hour"
                  :text="`${hour.hour}:00 - ${hour.count} ${t('metricsRequests')}`"
                  side="top"
                >
                  <div class="flex-1 h-full flex flex-col justify-end items-center">
                    <div
                      class="w-full bg-primary/80 rounded-t transition-all min-h-[2px]"
                      :style="{ height: `${Math.max(2, (hour.count / maxHourlyCount) * 100)}%` }"
                    ></div>
                  </div>
                </Tooltip>
              </div>
              <div class="flex justify-between mt-1 text-[10px] text-muted-foreground">
                <span>0h</span>
                <span>6h</span>
                <span>12h</span>
                <span>18h</span>
                <span>24h</span>
              </div>
            </div>
          </div>

          <!-- Operations Table -->
          <div class="border rounded-lg overflow-hidden">
            <table class="w-full text-sm">
              <thead class="bg-muted/50">
                <tr>
                  <th class="text-left py-2 px-3 font-medium">{{ t('metricsOperation') }}</th>
                  <th class="text-right py-2 px-3 font-medium">{{ t('metricsCount') }}</th>
                  <th class="text-right py-2 px-3 font-medium">{{ t('metricsSuccess') }}</th>
                  <th class="text-right py-2 px-3 font-medium">{{ t('metricsFailed') }}</th>
                  <th class="text-right py-2 px-3 font-medium">{{ t('metricsAvgTime') }}</th>
                  <th class="text-right py-2 px-3 font-medium">{{ t('metricsCost') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="op in operationData"
                  :key="op.operation"
                  class="border-t hover:bg-muted/30 transition-colors"
                >
                  <td class="py-2 px-3 font-mono text-xs">{{ op.operation }}</td>
                  <td class="text-right py-2 px-3">{{ op.count.toLocaleString() }}</td>
                  <td class="text-right py-2 px-3 text-green-600">
                    {{ op.successCount.toLocaleString() }}
                  </td>
                  <td class="text-right py-2 px-3 text-red-600">
                    {{ op.failedCount.toLocaleString() }}
                  </td>
                  <td class="text-right py-2 px-3">{{ op.avgDurationMs.toFixed(0) }}ms</td>
                  <td class="text-right py-2 px-3">${{ getOperationCost(op).toFixed(4) }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Top Buckets -->
          <div class="border rounded-lg p-4">
            <h3 class="font-medium mb-4">{{ t('metricsTopBuckets') }}</h3>
            <div v-if="topBuckets.length === 0" class="text-center py-4 text-muted-foreground">
              {{ t('metricsNoData') }}
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="bucket in topBuckets"
                :key="bucket.bucketName"
                class="flex items-center justify-between"
              >
                <span class="font-mono text-sm truncate max-w-[300px]">{{ bucket.bucketName }}</span>
                <span class="text-muted-foreground text-sm"
                  >{{ bucket.requestCount.toLocaleString() }} {{ t('metricsRequests') }}</span
                >
              </div>
            </div>
          </div>

          <!-- Errors Section -->
          <div v-if="errorData.length > 0" class="border border-red-500/30 rounded-lg p-4">
            <h3 class="font-medium mb-4 text-red-600">{{ t('metricsErrors') }}</h3>
            <div class="space-y-2">
              <div
                v-for="error in errorData"
                :key="error.category"
                class="flex items-center justify-between"
              >
                <span class="text-sm">{{ error.category }}</span>
                <div class="flex items-center gap-4">
                  <span class="text-red-600 font-medium">{{ error.count }}</span>
                  <span class="text-xs text-muted-foreground">{{
                    formatDate(error.lastOccurrence)
                  }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Storage Info -->
          <div class="flex items-center justify-between text-sm text-muted-foreground border-t pt-4">
            <div>
              {{ t('metricsStorageInfo') }}: {{ storageInfo.requestCount.toLocaleString() }}
              {{ t('metricsRecords') }}
              <span v-if="storageInfo.oldestDate"> ({{ t('metricsSince') }} {{ storageInfo.oldestDate }})</span>
            </div>
            <Button size="sm" variant="destructive" @click="showPurgeConfirm = true">
              {{ t('metricsPurge') }}
            </Button>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" @click="showMetricsModal = false">{{ t('close') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Purge Confirmation Dialog -->
    <Dialog v-model:open="showPurgeConfirm">
      <DialogContent class="max-w-md">
        <DialogHeader>
          <DialogTitle class="flex items-center gap-2 text-destructive">
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
            >
              <path d="M3 6h18" />
              <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
              <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
              <line x1="10" x2="10" y1="11" y2="17" />
              <line x1="14" x2="14" y1="11" y2="17" />
            </svg>
            {{ t('metricsPurge') }}
          </DialogTitle>
        </DialogHeader>
        <div class="py-4">
          <p class="text-sm text-muted-foreground">
            {{ t('metricsPurgeConfirm') }}
          </p>
          <p class="text-sm text-muted-foreground mt-2">
            <strong>{{ storageInfo.requestCount.toLocaleString() }}</strong> {{ t('metricsRecords') }}
          </p>
        </div>
        <DialogFooter class="gap-2">
          <Button variant="outline" @click="showPurgeConfirm = false">
            {{ t('cancel') }}
          </Button>
          <Button variant="destructive" @click="confirmPurge">
            {{ t('metricsPurge') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useMetrics, initMetricsListener } from '@/composables/useMetrics'
import { useCacheMetrics } from '@/composables/useCacheMetrics'
import { metricsStorage } from '@/services/metricsStorage'
import { useI18n } from '@/composables/useI18n'
import { useToast } from '@/composables/useToast'
import { useSettingsStore } from '@/stores/settings'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Tooltip } from '@/components/ui/tooltip'
import type { OperationStats, ErrorStats, BucketUsageStats, HourlyStats, S3Provider, CacheSummary } from '@/types/metrics'
import { OPERATION_CATEGORY_MAP, PROVIDER_INFO } from '@/types/metrics'

const { t } = useI18n()
const toast = useToast()
const metrics = useMetrics()
const cacheMetrics = useCacheMetrics()
const settingsStore = useSettingsStore()

const showMetricsModal = ref(false)
const showPurgeConfirm = ref(false)
const selectedPeriod = ref(1)
const isLoading = ref(false)
const showProviderDropdown = ref(false)

// Helper to create empty hourly data (24 hours)
const createEmptyHourlyData = (): HourlyStats[] =>
  Array.from({ length: 24 }, (_, i) => ({ hour: i, count: 0, successCount: 0, failedCount: 0 }))

// Data
const operationData = ref<OperationStats[]>([])
const errorData = ref<ErrorStats[]>([])
const topBuckets = ref<BucketUsageStats[]>([])
const hourlyData = ref<HourlyStats[]>(createEmptyHourlyData())
const storageInfo = ref<{ requestCount: number; oldestDate: string | null }>({
  requestCount: 0,
  oldestDate: null,
})
const cacheSummary = ref<CacheSummary | null>(null)

// Period options
const periods = [
  { days: 1, label: t('metricsToday') },
  { days: 7, label: '7d' },
  { days: 30, label: '30d' },
]

// Computed from useMetrics
const totalRequestsToday = computed(() => metrics.totalRequestsToday.value)
const errorRateToday = computed(() => metrics.errorRateToday.value)
const requestsPerHour = computed(() => metrics.requestsPerHour.value)

// Calculate cost today using selected provider pricing (not hardcoded AWS pricing)
const estimatedCostToday = computed(() => {
  const stats = metrics.todayStats.value
  if (!stats) return 0

  const pricing = currentPricing.value
  const listCost = (stats.listRequests / 1000) * pricing.listPerThousand
  const getCost = (stats.getRequests / 1000) * pricing.getPerThousand
  const putCost = (stats.putRequests / 1000) * pricing.putPerThousand
  // DELETE is typically free for all providers

  return listCost + getCost + putCost
})

// Calculate monthly projection using selected provider pricing
const estimatedCostMonth = computed(() => {
  const today = new Date()
  const daysInMonth = new Date(today.getFullYear(), today.getMonth() + 1, 0).getDate()
  const dayOfMonth = today.getDate()
  const dailyCost = estimatedCostToday.value
  return (dailyCost / Math.max(dayOfMonth, 1)) * daysInMonth
})

// Category data for chart
const categoryData = computed(() => {
  const stats = metrics.todayStats.value
  if (!stats || stats.totalRequests === 0) return []

  const total = stats.totalRequests
  return [
    {
      name: 'LIST',
      count: stats.listRequests,
      percentage: (stats.listRequests / total) * 100,
      color: 'bg-blue-500',
    },
    {
      name: 'GET',
      count: stats.getRequests,
      percentage: (stats.getRequests / total) * 100,
      color: 'bg-green-500',
    },
    {
      name: 'PUT',
      count: stats.putRequests,
      percentage: (stats.putRequests / total) * 100,
      color: 'bg-orange-500',
    },
    {
      name: 'DELETE',
      count: stats.deleteRequests,
      percentage: (stats.deleteRequests / total) * 100,
      color: 'bg-red-500',
    },
  ].filter((c) => c.count > 0)
})

// Max hourly count for chart scaling
const maxHourlyCount = computed(() => {
  if (hourlyData.value.length === 0) return 1
  return Math.max(...hourlyData.value.map((h) => h.count), 1)
})

// Load data when modal opens
watch(
  showMetricsModal,
  async (isOpen) => {
    if (isOpen) {
      // Ensure metrics storage is initialized before loading
      await metricsStorage.init().catch(() => {})
      await loadData()
    }
  },
  { flush: 'post' }
)

// Reload when period changes
watch(selectedPeriod, async () => {
  await loadData()
})

// Initialize metrics listener and cache metrics on mount
onMounted(async () => {
  await initMetricsListener()
  // Initialize cache metrics (required for cache summary)
  await cacheMetrics.init().catch((e) => console.warn('Failed to init cache metrics:', e))
  // Add click outside listener
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})

// Close dropdown when clicking outside
function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (showProviderDropdown.value && !target.closest('.relative')) {
    showProviderDropdown.value = false
  }
}

async function loadData() {
  isLoading.value = true
  try {
    // Refresh today stats first (needed for KPI cards)
    await metrics.refreshTodayStats()

    // Load main metrics data
    const [ops, errors, buckets, hourly, storage] = await Promise.all([
      metrics.getOperationStats(selectedPeriod.value),
      metrics.getErrorStats(selectedPeriod.value),
      metrics.getTopBuckets(selectedPeriod.value, 5),
      metrics.getHourlyStats(),
      metrics.getStorageInfo(),
    ])

    operationData.value = ops
    errorData.value = errors
    topBuckets.value = buckets
    hourlyData.value = hourly
    storageInfo.value = storage

    // Load cache summary separately (optional, won't break main data if it fails)
    try {
      cacheSummary.value = await cacheMetrics.getCacheSummary(selectedPeriod.value)
    } catch (cacheError) {
      console.warn('Failed to load cache summary:', cacheError)
      cacheSummary.value = null
    }
  } catch (error) {
    console.error('Failed to load metrics data:', error)
  } finally {
    isLoading.value = false
  }
}

async function refreshData() {
  await metrics.refreshTodayStats()
  await loadData()
}

// Current pricing (computed from settings)
const currentPricing = computed(() => settingsStore.getCurrentPricing)

// Provider selection
const selectedProvider = computed({
  get: () => settingsStore.metricsProvider,
  set: (value: S3Provider) => settingsStore.setMetricsProvider(value),
})

// Get current provider display name
const currentProviderName = computed(() => {
  const provider = PROVIDER_INFO.find((p) => p.id === selectedProvider.value)
  return provider?.name ?? 'AWS S3'
})

// Select provider and close dropdown
function selectProvider(providerId: S3Provider) {
  selectedProvider.value = providerId
  showProviderDropdown.value = false
}

// Custom pricing (for custom provider)
const customPricingGet = computed({
  get: () => settingsStore.customPricing.getPerThousand,
  set: (value: number) => {
    settingsStore.setCustomPricing({
      ...settingsStore.customPricing,
      getPerThousand: value,
    })
  },
})
const customPricingPut = computed({
  get: () => settingsStore.customPricing.putPerThousand,
  set: (value: number) => {
    settingsStore.setCustomPricing({
      ...settingsStore.customPricing,
      putPerThousand: value,
    })
  },
})
const customPricingList = computed({
  get: () => settingsStore.customPricing.listPerThousand,
  set: (value: number) => {
    settingsStore.setCustomPricing({
      ...settingsStore.customPricing,
      listPerThousand: value,
    })
  },
})

// Cache hit rate gauge
const cacheHitRate = computed(() => cacheSummary.value?.hitRate ?? 0)
const cacheRequestsSaved = computed(() => cacheSummary.value?.requestsSaved ?? 0)
const cacheCostSaved = computed(() => cacheSummary.value?.costSaved ?? 0)

function getOperationCost(op: OperationStats): number {
  const category = OPERATION_CATEGORY_MAP[op.operation]
  const pricing = currentPricing.value

  switch (category) {
    case 'GET':
      return (op.count / 1000) * pricing.getPerThousand
    case 'PUT':
      return (op.count / 1000) * pricing.putPerThousand
    case 'LIST':
      return (op.count / 1000) * pricing.listPerThousand
    default:
      return 0
  }
}

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString()
}

async function confirmPurge() {
  try {
    const count = storageInfo.value.requestCount
    await metrics.clearAllData()
    showPurgeConfirm.value = false
    toast.success(`${count} ${t('metricsRecordsDeleted')}`)
    // Reset hourly data to empty
    hourlyData.value = createEmptyHourlyData()
    await loadData()
  } catch (error) {
    console.error('Failed to purge data:', error)
    toast.error(t('errorOccurred'))
  }
}

</script>
