/**
 * 校验 JSON 格式
 * @returns 错误信息，校验通过返回空字符串
 */
export function validateJson(input: string): string {
  const trimmed = input.trim()
  if (!trimmed) return ''
  try {
    JSON.parse(trimmed)
    return ''
  } catch (e) {
    return `JSON 格式错误: ${(e as Error).message}`
  }
}

/**
 * 格式化 JSON（2空格缩进）
 * @returns 格式化后的字符串，失败返回原字符串
 */
export function formatJson(input: string): string {
  const trimmed = input.trim()
  if (!trimmed) return input
  try {
    const parsed = JSON.parse(trimmed)
    return JSON.stringify(parsed, null, 2)
  } catch {
    return input
  }
}