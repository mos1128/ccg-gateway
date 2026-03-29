<template>
  <div class="plugins-page">
    <!-- Icon Symbols -->
    <svg style="display:none">
      <defs>
        <symbol id="icon-puzzle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19.439 12.33a2.99 2.99 0 0 1 2.56 2.553 2.99 2.99 0 0 1-2.56 2.553h-1.99a2.99 2.99 0 0 1-2.553 2.56 2.99 2.99 0 0 1-2.553-2.56h-1.99a2.99 2.99 0 0 1-2.56-2.553 2.99 2.99 0 0 1 2.56-2.553h1.99a2.99 2.99 0 0 1 2.553-2.56 2.99 2.99 0 0 1 2.553 2.56Z"/>
          <path d="M15 2H9a2 2 0 0 0-2 2v2h10V4a2 2 0 0 0-2-2Z"/>
          <path d="M5 20v-8a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2Z"/>
        </symbol>
        <symbol id="icon-star" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
        </symbol>
        <symbol id="icon-store" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 9 12 3l9 6v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><polyline points="9 22 9 12 15 12 15 22"/>
        </symbol>
        <symbol id="icon-search" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
        </symbol>
        <symbol id="icon-refresh" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/>
        </symbol>
        <symbol id="icon-plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
        <symbol id="icon-download" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
        </symbol>
      </defs>
    </svg>

    <!-- Top Tabs -->
    <div class="top-tabs">
      <div class="tab-item" :class="{ active: activeTab === 'plugins' }" @click="activeTab = 'plugins'">插件</div>
      <div class="tab-item" :class="{ active: activeTab === 'marketplaces' }" @click="activeTab = 'marketplaces'">市场</div>
      <div class="tab-item" :class="{ active: activeTab === 'favorites' }" @click="activeTab = 'favorites'">收藏夹</div>
    </div>

    <!-- Main Content Area -->
    <div class="view-content-wrapper">
      
      <!-- TAB: PLUGINS -->
      <div v-if="activeTab === 'plugins'" class="tab-pane">
        <div class="page-header">
          <p class="page-subtitle">选择插件安装到Claude Code</p>
          <div style="display: flex; gap: 12px; align-items: center;">
            <div class="search-box" style="width: 260px; position: relative;">
              <svg class="search-icon" width="16" height="16" style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; pointer-events: none; z-index: 1;"><use href="#icon-search"/></svg>
              <input type="text" v-model="pluginSearchQuery" class="c-input" placeholder="搜索..." style="height: 38px; padding: 0 12px 0 36px; margin: 0; box-shadow: none;">
            </div>
            <button class="b-button-outline" style="padding: 0; width: 38px; height: 38px; display: flex; align-items: center; justify-content: center; margin: 0; box-shadow: none; flex-shrink: 0;" :disabled="loading" @click="handleRefresh" title="同步刷新">
              <svg width="20" height="20" :class="{ 'spin': loading }"><use href="#icon-refresh"/></svg>
            </button>
          </div>
        </div>

        <div v-loading="loading">
          <template v-if="filteredPlugins.length === 0">
            <div class="empty-state">
              <svg width="64" height="64" color="#e2e8f0"><use href="#icon-puzzle"/></svg>
              <p>暂无插件，尝试刷新或添加新市场</p>
            </div>
          </template>
          <div v-else class="plugin-grid">
            <div v-for="plugin in filteredPlugins" :key="getPluginId(plugin)" class="plugin-card" :class="{ 'not-installed': !plugin.is_installed }">
              <div class="card-header">
                <div class="plugin-icon-box" :class="{ disabled: plugin.is_installed && !plugin.is_enabled }">
                  <svg width="20" height="20"><use href="#icon-puzzle"/></svg>
                </div>
                <div class="header-main">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <h3 class="plugin-name">{{ plugin.name }}</h3>
                    <span v-if="plugin.version" class="plugin-ver mono">v{{ plugin.version }}</span>
                  </div>
                  <div class="plugin-market">@{{ plugin.marketplace_name }}</div>
                </div>
                <div class="header-star" @click="favoriteIds.has(getPluginId(plugin)) ? handleRemoveFavorite(plugin) : handleAddFavorite(plugin)">
                  <svg width="18" height="18" :style="favoriteIds.has(getPluginId(plugin)) ? 'fill: #f59e0b; color: #f59e0b;' : 'color: #cbd5e1;'"><use href="#icon-star"/></svg>
                </div>
              </div>

              <div class="plugin-body">
                <el-tooltip
                  v-if="plugin.description"
                  effect="light"
                  placement="top"
                  :enterable="true"
                  :show-after="200"
                >
                  <template #content>
                    <div style="max-width: 350px; line-height: 1.6; font-size: 13px; word-break: break-word; user-select: text; color: #334155;">
                      {{ plugin.description }}
                    </div>
                  </template>
                  <div class="plugin-desc" @click="copyDescription(plugin.description)">
                    {{ plugin.description }}
                  </div>
                </el-tooltip>
                <div v-else class="plugin-desc">
                  该插件暂无详细描述信息。
                </div>
              </div>

              <div class="plugin-footer">
                <div class="status-zone">
                  <div v-if="plugin.is_installed" class="status-pill" :class="plugin.is_enabled ? 'active' : 'inactive'">
                    {{ plugin.is_enabled ? '已启用' : '已停用' }}
                  </div>
                  <div v-else class="status-pill none">未安装</div>
                </div>
                
                <div class="actions-zone">
                  <template v-if="plugin.is_installed">
                    <button class="btn-sm" :class="plugin.is_enabled ? 'danger' : 'primary'" @click="plugin.is_enabled ? handleDisable(plugin) : handleEnable(plugin)">
                      {{ plugin.is_enabled ? '停用' : '启用' }}
                    </button>
                    <button class="btn-sm outline" @click="handleUpdate(plugin)">更新</button>
                    <button class="btn-sm outline danger-text" @click="handleUninstall(plugin)">卸载</button>
                  </template>
                  <template v-else>
                    <button class="btn-sm primary" @click="handleInstall(plugin)">
                      <svg width="14" height="14" style="margin-right: 4px;"><use href="#icon-download"/></svg>
                      安装
                    </button>
                  </template>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB: MARKETPLACES -->
      <div v-else-if="activeTab === 'marketplaces'" class="tab-pane">
        <div class="page-header">
          <p class="page-subtitle">插件市场</p>
          <button class="b-button" style="padding: 10px;" @click="showAddMarketDialog = true" title="添加市场">
            <svg width="20" height="20"><use href="#icon-plus"/></svg>
          </button>
        </div>

        <div v-loading="loadingMarketplaces">
          <div v-if="marketplaceList.length === 0" class="empty-state">
             <svg width="64" height="64" color="#e2e8f0"><use href="#icon-store"/></svg>
             <p>暂无配置市场，请点击上方按钮添加</p>
          </div>
          <div v-else class="market-grid">
            <div v-for="market in marketplaceList" :key="market.name" class="market-card">
              <div class="market-icon">
                <svg width="24" height="24"><use href="#icon-store"/></svg>
              </div>
              <div class="market-info">
                <div class="market-name">{{ market.name }}</div>
                <el-tooltip
                  effect="light"
                  placement="top"
                  :enterable="true"
                  :show-after="200"
                >
                  <template #content>
                    <div style="max-width: 350px; line-height: 1.6; font-size: 13px; word-break: break-all; user-select: text; color: #334155;">
                      {{ market.marketplace_source || '内建市场' }}
                    </div>
                  </template>
                  <div class="market-url mono" @click="copyDescription(market.marketplace_source || '内建市场')" style="cursor: pointer;">{{ market.marketplace_source || '内建市场' }}</div>
                </el-tooltip>
              </div>
              <div class="market-actions">
                <button class="btn-icon" title="同步市场" @click="handleUpdateMarketplace(market)">
                  <svg width="16" height="16"><use href="#icon-refresh"/></svg>
                </button>
                <button class="btn-icon danger" title="删除市场" @click="handleRemoveMarketplace(market)">
                  <svg width="16" height="16"><use href="#icon-trash"/></svg>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB: FAVORITES -->
      <div v-else class="tab-pane">
        <div class="page-header">
          <p class="page-subtitle">收藏的插件会同步到云端或随数据库备份，方便快速安装</p>
        </div>

        <div v-loading="loading">
          <div v-if="favoriteList.length === 0" class="empty-state">
            <svg width="64" height="64" color="#e2e8f0"><use href="#icon-star"/></svg>
            <p>暂无收藏插件</p>
          </div>
          <div v-else class="favorite-grid">
             <div v-for="fav in favoriteList" :key="fav.plugin_id" class="fav-card">
                <div class="fav-main">
                  <div class="fav-info">
                     <div class="fav-name">{{ fav.plugin_name }}</div>
                     <div class="fav-market">来自市场: {{ fav.marketplace_name }}</div>
                  </div>
                  <div class="fav-status">
                     <span class="pill" :class="fav.is_installed ? 'pill-green' : 'pill-grey'">
                       {{ fav.is_installed ? '已安装' : '未安装' }}
                     </span>
                  </div>
                </div>
                <div class="fav-footer">
                  <button v-if="!fav.is_installed" class="btn-sm primary" @click="handleInstallFavorite(fav)">立即安装</button>
                  <button class="btn-sm outline danger-text" @click="handleRemoveFavoriteById(fav)">移除收藏</button>
                </div>
             </div>
          </div>
        </div>
      </div>

    </div>

    <!-- Modals -->
    <AppModal v-model="showAddMarketDialog" title="添加插件市场" width="500px" :show-footer="false">
        <div class="form-group">
            <label class="c-label">市场源地址 <span class="required">*</span></label>
            <input
              type="text"
              v-model="marketForm.url"
              class="c-input"
              placeholder="支持 URL 地址、GitHub owner/repo、本地路径"
            >
          </div>

      <template #footer>
        <button class="b-button-outline" @click="showAddMarketDialog = false">取消</button>
        <button class="b-button" @click="handleAddMarketplace" :disabled="savingMarket">确认添加</button>
      </template>
    </AppModal>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessageBox, ElNotification } from 'element-plus'
