<template>
  <div class="plugins-page">
    <el-tabs v-model="activeTab">
      <!-- 插件列表 -->
      <el-tab-pane label="插件" name="plugins">
        <div class="page-header">
          <el-input
            v-model="pluginSearchQuery"
            placeholder="搜索插件..."
            clearable
            style="width: 250px"
          >
            <template #prefix>
              <el-icon><Search /></el-icon>
            </template>
          </el-input>
          <el-button type="primary" @click="handleRefresh" :loading="loading">
            刷新列表
          </el-button>
        </div>
        <el-table :data="filteredPlugins" stripe style="width: 100%" v-loading="loading">
          <el-table-column label="名称" min-width="150">
            <template #default="{ row }">
              <div class="plugin-name-cell">
                <span>{{ row.name }}</span>
                <span v-if="row.version" class="plugin-meta">v{{ row.version }}</span>
                <span class="plugin-meta">@{{ row.marketplace_name }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="description" label="描述" min-width="200">
            <template #default="{ row }">
              <el-tooltip
                v-if="row.description"
                :content="row.description"
                placement="top"
                :show-after="300"
                :hide-after="0"
                popper-class="plugin-desc-tooltip"
              >
                <div class="plugin-desc">{{ row.description }}</div>
              </el-tooltip>
              <span v-else class="text-muted">-</span>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag v-if="row.is_installed" :type="row.is_enabled ? 'success' : 'info'" size="small">
                {{ row.is_enabled ? '已启用' : '已停用' }}
              </el-tag>
              <el-tag v-else type="warning" size="small">未安装</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="300">
            <template #default="{ row }">
              <div class="action-buttons">
                <template v-if="row.is_installed">
                  <el-button
                    v-if="row.is_enabled"
                    size="small"
                    :loading="operatingPlugin === row.name && operatingType === 'disable'"
                    :disabled="!!operatingPlugin"
                    @click="handleDisable(row)"
                  >停用</el-button>
                  <el-button
                    v-else
                    size="small"
                    type="primary"
                    :loading="operatingPlugin === row.name && operatingType === 'enable'"
                    :disabled="!!operatingPlugin"
                    @click="handleEnable(row)"
                  >启用</el-button>
                  <el-button
                    size="small"
                    type="warning"
                    :loading="operatingPlugin === row.name && operatingType === 'update'"
                    :disabled="!!operatingPlugin"
                    @click="handleUpdate(row)"
                  >更新</el-button>
                  <el-button
                    size="small"
                    type="danger"
                    :loading="operatingPlugin === row.name && operatingType === 'uninstall'"
                    :disabled="!!operatingPlugin"
                    @click="handleUninstall(row)"
                  >卸载</el-button>
                </template>
                <template v-else>
                  <el-button
                    size="small"
                    type="primary"
                    :loading="operatingPlugin === row.name && operatingType === 'install'"
                    :disabled="!!operatingPlugin"
                    @click="handleInstall(row)"
                  >安装</el-button>
                </template>
                <el-button
                  v-if="!row.is_favorited"
                  size="small"
                  :icon="Star"
                  :loading="operatingPlugin === row.name && operatingType === 'favorite'"
                  :disabled="!!operatingPlugin"
                  @click="handleAddFavorite(row)"
                />
                <el-button
                  v-else
                  size="small"
                  type="warning"
                  :icon="StarFilled"
                  :loading="operatingPlugin === row.name && operatingType === 'unfavorite'"
                  :disabled="!!operatingPlugin"
                  @click="handleRemoveFavorite(row)"
                />
              </div>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 市场管理 -->
      <el-tab-pane label="市场" name="marketplaces">
        <div class="page-header">
          <span></span>
          <el-button type="primary" @click="showAddMarketDialog = true">
            <el-icon><Plus /></el-icon>
            添加市场
          </el-button>
        </div>
        <el-table :data="marketplaceList" stripe style="width: 100%" v-loading="loadingMarketplaces">
          <el-table-column prop="name" label="名称" min-width="150" />
          <el-table-column prop="description" label="描述" min-width="200">
            <template #default="{ row }">
              {{ row.description || '-' }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="160">
            <template #default="{ row }">
              <el-button
                size="small"
                type="primary"
                @click="handleUpdateMarketplace(row)"
              >更新</el-button>
              <el-button
                size="small"
                type="danger"
                @click="handleRemoveMarketplace(row)"
              >删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 收藏列表 -->
      <el-tab-pane label="收藏" name="favorites">
        <div class="favorites-tip">收藏的插件会跟随备份跨设备同步</div>
        <el-table :data="favoriteList" stripe style="width: 100%">
          <el-table-column label="名称" min-width="150">
            <template #default="{ row }">
              <div class="plugin-name-cell">
                <span>{{ row.name }}</span>
                <span v-if="row.version" class="plugin-meta">v{{ row.version }}</span>
                <span class="plugin-meta">@{{ row.marketplace_name }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="description" label="描述" min-width="200">
            <template #default="{ row }">
              <el-tooltip
                v-if="row.description"
                :content="row.description"
                placement="top"
                :show-after="300"
                :hide-after="0"
                popper-class="plugin-desc-tooltip"
              >
                <div class="plugin-desc">{{ row.description }}</div>
              </el-tooltip>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="row.is_installed ? 'success' : 'info'" size="small">
                {{ row.is_installed ? '已安装' : '未安装' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="160">
            <template #default="{ row }">
              <template v-if="!row.is_installed">
                <el-button
                  size="small"
                  type="primary"
                  :loading="operatingPlugin === row.name && operatingType === 'install'"
                  :disabled="!!operatingPlugin"
                  @click="handleInstall(row)"
                >安装</el-button>
              </template>
              <el-button
                size="small"
                type="danger"
                :loading="operatingPlugin === row.name && operatingType === 'unfavorite'"
                :disabled="!!operatingPlugin"
                @click="handleRemoveFavoriteById(row)"
              >移除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <!-- 添加市场对话框 -->
    <el-dialog v-model="showAddMarketDialog" title="添加市场" width="500px">
      <el-form :model="marketForm" label-width="80px">
        <el-form-item label="市场地址" required>
          <el-input v-model="marketForm.url" placeholder="支持URL地址、owner/repo、本地路径" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showAddMarketDialog = false" :disabled="savingMarket">取消</el-button>
        <el-button type="primary" @click="handleAddMarketplace" :loading="savingMarket">添加</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessageBox, ElNotification } from 'element-plus'
import { Plus, Search, Star, StarFilled } from '@element-plus/icons-vue'
import { pluginsApi } from '@/api/plugins'
import type { MarketplaceInfo, PluginItem } from '@/types/models'

const activeTab = ref('plugins')

// 插件列表
const allPlugins = ref<PluginItem[]>([])
const loading = ref(false)
const pluginSearchQuery = ref('')

// 市场列表
const marketplaceList = ref<MarketplaceInfo[]>([])
const loadingMarketplaces = ref(false)
const showAddMarketDialog = ref(false)
const marketForm = ref({ url: '' })
const savingMarket = ref(false)

// 操作状态
const operatingPlugin = ref<string | null>(null)
const operatingType = ref<string | null>(null)

// 已安装排顶部的排序列表
const sortedPlugins = computed(() => {
  const installed = allPlugins.value.filter(p => p.is_installed)
  const notInstalled = allPlugins.value.filter(p => !p.is_installed)
  return [...installed, ...notInstalled]
})

const filteredPlugins = computed(() => {
  if (!pluginSearchQuery.value) return sortedPlugins.value
  const query = pluginSearchQuery.value.toLowerCase()
  return sortedPlugins.value.filter(p =>
    p.name.toLowerCase().includes(query) ||
    p.description?.toLowerCase().includes(query) ||
    p.marketplace_name.toLowerCase().includes(query)
  )
})

// 收藏列表（从插件列表过滤）
const favoriteList = computed(() => {
  return allPlugins.value.filter(p => p.is_favorited)
})

// 获取 plugin_id
function getPluginId(plugin: PluginItem): string {
  return plugin.marketplace_name ? `${plugin.name}@${plugin.marketplace_name}` : plugin.name
}

// 展示 CLI 输出
function showCliOutput(output: string, isError: boolean = false) {
  if (!output) return
  ElNotification({
    title: isError ? '操作失败' : '操作结果',
    message: output,
    type: isError ? 'error' : 'success',
    duration: 5000,
    position: 'top-right'
  })
}

// 加载所有数据
async function loadAll() {
  loading.value = true
  loadingMarketplaces.value = true
  try {
    const [plugins, marketplaces] = await Promise.all([
      pluginsApi.getAll(),
      pluginsApi.getMarketplaces()
    ])
    allPlugins.value = plugins
    marketplaceList.value = marketplaces
  } catch (error: any) {
    const msg = error?.message || String(error) || '加载失败'
    ElNotification({
      title: '加载失败',
      message: msg,
      type: 'error',
      duration: 5000,
      position: 'top-right'
    })
  } finally {
    loading.value = false
    loadingMarketplaces.value = false
  }
}

// 刷新
async function handleRefresh() {
  loading.value = true
  try {
    allPlugins.value = await pluginsApi.refresh()
    showCliOutput('刷新成功')
  } catch (error: any) {
    showCliOutput(error?.message || '刷新失败', true)
  } finally {
    loading.value = false
  }
}

// 安装插件
async function handleInstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'install'
  try {
    const result = await pluginsApi.install(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '安装失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 卸载插件
async function handleUninstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  try {
    await ElMessageBox.confirm(`确定卸载插件 "${plugin.name}"?`, '确认')
    operatingPlugin.value = plugin.name
    operatingType.value = 'uninstall'
    const result = await pluginsApi.uninstall(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    if (error !== 'cancel') {
      showCliOutput(error?.message || '卸载失败', true)
    }
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 启用插件
async function handleEnable(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'enable'
  try {
    const result = await pluginsApi.enable(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 停用插件
async function handleDisable(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'disable'
  try {
    const result = await pluginsApi.disable(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 更新插件
async function handleUpdate(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'update'
  try {
    const result = await pluginsApi.update(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '更新失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 添加收藏
async function handleAddFavorite(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'favorite'
  try {
    const result = await pluginsApi.addFavorite(
      pluginId,
      plugin.name,
      plugin.marketplace_name,
      plugin.version || undefined,
      plugin.description || undefined
    )
    allPlugins.value = result.plugins
    showCliOutput('已收藏')
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 移除收藏
async function handleRemoveFavorite(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'unfavorite'
  try {
    const result = await pluginsApi.removeFavorite(pluginId)
    allPlugins.value = result.plugins
    showCliOutput('已取消收藏')
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 从收藏中移除（收藏列表页）
async function handleRemoveFavoriteById(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operatingPlugin.value = plugin.name
  operatingType.value = 'unfavorite'
  try {
    const result = await pluginsApi.removeFavorite(pluginId)
    allPlugins.value = result.plugins
    showCliOutput('已移除')
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    operatingPlugin.value = null
    operatingType.value = null
  }
}

// 添加市场
async function handleAddMarketplace() {
  if (!marketForm.value.url.trim()) {
    showCliOutput('请输入市场 URL', true)
    return
  }
  savingMarket.value = true
  try {
    const result = await pluginsApi.addMarketplace(marketForm.value.url.trim())
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    showCliOutput(result.cli_output)
    showAddMarketDialog.value = false
    marketForm.value = { url: '' }
  } catch (error: any) {
    showCliOutput(error?.message || '添加失败', true)
  } finally {
    savingMarket.value = false
  }
}

// 删除市场
async function handleRemoveMarketplace(market: MarketplaceInfo) {
  try {
    await ElMessageBox.confirm(`确定删除市场 "${market.name}"?`, '确认')
    const result = await pluginsApi.removeMarketplace(market.name)
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    showCliOutput(result.cli_output)
  } catch (error: any) {
    if (error !== 'cancel') {
      showCliOutput(error?.message || '删除失败', true)
    }
  }
}

// 更新市场
async function handleUpdateMarketplace(market: MarketplaceInfo) {
  try {
    const result = await pluginsApi.updateMarketplace(market.name)
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '更新失败', true)
  }
}

onMounted(() => {
  loadAll()
})
</script>

<style scoped>
.plugins-page {
  height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.text-muted {
  color: #909399;
}

.plugin-name-cell {
  display: flex;
  flex-direction: column;
}

.plugin-meta {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.action-buttons {
  display: flex;
  flex-wrap: nowrap;
  gap: 8px;
}

.favorites-tip {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  margin-bottom: 16px;
}

.plugin-desc {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.5;
}
</style>

<style>
.plugin-desc-tooltip {
  max-width: 400px;
  word-break: break-word;
}
</style>