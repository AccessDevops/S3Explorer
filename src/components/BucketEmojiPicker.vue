<script setup lang="ts">
import { ref, computed } from 'vue'
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover'
import { useBucketEmojis, BUCKET_EMOJIS } from '@/composables/useBucketEmojis'

const props = defineProps<{
  profileId: string
  bucketName: string
}>()

const isOpen = ref(false)
const { getEmoji, setEmoji } = useBucketEmojis()

const currentEmoji = computed(() => getEmoji(props.profileId, props.bucketName))

function selectEmoji(emoji: string) {
  setEmoji(props.profileId, props.bucketName, emoji)
  isOpen.value = false
}
</script>

<template>
  <Popover v-model:open="isOpen">
    <PopoverTrigger as-child>
      <button
        @click.stop
        class="text-lg hover:scale-110 transition-transform cursor-pointer select-none"
      >
        {{ currentEmoji }}
      </button>
    </PopoverTrigger>
    <PopoverContent align="start" :side-offset="8" class="w-auto p-2">
      <div class="grid grid-cols-7 gap-1 max-h-[216px] overflow-y-auto pr-1">
        <button
          v-for="emoji in BUCKET_EMOJIS"
          :key="emoji"
          @click="selectEmoji(emoji)"
          class="w-8 h-8 flex items-center justify-center text-lg rounded cursor-pointer transition-colors hover:bg-muted"
          :class="{ 'bg-primary/20 ring-1 ring-primary/40': emoji === currentEmoji }"
        >
          {{ emoji }}
        </button>
      </div>
    </PopoverContent>
  </Popover>
</template>
