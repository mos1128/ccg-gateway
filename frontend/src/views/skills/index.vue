<template>
  <div class="skills-page">
    <el-tabs v-model="activeTab">
      <!-- 已安装 Skills -->
      <el-tab-pane label="已安装" name="installed">
        <el-table :data="installedList" stripe style="width: 100%" v-loading="loadingInstalled">
          <el-table-column prop="name" label="名称" min-width="150" />
          <el-table-column label="来源" min-width="150">
            <template #default="{ row }">
              <span v-if="row.repo_owner">{{ row.repo_owner }}/{{ row.repo_name }}</span>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="Claude Code" width="120">
            <template #default="{ row }">
              <el-switch
                :model-value="row.cli_flags?.claude_code"
                @change="handleCliToggle(row, 'claude_code', $event as boolean)"
              />
            </template>
          </el-table-column>
          <el-table-column label="Codex" width="100">
            <template #default="{ row }">
              <el-switch
                :model-value="row.cli_flags?.codex"
                @change="handleCliToggle(row, 'codex', $event as boolean)"
              />
            </template>
          </el-table-column>
          <el-table-column label="Gemini" width="100">
            <template #default="{ row }">
              <el-switch
                :model-value="row.cli_flags?.gemini"
                @change="handleCliToggle(row, 'gemini', $event as boolean)"
              />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="180">
            <template #default="{ row }">
              <template v-if="row.exists_on_disk">
                <el-button size="small" type="warning" :loading="installingSkillId === `installed-${row.id}`" @click="handleReinstallFromInstalled(row)">重装</el-button>
                <el-button size="small" type="danger" @click="handleUninstall(row)">卸载</el-button>
              </template>
              <template v-else>
                <el-button size="small" type="primary" :loading="installingSkillId === `installed-${row.id}`" @click="handleInstallFromInstalled(row)">安装</el-button>
              </template>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 可安装 Skills -->
      <el-tab-pane label="可安装" name="available">
        <!-- 仓库列表视图 -->
        <div v-if="!currentRepo" class="repo-list">
          <div class="page-header">
            <span class="header-title">仓库列表</span>
            <el-button type="primary" @click="showAddRepoDialog = true">
              <el-icon><Plus /></el-icon>
              添加仓库
            </el-button>
          </div>
          <el-card v-loading="loadingRepos">
            <template v-if="repoList.length === 0">
              <el-empty description="暂无仓库，请添加 Skill 仓库" />
            </template>
            <template v-else>
              <div v-for="repo in repoList" :key="`${repo.owner}/${repo.name}`" class="repo-item">
                <div class="repo-info">
                  <div class="repo-name">
                    {{ repo.owner }}/{{ repo.name }}
                    <el-tag size="small" type="info">{{ repo.branch }}</el-tag>
                  </div>
                </div>
                <div class="repo-actions">
                  <el-button size="small" @click="handleRepoClick(repo)">查看 Skills</el-button>
                  <el-button size="small" @click="handleEditRepo(repo)">编辑</el-button>
                  <el-button size="small" type="danger" @click="handleRemoveRepo(repo)">删除</el-button>
                </div>
              </div>
            </template>
          </el-card>
        </div>

        <!-- 仓库 Skills 列表视图 -->
        <div v-else class="skill-list">
          <div class="page-header">
            <div class="header-left">
              <el-button :icon="ArrowLeft" @click="handleBackToRepos">返回</el-button>
              <div class="repo-title">
                <span class="header-title">{{ currentRepo.owner }}/{{ currentRepo.name }}</span>
                <el-tag size="small" type="info">{{ currentRepo.branch }}</el-tag>
              </div>
            </div>
            <div class="header-right">
              <el-input
                v-model="skillSearchQuery"
                placeholder="搜索 Skill..."
                clearable
                style="width: 200px"
              >
                <template #prefix>
                  <el-icon><Search /></el-icon>
                </template>
              </el-input>
              <el-button type="primary" @click="refreshRepoSkills" :loading="loadingSkills">
                获取最新列表
              </el-button>
            </div>
          </div>
          <el-card v-loading="loadingSkills">
            <template v-if="filteredSkillList.length === 0 && !loadingSkills">
              <el-empty :description="skillSearchQuery ? '无匹配结果' : '该仓库暂无 Skills'" />
            </template>
            <el-table v-else :data="filteredSkillList" stripe style="width: 100%">
              <el-table-column prop="name" label="名称" min-width="150" />
              <el-table-column prop="description" label="描述" min-width="300">
                <template #default="{ row }">
                  <el-tooltip
                    v-if="row.description"
                    :content="row.description"
                    placement="top"
                    :show-after="300"
                    :hide-after="0"
                    popper-class="skill-desc-tooltip"
                  >
                    <div class="skill-desc" @click="copyDescription(row.description)">
                      {{ row.description }}
                    </div>
                  </el-tooltip>
                  <span v-else>-</span>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="120">
                <template #default="{ row }">
                  <el-button
                    size="small"
                    :type="isInstalled(row.directory) ? 'warning' : 'primary'"
                    :loading="installingSkillId === row.key"
                    @click="handleInstall(row, isInstalled(row.directory))"
                  >
                    {{ isInstalled(row.directory) ? '重装' : '安装' }}
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </el-card>
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- 添加仓库对话框 -->
    <el-dialog v-model="showAddRepoDialog" title="添加仓库" width="500px">
      <el-form :model="repoForm" label-width="80px">
        <el-form-item label="仓库地址" required>
          <el-input v-model="repoForm.url" placeholder="https://github.com/owner/repo 或 owner/repo" />
        </el-form-item>
        <el-form-item label="分支">
          <el-input v-model="repoForm.branch" placeholder="默认 main" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showAddRepoDialog = false" :disabled="savingRepo">取消</el-button>
        <el-button type="primary" @click="handleAddRepo" :loading="savingRepo">添加</el-button>
      </template>
    </el-dialog>

    <!-- 编辑仓库对话框 -->
    <el-dialog v-model="showEditRepoDialog" title="编辑仓库" width="500px">
      <el-form :model="editRepoForm" label-width="80px">
        <el-form-item label="仓库地址" required>
          <el-input v-model="editRepoForm.url" placeholder="https://github.com/owner/repo 或 owner/repo" />
        </el-form-item>
        <el-form-item label="分支">
          <el-input v-model="editRepoForm.branch" placeholder="默认 main" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showEditRepoDialog = false" :disabled="savingRepo">取消</el-button>
        <el-button type="primary" @click="handleUpdateRepo" :loading="savingRepo">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, ArrowLeft, Search } from '@element-plus/icons-vue'
