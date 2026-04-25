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
      <div class="section-label">CLI 目录</div>
      <div class="input-group">
        <input type="text" v-model="form.config_dir" class="b-input" placeholder="CLI 配置目录">
        <button class="f-button ghost-plain" @click="handleResetDir" title="恢复默认">
          <svg width="14" height="14" style="margin-right: 4px;"><use href="#icon-rotate"/></svg>
        </button>
      </div>
    </div>
    
    <div class="form-section editor-section">
      <div class="editor-header">
        <div class="section-label" style="margin-bottom: 0;">全局预设</div>
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

    <div class="form-section write-mode-section">
      <div class="write-mode-row">
        <div class="frost-segmented write-mode-segmented">
          <div class="seg-item" :class="{ active: form.config_write_mode === 'merge' }" @click="form.config_write_mode = 'merge'">增量合并</div>
          <div class="seg-item" :class="{ active: form.config_write_mode === 'overwrite' }" @click="form.config_write_mode = 'overwrite'">全量写入</div>
        </div>
        <el-tooltip
          effect="light"
          placement="top"
          :fallback-placements="['bottom', 'top', 'right', 'left']"
          :offset="10"
          :show-after="150"
          :enterable="true"
          popper-class="write-mode-help-popper"
        >
          <template #content>
            <div class="write-mode-help-content">
              <div class="tooltip-title">配置写入模式</div>
              <div class="tooltip-item">
                <strong>增量合并</strong>
                <span>只写入需要变更的字段，保留配置文件中已有的其他配置（如 MCP / plugin 开关等配置）。</span>
              </div>
              <div class="tooltip-item">
                <strong>全量写入</strong>
                <span>每次写入时完全覆盖配置文件。中转模式会备份原始文件，关闭时自动恢复。保持配置干净，强迫症适用。</span>
              </div>
            </div>
          </template>
          <div class="help-icon-wrapper">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="help-icon">
              <circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </div>
        </el-tooltip>
        <div style="flex: 1;"></div>
        <button class="save-button" @click="handleSave">
          <svg width="16" height="16" style="margin-right: 6px;"><use href="#icon-save"/></svg>
          保存
        </button>
      </div>
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
  save: [cliType: string, data: { default_json_config: string; config_dir: string; config_write_mode: string }]
}>()

const form = ref({
  default_json_config: '',
  config_dir: '',
  config_write_mode: 'merge' as 'overwrite' | 'merge'
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
      config_dir: settings.config_dir,
      config_write_mode: settings.config_write_mode || 'merge'
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

.section-label { font-size: var(--fs-14); font-weight: var(--fw-500); color: var(--color-text); margin-bottom: 12px; letter-spacing: -0.2px; }

.input-group { display: flex; gap: 12px; }

.editor-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }

.f-textarea {
  flex: 1; min-height: 240px; padding: 16px; border: 1px solid var(--color-border); border-radius: 12px;
  font-size: var(--fs-14); background: var(--color-bg-page); color: var(--color-text); resize: none;
  outline: none; transition: all 0.2s; line-height: 1.6;
  word-break: break-all;
}
.f-textarea:focus { border-color: var(--color-primary); background: var(--color-bg); }

.hint-text { font-size: var(--fs-12); color: var(--color-text-weak); margin-top: 8px; font-weight: var(--fw-400); }
.error-text { font-size: var(--fs-12); color: var(--color-error); margin-top: 8px; font-weight: var(--fw-400); }

/* Buttons */
.f-button {
  background: var(--color-primary); color: var(--color-bg); border: none; padding: 12px 24px; border-radius: 10px;
  font-size: var(--fs-14); font-weight: var(--fw-600); cursor: pointer; display: flex; align-items: center;
  transition: background 0.2s;
}
.f-button:hover { background: var(--color-primary-hover); }

.f-button.ghost-plain { background: transparent; color: var(--color-text-muted); padding: 8px 12px; font-size: var(--fs-14); font-weight: var(--fw-600); border-radius: 8px; }
.f-button.ghost-plain:hover { color: var(--color-text); background: var(--color-bg-subtle); }
.f-button.ghost-plain.sm { padding: 4px 8px; font-size: var(--fs-12); }

/* Write mode section */
.write-mode-section { margin-top: 4px; margin-bottom: 0; flex-shrink: 0; }

.write-mode-row { display: flex; align-items: center; gap: 10px; }

.frost-segmented {
  display: inline-flex;
  background: var(--color-bg-subtle);
  border-radius: 10px;
  padding: 3px;
  gap: 2px;
}

.frost-segmented .seg-item {
  padding: 6px 14px;
  font-size: var(--fs-14);
  font-weight: var(--fw-600);
  color: var(--color-text-secondary);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
  user-select: none;
}

.frost-segmented .seg-item:hover { color: var(--color-text); }

.frost-segmented .seg-item.active {
  background: var(--color-bg);
  color: var(--color-primary);
  box-shadow: 0 1px 3px var(--color-shadow-sm);
}

.write-mode-segmented { flex-shrink: 0; }

/* Help icon */
.help-icon-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  cursor: help;
}

.help-icon {
  color: var(--color-text-weak);
  transition: color 0.2s;
}

.help-icon-wrapper:hover .help-icon { color: var(--color-text-muted); }

.write-mode-help-content {
  width: 300px;
}

:global(.write-mode-help-popper.el-popper) {
  border-radius: 12px;
  padding: 16px;
  box-shadow: 0 8px 24px var(--color-shadow-lg);
}

.tooltip-title {
  font-size: var(--fs-14);
  font-weight: var(--fw-700);
  color: var(--color-text);
  margin-bottom: 10px;
}

.tooltip-item {
  margin-bottom: 8px;
  font-size: var(--fs-12);
  line-height: 1.5;
  color: var(--color-text-muted);
}

.tooltip-item:last-child { margin-bottom: 0; }

.tooltip-item strong {
  display: block;
  color: var(--color-text-dark);
  font-weight: var(--fw-600);
  margin-bottom: 2px;
}

/* Save button */
.save-button {
  background: var(--color-primary-10); color: var(--color-primary); border: none; padding: 6px 14px; border-radius: 10px;
  font-size: var(--fs-14); font-weight: var(--fw-600); cursor: pointer; display: flex; align-items: center;
  transition: all 0.2s; flex-shrink: 0;
}
.save-button:hover { background: var(--color-primary-20); }
</style>
