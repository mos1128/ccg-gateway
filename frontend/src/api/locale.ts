import { invoke } from './tauri-bridge'

export type LanguageSetting = 'auto' | 'zh-CN' | 'en'
export type ResolvedLocale = 'zh-CN' | 'en'

export interface UiLocale {
  language: LanguageSetting
  locale: ResolvedLocale
}

export const localeApi = {
  get: () => invoke<UiLocale>('get_ui_locale'),
  set: (language: LanguageSetting) => invoke<UiLocale>('set_ui_language', { language })
}
