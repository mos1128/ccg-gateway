<template>
  <V2Drawer v-model="visible" :title="title" @confirm="emit('confirm')">
    <div v-if="remark" class="dr-agent-remark">
      <el-icon><InfoFilled /></el-icon>
      <span>{{ remark }}</span>
    </div>

    <V2Tabs>
      <div v-for="t in tabs" :key="t.id" class="v2-tab" :class="{ active: tab === t.id }" @click="tab = t.id">{{ t.label }}</div>
    </V2Tabs>

    <div v-show="tab === 'basic'">
      <div v-if="protocols.length > 1" class="v2-field">
        <label class="v2-label">端点类型 <span class="req">*</span></label>
        <AppSelect
          :model-value="form.protocol"
          :options="protocolOptions"
          width="100%"
          @change="value => form.protocol = value as Protocol"
        />
      </div>
      <div class="v2-field">
        <label class="v2-label">服务商名称 <span class="req">*</span></label>
        <input v-model="form.name" type="text" class="v2-input" placeholder="例如：OpenAI 官方">
      </div>
      <div class="v2-field">
        <label class="v2-label">服务地址 <span class="req">*</span></label>
        <input v-model="form.base_url" type="text" class="v2-input" :placeholder="baseUrlPlaceholder">
      </div>
      <div class="v2-field">
        <label class="v2-label">API 密钥 <span class="req">*</span></label>
        <div class="v2-input-wrapper">
          <input v-model="form.api_key" :type="showApiKey ? 'text' : 'password'" class="v2-input" placeholder="sk-...">
          <el-tooltip :content="showApiKey ? '隐藏 Token' : '显示 Token'" placement="top" effect="light" :show-after="250">
            <button type="button" class="v2-input-icon-btn" @click="showApiKey = !showApiKey">
              <svg v-if="showApiKey" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
              <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
          </el-tooltip>
        </div>
      </div>
      <div class="v2-field">
        <label class="v2-label">自定义 User-Agent</label>
        <input v-model="form.custom_useragent" type="text" class="v2-input" placeholder="留空则使用原始 UA">
      </div>
    </div>

    <div v-show="tab === 'model'">
      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title">可用模型</div>
            <div class="dr-group-hint">{{ modelSyncHint }}</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" :disabled="!canSyncModels || modelSyncLoading" @click="emit('sync-models')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg>
            {{ modelSyncLoading ? '同步中' : '同步' }}
          </button>
        </div>
        <div class="dr-group-body">
          <div v-if="providerModels.length" class="dr-model-list">
            <div v-for="model in providerModels" :key="model.id" class="dr-model-row">
              <el-tooltip :content="model.source === 'manual' ? '手动添加，同步时保留' : '自动同步'" placement="top" effect="light" :show-after="250">
                <span class="dr-model-dot" :class="{ manual: model.source === 'manual' }"></span>
              </el-tooltip>
              <span class="dr-model-name mono">{{ model.model_name }}</span>
              <button class="v2-x dr-model-x" @click="emit('remove-model', model.id)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </div>
          </div>
          <div v-if="providerModels.length" class="dr-model-legend">
            <span class="dr-model-dot manual"></span>手动（同步时保留）
            <span class="dr-model-dot"></span>自动同步
          </div>
          <div v-else class="dr-empty">
            <span>{{ canSyncModels ? '暂无可用模型，点击同步或手动添加' : '暂无可用模型，可先手动添加' }}</span>
          </div>
          <div class="dr-map dr-map-single dr-model-add">
            <input
              v-model="manualModel"
              class="v2-input"
              placeholder="手动输入模型名称，回车添加"
              @keydown.enter.prevent="submitManualModel"
            >
            <button class="v2-btn v2-btn-sm v2-btn-outline" :disabled="!manualModel.trim()" @click="submitManualModel">添加</button>
          </div>
        </div>
      </div>

      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title-wrapper">
              <span class="dr-group-title">模型映射</span>
              <el-tooltip effect="light" placement="top" :show-after="150" popper-class="v2-profile-pop v2-scope">
                <template #content>
                  <div class="profile-help">
                    <div class="tooltip-title">模型映射通配符规则</div>
                    <div class="tooltip-item" style="margin-bottom: 4px;"><strong>*</strong> ：匹配任意长度的字符</div>
                    <div class="tooltip-item" style="margin-bottom: 8px;"><strong>?</strong> ：匹配单个字符</div>
                    <div class="tooltip-item" style="border-top: 1px solid var(--v2-surface-2); padding-top: 8px; margin-top: 8px;">
                      <strong>示例</strong>：<code>*opus*</code> → <code>gml-5</code>
                      <div class="v2-hint" style="margin-top: 4px; line-height: 1.4;">表示将名称中包含 opus 的模型映射到服务商的 gml-5 模型。</div>
                    </div>
                  </div>
                </template>
                <span class="v2-help">
                  <el-icon><InfoFilled /></el-icon>
                </span>
              </el-tooltip>
            </div>
            <div class="dr-group-hint">将 Agent 请求的源模型名映射为服务商模型，候选模型来自上方「可用模型」</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" @click="emit('add-model-map')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            添加
          </button>
        </div>
        <div class="dr-group-body">
          <div v-for="(map, index) in form.model_maps" :key="'m' + index" class="dr-map">
            <input v-model="map.source_model" class="v2-input" placeholder="Agent 源模型">
            <span class="dr-arrow">→</span>
            <AppSelect
              :model-value="map.target_model"
              :options="modelOptions"
              width="100%"
              placeholder="选择或输入服务商模型"
              filterable
              allow-create
              @change="value => map.target_model = String(value)"
            />
            <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
              <button class="v2-x" @click="emit('remove-model-map', index)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </el-tooltip>
          </div>
          <div v-if="!form.model_maps.length" class="dr-empty">
            <span>暂无模型映射</span>
          </div>
        </div>
      </div>

      <div class="dr-group-card">
        <div class="dr-group-header">
          <div>
            <div class="dr-group-title">模型黑名单</div>
            <div class="dr-group-hint">配置服务商不支持的模型，请求时自动跳过</div>
          </div>
          <button class="v2-btn v2-btn-sm v2-btn-outline" @click="emit('add-model-blacklist')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            添加
          </button>
        </div>
        <div class="dr-group-body">
          <div v-for="(item, index) in form.model_blacklist" :key="'b' + index" class="dr-map dr-map-single">
            <input v-model="item.model_pattern" class="v2-input" placeholder="模型名称（支持 * ?）">
            <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
              <button class="v2-x" @click="emit('remove-model-blacklist', index)"><svg width="14" height="14" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
            </el-tooltip>
          </div>
          <div v-if="!form.model_blacklist.length" class="dr-empty">
            <span>暂无黑名单模型</span>
          </div>
        </div>
      </div>
    </div>

    <div v-show="tab === 'advanced'">
      <div class="dr-sec-title dr-price-title" style="margin-top: 0;">
        <span>容错配置</span>
        <el-tooltip effect="light" placement="top" :show-after="150" popper-class="v2-profile-pop v2-scope">
          <template #content>
            <div class="profile-help">
              <div class="tooltip-title">重试与熔断规则</div>
              <div class="tooltip-item" style="margin-bottom: 4px;">请求失败后先在当前服务商重试，连续失败「连续重试次数」次后切换下一个服务商</div>
              <div class="tooltip-item" style="margin-bottom: 4px;">所有服务商轮完一圈后从头再轮，直到某次成功或所有服务商熔断</div>
              <div class="tooltip-item" style="margin-bottom: 4px;"><strong>每次失败的尝试都会累计到该服务商的失败计数</strong></div>
              <div class="tooltip-item" style="margin-bottom: 4px;">密钥错误、模型不存在等问题不重试该服务商，直接切换</div>
              <div class="tooltip-item" style="border-top: 1px solid var(--v2-surface-2); padding-top: 8px; margin-top: 8px;">
                连续失败累计达到「失败阈值」后，服务商熔断「熔断时长」分钟，期间请求自动由其他服务商接管
              </div>
            </div>
          </template>
          <span class="v2-help"><el-icon><InfoFilled /></el-icon></span>
        </el-tooltip>
      </div>
      <div class="v2-grid-2">
        <div class="v2-field">
          <label class="v2-label">连续重试次数</label>
          <div class="v2-input-wrapper">
            <input v-model.number="form.retry_limit" type="number" min="1" max="20" class="v2-input">
            <span class="v2-input-unit">次</span>
          </div>
        </div>
        <div class="v2-field">
          <label class="v2-label">失败阈值</label>
          <div class="v2-input-wrapper">
            <input v-model.number="form.failure_threshold" type="number" min="0" class="v2-input">
            <span class="v2-input-unit">次</span>
          </div>
        </div>
        <div class="v2-field">
          <label class="v2-label">熔断时长</label>
          <div class="v2-input-wrapper">
            <input v-model.number="form.blacklist_minutes" type="number" min="0" class="v2-input">
            <span class="v2-input-unit">分钟</span>
          </div>
        </div>
      </div>
      <div class="dr-sec-title dr-price-title">计费配置</div>
      <div class="v2-field dr-multiplier-field">
        <label class="v2-label">模型倍率</label>
        <div class="v2-input-wrapper">
          <input v-model.number="form.price_multiplier" type="number" min="0.01" step="0.01" class="v2-input">
          <span class="v2-input-unit">×</span>
        </div>
        <div class="v2-hint">实际价格 = 全局价目表价格 × 倍率，留空按官方价（1×）计；价目表在列表页「价格」中同步</div>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2Tabs from '@/components/V2Tabs.vue'
