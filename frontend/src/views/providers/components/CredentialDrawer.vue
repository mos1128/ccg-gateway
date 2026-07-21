<template>
  <V2Drawer v-model="visible" :title="title" @confirm="emit('confirm')">
    <div class="v2-field">
      <label class="v2-label">凭证名称 <span class="req">*</span></label>
      <input v-model="form.name" type="text" class="v2-input" placeholder="例如：个人主账号">
    </div>

    <label class="v2-label dr-config-label">配置文件</label>

    <div v-for="file in fileDefinitions" :key="file.key" class="v2-field">
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
          <V2CodeEditor
            v-model="form.files[file.key]"
            class="cred-editor"
            :class="{ compact: file.compact }"
            :placeholder="file.placeholder"
          />
        </div>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import V2Drawer from '@/components/V2Drawer.vue'
import V2CodeEditor from '@/components/V2CodeEditor.vue'
import type { CredentialFileDefinition } from '@/types/models'

interface CredentialEditForm {
  name: string
  files: Record<string, string>
}

const props = defineProps<{
  modelValue: boolean
  title: string
  form: CredentialEditForm
  fileDefinitions: CredentialFileDefinition[]
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

<style scoped>
.dr-config-label { margin-bottom: 8px; }
.cred-editor {
  height: min(300px, 40vh);
  resize: vertical;
  overflow: hidden;
}
.cred-editor.compact {
  height: min(145px, 20vh);
}
</style>
