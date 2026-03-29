<template>
  <div class="prompts-page">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-quote" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 21c3 0 7-1 7-8V5c0-1.1-.9-2-2-2H4c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2h4c0 3.5-1 4.4-2 5.5l-1 1"/>
          <path d="M15 21c3 0 7-1 7-8V5c0-1.1-.9-2-2-2h-4c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2h4c0 3.5-1 4.4-2 5.5l-1 1"/>
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
      </defs>
    </svg>

    <div class="page-header">
      <p class="page-subtitle">预设常用提示词，快速注入</p>
      <button class="b-button" style="padding: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center;" @click="handleAdd" title="添加提示词">
        <svg width="20" height="20"><use href="#icon-plus"/></svg>
      </button>
    </div>

    <div v-loading="loading">
      <template v-if="promptList.length === 0">
        <div class="empty-state">
          <svg width="64" height="64" color="#e2e8f0"><use href="#icon-quote"/></svg>
          <p>暂无提示词，点击上方按钮开始添加</p>
        </div>
      </template>
      <div v-else class="prompt-grid">
        <div v-for="prompt in promptList" :key="prompt.id" class="prompt-card">
          <div class="card-top">
            <div class="prompt-icon">
              <svg width="24" height="24"><use href="#icon-quote"/></svg>
            </div>
            <div class="prompt-info">
              <h3 class="prompt-name">{{ prompt.name }}</h3>
              <div class="prompt-actions">
                <button class="action-btn" title="编辑" @click="handleEdit(prompt)">
                  <svg width="16" height="16"><use href="#icon-edit"/></svg>
                </button>
                <button class="action-btn delete" title="删除" @click="handleDelete(prompt)">
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
                :model-value="prompt.cli_flags?.claude_code"
                @change="handleCliToggle(prompt, 'claude_code', $event)"
              />
            </div>
            <div class="toggle-item">
              <span class="toggle-label">Codex</span>
              <el-switch
                size="small"
                :model-value="prompt.cli_flags?.codex"
                @change="handleCliToggle(prompt, 'codex', $event)"
              />
            </div>
            <div class="toggle-item">
              <span class="toggle-label">Gemini</span>
              <el-switch
                size="small"
                :model-value="prompt.cli_flags?.gemini"
                @change="handleCliToggle(prompt, 'gemini', $event)"
              />
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Modal -->
    <AppModal v-model="showDialog" :title="editingPrompt ? '编辑提示词' : '添加提示词'" width="800px" :show-footer="false">
        <div class="form-group">
          <label class="c-label">提示词名称 <span class="required">*</span></label>
          <input type="text" v-model="form.name" class="c-input" placeholder="例如: 单元测试生成器">
        </div>

        <div class="form-group">
          <label class="c-label">提示词内容 <span class="required">*</span></label>
          <textarea
            v-model="form.content"
            class="c-input mono"
            rows="16"
            placeholder="请输入提示词内容..."
          ></textarea>
        </div>

      <template #footer>
        <button class="b-button-outline" @click="showDialog = false">取消</button>
        <button class="b-button" @click="handleSave">保存提示词</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import AppModal from '@/components/AppModal.vue'
import { promptsApi } from '@/api/prompts'
import type { Prompt } from '@/types/models'

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

const form = ref({
  name: '',
  content: ''
})

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
  form.value = {
    name: prompt.name,
    content: prompt.content
  }
}

async function handleSave() {
  if (!form.value.name.trim() || !form.value.content.trim()) {
    notify('请填写完整的必填项', 'error')
    return
  }
  try {
    const data = {
      name: form.value.name.trim(),
      content: form.value.content.trim()
    }

    if (editingPrompt.value) {
      await promptsApi.update(editingPrompt.value.id, data)
      notify('更新成功')
    } else {
      await promptsApi.create(data)
      notify('添加成功')
    }
    showDialog.value = false
    form.value = { name: '', content: '' }
    await fetchList()
  } catch (error: any) {
    notify(error?.message || '操作失败', 'error')
  }
}

async function handleCliToggle(prompt: Prompt, cliType: string, enabled: boolean) {
  try {
    const cli_flags = [
      { cli_type: 'claude_code', enabled: cliType === 'claude_code' ? enabled : (prompt.cli_flags?.claude_code ?? false) },
      { cli_type: 'codex', enabled: cliType === 'codex' ? enabled : (prompt.cli_flags?.codex ?? false) },
      { cli_type: 'gemini', enabled: cliType === 'gemini' ? enabled : (prompt.cli_flags?.gemini ?? false) }
    ]
    await promptsApi.update(prompt.id, { cli_flags })
    // Update local state directly for snappy UI
    if (prompt.cli_flags) {
      prompt.cli_flags[cliType] = enabled
    }
    notify('已更新')
  } catch (error: any) {
    notify(error?.message || '更新失败', 'error')
    await fetchList() // Rollback
  }
}

async function handleDelete(prompt: Prompt) {
  try {
    await ElMessageBox.confirm(`确定删除提示词 "${prompt.name}"?`, '确认删除')
    await promptsApi.delete(prompt.id)
    notify('已删除')
    await fetchList()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      notify(error?.message || '删除失败', 'error')
    }
  }
}

function truncateText(text: string, length: number) {
  if (!text) return ''
  return text.length > length ? text.substring(0, length) + '...' : text
}

onMounted(fetchList)
</script>

<style scoped>
.prompts-page {
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
.prompt-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 24px;
}
.prompt-card {
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
.prompt-card:hover {
  border-color: #0ea5e9;
  box-shadow: 0 10px 20px -5px rgba(0,0,0,0.05);
}

.card-top {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}
.prompt-icon {
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
.prompt-info {
  flex: 1;
  min-width: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.prompt-name {
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-right: 8px;
}
.prompt-actions {
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
  gap: 10px;
  padding-top: 4px;
}
.toggle-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.toggle-label {
  font-size: 13px;
  font-weight: 500;
  color: #64748b;
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