import AppModal from '@/components/AppModal.vue'
import { pluginsApi } from '@/api/plugins'
import type { MarketplaceInfo, PluginItem, PluginFavoriteItem } from '@/types/models'

const activeTab = ref('plugins')

// Plugin State
const allPlugins = ref<PluginItem[]>([])
const loading = ref(false)
const pluginSearchQuery = ref('')

// Marketplace State
const marketplaceList = ref<MarketplaceInfo[]>([])
const loadingMarketplaces = ref(false)
const showAddMarketDialog = ref(false)
const marketForm = ref({ url: '' })
const savingMarket = ref(false)

// Favorites State
const favoriteList = ref<PluginFavoriteItem[]>([])

// Logic
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

const favoriteIds = computed(() => new Set(favoriteList.value.map(f => f.plugin_id)))

function getPluginId(plugin: PluginItem): string {
  return plugin.marketplace_name ? `${plugin.name}@${plugin.marketplace_name}` : plugin.name
}

function showCliOutput(output: string, isError: boolean = false) {
  if (!output) return
  ElNotification({
    title: isError ? '操作失败' : '操作结果',
    message: output.replace(/\n/g, '<br/>'),
    type: isError ? 'error' : 'success',
    duration: 5000,
    position: 'top-right',
    dangerouslyUseHTMLString: true
  })
}

