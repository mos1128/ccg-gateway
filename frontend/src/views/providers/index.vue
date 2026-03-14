<template>
  <div class="providers-page">
    <el-tabs v-model="activeCliType" @tab-change="handleCliTypeChange">
      <el-tab-pane label="Claude Code" name="claude_code" />
      <el-tab-pane label="Codex" name="codex" />
      <el-tab-pane label="Gemini" name="gemini" />
    </el-tabs>

    <!-- 模式切换 -->
    <div class="mode-switch">
      <el-radio-group :model-value="cliMode" @change="handleModeChange" size="large">
        <el-radio-button value="proxy">中转模式</el-radio-button>
        <el-radio-button value="direct">官方模式</el-radio-button>
      </el-radio-group>
    </div>

    <!-- 中转模式：服务商列表 -->
    <div v-if="cliMode === 'proxy'">
      <div class="page-header">
        <el-button type="primary" @click="showAddDialog = true">
          <el-icon><Plus /></el-icon>
          添加服务商
        </el-button>
      </div>

      <el-card v-loading="providerStore.loading">
        <template v-if="providerStore.providers.length === 0">
          <el-empty description="暂无服务商" />
        </template>
        <draggable
          v-else
          v-model="providerStore.providers"
          item-key="id"
          handle=".drag-handle"
          @end="handleDragEnd"
        >
          <template #item="{ element }">
            <div class="provider-item">
              <div class="drag-handle" aria-label="拖拽排序">
                <el-icon><Rank /></el-icon>
              </div>
              <div class="provider-info">
                <div class="provider-name">
                  {{ element.name }}
                  <el-tag v-if="element.is_blacklisted" type="danger" size="small">
                    {{ getUnblacklistTime(element) }}
                  </el-tag>
                  <el-tag v-else-if="!element.enabled" type="info" size="small">已禁用</el-tag>
                  <el-tag v-if="element.model_maps.length > 0" type="success" size="small">
                    {{ element.model_maps.length }}个模型映射
                  </el-tag>
                  <el-tag v-if="element.model_blacklist && element.model_blacklist.length > 0" type="warning" size="small">
                    {{ element.model_blacklist.length }}个黑名单
                  </el-tag>
                </div>
                <div class="provider-url">{{ element.base_url }}</div>
              </div>
              <div class="provider-stats">
                <span>失败：{{ element.consecutive_failures }}/{{ element.failure_threshold }}</span>
              </div>
              <div class="provider-actions">
                <el-switch
                  v-model="element.enabled"
                  @change="handleToggle(element)"
                />
                <el-button size="small" @click="handleEdit(element)">编辑</el-button>
                <el-dropdown @command="handleCommand($event, element)">
                  <el-button size="small">
                    更多<el-icon class="el-icon--right"><ArrowDown /></el-icon>
                  </el-button>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item command="reset">重置失败计数</el-dropdown-item>
                      <el-dropdown-item v-if="element.is_blacklisted" command="unblacklist">解除拉黑</el-dropdown-item>
                      <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </div>
            </div>
          </template>
        </draggable>
      </el-card>

      <!-- Add/Edit Provider Dialog -->
      <el-dialog
        v-model="showDialog"
        :title="editingProvider ? '编辑服务商' : '添加服务商'"
        width="700px"
      >
        <el-form ref="providerFormRef" :model="form" :rules="providerRules" label-width="120px">
          <el-form-item label="名称" prop="name" required>
            <el-input v-model="form.name" placeholder="服务商名称" />
          </el-form-item>
          <el-form-item label="Base URL" prop="base_url" required>
            <el-input v-model="form.base_url" :placeholder="baseUrlPlaceholder" />
          </el-form-item>
          <el-form-item :label="activeCliType === 'claude_code' ? 'API Token' : 'API Key'" prop="api_key" required>
            <el-input v-model="form.api_key" :placeholder="activeCliType === 'claude_code' ? 'API Token' : 'API Key'" />
          </el-form-item>
          <el-form-item label="失败阈值">
            <el-input-number v-model="form.failure_threshold" :min="1" :max="100" />
            <span class="form-tip">连续失败次数达到此值后拉黑</span>
          </el-form-item>
          <el-form-item label="拉黑时长 (分钟)">
            <el-input-number v-model="form.blacklist_minutes" :min="0" :max="1440" />
          </el-form-item>
          <el-form-item label="自定义 UA">
            <el-input v-model="form.custom_useragent" placeholder="留空则使用原始 UA" clearable />
            <span class="form-tip">替换转发请求的 User-Agent</span>
          </el-form-item>

          <el-divider>模型转发配置</el-divider>
          <div class="model-maps-section">
            <div class="model-maps-header">
              <span class="model-maps-tip">将 CLI 请求的模型名映射为服务商模型名</span>
              <el-button type="primary" size="small" @click="addModelMap">
                <el-icon><Plus /></el-icon>添加映射
              </el-button>
            </div>
            <div v-if="form.model_maps.length === 0" class="model-maps-empty">
              暂无模型映射配置
            </div>
            <div v-else class="model-maps-list">
              <div v-for="(map, index) in form.model_maps" :key="index" class="model-map-item">
                <el-input v-model="map.source_model" placeholder="源模型 (CLI 请求)" class="model-input" />
                <el-icon class="arrow-icon"><Right /></el-icon>
                <el-input v-model="map.target_model" placeholder="目标模型 (服务商)" class="model-input" />
                <el-button type="danger" size="small" circle @click="removeModelMap(index)">
                  <el-icon><Delete /></el-icon>
                </el-button>
              </div>
            </div>
          </div>

          <el-divider>模型黑名单</el-divider>
          <div class="model-maps-section">
            <div class="model-maps-header">
              <span class="model-maps-tip">配置不支持请求的模型（支持通配符 * 和 ?）</span>
              <el-button type="primary" size="small" @click="addModelBlacklist">
                <el-icon><Plus /></el-icon>添加黑名单
              </el-button>
            </div>
            <div v-if="form.model_blacklist.length === 0" class="model-maps-empty">
              暂无黑名单配置（默认支持所有模型）
            </div>
            <div v-else class="model-maps-list">
              <div v-for="(item, index) in form.model_blacklist" :key="index" class="model-map-item">
                <el-input v-model="item.model_pattern" placeholder="模型匹配模式 (如: claude-opus-*)" class="model-input" />
                <el-button type="danger" size="small" circle @click="removeModelBlacklist(index)">
                  <el-icon><Delete /></el-icon>
                </el-button>
              </div>
            </div>
          </div>
        </el-form>
        <template #footer>
          <el-button @click="showDialog = false">取消</el-button>
          <el-button type="primary" @click="handleSave">保存</el-button>
        </template>
      </el-dialog>
    </div>

    <!-- 官方模式：官方凭证列表 -->
    <div v-else>
      <div class="page-header">
        <el-button type="primary" @click="showAddCredentialDialog = true">
          <el-icon><Plus /></el-icon>
          添加凭证
        </el-button>
      </div>

      <el-card v-loading="credentialStore.loading">
        <template v-if="credentialStore.credentials.length === 0">
          <el-empty description="暂无凭证" />
        </template>
        <draggable
          v-else
          v-model="credentialStore.credentials"
          item-key="id"
          handle=".drag-handle"
          @end="handleCredentialDragEnd"
        >
          <template #item="{ element }">
            <div class="credential-item">
              <div class="drag-handle" aria-label="拖拽排序">
                <el-icon><Rank /></el-icon>
              </div>
              <div class="credential-info">
                <div class="credential-name">
                  {{ element.name }}
                  <el-tag v-if="element.is_active" type="success" size="small">激活中</el-tag>
                </div>
                <div class="credential-display-info">{{ element.display_info }}</div>
              </div>
              <div class="credential-actions">
                <el-button size="small" @click="handleEditCredential(element)">编辑</el-button>
                <el-button size="small" @click="handleDeleteCredential(element)">删除</el-button>
              </div>
            </div>
          </template>
        </draggable>
      </el-card>

      <!-- Add/Edit Credential Dialog -->
      <el-dialog
        v-model="showCredentialDialog"
        :title="editingCredential ? '编辑凭证' : '添加凭证'"
        width="700px"
      >
        <el-form ref="credentialFormRef" :model="credentialForm" :rules="credentialRules" label-width="120px">
          <el-form-item label="名称" prop="name" required>
            <el-input v-model="credentialForm.name" placeholder="凭证标识名" />
          </el-form-item>

          <el-divider>配置文件</el-divider>
          <div class="credential-files-section">
            <div class="credential-files-header">
              <span class="credential-files-tip">配置文件路径</span>
              <el-button type="primary" size="small" @click="handleReadFromCli">
                <el-icon><Refresh /></el-icon>读取当前 CLI 配置
              </el-button>
            </div>
            
            <div class="credential-files-list">
              <!-- Claude Code: settings.json -->
              <template v-if="activeCliType === 'claude_code'">
                <div class="credential-file-item">
                  <div class="credential-file-label">~/.claude/settings.json</div>
                  <el-input
                    v-model="credentialForm.claude_settings"
                    type="textarea"
                    :rows="10"
                    placeholder='{"ANTHROPIC_API_KEY": "sk-ant-xxx"}'
                  />
                </div>
              </template>

              <!-- Codex: auth.json -->
              <template v-if="activeCliType === 'codex'">
                <div class="credential-file-item">
                  <div class="credential-file-label">~/.codex/auth.json</div>
                  <el-input
                    v-model="credentialForm.codex_auth"
                    type="textarea"
                    :rows="10"
                    placeholder='{"OPENAI_API_KEY": null, "tokens": {...}}'
                  />
                </div>
              </template>

              <!-- Gemini: oauth_creds.json + google_accounts.json + settings.json -->
              <template v-if="activeCliType === 'gemini'">
                <div class="credential-file-item">
                  <div class="credential-file-label">~/.gemini/oauth_creds.json</div>
                  <el-input
                    v-model="credentialForm.gemini_oauth"
                    type="textarea"
                    :rows="8"
                    placeholder='{"access_token": "...", "refresh_token": "..."}'
                  />
                </div>
                <div class="credential-file-item">
                  <div class="credential-file-label">~/.gemini/google_accounts.json</div>
                  <el-input
                    v-model="credentialForm.gemini_accounts"
                    type="textarea"
                    :rows="4"
                    placeholder='{"active": "user@gmail.com", "old": []}'
                  />
                </div>
                <div class="credential-file-item">
                  <div class="credential-file-label">~/.gemini/settings.json</div>
                  <el-input
                    v-model="credentialForm.gemini_settings"
                    type="textarea"
                    :rows="6"
                    placeholder='{"general": {...}, "security": {...}}'
                  />
                </div>
              </template>
            </div>
          </div>
        </el-form>
        <template #footer>
          <el-button @click="showCredentialDialog = false">取消</el-button>
          <el-button type="primary" @click="handleSaveCredential">保存</el-button>
        </template>
      </el-dialog>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import draggable from 'vuedraggable'
