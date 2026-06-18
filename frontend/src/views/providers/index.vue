<template>
  <div class="prov-page">
    <svg style="display:none">
      <defs>
        <symbol id="v2i-write" viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></symbol>
        <symbol id="v2i-play" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </symbol>
        <symbol id="v2i-check" viewBox="0 0 24 24">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <polyline points="22 4 12 14.01 9 11.01" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </symbol>
        <symbol id="v2i-copy" viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></symbol>
        <symbol id="v2i-edit" viewBox="0 0 24 24"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></symbol>
        <symbol id="v2i-refresh" viewBox="0 0 24 24"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></symbol>
        <symbol id="v2i-trash" viewBox="0 0 24 24"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/></symbol>
      </defs>
    </svg>

    <div class="v2-tabs prov-clitabs">
      <div v-for="c in cliTabs" :key="c.id" class="v2-tab" :class="{ active: activeCliType === c.id }" @click="activeCliType = c.id">
        <span class="tab-label-text">{{ c.label }}</span>
      </div>
    </div>

    <div class="v2-card prov-shell">
    <div class="prov-toolbar">
      <div class="toolbar-left">
        <div class="v2-seg">
          <div class="v2-seg-slider" :style="{ transform: `translateX(${viewMode === 'proxy' ? 0 : 1}00%)`, width: 'calc((100% - 8px) / 2)' }"></div>
          <button class="v2-seg-btn" :class="{ active: viewMode === 'proxy' }" type="button" @click="handleSwitchProxy">{{ providerModeLabel }}</button>
          <button class="v2-seg-btn" :class="{ active: viewMode === 'direct' }" type="button" @click="handleSwitchDirect">官方直连</button>
        </div>

        <div v-if="showProfileControls" class="v2-seg profile-tabs">
          <div class="v2-seg-slider" :style="profileSliderStyle"></div>
          <el-tooltip
            v-for="profile in profileTabs"
            :key="profile.name"
            :ref="(el) => setProfileErrorTooltipRef(profile.name, el)"
            :visible="editingProfileName === profile.name && !!profileRenameError"
            :disabled="editingProfileName !== profile.name || !profileRenameError"
            effect="light"
            placement="bottom"
            popper-class="v2-profile-error-pop v2-scope"
          >
            <template #content>{{ profileRenameError }}</template>
            <button
              class="v2-seg-btn profile-tab-btn"
              :class="{ active: activeProfile === profile.name, disabled: !!profileSwitching }"
              type="button"
              @click="handleProfileSelect(profile.name)"
              @dblclick.stop="startProfileRename(profile)"
            >
              <input
                v-if="editingProfileName === profile.name"
                ref="profileRenameInput"
                v-model="profileRenameDraft"
                class="profile-rename-input"
                @click.stop
                @keydown.enter.prevent="commitProfileRename(profile)"
                @keydown.esc.prevent="cancelProfileRename"
                @blur="commitProfileRename(profile)"
              >
              <span v-if="editingProfileName !== profile.name" class="profile-tab-label">{{ profileDisplayLabel(profile) }}</span>
              <button
                v-if="activeProfile === profile.name && !profile.is_default && editingProfileName !== profile.name"
                class="profile-tab-delete"
                type="button"
                aria-label="删除 Profile"
                :disabled="profileLoading"
                @click.stop="handleDeleteProfile(profile)"
              >
                ×
              </button>
            </button>
          </el-tooltip>
          <span v-if="showProfileAddTab" class="profile-add-cell">
            <button
              class="v2-seg-btn profile-add-tab"
              type="button"
              aria-label="添加 Profile"
              :disabled="profileLoading || !canCreateProfile"
              @click.stop="handleAddProfile"
            >
              +
            </button>
          </span>
        </div>

        <el-tooltip
          v-if="showProfileHelp"
          effect="light"
          placement="top"
          :show-after="150"
          :enterable="true"
          popper-class="v2-profile-pop v2-scope"
        >
          <template #content>
            <div class="profile-help">
              <div class="tooltip-title">Profile 用法</div>
              <div class="tooltip-item"><span>{{ profileUsageText }}</span></div>
              <div class="profile-cmd">
                <div class="profile-cmd-head">
                  <strong>{{ currentProfileLabel }} 启动命令</strong>
                  <el-tooltip content="复制启动命令" placement="top" effect="light" :show-after="250">
                    <button class="profile-cmd-copy" type="button" :disabled="isCurrentProfileCommandLoading" @click.stop="copyCurrentProfileLaunchCommand">
                      <svg width="14" height="14"><use href="#v2i-copy"/></svg>
                    </button>
                  </el-tooltip>
                </div>
                <div class="profile-cmd-text mono" @click.stop="copyCurrentProfileLaunchCommand">{{ currentProfileLaunchCommand || '正在获取启动命令...' }}</div>
              </div>
            </div>
          </template>
          <span class="v2-help" @click.stop="copyCurrentProfileLaunchCommand">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          </span>
        </el-tooltip>
      </div>

      <div class="toolbar-right">
        <template v-if="viewMode === 'proxy'">
          <button class="v2-btn v2-btn-sm v2-btn-ghost" @click="showDetectDialog = true">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>
            检测
          </button>
          <button v-if="copiedProvider" class="v2-btn v2-btn-sm v2-btn-ghost" :disabled="pasteLoading" @click="handlePasteProvider">
            <svg width="16" height="16"><use href="#v2i-copy"/></svg>
            粘贴
          </button>
          <button class="v2-btn v2-btn-sm v2-btn-primary" @click="handleAddProvider">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
            添加
          </button>
        </template>
        <button v-else class="v2-btn v2-btn-sm v2-btn-primary" @click="showAddCredentialDialog = true">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
          添加
        </button>
      </div>
    </div>

    <template v-if="viewMode === 'proxy'">
      <V2Empty v-if="providerStore.providers.length === 0" v-loading="providerStore.loading" title="还没有服务商" description="添加服务商，网关会自动路由、负载均衡与故障转移">
        <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/></svg></template>
      </V2Empty>
      <div v-else class="prov-body" v-loading="providerStore.loading">
        <div class="pt-head">
          <div v-if="isProviderDirectMode" class="pt-cols m-direct">
            <div>启用</div>
            <div>服务商</div>
            <div class="pt-col-endpoint">端点</div>
            <div>状态</div>
            <div>操作</div>
          </div>
          <div v-else class="pt-cols m-route">
            <div>启用</div>
            <div>服务商</div>
            <div class="pt-col-endpoint">端点</div>
            <div>状态</div>
            <el-tooltip content="连续失败次数 / 熔断阈值" placement="top" effect="light" :show-after="250">
              <div>容错</div>
            </el-tooltip>
            <div class="pt-col-map">模型映射</div>
            <div>操作</div>
          </div>
        </div>
        <draggable v-model="providerStore.providers" item-key="id" handle=".pt-drag" @end="handleDragEnd">
          <template #item="{ element }">
            <ProviderRow
              :provider="element"
              :mode="isProviderDirectMode ? 'direct' : 'route'"
              :unblacklist-text="getUnblacklistTime(element)"
              :toggle-loading="toggleLoadingId === element.id"
              :write-loading="writeProviderLoadingId === element.id"
              @copy="handleCopyProvider"
              @edit="handleEdit"
              @write="handleWriteProviderDirect"
              @reset="handleReset"
              @delete="(provider) => handleCommand('delete', provider)"
              @toggle="handleToggle"
            />
          </template>
        </draggable>
      </div>
    </template>

    <template v-else>
      <V2Empty v-if="credentialStore.credentials.length === 0" v-loading="credentialStore.loading" title="还没有凭证" description="添加官方账号凭证，可快速写入对应 Agent 配置">
        <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/></svg></template>
      </V2Empty>
      <div v-else class="prov-body" v-loading="credentialStore.loading">
        <div class="pt-head">
          <div class="pt-cols m-cred">
            <div>启用</div>
            <div>凭证</div>
            <div>信息</div>
            <div>状态</div>
            <div>操作</div>
          </div>
        </div>
        <draggable v-model="credentialStore.credentials" item-key="id" handle=".pt-drag" @end="handleCredentialDragEnd">
          <template #item="{ element }">
            <CredentialRow
              :credential="element"
              :write-loading="writeCredentialLoadingId === element.id"
              @write="handleWriteCredential"
              @edit="handleEditCredential"
              @delete="handleDeleteCredential"
            />
          </template>
        </draggable>
      </div>
    </template>
    </div>

    <ProviderDrawer
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
    <CredentialDrawer
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
      v-model:test-text="detectTestText"
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
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import draggable from 'vuedraggable'
import ProviderRow from './components/ProviderRow.vue'
import CredentialRow from './components/CredentialRow.vue'
import ProviderDrawer from './components/ProviderDrawer.vue'
import CredentialDrawer from './components/CredentialDrawer.vue'
import ModelDetectionModal from './components/ModelDetectionModal.vue'
import V2Empty from '@/components/V2Empty.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { useProviderStore } from '@/stores/providers'
import { useCredentialStore } from '@/stores/credentials'
import { useUiStore } from '@/stores/ui'
import { useSettingsStore } from '@/stores/settings'
import { credentialsApi } from '@/api/credentials'
import { providersApi } from '@/api/providers'
import { settingsApi } from '@/api/settings'
import { CLI_TABS, PROFILE_CAPABLE_CLI_TYPES } from '@/types/models'
import type { Provider, CliType, CliMode, ProviderProfile, ProviderProfileItem, CliProfileSettingsStatus, OfficialCredential, OfficialCredentialCreate, TestProviderResult } from '@/types/models'
import { getReusableModelName, saveReusableModelName, getReusableTestText, saveReusableTestText } from '@/utils/modelDefaults'