async function loadAll() {
  loading.value = true
  loadingMarketplaces.value = true
  try {
    const [plugins, marketplaces, favorites] = await Promise.all([
      pluginsApi.getAll(),
      pluginsApi.getMarketplaces(),
      pluginsApi.getFavorites()
    ])
    allPlugins.value = plugins
    marketplaceList.value = marketplaces
    favoriteList.value = favorites
  } catch (error: any) {
    notify(error?.message || '加载失败', 'error')
  } finally {
    loading.value = false
    loadingMarketplaces.value = false
  }
}

async function handleRefresh() {
  loading.value = true
  try {
    allPlugins.value = await pluginsApi.refresh()
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput('刷新同步成功')
  } catch (error: any) {
    showCliOutput(error?.message || '刷新失败', true)
  } finally {
    loading.value = false
  }
}

async function handleInstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  loading.value = true
  try {
    const result = await pluginsApi.install(pluginId)
    allPlugins.value = result.plugins
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '安装失败', true)
  } finally {
    loading.value = false
  }
}

async function handleUninstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  try {
    await ElMessageBox.confirm(`确定卸载插件 "${plugin.name}"?`, '确认卸载')
    loading.value = true
    const result = await pluginsApi.uninstall(pluginId)
    allPlugins.value = result.plugins
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      showCliOutput(error?.message || '卸载失败', true)
    }
  } finally {
    loading.value = false
  }
}

