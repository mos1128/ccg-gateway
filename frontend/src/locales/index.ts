import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import en from './en'

export type AppLocale = 'zh-CN' | 'en'

export function detectBrowserLocale(): AppLocale {
  const language = typeof navigator === 'undefined' ? '' : navigator.language || ''
  return language.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en'
}

// 消息对象按域拆分，locale 消息类型不纳入 t() 的键推导（键由各域模块自己的
// `satisfies typeof zh` 保证中英对齐），避免动态键（如 `logs.${code}`）触发类型报错。
const messages = {
  'zh-CN': zhCN,
  en
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
} as any

export const i18n = createI18n({
  legacy: false,
  locale: detectBrowserLocale(),
  fallbackLocale: 'en',
  messages
})

export function setI18nLocale(locale: AppLocale) {
  i18n.global.locale.value = locale
}

/** 供非组件模块（utils / stores 等）使用的翻译入口 */
export function translate(key: string, params?: Record<string, unknown>): string {
  const t = i18n.global.t as unknown as (
    key: string,
    params?: Record<string, unknown>
  ) => string
  return params ? t(key, params) : t(key)
}