const providerStore = useProviderStore()
const credentialStore = useCredentialStore()
const uiStore = useUiStore()
const settingsStore = useSettingsStore()

const cliTabs = CLI_TABS

const profileTabs = ref<ProviderProfileItem[]>([
  { cli_type: 'claude_code', name: 'default', label: '默认', is_default: true, sort_order: 0 }
])
const profileLoading = ref(false)
const editingProfileName = ref<ProviderProfile | null>(null)
const profileRenameDraft = ref('')
const profileRenameError = ref('')
const profileRenameInput = ref<HTMLInputElement | HTMLInputElement[] | null>(null)
const profileErrorTooltipRefs = new Map<ProviderProfile, { updatePopper?: () => void }>()
const draftProfileName: ProviderProfile = '__draft_profile__:new'
const profileMaxCustomCount = 4
const profileNameMaxLength = 10
const profileNameRuleText = `仅支持英文、数字、空格、下划线和短横线，最多 ${profileNameMaxLength} 个字符`

const activeCliType = computed({
  get: () => uiStore.providersActiveCliType,
  set: (val) => uiStore.setProvidersActiveCliType(val)
})
const activeProfile = computed({
  get: () => uiStore.providersActiveProfile,
  set: (val) => uiStore.setProvidersActiveProfile(val)
})

