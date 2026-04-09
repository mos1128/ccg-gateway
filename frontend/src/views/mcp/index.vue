<template>
  <div class="mcp-page">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-boxes" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/>
          <path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>
        </symbol>
        <symbol id="icon-plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </symbol>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
        <symbol id="icon-code" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>
        </symbol>
      </defs>
    </svg>

    <div class="page-header">
      <p class="page-subtitle">配置 Model Context Protocol (MCP) 服务器</p>
      <button class="action-icon add-btn" @click="handleAdd" title="添加 MCP">
        <svg width="20" height="20"><use href="#icon-plus"/></svg>
      </button>
    </div>

    <div v-loading="loading" class="list-container">
      <template v-if="mcpList.length === 0">
        <div class="empty-state">
          <svg width="64" height="64" color="#e2e8f0"><use href="#icon-boxes"/></svg>
          <p>暂无 MCP，点击上方按钮开始添加</p>
        </div>
      </template>
      <div v-else class="scroll-area">
        <div class="mcp-grid">
          <div v-for="mcp in mcpList" :key="mcp.id" class="mcp-card">
            <div class="card-top">
              <div class="mcp-icon">
                <svg width="24" height="24"><use href="#icon-boxes"/></svg>
              </div>
              <div class="mcp-info">
                <h3 class="mcp-name">{{ mcp.name }}</h3>
                <div class="mcp-actions">
                  <button class="action-icon" title="编辑" @click="handleEdit(mcp)">
                    <svg width="18" height="18"><use href="#icon-edit"/></svg>
                  </button>
                  <button class="action-icon delete" title="删除" @click="handleDelete(mcp)">
                    <svg width="18" height="18"><use href="#icon-trash"/></svg>
                  </button>
                </div>
              </div>
            </div>
            
            <div class="cli-toggles">
              <div class="toggle-item">
                <span class="toggle-label">Claude Code</span>
                <el-switch
                  size="small"
                  :model-value="mcp.cli_flags?.claude_code"
                  @change="handleCliToggle(mcp, 'claude_code', $event as boolean)"
                />
              </div>
              <div class="toggle-item">
                <span class="toggle-label">Codex</span>
                <el-switch
                  size="small"
                  :model-value="mcp.cli_flags?.codex"
                  @change="handleCliToggle(mcp, 'codex', $event as boolean)"
                />
              </div>
              <div class="toggle-item">
                <span class="toggle-label">Gemini</span>
                <el-switch
                  size="small"
                  :model-value="mcp.cli_flags?.gemini"
                  @change="handleCliToggle(mcp, 'gemini', $event as boolean)"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Modal -->
    <AppModal v-model="showDialog" :title="editingMcp ? '编辑 MCP' : '添加 MCP'" width="640px" @confirm="handleSave">
        <div class="form-group">
          <label class="c-label">MCP 名称 <span class="required">*</span></label>
          <input type="text" v-model="form.name" class="c-input" placeholder="例如: Google Maps Search">
        </div>

        <div class="form-group">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
            <label class="c-label" style="margin-bottom: 0;">配置 JSON <span class="required">*</span></label>
            <button class="f-button ghost-plain sm" @click="formatJson">
              <svg width="12" height="12" style="margin-right: 4px;"><use href="#icon-code"/></svg>
              格式化
            </button>
          </div>
          <textarea
            v-model="form.config_json"
            class="c-input mono"
            rows="12"
            placeholder='{"command": "npx", "args": ["-y", "@example/mcp"]}'
            @blur="validateConfig"
          ></textarea>
          <div v-if="validationError" class="error-tip">{{ validationError }}</div>
        </div>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import AppModal from '@/components/AppModal.vue'
import { mcpApi } from '@/api/mcp'
import type { CliType, Mcp } from '@/types/models'
import { validateJson, formatJson as formatJsonUtil } from '@/utils/json'

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

const form = ref({
  name: '',
  config_json: ''
})

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
  form.value = {
    name: mcp.name,
    config_json: mcp.config_json
  }
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
    notify('请输入 MCP 名称', 'error')
    return
  }
  if (!validateConfig()) {
    notify('JSON 格式错误，请修正后再保存', 'error')
    return
  }
  try {
    const data = {
      name: form.value.name.trim(),
      config_json: form.value.config_json.trim()
    }

    if (editingMcp.value) {
      await mcpApi.update(editingMcp.value.id, data)
      notify('更新成功')
    } else {
      await mcpApi.create(data)
      notify('添加成功')
    }
    showDialog.value = false
    form.value = { name: '', config_json: '' }
    validationError.value = ''
    await fetchList()
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  }
}