import { skillsApi } from '@/api/skills'
import type { SkillRepo, DiscoverableSkill, InstalledSkill } from '@/types/models'

const activeTab = ref('installed')

// 已安装 Skills
const installedList = ref<InstalledSkill[]>([])
const loadingInstalled = ref(false)
const installingSkillId = ref<string | null>(null) // 正在安装/重装的 skill 标识

// 仓库
const repoList = ref<SkillRepo[]>([])
const loadingRepos = ref(false)
const showAddRepoDialog = ref(false)
const repoForm = ref({ url: '', branch: '' })
const showEditRepoDialog = ref(false)
const editRepoForm = ref({ oldOwner: '', oldName: '', url: '', branch: '' })
const savingRepo = ref(false) // 添加/更新仓库 loading

// 提取错误消息
function getErrorMessage(error: any, fallback: string): string {
  if (typeof error === 'string') return error
  return error?.message || fallback
}

// 当前选中的仓库
const currentRepo = ref<SkillRepo | null>(null)
const repoSkillList = ref<DiscoverableSkill[]>([])
const loadingSkills = ref(false)
const skillSearchQuery = ref('')

// 过滤后的 Skill 列表
const filteredSkillList = computed(() => {
  if (!skillSearchQuery.value) return repoSkillList.value
  const query = skillSearchQuery.value.toLowerCase()
  return repoSkillList.value.filter(s => s.name.toLowerCase().includes(query))
})

