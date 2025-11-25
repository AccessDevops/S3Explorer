<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { TooltipRoot, TooltipTrigger, TooltipPortal, TooltipContent, TooltipArrow } from 'radix-vue'
import { cn } from '@/lib/utils'

interface Props {
  text?: string
  side?: 'top' | 'right' | 'bottom' | 'left'
  sideOffset?: number
  align?: 'start' | 'center' | 'end'
  delayDuration?: number
  disabled?: boolean
  class?: HTMLAttributes['class']
  contentClass?: HTMLAttributes['class']
  showArrow?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  side: 'top',
  sideOffset: 6,
  align: 'center',
  delayDuration: 200,
  disabled: false,
  showArrow: true,
})
</script>

<template>
  <template v-if="disabled || (!text && !$slots.content)">
    <slot />
  </template>
  <TooltipRoot v-else :delay-duration="delayDuration">
    <TooltipTrigger as-child :class="props.class">
      <slot />
    </TooltipTrigger>
    <TooltipPortal>
      <TooltipContent
        :side="side"
        :side-offset="sideOffset"
        :align="align"
        :class="cn(
          'z-50 overflow-hidden rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground shadow-md',
          'animate-in fade-in-0 zoom-in-95',
          'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
          'data-[side=bottom]:slide-in-from-top-2',
          'data-[side=left]:slide-in-from-right-2',
          'data-[side=right]:slide-in-from-left-2',
          'data-[side=top]:slide-in-from-bottom-2',
          contentClass
        )"
      >
        <slot name="content">{{ text }}</slot>
        <TooltipArrow
          v-if="showArrow"
          class="fill-primary"
          :width="8"
          :height="4"
        />
      </TooltipContent>
    </TooltipPortal>
  </TooltipRoot>
</template>
