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
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
      </defs>
    </svg>

    <div class="page-header">
      <p class="page-subtitle">预设常用提示词，快速注入</p>
      <button class="action-icon add-btn" @click="handleAdd" title="添加提示词">
        <svg width="20" height="20"><use href="#icon-plus"/></svg>
      </button>
    </div>

    <div v-loading="loading" class="list-container">
      <template v-if="promptList.length === 0">
        <div class="empty-state">
          <svg width="64" height="64" color="var(--color-border)"><use href="#icon-quote"/></svg>
          <p>暂无提示词，点击上方按钮开始添加</p>
        </div>
      </template>
      <div v-else class="scroll-area">
        <div class="prompt-grid">
          <div v-for="prompt in promptList" :key="prompt.id" class="prompt-card">
            <div class="card-top">
              <div class="prompt-icon">
                <svg width="24" height="24"><use href="#icon-quote"/></svg>
              </div>
              <div class="prompt-info">
                <h3 class="prompt-name">{{ prompt.name }}</h3>
                <div class="prompt-actions">
                  <button class="action-icon" title="编辑" @click="handleEdit(prompt)">
                    <svg width="18" height="18"><use href="#icon-edit"/></svg>
                  </button>
                  <button class="action-icon delete" title="删除" @click="handleDelete(prompt)">
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
                  :model-value="prompt.cli_flags?.claude_code"
                  @change="handleCliToggle(prompt, 'claude_code', $event as boolean)"
                />
              </div>
              <div class="toggle-item">
                <span class="toggle-label">Codex</span>
                <el-switch
                  size="small"
                  :model-value="prompt.cli_flags?.codex"
                  @change="handleCliToggle(prompt, 'codex', $event as boolean)"
                />
              </div>
              <div class="toggle-item">
                <span class="toggle-label">Gemini</span>
                <el-switch
                  size="small"
                  :model-value="prompt.cli_flags?.gemini"
                  @change="handleCliToggle(prompt, 'gemini', $event as boolean)"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Modal -->
    <AppModal v-model="showDialog" :title="editingPrompt ? '编辑提示词' : '添加提示词'" width="800px" @confirm="handleSave">
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
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import AppModal from '@/components/AppModal.vue'
import { promptsApi } from '@/api/prompts'
import type { CliType, Prompt } from '@/types/models'

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
    notify(getErrorMessage(error, '操作失败'), 'error')
  }
}

async function handleCliToggle(prompt: Prompt, cliType: CliType, enabled: boolean) {
  try {
    const { data } = await promptsApi.toggleCli(prompt.id, cliType, enabled)
    prompt.cli_flags = data.cli_flags
    notify('已更新')
  } catch (error: any) {
    notify(getErrorMessage(error, '更新失败'), 'error')
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
      notify(getErrorMessage(error, '删除失败'), 'error')
    }
  }
}

onMounted(fetchList)
</script>

<style scoped>
.prompts-page {
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
  font-size: var(--fs-14);
  color: var(--color-text-muted);
  margin: 0;
}

/* Grid & Cards */
.prompt-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(480px, 1fr));
  gap: 24px;
}
.prompt-card {
  background: var(--color-bg);
  border-radius: 16px;
  border: 1px solid var(--color-border);
  padding: 24px;
  box-shadow: 0 4px 12px var(--color-shadow);
  display: flex;
  flex-direction: column;
  gap: 16px;
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
  background: var(--color-primary-light);
  color: var(--color-primary);
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
  font-size: var(--fs-16);
  font-weight: var(--fw-700);
  color: var(--color-text);
  margin: 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-right: 8px;
}
.prompt-actions {
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
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
  background: transparent;
  border: none;
  outline: none;
}
.action-icon:hover {
  background: var(--color-bg-subtle);
  color: var(--color-text);
}
.action-icon.delete:hover {
  background: var(--color-danger-light);
  color: var(--color-danger);
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
  font-weight: var(--fw-400);
  color: var(--color-text-muted);
}

/* Form Elements */
.form-group {
  margin-bottom: 24px;
}
.c-label {
  display: block;
  font-size: var(--fs-14);
  font-weight: var(--fw-400);
  color: var(--color-text-secondary);
  margin-bottom: 12px;
}
.required {
  color: var(--color-error);
}
.c-input {
  width: 100%;
  padding: 10px 14px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  font-size: var(--fs-14);
  color: var(--color-text);
  outline: none;
  transition: all 0.2s;
}
.c-input:focus {
  border-color: var(--color-primary);
}
textarea.c-input {
  resize: vertical;
  line-height: 1.6;
  word-break: break-all;
}

/* Buttons */
.b-button {
  background: var(--color-primary);
  color: var(--color-bg);
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
.b-button:hover {
  background: var(--color-primary-hover);
}
.b-button-outline {
  background: var(--color-bg);
  color: var(--color-text);
  border: 1px solid var(--color-border);
  padding: 8px 16px;
  border-radius: 8px;
  font-size: var(--fs-14);
  font-weight: var(--fw-400);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
}
.b-button-outline:hover {
  background: var(--color-bg-page);
  border-color: var(--color-border-hover);
}

.empty-state {
  padding: 80px 40px;
  text-align: center;
  color: var(--color-text-weak);
  background: var(--color-bg);
  border-radius: 24px;
  border: 2px dashed var(--color-border);
}
.empty-state p {
  margin-top: 16px;
  font-size: var(--fs-14);
}

.action-icon.add-btn {
  width: 36px;
  height: 36px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
}
.action-icon.add-btn:hover {
  background: color-mix(in srgb, var(--color-primary) 20%, transparent);
  color: var(--color-primary);
}
</style>
