import type { CliType } from '@/types/models'

export const DEFAULT_MODEL_NAMES: Record<CliType, string> = {
  claude_code: 'claude-opus-4-8',
  codex: 'gpt-5.5',
  gemini: 'gemini-3.1-pro-preview'
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
