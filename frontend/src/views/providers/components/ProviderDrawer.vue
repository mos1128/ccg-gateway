<template>
  <V2Drawer v-model="visible" :title="title" @confirm="emit('confirm')">
    <div v-if="remark" class="dr-agent-remark">
      <el-icon><InfoFilled /></el-icon>
      <span>{{ remark }}</span>
    </div>

    <V2Tabs>
      <div v-for="item in tabs" :key="item.id" class="v2-tab" :class="{ active: tab === item.id }" @click="tab = item.id">{{ item.label }}</div>
    </V2Tabs>

    <div v-show="tab === 'basic'">
      <div v-if="protocols.length > 1" class="v2-field">
        <label class="v2-label">{{ t('providers.form.protocolLabel') }} <span class="req">*</span></label>
        <AppSelect
          :model-value="form.protocol"
          :options="protocolOptions"
          width="100%"
          @change="value => form.protocol = value as Protocol"
        />
        <div v-if="translated" class="dr-translate-note">{{ t('providers.form.translateNote') }}</div>
      </div>
      <!-- 选了带 [转换] 标记的端点类型才真走转换，也才用得上这个兜底值 -->
      <div v-if="translated" class="v2-field">
        <label class="v2-label">{{ t('providers.form.translateMaxTokensLabel') }}</label>
        <input v-model.number="form.translate_max_tokens" type="number" min="1024" step="1024" class="v2-input">
        <div class="v2-hint">{{ t('providers.form.translateMaxTokensHint') }}</div>
      </div>
      <div class="v2-field">
        <label class="v2-label">{{ t('providers.form.nameLabel') }} <span class="req">*</span></label>
        <input v-model="form.name" type="text" class="v2-input" :placeholder="t('providers.form.namePlaceholder')">
      </div>
      <div class="v2-field">
        <label class="v2-label">{{ t('providers.form.baseUrlLabel') }} <span class="req">*</span></label>
        <input v-model="form.base_url" type="text" class="v2-input" :placeholder="baseUrlPlaceholder">
      </div>
      <div class="v2-field">
        <label class="v2-label">{{ t('providers.form.apiKeyLabel') }} <span class="req">*</span></label>
        <div class="v2-input-wrapper">
          <input v-model="form.api_key" :type="showApiKey ? 'text' : 'password'" class="v2-input" placeholder="sk-...">
          <el-tooltip :content="showApiKey ? t('providers.form.hideToken') : t('providers.form.showToken')" placement="top" effect="light" :show-after="250">
            <button type="button" class="v2-input-icon-btn" @click="showApiKey = !showApiKey">
              <svg v-if="showApiKey" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
              <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
          </el-tooltip>
        </div>
      </div>
      <div class="v2-field">
        <label class="v2-label">{{ t('providers.form.userAgentLabel') }}</label>
        <input v-model="form.custom_useragent" type="text" class="v2-input" :placeholder="t('providers.form.userAgentPlaceholder')">
      </div>
    </div>

    <div v-show="tab === 'model'">
      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title">{{ t('providers.models.available') }}</div>
            <el-tooltip v-if="modelSyncError" placement="top" effect="light" :show-after="250" :enterable="true" popper-class="v2-profile-pop v2-scope">
              <template #content>
                <div class="profile-help">
                  <div class="tooltip-item dr-sync-error-text">{{ modelSyncHint }}</div>
                </div>
              </template>
              <div class="dr-group-hint is-error">{{ modelSyncHint }}</div>
            </el-tooltip>
            <div v-else class="dr-group-hint">{{ modelSyncHint }}</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" :disabled="!canSyncModels || modelSyncLoading" @click="emit('sync-models')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg>
            {{ modelSyncLoading ? t('providers.models.syncing') : t('providers.models.sync') }}
          </button>
        </div>
        <div class="dr-group-body">
          <div v-if="providerModels.length" class="dr-model-list">
            <div v-for="model in providerModels" :key="model.id" class="dr-model-row">
              <el-tooltip :content="model.source === 'manual' ? t('providers.models.tipManual') : t('providers.models.tipAuto')" placement="top" effect="light" :show-after="250">
                <span class="dr-model-dot" :class="{ manual: model.source === 'manual' }"></span>
              </el-tooltip>
              <span class="dr-model-name mono">{{ model.model_name }}</span>
              <button class="v2-x dr-model-x" @click="emit('remove-model', model.id)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </div>
          </div>
          <div v-if="providerModels.length" class="dr-model-legend">
            <span class="dr-model-dot manual"></span>{{ t('providers.models.legendManual') }}
            <span class="dr-model-dot"></span>{{ t('providers.models.legendAuto') }}
          </div>
          <div v-else class="dr-empty">
            <span>{{ canSyncModels ? t('providers.models.emptySyncable') : t('providers.models.emptyManual') }}</span>
          </div>
          <div class="dr-map dr-map-single dr-model-add">
            <input
              v-model="manualModel"
              class="v2-input"
              :placeholder="t('providers.models.addPlaceholder')"
              @keydown.enter.prevent="submitManualModel"
            >
            <button class="v2-btn v2-btn-sm v2-btn-outline" :disabled="!manualModel.trim()" @click="submitManualModel">{{ t('common.add') }}</button>
          </div>
        </div>
      </div>

      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title-wrapper">
              <span class="dr-group-title">{{ t('providers.modelMap.title') }}</span>
              <el-tooltip effect="light" placement="top" :show-after="150" popper-class="v2-profile-pop v2-scope">
                <template #content>
                  <div class="profile-help">
                    <div class="tooltip-title">{{ t('providers.modelMap.wildcardTitle') }}</div>
                    <div class="tooltip-item" style="margin-bottom: 4px;"><strong>*</strong>{{ t('providers.modelMap.wildcardAny') }}</div>
                    <div class="tooltip-item" style="margin-bottom: 8px;"><strong>?</strong>{{ t('providers.modelMap.wildcardOne') }}</div>
                    <div class="tooltip-item" style="border-top: 1px solid var(--v2-surface-2); padding-top: 8px; margin-top: 8px;">
                      <strong>{{ t('providers.modelMap.wildcardExample') }}</strong><code>*opus*</code> → <code>gml-5</code>
                      <div class="v2-hint" style="margin-top: 4px; line-height: 1.4;">{{ t('providers.modelMap.wildcardExampleDesc') }}</div>
                    </div>
                  </div>
                </template>
                <span class="v2-help">
                  <el-icon><InfoFilled /></el-icon>
                </span>
              </el-tooltip>
            </div>
            <div class="dr-group-hint">{{ t('providers.modelMap.hint') }}</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" @click="emit('add-model-map')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            {{ t('common.add') }}
          </button>
        </div>
        <div class="dr-group-body">
          <div v-for="(map, index) in form.model_maps" :key="'m' + index" class="dr-map">
            <input v-model="map.source_model" class="v2-input" :placeholder="t('providers.modelMap.sourcePlaceholder')">
            <span class="dr-arrow">→</span>
            <el-autocomplete
              v-model="map.target_model"
              :fetch-suggestions="modelSuggestions"
              :debounce="0"
              :suffix-icon="ArrowDown"
              fit-input-width
              highlight-first-item
              class="dr-model-target"
              :placeholder="t('providers.modelMap.targetPlaceholder')"
            />
            <el-tooltip :content="t('common.delete')" placement="top" effect="light" :show-after="250">
              <button class="v2-x" @click="emit('remove-model-map', index)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </el-tooltip>
          </div>
          <div v-if="!form.model_maps.length" class="dr-empty">
            <span>{{ t('providers.modelMap.empty') }}</span>
          </div>
        </div>
      </div>

      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title">{{ t('providers.modelBlacklist.title') }}</div>
            <div class="dr-group-hint">{{ t('providers.modelBlacklist.hint') }}</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" @click="emit('add-model-blacklist')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            {{ t('common.add') }}
          </button>
        </div>
        <div class="dr-group-body">
          <div v-for="(item, index) in form.model_blacklist" :key="'b' + index" class="dr-map dr-map-single">
            <input v-model="item.model_pattern" class="v2-input" :placeholder="t('providers.modelBlacklist.patternPlaceholder')">
            <el-tooltip :content="t('common.delete')" placement="top" effect="light" :show-after="250">
              <button class="v2-x" @click="emit('remove-model-blacklist', index)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </el-tooltip>
          </div>
          <div v-if="!form.model_blacklist.length" class="dr-empty">
            <span>{{ t('providers.modelBlacklist.empty') }}</span>
          </div>
        </div>
      </div>
    </div>

    <div v-show="tab === 'advanced'">
      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title-wrapper">
              <span class="dr-group-title">{{ t('providers.tiers.title') }}</span>
              <el-tooltip effect="light" placement="top" :show-after="150" popper-class="v2-profile-pop v2-scope">
                <template #content>
                  <div class="profile-help">
                    <div class="tooltip-title">{{ t('providers.tiers.tipTitle') }}</div>
                    <div class="tooltip-item" style="margin-bottom: 4px;">{{ t('providers.tiers.tipRetry') }}</div>
                    <div class="tooltip-item" style="margin-bottom: 4px;">{{ t('providers.tiers.tipReach') }}</div>
                    <div class="tooltip-item" style="margin-bottom: 4px;"><strong>401/403/404</strong>{{ t('providers.tiers.tipAuthCode') }}</div>
                    <div class="tooltip-item" style="margin-bottom: 4px;"><strong>400/413/422</strong>{{ t('providers.tiers.tipClientCode') }}</div>
                    <div class="tooltip-item" style="border-top: 1px solid var(--v2-surface-2); padding-top: 8px; margin-top: 8px;">
                      {{ t('providers.tiers.tipKeepPrefix') }}<strong>{{ t('providers.tiers.tipClear') }}</strong>{{ t('providers.tiers.tipRepeatSuffix') }}
                    </div>
                  </div>
                </template>
                <span class="v2-help">
                  <el-icon><InfoFilled /></el-icon>
                </span>
              </el-tooltip>
            </div>
            <div class="dr-group-hint">{{ t('providers.tiers.hint') }}</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" @click="emit('add-blacklist-tier')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            {{ t('common.add') }}
          </button>
        </div>
        <div class="dr-group-body">
          <div v-for="(tier, index) in form.blacklist_tiers" :key="'t' + index" class="dr-map dr-tier-row">
            <span class="dr-tier-label">{{ t('providers.tiers.failureLabel') }}</span>
            <div class="v2-input-wrapper">
              <input v-model.number="tier.failure_count" type="number" min="1" max="50" class="v2-input">
              <span class="v2-input-unit">{{ t('providers.tiers.failureUnit') }}</span>
            </div>
            <span class="dr-arrow">→</span>
            <span class="dr-tier-label">{{ t('providers.tiers.blacklistLabel') }}</span>
            <div class="v2-input-wrapper">
              <input v-model.number="tier.blacklist_minutes" type="number" min="1" max="10080" class="v2-input">
              <span class="v2-input-unit">{{ t('providers.tiers.minutesUnit') }}</span>
            </div>
            <el-tooltip :content="t('common.delete')" placement="top" effect="light" :show-after="250">
              <button class="v2-x" @click="emit('remove-blacklist-tier', index)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </el-tooltip>
          </div>
          <div v-if="!form.blacklist_tiers.length" class="dr-empty">
            <span>{{ t('providers.tiers.empty') }}</span>
          </div>
        </div>
      </div>
      <div class="dr-sec-title dr-price-title">{{ t('providers.billing.title') }}</div>
      <div class="v2-field dr-multiplier-field">
        <label class="v2-label">{{ t('providers.billing.multiplierLabel') }}</label>
        <div class="v2-input-wrapper">
          <input v-model.number="form.price_multiplier" type="number" min="0.01" step="0.01" class="v2-input">
          <span class="v2-input-unit">×</span>
        </div>
        <div class="v2-hint">{{ t('providers.billing.multiplierHint') }}</div>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2Tabs from '@/components/V2Tabs.vue'