type ViewMode = 'proxy' | 'direct'
const currentCliMode = computed<CliMode>(() => settingsStore.settings?.cli_settings?.[activeCliType.value]?.cli_mode ?? 'disabled')
const isProviderDirectMode = computed(() => currentCliMode.value === 'provider_direct')
const isProxyRouteMode = computed(() => currentCliMode.value === 'proxy_route')
const isDisabledMode = computed(() => currentCliMode.value === 'disabled')
const viewModes = ref<Record<CliType, ViewMode>>({ claude_code: 'proxy', codex: 'proxy', gemini: 'proxy' })
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
const providerModeLabel = computed(() => isDisabledMode.value ? '停用' : isProviderDirectMode.value ? '中转直连' : '中转路由')
const profileCapableCliTypes = PROFILE_CAPABLE_CLI_TYPES
const showProfileControls = computed(() => viewMode.value === 'proxy' && profileCapableCliTypes.includes(activeCliType.value))
const showProfileHelp = computed(() => showProfileControls.value)
const currentProviderProfile = computed<ProviderProfile>(() => showProfileControls.value ? activeProfile.value : 'default')
const profileSwitching = ref<ProviderProfile | null>(null)
let testResultListener: (() => void) | null = null

const currentProfileLabel = computed(() =>
  profileDisplayLabel(profileTabs.value.find(profile => profile.name === activeProfile.value)) || activeProfile.value
)
const profileTabWidth = 120
const profileSliderStyle = computed(() => {
  const index = Math.max(profileTabs.value.findIndex(profile => activeProfile.value === profile.name), 0)
  return {
    transform: `translateX(${index * profileTabWidth}px)`,
    width: `${profileTabWidth}px`
  }
})
const customProfileCount = computed(() =>
  profileTabs.value.filter(profile => !profile.is_default && !isDraftProfile(profile)).length
)
const canCreateProfile = computed(() => customProfileCount.value < profileMaxCustomCount)
const hasDraftProfile = computed(() => profileTabs.value.some(profile => isDraftProfile(profile)))
const showProfileAddTab = computed(() => canCreateProfile.value && !hasDraftProfile.value)

const profileSettingsStatusMap = ref<Partial<Record<string, CliProfileSettingsStatus>>>({})
const profileCommandLoading = ref<string | null>(null)
let profileCommandRequestId = 0

function profileStatusKey(cliType: CliType, profile: ProviderProfile) {
  return `${cliType}:${profile}`
}
const currentProfileSettingsStatus = computed(() => profileSettingsStatusMap.value[profileStatusKey(activeCliType.value, activeProfile.value)])
const currentProfileLaunchCommand = computed(() => currentProfileSettingsStatus.value?.launch_command || '')
const isCurrentProfileCommandLoading = computed(() => profileCommandLoading.value === profileStatusKey(activeCliType.value, activeProfile.value))
const profileUsageText = computed(() => {
  if (isProviderDirectMode.value) {
    return '中转直连会将服务商写入当前 Profile 对应配置文件，通过对应启动命令启动的 Agent 会直连该服务商'
  }
  return '切换到 Profile 时会自动生成对应配置文件，通过对应启动命令启动的 Agent 会路由到对应 Profile 配置的服务商'
})

function profileDisplayLabel(profile?: ProviderProfileItem) {
  if (!profile) return ''
  if (isDraftProfile(profile)) return profile.label
  if (profile.is_default) return profile.label
  return profile.label.replace(/_/g, ' ')
}

function isDraftProfile(profile: ProviderProfileItem) {
  return profile.name === draftProfileName
}

function removeDraftProfile() {
  profileTabs.value = profileTabs.value.filter(profile => !isDraftProfile(profile))
}

function profileNameError(input: string) {
  const trimmed = input.trim()
  if (!trimmed) return '不能为空'
  if (!/^[A-Za-z0-9 _-]+$/.test(trimmed)) return profileNameRuleText
  if (profileNameFromInput(trimmed).length > profileNameMaxLength) return profileNameRuleText
  return ''
}

function profileNameExists(name: string, currentName: string) {
  return profileTabs.value.some(profile =>
    !isDraftProfile(profile) && profile.name.toLowerCase() === name.toLowerCase() && profile.name !== currentName
  )
}

function nextDefaultProfileName() {
  const existing = new Set(
    profileTabs.value
      .filter(profile => !isDraftProfile(profile))
      .map(profile => profile.name.toLowerCase())
  )
  let index = 1
  let name = `profile${index}`
  while (existing.has(name)) {
    index += 1
    name = `profile${index}`
  }
  return name
}