import { useProviderStore } from '@/stores/providers'
import { useCredentialStore } from '@/stores/credentials'
import { useUiStore } from '@/stores/ui'
import { useSettingsStore } from '@/stores/settings'
import { credentialsApi } from '@/api/credentials'
import type { Provider, ModelMap, ModelBlacklist, CliType } from '@/types/models'
import type { OfficialCredential, OfficialCredentialCreate } from '@/types/models'

const providerStore = useProviderStore()
const credentialStore = useCredentialStore()
const uiStore = useUiStore()
const settingsStore = useSettingsStore()

const providerFormRef = ref<FormInstance>()
const credentialFormRef = ref<FormInstance>()
const refreshTimer = ref<number>()
const forceRefresh = ref(0)
const hasRefreshed = ref(false)
const visibilityHandler = ref<() => void>()

const activeCliType = computed({
  get: () => uiStore.providersActiveCliType,
  set: (val) => uiStore.setProvidersActiveCliType(val)
})

const cliMode = computed(() => {
  return settingsStore.settings?.cli_settings?.[activeCliType.value]?.cli_mode ?? 'proxy'
})

const showAddDialog = ref(false)
const editingProvider = ref<Provider | null>(null)
const showAddCredentialDialog = ref(false)
const editingCredential = ref<OfficialCredential | null>(null)

