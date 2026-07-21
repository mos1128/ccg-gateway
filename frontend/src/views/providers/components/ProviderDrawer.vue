<template>
  <V2Drawer v-model="visible" :title="title" @confirm="emit('confirm')">
    <div class="v2-tabs">
      <div v-for="t in tabs" :key="t.id" class="v2-tab" :class="{ active: tab === t.id }" @click="tab = t.id">{{ t.label }}</div>
    </div>

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
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                </span>
              </el-tooltip>
            </div>
            <div class="dr-group-hint">将 Agent 请求的源模型名映射为服务商模型</div>
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
            <input v-model="map.target_model" class="v2-input" placeholder="服务商模型">
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
      <div class="dr-sec-title dr-price-title" style="margin-top: 0;">容错配置</div>
      <div class="v2-grid-2">
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
      <div class="v2-grid-2">
        <div class="v2-field"><label class="v2-label">输入价格 / M</label><input v-model.number="form.input_price_per_m" type="number" min="0" step="0.000001" class="v2-input"></div>
        <div class="v2-field"><label class="v2-label">输出价格 / M</label><input v-model.number="form.output_price_per_m" type="number" min="0" step="0.000001" class="v2-input"></div>
        <div class="v2-field"><label class="v2-label">缓存读取价格 / M</label><input v-model.number="form.cache_read_price_per_m" type="number" min="0" step="0.000001" class="v2-input"></div>
        <div class="v2-field"><label class="v2-label">缓存创建价格 / M</label><input v-model.number="form.cache_creation_price_per_m" type="number" min="0" step="0.000001" class="v2-input"></div>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import AppSelect from '@/components/AppSelect.vue'
import { PROTOCOL_LABELS } from '@/types/models'
import type { Protocol } from '@/types/models'

interface ProviderEditForm {
  protocol: Protocol | ''
  name: string
  base_url: string
  api_key: string
  failure_threshold: number
  blacklist_minutes: number
  custom_useragent: string
  input_price_per_m: number
  output_price_per_m: number
  cache_read_price_per_m: number
  cache_creation_price_per_m: number
  model_maps: Array<{ source_model: string; target_model: string; enabled: boolean }>
  model_blacklist: Array<{ model_pattern: string }>
}

const props = defineProps<{
  modelValue: boolean
  title: string
  form: ProviderEditForm
  baseUrlPlaceholder: string
  protocols: Protocol[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: []
  'add-model-map': []
  'remove-model-map': [index: number]
  'add-model-blacklist': []
  'remove-model-blacklist': [index: number]
}>()

const tabs = [
  { id: 'basic', label: '基本' },
  { id: 'model', label: '模型配置' },
  { id: 'advanced', label: '容错/计费' }
]
const tab = ref('basic')
const showApiKey = ref(false)
const protocolOptions = computed(() => props.protocols.map((protocol) => ({
  value: protocol,
  label: PROTOCOL_LABELS[protocol],
})))

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

watch(() => props.modelValue, (open) => {
  if (open) {
    tab.value = 'basic'
    showApiKey.value = false
  }
})
</script>

<style scoped>
.dr-sec-title { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-semibold); color: var(--v2-text); }
.dr-map { display: grid; grid-template-columns: 1fr auto 1fr auto; gap: 9px; align-items: center; }
.dr-map-single { grid-template-columns: 1fr auto; }
.dr-arrow { color: var(--v2-text-3); font-size: var(--v2-fs-sm); }

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
