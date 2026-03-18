<template>
  <div class="mcp-page">
    <div class="page-header">
      <el-button type="primary" @click="showAddDialog = true">
        <el-icon><Plus /></el-icon>
        添加 MCP
      </el-button>
    </div>

    <el-card>
      <el-table :data="mcpList" stripe style="width: 100%">
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
      :title="editingMcp ? '编辑 MCP' : '添加 MCP'"
      width="600px"
    >
      <el-form :model="form" label-width="100px">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="MCP 名称" />
        </el-form-item>
        <el-form-item label="配置 JSON" required>
          <el-input
            v-model="form.config_json"
            type="textarea"
            :rows="10"
            placeholder='{"command": "npx", "args": ["-y", "@example/mcp"]}'
            @blur="validateConfig"
          />
          <div v-if="validationError" class="error-tip">{{ validationError }}</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showDialog = false">取消</el-button>
        <el-button @click="formatJson">格式化</el-button>
        <el-button type="primary" @click="handleSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { mcpApi } from '@/api/mcp'
import type { Mcp } from '@/types/models'
import { validateJson, formatJson as formatJsonUtil } from '@/utils/json'

const mcpList = ref<Mcp[]>([])
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
  const { data } = await mcpApi.list()
  mcpList.value = data
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
  if (!validateConfig()) {
    ElMessage.error('JSON 格式错误，请修正后再保存')
    return
  }
  try {
    const data = {
      name: form.value.name.trim(),
      config_json: form.value.config_json.trim()
    }

    if (editingMcp.value) {
      await mcpApi.update(editingMcp.value.id, data)
      ElMessage.success('更新成功')
    } else {
      await mcpApi.create(data)
      ElMessage.success('添加成功')
    }
    showDialog.value = false
    form.value = { name: '', config_json: '' }
    validationError.value = ''
    await fetchList()
  } catch (error: any) {
    ElMessage.error(error?.message || '操作失败')
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
    await fetchList()
    ElMessage.success('已更新')
  } catch (error: any) {
    ElMessage.error(error?.message || '更新失败')
  }
}

async function handleDelete(mcp: Mcp) {
  try {
    await ElMessageBox.confirm('确定删除该 MCP?', '确认')
    await mcpApi.delete(mcp.id)
    ElMessage.success('已删除')
    await fetchList()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error?.message || '删除失败')
    }
  }
}

onMounted(fetchList)
</script>

<style scoped>
.page-header {
  margin-bottom: 20px;
}
.error-tip {
  margin-top: 5px;
  color: #f56c6c;
  font-size: 12px;
}
</style>
