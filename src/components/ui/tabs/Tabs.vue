<!-- eslint-disable vue/multi-word-component-names -->
<script setup lang="ts">
import { provide, ref } from 'vue'

const props = defineProps<{
  defaultValue: string
  modelValue?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const activeTab = ref(props.modelValue || props.defaultValue)

function setActiveTab(value: string) {
  activeTab.value = value
  emit('update:modelValue', value)
}

provide('activeTab', activeTab)
provide('setActiveTab', setActiveTab)
</script>

<template>
  <div class="tabs">
    <slot />
  </div>
</template>