function profileNameFromInput(input: string) {
  return input.trim().replace(/\s+/g, '_').toLowerCase()
}

function setProfileErrorTooltipRef(name: ProviderProfile, el: unknown) {
  if (!el) {
    profileErrorTooltipRefs.delete(name)
    return
  }
  const tooltip = el as { updatePopper?: () => void }
  if (typeof tooltip.updatePopper === 'function') profileErrorTooltipRefs.set(name, tooltip)
}

function updateProfileErrorTooltip() {
  const name = editingProfileName.value
  if (!name || !profileRenameError.value) return
  nextTick(() => requestAnimationFrame(() => profileErrorTooltipRefs.get(name)?.updatePopper?.()))
}

async function focusProfileRenameInput() {
  await nextTick()
  const input = Array.isArray(profileRenameInput.value)
    ? profileRenameInput.value[0]
    : profileRenameInput.value
  input?.focus()
  input?.select()
}

async function loadProfiles() {
  if (!profileCapableCliTypes.includes(activeCliType.value)) {
    profileTabs.value = [{ cli_type: activeCliType.value, name: 'default', label: '默认', is_default: true, sort_order: 0 }]
    activeProfile.value = 'default'
    return
  }

  profileLoading.value = true
  try {
    const { data } = await providersApi.listProfiles(activeCliType.value)
    profileTabs.value = data
    if (!profileTabs.value.some(profile => profile.name === activeProfile.value)) {
      activeProfile.value = 'default'
    }
  } catch (e: any) {
    notify(getErrorMessage(e, '加载 Profile 失败'), 'error')
  } finally {
    profileLoading.value = false
  }
}

async function handleAddProfile() {
  if (!canCreateProfile.value) {
    notify(`最多创建 ${profileMaxCustomCount} 个 Profile`, 'warning')
    return
  }
  removeDraftProfile()
  const displayName = nextDefaultProfileName()
  profileTabs.value = [
    ...profileTabs.value,
    { cli_type: activeCliType.value, name: draftProfileName, label: displayName, is_default: false, sort_order: profileTabs.value.length }
  ]
  editingProfileName.value = draftProfileName
  profileRenameDraft.value = displayName
  profileRenameError.value = ''
  await focusProfileRenameInput()
}

async function startProfileRename(profile: ProviderProfileItem) {
  if (profile.is_default) return
  editingProfileName.value = profile.name
  profileRenameDraft.value = profileDisplayLabel(profile)
  profileRenameError.value = ''
  await focusProfileRenameInput()
}

function cancelProfileRename() {
  if (editingProfileName.value === draftProfileName) removeDraftProfile()
  editingProfileName.value = null
  profileRenameDraft.value = ''
  profileRenameError.value = ''
}

async function commitProfileRename(profile: ProviderProfileItem) {
  if (editingProfileName.value !== profile.name) return
  if (!profileRenameDraft.value.trim() && isDraftProfile(profile)) {
    cancelProfileRename()
    return
  }
  const name = profileNameFromInput(profileRenameDraft.value)
  const creatingProfile = isDraftProfile(profile)
  if (!creatingProfile && name === profile.name) {
    cancelProfileRename()
    return
  }
  const error = profileNameError(profileRenameDraft.value)
  if (error) {
    profileRenameError.value = error
    await focusProfileRenameInput()
    return
  }

  if (profileNameExists(name, profile.name)) {
    profileRenameError.value = '名称已存在'
    await focusProfileRenameInput()
    return
  }

  profileLoading.value = true
  try {
    if (creatingProfile) {
      const { data } = await providersApi.createProfile(activeCliType.value, name)
      await loadProfiles()
      const ok = await ensureProfileReady(data.name)
      if (ok) activeProfile.value = data.name
      await providerStore.fetchProviders(activeCliType.value as CliType, data.name)
      cancelProfileRename()
      notify('Profile 已创建')
      return
    }

    const oldProfile = profile.name
    const { data } = await providersApi.renameProfile(activeCliType.value, oldProfile, name)
    const oldPrefix = `${activeCliType.value}_${oldProfile}`
    const newPrefix = `${activeCliType.value}_${data.name}`
    if (providerStore.providersMap[oldPrefix]) {
      providerStore.providersMap[newPrefix] = providerStore.providersMap[oldPrefix]
      delete providerStore.providersMap[oldPrefix]
    }
    profileSettingsStatusMap.value = {}
    if (activeProfile.value === oldProfile) activeProfile.value = data.name
    await loadProfiles()
    await ensureProfileReady(data.name)
    await providerStore.fetchProviders(activeCliType.value as CliType, data.name)
    cancelProfileRename()
    notify('Profile 已重命名')
  } catch (e: any) {
    notify(getErrorMessage(e, creatingProfile ? '创建 Profile 失败' : '重命名 Profile 失败'), 'error')
    await focusProfileRenameInput()
  } finally {
    profileLoading.value = false
  }
}

