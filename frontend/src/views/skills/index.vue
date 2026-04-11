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
              <div v-for="skill in installedList" :key="skill.id" class="skill-card">
                <div class="card-top">
                  <div class="skill-icon">
                    <svg width="24" height="24"><use href="#icon-zap"/></svg>
                  </div>
                  <div class="skill-info">
                    <div style="display: flex; align-items: center; gap: 8px; min-width: 0;">
                      <h3 class="skill-name">{{ skill.name }}</h3>
                      <div v-if="!skill.exists_on_disk" class="tag tag-red" style="flex-shrink: 0;">缺失文件</div>
                    </div>
                    <div 
                      class="skill-market" 
                      v-if="skill.market_display" 
                      :title="skill.market_display"
                    >
                      {{ skill.repo?.name ? `@${skill.repo.name}` : skill.market_display }}
                    </div>
                    <div class="skill-source mono" v-else>本地安装</div>
                  </div>
                  <div class="card-actions">
                    <button
                      class="action-icon star"
                      :class="{ 'star-active': skill.is_favorited }"
                      :title="skill.is_favorited ? '取消收藏' : '收藏技能'"
                      :disabled="!skill.can_favorite"
                      @click="toggleInstalledFavorite(skill)"
                    >
                      <svg
                        width="18"
                        height="18"
                        :style="skill.is_favorited ? 'fill: var(--color-warning);' : ''"
                      ><use href="#icon-star"/></svg>
                    </button>
                    <button class="action-icon" title="重装/更新" :disabled="installingSkillId === `installed-${skill.id}`" @click="handleReinstallFromInstalled(skill)">
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button class="action-icon delete" title="卸载" @click="handleUninstall(skill)">
                      <svg width="18" height="18"><use href="#icon-trash"/></svg>
                    </button>
                  </div>
                </div>

                <div class="cli-toggles">
                  <div class="toggle-item">
                    <span class="toggle-label">Claude Code</span>
                    <el-switch
                      size="small"
                      :model-value="skill.cli_flags?.claude_code"
                      @change="handleCliToggle(skill, 'claude_code', $event as boolean)"
                    />
                  </div>
                  <div class="toggle-item">
                    <span class="toggle-label">Codex</span>
                    <el-switch
                      size="small"
                      :model-value="skill.cli_flags?.codex"
                      @change="handleCliToggle(skill, 'codex', $event as boolean)"
                    />
                  </div>
                  <div class="toggle-item">
                    <span class="toggle-label">Gemini</span>
                    <el-switch
                      size="small"
                      :model-value="skill.cli_flags?.gemini"
                      @change="handleCliToggle(skill, 'gemini', $event as boolean)"
                    />
                  </div>
                </div>
              </div>
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
            <button class="action-icon add-btn" @click="showAddRepoDialog = true" title="添加仓库">
              <svg width="20" height="20"><use href="#icon-plus"/></svg>
            </button>
          </div>

          <div v-loading="loadingRepos" class="list-container">
            <template v-if="repoList.length === 0">
              <div class="empty-state">
                <svg width="64" height="64" color="var(--color-border)"><use href="#icon-store"/></svg>
                <p>暂未添加仓库</p>
              </div>
            </template>
            <div v-else class="scroll-area">
              <div class="repo-grid">
                <div v-for="repo in repoList" :key="repo.name" class="repo-card" @click="handleRepoClick(repo)">
                  <div class="repo-icon-box">
                    <svg width="24" height="24"><use href="#icon-store"/></svg>
                  </div>
                  <div class="repo-info-main">
                    <div class="repo-name-title">{{ repo.name }}</div>
                    <div class="repo-source-subtitle mono">{{ repo.source }}</div>
                  </div>
                  <div class="repo-actions-overlay" @click.stop>
                    <button class="action-icon" title="重装仓库" :disabled="loadingRepos" @click="handleReinstallRepo(repo)">
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button class="action-icon delete" title="删除" @click="handleRemoveRepo(repo)">
                      <svg width="18" height="18"><use href="#icon-trash"/></svg>
                    </button>
                  </div>
                </div>
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
                <input type="text" v-model="skillSearchQuery" class="c-input" placeholder="搜索..." style="height: 38px; padding: 0 12px 0 36px; margin: 0;">
              </div>
              <button class="action-icon" :disabled="loadingSkills" @click="refreshRepoSkills" title="刷新列表">
                <svg width="18" height="18"><use href="#icon-refresh"/></svg>
              </button>
            </div>
          </div>

          <div v-loading="loadingSkills || loadingSkillsOperation" class="list-container">
            <template v-if="filteredSkillList.length === 0">
              <el-empty :description="skillSearchQuery ? '无匹配结果' : '该仓库暂无 Skills'" />
            </template>
            <div v-else class="scroll-area">
              <div class="discover-list">
                <div v-for="skill in filteredSkillList" :key="skill.key" class="discover-item">
                  <div class="discover-info">
                    <div class="discover-name-row">
                      <span class="discover-name">{{ skill.name }}</span>
                      <span class="mono text-12 text-muted">{{ skill.directory }}</span>
                    </div>
                    <el-tooltip
                      v-if="skill.description"
                      effect="light"
                      placement="top"
                      :enterable="true"
                      :show-after="200"
                    >
                      <template #content>
                        <div class="text-14" style="max-width: 350px; line-height: 1.6; word-break: break-word; user-select: text; color: var(--color-text-dark);">
                          {{ skill.description }}
                        </div>
                      </template>
                      <div class="discover-desc" @click="copyDescription(skill.description)">
                        {{ skill.description }}
                      </div>
                    </el-tooltip>
                    <div v-else class="discover-desc">
                      暂无描述
                    </div>
                  </div>
                  <div class="discover-actions">
                    <button
                      v-if="skill.is_installed"
                      class="action-icon installed"
                      title="重装"
                      :disabled="installingSkillId === skill.key"
                      @click="handleInstall(skill, true)"
                    >
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button
                      v-else
                      class="action-icon install"
                      title="安装技能"
                      :disabled="installingSkillId === skill.key"
                      @click="handleInstall(skill, false)"
                    >
                      <svg width="18" height="18"><use href="#icon-plus"/></svg>
                    </button>
                  </div>
                </div>
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
              <div v-for="favorite in favoriteList" :key="favorite.key" class="fav-card">
                <div class="fav-main">
                  <div class="fav-info">
                    <div class="fav-name">{{ favorite.name }}</div>
                    <div class="fav-market" :title="favorite.repo.source">
                      来自仓库: {{ favorite.repo.name || favorite.repo.source }}
                    </div>
                  </div>
                  <div class="fav-actions">
                    <button
                      class="action-icon star-active"
                      title="取消收藏"
                      @click="handleRemoveFavoriteById(favorite)"
                    >
                      <svg width="18" height="18" style="fill: var(--color-warning);"><use href="#icon-star"/></svg>
                    </button>
                    <button
                      v-if="favorite.is_installed"
                      class="action-icon installed"
                      title="重装"
                      :disabled="installingSkillId === favorite.key"
                      @click="handleInstallFavorite(favorite, true)"
                    >
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button
                      v-else
                      class="action-icon install"
                      title="安装技能"
                      :disabled="installingSkillId === favorite.key"
                      @click="handleInstallFavorite(favorite, false)"
                    >
                      <svg width="18" height="18"><use href="#icon-plus"/></svg>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        </div>
      </div>
    </div>

    <!-- Modals -->
    <AppModal v-model="showAddRepoDialog" title="添加 Skill 仓库" width="500px" @confirm="handleAddRepo">
        <div class="form-group">
            <label class="c-label">仓库地址 <span class="required">*</span></label>
            <input
              type="text"
              v-model="repoForm.url"
              class="c-input"
              placeholder="仓库地址 、 owner/repo 、 本地目录"
            >
          </div>
    </AppModal>

    </template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import AppModal from '@/components/AppModal.vue'
