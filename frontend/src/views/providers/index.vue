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

    <V2Tabs class="prov-clitabs">
      <div v-for="c in cliTabs" :key="c.id" class="v2-tab" :class="{ active: activeCliType === c.id }" @click="activeCliType = c.id">
        <span class="tab-label-text">{{ c.label }}</span>
      </div>
    </V2Tabs>

    <div class="v2-card prov-shell">
    <div class="prov-toolbar">
      <div class="toolbar-left">
        <div class="v2-seg">
          <div class="v2-seg-slider" :style="{ transform: `translateX(${viewMode === 'relay' ? 0 : 1}00%)`, width: 'calc((100% - 8px) / 2)' }"></div>
          <button class="v2-seg-btn" :class="{ active: viewMode === 'relay' }" type="button" @click="handleSwitchRelay">{{ t('providers.view.relay') }}</button>
          <button class="v2-seg-btn" :class="{ active: viewMode === 'official' }" type="button" @click="handleSwitchOfficial">{{ t('providers.view.official') }}</button>
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
                :aria-label="t('providers.profile.deleteAria')"
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
              :aria-label="t('providers.profile.addAria')"
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
              <div class="tooltip-title">{{ t('providers.profile.usageTitle') }}</div>
              <div class="tooltip-item"><span>{{ profileUsageText }}</span></div>
              <div class="profile-cmd">
                <div class="profile-cmd-head">
                  <strong>{{ t('providers.profile.launchCommandTitle', { name: currentProfileLabel }) }}</strong>
                  <el-tooltip :content="t('providers.profile.copyLaunchCommandTip')" placement="top" effect="light" :show-after="250">
                    <button class="profile-cmd-copy" type="button" :disabled="isCurrentProfileCommandLoading" @click.stop="copyCurrentProfileLaunchCommand">
                      <svg width="14" height="14"><use href="#v2i-copy"/></svg>
                    </button>
                  </el-tooltip>
                </div>
                <div class="profile-cmd-text mono" @click.stop="copyCurrentProfileLaunchCommand">{{ currentProfileLaunchCommand || t('providers.profile.loadingLaunchCommand') }}</div>
              </div>
            </div>
          </template>
          <span class="v2-help" @click.stop="copyCurrentProfileLaunchCommand">
            <el-icon><InfoFilled /></el-icon>
          </span>
        </el-tooltip>
      </div>

      <div class="toolbar-right">
        <template v-if="viewMode === 'relay'">
          <button class="v2-btn v2-btn-sm v2-btn-ghost" @click="showPriceDialog = true">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"/><line x1="7" y1="7" x2="7.01" y2="7"/></svg>
            {{ t('providers.actions.price') }}
          </button>
          <button class="v2-btn v2-btn-sm v2-btn-ghost" @click="showDetectDialog = true">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>
            {{ t('providers.actions.detect') }}
          </button>
          <button v-if="copiedProvider" class="v2-btn v2-btn-sm v2-btn-ghost" :disabled="pasteLoading" @click="handlePasteProvider">
            <svg width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><use href="#v2i-copy"/></svg>
            {{ t('providers.actions.paste') }}
          </button>
          <button class="v2-btn v2-btn-sm v2-btn-primary" @click="handleAddProvider">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
            {{ t('common.add') }}
          </button>
        </template>
        <button v-else class="v2-btn v2-btn-sm v2-btn-primary" @click="handleAddCredential">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
          {{ t('common.add') }}
        </button>
      </div>
    </div>

    <template v-if="viewMode === 'relay'">
      <V2Empty v-if="providerStore.providers.length === 0" v-loading="providerStore.loading" :title="t('providers.empty.providerTitle')" :description="t('providers.empty.providerDescription')">
        <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/></svg></template>
      </V2Empty>
      <div v-else class="prov-body" v-loading="providerStore.loading">
        <div class="pt-head">
          <div class="pt-cols m-route">
            <div>{{ t('providers.table.enabled') }}</div>
            <div>{{ t('providers.table.provider') }}</div>
            <div class="pt-col-protocol">{{ t('providers.table.protocol') }}</div>
            <div>{{ t('providers.table.status') }}</div>
            <el-tooltip :content="t('providers.table.faultTip')" placement="top" effect="light" :show-after="250">
              <div>{{ t('providers.table.fault') }}</div>
            </el-tooltip>
            <div class="pt-col-map">{{ t('providers.table.modelMap') }}</div>
            <div>{{ t('providers.table.actions') }}</div>
          </div>
        </div>
        <draggable v-model="providerStore.providers" item-key="id" handle=".pt-drag" @end="handleDragEnd">
          <template #item="{ element }">
            <ProviderRow
              :provider="element"
              :unblacklist-text="getUnblacklistTime(element)"
              :translated="isTranslated(element.protocol)"
              :toggle-loading="toggleLoadingId === element.id"
              @copy="handleCopyProvider"
              @edit="handleEdit"
              @reset="handleReset"
              @delete="(provider) => handleCommand('delete', provider)"
              @toggle="handleToggle"
            />
          </template>
        </draggable>
      </div>
    </template>

    <template v-else>
      <V2Empty v-if="credentialStore.credentials.length === 0" v-loading="credentialStore.loading" :title="t('providers.empty.credentialTitle')" :description="t('providers.empty.credentialDescription')">
        <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/></svg></template>
      </V2Empty>
      <div v-else class="prov-body" v-loading="credentialStore.loading">
        <div class="pt-head">
          <div class="pt-cols m-cred">
            <div>{{ t('providers.table.enabled') }}</div>
            <div>{{ t('providers.table.credential') }}</div>
            <div>{{ t('providers.table.info') }}</div>
            <div>{{ t('providers.table.status') }}</div>
            <div>{{ t('providers.table.actions') }}</div>
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
      :title="editingProvider ? t('providers.drawer.editTitle') : t('providers.drawer.addTitle')"
      :form="form"
      :base-url-placeholder="baseUrlPlaceholder"
      :protocols="activeProtocols"
      :declared-protocols="declaredProtocols"
      :remark="activeAgentRemark"
      :model-sync="editingProviderModelSync"
      :model-sync-loading="!!editingProvider && modelSyncLoadingId === editingProvider.id"
      :can-sync-models="!!editingProvider"
      @confirm="handleSave"
      @add-model-map="addModelMap"
      @remove-model-map="removeModelMap"
      @add-model-blacklist="addModelBlacklist"
      @remove-model-blacklist="removeModelBlacklist"
      @add-blacklist-tier="addBlacklistTier"
      @remove-blacklist-tier="removeBlacklistTier"
      @sync-models="handleSyncEditingModels"
      @add-model="handleAddEditingModel"
      @remove-model="handleRemoveEditingModel"
    />
    <CredentialDrawer
      v-model="showCredentialDialog"
      :title="editingCredential ? t('providers.credential.editTitle') : t('providers.credential.addTitle')"
      :form="credentialForm"
      :file-definitions="credentialFileDefinitions"
      @confirm="handleSaveCredential"
      @read-from-cli="handleReadFromCli"
    />
    <ModelDetectionModal
      v-model="showDetectDialog"
      v-model:model="detectModel"
      v-model:test-text="detectTestText"
      :providers="detectProviderList"
      :selected-ids="detectSelectedIds"
      :loading="detectLoading"
      :results="detectResults"
      @confirm="handleStartDetect"
      @toggle-group="toggleAllDetectProviders"
      @toggle-provider="toggleDetectProvider"
      @copy-response="copyResponseText"
    />
    <ModelPriceModal
      v-model="showPriceDialog"
      :sync-state="priceSyncStatus"
      :sync-loading="priceSyncLoading"
      @sync="refreshPriceCatalog"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import draggable from 'vuedraggable'