const showDialog = computed({
  get: () => showAddDialog.value || !!editingProvider.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingProvider.value = null
    }
  }
})

const showCredentialDialog = computed({
  get: () => showAddCredentialDialog.value || !!editingCredential.value,
  set: (val) => {
    if (!val) {
      showAddCredentialDialog.value = false
      editingCredential.value = null
    }
  }
})

interface FormModelMap {
  source_model: string
  target_model: string
  enabled: boolean
}

interface FormModelBlacklist {
  model_pattern: string
}

const form = ref({
  name: '',
  base_url: '',
  api_key: '',
  failure_threshold: 3,
  blacklist_minutes: 10,
  custom_useragent: '',
  model_maps: [] as FormModelMap[],
  model_blacklist: [] as FormModelBlacklist[]
})

const credentialForm = ref({
  name: '',
  // Claude Code
  claude_settings: '',
  // Codex
  codex_auth: '',
  // Gemini
  gemini_oauth: '',
  gemini_accounts: '',
  gemini_settings: ''
})

const providerRules: FormRules = {
  name: [
    { required: true, message: '请输入服务商名称', trigger: 'blur' }
  ],
  base_url: [
    { required: true, message: '请输入 Base URL', trigger: 'blur' }
  ],
  api_key: [
    { required: true, message: '请输入 API Key', trigger: 'blur' }
  ]
}

