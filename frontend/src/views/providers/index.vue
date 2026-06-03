<template>
  <div class="providers-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-cloud" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>
        </symbol>
        <symbol id="icon-key" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="7.5" cy="15.5" r="5.5"/>
          <path d="m21 2-9.6 9.6"/>
          <path d="m15.5 7.5 3 3L22 7l-3-3"/>
        </symbol>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
        </symbol>
        <symbol id="icon-paste" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>
          <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
          <path d="M9 14h6"/>
          <path d="M12 11v6"/>
        </symbol>
        <symbol id="icon-write" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/>
          <path d="M14 2v4a2 2 0 0 0 2 2h4"/>
          <path d="M12 18v-6"/>
          <path d="m9 15 3 3 3-3"/>
        </symbol>
        <symbol id="icon-refresh" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
      </defs>
    </svg>
    
    <!-- Top Level Tabs -->
    <div class="top-tabs">
      <div
        v-for="cli in cliTabs"
        :key="cli.id"
        :class="['tab-item', { active: activeCliType === cli.id }]"
        @click="activeCliType = cli.id"
      >
        {{ cli.label }}
      </div>
    </div>

    <!-- Page Header & Segmented Control -->
    <div class="page-header">
      <div class="header-left">
        <div class="b-segmented">
          <div class="b-seg-btn" :class="{ active: viewMode === 'proxy' }" @click="handleSwitchProxy">中转路由</div>
          <div class="b-seg-btn" :class="{ active: viewMode === 'direct' }" @click="handleSwitchDirect">官方直连</div>
        </div>

        <div v-if="showProfileControls" class="b-segmented">
          <div
            v-for="profile in profileTabs"
            :key="profile.id"
            class="b-seg-btn"
            :class="{ active: activeProfile === profile.id, disabled: !!profileSwitching }"
            @click="handleProfileSelect(profile.id)"
          >
            {{ profile.label }}
          </div>
        </div>
        <el-tooltip
          v-if="showProfileHelp"
          effect="light"
          placement="top"
          :fallback-placements="['bottom', 'top', 'right', 'left']"
          :offset="10"
          :show-after="150"
          :enterable="true"
          popper-class="profile-help-popper"
        >
          <template #content>
            <div class="profile-help-content">
              <div class="tooltip-title">Profile 用法</div>
              <div class="tooltip-item">
                <span>{{ profileUsageText }}</span>
              </div>
              <div class="profile-command-panel">
                <div class="profile-command-header">
                  <strong>{{ profileLabels[activeProfile] }} 启动命令</strong>
                  <button
                    class="profile-command-copy"
                    type="button"
                    :disabled="isCurrentProfileCommandLoading"
                    @click.stop="copyCurrentProfileLaunchCommand"
                    title="复制启动命令"
                  >
                    <svg width="14" height="14"><use href="#icon-copy"/></svg>
                  </button>
                </div>
                <div class="profile-command-text" @click.stop="copyCurrentProfileLaunchCommand">
                  {{ currentProfileLaunchCommand || '正在获取启动命令...' }}
                </div>
              </div>
            </div>
          </template>
          <div class="help-icon-wrapper" @click.stop="copyCurrentProfileLaunchCommand" title="复制当前 Profile 启动命令">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="help-icon">
              <circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </div>
        </el-tooltip>
      </div>
      
      <div v-if="viewMode === 'proxy'" style="display: flex; align-items: center; gap: 8px;">
        <button
          class="action-icon primary"
          @click="showDetectDialog = true"
          title="检测模型可用性"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
          </svg>
        </button>
        <button
          v-if="copiedProvider"
          class="action-icon success"
          :disabled="pasteLoading"
          @click="handlePasteProvider"
          :title="pasteButtonTitle"
        >
          <svg width="18" height="18"><use href="#icon-paste"/></svg>
        </button>
        <button
          class="action-icon primary"
          @click="handleAddProvider"
          title="添加服务商"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 12h14"/><path d="M12 5v14"/>
          </svg>
        </button>
      </div>
      <div v-else>
        <button
          class="action-icon primary"
          @click="showAddCredentialDialog = true"
          title="添加凭证"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 12h14"/><path d="M12 5v14"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- PROXY MODE LIST -->
    <template v-if="viewMode === 'proxy'">
      <div v-if="providerStore.providers.length === 0" v-loading="providerStore.loading" class="list-container">
        <div class="empty-state">
          <svg width="64" height="64" color="var(--color-border)"><use href="#icon-cloud"/></svg>
          <p>暂无服务商</p>
        </div>
      </div>
      <div v-else class="b-card list-container" v-loading="providerStore.loading">
        <div class="scroll-area">
        <draggable
          v-model="providerStore.providers"
          item-key="id"
          handle=".drag-handle"
          @end="handleDragEnd"
        >
          <template #item="{ element, index }">
            <ProviderListItem
              :provider="element"
              :is-last="index === providerStore.providers.length - 1"
              :mode="isProviderDirectMode ? 'direct' : 'route'"
              :unblacklist-text="getUnblacklistTime(element)"
              :toggle-loading="toggleLoadingId === element.id"
              :write-loading="writeProviderLoadingId === element.id"
              @copy="handleCopyProvider"
              @edit="handleEdit"
              @write="handleWriteProviderDirect"
              @reset="handleReset"
              @delete="provider => handleCommand('delete', provider)"
              @toggle="handleToggle"
            />
          </template>
        </draggable>
        </div>
      </div>
    </template>

    <!-- DIRECT MODE -->
    <template v-else>
      <div v-if="credentialStore.credentials.length === 0" v-loading="credentialStore.loading" class="list-container">
        <div class="empty-state">
          <svg width="64" height="64" color="var(--color-border)"><use href="#icon-key"/></svg>
          <p>暂无凭证</p>
        </div>
      </div>
      <div v-else class="b-card list-container" v-loading="credentialStore.loading">
        <div class="scroll-area">
        <draggable
          v-model="credentialStore.credentials"
          item-key="id"
          handle=".drag-handle"
          @end="handleCredentialDragEnd"
        >
          <template #item="{ element, index }">
            <CredentialListItem
              :credential="element"
              :is-last="index === credentialStore.credentials.length - 1"
              :write-loading="writeCredentialLoadingId === element.id"
              @write="handleWriteCredential"
              @edit="handleEditCredential"
              @delete="handleDeleteCredential"
            />
          </template>
        </draggable>
        </div>
      </div>
    </template>

    <ProviderEditModal
      v-model="showDialog"
      :title="editingProvider ? '编辑服务商' : '添加服务商'"
      :form="form"
      :active-cli-type="activeCliType"
      :base-url-placeholder="baseUrlPlaceholder"
      @confirm="handleSave"
      @add-model-map="addModelMap"
      @remove-model-map="removeModelMap"
      @add-model-blacklist="addModelBlacklist"
      @remove-model-blacklist="removeModelBlacklist"
    />

    <CredentialEditModal
      v-model="showCredentialDialog"
      :title="editingCredential ? '编辑凭证' : '添加凭证'"
      :form="credentialForm"
      :active-cli-type="activeCliType"
      @confirm="handleSaveCredential"
      @read-from-cli="handleReadFromCli"
    />

    <ModelDetectionModal
      v-model="showDetectDialog"
      v-model:model="detectModel"
      :providers="detectProviderList"
      :selected-ids="detectSelectedIds"
      :is-all-selected="isAllDetectSelected"
      :loading="detectLoading"
      :results="detectResults"
      @confirm="handleStartDetect"
      @toggle-all="toggleAllDetectProviders"
      @toggle-provider="toggleDetectProvider"
      @copy-response="copyResponseText"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import draggable from 'vuedraggable'
