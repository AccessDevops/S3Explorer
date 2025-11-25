<script setup lang="ts">
import { TooltipProvider } from 'radix-vue'

interface Props {
  delayDuration?: number
  skipDelayDuration?: number
  disableHoverableContent?: boolean
}

withDefaults(defineProps<Props>(), {
  delayDuration: 200,
  skipDelayDuration: 300,
  disableHoverableContent: false,
})
</script>

<template>
  <TooltipProvider
    :delay-duration="delayDuration"
    :skip-delay-duration="skipDelayDuration"
    :disable-hoverable-content="disableHoverableContent"
  >
    <slot />
  </TooltipProvider>
</template>