const credentialRules: FormRules = {
  name: [
    { required: true, message: '请输入凭证名称', trigger: 'blur' }
  ]
}

const baseUrlPlaceholder = computed(() => {
  if (activeCliType.value === 'codex') return 'https://api.example.com/v1'
  return 'https://api.example.com'
})

function resetForm() {
  form.value = {
    name: '',
    base_url: '',
    api_key: '',
    failure_threshold: 3,
    blacklist_minutes: 10,
    custom_useragent: '',
    model_maps: [],
    model_blacklist: []
  }
}

function resetCredentialForm() {
  credentialForm.value = {
    name: '',
    claude_settings: '',
    codex_auth: '',
    gemini_oauth: '',
    gemini_accounts: '',
    gemini_settings: ''
  }
}

function addModelMap() {
  form.value.model_maps.push({
    source_model: '',
    target_model: '',
    enabled: true
  })
}

function removeModelMap(index: number) {
  form.value.model_maps.splice(index, 1)
}

function addModelBlacklist() {
  form.value.model_blacklist.push({
    model_pattern: ''
  })
}

function removeModelBlacklist(index: number) {
  form.value.model_blacklist.splice(index, 1)
}

function handleCliTypeChange(cliType: string) {
  providerStore.fetchProviders(cliType)
  credentialStore.fetchCredentials(cliType as CliType)
}

async function handleModeChange(newMode: 'proxy' | 'direct') {
  // Claude Code 暂未实现官方模式
  if (newMode === 'direct' && activeCliType.value === 'claude_code') {
    ElMessage.warning('Claude Code 暂未实现官方模式功能')
    return
  }

  try {
    await settingsStore.setCliMode(activeCliType.value, newMode)
    
    if (newMode === 'direct') {
      ElMessage.success(`${activeCliType.value} 已切换到官方模式`)
    } else {
      ElMessage.success(`${activeCliType.value} 已切换到中转模式`)
    }
    
    providerStore.fetchProviders(activeCliType.value)
    credentialStore.fetchCredentials(activeCliType.value)
  } catch (e: any) {
    console.error('set_cli_mode error:', e)
    ElMessage.error(`切换模式失败: ${e?.message || e}`)
  }
}

