<template>
  <div class="skills-page">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-zap" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
        </symbol>
        <symbol id="icon-store" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 9 12 3l9 6v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><polyline points="9 22 9 12 15 12 15 22"/>
        </symbol>
        <symbol id="icon-star" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
        </symbol>
        <symbol id="icon-plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
        <symbol id="icon-refresh" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/>
        </symbol>
        <symbol id="icon-back" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>
        </symbol>
        <symbol id="icon-search" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
        </symbol>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-external" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
        </symbol>
      </defs>
    </svg>

    <!-- Top Tabs -->
    <div class="top-tabs">
      <div 
        class="tab-item" 
        :class="{ active: activeTab === 'skills' }" 
        @click="activeTab = 'skills'"
      >技能</div>
      <div 
        class="tab-item" 
        :class="{ active: activeTab === 'repos' }" 
        @click="activeTab = 'repos'"
      >仓库</div>
      <div 
        class="tab-item" 
        :class="{ active: activeTab === 'favorites' }" 
        @click="activeTab = 'favorites'"
      >收藏</div>
    </div>

    <!-- Main Content Area -->
    <div class="view-content-wrapper">

      <!-- TAB: INSTALLED -->
      <div v-if="activeTab === 'skills'" class="tab-pane">
        <div class="skills-view">
          <div v-loading="loadingInstalled || loadingInstalledOperation" class="list-container">
          <template v-if="installedList.length === 0">
            <div class="empty-state">
              <svg width="64" height="64" color="var(--color-border)"><use href="#icon-zap"/></svg>
              <p>暂无已安装技能</p>
            </div>
          </template>
          <div v-else class="scroll-area">
            <div class="skill-grid">
              <InstalledSkillCard
                v-for="skill in installedList"
                :key="skill.id"
                :skill="skill"
                :reinstalling="installingSkillId === `installed-${skill.id}`"
                @favorite="toggleInstalledFavorite"
                @reinstall="handleReinstallFromInstalled"
                @uninstall="handleUninstall"
                @cli-toggle="handleCliToggle"
              />
            </div>
          </div>
        </div>
        </div>
      </div>

      <!-- TAB: AVAILABLE -->
      <div v-else-if="activeTab === 'repos'" class="tab-pane">
        
        <!-- Repo List View -->
        <div v-if="!currentRepo" class="repo-list-view">
          <div class="page-header">
            <p class="page-subtitle">从 GitHub 仓库发现并安装 Skill 扩展</p>
            <button class="action-icon primary" @click="showAddRepoDialog = true" title="添加仓库">
              <svg width="20" height="20"><use href="#icon-plus"/></svg>
            </button>
          </div>

          <div v-loading="loadingRepos" class="list-container">
            <template v-if="repoList.length === 0">
              <div class="empty-state">
                <svg width="64" height="64" color="var(--color-border)"><use href="#icon-store"/></svg>
                <p>暂无技能仓库</p>
              </div>
            </template>
            <div v-else class="scroll-area">
              <div class="repo-grid">
                <SkillRepoCard
                  v-for="repo in repoList"
                  :key="repo.name"
                  :repo="repo"
                  :loading="loadingRepos"
                  @open="handleRepoClick"
                  @reinstall="handleReinstallRepo"
                  @remove="handleRemoveRepo"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Repo Skills List View -->
        <div v-else class="repo-skills-view">
          <div class="page-header">
            <div style="display: flex; align-items: center; gap: 16px;">
              <button class="action-icon" @click="handleBackToRepos" title="返回">
                <svg width="18" height="18"><use href="#icon-back"/></svg>
              </button>
              <div>
                <h2 class="page-title text-20">{{ currentRepo.name }}</h2>
                <div class="mono text-14 text-muted">{{ currentRepo.source }}</div>
              </div>
            </div>
            <div style="display: flex; gap: 12px; align-items: center;">
              <div class="search-box" style="width: 240px; position: relative;">
                <svg class="search-icon" width="16" height="16" style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--color-text-weak); pointer-events: none; z-index: 1;"><use href="#icon-search"/></svg>
                <input type="text" v-model="skillSearchQuery" class="b-input search-input" placeholder="搜索...">
              </div>
              <button class="action-icon" :disabled="loadingSkills" @click="refreshRepoSkills" title="刷新列表">
                <svg width="18" height="18"><use href="#icon-refresh"/></svg>
              </button>
            </div>
          </div>

          <div v-loading="loadingSkills || loadingSkillsOperation" class="list-container">
            <template v-if="filteredSkillList.length === 0">
              <div class="empty-state">
                <svg width="64" height="64" color="var(--color-border)"><use href="#icon-zap"/></svg>
                <p>{{ skillSearchQuery ? '无匹配结果' : '该仓库暂无 Skills' }}</p>
              </div>
            </template>
            <div v-else class="scroll-area">
              <div class="discover-list">
                <DiscoverableSkillItem
                  v-for="skill in filteredSkillList"
                  :key="skill.key"
                  :skill="skill"
                  :installing="installingSkillId === skill.key"
                  @install="handleInstall"
                  @copy-description="copyDescription"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-else class="tab-pane">
        <div class="favorites-view">
          <div class="page-header">
            <p class="page-subtitle">收藏的技能会保留仓库信息，方便后续快速安装</p>
          </div>

          <div v-loading="loadingFavorites || loadingFavoritesOperation" class="list-container">
          <div v-if="favoriteList.length === 0" class="empty-state">
            <svg width="64" height="64" color="var(--color-border)"><use href="#icon-star"/></svg>
            <p>暂无收藏技能</p>
          </div>
          <div v-else class="scroll-area">
            <div class="favorite-grid">
              <SkillFavoriteCard
                v-for="favorite in favoriteList"
                :key="favorite.key"
                :favorite="favorite"
                :installing="installingSkillId === favorite.key"
                @remove="handleRemoveFavoriteById"
                @install="handleInstallFavorite"
              />
            </div>
          </div>
        </div>
        </div>
      </div>
    </div>

    <AddSkillRepoModal
      v-model="showAddRepoDialog"
      v-model:url="repoForm.url"
      @confirm="handleAddRepo"
    />
  </div>

    </template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import InstalledSkillCard from './components/InstalledSkillCard.vue'
