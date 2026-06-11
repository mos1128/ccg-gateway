<template>
  <div class="sk-page">
    <div class="v2-tabs sk-tabs">
      <div class="v2-tab" :class="{ active: activeTab === 'skills' }" @click="activeTab = 'skills'">技能</div>
      <div class="v2-tab" :class="{ active: activeTab === 'repos' }" @click="activeTab = 'repos'">仓库</div>
      <div class="v2-tab" :class="{ active: activeTab === 'favorites' }" @click="activeTab = 'favorites'">收藏</div>
    </div>

    <!-- 技能 -->
    <template v-if="activeTab === 'skills'">
      <div v-loading="loadingInstalled || loadingInstalledOperation" class="sk-body">
        <V2Empty class="v2-card" v-if="installedList.length === 0" title="还没有已安装技能" description="到「仓库」或「收藏」里安装技能，启用后即可应用到各 Agent">
          <template #icon><SkillIcon :size="40" :stroke-width="1.6" /></template>
        </V2Empty>
        <div v-else class="v2-cardgrid">
          <div v-for="skill in installedList" :key="skill.id" class="v2-card v2-ccard">
            <div class="v2-ccard-head">
              <div class="v2-ccard-icon"><SkillIcon /></div>
              <div class="v2-ccard-tt">
                <div class="v2-ccard-name">{{ skill.name }}<span v-if="!skill.exists_on_disk" class="v2-pill v2-pill-warn sk-inline">失效</span></div>
                <div class="v2-ccard-sub mono">{{ skill.market_display }}</div>
              </div>
              <div class="v2-ccard-acts">
                <el-tooltip :content="skill.is_favorited ? '取消收藏' : '收藏'" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ 'sk-star-on': skill.is_favorited }" @click="toggleInstalledFavorite(skill)"><svg width="16" height="16" viewBox="0 0 24 24" :fill="skill.is_favorited ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg></button>
                </el-tooltip>
                <el-tooltip content="重装" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ off: installingSkillId === `installed-${skill.id}` }" @click="handleReinstallFromInstalled(skill)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
                </el-tooltip>
                <el-tooltip content="卸载" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act danger" @click="handleUninstall(skill)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                </el-tooltip>
              </div>
            </div>
            <V2CliChips :flags="skill.cli_flags" @toggle="(c, e) => handleCliToggle(skill, c, e)" />
          </div>
        </div>
      </div>
    </template>

    <!-- 仓库 -->
    <template v-else-if="activeTab === 'repos'">
      <template v-if="!currentRepo">
        <div v-loading="loadingRepos" class="sk-body">
          <div class="v2-cardgrid">
            <div class="v2-addcard v2-addcard-row" @click="showAddRepoDialog = true">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></svg>
              <span>添加仓库</span>
            </div>
            <div v-for="repo in repoList" :key="repo.name" class="v2-card v2-ccard sk-repocard" @click="handleRepoClick(repo)">
              <div class="v2-ccard-head">
                <div class="v2-ccard-icon"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9 12 3l9 6v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><polyline points="9 22 9 12 15 12 15 22"/></svg></div>
                <div class="v2-ccard-tt"><div class="v2-ccard-name">{{ repo.name }}</div><div class="v2-ccard-sub mono">{{ repo.source }}</div></div>
                <div class="v2-ccard-acts">
                  <el-tooltip content="重装" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act" @click.stop="handleReinstallRepo(repo)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
                  </el-tooltip>
                  <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act danger" @click.stop="handleRemoveRepo(repo)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                  </el-tooltip>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>
      <template v-else>
        <div class="sk-repohead">
          <div class="sk-repohead-l">
            <el-tooltip content="返回" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act" @click="handleBackToRepos"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg></button>
            </el-tooltip>
            <div><div class="sk-repohead-t">{{ currentRepo.name }}</div><div class="v2-hint mono">{{ currentRepo.source }}</div></div>
          </div>
          <div class="sk-repohead-r">
            <input v-model="skillSearchQuery" class="v2-input sk-search" placeholder="搜索…">
            <el-tooltip content="刷新" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act" :disabled="loadingSkills" @click="refreshRepoSkills"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
            </el-tooltip>
          </div>
        </div>
        <div v-loading="loadingSkills || loadingSkillsOperation" class="sk-body">
          <V2Empty class="v2-card" v-if="filteredSkillList.length === 0" :title="skillSearchQuery ? '无匹配结果' : '该仓库暂无 Skills'" />
          <div v-else class="v2-card sk-dlist">
            <div v-for="skill in filteredSkillList" :key="skill.key" class="sk-drow">
              <div class="sk-drow-icon">
                <SkillIcon :size="15" />
              </div>
              <div class="sk-dinfo">
                <div class="sk-dname">{{ skill.name }}</div>
                <div v-if="skill.description" class="sk-ddesc" @click="copyDescription(skill.description)">{{ skill.description }}</div>
              </div>
              <button class="v2-btn v2-btn-sm sk-install" :class="skill.is_installed ? 'v2-btn-outline' : 'v2-btn-primary'" :disabled="installingSkillId === skill.key" @click="handleInstall(skill, skill.is_installed)">{{ installingSkillId === skill.key ? '安装中…' : (skill.is_installed ? '重装' : '安装') }}</button>
            </div>
          </div>
        </div>
      </template>
    </template>

    <!-- 收藏 -->
    <template v-else>
      <div v-loading="loadingFavorites || loadingFavoritesOperation" class="sk-body">
        <V2Empty class="v2-card" v-if="favoriteList.length === 0" title="还没有收藏技能" description="收藏会保留来源仓库，方便随时一键安装">
          <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg></template>
        </V2Empty>
        <div v-else class="v2-cardgrid">
          <div v-for="favorite in favoriteList" :key="favorite.key" class="v2-card v2-ccard">
            <div class="v2-ccard-head">
              <div class="v2-ccard-icon"><SkillIcon /></div>
              <div class="v2-ccard-tt"><div class="v2-ccard-name">{{ favorite.name }}</div><div class="v2-ccard-sub mono">{{ favorite.repo.name }}</div></div>
              <div class="v2-ccard-acts">
                <el-tooltip :content="favorite.is_installed ? '重装' : '安装'" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ off: installingSkillId === favorite.key }" @click="handleInstallFavorite(favorite, favorite.is_installed)">
                    <svg v-if="favorite.is_installed" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg>
                    <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                  </button>
                </el-tooltip>
                <el-tooltip content="移除收藏" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act danger" @click="handleRemoveFavoriteById(favorite)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                </el-tooltip>
              </div>
            </div>
            <div v-if="favorite.description" class="sk-fdesc">{{ favorite.description }}</div>
          </div>
        </div>
      </div>
    </template>

    <V2Drawer v-model="showAddRepoDialog" title="添加技能仓库" @confirm="handleAddRepo">
      <div class="v2-field"><label class="v2-label">仓库地址 <span class="req">*</span></label><input v-model="repoForm.url" class="v2-input mono" placeholder="owner/repo 或本地路径"></div>
      <div class="v2-hint">支持 GitHub 仓库 URL 或本地目录路径。</div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2CliChips from '@/components/V2CliChips.vue'
