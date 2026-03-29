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
          <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/><path d="m15 5 4 4"/>
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
        :class="{ active: activeTab === 'installed' }" 
        @click="activeTab = 'installed'"
      >已安装</div>
      <div 
        class="tab-item" 
        :class="{ active: activeTab === 'available' }" 
        @click="activeTab = 'available'"
      >仓库</div>
    </div>

    <!-- Main Content Area -->
    <div class="view-content-wrapper">
      
      <!-- TAB: INSTALLED -->
      <div v-if="activeTab === 'installed'" class="tab-pane">
        <div v-loading="loadingInstalled">
          <template v-if="installedList.length === 0">
            <div class="empty-state">
              <svg width="64" height="64" color="#e2e8f0"><use href="#icon-zap"/></svg>
              <p>暂无已安装技能</p>
            </div>
          </template>
          <div v-else class="skill-grid">
            <div v-for="skill in installedList" :key="skill.id" class="skill-card">
              <div class="card-top">
                <div class="skill-icon">
                  <svg width="24" height="24"><use href="#icon-zap"/></svg>
                </div>
                <div class="skill-info">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <h3 class="skill-name">{{ skill.name }}</h3>
                    <div v-if="!skill.exists_on_disk" class="tag tag-red">缺失文件</div>
                  </div>
                  <div class="skill-market" v-if="skill.repo_owner">@{{ skill.repo_owner }}/{{ skill.repo_name }}</div>
                  <div class="skill-source mono" v-else>本地安装</div>
                </div>
                <div class="card-actions">
                  <template v-if="skill.exists_on_disk">
                    <button class="action-btn" title="重装/更新" :disabled="installingSkillId === `installed-${skill.id}`" @click="handleReinstallFromInstalled(skill)">
                      <svg width="16" height="16"><use href="#icon-refresh"/></svg>
                    </button>
                    <button class="action-btn delete" title="卸载" @click="handleUninstall(skill)">
                      <svg width="16" height="16"><use href="#icon-trash"/></svg>
                    </button>
                  </template>
                  <template v-else>
                    <button class="b-button-outline" style="font-size: 11px; padding: 4px 8px;" :disabled="installingSkillId === `installed-${skill.id}`" @click="handleInstallFromInstalled(skill)">
                      安装技能
                    </button>
                  </template>
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

      <!-- TAB: AVAILABLE -->
      <div v-else class="tab-pane">
        
        <!-- Repo List View -->
        <div v-if="!currentRepo">
          <div class="page-header">
            <p class="page-subtitle">从 GitHub 仓库发现并安装 Skill 扩展</p>
            <button class="b-button" style="padding: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center;" @click="showAddRepoDialog = true" title="添加仓库">
              <svg width="20" height="20"><use href="#icon-plus"/></svg>
            </button>
          </div>

          <div v-loading="loadingRepos">
            <template v-if="repoList.length === 0">
              <div class="empty-state">
                <svg width="64" height="64" color="#e2e8f0"><use href="#icon-store"/></svg>
                <p>暂未添加仓库</p>
              </div>
            </template>
            <div v-else class="repo-grid">
              <div v-for="repo in repoList" :key="repo.name" class="repo-card" @click="handleRepoClick(repo)">
                <div class="repo-icon-box">
                  <svg width="24" height="24"><use href="#icon-store"/></svg>
                </div>
                <div class="repo-info-main">
                  <div class="repo-name-title">{{ repo.name }}</div>
                  <div class="repo-owner-subtitle mono">{{ repo.source }}</div>
                </div>
                <div class="repo-actions-overlay" @click.stop>
                   <button class="action-btn" title="编辑" @click="handleEditRepo(repo)">
                     <svg width="16" height="16"><use href="#icon-edit"/></svg>
                   </button>
                   <button class="action-btn delete" title="删除" @click="handleRemoveRepo(repo)">
                     <svg width="16" height="16"><use href="#icon-trash"/></svg>
                   </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Repo Skills List View -->
        <div v-else>
          <div class="page-header">
            <div style="display: flex; align-items: center; gap: 16px;">
              <button class="b-button-outline" style="border: none; background: transparent; box-shadow: none; padding: 6px; color: #64748b;" @click="handleBackToRepos">
                <svg width="20" height="20"><use href="#icon-back"/></svg>
              </button>
              <div>
                <h2 class="page-title" style="font-size: 20px; margin-bottom: 2px;">{{ currentRepo.name }}</h2>
                <div style="font-size: 13px; color: #94a3b8;">源: {{ currentRepo.source }} <span v-if="currentRepo.branch">| 分支: {{ currentRepo.branch }}</span></div>
              </div>
            </div>
            <div style="display: flex; gap: 12px; align-items: center;">
              <div class="search-box" style="width: 240px; position: relative;">
                <svg class="search-icon" width="16" height="16" style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; pointer-events: none; z-index: 1;"><use href="#icon-search"/></svg>
                <input type="text" v-model="skillSearchQuery" class="c-input" placeholder="搜索..." style="height: 38px; padding: 0 12px 0 36px; margin: 0; box-shadow: none;">
              </div>
              <button class="b-button-outline" style="padding: 0; height: 38px; width: 38px; display: flex; align-items: center; justify-content: center; margin: 0; box-shadow: none; flex-shrink: 0;" :disabled="loadingSkills" @click="refreshRepoSkills" title="刷新列表">
                <svg width="20" height="20"><use href="#icon-refresh"/></svg>
              </button>
            </div>
          </div>

          <div v-loading="loadingSkills">
            <template v-if="filteredSkillList.length === 0">
              <el-empty :description="skillSearchQuery ? '无匹配结果' : '该仓库暂无 Skills'" />
            </template>
            <div v-else class="discover-list">
              <div v-for="skill in filteredSkillList" :key="skill.key" class="discover-item">
                <div class="discover-info">
                  <div class="discover-name-row">
                    <span class="discover-name">{{ skill.name }}</span>
                    <span class="mono" style="font-size: 11px; color: #94a3b8;">/{{ skill.directory }}</span>
                  </div>
                  <el-tooltip
                    v-if="skill.description"
                    effect="light"
                    placement="top"
                    :enterable="true"
                    :show-after="200"
                  >
                    <template #content>
                      <div style="max-width: 350px; line-height: 1.6; font-size: 13px; word-break: break-word; user-select: text; color: #334155;">
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
                    v-if="isInstalled(skill.directory)"
                    class="action-btn"
                    style="color: #f59e0b; background: rgba(245, 158, 11, 0.1);"
                    title="重装"
                    :disabled="installingSkillId === skill.key" 
                    @click="handleInstall(skill, true)"
                  >
                    <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                  </button>
                  <button 
                    v-else
                    class="action-btn"
                    style="color: #0ea5e9; background: rgba(14, 165, 233, 0.1);"
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

    <!-- Modals -->
    <AppModal v-model="showAddRepoDialog" title="添加 Skill 仓库" width="500px" :show-footer="false">
        <div class="form-group">
            <label class="c-label">仓库地址 <span class="required">*</span></label>
            <input
              type="text"
              v-model="repoForm.url"
              class="c-input"
              placeholder="例如: owner/repo 或 GitHub 链接"
            >
          </div>
          <div class="form-group">
            <label class="c-label">分支</label>
            <input
              type="text"
              v-model="repoForm.branch"
              class="c-input"
              placeholder="默认 main"
            >
          </div>

      <template #footer>
        <button class="b-button-outline" @click="showAddRepoDialog = false" :disabled="savingRepo">取消</button>
        <button class="b-button" @click="handleAddRepo" :disabled="savingRepo">确认添加</button>
      </template>
    </AppModal>

    <AppModal v-model="showEditRepoDialog" title="编辑仓库" width="500px" :show-footer="false">
        <div class="form-group">
            <label class="c-label">仓库地址 <span class="required">*</span></label>
            <input
              type="text"
              v-model="editRepoForm.url"
              class="c-input"
              placeholder="例如: owner/repo 或 GitHub 链接"
            >
          </div>
          <div class="form-group">
            <label class="c-label">分支</label>
            <input
              type="text"
              v-model="editRepoForm.branch"
              class="c-input"
              placeholder="默认 main"
            >
          </div>

      <template #footer>
        <button class="b-button-outline" @click="showEditRepoDialog = false" :disabled="savingRepo">取消</button>
        <button class="b-button" @click="handleUpdateRepo" :disabled="savingRepo">保存更改</button>
      </template>
    </AppModal>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import AppModal from '@/components/AppModal.vue'
