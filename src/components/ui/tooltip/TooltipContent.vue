<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { TooltipContent, TooltipPortal, TooltipArrow } from 'radix-vue'
import { cn } from '@/lib/utils'

interface Props {
  class?: HTMLAttributes['class']
  side?: 'top' | 'right' | 'bottom' | 'left'
  sideOffset?: number
  align?: 'start' | 'center' | 'end'
  alignOffset?: number
  avoidCollisions?: boolean
  showArrow?: boolean
  arrowClass?: HTMLAttributes['class']
}

const props = withDefaults(defineProps<Props>(), {
  side: 'top',
  sideOffset: 6,
  align: 'center',
  alignOffset: 0,
  avoidCollisions: true,
  showArrow: true,
})
</script>

<template>
  <TooltipPortal>
    <TooltipContent
      :side="props.side"
      :side-offset="props.sideOffset"
      :align="props.align"
      :align-offset="props.alignOffset"
      :avoid-collisions="props.avoidCollisions"
      :class="cn(
        'z-50 overflow-hidden rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground shadow-md',
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
      <TooltipArrow
        v-if="props.showArrow"
        :class="cn('fill-primary', props.arrowClass)"
        :width="8"
        :height="4"
      />
    </TooltipContent>
  </TooltipPortal>
</template>