import { skillsApi } from '@/api/skills'
import type { SkillRepo, DiscoverableSkill, InstalledSkill, SkillFavoriteItem } from '@/types/models'

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

async function handleCliToggle(skill: InstalledSkill, cliType: string, enabled: boolean) {
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
.tab-item { padding-bottom: 12px; color: var(--color-text-weak); font-weight: var(--fw-400); font-size: var(--fs-14); cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: var(--color-text-secondary); }
.tab-item.active { color: var(--color-primary); font-weight: var(--fw-600); border-bottom: 2px solid var(--color-primary); }

.view-content-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: 0 40px;
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

.list-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

/* Header */
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; flex-shrink: 0; }
.page-subtitle { font-size: var(--fs-14); color: var(--color-text-muted); margin: 0; }
.page-title.text-20 { margin: 0; }

/* Grid & Cards (Installed) */
.skill-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 24px; }
.skill-card {
  background: var(--color-bg); border-radius: 16px; border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent); padding: 24px;
  box-shadow: 0 4px 12px var(--color-shadow); display: flex; flex-direction: column; gap: 20px;
}

.card-top { display: flex; gap: 16px; align-items: flex-start; }
.skill-icon {
  width: 48px; height: 48px; border-radius: 12px; background: var(--color-violet-light); color: var(--color-violet);
  display: flex; align-items: center; justify-content: center; flex-shrink: 0;
}
.skill-info { flex: 1; min-width: 0; }
.skill-name {
  font-size: var(--fs-16); font-weight: var(--fw-700); color: var(--color-text); margin: 0 0 4px 0;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  overflow: hidden; text-overflow: ellipsis;
}.skill-market {
  font-size: var(--fs-12); color: var(--color-text-muted); font-weight: var(--fw-400);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.skill-source { font-size: var(--fs-12); color: var(--color-text-weak); }

.card-actions { display: flex; gap: 4px; flex-shrink: 0; }
.action-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
  outline: none;
  background: transparent;
  border: none;
}
.action-icon:hover { background: var(--color-bg-subtle); color: var(--color-text); }
.action-icon.delete:hover { background: var(--color-danger-light); color: var(--color-danger); }
.action-icon.star:disabled { cursor: not-allowed; opacity: 0.45; }
.action-icon.star:disabled:hover { background: transparent; color: var(--color-text-muted); }
.action-icon.star-active { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 10%, transparent); }
.action-icon.installed { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 10%, transparent); }
.action-icon.install { color: var(--color-primary); background: color-mix(in srgb, var(--color-primary) 15%, transparent); }