async function handleDeleteProfile(profile: ProviderProfileItem) {
  if (profile.is_default) return
  try {
    await confirm(`确定删除 Profile「${profileDisplayLabel(profile)}」？该 Profile 下的服务商和定时任务会一并删除。`, '确认')
    profileLoading.value = true
    const deletedIndex = profileTabs.value.findIndex(item => item.name === profile.name)
    const previousProfile = profileTabs.value[deletedIndex - 1].name
    await providersApi.deleteProfile(activeCliType.value, profile.name)
    delete providerStore.providersMap[`${activeCliType.value}_${profile.name}`]
    profileSettingsStatusMap.value = {}
    if (activeProfile.value === profile.name) activeProfile.value = previousProfile
    await loadProfiles()
    await providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
  } catch (e: any) {
    if (e !== 'cancel') notify(getErrorMessage(e, '删除 Profile 失败'), 'error')
  } finally {
    profileLoading.value = false
  }
}

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
  input_price_per_m: number
  output_price_per_m: number
  cache_read_price_per_m: number
  cache_creation_price_per_m: number
  model_maps: FormModelMap[]
  model_blacklist: FormModelBlacklist[]
}
interface ProviderTogglePayload { provider: Provider; enabled: boolean }

const toggleLoadingId = ref<number | null>(null)
const writeProviderLoadingId = ref<number | null>(null)
const writeCredentialLoadingId = ref<number | null>(null)

const form = ref({
  name: '', base_url: '', api_key: '', failure_threshold: 5, blacklist_minutes: 10,
  custom_useragent: '', input_price_per_m: 0, output_price_per_m: 0,
  cache_read_price_per_m: 0, cache_creation_price_per_m: 0,
  model_maps: [] as FormModelMap[], model_blacklist: [] as FormModelBlacklist[]
})
const copiedProvider = ref<ProviderDraft | null>(null)
const pasteLoading = ref(false)

const credentialForm = ref({ name: '', claude_settings: '', codex_auth: '', gemini_oauth: '', gemini_accounts: '' })

const baseUrlPlaceholder = computed(() => activeCliType.value === 'codex' ? 'https://api.example.com/v1' : 'https://api.example.com')

