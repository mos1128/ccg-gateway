<template>
  <el-form :model="form" label-width="80px">
    <el-form-item label="配置目录">
      <el-input
        v-model="form.config_dir"
        placeholder="CLI 配置目录路径"
      >
        <template #append>
          <el-button @click="handleResetDir">重置</el-button>
        </template>
      </el-input>
      <div class="form-tip">{{ configDirTip }}</div>
    </el-form-item>
    <el-form-item label="默认配置">
      <el-input
        v-model="form.default_json_config"
        type="textarea"
        :rows="10"
        :placeholder="placeholder"
        @blur="validateConfig"
      />
      <div v-if="validationError" class="error-tip">{{ validationError }}</div>
      <div class="form-tip">{{ tip }}</div>
    </el-form-item>
    <el-form-item>
      <el-button @click="formatJson" :disabled="!isJsonFormat">格式化</el-button>
      <el-button type="primary" @click="handleSave">保存</el-button>
    </el-form-item>
  </el-form>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { ElMessage } from 'element-plus'
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

// 默认配置目录（固定值，不随用户保存的值变化）
const defaultConfigDir = ref('')

const validationError = ref('')

const placeholder = computed(() => {
  switch (props.cliType) {
    case 'codex':
      return `model_reasoning_effort = "high"
model_reasoning_summary = "detailed"`
    case 'claude_code':
      return `{
  "env": {},
  "permissions": {}
}`
    case 'gemini':
      return `{
  "theme": "dark"
}`
    default:
      return '{}'
  }
})

const tip = computed(() => {
  switch (props.cliType) {
    case 'codex':
      return '此处配置会合并到 config.toml（TOML 格式）'
    case 'claude_code':
      return '此处配置会合并到 settings.json（JSON 格式）'
    case 'gemini':
      return '此处配置会合并到 settings.json（JSON 格式）'
    default:
      return '此处配置会合并到 CLI 的配置文件中'
  }
})

const isJsonFormat = computed(() => {
  return props.cliType === 'claude_code' || props.cliType === 'gemini'
})

const configDirTip = computed(() => {
  return `默认：${defaultConfigDir.value}`
})

watch(() => props.settings, (settings) => {
  if (settings) {
    form.value = {
      default_json_config: settings.default_json_config,
      config_dir: settings.config_dir
    }
    // 默认路径由后端硬编码返回（如 C:\Users\18054\.codex）
    defaultConfigDir.value = settings.default_config_dir
  }
}, { immediate: true })

function handleResetDir() {
  form.value.config_dir = defaultConfigDir.value
}

function validateConfig() {
  validationError.value = ''

  const config = form.value.default_json_config.trim()
  if (!config) {
    return true
  }

  // 对于 claude_code 和 gemini，验证 JSON 格式
  if (props.cliType === 'claude_code' || props.cliType === 'gemini') {
    validationError.value = validateJson(config)
    return !validationError.value
  }

  // 对于 codex，验证 TOML 格式（简单检查）
  if (props.cliType === 'codex') {
    if (config.includes('{') || config.includes('[') && config.includes(']') && config.includes(',')) {
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
    ElMessage.error('配置格式错误，请修正后再保存')
    return
  }
  emit('save', props.cliType, form.value)
}
</script>

<style scoped>
.form-tip {
  margin-top: 5px;
  color: #999;
  font-size: 12px;
}
.error-tip {
  margin-top: 5px;
  color: #f56c6c;
  font-size: 12px;
}
:deep(.el-form-item__label) {
  font-weight: 500;
  color: #606266;
}
</style>