function handleEdit(provider: Provider) {
  editingProvider.value = provider
  form.value = {
    name: provider.name,
    base_url: provider.base_url,
    api_key: provider.api_key,
    failure_threshold: provider.failure_threshold,
    blacklist_minutes: provider.blacklist_minutes,
    custom_useragent: provider.custom_useragent || '',
    model_maps: provider.model_maps.map(m => ({
      source_model: m.source_model,
      target_model: m.target_model,
      enabled: m.enabled
    })),
    model_blacklist: provider.model_blacklist.map(b => ({
      model_pattern: b.model_pattern
    }))
  }
}

function buildModelMaps(): ModelMap[] {
  return form.value.model_maps
    .filter(m => m.source_model && m.target_model)
    .map(m => ({
      source_model: m.source_model.trim(),
      target_model: m.target_model.trim(),
      enabled: true
    }))
}

function buildModelBlacklist(): ModelBlacklist[] {
  return form.value.model_blacklist
    .filter(b => b.model_pattern)
    .map(b => ({
      model_pattern: b.model_pattern.trim()
    }))
}

async function handleSave() {
  if (!providerFormRef.value) return
  
  await providerFormRef.value.validate(async (valid) => {
    if (!valid) return

    const data = {
      cli_type: activeCliType.value,
      name: form.value.name.trim(),
      base_url: form.value.base_url.trim(),
      api_key: form.value.api_key.trim(),
      failure_threshold: form.value.failure_threshold,
      blacklist_minutes: form.value.blacklist_minutes,
      custom_useragent: form.value.custom_useragent.trim(),
      model_maps: buildModelMaps(),
      model_blacklist: buildModelBlacklist()
    }

    try {
      if (editingProvider.value) {
        await providerStore.updateProvider(editingProvider.value.id, data)
        ElMessage.success('更新成功')
      } else {
        await providerStore.createProvider(data)
        ElMessage.success('添加成功')
      }
      showDialog.value = false
      resetForm()
      providerStore.fetchProviders(activeCliType.value)
    } catch {
      // error handled by interceptor
    }
  })
}

async function handleToggle(provider: Provider) {
  try {
    await providerStore.updateProvider(provider.id, { enabled: provider.enabled })
    ElMessage.success(provider.enabled ? '已启用' : '已禁用')
  } catch {
    provider.enabled = !provider.enabled
  }
}

async function handleDragEnd() {
  const ids = providerStore.providers.map(p => p.id)
  await providerStore.reorderProviders(ids)
  ElMessage.success('排序已保存')
}

async function handleCommand(command: string, provider: Provider) {
  if (command === 'reset') {
    await providerStore.resetFailures(provider.id)
    ElMessage.success('已重置')
  } else if (command === 'unblacklist') {
    await providerStore.unblacklist(provider.id)
    ElMessage.success('已解除拉黑')
  } else if (command === 'delete') {
    await ElMessageBox.confirm('确定删除该服务商？', '确认')
    await providerStore.deleteProvider(provider.id)
    ElMessage.success('已删除')
  }
}

// Credential handlers
function handleEditCredential(credential: OfficialCredential) {
  editingCredential.value = credential
  credentialForm.value.name = credential.name
  
  // 解析 credential_json 为文件列表
  try {
    const filesData = JSON.parse(credential.credential_json)
    if (Array.isArray(filesData)) {
      // 根据文件路径填充到对应字段
      filesData.forEach(file => {
        const path = file.path || ''
        const content = file.content || ''
        
        if (path.includes('.claude') && path.includes('settings.json')) {
          credentialForm.value.claude_settings = content
        } else if (path.includes('auth.json')) {
          credentialForm.value.codex_auth = content
        } else if (path.includes('oauth_creds.json')) {
          credentialForm.value.gemini_oauth = content
        } else if (path.includes('google_accounts.json')) {
          credentialForm.value.gemini_accounts = content
        } else if (path.includes('.gemini') && path.includes('settings.json')) {
          credentialForm.value.gemini_settings = content
        }
      })
    }
  } catch (e) {
    console.error('解析凭证 JSON 失败:', e)
  }
}