import { skillsApi } from '@/api/skills'
import type { SkillRepo, DiscoverableSkill, InstalledSkill } from '@/types/models'

const activeTab = ref('installed')

// Installed Skills
const installedList = ref<InstalledSkill[]>([])
const loadingInstalled = ref(false)
const installingSkillId = ref<string | null>(null)

// Repos
const repoList = ref<SkillRepo[]>([])
const loadingRepos = ref(false)
const showAddRepoDialog = ref(false)
const repoForm = ref({ url: '', branch: '' })
const showEditRepoDialog = ref(false)
const editRepoForm = ref({ oldName: '', url: '', branch: '' })
const savingRepo = ref(false)

// Discovery
const currentRepo = ref<SkillRepo | null>(null)
const repoSkillList = ref<DiscoverableSkill[]>([])
const loadingSkills = ref(false)
const skillSearchQuery = ref('')

const filteredSkillList = computed(() => {
  if (!skillSearchQuery.value) return repoSkillList.value
  const query = skillSearchQuery.value.toLowerCase()
  return repoSkillList.value.filter(s => 
    s.name.toLowerCase().includes(query) || 
    s.directory.toLowerCase().includes(query) ||
    s.description?.toLowerCase().includes(query)
  )
})

const installedDirectories = computed(() => new Set(installedList.value.map(s => s.directory)))
function isInstalled(directory: string): boolean {
  const dirName = directory.split('/').pop() || directory
  return installedDirectories.value.has(dirName)
}