import V2Empty from '@/components/V2Empty.vue'
import SkillIcon from '@/components/SkillIcon.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { skillsApi } from '@/api/skills'
import type { CliType, SkillRepo, DiscoverableSkill, InstalledSkill, SkillFavoriteItem } from '@/types/models'

const activeTab = ref<'skills' | 'repos' | 'favorites'>('skills')

const installedList = ref<InstalledSkill[]>([])
const loadingInstalled = ref(false)
const loadingInstalledOperation = ref(false)
const installingSkillId = ref<string | null>(null)

const repoList = ref<SkillRepo[]>([])
const loadingRepos = ref(false)
const showAddRepoDialog = ref(false)
const repoForm = ref({ url: '' })

const currentRepo = ref<SkillRepo | null>(null)
const repoSkillList = ref<DiscoverableSkill[]>([])
const loadingSkills = ref(false)
const loadingSkillsOperation = ref(false)
const skillSearchQuery = ref('')

const favoriteList = ref<SkillFavoriteItem[]>([])
const loadingFavorites = ref(false)
const loadingFavoritesOperation = ref(false)

const filteredSkillList = computed(() => {
  if (!skillSearchQuery.value) return repoSkillList.value
  const q = skillSearchQuery.value.toLowerCase()
  return repoSkillList.value.filter(s => s.name.toLowerCase().includes(q) || s.directory.toLowerCase().includes(q) || s.description?.toLowerCase().includes(q))
})

