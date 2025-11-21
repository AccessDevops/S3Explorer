<script setup lang="ts">
import { inject, computed, type Ref } from 'vue'

const props = defineProps<{
  value: string
}>()

const activeTab = inject<Ref<string>>('activeTab')
const isActive = computed(() => activeTab?.value === props.value)
</script>

<template>
  <div v-if="isActive" class="mt-4">
    <slot />
  </div>
</template>