import ProviderRow from './components/ProviderRow.vue'
import CredentialRow from './components/CredentialRow.vue'
import ProviderDrawer from './components/ProviderDrawer.vue'
import CredentialDrawer from './components/CredentialDrawer.vue'
import ModelDetectionModal from './components/ModelDetectionModal.vue'
import ModelPriceModal from './components/ModelPriceModal.vue'
import V2Empty from '@/components/V2Empty.vue'
import V2Tabs from '@/components/V2Tabs.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { useProviderStore } from '@/stores/providers'
import { useCredentialStore } from '@/stores/credentials'
import { useUiStore } from '@/stores/ui'
import { useSettingsStore } from '@/stores/settings'
import { useAgentStore } from '@/stores/agents'
import { useLocaleStore } from '@/stores/locale'
import { credentialsApi } from '@/api/credentials'
import { providersApi } from '@/api/providers'
import { settingsApi } from '@/api/settings'
import { InfoFilled } from '@element-plus/icons-vue'
import { CONVERTIBLE_PROTOCOLS } from '@/types/models'
import type { Provider, ProviderCreate, ProviderUpdate, CliType, ConfigFormat, Protocol, ProviderProfile, ProviderProfileItem, CliProfileSettingsStatus, CredentialFileDefinition, OfficialCredential, OfficialCredentialCreate, OfficialCredentialPayload, OfficialLoginOperation, TestProviderResult, ProviderModelsResponse, PriceSyncState } from '@/types/models'
import { getReusableModelName, saveReusableModelName, getReusableTestText, saveReusableTestText } from '@/utils/modelDefaults'

const providerStore = useProviderStore()
const credentialStore = useCredentialStore()
const uiStore = useUiStore()
const settingsStore = useSettingsStore()
const agentStore = useAgentStore()

const { t } = useI18n()

const cliTabs = computed(() => agentStore.visibleAgents
  .filter((agent) => agent.features.provider_config.enabled)
  .map((agent) => ({ id: agent.id, label: agent.name })))