async function fetchInstalled() {
  loadingInstalled.value = true
  try {
    installedList.value = await skillsApi.getInstalled()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingInstalled.value = false
  }
}
async function fetchRepos() {
  loadingRepos.value = true
  try {
    repoList.value = await skillsApi.getRepos()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingRepos.value = false
  }
}
async function fetchFavorites() {
  loadingFavorites.value = true
  try {
    favoriteList.value = await skillsApi.getFavorites()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingFavorites.value = false
  }
}
async function refreshInstallationState() {
  await Promise.all([fetchInstalled(), fetchFavorites(), fetchRepos()])
}
async function refreshCurrentRepoSkillsIfNeeded(repoName?: string) {
  if (!currentRepo.value) return
  if (repoName && currentRepo.value.name !== repoName) return
  await fetchRepoSkills()
}
function handleRepoClick(repo: SkillRepo) {
  currentRepo.value = repo
  fetchRepoSkills()
}
async function handleReinstallRepo(repo: SkillRepo) {
  loadingRepos.value = true
  try {
    await skillsApi.reinstallRepo(repo.name)
    notify('重装成功')
    await fetchRepos()
    if (currentRepo.value?.name === repo.name) await fetchRepoSkills()
  } catch (error: any) {
    notify(getErrorMessage(error, '重装失败'), 'error')
  } finally {
    loadingRepos.value = false
  }
}
async function fetchRepoSkills() {
  if (!currentRepo.value) return
  loadingSkills.value = true
  try {
    repoSkillList.value = await skillsApi.discoverRepoSkills(currentRepo.value.name)
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingSkills.value = false
  }
}
async function refreshRepoSkills() {
  if (!currentRepo.value) return
  loadingSkills.value = true
  try {
    repoSkillList.value = await skillsApi.reinstallRepo(currentRepo.value.name)
    notify('已获取最新列表')
  } catch (error: any) {
    notify(getErrorMessage(error, '刷新失败'), 'error')
  } finally {
    loadingSkills.value = false
  }
}
function handleBackToRepos() {
  currentRepo.value = null
  repoSkillList.value = []
  skillSearchQuery.value = ''
}
async function handleCliToggle(skill: InstalledSkill, cliType: CliType, enabled: boolean) {
  loadingInstalledOperation.value = true
  try {
    await skillsApi.toggleCli(skill.id, cliType, enabled)
    if (skill.cli_flags) skill.cli_flags[cliType] = enabled
    notify('已更新')
  } catch (error: any) {
    notify(getErrorMessage(error, '更新失败'), 'error')
    await fetchInstalled()
  } finally {
    loadingInstalledOperation.value = false
  }
}
async function handleUninstall(skill: InstalledSkill) {
  try {
    await confirm(`确定卸载技能 "${skill.name}"?`, '确认卸载')
    loadingInstalledOperation.value = true
    await skillsApi.uninstall(skill.id)
    notify('已卸载')
    await Promise.all([refreshInstallationState(), refreshCurrentRepoSkillsIfNeeded(skill.repo?.name)])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, '卸载失败'), 'error')
  } finally {
    loadingInstalledOperation.value = false
  }
}
async function handleInstall(skill: DiscoverableSkill, reinstall = false) {
  try {
    if (reinstall) await confirm(`确定重装 "${skill.name}"? (将更新为最新版本)`, '确认重装')
    loadingSkillsOperation.value = true
    installingSkillId.value = skill.key
    await skillsApi.install(skill, reinstall)
    notify(reinstall ? '重装成功' : '安装成功')
    await Promise.all([refreshInstallationState(), refreshCurrentRepoSkillsIfNeeded(skill.repo.name)])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, '安装失败'), 'error')
  } finally {
    installingSkillId.value = null
    loadingSkillsOperation.value = false
  }
}
async function handleReinstallFromInstalled(skill: InstalledSkill) {
  if (!skill.can_favorite) {
    notify('缺少仓库信息，无法重装', 'error')
    return
  }
  try {
    await confirm(`确定重装技能 "${skill.name}"?`, '确认重装')
    loadingInstalledOperation.value = true
    installingSkillId.value = `installed-${skill.id}`
    await skillsApi.reinstall(skill.directory)
    notify('重装成功')
    await Promise.all([refreshInstallationState(), refreshCurrentRepoSkillsIfNeeded(skill.repo?.name)])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, '重装失败'), 'error')
  } finally {
    installingSkillId.value = null
    loadingInstalledOperation.value = false
  }
}
async function toggleInstalledFavorite(skill: InstalledSkill) {
  loadingInstalledOperation.value = true
  try {
    const isFavorited = await skillsApi.toggleInstalledFavorite(skill.id)
    notify(isFavorited ? '已收藏' : '已取消收藏')
    await fetchInstalled()
    await fetchFavorites()
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    loadingInstalledOperation.value = false
  }
}
async function handleInstallFavorite(favorite: SkillFavoriteItem, reinstall = false) {
  try {
    if (reinstall) await confirm(`确定重装 "${favorite.name}"? (将更新为最新版本)`, '确认重装')
    loadingFavoritesOperation.value = true
    installingSkillId.value = favorite.key
    if (reinstall) await skillsApi.reinstallFavorite(favorite.key)
    else await skillsApi.installFavorite(favorite.key)
    notify(reinstall ? '重装成功' : '安装成功')
    await Promise.all([refreshInstallationState(), refreshCurrentRepoSkillsIfNeeded(favorite.repo.name)])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, reinstall ? '重装失败' : '安装失败'), 'error')
  } finally {
    installingSkillId.value = null
    loadingFavoritesOperation.value = false
  }
}
async function handleRemoveFavoriteById(favorite: SkillFavoriteItem) {
  loadingFavoritesOperation.value = true
  try {
    await skillsApi.removeFavorite(favorite.key)
    await Promise.all([fetchFavorites(), fetchInstalled()])
    notify('已移除')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    loadingFavoritesOperation.value = false
  }
}
async function copyDescription(text: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    notify('描述已复制')
  } catch {
    notify('复制失败', 'error')
  }
}
async function handleAddRepo() {
  if (!repoForm.value.url.trim()) {
    notify('请输入仓库地址', 'error')
    return
  }
  const payload = { url: repoForm.value.url.trim() }
  showAddRepoDialog.value = false
  repoForm.value = { url: '' }
  loadingRepos.value = true
  try {
    await skillsApi.addRepo(payload)
    notify('添加成功')
    await fetchRepos()
  } catch (error: any) {
    notify(getErrorMessage(error, '添加失败'), 'error')
    loadingRepos.value = false
  }
}
async function handleRemoveRepo(repo: SkillRepo) {
  try {
    await confirm(`确定删除仓库 "${repo.name}" 并卸载该仓库下所有已安装技能？`, '确认删除')
    loadingRepos.value = true
    await skillsApi.removeRepo(repo.name)
    notify('已删除')
    await refreshInstallationState()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, '删除失败'), 'error')
    loadingRepos.value = false
  }
}

