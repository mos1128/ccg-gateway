<template>
  <div class="mcp-page">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-boxes" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.54l-3.97-2.38a2 2 0 0 0-2.06 0L7 10.54Z"/>
          <path d="m7 21 5-3 5 3"/><path d="M12 18V12"/><path d="m7 15 5-3 5 3"/><path d="m2 14.63 10-6 10 6"/><path d="m12 8.63-3.97-2.38a2 2 0 0 1 0-3.44l3-1.8a2 2 0 0 1 2.06 0l3 1.8a2 2 0 0 1 0 3.44Z"/>
        </symbol>
        <symbol id="icon-plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </symbol>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/><path d="m15 5 4 4"/>
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
      <p class="page-subtitle">配置并快速启用  Model Context Protocol 扩展</p>
      <button class="b-button" style="padding: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center;" @click="handleAdd" title="添加 MCP">
        <svg width="20" height="20"><use href="#icon-plus"/></svg>
      </button>
    </div>

    <div v-loading="loading">
      <template v-if="mcpList.length === 0">
        <div class="empty-state">
          <svg width="64" height="64" color="#e2e8f0"><use href="#icon-boxes"/></svg>
          <p>暂无 MCP，点击上方按钮开始添加</p>
        </div>
      </template>
      <div v-else class="mcp-grid">
        <div v-for="mcp in mcpList" :key="mcp.id" class="mcp-card">
          <div class="card-top">
            <div class="mcp-icon">
              <svg width="24" height="24"><use href="#icon-boxes"/></svg>
            </div>
            <div class="mcp-info">
              <h3 class="mcp-name">{{ mcp.name }}</h3>
              <div class="mcp-actions">
                <button class="action-btn" title="编辑" @click="handleEdit(mcp)">
                  <svg width="16" height="16"><use href="#icon-edit"/></svg>
                </button>
                <button class="action-btn delete" title="删除" @click="handleDelete(mcp)">
                  <svg width="16" height="16"><use href="#icon-trash"/></svg>
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
                @change="handleCliToggle(mcp, 'claude_code', $event)"
              />
            </div>
            <div class="toggle-item">
              <span class="toggle-label">Codex</span>
              <el-switch
                size="small"
                :model-value="mcp.cli_flags?.codex"
                @change="handleCliToggle(mcp, 'codex', $event)"
              />
            </div>
            <div class="toggle-item">
              <span class="toggle-label">Gemini</span>
              <el-switch
                size="small"
                :model-value="mcp.cli_flags?.gemini"
                @change="handleCliToggle(mcp, 'gemini', $event)"
              />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Modal -->
    <AppModal v-model="showDialog" :title="editingMcp ? '编辑 MCP' : '添加 MCP'" width="640px" :show-footer="false">
        <div class="form-group">
          <label class="c-label">MCP 名称 <span class="required">*</span></label>
          <input type="text" v-model="form.name" class="c-input" placeholder="例如: Google Maps Search">
        </div>

        <div class="form-group">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
            <label class="c-label" style="margin-bottom: 0;">配置 JSON <span class="required">*</span></label>
            <button class="b-button-outline" style="font-size: 12px; padding: 4px 10px;" @click="formatJson">
              <svg width="14" height="14" style="margin-right: 4px;"><use href="#icon-code"/></svg>
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

      <template #footer>
        <button class="b-button-outline" @click="showDialog = false">取消</button>
        <button class="b-button" @click="handleSave">保存配置</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import AppModal from '@/components/AppModal.vue'
import { mcpApi } from '@/api/mcp'
import type { Mcp } from '@/types/models'
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
    notify(error?.message || '操作失败', 'error')
  }
}

async function handleCliToggle(mcp: Mcp, cliType: string, enabled: boolean) {
  try {
    const cli_flags = [
      { cli_type: 'claude_code', enabled: cliType === 'claude_code' ? enabled : (mcp.cli_flags?.claude_code ?? false) },
      { cli_type: 'codex', enabled: cliType === 'codex' ? enabled : (mcp.cli_flags?.codex ?? false) },
      { cli_type: 'gemini', enabled: cliType === 'gemini' ? enabled : (mcp.cli_flags?.gemini ?? false) }
    ]
    await mcpApi.update(mcp.id, { cli_flags })
    // Update local state directly for snappy UI
    if (mcp.cli_flags) {
      mcp.cli_flags[cliType] = enabled
    }
    notify('已更新')
  } catch (error: any) {
    notify(error?.message || '更新失败', 'error')
    await fetchList() // Rollback
  }
}

async function handleDelete(mcp: Mcp) {
  try {
    await ElMessageBox.confirm(`确定删除 MCP "${mcp.name}"?`, '确认删除')
    await mcpApi.delete(mcp.id)
    notify('已删除')
    await fetchList()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(error?.message || '删除失败', 'error')
    }
  }
}

onMounted(fetchList)
</script>

<style scoped>
.mcp-page {
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

/* Header */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 32px;
}
.page-subtitle {
  font-size: 14px;
  color: #64748b;
  margin: 0;
}

/* Grid & Cards */
.mcp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
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
  gap: 20px;
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
}
.mcp-name {
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-right: 8px;
}
.mcp-actions {
  display: flex;
  gap: 4px;
}

/* Action Buttons */
.action-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  padding: 6px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  outline: none;
}
.action-btn:hover {
  background: #f1f5f9;
  color: #0f172a;
}
.action-btn.delete:hover {
  background: #fef2f2;
  color: #f43f5e;
}

/* CLI Toggles */
.cli-toggles {
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: #f8fafc;
  padding: 16px;
  border-radius: 12px;
}
.toggle-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.toggle-label {
  font-size: 13px;
  font-weight: 500;
  color: #475569;
}

/* Form Elements */
.form-group {
  margin-bottom: 24px;
}
.c-label {
  display: block;
  font-size: 14px;
  font-weight: 600;
  color: #475569;
  margin-bottom: 8px;
}
.required {
  color: #f43f5e;
}
.c-input {
  width: 100%;
  padding: 12px 16px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  font-size: 14px;
  color: #0f172a;
  outline: none;
  transition: all 0.2s;
}
.c-input:focus {
  border-color: #0ea5e9;
  box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1);
}
textarea.c-input {
  resize: vertical;
  line-height: 1.6;
}
.mono {
  font-family: "JetBrains Mono", monospace;
}
.error-tip {
  color: #f43f5e;
  font-size: 12px;
  margin-top: 6px;
}

/* Buttons */
.b-button {
  background: #0ea5e9;
  color: #ffffff;
  border: none;
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3);
  transition: all 0.2s;
}
.b-button:hover {
  background: #0284c7;
  transform: translateY(-1px);
}
.b-button-outline {
  background: #ffffff;
  color: #475569;
  border: 1px solid #e2e8f0;
  padding: 10px 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
}
.b-button-outline:hover {
  background: #f8fafc;
  color: #0f172a;
  border-color: #cbd5e1;
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
  font-size: 15px;
}
</style>
