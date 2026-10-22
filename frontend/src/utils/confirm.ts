import { createApp, h, ref, onMounted } from 'vue'
import AppConfirm from '@/components/AppConfirm.vue'
import { i18n } from '@/locales'

interface ConfirmOptions {
  confirmText?: string
  cancelText?: string
  markdown?: boolean
}

export function confirm(
  message: string,
  title?: string,
  options?: ConfirmOptions
): Promise<void> {
  return new Promise((resolve, reject) => {
    const container = document.createElement('div')
    document.body.appendChild(container)

    let cleanup: () => void

    const Comp = {
      setup() {
        const modalRef = ref<{ open: (msg: string, t?: string, opts?: ConfirmOptions) => Promise<boolean> } | null>(null)

        onMounted(() => {
          modalRef.value?.open(message, title, options).then((result) => {
            cleanup()
            if (result) resolve()
            else reject('cancel')
          })
        })

        return () => h(AppConfirm, { ref: modalRef })
      }
    }

    const app = createApp(Comp)
    // 独立挂载的 app 不继承主 app 的插件，需单独安装 i18n，否则 AppConfirm 内 useI18n 会抛错
    app.use(i18n)
    cleanup = () => {
      app.unmount()
      container.remove()
    }
    app.mount(container)
  })
}