async function fetchInstalled() {
  loadingInstalled.value = true
  try {
    installedList.value = await skillsApi.getInstalled()
  } catch (error: any) {
    notify(error?.message || '加载失败', 'error')
  } finally {
    loadingInstalled.value = false
  }
}

async function fetchRepos() {
  loadingRepos.value = true
  try {
    repoList.value = await skillsApi.getRepos()
  } catch (error: any) {
    notify(error?.message || '加载失败', 'error')
  } finally {
    loadingRepos.value = false
  }
}

function handleRepoClick(repo: SkillRepo) {
  currentRepo.value = repo
  fetchRepoSkills()
}

async function fetchRepoSkills() {
  if (!currentRepo.value) return
  loadingSkills.value = true
  try {
    repoSkillList.value = await skillsApi.discoverRepoSkills(currentRepo.value.name)
  } catch (error: any) {
    notify(error?.message || '加载失败', 'error')
  } finally {
    loadingSkills.value = false
  }
}

async function refreshRepoSkills() {
  if (!currentRepo.value) return
  loadingSkills.value = true
  try {
    repoSkillList.value = await skillsApi.refreshRepoSkills(currentRepo.value.name)
    notify('已获取最新列表')
  } catch (error: any) {
    notify(error?.message || '刷新失败', 'error')
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
  try {
    await skillsApi.toggleCli(skill.id, cliType, enabled)
    // Snappy UI update
    if (skill.cli_flags) {
      skill.cli_flags[cliType] = enabled
    }
    notify('已更新')
  } catch (error: any) {
    notify(error?.message || '更新失败', 'error')
    await fetchInstalled() // Rollback
  }
}

async function handleUninstall(skill: InstalledSkill) {
  try {
    await ElMessageBox.confirm(`确定卸载技能 "${skill.name}"?`, '确认卸载')
    await skillsApi.uninstall(skill.id)
    notify('已卸载')
    await fetchInstalled()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(error?.message || '卸载失败', 'error')
    }
  }
}

async function handleInstall(skill: DiscoverableSkill, reinstall: boolean = false) {
  try {
    if (reinstall) {
      await ElMessageBox.confirm(`确定重装 "${skill.name}"? (将更新为最新版本)`, '确认重装')
    }
    installingSkillId.value = skill.key
    await skillsApi.install(skill, reinstall)
    notify(reinstall ? '重装成功' : '安装成功')
    await fetchInstalled()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(error?.message || '安装失败', 'error')
    }
  } finally {
    installingSkillId.value = null
  }
}

function toDiscoverableSkill(installed: InstalledSkill): DiscoverableSkill {
  return {
    key: `${installed.repo_owner}/${installed.repo_name}:${installed.directory}`,
    name: installed.name,
    description: installed.description || '',
    directory: installed.directory,
    readme_url: installed.readme_url,
    repo_owner: installed.repo_owner || '',
    repo_name: installed.repo_name || '',
    repo_branch: installed.repo_branch || 'main',
  }
}