import AppSelect from '@/components/AppSelect.vue'
import { useI18n } from 'vue-i18n'
import { ArrowDown, InfoFilled } from '@element-plus/icons-vue'
import { CONVERTIBLE_PROTOCOLS, PROTOCOL_LABELS } from '@/types/models'
import type { Protocol, ProviderModelsResponse } from '@/types/models'

interface ProviderEditForm {
  protocol: Protocol | ''
  name: string
  base_url: string
  api_key: string
  custom_useragent: string
  price_multiplier: number
  translate_max_tokens: number
  model_maps: Array<{ source_model: string; target_model: string; enabled: boolean }>
  model_blacklist: Array<{ model_pattern: string }>
  blacklist_tiers: Array<{ failure_count: number; blacklist_minutes: number }>
}

const props = defineProps<{
  modelValue: boolean
  title: string
  form: ProviderEditForm
  baseUrlPlaceholder: string
  protocols: Protocol[]
  declaredProtocols: Protocol[]
  remark?: string | null
  modelSync?: ProviderModelsResponse
  modelSyncLoading?: boolean
  canSyncModels?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: []
  'add-model-map': []
  'remove-model-map': [index: number]
  'add-model-blacklist': []
  'remove-model-blacklist': [index: number]
  'add-blacklist-tier': []
  'remove-blacklist-tier': [index: number]
  'sync-models': []
  'add-model': [modelName: string]
  'remove-model': [modelId: number]
}>()