const profileTabs = ref<ProviderProfileItem[]>([
  { cli_type: 'claude_code', name: 'default', label: t('providers.profile.default'), is_default: true, sort_order: 0 }
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
const profileNameRuleText = computed(() => t('providers.profile.nameRule', { max: profileNameMaxLength }))

const activeCliType = computed({
  get: () => uiStore.providersActiveCliType,
  set: (val) => uiStore.setProvidersActiveCliType(val)
})
const activeProfile = computed({
  get: () => uiStore.providersActiveProfile,
  set: (val) => uiStore.setProvidersActiveProfile(val)
})

type ViewMode = 'relay' | 'official'
const viewMode = ref<ViewMode>('relay')
const activeAgent = computed(() => agentStore.get(activeCliType.value))
const localeStore = useLocaleStore()
const activeAgentRemark = computed(() => {
  const agent = activeAgent.value
  if (!agent) return ''
  if (localeStore.locale === 'en') return agent.remark_en || agent.remark || ''
  return agent.remark || ''
})
const declaredProtocols = computed<Protocol[]>(() => activeAgent.value?.protocols ?? [])
// 端点类型可以和 Agent 声明的协议不同，网关转发时自动转换协议；Gemini 不参与转换。
const activeProtocols = computed<Protocol[]>(() => {
  const declared = declaredProtocols.value
  if (!declared.some((protocol) => CONVERTIBLE_PROTOCOLS.includes(protocol))) return declared
  return [...declared, ...CONVERTIBLE_PROTOCOLS.filter((protocol) => !declared.includes(protocol))]
})
// 两边都在可转换集合里、且确实不同，才会真的走转换；Gemini 只能透传。
const isTranslated = (protocol: Protocol) => CONVERTIBLE_PROTOCOLS.includes(protocol)
  && declaredProtocols.value.some((declared) => CONVERTIBLE_PROTOCOLS.includes(declared))
  && !declaredProtocols.value.includes(protocol)
const profileFeature = computed(() => activeAgent.value?.features.profiles)
const supportsProfiles = computed(() => profileFeature.value?.enabled === true)
const officialLoginFeature = computed(() => activeAgent.value?.features.official_login)
const canUseOfficialCredentials = computed(() => Boolean(
  officialLoginFeature.value?.enabled === true && officialLoginFeature.value.operations?.length,
))
const showProfileControls = computed(() => viewMode.value === 'relay' && supportsProfiles.value)
const showProfileHelp = computed(() => showProfileControls.value && supportsProfiles.value)
const currentProviderProfile = computed<ProviderProfile>(() => showProfileControls.value ? activeProfile.value : 'default')
const profileSwitching = ref<ProviderProfile | null>(null)
let testResultListener: (() => void) | null = null
let healthListener: (() => void) | null = null

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
const profileUsageText = computed(() => t('providers.profile.usageText'))

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
  if (!trimmed) return t('providers.profile.nameEmpty')
  if (!/^[A-Za-z0-9 _-]+$/.test(trimmed)) return profileNameRuleText.value
  if (profileNameFromInput(trimmed).length > profileNameMaxLength) return profileNameRuleText.value
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
  if (!supportsProfiles.value) {
    profileTabs.value = [{ cli_type: activeCliType.value, name: 'default', label: t('providers.profile.default'), is_default: true, sort_order: 0 }]
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
    notify(getErrorMessage(e, t('providers.notify.loadProfilesFailed')), 'error')
  } finally {
    profileLoading.value = false
  }
}

async function handleAddProfile() {
  if (!canCreateProfile.value) {
    notify(t('providers.notify.profileLimit', { max: profileMaxCustomCount }), 'warning')
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
    profileRenameError.value = t('providers.profile.nameExists')
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
      notify(t('providers.notify.profileCreated'))
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
    notify(t('providers.notify.profileRenamed'))
  } catch (e: any) {
    notify(getErrorMessage(e, creatingProfile ? t('providers.notify.createProfileFailed') : t('providers.notify.renameProfileFailed')), 'error')
    await focusProfileRenameInput()
  } finally {
    profileLoading.value = false
  }
}

async function handleDeleteProfile(profile: ProviderProfileItem) {
  if (profile.is_default) return
  try {
    await confirm(t('providers.confirm.deleteProfile', { name: profileDisplayLabel(profile) }), t('providers.confirm.title'))
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
    if (e !== 'cancel') notify(getErrorMessage(e, t('providers.notify.deleteProfileFailed')), 'error')
  } finally {
    profileLoading.value = false
  }
}

async function handleSwitchRelay() {
  if (viewMode.value === 'relay') return
  viewMode.value = 'relay'
  const profile = await ensureCurrentProfileOrFallback()
  const key = providerStore.getCacheKey(activeCliType.value, profile)
  if (!providerStore.providersMap[key]?.length) {
    providerStore.fetchProviders(activeCliType.value, profile)
  }
}
function handleSwitchOfficial() {
  if (viewMode.value === 'official') return
  if (!canUseOfficialCredentials.value) {
    notify(t('providers.notify.officialUnsupported'), 'warning')
    return
  }
  viewMode.value = 'official'
  credentialStore.fetchCredentials(activeCliType.value)
}

function handleAddCredential() {
  resetCredentialForm()
  editingCredential.value = null
  showAddCredentialDialog.value = true
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
interface FormBlacklistTier { failure_count: number; blacklist_minutes: number }
interface ProviderDraft {
  protocol: Protocol
  name: string
  base_url: string
  api_key: string
  enabled: boolean
  custom_useragent: string
  price_multiplier: number
  translate_max_tokens: number
  model_maps: FormModelMap[]
  model_blacklist: FormModelBlacklist[]
  blacklist_tiers: FormBlacklistTier[]
}
interface ProviderTogglePayload { provider: Provider; enabled: boolean }

const toggleLoadingId = ref<number | null>(null)
const writeCredentialLoadingId = ref<number | null>(null)

function defaultBlacklistTiers(): FormBlacklistTier[] {
  return [{ failure_count: 5, blacklist_minutes: 10 }]
}

const form = ref({
  protocol: '' as Protocol | '', name: '', base_url: '', api_key: '',
  custom_useragent: '', price_multiplier: 1, translate_max_tokens: 32000,
  model_maps: [] as FormModelMap[], model_blacklist: [] as FormModelBlacklist[],
  blacklist_tiers: defaultBlacklistTiers()
})
const copiedProvider = ref<ProviderDraft | null>(null)
const pasteLoading = ref(false)
const modelSyncMap = ref<Record<number, ProviderModelsResponse>>({})
const modelSyncSignatures = ref<Record<number, string>>({})
const modelSyncRequestVersions = new Map<number, number>()
const modelSyncLoadingId = ref<number | null>(null)
const priceSyncStatus = ref<PriceSyncState | null>(null)
const priceSyncLoading = ref(false)
const showPriceDialog = ref(false)
// Manual models added while creating a provider, flushed once it has an id.
const pendingManualModels = ref<string[]>([])
const editingProviderModelSync = computed<ProviderModelsResponse | undefined>(() => {
  const providerId = editingProvider.value?.id
  if (providerId) return modelSyncMap.value[providerId]
  if (!pendingManualModels.value.length) return undefined
  // A provider that has not been created yet owns no rows, so the buffered
  // names are shown as if they were already stored (negative ids mark them).
  return {
    provider_id: 0,
    models: pendingManualModels.value.map((modelName, index) => ({
      id: -(index + 1),
      provider_id: 0,
      model_name: modelName,
      source: 'manual',
      enabled: true,
      first_seen_at: 0,
      last_seen_at: 0,
    })),
    sync_state: null,
  }
})

const credentialForm = ref<{ name: string; files: Record<string, string> }>({ name: '', files: {} })

function credentialSource(operation: OfficialLoginOperation) {
  return operation.op === 'replace_file' ? operation.content_from : operation.value_from
}

function credentialFormat(operation: OfficialLoginOperation): ConfigFormat {
  return operation.format || 'json'
}

const credentialFileDefinitions = computed<CredentialFileDefinition[]>(() => {
  const targets = new Map<string, { names: Set<string>; format: ConfigFormat }>()
  for (const operation of officialLoginFeature.value?.operations ?? []) {
    const source = credentialSource(operation)
    if (!source) continue
    const target = targets.get(source.file_id) ?? {
      names: new Set<string>(),
      format: credentialFormat(operation),
    }
    target.names.add(operation.file.split(/[\\/]/).pop() || operation.file)
    targets.set(source.file_id, target)
  }
  const compact = targets.size > 1
  return Array.from(targets, ([key, target]) => ({
    key,
    name: Array.from(target.names).join(t('providers.listSeparator')),
    format: target.format,
    placeholder: target.format === 'json' ? '{}' : '',
    compact,
  }))
})

const baseUrlPlaceholder = computed(() => {
  switch (form.value.protocol) {
    case 'openai_chat':
    case 'openai_responses':
      return 'https://api.example.com/v1'
    case 'gemini_generate_content':
      return 'https://generativelanguage.googleapis.com/v1beta'
    default:
      return 'https://api.example.com'
  }
})

function defaultProtocol(): Protocol | '' {
  return declaredProtocols.value.length === 1 ? declaredProtocols.value[0] : ''
}

function resetForm() {
  form.value = {
    protocol: defaultProtocol(), name: '', base_url: '', api_key: '',
    custom_useragent: '', price_multiplier: 1, translate_max_tokens: 32000,
    model_maps: [], model_blacklist: [], blacklist_tiers: defaultBlacklistTiers()
  }
  pendingManualModels.value = []
}
function resetCredentialForm() {
  credentialForm.value = {
    name: '',
    files: Object.fromEntries(credentialFileDefinitions.value.map((file) => [file.key, ''])),
  }
}
function cloneProviderDraft(draft: ProviderDraft): ProviderDraft {
  return {
    ...draft,
    model_maps: draft.model_maps.map((m) => ({ ...m })),
    model_blacklist: draft.model_blacklist.map((b) => ({ ...b })),
    blacklist_tiers: draft.blacklist_tiers.map((t) => ({ ...t }))
  }
}
function providerDraftTiers(provider: Provider): FormBlacklistTier[] {
  const tiers = provider.blacklist_tiers?.map(({ failure_count, blacklist_minutes }) => ({ failure_count, blacklist_minutes })) ?? []
  return tiers.length ? tiers : defaultBlacklistTiers()
}
function createProviderDraft(provider: Provider): ProviderDraft {
  return {
    protocol: provider.protocol, name: provider.name, base_url: provider.base_url, api_key: provider.api_key, enabled: provider.enabled,
    custom_useragent: provider.custom_useragent || '',
    price_multiplier: normalizeMultiplier(provider.price_multiplier),
    translate_max_tokens: normalizeTranslateMaxTokens(provider.translate_max_tokens),
    model_maps: provider.model_maps.map(({ source_model, target_model, enabled }) => ({ source_model, target_model, enabled })),
    model_blacklist: provider.model_blacklist.map(({ model_pattern }) => ({ model_pattern })),
    blacklist_tiers: providerDraftTiers(provider)
  }
}
function makeUniqueProviderName(name: string): string {
  const trimmedName = name.trim() || t('providers.provider.unnamed')
  const existingNames = new Set(providerStore.providers.map((p) => p.name.trim().toLowerCase()))
  if (!existingNames.has(trimmedName.toLowerCase())) return trimmedName
  const baseName = t('providers.provider.copySuffix', { name: trimmedName })
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

const detectProviderList = computed(() => providerStore.providers)

function toggleDetectProvider(id: number) {
  const idx = detectSelectedIds.value.indexOf(id)
  if (idx >= 0) detectSelectedIds.value.splice(idx, 1)
  else detectSelectedIds.value.push(id)
}
function toggleAllDetectProviders(enabled: boolean) {
  const ids = detectProviderList.value
    .filter((provider) => provider.enabled === enabled)
    .map((provider) => provider.id)
  if (ids.every((id) => detectSelectedIds.value.includes(id))) {
    detectSelectedIds.value = detectSelectedIds.value.filter((id) => !ids.includes(id))
  } else {
    detectSelectedIds.value = [...new Set([...detectSelectedIds.value, ...ids])]
  }
}

watch(showDetectDialog, (open) => {
  if (open) {
    detectModel.value = getReusableModelName(activeCliType.value)
    detectTestText.value = getReusableTestText(activeCliType.value)
    detectSelectedIds.value = detectProviderList.value.filter((provider) => provider.enabled).map((p) => p.id)
    detectResults.value = []
    detectLoading.value = false
  } else if (testResultListener) {
    testResultListener()
    testResultListener = null
  }
})

async function handleStartDetect() {
  if (!detectModel.value.trim()) {
    notify(t('providers.notify.detectModelRequired'), 'error')
    return
  }
  if (detectSelectedIds.value.length === 0) {
    notify(t('providers.notify.detectProviderRequired'), 'error')
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
    notify(getErrorMessage(e, t('providers.notify.detectFailed')), 'error')
    detectLoading.value = false
  }
}
async function copyResponseText(text: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    notify(t('providers.notify.responseCopied'))
  } catch {
    notify(t('providers.notify.copyFailed'), 'error')
  }
}

function cacheProfileSettingsStatus(cliType: CliType, status: CliProfileSettingsStatus) {
  profileSettingsStatusMap.value = { ...profileSettingsStatusMap.value, [profileStatusKey(cliType, status.profile)]: status }
}
async function loadProfileSettingsStatus(cliType: CliType, profile: ProviderProfile, silent = false): Promise<CliProfileSettingsStatus | null> {
  const requestId = ++profileCommandRequestId
  profileCommandLoading.value = profileStatusKey(cliType, profile)
  try {
    const { data } = await settingsApi.getProfileSettingsStatus(cliType, profile)
    cacheProfileSettingsStatus(cliType, data)
    return data
  } catch (e: any) {
    if (!silent) notify(getErrorMessage(e, t('providers.notify.launchCommandFailed')), 'error')
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
    notify(t('providers.notify.launchCommandCopied', { name: label }))
  } catch {
    notify(t('providers.notify.copyFailed'), 'error')
  }
}

function addModelMap() { form.value.model_maps.push({ source_model: '', target_model: '', enabled: true }) }
function removeModelMap(index: number) { form.value.model_maps.splice(index, 1) }
function addModelBlacklist() { form.value.model_blacklist.push({ model_pattern: '' }) }
function removeModelBlacklist(index: number) { form.value.model_blacklist.splice(index, 1) }
function addBlacklistTier() { form.value.blacklist_tiers.push({ failure_count: 3, blacklist_minutes: 5 }) }
function removeBlacklistTier(index: number) { form.value.blacklist_tiers.splice(index, 1) }

function normalizeMultiplier(value: unknown): number {
  const numberValue = Number(value)
  return Number.isFinite(numberValue) && numberValue > 0 ? numberValue : 1
}

/** 转换兜底的 max_tokens 最小 1024：再低连思考预算都摆不下。 */
function normalizeTranslateMaxTokens(value: unknown): number {
  const numberValue = Number(value)
  if (!Number.isFinite(numberValue) || numberValue < 1024) return 32000
  return Math.floor(numberValue)
}

function providerModelSyncSignature(provider: Provider): string {
  return JSON.stringify([
    provider.id,
    provider.profile,
    provider.protocol,
    provider.base_url.trim().replace(/\/+$/, ''),
    provider.api_key.trim(),
  ])
}

function nextModelSyncRequest(providerId: number): number {
  const version = (modelSyncRequestVersions.get(providerId) || 0) + 1
  modelSyncRequestVersions.set(providerId, version)
  return version
}

function isCurrentModelSyncRequest(providerId: number, signature: string, version: number): boolean {
  const current = providerStore.providers.find((item) => item.id === providerId)
  return modelSyncRequestVersions.get(providerId) === version
    && modelSyncSignatures.value[providerId] === signature
    && !!current
    && providerModelSyncSignature(current) === signature
}

function clearModelSyncEntry(providerId: number) {
  const nextModels = { ...modelSyncMap.value }
  delete nextModels[providerId]
  modelSyncMap.value = nextModels
  const nextSignatures = { ...modelSyncSignatures.value }
  delete nextSignatures[providerId]
  modelSyncSignatures.value = nextSignatures
  nextModelSyncRequest(providerId)
}

async function loadProviderModels(
  provider: Provider,
  silent = true,
  signature = providerModelSyncSignature(provider),
) {
  const requestVersion = nextModelSyncRequest(provider.id)
  try {
    const { data } = await providersApi.getModels(provider.id)
    if (!isCurrentModelSyncRequest(provider.id, signature, requestVersion)) return
    modelSyncMap.value = { ...modelSyncMap.value, [provider.id]: data }
  } catch (e: any) {
    if (!silent && isCurrentModelSyncRequest(provider.id, signature, requestVersion)) {
      notify(getErrorMessage(e, t('providers.notify.loadModelsFailed')), 'error')
    }
  }
}

async function applyModelMutation(
  provider: Provider,
  request: () => Promise<{ data: ProviderModelsResponse }>,
  errorText: string,
): Promise<boolean> {
  modelSyncLoadingId.value = provider.id
  const signature = providerModelSyncSignature(provider)
  modelSyncSignatures.value = { ...modelSyncSignatures.value, [provider.id]: signature }
  const requestVersion = nextModelSyncRequest(provider.id)
  try {
    const { data } = await request()
    if (!isCurrentModelSyncRequest(provider.id, signature, requestVersion)) return false
    modelSyncMap.value = { ...modelSyncMap.value, [provider.id]: data }
    return true
  } catch (e: any) {
    if (isCurrentModelSyncRequest(provider.id, signature, requestVersion)) {
      notify(getErrorMessage(e, errorText), 'error')
    }
    return false
  } finally {
    modelSyncLoadingId.value = null
  }
}

async function syncProviderModels(provider: Provider) {
  if (modelSyncLoadingId.value !== null) return
  const ok = await applyModelMutation(provider, () => providersApi.syncModels(provider.id), t('providers.notify.syncModelsFailed'))
  if (ok) notify(t('providers.notify.modelsSynced', { name: provider.name }))
}

// The backend syncs against the saved endpoint, so unsaved credential edits
// would silently produce a model list for the previous configuration.
function handleSyncEditingModels() {
  const provider = editingProvider.value
  if (!provider) return
  const endpointEdited = form.value.protocol !== provider.protocol
    || form.value.base_url.trim() !== provider.base_url.trim()
    || form.value.api_key.trim() !== provider.api_key.trim()
  if (endpointEdited) {
    notify(t('providers.notify.endpointEdited'), 'warning')
    return
  }
  void syncProviderModels(provider)
}
async function handleAddEditingModel(modelName: string) {
  const provider = editingProvider.value
  if (!provider) {
    const name = modelName.trim()
    if (!name || pendingManualModels.value.some((item) => item.toLowerCase() === name.toLowerCase())) return
    pendingManualModels.value = [...pendingManualModels.value, name]
    return
  }
  if (modelSyncLoadingId.value !== null) return
  await applyModelMutation(provider, () => providersApi.addManualModel(provider.id, modelName), t('providers.notify.addModelFailed'))
}
async function handleRemoveEditingModel(modelId: number) {
  const provider = editingProvider.value
  if (!provider) {
    const index = -modelId - 1
    pendingManualModels.value = pendingManualModels.value.filter((_, position) => position !== index)
    return
  }
  if (modelSyncLoadingId.value !== null) return
  await applyModelMutation(provider, () => providersApi.deleteModel(provider.id, modelId), t('providers.notify.removeModelFailed'))
}

async function flushPendingManualModels(providerId: number) {
  const failed: string[] = []
  for (const modelName of pendingManualModels.value) {
    try {
      await providersApi.addManualModel(providerId, modelName)
    } catch {
      failed.push(modelName)
    }
  }
  if (failed.length) notify(t('providers.notify.addModelsFailed', { names: failed.join(t('providers.listSeparator')) }), 'warning')
}

async function refreshPriceCatalog() {
  if (priceSyncLoading.value) return
  priceSyncLoading.value = true
  try {
    const { data } = await providersApi.syncPrices()
    priceSyncStatus.value = data
    notify(t('providers.notify.priceCatalogUpdated', { count: data.model_count }))
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.updatePriceCatalogFailed')), 'error')
    try {
      const { data } = await providersApi.getPriceSyncStatus()
      priceSyncStatus.value = data
    } catch { /* keep the previous status */ }
  } finally {
    priceSyncLoading.value = false
  }
}

function loadPriceSyncStatus() {
  providersApi.getPriceSyncStatus().then(({ data }) => {
    priceSyncStatus.value = data
  }).catch(() => undefined)
}

async function ensureProfileReady(profile: ProviderProfile): Promise<boolean> {
  const cliType = activeCliType.value
  if (!supportsProfiles.value || viewMode.value !== 'relay') return true
  if (profile === 'default') return true
  profileSwitching.value = profile
  try {
    const { data: status } = await settingsApi.getProfileSettingsStatus(cliType, profile)
    cacheProfileSettingsStatus(cliType, status)
    if (status.uses_gateway) return true
    const { data: ensured } = await settingsApi.ensureProfileSettings(cliType, profile)
    cacheProfileSettingsStatus(cliType, ensured)
    if (!ensured.uses_gateway) {
      notify(t('providers.notify.profileConfigInvalid', { path: ensured.path }), 'error')
      return false
    }

    return true
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.writeProfileConfigFailed')), 'error')
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
  viewMode.value = 'relay'
  if (!showDialog.value) resetForm()
  await loadProfiles()
  const profile = await ensureCurrentProfileOrFallback()
  const key = providerStore.getCacheKey(cliType as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(cliType as CliType, profile)
  }
  if (canUseOfficialCredentials.value) credentialStore.fetchCredentials(cliType as CliType)
})
watch(() => activeProfile.value, (profile) => {
  if (!showProfileControls.value) return
  const key = providerStore.getCacheKey(activeCliType.value as CliType, profile)
  if (!providerStore.providersMap[key] || providerStore.providersMap[key].length === 0) {
    providerStore.fetchProviders(activeCliType.value as CliType, profile)
  }
})
watch(
  () => providerStore.providers.map((provider) => ({
    provider,
    signature: providerModelSyncSignature(provider),
  })),
  (entries) => {
    const activeIds = new Set<number>()
    for (const { provider, signature } of entries) {
      const { id } = provider
      activeIds.add(id)
      if (modelSyncSignatures.value[id] !== signature) {
        modelSyncSignatures.value = { ...modelSyncSignatures.value, [id]: signature }
        const nextModels = { ...modelSyncMap.value }
        delete nextModels[id]
        modelSyncMap.value = nextModels
        nextModelSyncRequest(id)
      }
      if (!modelSyncMap.value[id]) void loadProviderModels(provider, true, signature)
    }
    for (const key of Object.keys(modelSyncMap.value)) {
      const id = Number(key)
      if (!activeIds.has(id)) clearModelSyncEntry(id)
    }
    for (const key of Object.keys(modelSyncSignatures.value)) {
      const id = Number(key)
      if (!activeIds.has(id)) clearModelSyncEntry(id)
    }
  },
  { immediate: true },
)
watch(profileRenameDraft, (value) => {
  if (!editingProfileName.value) return
  profileRenameError.value = profileNameError(value)
})
watch([profileRenameError, editingProfileName], updateProfileErrorTooltip)
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
  notify(t('providers.notify.providerCopied', { name: provider.name }))
}
async function handlePasteProvider() {
  if (!copiedProvider.value || pasteLoading.value) return
  const targetCliType = activeCliType.value as CliType
  const targetProfile = currentProviderProfile.value
  pasteLoading.value = true
  try {
    await providerStore.fetchProviders(targetCliType, targetProfile)
    const draft = cloneProviderDraft(copiedProvider.value)
    if (!activeProtocols.value.includes(draft.protocol)) {
      notify(t('providers.notify.protocolUnsupported'), 'error')
      return
    }
    const data: ProviderCreate = {
      cli_type: targetCliType, profile: targetProfile, ...draft,
      name: makeUniqueProviderName(draft.name),
      model_maps: draft.model_maps.filter((m) => m.source_model && m.target_model),
      model_blacklist: draft.model_blacklist.filter((b) => b.model_pattern)
    }
    await providerStore.createProvider(data)
    notify(t('providers.notify.providerPasted', { name: data.name }))
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.pasteFailed')), 'error')
  } finally {
    pasteLoading.value = false
  }
}
function handleEdit(provider: Provider) {
  editingProvider.value = provider
  form.value = {
    protocol: provider.protocol, name: provider.name, base_url: provider.base_url, api_key: provider.api_key,
    custom_useragent: provider.custom_useragent || '',
    price_multiplier: normalizeMultiplier(provider.price_multiplier),
    translate_max_tokens: normalizeTranslateMaxTokens(provider.translate_max_tokens),
    model_maps: provider.model_maps.map((m) => ({ ...m })), model_blacklist: provider.model_blacklist.map((b) => ({ ...b })),
    blacklist_tiers: providerDraftTiers(provider)
  }
}
async function handleSave() {
  const protocol = form.value.protocol
  if (!protocol || !activeProtocols.value.includes(protocol)) {
    notify(t('providers.notify.selectProtocol'), 'error')
    return
  }
  if (!form.value.name.trim() || !form.value.base_url.trim() || !form.value.api_key.trim()) {
    notify(t('providers.notify.requiredFields'), 'error')
    return
  }
  const data = {
    profile: currentProviderProfile.value,
    protocol,
    name: form.value.name,
    base_url: form.value.base_url,
    api_key: form.value.api_key,
    custom_useragent: form.value.custom_useragent,
    price_multiplier: normalizeMultiplier(form.value.price_multiplier),
    translate_max_tokens: normalizeTranslateMaxTokens(form.value.translate_max_tokens),
    model_maps: form.value.model_maps.filter((m) => m.source_model && m.target_model),
    model_blacklist: form.value.model_blacklist.filter((b) => b.model_pattern),
    blacklist_tiers: form.value.blacklist_tiers.filter((t) => t.failure_count >= 1 && t.blacklist_minutes >= 1),
  } satisfies ProviderUpdate
  try {
    if (editingProvider.value) {
      await providerStore.updateProvider(editingProvider.value.id, data)
      notify(t('providers.notify.updated'))
    } else {
      const created = await providerStore.createProvider({ cli_type: activeCliType.value, ...data })
      notify(t('providers.notify.created'))
      await flushPendingManualModels(created.id)
    }
    showDialog.value = false
    resetForm()
    providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.saveFailed')), 'error')
  }
}
async function handleToggle({ provider, enabled }: ProviderTogglePayload) {
  toggleLoadingId.value = provider.id
  try {
    await providerStore.updateProvider(provider.id, { enabled })
    await settingsStore.fetchSettings()
    notify(enabled ? t('providers.notify.enabled') : t('providers.notify.disabled'))
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.toggleFailed')), 'error')
  } finally {
    toggleLoadingId.value = null
  }
}
async function handleDragEnd() {
  const ids = providerStore.providers.map((p) => p.id)
  await providerStore.reorderProviders(ids)
  notify(t('providers.notify.orderSaved'))
}
async function handleReset(provider: Provider) {
  await providerStore.resetFailures(provider.id)
  notify(t('providers.notify.resetSuccess'))
}
async function handleCommand(command: string, provider: Provider) {
  if (command === 'delete') {
    try {
      await confirm(t('providers.confirm.deleteProvider'), t('providers.confirm.title'))
      await providerStore.deleteProvider(provider.id)
      await settingsStore.fetchSettings()
      clearModelSyncEntry(provider.id)
      notify(t('providers.notify.deleted'))
    } catch (e) {
      if (e !== 'cancel') notify(getErrorMessage(e, t('providers.notify.deleteFailed')), 'error')
    }
  }
}
function applyCredentialPayload(raw: string) {
  const payload = JSON.parse(raw) as OfficialCredentialPayload
  if (payload.schema_version !== 1 || !payload.files || Array.isArray(payload.files)) {
    throw new Error(t('providers.credential.invalidFormat'))
  }
  for (const definition of credentialFileDefinitions.value) {
    const file = payload.files[definition.key]
    credentialForm.value.files[definition.key] = file
      ? file.format === 'json'
        ? JSON.stringify(file.content, null, 2)
        : typeof file.content === 'string' ? file.content : String(file.content ?? '')
      : ''
  }
}

function handleEditCredential(credential: OfficialCredential) {
  resetCredentialForm()
  editingCredential.value = credential
  credentialForm.value.name = credential.name
  try {
    applyCredentialPayload(credential.credential_json)
  } catch (error) {
    notify(getErrorMessage(error, t('providers.credential.invalidFormat')), 'error')
  }
}
async function handleDeleteCredential(credential: OfficialCredential) {
  try {
    await confirm(t('providers.confirm.deleteCredential'), t('providers.confirm.title'))
    await credentialStore.deleteCredential(credential.id)
    notify(t('providers.notify.deleted'))
  } catch (e) {
    if (e !== 'cancel') notify(getErrorMessage(e, t('providers.notify.deleteFailed')), 'error')
  }
}
async function handleWriteCredential(credential: OfficialCredential) {
  writeCredentialLoadingId.value = credential.id
  try {
    await credentialsApi.writeConfig(credential.id)
    await settingsStore.fetchSettings()
    await credentialStore.fetchCredentials(activeCliType.value as CliType)
    notify(t('providers.notify.credentialWritten', { name: credential.name }))
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.writeFailed')), 'error')
  } finally {
    writeCredentialLoadingId.value = null
  }
}
async function handleReadFromCli() {
  try {
    const { data } = await credentialsApi.readCliCredential(activeCliType.value as CliType)
    applyCredentialPayload(data)
    notify(t('providers.notify.readSuccess'))
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.readFailed')), 'error')
  }
}
async function handleSaveCredential() {
  if (!credentialForm.value.name) {
    notify(t('providers.notify.credentialNameRequired'), 'error')
    return
  }
  if (!canUseOfficialCredentials.value || credentialFileDefinitions.value.length === 0) {
    notify(t('providers.notify.noCredentialRules'), 'error')
    return
  }
  const files: OfficialCredentialPayload['files'] = {}
  for (const definition of credentialFileDefinitions.value) {
    const raw = credentialForm.value.files[definition.key]?.trim()
    if (!raw) {
      notify(t('providers.notify.fileRequired', { key: definition.key }), 'error')
      return
    }
    if (definition.format === 'json') {
      try {
        files[definition.key] = { format: definition.format, content: JSON.parse(raw) }
      } catch {
        notify(t('providers.notify.fileNotJson', { name: definition.name }), 'error')
        return
      }
    } else {
      files[definition.key] = { format: definition.format, content: raw }
    }
  }
  const payload: OfficialCredentialPayload = { schema_version: 1, files }
  const data: OfficialCredentialCreate = {
    cli_type: activeCliType.value as CliType,
    name: credentialForm.value.name.trim(),
    credential_json: JSON.stringify(payload),
  }
  try {
    if (editingCredential.value) {
      await credentialStore.updateCredential(editingCredential.value.id, { name: data.name, credential_json: data.credential_json })
      notify(t('providers.notify.updated'))
    } else {
      await credentialStore.createCredential(data)
      notify(t('providers.notify.created'))
    }
    showCredentialDialog.value = false
    resetCredentialForm()
    credentialStore.fetchCredentials(activeCliType.value as CliType)
  } catch (e: any) {
    notify(getErrorMessage(e, t('providers.notify.saveFailed')), 'error')
  }
}
async function handleCredentialDragEnd() {
  const ids = credentialStore.credentials.map((c) => c.id)
  await credentialStore.reorderCredentials(ids, activeCliType.value)
  notify(t('providers.notify.orderSaved'))
}

const now = ref(Date.now())
let timer: ReturnType<typeof setInterval> | null = null

function handleVisibilityChange() {
  if (document.visibilityState !== 'visible') return
  now.value = Date.now()
  providerStore.expireBlacklists(now.value / 1000)
}
function getUnblacklistTime(provider: Provider): string {
  if (!provider.is_blacklisted || !provider.blacklisted_until) return t('providers.status.blacklisted')
  const diffSeconds = provider.blacklisted_until - (now.value / 1000)
  if (diffSeconds <= 0) return t('providers.status.released')
  const mins = Math.floor(diffSeconds / 60)
  return mins === 0 ? t('providers.status.releaseInSeconds', { seconds: Math.ceil(diffSeconds) }) : t('providers.status.releaseInMinutes', { minutes: mins, seconds: Math.ceil(diffSeconds % 60) })
}

onMounted(async () => {
  // 先挂监听再拉列表，避免首屏拉取到监听生效之间漏掉熔断事件
  healthListener = await providersApi.listenHealthChanges((health) => providerStore.applyHealthEvent(health))
  await Promise.all([
    agentStore.agents.length ? Promise.resolve() : agentStore.fetchAgents(),
    settingsStore.fetchSettings(),
  ])
  if (!cliTabs.value.some((tab) => tab.id === activeCliType.value) && cliTabs.value.length) {
    activeCliType.value = cliTabs.value[0].id
  }
  await loadProfiles()
  const profile = await ensureCurrentProfileOrFallback()
  providerStore.fetchProviders(activeCliType.value as CliType, profile)
  loadPriceSyncStatus()
  if (canUseOfficialCredentials.value) {
    credentialStore.fetchCredentials(activeCliType.value as CliType)
  }
  document.addEventListener('visibilitychange', handleVisibilityChange)
  timer = setInterval(() => {
    if (document.visibilityState !== 'visible') return
    now.value = Date.now()
    providerStore.expireBlacklists(now.value / 1000)
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
  if (healthListener) {
    healthListener()
    healthListener = null
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
.toolbar-right { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }

.prov-body { flex: 1; min-height: 0; overflow: auto; }
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
.pt-cols.m-cred { grid-template-columns: 72px repeat(3, minmax(80px, 1fr)) 72px; }
.pt-cols > div { text-align: center; min-width: 0; }
.pt-cols.m-route > div:nth-child(2),
.pt-cols.m-cred > div:nth-child(2) { text-align: left; }
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
.pt-row:hover { background: var(--v2-row-hover-bg); }
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
