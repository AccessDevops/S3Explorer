<script setup lang="ts">
import { inject, computed, type Ref } from 'vue'

const props = defineProps<{
  value: string
}>()

const activeTab = inject<Ref<string>>('activeTab')
const setActiveTab = inject<(value: string) => void>('setActiveTab')

const isActive = computed(() => activeTab?.value === props.value)
</script>

<template>
  <button
    @click="setActiveTab?.(value)"
    :class="[
      'px-4 py-2 text-sm font-medium transition-colors',
      'hover:text-foreground border-b-2',
      isActive
        ? 'border-primary text-foreground'
        : 'border-transparent text-muted-foreground hover:border-border',
    ]"
  >
    <slot />
  </button>
</template>
