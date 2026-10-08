<template>
  <div>
    <div v-loading="loading" class="v2-cardgrid">
      <div class="v2-addcard" @click="handleAdd">
        <svg width="22" height="22" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>
        <span>{{ t('prompts.addPrompt') }}</span>
      </div>
      <ConfigCard
        v-for="prompt in promptList"
        :key="prompt.id"
        icon="prompt"
        :title="prompt.name"
        :subtitle="contentPreview(prompt.content)"
        :flags="prompt.cli_flags"
        @edit="handleEdit(prompt)"
        @delete="handleDelete(prompt)"
        @toggle="(c, e) => handleCliToggle(prompt, c, e)"
      />
    </div>

    <V2Drawer v-model="showDialog" :title="editingPrompt ? t('prompts.editPrompt') : t('prompts.addPrompt')" @confirm="handleSave">
      <div class="v2-field">
        <label class="v2-label">{{ t('prompts.name') }} <span class="req">*</span></label>
        <input v-model="form.name" type="text" class="v2-input" :placeholder="t('prompts.namePlaceholder')">
      </div>
      <div class="v2-field">
        <label class="v2-label">{{ t('prompts.content') }} <span class="req">*</span></label>
        <div class="v2-file-editor">
          <div class="v2-file-editor-header">
            <div class="v2-file-editor-title">
              <svg class="file-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
              </svg>
              <span class="v2-file-editor-name">prompt.md</span>
            </div>
          </div>
          <div class="v2-file-editor-body">
            <V2CodeEditor
              v-model="form.content"
              class="prompt-content-editor"
              rows="16"
              :placeholder="t('prompts.contentPlaceholder')"
            />
          </div>
        </div>
      </div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import V2Drawer from '@/components/V2Drawer.vue'
import V2CodeEditor from '@/components/V2CodeEditor.vue'
import ConfigCard from '@/components/ConfigCard.vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import { promptsApi } from '@/api/prompts'
import type { CliType, Prompt } from '@/types/models'

const { t } = useI18n()

const promptList = ref<Prompt[]>([])
const loading = ref(false)
const showAddDialog = ref(false)
const editingPrompt = ref<Prompt | null>(null)

const showDialog = computed({
  get: () => showAddDialog.value || !!editingPrompt.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingPrompt.value = null
    }
  }
})

const form = ref({ name: '', content: '' })

function contentPreview(content: string): string {
  return content.replace(/\s+/g, ' ').trim().slice(0, 60)
}

async function fetchList() {
  loading.value = true
  try {
    const { data } = await promptsApi.list()
    promptList.value = data
  } finally {
    loading.value = false
  }
}

function handleAdd() {
  editingPrompt.value = null
  form.value = { name: '', content: '' }
  showAddDialog.value = true
}

function handleEdit(prompt: Prompt) {
  editingPrompt.value = prompt
  form.value = { name: prompt.name, content: prompt.content }
}

async function handleSave() {
  if (!form.value.name.trim() || !form.value.content.trim()) {
    notify(t('prompts.requiredFields'), 'error')
    return
  }
  try {
    const data = { name: form.value.name.trim(), content: form.value.content.trim() }
    if (editingPrompt.value) {
      await promptsApi.update(editingPrompt.value.id, data)
      notify(t('prompts.updateSuccess'))
    } else {
      await promptsApi.create(data)
      notify(t('prompts.addSuccess'))
    }
    showDialog.value = false
    form.value = { name: '', content: '' }
    await fetchList()
  } catch (error: any) {
    notify(getErrorMessage(error, t('common.operationFailed')), 'error')
  }
}

async function handleCliToggle(prompt: Prompt, cliType: CliType, enabled: boolean) {
  try {
    const { data } = await promptsApi.toggleCli(prompt.id, cliType, enabled)
    prompt.cli_flags = data.cli_flags
    // 提示词每个 CLI 只能启用一个：启用时关闭同 CLI 下的其它提示词
    if (enabled) {
      for (const p of promptList.value) {
        if (p.id !== prompt.id && p.cli_flags) p.cli_flags[cliType] = false
      }
    }
    notify(t('prompts.updated'))
  } catch (error: any) {
    notify(getErrorMessage(error, t('prompts.updateFailed')), 'error')
    await fetchList()
  }
}

async function handleDelete(prompt: Prompt) {
  try {
    await confirm(t('prompts.deleteConfirm', { name: prompt.name }), t('prompts.deleteConfirmTitle'))
    await promptsApi.delete(prompt.id)
    notify(t('prompts.deleted'))
    await fetchList()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, t('prompts.deleteFailed')), 'error')
  }
}

onMounted(fetchList)
</script>

<style scoped>
.prompt-content-editor { resize: vertical; overflow: hidden; }
</style>
