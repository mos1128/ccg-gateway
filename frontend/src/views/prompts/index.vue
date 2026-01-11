<template>
  <div class="prompts-page">
    <div class="page-header">
      <el-button type="primary" @click="showAddDialog = true">
        <el-icon><Plus /></el-icon>
        添加提示词
      </el-button>
    </div>

    <el-card>
      <el-table :data="promptList" stripe style="width: 100%">
        <el-table-column prop="name" label="名称" min-width="200" />
        <el-table-column label="ClaudeCode" width="130">
          <template #default="{ row }">
            <el-switch
              :model-value="row.cli_flags?.claude_code"
              @change="handleCliToggle(row, 'claude_code', $event)"
            />
          </template>
        </el-table-column>
        <el-table-column label="Codex" width="130">
          <template #default="{ row }">
            <el-switch
              :model-value="row.cli_flags?.codex"
              @change="handleCliToggle(row, 'codex', $event)"
            />
          </template>
        </el-table-column>
        <el-table-column label="Gemini" width="130">
          <template #default="{ row }">
            <el-switch
              :model-value="row.cli_flags?.gemini"
              @change="handleCliToggle(row, 'gemini', $event)"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150">
          <template #default="{ row }">
            <el-button size="small" @click="handleEdit(row)">编辑</el-button>
            <el-button size="small" type="danger" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- Add/Edit Dialog -->
    <el-dialog
      v-model="showDialog"
      :title="editingPrompt ? '编辑提示词' : '添加提示词'"
      width="700px"
    >
      <el-form :model="form" label-width="80px">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="提示词名称" />
        </el-form-item>
        <el-form-item label="内容" required>
          <el-input
            v-model="form.content"
            type="textarea"
            :rows="15"
            placeholder="提示词内容..."
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showDialog = false">取消</el-button>
        <el-button type="primary" @click="handleSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { promptsApi } from '@/api/prompts'
import type { Prompt } from '@/types/models'

const promptList = ref<Prompt[]>([])
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
  const { data } = await promptsApi.list()
  promptList.value = data
}

function handleEdit(prompt: Prompt) {
  editingPrompt.value = prompt
  form.value = {
    name: prompt.name,
    content: prompt.content
  }
}

async function handleSave() {
  try {
    if (editingPrompt.value) {
      await promptsApi.update(editingPrompt.value.id, form.value)
      ElMessage.success('更新成功')
    } else {
      await promptsApi.create(form.value)
      ElMessage.success('添加成功')
    }
    showDialog.value = false
    form.value = { name: '', content: '' }
    await fetchList()
  } catch {
    // handled by interceptor
  }
}

async function handleCliToggle(prompt: Prompt, cliType: string, enabled: boolean) {
  const cli_flags = {
    claude_code: prompt.cli_flags?.claude_code ?? false,
    codex: prompt.cli_flags?.codex ?? false,
    gemini: prompt.cli_flags?.gemini ?? false,
    [cliType]: enabled
  }
  await promptsApi.update(prompt.id, { cli_flags })
  await fetchList()
  ElMessage.success('已更新')
}

async function handleDelete(prompt: Prompt) {
  await ElMessageBox.confirm('确定删除该提示词?', '确认')
  await promptsApi.delete(prompt.id)
  ElMessage.success('已删除')
  await fetchList()
}

onMounted(fetchList)
</script>

<style scoped>
.page-header {
  margin-bottom: 20px;
}
</style>
