import { CLI_TYPES } from '@/types/models'
import type { CliFlagItem, CliType } from '@/types/models'

export function transformCliFlags(cliFlags: CliFlagItem[]): Record<CliType, boolean> {
  const result = {} as Record<CliType, boolean>
  for (const cliType of CLI_TYPES) {
    result[cliType] = false
  }
  for (const flag of cliFlags) {
    result[flag.cli_type] = flag.enabled
  }
  return result
}