/* CLI Toggles */
.cli-toggles { display: flex; flex-direction: column; gap: 12px; background: var(--color-bg-page); padding: 16px; border-radius: 12px; }
.toggle-item { display: flex; justify-content: space-between; align-items: center; }
.toggle-label { font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-secondary); }

/* Repo Grid (Available) */
.repo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }
.repo-card {
  background: var(--color-bg); border-radius: 16px; border: 1px solid var(--color-bg-subtle); padding: 20px;
  cursor: pointer; position: relative; transition: all 0.2s; display: flex; align-items: center; gap: 16px;
}
.repo-card:hover { border-color: var(--color-primary); background: var(--color-bg-page); }

.repo-icon-box {
  width: 40px; height: 40px; border-radius: 10px; background: var(--color-bg-subtle); color: var(--color-text-muted);
  display: flex; align-items: center; justify-content: center;
}
.repo-info-main { flex: 1; min-width: 0; }
.repo-name-title { font-weight: var(--fw-700); font-size: var(--fs-14); color: var(--color-text); margin-bottom: 4px; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.repo-source-subtitle { font-size: var(--fs-12); color: var(--color-text-weak); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.repo-actions-overlay { display: flex; gap: 4px; flex-shrink: 0; }

/* Discover List */
.discover-list { background: var(--color-bg); border-radius: 16px; overflow: hidden; border: 1px solid var(--color-bg-subtle); }
.discover-item {
  display: flex; justify-content: space-between; align-items: center; padding: 20px 24px;
  border-bottom: 1px solid var(--color-bg-subtle); transition: background 0.2s;
}
.discover-item:last-child { border-bottom: none; }
.discover-item:hover { background: var(--color-bg-page); }
.discover-info { flex: 1; min-width: 0; padding-right: 40px; }
.discover-name-row { margin-bottom: 6px; display: flex; align-items: center; gap: 8px; }
.discover-name { font-weight: var(--fw-700); font-size: var(--fs-14); color: var(--color-text); }
.discover-desc {
  font-size: var(--fs-14); color: var(--color-text-muted); line-height: 1.5; cursor: pointer;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}
.discover-actions { flex-shrink: 0; display: flex; gap: 4px; }

/* Favorites */
.favorite-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }
.fav-card { background: var(--color-bg); border-radius: 16px; border: 1px solid var(--color-bg-subtle); padding: 20px; }
.fav-main { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
.fav-info { min-width: 0; flex: 1; }
.fav-name {
  font-weight: var(--fw-700); font-size: var(--fs-16); color: var(--color-text); margin-bottom: 4px;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  overflow: hidden; text-overflow: ellipsis;
}.fav-market {
  font-size: var(--fs-12); color: var(--color-text-weak);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.fav-actions { flex-shrink: 0; display: flex; gap: 4px; }

/* Shared styles */
.tag { padding: 2px 8px; border-radius: 4px; font-size: var(--fs-12); font-weight: var(--fw-700); text-transform: uppercase; }
.tag-red { background: var(--color-error-light); color: var(--color-error); }

.search-box { position: relative; }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--color-text-weak); }
.c-input {
  width: 100%; padding: 10px 14px; background: var(--color-bg); border: 1px solid var(--color-border);
  border-radius: 8px; font-size: var(--fs-14); color: var(--color-text); outline: none; transition: all 0.2s;
}
.c-input:focus { border-color: var(--color-primary); }

.b-button {
  background: var(--color-primary); color: var(--color-bg); border: none; padding: 8px 16px; border-radius: 8px;
  font-size: var(--fs-14); font-weight: var(--fw-400); cursor: pointer; display: flex; align-items: center;
  transition: all 0.2s; white-space: nowrap;
}
.b-button:hover { background: var(--color-primary-hover); }
.b-button:disabled { background: var(--color-text-weak); cursor: not-allowed; }

.b-button-outline {
  background: var(--color-bg); color: var(--color-text); border: 1px solid var(--color-border); padding: 8px 16px; border-radius: 8px;
  font-size: var(--fs-14); font-weight: var(--fw-400); cursor: pointer; transition: all 0.2s; display: flex; align-items: center;
}
.b-button-outline:hover { background: var(--color-bg-page); border-color: var(--color-border-hover); }

.empty-state { padding: 80px 40px; text-align: center; color: var(--color-text-weak); background: var(--color-bg); border-radius: 24px; border: 2px dashed var(--color-border); }
.empty-state p { margin-top: 16px; font-size: var(--fs-14); }

.form-group { margin-bottom: 24px; }
.c-label { display: block; font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-secondary); margin-bottom: 12px; }
.required { color: var(--color-error); }

.action-icon.add-btn {
  width: 36px;
  height: 36px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 15%, transparent);
}
.action-icon.add-btn:hover {
  background: color-mix(in srgb, var(--color-primary) 25%, transparent);
  color: var(--color-primary);
}
</style>
