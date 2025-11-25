<script setup lang="ts">
import { TooltipRoot } from 'radix-vue'

interface Props {
  defaultOpen?: boolean
  open?: boolean
  delayDuration?: number
  disableHoverableContent?: boolean
  disableClosingTrigger?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  defaultOpen: false,
  delayDuration: undefined,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()
</script>

<template>
  <TooltipRoot
    :default-open="props.defaultOpen"
    :open="props.open"
    :delay-duration="props.delayDuration"
    :disable-hoverable-content="props.disableHoverableContent"
    :disable-closing-trigger="props.disableClosingTrigger"
    @update:open="emit('update:open', $event)"
  >
    <slot />
  </TooltipRoot>
</template>