async function handleInstallFromInstalled(skill: InstalledSkill) {
  if (!skill.repo_owner || !skill.repo_name) {
    notify('缺少仓库信息，无法安装', 'error')
    return
  }
  installingSkillId.value = `installed-${skill.id}`
  try {
    await skillsApi.install(toDiscoverableSkill(skill), true)
    notify('安装成功')
    await fetchInstalled()
  } catch (error: any) {
    notify(error?.message || '安装失败', 'error')
  } finally {
    installingSkillId.value = null
  }
}

async function handleReinstallFromInstalled(skill: InstalledSkill) {
  if (!skill.repo_owner || !skill.repo_name) {
    notify('缺少仓库信息，无法重装', 'error')
    return
  }
  try {
    await ElMessageBox.confirm(`确定重装技能 "${skill.name}"?`, '确认重装')
    installingSkillId.value = `installed-${skill.id}`
    await skillsApi.install(toDiscoverableSkill(skill), true)
    notify('重装成功')
    await fetchInstalled()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(error?.message || '重装失败', 'error')
    }
  } finally {
    installingSkillId.value = null
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
  savingRepo.value = true
  try {
    await skillsApi.addRepo({
      url: repoForm.value.url.trim(),
      branch: repoForm.value.branch.trim() || undefined
    })
    notify('添加成功')
    showAddRepoDialog.value = false
    repoForm.value = { url: '', branch: '' }
    await fetchRepos()
  } catch (error: any) {
    notify(error?.message || '添加失败', 'error')
  } finally {
    savingRepo.value = false
  }
}

async function handleRemoveRepo(repo: SkillRepo) {
  try {
    await ElMessageBox.confirm(`确定删除仓库 "${repo.name}"?`, '确认删除')
    await skillsApi.removeRepo(repo.name)
    notify('已删除')
    await fetchRepos()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(error?.message || '删除失败', 'error')
    }
  }
}

function handleEditRepo(repo: SkillRepo) {
  editRepoForm.value = {
    oldName: repo.name,
    url: repo.source,
    branch: repo.branch || ''
  }
  showEditRepoDialog.value = true
}

async function handleUpdateRepo() {
  if (!editRepoForm.value.url.trim()) {
    notify('请输入仓库地址', 'error')
    return
  }
  savingRepo.value = true
  try {
    await skillsApi.updateRepo(
      editRepoForm.value.oldName,
      editRepoForm.value.url.trim(),
      editRepoForm.value.branch.trim()
    )
    notify('更新成功')
    showEditRepoDialog.value = false
    await fetchRepos()
  } catch (error: any) {
    notify(error?.message || '更新失败', 'error')
  } finally {
    savingRepo.value = false
  }
}

onMounted(() => {
  fetchInstalled()
  fetchRepos()
})
</script>

