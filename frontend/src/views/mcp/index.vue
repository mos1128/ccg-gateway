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
import { mcpApi } from '@/api/mcp'
import type { Mcp } from '@/types/models'

const mcpList = ref<Mcp[]>([])
const showAddDialog = ref(false)
const editingMcp = ref<Mcp | null>(null)

const showDialog = computed({
  get: () => showAddDialog.value || !!editingMcp.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingMcp.value = null
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
}

async function handleSave() {
  try {
    if (editingMcp.value) {
      await mcpApi.update(editingMcp.value.id, form.value)
      ElMessage.success('更新成功')
    } else {
      await mcpApi.create(form.value)
      ElMessage.success('添加成功')
    }
    showDialog.value = false
    form.value = { name: '', config_json: '' }
    await fetchList()
  } catch {
    // handled by interceptor
  }
}

async function handleCliToggle(mcp: Mcp, cliType: string, enabled: boolean) {
  const cli_flags = {
    claude_code: mcp.cli_flags?.claude_code ?? false,
    codex: mcp.cli_flags?.codex ?? false,
    gemini: mcp.cli_flags?.gemini ?? false,
    [cliType]: enabled
  }
  await mcpApi.update(mcp.id, { cli_flags })
  await fetchList()
  ElMessage.success('已更新')
}

async function handleDelete(mcp: Mcp) {
  await ElMessageBox.confirm('确定删除该 MCP?', '确认')
  await mcpApi.delete(mcp.id)
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