import ProviderListItem from './components/ProviderListItem.vue'
import CredentialListItem from './components/CredentialListItem.vue'
import ProviderEditModal from './components/ProviderEditModal.vue'
import CredentialEditModal from './components/CredentialEditModal.vue'
import ModelDetectionModal from './components/ModelDetectionModal.vue'
import { useProviderStore } from '@/stores/providers'
import { useCredentialStore } from '@/stores/credentials'
import { useUiStore } from '@/stores/ui'
import { useSettingsStore } from '@/stores/settings'
import { credentialsApi } from '@/api/credentials'
import { providersApi } from '@/api/providers'
import { settingsApi } from '@/api/settings'
import { CLI_TABS, PROFILE_CAPABLE_CLI_TYPES } from '@/types/models'
import type { Provider, CliType, CliMode, ProviderProfile, CliProfileSettingsStatus, OfficialCredential, OfficialCredentialCreate, TestProviderResult } from '@/types/models'
import { getReusableModelName, saveReusableModelName } from '@/utils/modelDefaults'

const providerStore = useProviderStore()
const credentialStore = useCredentialStore()
const uiStore = useUiStore()
const settingsStore = useSettingsStore()

const cliTabs = CLI_TABS

const profileTabs: { id: ProviderProfile; label: string }[] = [
  { id: 'default', label: '默认' },
  { id: 'profile1', label: 'Profile 1' },
  { id: 'profile2', label: 'Profile 2' },
  { id: 'profile3', label: 'Profile 3' }
]

