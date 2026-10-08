import { translate } from '@/locales'

export function getErrorMessage(error: unknown, fallback?: string): string {
  const defaultMessage = fallback ?? translate('common.operationFailed')
  if (typeof error === 'string') return error || defaultMessage
  if (error instanceof Error) return error.message || defaultMessage
  if (typeof error === 'object' && error !== null && 'message' in error) return String((error as any).message) || defaultMessage
  return defaultMessage
}