import SkillRepoCard from './components/SkillRepoCard.vue'
import DiscoverableSkillItem from './components/DiscoverableSkillItem.vue'
import SkillFavoriteCard from './components/SkillFavoriteCard.vue'
import AddSkillRepoModal from './components/AddSkillRepoModal.vue'
import { skillsApi } from '@/api/skills'
import type { CliType, SkillRepo, DiscoverableSkill, InstalledSkill, SkillFavoriteItem } from '@/types/models'

const activeTab = ref<'skills' | 'repos' | 'favorites'>('skills')

// Installed Skills
const installedList = ref<InstalledSkill[]>([])
const loadingInstalled = ref(false)
const loadingInstalledOperation = ref(false)
const installingSkillId = ref<string | null>(null)

// Repos
const repoList = ref<SkillRepo[]>([])
const loadingRepos = ref(false)
const showAddRepoDialog = ref(false)
const repoForm = ref({ url: '' })

// Discovery
const currentRepo = ref<SkillRepo | null>(null)
const repoSkillList = ref<DiscoverableSkill[]>([])
const loadingSkills = ref(false)
const loadingSkillsOperation = ref(false)
const skillSearchQuery = ref('')

// Favorites
const favoriteList = ref<SkillFavoriteItem[]>([])
const loadingFavorites = ref(false)
const loadingFavoritesOperation = ref(false)

const filteredSkillList = computed(() => {
  if (!skillSearchQuery.value) return repoSkillList.value
  const query = skillSearchQuery.value.toLowerCase()
  return repoSkillList.value.filter(s =>
    s.name.toLowerCase().includes(query) ||
    s.directory.toLowerCase().includes(query) ||
    s.description?.toLowerCase().includes(query)
  )
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
    if (currentRepo.value?.name === repo.name) {
      await fetchRepoSkills()
    }
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
    if (skill.cli_flags) {
      skill.cli_flags[cliType] = enabled
    }
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
    await Promise.all([
      refreshInstallationState(),
      refreshCurrentRepoSkillsIfNeeded(skill.repo?.name)
    ])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(getErrorMessage(error, '卸载失败'), 'error')
    }
  } finally {
    loadingInstalledOperation.value = false
  }
}

async function handleInstall(skill: DiscoverableSkill, reinstall: boolean = false) {
  try {
    if (reinstall) {
      await confirm(`确定重装 "${skill.name}"? (将更新为最新版本)`, '确认重装')
    }
    loadingSkillsOperation.value = true
    installingSkillId.value = skill.key
    await skillsApi.install(skill, reinstall)
    notify(reinstall ? '重装成功' : '安装成功')
    await Promise.all([
      refreshInstallationState(),
      refreshCurrentRepoSkillsIfNeeded(skill.repo.name)
    ])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(getErrorMessage(error, '安装失败'), 'error')
    }
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
    await Promise.all([
      refreshInstallationState(),
      refreshCurrentRepoSkillsIfNeeded(skill.repo?.name)
    ])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(getErrorMessage(error, '重装失败'), 'error')
    }
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

async function handleInstallFavorite(favorite: SkillFavoriteItem, reinstall: boolean = false) {
  try {
    if (reinstall) {
      await confirm(`确定重装 "${favorite.name}"? (将更新为最新版本)`, '确认重装')
    }
    loadingFavoritesOperation.value = true
    installingSkillId.value = favorite.key
    if (reinstall) {
      await skillsApi.reinstallFavorite(favorite.key)
    } else {
      await skillsApi.installFavorite(favorite.key)
    }
    notify(reinstall ? '重装成功' : '安装成功')
    await Promise.all([
      refreshInstallationState(),
      refreshCurrentRepoSkillsIfNeeded(favorite.repo.name)
    ])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(getErrorMessage(error, reinstall ? '重装失败' : '安装失败'), 'error')
    }
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
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(getErrorMessage(error, '删除失败'), 'error')
      loadingRepos.value = false
    } else {
      loadingRepos.value = false
    }
  }
}

onMounted(() => {
  fetchInstalled()
  fetchRepos()
  fetchFavorites()
})
</script>

<style scoped>
.skills-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }

.view-content-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tab-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.repo-list-view, .repo-skills-view, .favorites-view, .skills-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* Header */
.page-title.text-20 { margin: 0; }

.skill-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 24px; }

.repo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }

.discover-list { background: var(--color-bg); border-radius: 16px; overflow: hidden; border: 1px solid var(--color-bg-subtle); }

.favorite-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }

.search-box { position: relative; }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--color-text-weak); }


</style>