async function handleCliToggle(mcp: Mcp, cliType: CliType, enabled: boolean) {
  try {
    const { data } = await mcpApi.toggleCli(mcp.id, cliType, enabled)
    mcp.cli_flags = data.cli_flags
    notify('已更新')
  } catch (error: any) {
    notify(getErrorMessage(error, '更新失败'), 'error')
    await fetchList() // Rollback
  }
}

async function handleDelete(mcp: Mcp) {
  try {
    await ElMessageBox.confirm(`确定删除 MCP 服务器 "${mcp.name}"?`, '确认删除')
    await mcpApi.delete(mcp.id)
    notify('已删除')
    await fetchList()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(getErrorMessage(error, '删除失败'), 'error')
    }
  }
}

onMounted(fetchList)
</script>

<style scoped>
.mcp-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Header */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: 0 40px 32px 40px;
  flex-shrink: 0;
}

.list-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: 0 40px;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.page-subtitle {
  font-size: 14px;
  color: #64748b;
  margin: 0;
}

/* Grid & Cards */
.mcp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(480px, 1fr));
  gap: 24px;
}
.mcp-card {
  background: #ffffff;
  border-radius: 16px;
  border: 1px solid rgba(226, 232, 240, 0.8);
  padding: 24px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.03);
  transition: all 0.2s;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.mcp-card:hover {
  border-color: #0ea5e9;
  box-shadow: 0 10px 20px -5px rgba(0,0,0,0.05);
}

.card-top {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}
.mcp-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: #f0f9ff;
  color: #0ea5e9;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.mcp-info {
  flex: 1;
  min-width: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
.mcp-name {
  font-size: var(--fs-16);
  font-weight: var(--fw-700);
  color: #0f172a;
  margin: 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-right: 8px;
}
  padding-right: 8px;
}
.mcp-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

/* Action Icons */
.action-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
  background: transparent;
  border: none;
  outline: none;
}
.action-icon:hover {
  background: #f1f5f9;
  color: #0f172a;
}
.action-icon.delete:hover {
  background: #fee2e2;
  color: #ef4444;
}

/* CLI Toggles */
.cli-toggles {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 4px;
}
.toggle-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.toggle-label {
  font-size: var(--fs-14);
  font-weight: var(--fw-500);
  color: #64748b;
}

/* Form Elements */
.form-group {
  margin-bottom: 24px;
}
.c-label {
  display: block;
  font-size: var(--fs-14);
  font-weight: var(--fw-400);
  color: #475569;
  margin-bottom: 12px;
}
.required {
  color: #f43f5e;
}
.c-input {
  width: 100%;
  padding: 10px 14px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  font-size: var(--fs-14);
  color: #0f172a;
  outline: none;
  transition: all 0.2s;
}
.c-input:focus {
  border-color: #0ea5e9;
}
textarea.c-input {
  resize: vertical;
  line-height: 1.6;
  word-break: break-all;
}
.error-tip {
  color: #f43f5e;
  font-size: var(--fs-12);
  margin-top: 6px;
}

/* Buttons */
.f-button {
  background: #0ea5e9;
  color: #ffffff;
  border: none;
  padding: 8px 16px;
  border-radius: 8px;
  font-size: var(--fs-14);
  font-weight: var(--fw-400);
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: all 0.2s;
}
.f-button:hover {
  background: #0284c7;
}
.f-button.ghost-plain {
  background: transparent;
  color: #64748b;
  padding: 8px 12px;
  font-size: var(--fs-14);
  font-weight: var(--fw-600);
  border-radius: 8px;
}
.f-button.ghost-plain:hover {
  color: #0f172a;
  background: #f1f5f9;
}
.f-button.ghost-plain.sm {
  padding: 4px 8px;
  font-size: var(--fs-12);
}

.empty-state {
  padding: 80px 40px;
  text-align: center;
  color: #94a3b8;
  background: #ffffff;
  border-radius: 24px;
  border: 2px dashed #e2e8f0;
}
.empty-state p {
  margin-top: 16px;
  font-size: var(--fs-14);
}

.action-icon.add-btn {
  width: 36px;
  height: 36px;
  color: #0ea5e9;
  background: rgba(14, 165, 233, 0.1);
}
.action-icon.add-btn:hover {
  background: rgba(14, 165, 233, 0.2);
  color: #0ea5e9;
}
</style>