const { t } = useI18n()

const tabs = computed(() => [
  { id: 'basic', label: t('providers.tabs.basic') },
  { id: 'model', label: t('providers.tabs.model') },
  { id: 'advanced', label: t('providers.tabs.advanced') }
])
const tab = ref('basic')
const showApiKey = ref(false)
const manualModel = ref('')
// Agent 不支持但可转换的端点类型：选中才会真走转换，Gemini 只能透传。
function needsTranslate(protocol: Protocol): boolean {
  return CONVERTIBLE_PROTOCOLS.includes(protocol)
    && props.declaredProtocols.some((declared) => CONVERTIBLE_PROTOCOLS.includes(declared))
    && !props.declaredProtocols.includes(protocol)
}
const protocolOptions = computed(() => props.protocols.map((protocol) => ({
  value: protocol,
  label: needsTranslate(protocol) ? `${t('providers.form.convertTag')}${PROTOCOL_LABELS[protocol]}` : PROTOCOL_LABELS[protocol],
})))
const translated = computed(() => needsTranslate(props.form.protocol as Protocol))

const providerModels = computed(() => props.modelSync?.models ?? [])
const modelOptions = computed(() => providerModels.value
  .filter((model) => model.enabled === true || model.enabled === 1)
  .map((model) => ({ value: model.model_name })))
