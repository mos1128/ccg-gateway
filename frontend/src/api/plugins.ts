import { invoke } from '@/api/tauri-bridge'
import type { PluginItem, PluginActionResult } from '@/types/models'

export const pluginsApi = {
  // 获取已安装插件列表（覆盖所有已初始化的 dsh profile）
  getInstalled: async (): Promise<PluginItem[]> => {
    return await invoke<PluginItem[]>('get_installed_plugins')
  },

  // 插件操作：install 传仓库地址，uninstall/update 传插件包名
  pluginAction: async (action: string, profile: string, param: string): Promise<PluginActionResult> => {
    return await invoke<PluginActionResult>('plugin_action', { action, profile, param })
  },

  // 便捷方法
  install: (profile: string, repoUrl: string) => pluginsApi.pluginAction('install', profile, repoUrl),
  uninstall: (profile: string, name: string) => pluginsApi.pluginAction('uninstall', profile, name),
  update: (profile: string, name: string) => pluginsApi.pluginAction('update', profile, name)
}
