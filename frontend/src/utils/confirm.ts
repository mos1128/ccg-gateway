import { createApp, h, ref, onMounted } from 'vue'
import AppConfirm from '@/components/AppConfirm.vue'

export function confirm(
  message: string,
  title?: string,
  options?: { confirmText?: string; cancelText?: string }
): Promise<void> {
  return new Promise((resolve, reject) => {
    const container = document.createElement('div')
    document.body.appendChild(container)

    let cleanup: () => void

    const Comp = {
      setup() {
        const modalRef = ref<{ open: (msg: string, t?: string, opts?: { confirmText?: string; cancelText?: string }) => Promise<boolean> } | null>(null)

        onMounted(() => {
          modalRef.value?.open(message, title, options).then((result) => {
            cleanup()
            if (result) resolve()
            else reject(new Error('cancel'))
          })
        })

        return () => h(AppConfirm, { ref: modalRef })
      }
    }

    const app = createApp(Comp)
    cleanup = () => {
      app.unmount()
      container.remove()
    }
    app.mount(container)
  })
}