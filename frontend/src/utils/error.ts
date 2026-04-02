export function getErrorMessage(error: unknown, fallback: string = '操作失败'): string {
  if (typeof error === 'string') return error || fallback
  if (error instanceof Error) return error.message || fallback
  if (typeof error === 'object' && error !== null && 'message' in error) return String((error as any).message) || fallback
  return fallback
}