async function handleDeleteCredential(credential: OfficialCredential) {
  await ElMessageBox.confirm('确定删除该凭证？', '确认')
  await credentialStore.deleteCredential(credential.id)
  ElMessage.success('已删除')
}

async function handleReadFromCli() {
  try {
    const { data } = await credentialsApi.readCliCredential(activeCliType.value)
    
    // 解析返回的文件数据
    try {
      const filesData = JSON.parse(data)
      if (Array.isArray(filesData)) {
        // 根据文件路径填充到对应字段
        filesData.forEach(file => {
          const path = file.path || ''
          const content = file.content || ''
          
          if (path.includes('.claude') && path.includes('settings.json')) {
            credentialForm.value.claude_settings = content
          } else if (path.includes('auth.json')) {
            credentialForm.value.codex_auth = content
          } else if (path.includes('oauth_creds.json')) {
            credentialForm.value.gemini_oauth = content
          } else if (path.includes('google_accounts.json')) {
            credentialForm.value.gemini_accounts = content
          } else if (path.includes('.gemini') && path.includes('settings.json')) {
            credentialForm.value.gemini_settings = content
          }
        })
      }
    } catch (e) {
      console.error('解析文件数据失败:', e)
    }
    
    ElMessage.success('读取成功')
  } catch (e: any) {
    ElMessage.error(e.message || '读取失败')
  }
}

async function handleSaveCredential() {
  if (!credentialFormRef.value) return
  
  await credentialFormRef.value.validate(async (valid) => {
    if (!valid) return

    // 根据 CLI 类型构建文件列表
    const files: Array<{ path: string; content: string }> = []
    
    if (activeCliType.value === 'claude_code') {
      if (credentialForm.value.claude_settings) {
        files.push({
          path: '~/.claude/settings.json',
          content: credentialForm.value.claude_settings
        })
      }
    } else if (activeCliType.value === 'codex') {
      if (credentialForm.value.codex_auth) {
        files.push({
          path: '~/.codex/auth.json',
          content: credentialForm.value.codex_auth
        })
      }
    } else if (activeCliType.value === 'gemini') {
      if (credentialForm.value.gemini_oauth) {
        files.push({
          path: '~/.gemini/oauth_creds.json',
          content: credentialForm.value.gemini_oauth
        })
      }
      if (credentialForm.value.gemini_accounts) {
        files.push({
          path: '~/.gemini/google_accounts.json',
          content: credentialForm.value.gemini_accounts
        })
      }
      if (credentialForm.value.gemini_settings) {
        files.push({
          path: '~/.gemini/settings.json',
          content: credentialForm.value.gemini_settings
        })
      }
    }

    if (files.length === 0) {
      ElMessage.error('请至少填写一个文件内容')
      return
    }

    const filesJson = JSON.stringify(files)

    const data: OfficialCredentialCreate = {
      cli_type: activeCliType.value,
      name: credentialForm.value.name.trim(),
      credential_json: filesJson
    }

    try {
      if (editingCredential.value) {
        await credentialStore.updateCredential(editingCredential.value.id, {
          name: data.name,
          credential_json: data.credential_json
        })
        ElMessage.success('更新成功')
      } else {
        await credentialStore.createCredential(data)
        ElMessage.success('添加成功')
      }
      showCredentialDialog.value = false
      resetCredentialForm()
      credentialStore.fetchCredentials(activeCliType.value)
    } catch {
      // error handled by interceptor
    }
  })
}

async function handleCredentialDragEnd() {
  const ids = credentialStore.credentials.map(c => c.id)
  await credentialStore.reorderCredentials(ids)
  ElMessage.success('排序已保存，第一位置凭证已激活')
}

