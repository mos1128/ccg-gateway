<template>
  <div class="providers-page">
    <el-tabs v-model="activeCliType" @tab-change="handleCliTypeChange">
      <el-tab-pane label="Claude Code" name="claude_code" />
      <el-tab-pane label="Codex" name="codex" />
      <el-tab-pane label="Gemini" name="gemini" />
    </el-tabs>

    <div class="page-header">
      <el-button type="primary" @click="showAddDialog = true">
        <el-icon><Plus /></el-icon>
        添加服务商
      </el-button>
    </div>

    <el-card v-loading="providerStore.loading">
      <template v-if="providerStore.providers.length === 0">
        <el-empty description="暂无服务商" />
      </template>
      <draggable
        v-else
        v-model="providerStore.providers"
        item-key="id"
        handle=".drag-handle"
        @end="handleDragEnd"
      >
        <template #item="{ element }">
          <div class="provider-item">
            <div class="drag-handle" aria-label="拖拽排序">
              <el-icon><Rank /></el-icon>
            </div>
            <div class="provider-info">
              <div class="provider-name">
                {{ element.name }}
                <el-tag v-if="element.is_blacklisted" type="danger" size="small">已拉黑</el-tag>
                <el-tag v-else-if="!element.enabled" type="info" size="small">已禁用</el-tag>
                <el-tag v-if="element.model_maps.length > 0" type="success" size="small">
                  {{ element.model_maps.length }}个模型映射
                </el-tag>
              </div>
              <div class="provider-url">{{ element.base_url }}</div>
            </div>
            <div class="provider-stats">
              <span>失败: {{ element.consecutive_failures }}/{{ element.failure_threshold }}</span>
            </div>
            <div class="provider-actions">
              <el-switch
                v-model="element.enabled"
                @change="handleToggle(element)"
              />
              <el-button size="small" @click="handleEdit(element)">编辑</el-button>
              <el-dropdown @command="handleCommand($event, element)">
                <el-button size="small">
                  更多<el-icon class="el-icon--right"><ArrowDown /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="reset">重置失败计数</el-dropdown-item>
                    <el-dropdown-item v-if="element.is_blacklisted" command="unblacklist">解除拉黑</el-dropdown-item>
                    <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </div>
        </template>
      </draggable>
    </el-card>

    <!-- Add/Edit Dialog -->
    <el-dialog
      v-model="showDialog"
      :title="editingProvider ? '编辑服务商' : '添加服务商'"
      width="700px"
    >
      <el-form :model="form" label-width="120px">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="服务商名称" />
        </el-form-item>
        <el-form-item label="Base URL" required>
          <el-input v-model="form.base_url" :placeholder="baseUrlPlaceholder" />
        </el-form-item>
        <el-form-item :label="activeCliType === 'claude_code' ? 'API Token' : 'API Key'" required>
          <el-input v-model="form.api_key" :placeholder="activeCliType === 'claude_code' ? 'API Token' : 'API Key'" />
        </el-form-item>
        <el-form-item label="失败阈值">
          <el-input-number v-model="form.failure_threshold" :min="1" :max="100" />
          <span class="form-tip">连续失败次数达到此值后拉黑</span>
        </el-form-item>
        <el-form-item label="拉黑时长(分钟)">
          <el-input-number v-model="form.blacklist_minutes" :min="0" :max="1440" />
        </el-form-item>

        <el-divider>模型转发配置</el-divider>
        <div class="model-maps-section">
          <div class="model-maps-header">
            <span class="model-maps-tip">将CLI请求的模型名映射为服务商模型名</span>
            <el-button type="primary" size="small" @click="addModelMap">
              <el-icon><Plus /></el-icon>添加映射
            </el-button>
          </div>
          <div v-if="form.model_maps.length === 0" class="model-maps-empty">
            暂无模型映射配置
          </div>
          <div v-else class="model-maps-list">
            <div v-for="(map, index) in form.model_maps" :key="index" class="model-map-item">
              <el-input v-model="map.source_model" placeholder="源模型 (CLI请求)" class="model-input" />
              <el-icon class="arrow-icon"><Right /></el-icon>
              <el-input v-model="map.target_model" placeholder="目标模型 (服务商)" class="model-input" />
              <el-button type="danger" size="small" circle @click="removeModelMap(index)">
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
        </div>
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
import draggable from 'vuedraggable'
import { useProviderStore } from '@/stores/providers'
import type { Provider, ModelMap, CliType } from '@/types/models'

const providerStore = useProviderStore()

const activeCliType = computed({
  get: () => providerStore.currentCliType as CliType,
  set: (val) => providerStore.setCliType(val)
})
const showAddDialog = ref(false)
const editingProvider = ref<Provider | null>(null)

