import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { ElNotification } from 'element-plus'
import { notify } from '@/utils/notification'
import { confirm } from '@/utils/confirm'
import { translate } from '@/locales'

// GitHub 仓库配置
const GITHUB_OWNER = 'mos1128'
const GITHUB_REPO = 'ccg-gateway'

interface GitHubRelease {
  tag_name: string
  name: string | null
  body: string | null
  html_url: string
  published_at: string | null
}

/**
 * 比较语义化版本号
 * 返回: 1 表示 v1 > v2, -1 表示 v1 < v2, 0 表示相等
 */
function compareVersions(v1: string, v2: string): number {
  // 移除 'v' 前缀
  const normalize = (v: string) => v.replace(/^v/, '').split('.').map(n => parseInt(n, 10))
  const parts1 = normalize(v1)
  const parts2 = normalize(v2)
  
  for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
    const p1 = parts1[i] || 0
    const p2 = parts2[i] || 0
    if (p1 > p2) return 1
    if (p1 < p2) return -1
  }
  return 0
}

/**
 * 获取最新的 GitHub Release（通过 Rust 后端请求，支持系统代理）
 */
async function getLatestRelease(): Promise<GitHubRelease | null> {
  try {
    return await invoke<GitHubRelease | null>('check_for_updates')
  } catch (error) {
    console.error('获取最新版本失败:', error)
    throw error // 抛出错误以区分网络问题和无版本
  }
}

/**
 * 检查更新
 * @param silent 是否静默模式（静默模式下如果没有更新不会提示）
 */
export async function checkForUpdates(silent: boolean = true): Promise<void> {
  try {
    const currentVersion = await getVersion()
    const latestRelease = await getLatestRelease()
    
    if (!latestRelease) {
      // 仓库没有发布任何 release
      if (!silent) {
        notify(translate('updater.noRelease'), 'info')
      }
      return
    }
    
    const latestVersion = latestRelease.tag_name
    
    if (compareVersions(latestVersion, currentVersion) > 0) {
      // 有新版本
      const updateTitle = translate('updater.newVersionTitle', {
        version: latestVersion,
        current: currentVersion
      })
      const releaseBody = latestRelease.body?.trim()
      const releaseNotes = releaseBody
        ? releaseBody
        : translate('updater.noReleaseNotes')

      confirm(
        releaseNotes,
        updateTitle,
        { confirmText: translate('updater.download'), cancelText: translate('updater.later'), markdown: true }
      ).then(() => {
        open(latestRelease.html_url)
      }).catch(() => {})
    } else if (!silent) {
      notify(translate('updater.upToDate', { version: currentVersion }))
    }
  } catch (error) {
    console.error('检查更新失败:', error)
    if (!silent) {
      confirm(
        translate('updater.fetchFailed'),
        translate('updater.checkUpdate'),
        { confirmText: translate('updater.releasePage'), cancelText: translate('common.cancel') }
      ).then(() => {
        open(`https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases`)
      }).catch(() => {})
    }
  }
}

/**
 * 显示通知形式的更新提示
 */
export async function checkForUpdatesNotification(): Promise<void> {
  try {
    const currentVersion = await getVersion()
    const latestRelease = await getLatestRelease()
    
    if (!latestRelease) return
    
    const latestVersion = latestRelease.tag_name
    
    if (compareVersions(latestVersion, currentVersion) > 0) {
      ElNotification({
        title: translate('updater.foundTitle'),
        message: translate('updater.foundMessage', { version: latestVersion }),
        type: 'info',
        duration: 0,
        onClick: () => {
          open(latestRelease.html_url)
        }
      })
    }
  } catch (error) {
    console.error('检查更新失败:', error)
  }
}
