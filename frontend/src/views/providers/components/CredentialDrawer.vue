<template>
  <V2Drawer v-model="visible" :title="title" @confirm="emit('confirm')">
    <div class="v2-field">
      <label class="v2-label">凭证名称 <span class="req">*</span></label>
      <input v-model="form.name" type="text" class="v2-input" placeholder="例如：个人主账号">
    </div>

    <label class="v2-label dr-config-label">配置文件</label>

    <div v-for="file in credentialFiles" :key="file.key" class="v2-field">
      <div class="v2-file-editor">
        <div class="v2-file-editor-header">
          <div class="v2-file-editor-title">
            <svg class="file-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
            <span class="v2-file-editor-name">{{ file.name }}</span>
          </div>
          <button class="v2-file-editor-action" type="button" @click="emit('read-from-cli')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
              <path d="M3 3v5h5"/>
            </svg>
            <span>读取agent配置</span>
          </button>
        </div>
        <div class="v2-file-editor-body">
          <textarea
            v-model="form[file.key]"
            class="v2-file-editor-textarea cred-editor-textarea"
            :class="{ compact: file.compact }"
            :placeholder="file.placeholder"
          ></textarea>
        </div>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import type { CliType } from '@/types/models'

interface CredentialEditForm {
  name: string
  claude_settings: string
  codex_auth: string
  gemini_oauth: string
  gemini_accounts: string
}
type CredentialFileKey = Exclude<keyof CredentialEditForm, 'name'>

const props = defineProps<{
  modelValue: boolean
  title: string
  form: CredentialEditForm
  activeCliType: CliType
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: []
  'read-from-cli': []
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const credentialFiles = computed<Array<{ key: CredentialFileKey; name: string; placeholder?: string; compact?: boolean }>>(() => {
  switch (props.activeCliType) {
    case 'claude_code':
      return [{ key: 'claude_settings', name: '~/.claude/settings.json', placeholder: '{"ANTHROPIC_API_KEY": "..."}' }]
    case 'codex':
      return [{ key: 'codex_auth', name: '~/.codex/auth.json' }]
    default:
      return [
        { key: 'gemini_oauth', name: '~/.gemini/oauth_creds.json', compact: true },
        { key: 'gemini_accounts', name: '~/.gemini/google_accounts.json', compact: true }
      ]
  }
})
</script>

<style scoped>
.dr-config-label { margin-bottom: 8px; }
.cred-editor-textarea {
  height: min(300px, 40vh);
  resize: vertical;
}
.cred-editor-textarea.compact {
  height: min(145px, 20vh);
}
</style>
