import { ref } from 'vue'

interface DialogOptions {
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  variant?: 'default' | 'destructive'
}

const isOpen = ref(false)
const dialogTitle = ref('')
const dialogMessage = ref('')
const dialogConfirmText = ref('OK')
const dialogCancelText = ref('Cancel')
const dialogVariant = ref<'default' | 'destructive'>('default')
const dialogResolve = ref<((value: boolean) => void) | null>(null)

export function useDialog() {
  const confirm = (options: DialogOptions): Promise<boolean> => {
    return new Promise((resolve) => {
      dialogTitle.value = options.title
      dialogMessage.value = options.message
      dialogConfirmText.value = options.confirmText || 'Confirm'
      dialogCancelText.value = options.cancelText || 'Cancel'
      dialogVariant.value = options.variant || 'default'
      dialogResolve.value = resolve
      isOpen.value = true
    })
  }

  const handleConfirm = () => {
    isOpen.value = false
    if (dialogResolve.value) {
      dialogResolve.value(true)
      dialogResolve.value = null
    }
  }

  const handleCancel = () => {
    isOpen.value = false
    if (dialogResolve.value) {
      dialogResolve.value(false)
      dialogResolve.value = null
    }
  }

  const handleClose = () => {
    isOpen.value = false
    if (dialogResolve.value) {
      dialogResolve.value(false)
      dialogResolve.value = null
    }
  }

  return {
    // Dialog state
    isOpen,
    dialogTitle,
    dialogMessage,
    dialogConfirmText,
    dialogCancelText,
    dialogVariant,

    // Methods
    confirm,
    handleConfirm,
    handleCancel,
    handleClose,
  }
}