// 检查是否已安装
const installedDirectories = computed(() => new Set(installedList.value.map(s => s.directory)))
function isInstalled(directory: string): boolean {
  const dirName = directory.split('/').pop() || directory
  return installedDirectories.value.has(dirName)
}

// 加载已安装 Skills
async function fetchInstalled() {
  loadingInstalled.value = true
  try {
    installedList.value = await skillsApi.getInstalled()
  } catch (error: any) {
    ElMessage.error(error?.message || '加载失败')
  } finally {
    loadingInstalled.value = false
  }
}

// 加载仓库列表
async function fetchRepos() {
  loadingRepos.value = true
  try {
    repoList.value = await skillsApi.getRepos()
  } catch (error: any) {
    ElMessage.error(error?.message || '加载失败')
  } finally {
    loadingRepos.value = false
  }
}

// 点击仓库，加载该仓库的 skills
function handleRepoClick(repo: SkillRepo) {
  currentRepo.value = repo
  fetchRepoSkills()
}

// 加载当前仓库的 Skills
async function fetchRepoSkills() {
  if (!currentRepo.value) return
  loadingSkills.value = true
  try {
    repoSkillList.value = await skillsApi.discoverRepoSkills(
      currentRepo.value.owner,
      currentRepo.value.name,
      currentRepo.value.branch
    )
  } catch (error: any) {
    ElMessage.error(error?.message || '加载失败')
  } finally {
    loadingSkills.value = false
  }
}

// 强制刷新当前仓库的 Skills（删除缓存后重新下载）
async function refreshRepoSkills() {
  if (!currentRepo.value) return
  loadingSkills.value = true
  try {
    repoSkillList.value = await skillsApi.refreshRepoSkills(
      currentRepo.value.owner,
      currentRepo.value.name,
      currentRepo.value.branch
    )
    ElMessage.success('已获取最新列表')
  } catch (error: any) {
    ElMessage.error(error?.message || '刷新失败')
  } finally {
    loadingSkills.value = false
  }
}

// 返回仓库列表
function handleBackToRepos() {
  currentRepo.value = null
  repoSkillList.value = []
}

// CLI 启用/禁用
async function handleCliToggle(skill: InstalledSkill, cliType: string, enabled: boolean) {
  try {
    await skillsApi.toggleCli(skill.id, cliType, enabled)
    await fetchInstalled()
    ElMessage.success('已更新')
  } catch (error: any) {
    ElMessage.error(error?.message || '更新失败')
  }
}

// 卸载 Skill
async function handleUninstall(skill: InstalledSkill) {
  try {
    await ElMessageBox.confirm(`确定卸载 "${skill.name}"?`, '确认')
    await skillsApi.uninstall(skill.id)
    ElMessage.success('已卸载')
    await fetchInstalled()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error?.message || '卸载失败')
    }
  }
}

// 安装/重装 Skill (从可安装列表)
async function handleInstall(skill: DiscoverableSkill, reinstall: boolean = false) {
  try {
    if (reinstall) {
      await ElMessageBox.confirm(`确定重装 "${skill.name}"?（将更新为最新版本）`, '确认')
    }
    installingSkillId.value = skill.key
    await skillsApi.install(skill, reinstall)
    ElMessage.success(reinstall ? '重装成功' : '安装成功')
    await fetchInstalled()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error?.message || '安装失败')
    }
  } finally {
    installingSkillId.value = null
  }
}

// 从已安装列表构造 DiscoverableSkill 并安装
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