function getUnblacklistTime(provider: Provider): string {
  void forceRefresh.value

  if (!provider.is_blacklisted || !provider.blacklisted_until) {
    return '已拉黑'
  }

  const now = Date.now() / 1000
  const blacklistedUntil = provider.blacklisted_until
  const diffSeconds = blacklistedUntil - now

  if (diffSeconds <= 0) {
    // 倒计时归零，自动刷新数据（只刷新一次）
    if (!hasRefreshed.value) {
      hasRefreshed.value = true
      providerStore.fetchProviders()
    }
    return '已拉黑'
  }

  const totalSeconds = Math.ceil(diffSeconds)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60

  if (minutes === 0) {
    return `${seconds}秒后解除拉黑`
  } else {
    return `${minutes}分${seconds}秒后解除拉黑`
  }
}

onMounted(() => {
  providerStore.fetchProviders()
  credentialStore.fetchCredentials(activeCliType.value)

  // 启动定时器，每秒刷新一次倒计时显示
  refreshTimer.value = window.setInterval(() => {
    forceRefresh.value++
    hasRefreshed.value = false
  }, 1000)

  // 页面可见性监听：当页面切换到后台时暂停定时器，切回时恢复
  let wasRunning = true
  visibilityHandler.value = () => {
    if (document.hidden) {
      // 页面不可见，暂停定时器
      if (refreshTimer.value) {
        clearInterval(refreshTimer.value)
        refreshTimer.value = undefined
      }
      wasRunning = true
    } else if (wasRunning) {
      // 页面重新可见，恢复定时器
      refreshTimer.value = window.setInterval(() => {
        forceRefresh.value++
        hasRefreshed.value = false
      }, 1000)
      // 切回时立即刷新一次数据
      providerStore.fetchProviders()
    }
  }
  document.addEventListener('visibilitychange', visibilityHandler.value)
})

onUnmounted(() => {
  if (refreshTimer.value) {
    clearInterval(refreshTimer.value)
  }
  if (visibilityHandler.value) {
    document.removeEventListener('visibilitychange', visibilityHandler.value)
  }
})
</script>

<style scoped>
.page-header {
  margin-bottom: 20px;
}

.mode-switch {
  margin: 20px 0;
}

.provider-item,
.credential-item {
  display: flex;
  align-items: center;
  padding: 15px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.provider-item:last-child,
.credential-item:last-child {
  border-bottom: none;
}

.drag-handle {
  cursor: move;
  padding: 10px;
  color: var(--el-text-color-secondary);
}

.provider-info,
.credential-info {
  flex: 1;
  margin-left: 10px;
}

.provider-name,
.credential-name {
  font-weight: bold;
  margin-bottom: 5px;
}

.provider-url {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.credential-display-info {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.provider-stats {
  display: flex;
  gap: 20px;
  margin-right: 20px;
  color: var(--el-text-color-regular);
  font-size: 14px;
}

.provider-actions,
.credential-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.form-tip {
  margin-left: 10px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.model-maps-section {
  padding: 0 20px;
}

.model-maps-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
}

.model-maps-tip {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.model-maps-empty {
  text-align: center;
  padding: 20px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-radius: 4px;
}

.model-maps-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.model-map-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.model-input {
  flex: 1;
}

.arrow-icon {
  color: var(--el-text-color-secondary);
  font-size: 16px;
}

.credential-files-section {
  padding: 0 20px;
}

.credential-files-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
}

.credential-files-tip {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.credential-files-list {
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.credential-file-item {
  width: 100%;
}

.credential-file-label {
  font-weight: 500;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  margin-bottom: 8px;
}

.files-section {
  padding: 0 20px;
}

.files-tip {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px;
  margin-bottom: 15px;
  background: var(--el-color-info-light-9);
  border-radius: 4px;
  color: var(--el-text-color-regular);
  font-size: 13px;
}

.file-item {
  margin-bottom: 20px;
}

.file-label {
  font-weight: 500;
  color: var(--el-text-color-primary);
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  margin-bottom: 8px;
  padding: 8px 12px;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  border-left: 3px solid var(--el-color-primary);
}
</style>
