import { ElNotification } from 'element-plus'
import { translate } from '@/locales'

type NotifyType = 'success' | 'error' | 'warning' | 'info'

export function notify(message: string, type: NotifyType = 'success') {
  ElNotification({
    title: translate(`common.notify.${type}`),
    message,
    type,
    position: 'top-right',
    offset: 48,
    duration: 2000
  })
}
