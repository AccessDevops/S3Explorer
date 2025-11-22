<template>
  <div id="app" class="w-full h-screen overflow-hidden">
    <RouterView />

    <!-- Global Confirmation Dialog -->
    <ConfirmDialog
      :is-open="dialog.isOpen.value"
      :title="dialog.dialogTitle.value"
      :message="dialog.dialogMessage.value"
      :confirm-text="dialog.dialogConfirmText.value"
      :cancel-text="dialog.dialogCancelText.value"
      :variant="dialog.dialogVariant.value"
      @confirm="dialog.handleConfirm"
      @cancel="dialog.handleCancel"
      @close="dialog.handleClose"
    />

    <!-- Toast Notifications -->
    <ToastContainer />

    <!-- Upload Progress (Rust-managed) -->
    <RustUploadProgress />
  </div>
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router'
import ConfirmDialog from './components/ConfirmDialog.vue'
import ToastContainer from './components/ToastContainer.vue'
import RustUploadProgress from './components/RustUploadProgress.vue'
import { useDialog } from './composables/useDialog'

const dialog = useDialog()
</script>
