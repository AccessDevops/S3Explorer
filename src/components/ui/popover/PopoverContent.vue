<script setup lang="ts">
import type { PopoverContentProps } from 'radix-vue'
import type { HTMLAttributes } from 'vue'
import { PopoverPortal, PopoverContent, useForwardProps } from 'radix-vue'
import { cn } from '@/lib/utils'

interface Props extends PopoverContentProps {
  class?: HTMLAttributes['class']
}

const props = withDefaults(defineProps<Props>(), {
  side: 'bottom',
  sideOffset: 4,
  align: 'start',
})

const forwarded = useForwardProps(() => ({
  side: props.side,
  sideOffset: props.sideOffset,
  align: props.align,
  alignOffset: props.alignOffset,
  avoidCollisions: props.avoidCollisions,
  collisionBoundary: props.collisionBoundary,
  collisionPadding: props.collisionPadding,
  sticky: props.sticky,
  hideWhenDetached: props.hideWhenDetached,
}))
</script>

<template>
  <PopoverPortal>
    <PopoverContent
      v-bind="forwarded"
      :class="cn(
        'z-50 rounded-md border bg-popover p-2 text-popover-foreground shadow-md outline-none',
        'animate-in fade-in-0 zoom-in-95',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
        'data-[side=bottom]:slide-in-from-top-2',
        'data-[side=left]:slide-in-from-right-2',
        'data-[side=right]:slide-in-from-left-2',
        'data-[side=top]:slide-in-from-bottom-2',
        props.class
      )"
    >
      <slot />
    </PopoverContent>
  </PopoverPortal>
</template>