<style scoped>
.skills-page {
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin-bottom: 24px; padding-top: 8px; }
.tab-item { padding-bottom: 12px; color: #94a3b8; font-weight: 500; font-size: 15px; cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: #475569; }
.tab-item.active { color: #0f172a; font-weight: 600; border-bottom: 2px solid #0f172a; }

/* Header */
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; }
.page-subtitle { font-size: 14px; color: #64748b; margin: 0; }

/* Grid & Cards (Installed) */
.skill-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 24px; }
.skill-card { 
  background: #ffffff; border-radius: 16px; border: 1px solid rgba(226, 232, 240, 0.8); padding: 24px; 
  box-shadow: 0 4px 12px rgba(0,0,0,0.03); transition: all 0.2s; display: flex; flex-direction: column; gap: 20px; 
}
.skill-card:hover { border-color: #0ea5e9; box-shadow: 0 10px 20px -5px rgba(0,0,0,0.05); }

.card-top { display: flex; gap: 16px; align-items: flex-start; }
.skill-icon { 
  width: 48px; height: 48px; border-radius: 12px; background: #f5f3ff; color: #8b5cf6; 
  display: flex; align-items: center; justify-content: center; flex-shrink: 0; 
}
.skill-info { flex: 1; min-width: 0; }
.skill-name { font-size: 16px; font-weight: 700; color: #0f172a; margin: 0 0 4px 0; }
.skill-market { font-size: 12px; color: #64748b; font-weight: 500; }
.skill-source { font-size: 12px; color: #94a3b8; }

.card-actions { display: flex; gap: 4px; }
.action-btn { 
  background: transparent; border: none; color: #94a3b8; padding: 6px; border-radius: 6px; 
  cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.2s; 
  outline: none;
}
.action-btn:hover { background: #f1f5f9; color: #0f172a; }
.action-btn.delete:hover { background: #fef2f2; color: #f43f5e; }

/* CLI Toggles */
.cli-toggles { display: flex; flex-direction: column; gap: 12px; background: #f8fafc; padding: 16px; border-radius: 12px; }
.toggle-item { display: flex; justify-content: space-between; align-items: center; }
.toggle-label { font-size: 13px; font-weight: 500; color: #475569; }

/* Repo Grid (Available) */
.repo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(400px, 1fr)); gap: 20px; }
.repo-card { 
  background: #ffffff; border-radius: 16px; border: 1px solid #f1f5f9; padding: 20px; 
  cursor: pointer; position: relative; transition: all 0.2s; display: flex; align-items: center; gap: 16px; 
}
.repo-card:hover { border-color: #0ea5e9; background: #f8fafc; }

.repo-icon-box { 
  width: 40px; height: 40px; border-radius: 10px; background: #f1f5f9; color: #64748b; 
  display: flex; align-items: center; justify-content: center; 
}
.repo-info-main { flex: 1; min-width: 0; }
.repo-name-title { font-weight: 700; font-size: 15px; color: #0f172a; margin-bottom: 4px; overflow: hidden; text-overflow: ellipsis; }
.repo-owner-subtitle { font-size: 12px; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.repo-actions-overlay { display: flex; gap: 4px; flex-shrink: 0; }

/* Discover List */
.discover-list { background: #ffffff; border-radius: 16px; overflow: hidden; border: 1px solid #f1f5f9; }
.discover-item { 
  display: flex; justify-content: space-between; align-items: center; padding: 20px 24px; 
  border-bottom: 1px solid #f1f5f9; transition: background 0.2s; 
}
.discover-item:last-child { border-bottom: none; }
.discover-item:hover { background: #f8fafc; }
.discover-info { flex: 1; min-width: 0; padding-right: 40px; }
.discover-name-row { margin-bottom: 6px; display: flex; align-items: center; gap: 8px; }
.discover-name { font-weight: 700; font-size: 15px; color: #0f172a; }
.discover-desc { 
  font-size: 13px; color: #64748b; line-height: 1.5; cursor: pointer; 
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}
.discover-actions { flex-shrink: 0; }


/* Shared styles */
.mono { font-family: "JetBrains Mono", monospace; }
.tag { padding: 2px 8px; border-radius: 4px; font-size: 10px; font-weight: 700; text-transform: uppercase; }
.tag-red { background: #fef2f2; color: #f43f5e; }
.pill { padding: 4px 10px; border-radius: 999px; font-size: 11px; font-weight: 600; }
.pill-grey { background: #f1f5f9; color: #64748b; }

.search-box { position: relative; }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; }
.c-input { 
  width: 100%; padding: 8px 12px 8px 36px; background: #ffffff; border: 1px solid #e2e8f0; 
  border-radius: 8px; font-size: 13px; color: #0f172a; outline: none; transition: all 0.2s; 
}
.c-input:focus { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); }

.b-button { 
  background: #0ea5e9; color: #ffffff; border: none; padding: 10px 20px; border-radius: 10px; 
  font-size: 14px; font-weight: 600; cursor: pointer; display: flex; align-items: center; 
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.2); transition: all 0.2s; white-space: nowrap;
}
.b-button:hover { background: #0284c7; transform: translateY(-1px); }
.b-button:disabled { background: #94a3b8; cursor: not-allowed; box-shadow: none; transform: none; }

.b-button-outline { 
  background: #ffffff; color: #475569; border: 1px solid #e2e8f0; padding: 8px 16px; border-radius: 8px; 
  font-size: 13px; font-weight: 500; cursor: pointer; transition: all 0.2s; display: flex; align-items: center; 
}
.b-button-outline:hover { background: #f8fafc; color: #0f172a; border-color: #cbd5e1; }

.empty-state { padding: 80px 40px; text-align: center; color: #94a3b8; background: #ffffff; border-radius: 24px; border: 2px dashed #e2e8f0; }
.empty-state p { margin-top: 16px; font-size: 15px; }

.form-group { margin-bottom: 24px; }
.c-label { display: block; font-size: 14px; font-weight: 600; color: #475569; margin-bottom: 8px; }
.required { color: #f43f5e; }
</style>
