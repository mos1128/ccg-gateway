import { defineStore } from 'pinia'
import { ref } from 'vue'
import { detectBrowserLocale, setI18nLocale, type AppLocale } from '@/locales'
import { localeApi, type LanguageSetting, type UiLocale } from '@/api/locale'

const CACHE_KEY = 'ui-locale'

export type { LanguageSetting } from '@/api/locale'

interface LocaleCache {
  language: LanguageSetting
  locale: AppLocale
}

function readCache(): LocaleCache | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(CACHE_KEY) || 'null')
    if (typeof value !== 'object' || value === null) return null
    const { language, locale } = value as Partial<LocaleCache>
    if (language !== 'auto' && language !== 'zh-CN' && language !== 'en') return null
    if (locale !== 'zh-CN' && locale !== 'en') return null
    return { language, locale }
  } catch {
    return null
  }
}

export const useLocaleStore = defineStore('locale', () => {
  const cached = readCache()
  const language = ref<LanguageSetting>(cached?.language ?? 'auto')
  const locale = ref<AppLocale>(cached?.locale ?? detectBrowserLocale())

  function apply() {
    setI18nLocale(locale.value)
    document.documentElement.lang = locale.value
    try {
      localStorage.setItem(
        CACHE_KEY,
        JSON.stringify({ language: language.value, locale: locale.value })
      )
    } catch (e) {
      console.error('Failed to cache UI locale:', e)
    }
  }

  function applyState(state: UiLocale) {
    language.value = state.language
    locale.value = state.locale
    apply()
  }

  /** 先按本地缓存即时渲染，再与后端设置对账（首次运行 / 外部修改场景） */
  async function init() {
    apply()
    try {
      const state = await localeApi.get()
      if (state.language !== language.value || state.locale !== locale.value) {
        applyState(state)
      }
    } catch (e) {
      console.error('Failed to load UI locale:', e)
    }
  }

  async function setLanguage(next: LanguageSetting) {
    try {
      const state = await localeApi.set(next)
      applyState(state)
    } catch (e) {
      console.error('Failed to set UI language:', e)
    }
  }

  return { language, locale, init, setLanguage }
})
