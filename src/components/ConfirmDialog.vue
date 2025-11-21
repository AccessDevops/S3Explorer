<template>
  <Dialog :open="isOpen" @update:open="handleClose">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription v-if="description">
          {{ description }}
        </DialogDescription>
      </DialogHeader>

      <div v-if="message" class="py-4">
        <p class="text-sm">{{ message }}</p>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="handleCancel">
          {{ cancelText || 'Cancel' }}
        </Button>
        <Button
          :variant="variant"
          @click="handleConfirm"
        >
          {{ confirmText || 'Confirm' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

interface Props {
  isOpen: boolean
  title: string
  message?: string
  description?: string
  confirmText?: string
  cancelText?: string
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link'
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
})

const emit = defineEmits<{
  confirm: []
  cancel: []
  close: []
}>()

function handleConfirm() {
  emit('confirm')
}

function handleCancel() {
  emit('cancel')
}

function handleClose(open: boolean) {
  if (!open) {
    emit('close')
  }
}
</script>
