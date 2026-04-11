import { invoke } from '@tauri-apps/api/core'
import type { SkillRepo, SkillRepoCreate, DiscoverableSkill, InstalledSkill, SkillFavoriteItem } from '@/types/models'

// 后端返回的 cli_flags 格式
type SkillCliFlagBackend = { cli_type: string; enabled: boolean }
type InstalledSkillBackend = Omit<InstalledSkill, 'cli_flags'> & { cli_flags: SkillCliFlagBackend[] }

// 将后端数组格式转换为前端对象格式
function transformCliFlags(cliFlags: SkillCliFlagBackend[]): Record<string, boolean> {
  const result: Record<string, boolean> = {}
  for (const flag of cliFlags) {
    result[flag.cli_type] = flag.enabled
  }
  return result
}

function transformInstalledSkill(skill: InstalledSkillBackend): InstalledSkill {
  return {
    ...skill,
    cli_flags: transformCliFlags(skill.cli_flags)
  }
}

export const skillsApi = {
  // ==================== 仓库管理 ====================
  getRepos: async (): Promise<SkillRepo[]> => {
    return await invoke<SkillRepo[]>('get_skill_repos')
  },

  addRepo: async (input: SkillRepoCreate): Promise<SkillRepo> => {
    return await invoke<SkillRepo>('add_skill_repo', { input })
  },

  removeRepo: async (name: string): Promise<void> => {
    await invoke('remove_skill_repo', { name })
  },

  // ==================== Skill 发现 ====================
  discoverRepoSkills: async (name: string): Promise<DiscoverableSkill[]> => {
    return await invoke<DiscoverableSkill[]>('discover_repo_skills', { name })
  },

  reinstallRepo: async (name: string): Promise<DiscoverableSkill[]> => {
    return await invoke<DiscoverableSkill[]>('reinstall_skill_repo', { name })
  },

  // ==================== Skill 安装/卸载 ====================
  install: async (skill: DiscoverableSkill, reinstall: boolean = false): Promise<InstalledSkill> => {
    const result = await invoke<InstalledSkillBackend>('install_skill', { skill, reinstall })
    return transformInstalledSkill(result)
  },

  reinstall: async (directory: string): Promise<InstalledSkill> => {
    const result = await invoke<InstalledSkillBackend>('reinstall_skill', { directory })
    return transformInstalledSkill(result)
  },

  uninstall: async (id: string): Promise<void> => {
    await invoke('uninstall_skill', { id })
  },

  // ==================== 已安装 Skill 管理 ====================
  getInstalled: async (): Promise<InstalledSkill[]> => {
    const data = await invoke<InstalledSkillBackend[]>('get_installed_skills')
    return data.map(transformInstalledSkill)
  },

  toggleCli: async (id: string, cliType: string, enabled: boolean): Promise<void> => {
    await invoke('toggle_skill_cli', { id, cliType, enabled })
  },

  // ==================== Skill 收藏 ====================
  getFavorites: async (): Promise<SkillFavoriteItem[]> => {
    return await invoke<SkillFavoriteItem[]>('get_skill_favorites')
  },

  addFavorite: async (skill: DiscoverableSkill): Promise<void> => {
    await invoke('add_skill_favorite', { skillItem: skill })
  },

  toggleInstalledFavorite: async (directory: string): Promise<boolean> => {
    return await invoke<boolean>('toggle_installed_skill_favorite', { directory })
  },

  removeFavorite: async (key: string): Promise<void> => {
    await invoke('remove_skill_favorite', { key })
  },

  installFavorite: async (key: string): Promise<InstalledSkill> => {
    const result = await invoke<InstalledSkillBackend>('install_favorite_skill', { key })
    return transformInstalledSkill(result)
  },

  reinstallFavorite: async (key: string): Promise<InstalledSkill> => {
    const result = await invoke<InstalledSkillBackend>('reinstall_favorite_skill', { key })
    return transformInstalledSkill(result)
  },
}