const showDialog = computed({
  get: () => showAddDialog.value || !!editingProvider.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingProvider.value = null
    }
  }
})

interface FormModelMap {
  source_model: string
  target_model: string
  enabled: boolean
}

const form = ref({
  name: '',
  base_url: '',
  api_key: '',
  failure_threshold: 3,
  blacklist_minutes: 10,
  model_maps: [] as FormModelMap[]
})

const baseUrlPlaceholder = computed(() => {
  if (activeCliType.value === 'codex') return 'https://api.example.com/v1'
  return 'https://api.example.com'
})

function resetForm() {
  form.value = {
    name: '',
    base_url: '',
    api_key: '',
    failure_threshold: 3,
    blacklist_minutes: 10,
    model_maps: []
  }
}

function addModelMap() {
  form.value.model_maps.push({
    source_model: '',
    target_model: '',
    enabled: true
  })
}

function removeModelMap(index: number) {
  form.value.model_maps.splice(index, 1)
}

function handleCliTypeChange(cliType: string) {
  providerStore.fetchProviders(cliType)
}

function handleEdit(provider: Provider) {
  editingProvider.value = provider
  form.value = {
    name: provider.name,
    base_url: provider.base_url,
    api_key: provider.api_key,
    failure_threshold: provider.failure_threshold,
    blacklist_minutes: provider.blacklist_minutes,
    model_maps: provider.model_maps.map(m => ({
      source_model: m.source_model,
      target_model: m.target_model,
      enabled: m.enabled
    }))
  }
}

function buildModelMaps(): ModelMap[] {
  return form.value.model_maps
    .filter(m => m.source_model && m.target_model)
    .map(m => ({
      source_model: m.source_model,
      target_model: m.target_model,
      enabled: true
    }))
}

async function handleSave() {
  const data = {
    cli_type: activeCliType.value,
    name: form.value.name,
    base_url: form.value.base_url,
    api_key: form.value.api_key,
    failure_threshold: form.value.failure_threshold,
    blacklist_minutes: form.value.blacklist_minutes,
    model_maps: buildModelMaps()
  }

  try {
    if (editingProvider.value) {
      await providerStore.updateProvider(editingProvider.value.id, data)
      ElMessage.success('更新成功')
    } else {
      await providerStore.createProvider(data)
      ElMessage.success('添加成功')
    }
    showDialog.value = false
    resetForm()
    providerStore.fetchProviders(activeCliType.value)
  } catch {
    // error handled by interceptor
  }
}

async function handleToggle(provider: Provider) {
  try {
    await providerStore.updateProvider(provider.id, { enabled: provider.enabled })
    ElMessage.success(provider.enabled ? '已启用' : '已禁用')
  } catch {
    provider.enabled = !provider.enabled
  }
}

async function handleDragEnd() {
  const ids = providerStore.providers.map(p => p.id)
  await providerStore.reorderProviders(ids)
  ElMessage.success('排序已保存')
}

async function handleCommand(command: string, provider: Provider) {
  if (command === 'reset') {
    await providerStore.resetFailures(provider.id)
    ElMessage.success('已重置')
  } else if (command === 'unblacklist') {
    await providerStore.unblacklist(provider.id)
    ElMessage.success('已解除拉黑')
  } else if (command === 'delete') {
    await ElMessageBox.confirm('确定删除该服务商?', '确认')
    await providerStore.deleteProvider(provider.id)
    ElMessage.success('已删除')
  }
}

onMounted(() => {
  providerStore.fetchProviders()
})
</script>

<style scoped>
.page-header {
  margin-bottom: 20px;
}

.provider-item {
  display: flex;
  align-items: center;
  padding: 15px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.provider-item:last-child {
  border-bottom: none;
}

.drag-handle {
  cursor: move;
  padding: 10px;
  color: var(--el-text-color-secondary);
}

.provider-info {
  flex: 1;
  margin-left: 10px;
}

.provider-name {
  font-weight: bold;
  margin-bottom: 5px;
}

.provider-url {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.provider-stats {
  display: flex;
  gap: 20px;
  margin-right: 20px;
  color: var(--el-text-color-regular);
  font-size: 14px;
}

.provider-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.form-tip {
  margin-left: 10px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.model-maps-section {
  padding: 0 20px;
}

.model-maps-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
}

.model-maps-tip {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.model-maps-empty {
  text-align: center;
  padding: 20px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
  border-radius: 4px;
}

.model-maps-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.model-map-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.model-input {
  flex: 1;
}

.arrow-icon {
  color: var(--el-text-color-secondary);
  font-size: 16px;
}
</style>