import AppSelect, { type AppSelectOption } from '@/components/AppSelect.vue'
import { InfoFilled } from '@element-plus/icons-vue'
import { PROTOCOL_LABELS } from '@/types/models'
import type { Protocol, ProviderModelsResponse } from '@/types/models'

interface ProviderEditForm {
  protocol: Protocol | ''
  name: string
  base_url: string
  api_key: string
  failure_threshold: number
  retry_limit: number
  blacklist_minutes: number
  custom_useragent: string
  price_multiplier: number
  model_maps: Array<{ source_model: string; target_model: string; enabled: boolean }>
  model_blacklist: Array<{ model_pattern: string }>
}

const props = defineProps<{
  modelValue: boolean
  title: string
  form: ProviderEditForm
  baseUrlPlaceholder: string
  protocols: Protocol[]
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
  'sync-models': []
  'add-model': [modelName: string]
  'remove-model': [modelId: number]
}>()

const tabs = [
  { id: 'basic', label: '基本' },
  { id: 'model', label: '模型配置' },
  { id: 'advanced', label: '容错/计费' }
]
const tab = ref('basic')
const showApiKey = ref(false)
const manualModel = ref('')
const protocolOptions = computed(() => props.protocols.map((protocol) => ({
  value: protocol,
  label: PROTOCOL_LABELS[protocol],
})))

const providerModels = computed(() => props.modelSync?.models ?? [])
const modelOptions = computed<AppSelectOption[]>(() => providerModels.value
  .filter((model) => model.enabled === true || model.enabled === 1)
  .map((model) => ({ value: model.model_name, label: model.model_name })))
const modelSyncHint = computed(() => {
  if (!props.canSyncModels) return '可先手动添加，保存服务商后能从接口同步'
  const state = props.modelSync?.sync_state
  if (state?.last_error) return `上次同步失败：${state.last_error}`
  if (state?.last_success_at) return `上次同步：${new Date(state.last_success_at * 1000).toLocaleString()}`
  return '从服务商接口拉取可用模型，也可手动添加'
})

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
.dr-sec-title { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-semibold); color: var(--v2-text); }
.dr-map { display: grid; grid-template-columns: 1fr auto 1fr auto; gap: 9px; align-items: center; }
.dr-map-single { grid-template-columns: 1fr auto; }
.dr-arrow { color: var(--v2-text-3); font-size: var(--v2-fs-sm); }

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
