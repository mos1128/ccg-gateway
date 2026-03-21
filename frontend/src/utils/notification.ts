import { ElNotification } from 'element-plus'

type NotifyType = 'success' | 'error' | 'warning' | 'info'

const titles: Record<NotifyType, string> = {
  success: '成功',
  error: '错误',
  warning: '警告',
  info: '提示'
}

export function notify(message: string, type: NotifyType = 'success') {
  ElNotification({
    title: titles[type],
    message,
    type,
    position: 'top-right',
    duration: 3000
  })
}