async function handleEnable(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  loading.value = true
  try {
    const result = await pluginsApi.enable(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    loading.value = false
  }
}

async function handleDisable(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  loading.value = true
  try {
    const result = await pluginsApi.disable(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '操作失败', true)
  } finally {
    loading.value = false
  }
}

async function handleUpdate(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  loading.value = true
  try {
    const result = await pluginsApi.update(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '更新失败', true)
  } finally {
    loading.value = false
  }
}

async function handleAddFavorite(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  try {
    await pluginsApi.addFavorite(pluginId, plugin.name, plugin.marketplace_name)
    favoriteList.value = await pluginsApi.getFavorites()
    notify('已收藏')
  } catch (error: any) {
    notify(error?.message || '操作失败', 'error')
  }
}

async function handleRemoveFavorite(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  try {
    await pluginsApi.removeFavorite(pluginId)
    favoriteList.value = await pluginsApi.getFavorites()
    notify('已取消收藏')
  } catch (error: any) {
    notify(error?.message || '操作失败', 'error')
  }
}

async function handleInstallFavorite(favorite: PluginFavoriteItem) {
  loading.value = true
  try {
    const result = await pluginsApi.installFavorite(
      favorite.plugin_id,
      favorite.marketplace_name,
      favorite.marketplace_source ?? undefined
    )
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '安装失败', true)
  } finally {
    loading.value = false
  }
}

async function handleRemoveFavoriteById(favorite: PluginFavoriteItem) {
  try {
    await pluginsApi.removeFavorite(favorite.plugin_id)
    favoriteList.value = await pluginsApi.getFavorites()
    notify('已移除')
  } catch (error: any) {
    notify(error?.message || '操作失败', 'error')
  }
}

async function handleAddMarketplace() {
  if (!marketForm.value.url.trim()) {
    notify('请输入市场地址', 'error')
    return
  }
  savingMarket.value = true
  try {
    const result = await pluginsApi.addMarketplace(marketForm.value.url.trim())
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
    showAddMarketDialog.value = false
    marketForm.value = { url: '' }
  } catch (error: any) {
    showCliOutput(error?.message || '添加失败', true)
  } finally {
    savingMarket.value = false
  }
}

async function handleRemoveMarketplace(market: MarketplaceInfo) {
  try {
    await ElMessageBox.confirm(`确定删除市场 "${market.name}"?`, '确认删除')
    loadingMarketplaces.value = true
    const result = await pluginsApi.removeMarketplace(market.name)
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      showCliOutput(error?.message || '删除失败', true)
    }
  } finally {
    loadingMarketplaces.value = false
  }
}

