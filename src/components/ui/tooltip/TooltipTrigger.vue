<script setup lang="ts">
import { TooltipTrigger } from 'radix-vue'

interface Props {
  asChild?: boolean
}

withDefaults(defineProps<Props>(), {
  asChild: true,
})
</script>

<template>
  <TooltipTrigger :as-child="asChild">
    <slot />
  </TooltipTrigger>
</template>
