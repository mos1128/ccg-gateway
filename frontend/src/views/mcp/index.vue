<template>
  <div>
    <div v-loading="loading" class="v2-cardgrid">
      <div class="v2-addcard" @click="handleAdd">
        <svg width="22" height="22" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>
        <span>{{ t('mcp.addMcp') }}</span>
      </div>
      <ConfigCard
        v-for="mcp in mcpList"
        :key="mcp.id"
        icon="mcp"
        :title="mcp.name"
        :subtitle="configSummary(mcp.config_json)"
        :flags="mcp.cli_flags"
        @edit="handleEdit(mcp)"
        @delete="handleDelete(mcp)"
        @toggle="(c, e) => handleCliToggle(mcp, c, e)"
      />
    </div>

    <V2Drawer v-model="showDialog" :title="editingMcp ? t('mcp.editMcp') : t('mcp.addMcp')" @confirm="handleSave">
      <div class="v2-field">
        <label class="v2-label">{{ t('mcp.name') }} <span class="req">*</span></label>
        <input v-model="form.name" type="text" class="v2-input" :placeholder="t('mcp.namePlaceholder')">
      </div>
      <div class="v2-field">
        <label class="v2-label">{{ t('mcp.configJson') }} <span class="req">*</span></label>
        <div class="v2-file-editor">
          <div class="v2-file-editor-header">
            <div class="v2-file-editor-title">
              <svg class="file-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
              </svg>
              <span class="v2-file-editor-name">mcp.json</span>
            </div>
            <button class="v2-file-editor-action" type="button" @click="formatJson">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
              <span>{{ t('mcp.format') }}</span>
            </button>
          </div>
          <div class="v2-file-editor-body">
            <V2CodeEditor
              v-model="form.config_json"
              class="mcp-json-editor"
              rows="14"
              placeholder='{"command": "npx", "args": ["-y", "@example/mcp"]}'
              @blur="validateConfig"
            />
          </div>
        </div>
        <div v-if="validationError" class="json-err">{{ validationError }}</div>
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
import { mcpApi } from '@/api/mcp'
import { validateJson, formatJson as formatJsonUtil } from '@/utils/json'
import type { CliType, Mcp } from '@/types/models'

const { t } = useI18n()

const mcpList = ref<Mcp[]>([])
const loading = ref(false)
const showAddDialog = ref(false)
const editingMcp = ref<Mcp | null>(null)
const validationError = ref('')

const showDialog = computed({
  get: () => showAddDialog.value || !!editingMcp.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingMcp.value = null
      validationError.value = ''
    }
  }
})

const form = ref({ name: '', config_json: '' })

function configSummary(json: string): string {
  try {
    const obj = JSON.parse(json)
    const inner = obj.command ? obj : (obj.mcpServers ? Object.values(obj.mcpServers)[0] as any : obj)
    if (inner?.command) return [inner.command, ...(inner.args || [])].join(' ')
    if (inner?.url) return inner.url
  } catch { /* fallthrough */ }
  return json.replace(/\s+/g, ' ').trim()
}

async function fetchList() {
  loading.value = true
  try {
    const { data } = await mcpApi.list()
    mcpList.value = data
  } finally {
    loading.value = false
  }
}

function handleAdd() {
  editingMcp.value = null
  form.value = { name: '', config_json: '' }
  validationError.value = ''
  showAddDialog.value = true
}

function handleEdit(mcp: Mcp) {
  editingMcp.value = mcp
  form.value = { name: mcp.name, config_json: mcp.config_json }
  validationError.value = ''
}

function validateConfig(): boolean {
  validationError.value = validateJson(form.value.config_json)
  return !validationError.value
}

function formatJson() {
  const result = formatJsonUtil(form.value.config_json)
  if (result === form.value.config_json) {
    validationError.value = validateJson(form.value.config_json)
  } else {
    form.value.config_json = result
    validationError.value = ''
  }
}

async function handleSave() {
  if (!form.value.name.trim()) {
    notify(t('mcp.nameRequired'), 'error')
    return
  }
  if (!validateConfig()) {
    notify(t('mcp.jsonInvalid'), 'error')
    return
  }
  try {
    const data = { name: form.value.name.trim(), config_json: form.value.config_json.trim() }
    if (editingMcp.value) {
      await mcpApi.update(editingMcp.value.id, data)
      notify(t('mcp.updateSuccess'))
    } else {
      await mcpApi.create(data)
      notify(t('mcp.addSuccess'))
    }
    showDialog.value = false
    form.value = { name: '', config_json: '' }
    validationError.value = ''
    await fetchList()
  } catch (error: any) {
    notify(getErrorMessage(error, t('common.operationFailed')), 'error')
  }
}

async function handleCliToggle(mcp: Mcp, cliType: CliType, enabled: boolean) {
  try {
    const { data } = await mcpApi.toggleCli(mcp.id, cliType, enabled)
    mcp.cli_flags = data.cli_flags
    notify(t('mcp.updated'))
  } catch (error: any) {
    notify(getErrorMessage(error, t('mcp.updateFailed')), 'error')
    await fetchList()
  }
}

async function handleDelete(mcp: Mcp) {
  try {
    await confirm(t('mcp.deleteConfirm', { name: mcp.name }), t('mcp.deleteConfirmTitle'))
    await mcpApi.delete(mcp.id)
    notify(t('mcp.deleted'))
    await fetchList()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') notify(getErrorMessage(error, t('mcp.deleteFailed')), 'error')
  }
}

onMounted(fetchList)
</script>

<style scoped>
.mcp-json-editor { resize: vertical; overflow: hidden; }
.json-err { color: var(--v2-danger); font-size: var(--v2-fs-xs); margin-top: 6px; }
</style>