onMounted(() => {
  fetchInstalled()
  fetchRepos()
  fetchFavorites()
})
</script>

<style scoped>
.sk-page { height: 100%; display: flex; flex-direction: column; min-height: 0; margin-top: -16px; }
.sk-tabs { margin-bottom: 16px; flex-shrink: 0; }
.sk-toolbar { display: flex; justify-content: flex-end; margin-bottom: 14px; flex-shrink: 0; }
.sk-body { flex: 1; overflow-y: auto; min-height: 0; }
.sk-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 9px; padding: 60px; color: var(--v2-text-3); }
.sk-empty-t { font-size: var(--v2-fs-base); font-weight: 500; color: var(--v2-text-2); }
.sk-inline { margin-left: 8px; }
.sk-star-on { color: var(--v2-warning); }

.sk-repocard { cursor: pointer; transition: border-color 0.15s; }
.sk-repocard:hover { border-color: var(--v2-surface-3); }

.sk-repohead { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 14px; flex-shrink: 0; }
.sk-repohead-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
.sk-repohead-t { font-size: var(--v2-fs-md); font-weight: 600; color: var(--v2-text); }
.sk-repohead-r { display: flex; align-items: center; gap: 8px; }
.sk-search { width: 220px; }

.sk-dlist { padding: 4px 0; }
.sk-drow { display: flex; align-items: center; gap: 16px; padding: 13px 16px; border-bottom: 1px solid var(--v2-surface-2); }
.sk-drow:last-child { border-bottom: none; }
.sk-drow-icon { width: 32px; height: 32px; border-radius: 6px; background: var(--v2-surface-2); color: var(--v2-text-2); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.sk-dinfo { flex: 1; min-width: 0; }
.sk-dname { font-size: var(--v2-fs-sm); font-weight: 600; color: var(--v2-text); }
.sk-ddesc { font-size: var(--v2-fs-sm); color: var(--v2-text-3); margin-top: 3px; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sk-install { flex-shrink: 0; }
.sk-fdesc { font-size: var(--v2-fs-sm); color: var(--v2-text-3); line-height: 1.5; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.v2-ccard-name { display: flex; align-items: center; }
</style>
