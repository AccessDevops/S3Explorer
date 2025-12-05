<script setup lang="ts">
import type { PopoverTriggerProps } from 'radix-vue'
import { PopoverTrigger, useForwardProps } from 'radix-vue'

const props = defineProps<PopoverTriggerProps>()

const forwarded = useForwardProps(props)
</script>

<template>
  <PopoverTrigger v-bind="forwarded">
    <slot />
  </PopoverTrigger>
</template>
