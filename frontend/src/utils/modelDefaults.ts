import type { CliType } from '@/types/models'
import { translate } from '@/locales'

export const DEFAULT_MODEL_NAMES: Record<string, string> = {
  claude_code: 'claude-opus-4-8',
  codex: 'gpt-5.5',
  gemini: 'gemini-3.1-pro-preview',
}

function modelStorageKey(cliType: CliType): string {
  return `detect_model_${cliType}`
}

export function getReusableModelName(cliType: CliType): string {
  return localStorage.getItem(modelStorageKey(cliType)) || DEFAULT_MODEL_NAMES[cliType] || ''
}

export function saveReusableModelName(cliType: CliType, modelName: string): void {
  const value = modelName.trim()
  if (value) localStorage.setItem(modelStorageKey(cliType), value)
}

export function defaultTestText(): string {
  return translate('common.defaultTestText')
}

function testTextStorageKey(cliType: CliType): string {
  return `detect_text_${cliType}`
}

export function getReusableTestText(cliType: CliType): string {
  return localStorage.getItem(testTextStorageKey(cliType)) || defaultTestText()
}

export function saveReusableTestText(cliType: CliType, text: string): void {
  const value = text.trim()
  if (value) localStorage.setItem(testTextStorageKey(cliType), value)
}