function modelSuggestions(queryString: string) {
  const keyword = queryString.toLowerCase()
  return modelOptions.value.filter((option) => option.value.toLowerCase().includes(keyword))
}
const modelSyncHint = computed(() => {
  if (!props.canSyncModels) return t('providers.models.hintAfterSave')
  const state = props.modelSync?.sync_state
  if (state?.last_error) return t('providers.models.hintLastSyncFailed', { error: state.last_error })
  if (state?.last_success_at) return t('providers.models.hintLastSync', { time: new Date(state.last_success_at * 1000).toLocaleString() })
  return t('providers.models.hintSync')
})
const modelSyncError = computed(() => props.canSyncModels && !!props.modelSync?.sync_state?.last_error)

function submitManualModel() {
  const modelName = manualModel.value.trim()
  if (!modelName) return
  emit('add-model', modelName)
  manualModel.value = ''
}

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

watch(() => props.modelValue, (open) => {
  if (open) {
    tab.value = 'basic'
    showApiKey.value = false
    manualModel.value = ''
  }
})
</script>

<style scoped>
.dr-agent-remark {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: var(--v2-space-4);
  padding: 10px 12px;
  border-left: 2px solid var(--v2-accent);
  border-radius: 4px;
  background: var(--v2-surface-2);
  color: var(--v2-text-2);
  font-size: var(--v2-fs-sm);
  line-height: 1.5;
}
.dr-agent-remark .el-icon {
  flex: 0 0 auto;
  margin-top: 2px;
  color: var(--v2-text-3);
  font-size: 14px;
}
.dr-agent-remark span {
  min-width: 0;
  overflow-wrap: anywhere;
  white-space: pre-line;
}
.dr-translate-note { margin-top: 6px; color: var(--v2-warning); font-size: var(--v2-fs-xs); line-height: 1.5; }
.dr-sec-title { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-semibold); color: var(--v2-text); }
.dr-map { display: grid; grid-template-columns: 1fr auto 1fr auto; gap: 9px; align-items: center; }
.dr-map-single { grid-template-columns: 1fr auto; }
.dr-tier-row { grid-template-columns: auto 1fr auto auto 1fr auto; gap: 8px; }
.dr-tier-label { color: var(--v2-text-2); font-size: var(--v2-fs-sm); white-space: nowrap; }
.dr-tier-row .v2-input-wrapper { min-width: 0; }
.dr-arrow { color: var(--v2-text-3); font-size: var(--v2-fs-sm); }
.dr-map :deep(.dr-model-target) { width: 100%; min-width: 0; }
.dr-map :deep(.dr-model-target .el-input__wrapper) {
  height: 30px;
  padding: 0 12px;
  border-radius: var(--v2-r);
  background: var(--v2-bg-base);
  box-shadow: none !important;
}
.dr-map :deep(.dr-model-target .el-input__inner) {
  color: var(--v2-text);
  font-size: 13px;
  font-weight: var(--v2-fw-medium);
}

