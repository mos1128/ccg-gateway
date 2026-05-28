<template>
  <AppModal v-model="visible" :title="title" width="720px" @confirm="$emit('confirm')">
    <div style="margin-bottom: 32px;">
      <label class="c-label">凭证名称 <span class="required">*</span></label>
      <input type="text" v-model="form.name" class="b-input" placeholder="例如: 个人主账号">
    </div>

    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
      <div class="text-16 fw-normal text-primary">配置文件</div>
      <button class="b-button-outline text-14" style="padding: 6px 12px;" @click="$emit('read-from-cli')">读取当前 CLI 配置</button>
    </div>

    <template v-if="activeCliType === 'claude_code'">
      <div style="margin-bottom: 24px;">
        <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.claude/settings.json</div>
        <textarea class="b-input mono" rows="10" v-model="form.claude_settings" placeholder='{"ANTHROPIC_API_KEY": "..."}'></textarea>
      </div>
    </template>

    <template v-if="activeCliType === 'codex'">
      <div style="margin-bottom: 24px;">
        <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.codex/auth.json</div>
        <textarea class="b-input mono" rows="10" v-model="form.codex_auth"></textarea>
      </div>
    </template>

    <template v-if="activeCliType === 'gemini'">
      <div style="margin-bottom: 24px;">
        <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.gemini/oauth_creds.json</div>
        <textarea class="b-input mono" rows="4" v-model="form.gemini_oauth"></textarea>
      </div>
      <div style="margin-bottom: 24px;">
        <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.gemini/google_accounts.json</div>
        <textarea class="b-input mono" rows="3" v-model="form.gemini_accounts"></textarea>
      </div>
    </template>
  </AppModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AppModal from '@/components/AppModal.vue'
import type { CliType } from '@/types/models'

interface CredentialEditForm {
  name: string
  claude_settings: string
  codex_auth: string
  gemini_oauth: string
  gemini_accounts: string
}

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
</script>