// 从已安装列表安装 (exists_on_disk = false 时)
async function handleInstallFromInstalled(skill: InstalledSkill) {
  if (!skill.repo_owner || !skill.repo_name) {
    ElMessage.error('缺少仓库信息，无法安装')
    return
  }
  installingSkillId.value = `installed-${skill.id}`
  try {
    await skillsApi.install(toDiscoverableSkill(skill), true) // 使用 reinstall=true 更新记录
    ElMessage.success('安装成功')
    await fetchInstalled()
  } catch (error: any) {
    ElMessage.error(error?.message || '安装失败')
  } finally {
    installingSkillId.value = null
  }
}

// 从已安装列表重装 (exists_on_disk = true 时)
async function handleReinstallFromInstalled(skill: InstalledSkill) {
  if (!skill.repo_owner || !skill.repo_name) {
    ElMessage.error('缺少仓库信息，无法重装')
    return
  }
  try {
    await ElMessageBox.confirm(`确定重装 "${skill.name}"?（将更新为最新版本）`, '确认')
    installingSkillId.value = `installed-${skill.id}`
    await skillsApi.install(toDiscoverableSkill(skill), true)
    ElMessage.success('重装成功')
    await fetchInstalled()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error?.message || '重装失败')
    }
  } finally {
    installingSkillId.value = null
  }
}

// 复制描述
async function copyDescription(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    ElMessage.success('已复制')
  } catch {
    ElMessage.error('复制失败')
  }
}

// 添加仓库
async function handleAddRepo() {
  if (!repoForm.value.url.trim()) {
    ElMessage.error('请输入仓库地址')
    return
  }
  savingRepo.value = true
  try {
    await skillsApi.addRepo({
      url: repoForm.value.url.trim(),
      branch: repoForm.value.branch.trim() || undefined
    })
    ElMessage.success('添加成功')
    showAddRepoDialog.value = false
    repoForm.value = { url: '', branch: '' }
    await fetchRepos()
  } catch (error: any) {
    ElMessage.error(getErrorMessage(error, '添加失败'))
  } finally {
    savingRepo.value = false
  }
}

// 删除仓库
async function handleRemoveRepo(repo: SkillRepo) {
  try {
    await ElMessageBox.confirm(`确定删除仓库 "${repo.owner}/${repo.name}"?`, '确认')
    await skillsApi.removeRepo(repo.owner, repo.name)
    ElMessage.success('已删除')
    await fetchRepos()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error?.message || '删除失败')
    }
  }
}

// 编辑仓库
function handleEditRepo(repo: SkillRepo) {
  editRepoForm.value = {
    oldOwner: repo.owner,
    oldName: repo.name,
    url: `${repo.owner}/${repo.name}`,
    branch: repo.branch
  }
  showEditRepoDialog.value = true
}

async function handleUpdateRepo() {
  if (!editRepoForm.value.url.trim()) {
    ElMessage.error('请输入仓库地址')
    return
  }
  savingRepo.value = true
  try {
    await skillsApi.updateRepo(
      editRepoForm.value.oldOwner,
      editRepoForm.value.oldName,
      editRepoForm.value.url.trim(),
      editRepoForm.value.branch.trim()
    )
    ElMessage.success('更新成功')
    showEditRepoDialog.value = false
    await fetchRepos()
  } catch (error: any) {
    ElMessage.error(getErrorMessage(error, '更新失败'))
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
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.repo-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-title {
  font-size: 18px;
  font-weight: 600;
}

.repo-item {
  display: flex;
  align-items: center;
  padding: 15px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.repo-item:last-child {
  border-bottom: none;
}

.repo-info {
  flex: 1;
  min-width: 0;
}

.repo-name {
  font-weight: bold;
  display: flex;
  align-items: center;
  gap: 8px;
}

.repo-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.skill-desc {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
  line-height: 1.5;
}

.skill-desc:hover {
  color: var(--el-color-primary);
}
</style>

<style>
.skill-desc-tooltip {
  max-width: 400px;
  word-break: break-word;
}
</style>