const activeCliType = computed({
  get: () => uiStore.providersActiveCliType,
  set: (val) => uiStore.setProvidersActiveCliType(val)
})

const activeProfile = computed({
  get: () => uiStore.providersActiveProfile,
  set: (val) => uiStore.setProvidersActiveProfile(val)
})

type ViewMode = 'proxy' | 'direct'

const viewModes = ref<Record<CliType, ViewMode>>({
  claude_code: 'proxy',
  codex: 'proxy',
  gemini: 'proxy'
})
const viewMode = computed<ViewMode>({
  get: () => activeCliType.value === 'claude_code' ? 'proxy' : viewModes.value[activeCliType.value],
  set: (mode) => {
    if (activeCliType.value === 'claude_code') {
      viewModes.value.claude_code = 'proxy'
      if (mode === 'direct') notify('Claude Code 暂未实现官方直连功能', 'warning')
      return
    }
    viewModes.value[activeCliType.value] = mode
  }
})
const currentCliMode = computed<CliMode>(() =>
  settingsStore.settings?.cli_settings?.[activeCliType.value]?.cli_mode ?? 'proxy_route'
)
const isProviderDirectMode = computed(() => currentCliMode.value === 'provider_direct')
const isProxyRouteMode = computed(() => currentCliMode.value === 'proxy_route')
const profileCapableCliTypes = PROFILE_CAPABLE_CLI_TYPES
const showProfileControls = computed(() => viewMode.value === 'proxy' && profileCapableCliTypes.includes(activeCliType.value))
const showProfileHelp = computed(() => showProfileControls.value)
const currentProviderProfile = computed<ProviderProfile>(() =>
  showProfileControls.value ? activeProfile.value : 'default'
)
const profileSwitching = ref<ProviderProfile | null>(null)
let testResultListener: (() => void) | null = null

const profileLabels: Record<ProviderProfile, string> = {
  default: '默认',
  profile1: 'Profile 1',
  profile2: 'Profile 2',
  profile3: 'Profile 3'
}

const profileSettingsStatusMap = ref<Partial<Record<string, CliProfileSettingsStatus>>>({})
const profileCommandLoading = ref<string | null>(null)
let profileCommandRequestId = 0

function profileStatusKey(cliType: CliType, profile: ProviderProfile) {
  return `${cliType}:${profile}`
}

const currentProfileSettingsStatus = computed(() =>
  profileSettingsStatusMap.value[profileStatusKey(activeCliType.value, activeProfile.value)]
)
const currentProfileLaunchCommand = computed(() => currentProfileSettingsStatus.value?.launch_command || '')
const isCurrentProfileCommandLoading = computed(() =>
  profileCommandLoading.value === profileStatusKey(activeCliType.value, activeProfile.value)
)
const profileUsageText = computed(() => {
  if (isProviderDirectMode.value) {
    return '中转直连会将服务商写入当前 Profile 对应配置文件，通过对应启动命令启动的 Agent 会直连该服务商'
  }
  return '切换到 Profile 时会自动生成对应配置文件，通过对应启动命令启动的 Agent 会路由到对应 Profile 配置的服务商'
})

function handleSwitchProxy() {
  if (viewMode.value === 'proxy') return
  viewMode.value = 'proxy'
}

function handleSwitchDirect() {
  if (viewMode.value === 'direct') return
  viewMode.value = 'direct'
  credentialStore.fetchCredentials(activeCliType.value as CliType)
}

const showAddDialog = ref(false)
const showAddCredentialDialog = ref(false)
const editingProvider = ref<Provider | null>(null)
const editingCredential = ref<OfficialCredential | null>(null)

