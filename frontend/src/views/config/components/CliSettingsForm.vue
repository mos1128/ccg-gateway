<template>
  <el-form :model="form" label-width="0">
    <el-form-item>
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
      <el-button type="primary" @click="handleSave">保存</el-button>
    </el-form-item>
  </el-form>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { ElMessage } from 'element-plus'
import type { CliSettings } from '@/types/models'

const props = defineProps<{
  cliType: string
  settings?: CliSettings
}>()

const emit = defineEmits<{
  save: [cliType: string, data: { default_json_config: string }]
}>()

const form = ref({
  default_json_config: ''
})

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
      return '此处配置会合并到 ~/.codex/config.toml（TOML 格式）'
    case 'claude_code':
      return '此处配置会合并到 ~/.claude/settings.json（JSON 格式）'
    case 'gemini':
      return '此处配置会合并到 ~/.gemini/settings.json（JSON 格式）'
    default:
      return '此处配置会合并到 CLI 的配置文件中'
  }
})

watch(() => props.settings, (settings) => {
  if (settings) {
    form.value = {
      default_json_config: settings.default_json_config
    }
  }
}, { immediate: true })

function validateConfig() {
  validationError.value = ''

  const config = form.value.default_json_config.trim()
  if (!config) {
    return true
  }

  // 对于 claude_code 和 gemini，验证 JSON 格式
  if (props.cliType === 'claude_code' || props.cliType === 'gemini') {
    try {
      JSON.parse(config)
      return true
    } catch (e) {
      validationError.value = `JSON 格式错误: ${(e as Error).message}`
      return false
    }
  }

  // 对于 codex，验证 TOML 格式（简单检查）
  if (props.cliType === 'codex') {
    // TOML 格式较为宽松，这里做基本检查
    // 检查是否有明显的 JSON 语法（常见错误）
    if (config.includes('{') || config.includes('[') && config.includes(']') && config.includes(',')) {
      validationError.value = 'TOML 格式错误: 请使用 TOML 格式而非 JSON 格式'
      return false
    }
    return true
  }

  return true
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
</style>