async function handleUpdateMarketplace(market: MarketplaceInfo) {
  loadingMarketplaces.value = true
  try {
    const result = await pluginsApi.updateMarketplace(market.name)
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(error?.message || '更新失败', true)
  } finally {
    loadingMarketplaces.value = false
  }
}

async function copyDescription(text: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    notify('描述已复制')
  } catch {
    notify('复制失败', 'error')
  }
}

function notify(message: string, type: 'success' | 'error' | 'warning' | 'info' = 'success') {
  ElNotification({
    title: type === 'success' ? '成功' : type === 'error' ? '错误' : '提示',
    message,
    type,
    duration: 3000,
    position: 'top-right'
  })
}

onMounted(loadAll)
</script>

<style scoped>
.plugins-page {
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin-bottom: 24px; padding-top: 8px; }
.tab-item { padding-bottom: 12px; color: #94a3b8; font-weight: 500; font-size: 15px; cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: #475569; }
.tab-item.active { color: #0f172a; font-weight: 600; border-bottom: 2px solid #0f172a; }

/* Header */
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; }
.page-subtitle { font-size: 14px; color: #64748b; margin: 0; }

/* Plugin Grid & Cards */
.plugin-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); gap: 24px; }
.plugin-card { 
  background: #ffffff; border-radius: 20px; border: 1px solid rgba(226, 232, 240, 0.8); 
  padding: 24px; box-shadow: 0 4px 12px rgba(0,0,0,0.03); transition: all 0.2s; 
  display: flex; flex-direction: column; gap: 16px; 
}
.plugin-card:hover { border-color: #0ea5e9; box-shadow: 0 10px 20px -5px rgba(0,0,0,0.05); }
.plugin-card.not-installed { opacity: 0.95; }

.card-header { display: flex; gap: 14px; align-items: flex-start; }
.plugin-icon-box { 
  width: 44px; height: 44px; border-radius: 12px; background: #e0f2fe; color: #0ea5e9; 
  display: flex; align-items: center; justify-content: center; flex-shrink: 0; transition: all 0.2s;
}
.plugin-icon-box.disabled { background: #f1f5f9; color: #94a3b8; }
.header-main { flex: 1; min-width: 0; }
.plugin-name { font-size: 16px; font-weight: 700; color: #0f172a; margin: 0 0 2px 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.plugin-ver { font-size: 11px; color: #94a3b8; }
.plugin-market { font-size: 12px; color: #64748b; font-weight: 500; }
.header-star { cursor: pointer; padding: 4px; border-radius: 4px; transition: background 0.2s; }
.header-star:hover { background: #f8fafc; }

.plugin-body { flex: 1; }
.plugin-desc { 
  font-size: 13px; color: #475569; line-height: 1.6; cursor: pointer;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-footer { display: flex; justify-content: space-between; align-items: center; padding-top: 12px; border-top: 1px dashed #f1f5f9; }
.status-pill { font-size: 11px; font-weight: 700; padding: 4px 10px; border-radius: 99px; text-transform: uppercase; }
.status-pill.active { background: #ecfdf5; color: #10b981; }
.status-pill.inactive { background: #f1f5f9; color: #64748b; }
.status-pill.none { background: #fffbeb; color: #f59e0b; }

.actions-zone { display: flex; gap: 8px; }
.btn-sm { 
  border: none; padding: 6px 12px; border-radius: 8px; font-size: 12px; font-weight: 600; 
  cursor: pointer; transition: all 0.2s; display: flex; align-items: center; 
}
.btn-sm.primary { background: #0ea5e9; color: white; }
.btn-sm.primary:hover { background: #0284c7; }
.btn-sm.outline { background: white; border: 1px solid #e2e8f0; color: #475569; }
.btn-sm.outline:hover { background: #f8fafc; color: #0f172a; }
.btn-sm.danger { background: #fef2f2; color: #f43f5e; }
.btn-sm.danger:hover { background: #fee2e2; }
.btn-sm.danger-text { color: #f43f5e; }
.btn-sm.danger-text:hover { background: #fef2f2; }

/* Market List */
.market-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(400px, 1fr)); gap: 20px; }
.market-card { 
  background: white; border-radius: 16px; border: 1px solid #f1f5f9; padding: 20px; 
  display: flex; align-items: center; gap: 16px; transition: all 0.2s; 
}
.market-card:hover { border-color: #0ea5e9; background: #f8fafc; }
.market-icon { width: 44px; height: 44px; border-radius: 12px; background: #f8fafc; color: #64748b; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.market-info { flex: 1; min-width: 0; }
.market-name { font-weight: 700; font-size: 15px; color: #0f172a; margin-bottom: 4px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.market-url { font-size: 12px; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.market-actions { display: flex; gap: 4px; flex-shrink: 0; }
.btn-icon { background: white; border: 1px solid #f1f5f9; color: #64748b; padding: 8px; border-radius: 8px; cursor: pointer; transition: all 0.2s; display: flex; align-items: center; justify-content: center; }
.btn-icon:hover { background: #f1f5f9; color: #0f172a; border-color: #cbd5e1; }
.btn-icon.danger:hover { background: #fef2f2; color: #f43f5e; border-color: #fca5a5; }

/* Favorites */
.favorite-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 20px; }
.fav-card { background: white; border-radius: 16px; border: 1px solid #f1f5f9; padding: 20px; display: flex; flex-direction: column; gap: 16px; }
.fav-main { display: flex; justify-content: space-between; align-items: flex-start; }
.fav-name { font-weight: 700; font-size: 16px; color: #0f172a; margin-bottom: 4px; }
.fav-market { font-size: 12px; color: #94a3b8; }
.fav-footer { display: flex; gap: 10px; padding-top: 12px; border-top: 1px dashed #f1f5f9; }

/* Shared Global styles */
.mono { font-family: "JetBrains Mono", monospace; }
.pill { padding: 4px 10px; border-radius: 999px; font-size: 11px; font-weight: 700; text-transform: uppercase; }
.pill-green { background: #ecfdf5; color: #10b981; }
.pill-grey { background: #f1f5f9; color: #64748b; }

.search-box { position: relative; }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; }
.c-input { 
  width: 100%; padding: 8px 12px 8px 36px; background: #ffffff; border: 1px solid #e2e8f0; 
  border-radius: 8px; font-size: 13px; color: #0f172a; outline: none; transition: all 0.2s; 
}
.c-input:focus { border-color: #0ea5e9; box-shadow: 0 0 0 3px rgba(14, 165, 233, 0.1); }

.b-button { 
  background: #0ea5e9; color: #ffffff; border: none; padding: 10px 20px; border-radius: 10px; 
  font-size: 14px; font-weight: 600; cursor: pointer; display: flex; align-items: center; 
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.2); transition: all 0.2s; 
}
.b-button:hover { background: #0284c7; transform: translateY(-1px); }

.b-button-outline { 
  background: #ffffff; color: #475569; border: 1px solid #e2e8f0; padding: 8px 16px; border-radius: 8px; 
  font-size: 13px; font-weight: 500; cursor: pointer; transition: all 0.2s; display: flex; align-items: center; 
}
.b-button-outline:hover { background: #f8fafc; color: #0f172a; border-color: #cbd5e1; }

.empty-state { padding: 80px 40px; text-align: center; color: #94a3b8; background: #ffffff; border-radius: 24px; border: 2px dashed #e2e8f0; }
.empty-state p { margin-top: 16px; font-size: 15px; }

.form-group { margin-bottom: 24px; }
.c-label { display: block; font-size: 14px; font-weight: 600; color: #475569; margin-bottom: 8px; }
.required { color: #f43f5e; }
</style>
