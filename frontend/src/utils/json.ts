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

/**
 * 格式化 Token 数量
 * 小于 1000 显示原数字
 * 1000 - 999,999 显示为 K 单位
 * 1,000,000 及以上显示为 M 单位
 */
export function formatTokens(tokens: number | undefined): string {
  if (!tokens) return '0'
  if (tokens < 1000) return tokens.toString()
  if (tokens < 1000000) return (tokens / 1000).toFixed(1) + 'K'
  return (tokens / 1000000).toFixed(1) + 'M'
}