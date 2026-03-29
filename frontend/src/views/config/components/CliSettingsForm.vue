<template>
  <div class="cli-form">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-code" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>
        </symbol>
        <symbol id="icon-rotate" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>
        </symbol>
        <symbol id="icon-save" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/>
        </symbol>
      </defs>
    </svg>

    <div class="form-section">
      <div class="section-label">配置目录</div>
      <div class="input-group">
        <input type="text" v-model="form.config_dir" class="f-input" placeholder="CLI 配置目录">
        <button class="f-button ghost-plain" @click="handleResetDir" title="恢复默认">
          <svg width="14" height="14" style="margin-right: 4px;"><use href="#icon-rotate"/></svg>
          重置
        </button>
      </div>
    </div>
    
    <div class="form-section editor-section">
      <div class="editor-header">
        <div class="section-label" style="margin-bottom: 0;">预设配置</div>
        <button v-if="isJsonFormat" class="f-button ghost-plain sm" @click="formatJson">
          <svg width="12" height="12" style="margin-right: 4px;"><use href="#icon-code"/></svg>
          格式化
        </button>
      </div>
      <textarea 
        v-model="form.default_json_config"
        @blur="validateConfig"
        class="f-textarea mono"
        :placeholder="placeholder"
      ></textarea>
      <div v-if="validationError" class="error-text">{{ validationError }}</div>
      <div class="hint-text">{{ tip }}</div>
    </div>
    
    </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { notify } from '@/utils/notification'
import type { CliSettings } from '@/types/models'
import { validateJson, formatJson as formatJsonUtil } from '@/utils/json'

const props = defineProps<{
  cliType: string
  settings?: CliSettings
}>()

const emit = defineEmits<{
  save: [cliType: string, data: { default_json_config: string; config_dir: string }]
}>()

const form = ref({
  default_json_config: '',
  config_dir: ''
})

const defaultConfigDir = ref('')
const validationError = ref('')

const placeholder = computed(() => {
  switch (props.cliType) {
    case 'codex':
      return `model_reasoning_effort = "high"\nmodel_reasoning_summary = "detailed"`
    case 'claude_code':
      return `{\n  "env": {},\n  "permissions": {}\n}`
    case 'gemini':
      return `{\n  "theme": "dark"\n}`
    default:
      return '{}'
  }
})

const tip = computed(() => {
  switch (props.cliType) {
    case 'codex':
      return '此处配置将合并到 config.toml（TOML 格式）'
    case 'claude_code':
      return '此处配置将合并到 settings.json（JSON 格式）'
    case 'gemini':
      return '此处配置将合并到 settings.json（JSON 格式）'
    default:
      return '此处配置将合并到 CLI 的配置文件中'
  }
})

const isJsonFormat = computed(() => props.cliType === 'claude_code' || props.cliType === 'gemini')

watch(() => props.settings, (settings) => {
  if (settings) {
    form.value = {
      default_json_config: settings.default_json_config,
      config_dir: settings.config_dir
    }
    defaultConfigDir.value = settings.default_config_dir
  }
}, { immediate: true })

function handleResetDir() {
  form.value.config_dir = defaultConfigDir.value
}

function validateConfig() {
  validationError.value = ''
  const config = form.value.default_json_config.trim()
  if (!config) return true

  if (props.cliType === 'claude_code' || props.cliType === 'gemini') {
    validationError.value = validateJson(config)
    return !validationError.value
  }

  if (props.cliType === 'codex') {
    if (config.includes('{') || (config.includes('[') && config.includes(']') && config.includes(','))) {
      validationError.value = 'TOML 格式错误: 请使用 TOML 格式而非 JSON 格式'
      return false
    }
  }

  return true
}

function formatJson() {
  const result = formatJsonUtil(form.value.default_json_config)
  if (result === form.value.default_json_config) {
    validationError.value = validateJson(form.value.default_json_config)
  } else {
    form.value.default_json_config = result
    validationError.value = ''
  }
}

function handleSave() {
  if (!validateConfig()) {
    notify('配置格式错误，请修正后再保存', 'error')
    return
  }
  emit('save', props.cliType, form.value)
}

defineExpose({ handleSave })
</script>

<style scoped>
.cli-form { display: flex; flex-direction: column; height: 100%; }

.form-section { margin-bottom: 28px; }
.editor-section { flex: 1; display: flex; flex-direction: column; min-height: 0; }

.section-label { font-size: 14px; font-weight: 700; color: #475569; margin-bottom: 12px; letter-spacing: -0.2px; }

.input-group { display: flex; gap: 12px; }

.f-input { 
  flex: 1; padding: 10px 14px; background: #ffffff; border: 1px solid #e2e8f0; 
  border-radius: 10px; font-size: 14px; color: #0f172a; outline: none; transition: all 0.2s; 
}
.f-input:focus { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); }

.editor-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }

.f-textarea { 
  flex: 1; min-height: 240px; padding: 16px; border: 1px solid #e2e8f0; border-radius: 12px; 
  font-size: 13px; background: #f8fafc; color: #0f172a; resize: none; 
  outline: none; transition: all 0.2s; line-height: 1.6;
}
.f-textarea:focus { border-color: #0ea5e9; background: #ffffff; }

.mono { font-family: "JetBrains Mono", "Cascadia Code", monospace; }

.hint-text { font-size: 12px; color: #94a3b8; margin-top: 8px; font-weight: 500; }
.error-text { font-size: 12px; color: #f43f5e; margin-top: 8px; font-weight: 600; }

/* Buttons */
.f-button { 
  background: #0ea5e9; color: #ffffff; border: none; padding: 12px 24px; border-radius: 10px; 
  font-size: 14px; font-weight: 600; cursor: pointer; display: flex; align-items: center; 
  transition: background 0.2s; 
}
.f-button:hover { background: #0284c7; }

.f-button.ghost-plain { background: transparent; color: #64748b; padding: 8px 12px; font-size: 13px; font-weight: 600; border-radius: 8px; }
.f-button.ghost-plain:hover { color: #0f172a; background: #f1f5f9; }
.f-button.ghost-plain.sm { padding: 4px 8px; font-size: 12px; }
</style>