.dr-model-list { display: flex; flex-wrap: wrap; gap: 6px; max-height: 232px; overflow-y: auto; }
.dr-model-row { display: inline-flex; align-items: center; gap: 4px; padding: 2px 4px 2px 8px; border-radius: var(--v2-r-sm); background: var(--v2-bg-base); max-width: 100%; }
.dr-model-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--v2-fs-sm); color: var(--v2-text); }
.dr-model-dot { width: 6px; height: 6px; border-radius: var(--v2-r-full, 999px); background: var(--v2-text-3); flex-shrink: 0; cursor: help; }
.dr-model-dot.manual { background: var(--v2-accent); }
.dr-model-legend { display: flex; align-items: center; gap: 5px; margin-top: 8px; font-size: var(--v2-fs-xs); color: var(--v2-text-3); }
.dr-model-legend .dr-model-dot { cursor: default; }
.dr-model-legend > span:not(:first-child) { margin-left: 12px; }
.dr-model-x { width: 20px; height: 20px; border: none; background: transparent; flex-shrink: 0; }
.dr-model-add { padding-top: 4px; }

.dr-group-card {
  border: 1px solid rgba(0, 0, 0, 0.045);
  border-radius: var(--v2-r-lg);
  background: var(--v2-surface);
  box-shadow: none;
  overflow: hidden;
  margin-bottom: 20px;
}
html.dark .dr-group-card {
  border-color: rgba(255, 255, 255, 0.04);
}
.dr-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: var(--v2-surface-2);
  border-bottom: 1px solid var(--v2-surface-3);
}
.dr-group-header > div:first-child { min-width: 0; }
.dr-group-header > button { flex-shrink: 0; }
.dr-group-title-wrapper {
  display: flex;
  align-items: center;
  gap: 6px;
}
.dr-group-title {
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text);
}

.dr-group-hint {
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  margin-top: 2px;
  overflow-wrap: anywhere;
}
.dr-group-hint.is-error {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  cursor: help;
}
.dr-sync-error-text {
  overflow-wrap: anywhere;
  max-height: 220px;
  overflow: auto;
}
.dr-group-body {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 104px;
}
.dr-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--v2-text-3);
  font-size: var(--v2-fs-xs);
}
.dr-price-title { margin: 22px 0 12px; display: flex; align-items: baseline; gap: 8px; }
.dr-price-title .v2-hint { margin-top: 0; }
.dr-multiplier-field { max-width: 240px; }

.v2-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}
.v2-input-wrapper .v2-input {
  padding-right: 36px;
}
.v2-input-icon-btn {
  position: absolute;
  right: 10px;
  background: transparent;
  border: none;
  color: var(--v2-text-3);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: color 0.15s;
}
.v2-input-icon-btn:hover {
  color: var(--v2-text);
}
.v2-input-unit {
  position: absolute;
  right: 12px;
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  pointer-events: none;
}
</style>