const showDialog = computed({
  get: () => showAddDialog.value || !!editingProvider.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingProvider.value = null
      resetForm()
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

interface FormModelMap { source_model: string; target_model: string; enabled: boolean }
interface FormModelBlacklist { model_pattern: string }
interface ProviderDraft {
  name: string
  base_url: string
  api_key: string
  enabled: boolean
  failure_threshold: number
  blacklist_minutes: number
  custom_useragent: string
  model_maps: FormModelMap[]
  model_blacklist: FormModelBlacklist[]
}
interface ProviderTogglePayload { provider: Provider; enabled: boolean }

const toggleLoadingId = ref<number | null>(null)
const writeProviderLoadingId = ref<number | null>(null)
const writeCredentialLoadingId = ref<number | null>(null)

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
const copiedProvider = ref<ProviderDraft | null>(null)
const pasteLoading = ref(false)

const credentialForm = ref({
  name: '',
  claude_settings: '',
  codex_auth: '',
  gemini_oauth: '',
  gemini_accounts: ''
})

const baseUrlPlaceholder = computed(() => {
  if (activeCliType.value === 'codex') return 'https://api.example.com/v1'
  return 'https://api.example.com'
})

const pasteButtonTitle = computed(() => {
  if (!copiedProvider.value) return '粘贴服务商'
  return `粘贴服务商：${copiedProvider.value.name}`
})

function resetForm() {
  form.value = {
    name: '', base_url: '', api_key: '', failure_threshold: 3, blacklist_minutes: 10,
    custom_useragent: '', model_maps: [], model_blacklist: []
  }
}
function resetCredentialForm() {
  credentialForm.value = { name: '', claude_settings: '', codex_auth: '', gemini_oauth: '', gemini_accounts: '' }
}

function cloneProviderDraft(draft: ProviderDraft): ProviderDraft {
  return {
    ...draft,
    model_maps: draft.model_maps.map(m => ({ ...m })),
    model_blacklist: draft.model_blacklist.map(b => ({ ...b }))
  }
}

function createProviderDraft(provider: Provider): ProviderDraft {
  return {
    name: provider.name,
    base_url: provider.base_url,
    api_key: provider.api_key,
    enabled: provider.enabled,
    failure_threshold: provider.failure_threshold,
    blacklist_minutes: provider.blacklist_minutes,
    custom_useragent: provider.custom_useragent || '',
    model_maps: provider.model_maps.map(({ source_model, target_model, enabled }) => ({ source_model, target_model, enabled })),
    model_blacklist: provider.model_blacklist.map(({ model_pattern }) => ({ model_pattern }))
  }
}

function makeUniqueProviderName(name: string): string {
  const trimmedName = name.trim() || '未命名服务商'
  const existingNames = new Set(providerStore.providers.map(p => p.name.trim().toLowerCase()))
  if (!existingNames.has(trimmedName.toLowerCase())) return trimmedName

  const baseName = `${trimmedName} 副本`
  let candidate = baseName
  let index = 2
  while (existingNames.has(candidate.toLowerCase())) {
    candidate = `${baseName} ${index}`
    index += 1
  }
  return candidate
}

// ==================== Model Detection ====================
const showDetectDialog = ref(false)
const detectLoading = ref(false)
const detectModel = ref('')
const detectSelectedIds = ref<number[]>([])
const detectResults = ref<TestProviderResult[]>([])

const detectProviderList = computed(() =>
  isProviderDirectMode.value ? providerStore.providers : providerStore.providers.filter(p => p.enabled)
)

const isAllDetectSelected = computed(() =>
  detectProviderList.value.length > 0 && detectSelectedIds.value.length === detectProviderList.value.length
)

function toggleDetectProvider(id: number) {
  const idx = detectSelectedIds.value.indexOf(id)
  if (idx >= 0) detectSelectedIds.value.splice(idx, 1)
  else detectSelectedIds.value.push(id)
}

function toggleAllDetectProviders() {
  if (isAllDetectSelected.value) {
    detectSelectedIds.value = []
  } else {
    detectSelectedIds.value = detectProviderList.value.map(p => p.id)
  }
}

watch(showDetectDialog, (open) => {
  if (open) {
    detectModel.value = getReusableModelName(activeCliType.value)
    detectSelectedIds.value = detectProviderList.value.map(p => p.id)
    detectResults.value = []
    detectLoading.value = false
  } else {
    // 关闭对话框时清理监听器
    if (testResultListener) {
      testResultListener()
      testResultListener = null
    }
  }
})

async function handleStartDetect() {
  if (!detectModel.value.trim()) {
    notify('请输入检测模型名称', 'error')
    return
  }
  if (detectSelectedIds.value.length === 0) {
    notify('请至少选择一个服务商', 'error')
    return
  }

  saveReusableModelName(activeCliType.value, detectModel.value)

  detectResults.value = detectSelectedIds.value.map(id => {
    const p = providerStore.providers.find(x => x.id === id)
    return {
      provider_id: id,
      provider_name: p?.name || 'Unknown',
      actual_model: '...',
      status_code: null,
      elapsed_ms: 0,
      response_text: '',
      request_url: '',
      request_headers: '',
      request_body: '',
      response_headers: '',
      response_body: '',
    }
  })
  detectLoading.value = true

  // 清理之前的监听器
  if (testResultListener) {
    testResultListener()
    testResultListener = null
  }

  // 监听测试结果事件
  testResultListener = await providersApi.listenTestResults((result) => {
    const idx = detectResults.value.findIndex(r => r.provider_id === result.provider_id)
    if (idx >= 0) {
      detectResults.value[idx] = result
    }
    // 检查是否所有结果都已返回（response_text 非空表示有结果）
    if (detectResults.value.every(r => r.response_text !== '')) {
      detectLoading.value = false
    }
  })

  try {
    await providersApi.startTestModels(detectModel.value.trim(), detectSelectedIds.value)
  } catch (e: any) {
    notify(getErrorMessage(e, '检测失败'), 'error')
    detectLoading.value = false
  }
}

async function copyResponseText(text: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    notify('响应已复制到剪贴板')
  } catch {
    notify('复制失败', 'error')
  }
}

function cacheProfileSettingsStatus(cliType: CliType, status: CliProfileSettingsStatus) {
  profileSettingsStatusMap.value = {
    ...profileSettingsStatusMap.value,
    [profileStatusKey(cliType, status.profile)]: status
  }
}

async function loadProfileSettingsStatus(cliType: CliType, profile: ProviderProfile, silent = false): Promise<CliProfileSettingsStatus | null> {
  const requestId = ++profileCommandRequestId
  const loadingKey = profileStatusKey(cliType, profile)
  profileCommandLoading.value = loadingKey
  try {
    const { data } = cliType === 'codex'
      ? await settingsApi.getCodexProfileSettingsStatus(profile)
      : await settingsApi.getClaudeProfileSettingsStatus(profile)
    cacheProfileSettingsStatus(cliType, data)
    return data
  } catch (e: any) {
    if (!silent) notify(getErrorMessage(e, '获取启动命令失败'), 'error')
    return null
  } finally {
    if (requestId === profileCommandRequestId) {
      profileCommandLoading.value = null
    }
  }
}

async function copyCurrentProfileLaunchCommand() {
  if (!showProfileControls.value) return

  const cliType = activeCliType.value
  const profile = activeProfile.value
  const key = profileStatusKey(cliType, profile)
  const status = profileSettingsStatusMap.value[key] || await loadProfileSettingsStatus(cliType, profile)
  const command = status?.launch_command
  if (!command) return

  try {
    await navigator.clipboard.writeText(command)
    notify(`已复制 ${profileLabels[profile]} 启动命令`)
  } catch {
    notify('复制失败', 'error')
  }
}

function addModelMap() { form.value.model_maps.push({ source_model: '', target_model: '', enabled: true }) }
function removeModelMap(index: number) { form.value.model_maps.splice(index, 1) }
function addModelBlacklist() { form.value.model_blacklist.push({ model_pattern: '' }) }
function removeModelBlacklist(index: number) { form.value.model_blacklist.splice(index, 1) }

async function ensureProfileReady(profile: ProviderProfile): Promise<boolean> {
  const cliType = activeCliType.value
  if (!profileCapableCliTypes.includes(cliType) || viewMode.value !== 'proxy' || !isProxyRouteMode.value) {
    return true
  }
  if (profile === 'default') {
    return true
  }

  profileSwitching.value = profile
  try {
    const { data: status } = cliType === 'codex'
      ? await settingsApi.getCodexProfileSettingsStatus(profile)
      : await settingsApi.getClaudeProfileSettingsStatus(profile)
    cacheProfileSettingsStatus(cliType, status)
    if (status.uses_gateway) return true

    const { data: ensured } = cliType === 'codex'
      ? await settingsApi.ensureCodexProfileSettings(profile)
      : await settingsApi.ensureClaudeProfileSettings(profile)
    cacheProfileSettingsStatus(cliType, ensured)
    if (!ensured.uses_gateway) {
      notify(`写入后仍未检测到有效配置：${ensured.path}`, 'error')
      return false
    }

    notify(`已写入 ${ensured.path}`)
    return true
  } catch (e: any) {
    notify(getErrorMessage(e, '写入 Profile 配置失败'), 'error')
    return false
  } finally {
    profileSwitching.value = null
  }
}

async function handleProfileSelect(profile: ProviderProfile) {
  if (profile === activeProfile.value || profileSwitching.value) return

  const ok = await ensureProfileReady(profile)
  if (!ok) return

  activeProfile.value = profile
}

async function ensureCurrentProfileOrFallback(): Promise<ProviderProfile> {
  if (!showProfileControls.value) return 'default'

  const profile = activeProfile.value
  if (await ensureProfileReady(profile)) return profile

  activeProfile.value = 'default'
  return 'default'
}

// Listen for tab changes
watch(() => activeCliType.value, async (cliType) => {
  const profile = await ensureCurrentProfileOrFallback()
  const key = providerStore.getCacheKey(cliType as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(cliType as CliType, profile)
  }
  credentialStore.fetchCredentials(cliType as CliType)
})

watch(() => activeProfile.value, (profile) => {
  if (!showProfileControls.value) return
  const key = providerStore.getCacheKey(activeCliType.value as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(activeCliType.value as CliType, profile)
  }
})

watch(() => viewMode.value, async (mode) => {
  if (mode !== 'proxy') return
  const profile = await ensureCurrentProfileOrFallback()
  const key = providerStore.getCacheKey(activeCliType.value as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(activeCliType.value as CliType, profile)
  }
})

watch(() => currentCliMode.value, async () => {
  if (viewMode.value === 'proxy') {
    const profile = await ensureCurrentProfileOrFallback()
    providerStore.fetchProviders(activeCliType.value as CliType, profile)
  } else {
    credentialStore.fetchCredentials(activeCliType.value as CliType)
  }
})

watch([showProfileHelp, activeCliType, activeProfile], ([visible, cliType, profile]) => {
  if (!visible || profileSettingsStatusMap.value[profileStatusKey(cliType, profile)]) return
  loadProfileSettingsStatus(cliType, profile, true)
}, { immediate: true })

function handleAddProvider() {
  editingProvider.value = null
  resetForm()
  showAddDialog.value = true
}

function handleCopyProvider(provider: Provider) {
  copiedProvider.value = createProviderDraft(provider)
  notify(`已复制服务商：${provider.name}`)
}

async function handlePasteProvider() {
  if (!copiedProvider.value || pasteLoading.value) return

  const targetCliType = activeCliType.value as CliType
  const targetProfile = currentProviderProfile.value
  pasteLoading.value = true
  try {
    await providerStore.fetchProviders(targetCliType, targetProfile)
    const draft = cloneProviderDraft(copiedProvider.value)
    const data = {
      cli_type: targetCliType,
      profile: targetProfile,
      ...draft,
      name: makeUniqueProviderName(draft.name),
      model_maps: draft.model_maps.filter(m => m.source_model && m.target_model),
      model_blacklist: draft.model_blacklist.filter(b => b.model_pattern)
    }
    await providerStore.createProvider(data)
    notify(`已粘贴服务商：${data.name}`)
  } catch (e: any) {
    notify(getErrorMessage(e, '粘贴失败'), 'error')
  } finally {
    pasteLoading.value = false
  }
}

function handleEdit(provider: Provider) {
  editingProvider.value = provider
  form.value = {
    name: provider.name, base_url: provider.base_url, api_key: provider.api_key,
    failure_threshold: provider.failure_threshold, blacklist_minutes: provider.blacklist_minutes,
    custom_useragent: provider.custom_useragent || '',
    model_maps: provider.model_maps.map(m => ({ ...m })),
    model_blacklist: provider.model_blacklist.map(b => ({ ...b }))
  }
}

async function handleSave() {
  if (!form.value.name.trim() || !form.value.base_url.trim() || !form.value.api_key.trim()) {
    notify('请填写完整的必填项', 'error')
    return
  }
  const data = {
    cli_type: activeCliType.value,
    profile: currentProviderProfile.value,
    ...form.value,
    model_maps: form.value.model_maps.filter(m => m.source_model && m.target_model),
    model_blacklist: form.value.model_blacklist.filter(b => b.model_pattern)
  }
  
  try {
    if (editingProvider.value) {
      await providerStore.updateProvider(editingProvider.value.id, data)
      notify('更新成功')
    } else {
      await providerStore.createProvider(data as any)
      notify('添加成功')
    }
    showDialog.value = false
    resetForm()
    providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}

async function handleToggle({ provider, enabled }: ProviderTogglePayload) {
  if (isProviderDirectMode.value) return
  toggleLoadingId.value = provider.id
  try {
    await providerStore.updateProvider(provider.id, { enabled })
    provider.enabled = enabled
    notify(enabled ? '已启用' : '已停用')
  } catch (e: any) {
    notify(getErrorMessage(e, '切换失败'), 'error')
  } finally {
    toggleLoadingId.value = null
  }
}

async function handleWriteProviderDirect(provider: Provider) {
  writeProviderLoadingId.value = provider.id
  try {
    await providersApi.writeDirectConfig(provider.id)
    await settingsStore.fetchSettings()
    await providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
    notify(`已写入服务商：${provider.name}`)
  } catch (e: any) {
    notify(getErrorMessage(e, '写入失败'), 'error')
  } finally {
    writeProviderLoadingId.value = null
  }
}

async function handleDragEnd() {
  const ids = providerStore.providers.map(p => p.id)
  await providerStore.reorderProviders(ids)
  notify('排序已保存')
}

async function handleReset(provider: Provider) {
  await providerStore.resetFailures(provider.id)
  if (provider.is_blacklisted) {
    await providerStore.unblacklist(provider.id)
  }
  notify('重置成功')
}

async function handleCommand(command: string, provider: Provider) {
  if (command === 'reset') {
    await providerStore.resetFailures(provider.id)
    notify('已重置')
  } else if (command === 'unblacklist') {
    await providerStore.unblacklist(provider.id)
    notify('已解除拉黑')
  } else if (command === 'delete') {
    try {
      await confirm('确定删除该服务商？', '确认')
      await providerStore.deleteProvider(provider.id)
      notify('已删除')
    } catch (e) {
      if (e !== 'cancel') notify(getErrorMessage(e, '删除失败'), 'error')
    }
  }
}

function handleEditCredential(credential: OfficialCredential) {
  editingCredential.value = credential
  credentialForm.value.name = credential.name
  try {
    const filesData = JSON.parse(credential.credential_json)
    if (Array.isArray(filesData)) {
      filesData.forEach(file => {
        const path = file.path || ''; const content = file.content || ''
        if (path.includes('.claude') && path.includes('settings.json')) credentialForm.value.claude_settings = content
        else if (path.includes('auth.json')) credentialForm.value.codex_auth = content
        else if (path.includes('oauth_creds.json')) credentialForm.value.gemini_oauth = content
        else if (path.includes('google_accounts.json')) credentialForm.value.gemini_accounts = content
      })
    }
  } catch (e) {}
}

async function handleDeleteCredential(credential: OfficialCredential) {
  try {
    await confirm('确定删除该凭证？', '确认')
    await credentialStore.deleteCredential(credential.id)
    notify('已删除')
  } catch (e) {
    if (e !== 'cancel') notify(getErrorMessage(e, '删除失败'), 'error')
  }
}

async function handleWriteCredential(credential: OfficialCredential) {
  writeCredentialLoadingId.value = credential.id
  try {
    await credentialsApi.writeConfig(credential.id)
    await settingsStore.fetchSettings()
    await credentialStore.fetchCredentials(activeCliType.value as CliType)
    notify(`已写入凭证：${credential.name}`)
  } catch (e: any) {
    notify(getErrorMessage(e, '写入失败'), 'error')
  } finally {
    writeCredentialLoadingId.value = null
  }
}

async function handleReadFromCli() {
  try {
    const { data } = await credentialsApi.readCliCredential(activeCliType.value as CliType)
    try {
      const filesData = JSON.parse(data)
      if (Array.isArray(filesData)) {
        filesData.forEach(file => {
          const path = file.path || ''; const content = file.content || ''
          if (path.includes('.claude') && path.includes('settings.json')) credentialForm.value.claude_settings = content
          else if (path.includes('auth.json')) credentialForm.value.codex_auth = content
          else if (path.includes('oauth_creds.json')) credentialForm.value.gemini_oauth = content
          else if (path.includes('google_accounts.json')) credentialForm.value.gemini_accounts = content
        })
      }
    } catch {}
    notify('读取成功')
  } catch (e: any) {
    notify(getErrorMessage(e, '读取失败'), 'error')
  }
}

async function handleSaveCredential() {
  if (!credentialForm.value.name) {
    notify('请输入凭证名称', 'error')
    return
  }
  const files: Array<{ path: string; content: string }> = []
  if (activeCliType.value === 'claude_code') {
    if (credentialForm.value.claude_settings) files.push({ path: '~/.claude/settings.json', content: credentialForm.value.claude_settings })
  } else if (activeCliType.value === 'codex') {
    if (credentialForm.value.codex_auth) files.push({ path: '~/.codex/auth.json', content: credentialForm.value.codex_auth })
  } else if (activeCliType.value === 'gemini') {
    if (credentialForm.value.gemini_oauth) files.push({ path: '~/.gemini/oauth_creds.json', content: credentialForm.value.gemini_oauth })
    if (credentialForm.value.gemini_accounts) files.push({ path: '~/.gemini/google_accounts.json', content: credentialForm.value.gemini_accounts })
  }
  if (files.length === 0) {
    notify('请至少填写一个文件内容', 'error')
    return
  }

  const data: OfficialCredentialCreate = {
    cli_type: activeCliType.value as CliType,
    name: credentialForm.value.name.trim(),
    credential_json: JSON.stringify(files)
  }

  try {
    if (editingCredential.value) {
      await credentialStore.updateCredential(editingCredential.value.id, { name: data.name, credential_json: data.credential_json })
      notify('更新成功')
    } else {
      await credentialStore.createCredential(data)
      notify('添加成功')
    }
    showCredentialDialog.value = false
    resetCredentialForm()
    credentialStore.fetchCredentials(activeCliType.value as CliType)
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}

async function handleCredentialDragEnd() {
  const ids = credentialStore.credentials.map(c => c.id)
  await credentialStore.reorderCredentials(ids)
  notify('排序已保存')
}

const now = ref(Date.now())
let timer: ReturnType<typeof setInterval> | null = null

function handleVisibilityChange() {
  if (document.visibilityState === 'visible') {
    now.value = Date.now()
  }
}

function getUnblacklistTime(provider: Provider): string {
  if (!provider.is_blacklisted || !provider.blacklisted_until) return '已拉黑'
  const diffSeconds = provider.blacklisted_until - (now.value / 1000)
  if (diffSeconds <= 0) return '已解除'
  const mins = Math.floor(diffSeconds / 60)
  return mins === 0 ? `${Math.ceil(diffSeconds)}秒后解除` : `${mins}分${Math.ceil(diffSeconds % 60)}秒后解除`
}

onMounted(async () => {
  await settingsStore.fetchSettings()
  const profile = await ensureCurrentProfileOrFallback()
  providerStore.fetchProviders(activeCliType.value as CliType, profile)
  credentialStore.fetchCredentials(activeCliType.value as CliType)

  document.addEventListener('visibilitychange', handleVisibilityChange)

  // 每秒更新一次时间，触发倒计时重绘（后台标签页跳过）
  timer = setInterval(() => {
    if (document.visibilityState !== 'visible') return
    const oldNow = now.value
    now.value = Date.now()

    // 检查是否有服务商的拉黑时间刚刚到期
    const hasExpired = providerStore.providers.some(p => {
      if (p.is_blacklisted && p.blacklisted_until) {
        return p.blacklisted_until > (oldNow / 1000) && p.blacklisted_until <= (now.value / 1000)
      }
      return false
    })

    if (hasExpired) {
      providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
    }
  }, 1000)
})

onUnmounted(() => {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  if (testResultListener) {
    testResultListener()
    testResultListener = null
  }
})
</script>

<style scoped>
.providers-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid var(--color-border-light); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }

.header-left { display: flex; align-items: center; gap: 12px; min-width: 0; }
:global(.profile-help-popper.el-popper) {
  border-radius: 12px;
  padding: 16px;
  box-shadow: 0 8px 24px var(--color-shadow-lg);
}

.profile-help-content {
  width: 320px;
}

.profile-command-panel {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--color-border-light);
}

.profile-command-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 6px;
  font-size: var(--fs-12);
  color: var(--color-text-dark);
}

.profile-command-copy {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-bg);
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
}

.profile-command-copy:hover:not(:disabled) {
  background: var(--color-bg-page);
  color: var(--color-primary);
  border-color: var(--color-primary);
}

.profile-command-copy:disabled {
  cursor: wait;
  opacity: 0.6;
}

.profile-command-text {
  padding: 8px;
  border: 1px solid var(--color-border-light);
  border-radius: 6px;
  background: var(--color-bg-page);
  color: var(--color-text-secondary);
  font-family: ui-monospace, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: var(--fs-12);
  line-height: 1.45;
  word-break: break-all;
  cursor: pointer;
  user-select: text;
}


.b-card { background: var(--color-bg); border-radius: 16px; box-shadow: 0 4px 12px var(--color-shadow); margin-bottom: 24px; border: none; overflow: hidden; }
</style>