function resetForm() {
  form.value = {
    name: '', base_url: '', api_key: '', failure_threshold: 5, blacklist_minutes: 10,
    custom_useragent: '', input_price_per_m: 0, output_price_per_m: 0,
    cache_read_price_per_m: 0, cache_creation_price_per_m: 0, model_maps: [], model_blacklist: []
  }
}
function resetCredentialForm() {
  credentialForm.value = { name: '', claude_settings: '', codex_auth: '', gemini_oauth: '', gemini_accounts: '' }
}
function cloneProviderDraft(draft: ProviderDraft): ProviderDraft {
  return { ...draft, model_maps: draft.model_maps.map((m) => ({ ...m })), model_blacklist: draft.model_blacklist.map((b) => ({ ...b })) }
}
function createProviderDraft(provider: Provider): ProviderDraft {
  return {
    name: provider.name, base_url: provider.base_url, api_key: provider.api_key, enabled: provider.enabled,
    failure_threshold: provider.failure_threshold, blacklist_minutes: provider.blacklist_minutes,
    custom_useragent: provider.custom_useragent || '',
    input_price_per_m: provider.input_price_per_m || 0, output_price_per_m: provider.output_price_per_m || 0,
    cache_read_price_per_m: provider.cache_read_price_per_m || 0, cache_creation_price_per_m: provider.cache_creation_price_per_m || 0,
    model_maps: provider.model_maps.map(({ source_model, target_model, enabled }) => ({ source_model, target_model, enabled })),
    model_blacklist: provider.model_blacklist.map(({ model_pattern }) => ({ model_pattern }))
  }
}
function makeUniqueProviderName(name: string): string {
  const trimmedName = name.trim() || '未命名服务商'
  const existingNames = new Set(providerStore.providers.map((p) => p.name.trim().toLowerCase()))
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

const showDetectDialog = ref(false)
const detectLoading = ref(false)
const detectModel = ref('')
const detectTestText = ref('')
const detectSelectedIds = ref<number[]>([])
const detectResults = ref<TestProviderResult[]>([])

const detectProviderList = computed(() => isProviderDirectMode.value ? providerStore.providers : providerStore.providers.filter((p) => p.enabled))
const isAllDetectSelected = computed(() => detectProviderList.value.length > 0 && detectSelectedIds.value.length === detectProviderList.value.length)

function toggleDetectProvider(id: number) {
  const idx = detectSelectedIds.value.indexOf(id)
  if (idx >= 0) detectSelectedIds.value.splice(idx, 1)
  else detectSelectedIds.value.push(id)
}
function toggleAllDetectProviders() {
  if (isAllDetectSelected.value) detectSelectedIds.value = []
  else detectSelectedIds.value = detectProviderList.value.map((p) => p.id)
}

watch(showDetectDialog, (open) => {
  if (open) {
    detectModel.value = getReusableModelName(activeCliType.value)
    detectTestText.value = getReusableTestText(activeCliType.value)
    detectSelectedIds.value = detectProviderList.value.map((p) => p.id)
    detectResults.value = []
    detectLoading.value = false
  } else if (testResultListener) {
    testResultListener()
    testResultListener = null
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
  saveReusableTestText(activeCliType.value, detectTestText.value)
  detectResults.value = detectSelectedIds.value.map((id) => {
    const p = providerStore.providers.find((x) => x.id === id)
    return {
      provider_id: id, provider_name: p?.name || 'Unknown', actual_model: '...', status_code: null,
      elapsed_ms: 0, response_text: '', request_url: '', request_headers: '', request_body: '', response_headers: '', response_body: ''
    }
  })
  detectLoading.value = true
  if (testResultListener) {
    testResultListener()
    testResultListener = null
  }
  testResultListener = await providersApi.listenTestResults((result) => {
    const idx = detectResults.value.findIndex((r) => r.provider_id === result.provider_id)
    if (idx >= 0) detectResults.value[idx] = result
    if (detectResults.value.every((r) => r.response_text !== '')) detectLoading.value = false
  })
  try {
    await providersApi.startTestModels(detectModel.value.trim(), detectSelectedIds.value, detectTestText.value.trim())
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
  profileSettingsStatusMap.value = { ...profileSettingsStatusMap.value, [profileStatusKey(cliType, status.profile)]: status }
}
async function loadProfileSettingsStatus(cliType: CliType, profile: ProviderProfile, silent = false): Promise<CliProfileSettingsStatus | null> {
  const requestId = ++profileCommandRequestId
  profileCommandLoading.value = profileStatusKey(cliType, profile)
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
    if (requestId === profileCommandRequestId) profileCommandLoading.value = null
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
    const label = profileDisplayLabel(profileTabs.value.find(item => item.name === profile)) || profile
    notify(`已复制 ${label} 启动命令`)
  } catch {
    notify('复制失败', 'error')
  }
}

function addModelMap() { form.value.model_maps.push({ source_model: '', target_model: '', enabled: true }) }
function removeModelMap(index: number) { form.value.model_maps.splice(index, 1) }
function addModelBlacklist() { form.value.model_blacklist.push({ model_pattern: '' }) }
function removeModelBlacklist(index: number) { form.value.model_blacklist.splice(index, 1) }

function normalizePrice(value: unknown): number {
  const numberValue = Number(value)
  return Number.isFinite(numberValue) && numberValue > 0 ? numberValue : 0
}

async function ensureProfileReady(profile: ProviderProfile): Promise<boolean> {
  const cliType = activeCliType.value
  if (!profileCapableCliTypes.includes(cliType) || viewMode.value !== 'proxy' || !isProxyRouteMode.value) return true
  if (profile === 'default') return true
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

    return true
  } catch (e: any) {
    notify(getErrorMessage(e, '写入 Profile 配置失败'), 'error')
    return false
  } finally {
    profileSwitching.value = null
  }
}
async function handleProfileSelect(profile: ProviderProfile) {
  if (profile === draftProfileName) {
    await focusProfileRenameInput()
    return
  }
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

watch(() => activeCliType.value, async (cliType) => {
  await loadProfiles()
  const profile = await ensureCurrentProfileOrFallback()
  const key = providerStore.getCacheKey(cliType as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(cliType as CliType, profile)
  }
  credentialStore.fetchCredentials(cliType as CliType)
})
watch([activeCliType, currentCliMode], ([cliType, mode]) => {
  viewModes.value[cliType as CliType] = mode === 'official_direct' ? 'direct' : 'proxy'
}, { immediate: true })
watch(() => activeProfile.value, (profile) => {
  if (!showProfileControls.value) return
  const key = providerStore.getCacheKey(activeCliType.value as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(activeCliType.value as CliType, profile)
  }
})
watch(profileRenameDraft, (value) => {
  if (!editingProfileName.value) return
  profileRenameError.value = profileNameError(value)
})
watch([profileRenameError, editingProfileName], updateProfileErrorTooltip)
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
  if (!visible || profileSettingsStatusMap.value[profileStatusKey(cliType as CliType, profile as ProviderProfile)]) return
  loadProfileSettingsStatus(cliType as CliType, profile as ProviderProfile, true)
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
      cli_type: targetCliType, profile: targetProfile, ...draft,
      name: makeUniqueProviderName(draft.name),
      model_maps: draft.model_maps.filter((m) => m.source_model && m.target_model),
      model_blacklist: draft.model_blacklist.filter((b) => b.model_pattern)
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
    input_price_per_m: provider.input_price_per_m || 0, output_price_per_m: provider.output_price_per_m || 0,
    cache_read_price_per_m: provider.cache_read_price_per_m || 0, cache_creation_price_per_m: provider.cache_creation_price_per_m || 0,
    model_maps: provider.model_maps.map((m) => ({ ...m })), model_blacklist: provider.model_blacklist.map((b) => ({ ...b }))
  }
}
async function handleSave() {
  if (!form.value.name.trim() || !form.value.base_url.trim() || !form.value.api_key.trim()) {
    notify('请填写完整的必填项', 'error')
    return
  }
  const data = {
    cli_type: activeCliType.value, profile: currentProviderProfile.value, ...form.value,
    input_price_per_m: normalizePrice(form.value.input_price_per_m),
    output_price_per_m: normalizePrice(form.value.output_price_per_m),
    cache_read_price_per_m: normalizePrice(form.value.cache_read_price_per_m),
    cache_creation_price_per_m: normalizePrice(form.value.cache_creation_price_per_m),
    model_maps: form.value.model_maps.filter((m) => m.source_model && m.target_model),
    model_blacklist: form.value.model_blacklist.filter((b) => b.model_pattern)
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
  if (isProviderDirectMode.value || isDisabledMode.value) return
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
  const ids = providerStore.providers.map((p) => p.id)
  await providerStore.reorderProviders(ids)
  notify('排序已保存')
}
async function handleReset(provider: Provider) {
  await providerStore.resetFailures(provider.id)
  if (provider.is_blacklisted) await providerStore.unblacklist(provider.id)
  notify('重置成功')
}
async function handleCommand(command: string, provider: Provider) {
  if (command === 'delete') {
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
      filesData.forEach((file) => {
        const path = file.path || ''
        const content = file.content || ''
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
        filesData.forEach((file) => {
          const path = file.path || ''
          const content = file.content || ''
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
    cli_type: activeCliType.value as CliType, name: credentialForm.value.name.trim(), credential_json: JSON.stringify(files)
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
  const ids = credentialStore.credentials.map((c) => c.id)
  await credentialStore.reorderCredentials(ids)
  notify('排序已保存')
}

const now = ref(Date.now())
let timer: ReturnType<typeof setInterval> | null = null

function handleVisibilityChange() {
  if (document.visibilityState === 'visible') now.value = Date.now()
}
function getUnblacklistTime(provider: Provider): string {
  if (!provider.is_blacklisted || !provider.blacklisted_until) return '已熔断'
  const diffSeconds = provider.blacklisted_until - (now.value / 1000)
  if (diffSeconds <= 0) return '已解除'
  const mins = Math.floor(diffSeconds / 60)
  return mins === 0 ? `${Math.ceil(diffSeconds)}秒后解除` : `${mins}分${Math.ceil(diffSeconds % 60)}秒后解除`
}

onMounted(async () => {
  await settingsStore.fetchSettings()
  await loadProfiles()
  const profile = await ensureCurrentProfileOrFallback()
  providerStore.fetchProviders(activeCliType.value as CliType, profile)
  credentialStore.fetchCredentials(activeCliType.value as CliType)
  document.addEventListener('visibilitychange', handleVisibilityChange)
  timer = setInterval(() => {
    if (document.visibilityState !== 'visible') return
    const oldNow = now.value
    now.value = Date.now()
    const hasExpired = providerStore.providers.some((p) => {
      if (p.is_blacklisted && p.blacklisted_until) {
        return p.blacklisted_until > (oldNow / 1000) && p.blacklisted_until <= (now.value / 1000)
      }
      return false
    })
    if (hasExpired) providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
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
.prov-page { flex: 1; min-height: 0; display: flex; flex-direction: column; margin-top: -16px; }

.prov-clitabs { flex-shrink: 0; margin-bottom: 16px; }
.profile-tabs .v2-seg-btn.active { pointer-events: auto; }
.profile-tabs { --profile-tab-width: 120px; max-width: min(680px, 58vw); overflow-x: auto; scrollbar-width: none; }
.profile-tabs::-webkit-scrollbar { display: none; }
.profile-tabs:has(.v2-seg-slider) { display: inline-flex; grid-auto-columns: unset; }
.profile-tabs:has(.v2-seg-slider) .profile-tab-btn { width: var(--profile-tab-width); min-width: var(--profile-tab-width); max-width: var(--profile-tab-width); display: inline-flex; align-items: center; justify-content: center; overflow: visible; padding-left: 8px; padding-right: 8px; }
.profile-tab-label { box-sizing: border-box; display: block; width: 100%; min-width: 0; overflow: hidden; text-align: center; text-overflow: ellipsis; }
.profile-tab-btn.active .profile-tab-label { padding: 0 18px; }
.profile-tab-delete { position: absolute; top: 50%; right: 8px; width: 16px; height: 16px; transform: translateY(-50%); border: 0; background: transparent; color: var(--v2-text-3); cursor: pointer; font-size: 14px; line-height: 16px; padding: 0; pointer-events: auto; }
.profile-tab-delete:hover:not(:disabled) { color: var(--v2-danger); }
.profile-tab-delete:disabled { cursor: default; opacity: 0.45; }
.profile-rename-input { width: 100%; min-width: 0; height: 18px; border: 0; outline: 0; background: transparent; color: inherit; font: inherit; line-height: 18px; text-align: center; padding: 0; }
.profile-add-cell { position: relative; z-index: 1; display: inline-flex; width: var(--profile-tab-width); min-width: var(--profile-tab-width); }
.profile-tabs:has(.v2-seg-slider) .profile-add-tab { width: 100%; min-width: 0; padding: 5px 0; font-size: 18px; line-height: 20px; }
.profile-add-tab:disabled { cursor: default; opacity: 0.45; }

.prov-shell { flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 0; overflow: hidden; }



.prov-toolbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  padding: 13px 18px;
  border-bottom: 1px solid var(--v2-surface-2);
}
.toolbar-left { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.toolbar-right { display: flex; align-items: center; gap: 8px; }

.prov-body { flex: 1; min-height: 0; overflow: auto; scrollbar-gutter: stable; }
.prov-shell > .v2-empty { flex: 1; min-height: 0; }

.prov-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 60px 24px;
  text-align: center;
}
.prov-empty-icon { color: var(--v2-text-3); opacity: 0.6; margin-bottom: 4px; }
.prov-empty-title { font-size: var(--v2-fs-md); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); }
.prov-empty-sub { font-size: var(--v2-fs-sm); color: var(--v2-text-3); max-width: 320px; line-height: 1.5; margin-bottom: 8px; }

.profile-help { width: 320px; }
</style>

<style>
.v2-profile-error-pop.el-popper {
  color: var(--v2-danger);
}
.v2-profile-pop .profile-cmd { margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--v2-surface-2); }
.v2-profile-pop .profile-cmd-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 6px; font-size: var(--v2-fs-xs); color: var(--v2-text-2); }
.v2-profile-pop .profile-cmd-copy { width: 24px; height: 24px; display: inline-flex; align-items: center; justify-content: center; border: 1px solid var(--v2-surface-2); border-radius: 6px; background: var(--v2-surface); color: var(--v2-text-3); cursor: pointer; }
.v2-profile-pop .profile-cmd-copy:hover:not(:disabled) { color: var(--v2-text); border-color: var(--v2-surface-3); }
.v2-profile-pop .profile-cmd-copy svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.v2-profile-pop .profile-cmd-text { padding: 8px; border: 1px solid var(--v2-surface-2); border-radius: 6px; background: var(--v2-surface-2); color: var(--v2-text-2); font-size: var(--v2-fs-xs); line-height: 1.45; word-break: break-all; cursor: pointer; user-select: text; }

/* 服务商表格 */
.pt-head { position: sticky; top: 0; z-index: 2; padding: 11px 18px; border-bottom: 1px solid var(--v2-surface-2); background: var(--v2-surface-2); overflow-x: visible; }
.pt-head .pt-cols > div { font-size: var(--v2-fs-xs); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); }
.pt-cols { display: grid; align-items: center; gap: 16px; width: 100%; }
.pt-cols.m-route { grid-template-columns: 72px repeat(5, minmax(80px, 1fr)) 128px; }
.pt-cols.m-direct { grid-template-columns: 72px repeat(3, minmax(80px, 1fr)) 96px; }
.pt-cols.m-cred { grid-template-columns: 72px repeat(3, minmax(80px, 1fr)) 72px; }
.pt-cols > div { text-align: center; min-width: 0; }
.pt-cols.m-route > div:nth-child(2),
.pt-cols.m-direct > div:nth-child(2),
.pt-cols.m-cred > div:nth-child(2) { text-align: left; }
.pt-cols .pt-col-endpoint { text-align: left; }
.pt-row { position: relative; padding: 0 18px; border-bottom: 1px solid var(--v2-surface-2); transition: background 0.15s; }
.pt-row:last-child { border-bottom: none; }
.pt-row::before {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: transparent;
  transition: background-color 0.15s;
}
.pt-row:hover::before {
  background: var(--v2-success);
}
.pt-row.bl:hover::before {
  background: var(--v2-danger);
}
.pt-row.off:hover::before {
  background: var(--v2-text-3);
}
.pt-row:hover { background: var(--v2-surface-2); }
.pt-row.bl { background: var(--v2-danger-bg); }
.pt-row .pt-cols { height: 54px; }
.pt-row .pt-cols > div { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-regular); line-height: 1.35; color: var(--v2-text); }
.pt-row .mono { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-regular); }
.pt-row .v2-pill { font-size: var(--v2-fs-xs); line-height: 16px; font-weight: var(--v2-fw-medium); }
.pt-drag { position: absolute; left: 1px; top: 0; bottom: 0; display: flex; justify-content: center; align-items: center; width: 16px; cursor: grab; opacity: 0; transition: opacity 0.15s; color: var(--v2-text-3); }
.pt-row:hover .pt-drag { opacity: 0.55; }
.pt-name { min-width: 0; font: inherit; font-weight: var(--v2-fw-medium); color: var(--v2-text); }
.pt-name.off { color: var(--v2-text-3); }


.pt-cell { font: inherit; color: var(--v2-text); min-width: 0; }
.pt-cell.muted { color: var(--v2-text-3); }
.pt-endpoint { color: var(--v2-text-2); }
.pt-fail { font: inherit; color: var(--v2-text); }
.pt-fail.danger { color: var(--v2-danger); }
.pt-switch { display: flex; align-items: center; justify-content: center; }
.pt-status { transition: filter 0.15s; }
.pt-status.pt-status-clickable { cursor: pointer; }
.pt-status.pt-status-clickable:hover { filter: brightness(0.96); }
.pt-acts { display: flex; align-items: center; gap: 2px; justify-content: flex-end; }
.pt-act { width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; border: none; background: transparent; color: var(--v2-text-btn); border-radius: 6px; cursor: pointer; transition: background 0.15s, color 0.15s; }
.pt-act:hover { background: var(--v2-surface-3); color: var(--v2-text); }
.pt-act.danger:hover { background: var(--v2-danger-bg); color: var(--v2-danger); }
.pt-act.off { color: var(--v2-text-3); opacity: 0.4; pointer-events: none; }
.pt-act svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
@media (max-width: 920px) {
  .pt-col-map { display: none; }
}
</style>
