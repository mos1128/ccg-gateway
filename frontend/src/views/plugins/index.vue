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
        <symbol id="icon-back" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>
        </symbol>
      </defs>
    </svg>

    <!-- Top Tabs -->
    <div class="top-tabs">
      <div class="tab-item" :class="{ active: activeTab === 'plugins' }" @click="activeTab = 'plugins'">插件</div>
      <div class="tab-item" :class="{ active: activeTab === 'marketplaces' }" @click="activeTab = 'marketplaces'">市场</div>
      <div class="tab-item" :class="{ active: activeTab === 'favorites' }" @click="activeTab = 'favorites'">收藏</div>
    </div>

    <!-- Main Content Area -->
    <div class="view-content-wrapper" v-loading="operationLoading">
      
      <!-- TAB: INSTALLED PLUGINS -->
      <div v-if="activeTab === 'plugins'" class="tab-pane">
        <div v-loading="loading" class="list-container">
          <template v-if="installedPlugins.length === 0">
            <div class="empty-state">
              <svg width="64" height="64" color="#e2e8f0"><use href="#icon-puzzle"/></svg>
              <p>暂无已安装插件</p>
            </div>
          </template>
          <div v-else class="scroll-area">
            <div class="skill-grid">
              <div v-for="plugin in installedPlugins" :key="getPluginId(plugin)" class="skill-card">
                <div class="card-top">
                  <div class="skill-icon">
                    <svg width="24" height="24"><use href="#icon-puzzle"/></svg>
                  </div>
                  <div class="skill-info">
                    <div style="display: flex; align-items: center; gap: 8px; min-width: 0;">
                      <h3 class="skill-name">{{ plugin.name }}</h3>
                      <span v-if="plugin.version" class="plugin-ver mono">v{{ plugin.version }}</span>
                    </div>
                    <div class="skill-market">@{{ plugin.marketplace_name }}</div>
                  </div>
                  <div class="card-actions">
                    <button
                      class="action-icon star"
                      :class="{ 'star-active': favoriteIds.has(getPluginId(plugin)) }"
                      title="收藏/取消"
                      @click="favoriteIds.has(getPluginId(plugin)) ? handleRemoveFavorite(plugin) : handleAddFavorite(plugin)"
                    >
                      <svg width="18" height="18" :style="favoriteIds.has(getPluginId(plugin)) ? 'fill: #f59e0b;' : ''"><use href="#icon-star"/></svg>
                    </button>
                    <button class="action-icon" title="更新" :disabled="operationLoading" @click="handleUpdate(plugin)">
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button class="action-icon delete" title="卸载" :disabled="operationLoading" @click="handleUninstall(plugin)">
                      <svg width="18" height="18"><use href="#icon-trash"/></svg>
                    </button>
                  </div>
                </div>

                <div class="cli-toggles">
                  <div class="toggle-item">
                    <span class="toggle-label">Claude Code</span>
                    <el-switch
                      size="small"
                      :model-value="plugin.is_enabled"
                      @change="handleToggleEnable(plugin, $event as boolean)"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB: MARKETPLACES -->
      <div v-else-if="activeTab === 'marketplaces'" class="tab-pane">
        <!-- Market List View -->
        <div v-if="!currentMarket" class="repo-list-view">
          <div class="page-header">
            <p class="page-subtitle">从市场发现并安装插件</p>
            <button class="b-button" style="padding: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center;" @click="showAddMarketDialog = true" title="添加市场">
              <svg width="20" height="20"><use href="#icon-plus"/></svg>
            </button>
          </div>

          <div v-loading="loadingMarketplaces" class="list-container">
            <div v-if="marketplaceList.length === 0" class="empty-state">
               <svg width="64" height="64" color="#e2e8f0"><use href="#icon-store"/></svg>
               <p>暂无配置市场，请点击上方按钮添加</p>
            </div>
            <div v-else class="scroll-area">
              <div class="repo-grid">
                <div v-for="market in marketplaceList" :key="market.name" class="repo-card" @click="handleMarketClick(market)">
                  <div class="repo-icon-box">
                    <svg width="24" height="24"><use href="#icon-store"/></svg>
                  </div>
                  <div class="repo-info-main">
                    <div class="repo-name-title">{{ market.name }}</div>
                    <div class="repo-source-subtitle mono">{{ market.marketplace_source || '内建市场' }}</div>
                  </div>
                  <div class="repo-actions-overlay" @click.stop>
                    <button class="action-icon" title="同步市场" @click="handleUpdateMarketplace(market)">
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button class="action-icon delete" title="删除市场" @click="handleRemoveMarketplace(market)">
                      <svg width="18" height="18"><use href="#icon-trash"/></svg>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Market Plugins List View -->
        <div v-else class="repo-skills-view">
          <div class="page-header">
            <div style="display: flex; align-items: center; gap: 16px;">
              <button class="action-icon" @click="handleBackToMarkets" title="返回">
                <svg width="18" height="18"><use href="#icon-back"/></svg>
              </button>
              <div>
                <h2 class="page-title" style="font-size: 20px; margin-bottom: 2px;">{{ currentMarket.name }}</h2>
                <div class="mono" style="font-size: 13px; color: #94a3b8;">{{ currentMarket.marketplace_source || '内建市场' }}</div>
              </div>
            </div>
            <div style="display: flex; gap: 12px; align-items: center;">
              <div class="search-box" style="width: 240px; position: relative;">
                <svg class="search-icon" width="16" height="16" style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; pointer-events: none; z-index: 1;"><use href="#icon-search"/></svg>
                <input type="text" v-model="pluginSearchQuery" class="c-input" placeholder="搜索..." style="height: 38px; padding: 0 12px 0 36px; margin: 0;">
              </div>
              <button class="action-icon" :disabled="loading" @click="handleUpdateMarketplace(currentMarket)" title="刷新市场">
                <svg width="18" height="18"><use href="#icon-refresh"/></svg>
              </button>
            </div>
          </div>

          <div v-loading="loading || operationLoading" class="list-container">
            <template v-if="marketPlugins.length === 0">
              <el-empty :description="pluginSearchQuery ? '无匹配结果' : '该市场暂无插件'" />
            </template>
            <div v-else class="scroll-area">
              <div class="discover-list">
                <div v-for="plugin in marketPlugins" :key="getPluginId(plugin)" class="discover-item">
                  <div class="discover-info">
                    <div class="discover-name-row">
                      <span class="discover-name">{{ plugin.name }}</span>
                      <span v-if="plugin.version" class="mono" style="font-size: 11px; color: #94a3b8;">v{{ plugin.version }}</span>
                    </div>
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
                      <div class="discover-desc" @click="copyDescription(plugin.description)">
                        {{ plugin.description }}
                      </div>
                    </el-tooltip>
                    <div v-else class="discover-desc">暂无描述</div>
                  </div>
                  <div class="discover-actions">
                    <button
                      v-if="plugin.is_installed"
                      class="action-icon installed"
                      title="更新"
                      :disabled="installingPluginId === getPluginId(plugin)"
                      @click="handleUpdate(plugin)"
                    >
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button
                      v-else
                      class="action-icon install"
                      title="安装插件"
                      :disabled="installingPluginId === getPluginId(plugin)"
                      @click="handleInstall(plugin)"
                    >
                      <svg width="18" height="18"><use href="#icon-plus"/></svg>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB: FAVORITES -->
      <div v-else class="tab-pane">
        <div class="page-header">
          <p class="page-subtitle">收藏的插件会保留市场信息，方便后续快速安装</p>
        </div>

        <div v-loading="loading || operationLoading" class="list-container">
          <div v-if="favoriteList.length === 0" class="empty-state">
            <svg width="64" height="64" color="#e2e8f0"><use href="#icon-star"/></svg>
            <p>暂无收藏插件</p>
          </div>
          <div v-else class="scroll-area">
            <div class="favorite-grid">
              <div v-for="fav in favoriteList" :key="fav.plugin_id" class="fav-card">
                <div class="fav-main">
                  <div class="fav-info">
                    <div class="fav-name">{{ fav.plugin_name }}</div>
                    <div class="fav-market" :title="fav.marketplace_source">来自市场: {{ fav.marketplace_name }}</div>
                  </div>
                  <div class="fav-actions">
                    <button
                      class="action-icon star-active"
                      title="取消收藏"
                      @click="handleRemoveFavoriteById(fav)"
                    >
                      <svg width="18" height="18" style="fill: #f59e0b;"><use href="#icon-star"/></svg>
                    </button>
                    <button
                      v-if="fav.is_installed"
                      class="action-icon installed"
                      title="已安装(点击更新)"
                      :disabled="installingPluginId === fav.plugin_id"
                      @click="handleInstallFavorite(fav)"
                    >
                      <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                    </button>
                    <button
                      v-else
                      class="action-icon install"
                      title="安装插件"
                      :disabled="installingPluginId === fav.plugin_id"
                      @click="handleInstallFavorite(fav)"
                    >
                      <svg width="18" height="18"><use href="#icon-plus"/></svg>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

    </div>

    <!-- Modals -->
    <AppModal v-model="showAddMarketDialog" title="添加插件市场" width="500px" @confirm="handleAddMarketplace">
        <div class="form-group">
            <label class="c-label">市场源地址 <span class="required">*</span></label>
            <input
              type="text"
              v-model="marketForm.url"
              class="c-input"
              placeholder="支持 URL 地址、GitHub owner/repo、本地路径"
            >
          </div>
    </AppModal>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessageBox, ElNotification } from 'element-plus'
import AppModal from '@/components/AppModal.vue'
import { pluginsApi } from '@/api/plugins'
import { getErrorMessage } from '@/utils/error'
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

// Favorites State
const favoriteList = ref<PluginFavoriteItem[]>([])

// Operation State
const currentMarket = ref<MarketplaceInfo | null>(null)
const operationLoading = ref(false)
const installingPluginId = ref<string | null>(null)

// Logic
const installedPlugins = computed(() => allPlugins.value.filter(p => p.is_installed))

const marketPlugins = computed(() => {
  if (!currentMarket.value) return []
  let list = allPlugins.value.filter(p => p.marketplace_name === currentMarket.value?.name)
  if (pluginSearchQuery.value) {
    const q = pluginSearchQuery.value.toLowerCase()
    list = list.filter(p =>
      p.name.toLowerCase().includes(q) ||
      p.description?.toLowerCase().includes(q)
    )
  }
  return list
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
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loading.value = false
    loadingMarketplaces.value = false
  }
}

function handleMarketClick(market: MarketplaceInfo) {
  currentMarket.value = market
  pluginSearchQuery.value = ''
}

function handleBackToMarkets() {
  currentMarket.value = null
  pluginSearchQuery.value = ''
}

async function handleToggleEnable(plugin: PluginItem, enabled: boolean) {
  operationLoading.value = true
  const pluginId = getPluginId(plugin)
  try {
    const result = enabled
      ? await pluginsApi.enable(pluginId)
      : await pluginsApi.disable(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '操作失败'), true)
  } finally {
    operationLoading.value = false
  }
}

async function handleInstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operationLoading.value = true
  installingPluginId.value = pluginId
  try {
    const result = await pluginsApi.install(pluginId)
    allPlugins.value = result.plugins
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '安装失败'), true)
  } finally {
    operationLoading.value = false
    installingPluginId.value = null
  }
}

async function handleUninstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  try {
    await ElMessageBox.confirm(`确定卸载插件 "${plugin.name}"?`, '确认卸载')
    operationLoading.value = true
    const result = await pluginsApi.uninstall(pluginId)
    allPlugins.value = result.plugins
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      showCliOutput(getErrorMessage(error, '卸载失败'), true)
    }
  } finally {
    operationLoading.value = false
  }
}

async function handleUpdate(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  operationLoading.value = true
  installingPluginId.value = pluginId
  try {
    const result = await pluginsApi.update(pluginId)
    allPlugins.value = result.plugins
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '更新失败'), true)
  } finally {
    operationLoading.value = false
    installingPluginId.value = null
  }
}

async function handleAddFavorite(plugin: PluginItem) {
  operationLoading.value = true
  const pluginId = getPluginId(plugin)
  try {
    await pluginsApi.addFavorite(pluginId, plugin.name, plugin.marketplace_name)
    favoriteList.value = await pluginsApi.getFavorites()
    notify('已收藏')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    operationLoading.value = false
  }
}

async function handleRemoveFavorite(plugin: PluginItem) {
  operationLoading.value = true
  const pluginId = getPluginId(plugin)
  try {
    await pluginsApi.removeFavorite(pluginId)
    favoriteList.value = await pluginsApi.getFavorites()
    notify('已取消收藏')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    operationLoading.value = false
  }
}

async function handleInstallFavorite(favorite: PluginFavoriteItem) {
  operationLoading.value = true
  installingPluginId.value = favorite.plugin_id
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
    showCliOutput(getErrorMessage(error, '安装失败'), true)
  } finally {
    operationLoading.value = false
    installingPluginId.value = null
  }
}

async function handleRemoveFavoriteById(favorite: PluginFavoriteItem) {
  operationLoading.value = true
  try {
    await pluginsApi.removeFavorite(favorite.plugin_id)
    favoriteList.value = await pluginsApi.getFavorites()
    notify('已移除')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    operationLoading.value = false
  }
}

async function handleAddMarketplace() {
  if (!marketForm.value.url.trim()) {
    notify('请输入市场地址', 'error')
    return
  }
  
  const url = marketForm.value.url.trim()
  showAddMarketDialog.value = false
  marketForm.value = { url: '' }
  
  loadingMarketplaces.value = true
  try {
    const result = await pluginsApi.addMarketplace(url)
    allPlugins.value = result.plugins
    marketplaceList.value = result.marketplaces
    favoriteList.value = await pluginsApi.getFavorites()
    showCliOutput(result.cli_output)
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '添加失败'), true)
  } finally {
    loadingMarketplaces.value = false
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
      showCliOutput(getErrorMessage(error, '删除失败'), true)
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
    showCliOutput(getErrorMessage(error, '更新失败'), true)
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
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }
.tab-item { padding-bottom: 12px; color: #94a3b8; font-weight: 500; font-size: 15px; cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: #475569; }
.tab-item.active { color: #0f172a; font-weight: 600; border-bottom: 2px solid #0f172a; }

.view-content-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: 0 40px;
}

.tab-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.repo-list-view, .repo-skills-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.list-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

/* Header */
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; flex-shrink: 0; }
.page-subtitle { font-size: 14px; color: #64748b; margin: 0; }

/* Grid & Cards (Installed) */
.skill-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 24px; }
.skill-card { 
  background: #ffffff; border-radius: 16px; border: 1px solid rgba(226, 232, 240, 0.8); padding: 24px; 
  box-shadow: 0 4px 12px rgba(0,0,0,0.03); transition: all 0.2s; display: flex; flex-direction: column; gap: 20px; 
}
.skill-card:hover { border-color: #0ea5e9; box-shadow: 0 10px 20px -5px rgba(0,0,0,0.05); }

.card-top { display: flex; gap: 16px; align-items: flex-start; }
.skill-icon { 
  width: 48px; height: 48px; border-radius: 12px; background: #e0f2fe; color: #0ea5e9; 
  display: flex; align-items: center; justify-content: center; flex-shrink: 0; 
}
.skill-info { flex: 1; min-width: 0; }
.skill-name {
  font-size: 16px; font-weight: 700; color: #0f172a; margin: 0 0 4px 0;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  overflow: hidden; text-overflow: ellipsis;
}.plugin-ver { font-size: 11px; color: #94a3b8; }
.skill-market { 
  font-size: 12px; color: #64748b; font-weight: 500; 
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; 
}

.card-actions { display: flex; gap: 4px; flex-shrink: 0; }
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
  outline: none;
  background: transparent;
  border: none;
}
.action-icon:hover { background: #f1f5f9; color: #0f172a; }
.action-icon.delete:hover { background: #fee2e2; color: #ef4444; }
.action-icon.star:disabled { cursor: not-allowed; opacity: 0.45; }
.action-icon.star:disabled:hover { background: transparent; color: #64748b; }
.action-icon.star-active { color: #f59e0b; background: rgba(245, 158, 11, 0.1); }
.action-icon.installed { color: #f59e0b; background: rgba(245, 158, 11, 0.1); }
.action-icon.install { color: #0ea5e9; background: rgba(14, 165, 233, 0.1); }

/* CLI Toggles */
.cli-toggles { display: flex; flex-direction: column; gap: 12px; background: #f8fafc; padding: 16px; border-radius: 12px; }
.toggle-item { display: flex; justify-content: space-between; align-items: center; }
.toggle-label { font-size: 13px; font-weight: 500; color: #475569; }

/* Repo Grid (Available) */
.repo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }
.repo-card { 
  background: #ffffff; border-radius: 16px; border: 1px solid #f1f5f9; padding: 20px; 
  cursor: pointer; position: relative; transition: all 0.2s; display: flex; align-items: center; gap: 16px; 
}
.repo-card:hover { border-color: #0ea5e9; background: #f8fafc; }

.repo-icon-box { 
  width: 40px; height: 40px; border-radius: 10px; background: #f1f5f9; color: #64748b; 
  display: flex; align-items: center; justify-content: center; 
}
.repo-info-main { flex: 1; min-width: 0; }
.repo-name-title { font-weight: 700; font-size: 15px; color: #0f172a; margin-bottom: 4px; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.repo-source-subtitle { font-size: 12px; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.repo-actions-overlay { display: flex; gap: 4px; flex-shrink: 0; }

/* Discover List */
.discover-list { background: #ffffff; border-radius: 16px; overflow: hidden; border: 1px solid #f1f5f9; }
.discover-item { 
  display: flex; justify-content: space-between; align-items: center; padding: 20px 24px; 
  border-bottom: 1px solid #f1f5f9; transition: background 0.2s; 
}
.discover-item:last-child { border-bottom: none; }
.discover-item:hover { background: #f8fafc; }
.discover-info { flex: 1; min-width: 0; padding-right: 40px; }
.discover-name-row { margin-bottom: 6px; display: flex; align-items: center; gap: 8px; }
.discover-name { font-weight: 700; font-size: 15px; color: #0f172a; }
.discover-desc { 
  font-size: 13px; color: #64748b; line-height: 1.5; cursor: pointer; 
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-overflow: ellipsis;
}
.discover-actions { flex-shrink: 0; display: flex; gap: 4px; }

/* Favorites */
.favorite-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }
.fav-card { background: white; border-radius: 16px; border: 1px solid #f1f5f9; padding: 20px; }
.fav-main { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
.fav-info { min-width: 0; flex: 1; }
.fav-name {
  font-weight: 700; font-size: 16px; color: #0f172a; margin-bottom: 4px;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  overflow: hidden; text-overflow: ellipsis;
}.fav-market { 
  font-size: 12px; color: #94a3b8; 
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; 
}
.fav-actions { flex-shrink: 0; display: flex; gap: 4px; }

/* Shared styles */
.mono { font-family: "JetBrains Mono", monospace; }
.search-box { position: relative; }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: #94a3b8; }
.c-input {
  width: 100%; padding: 10px 14px; background: #ffffff; border: 1px solid #e2e8f0;
  border-radius: 8px; font-size: 14px; color: #0f172a; outline: none; transition: all 0.2s;
}
.c-input:focus { border-color: #0ea5e9; }

.b-button {
  background: #0ea5e9; color: #ffffff; border: none; padding: 8px 16px; border-radius: 8px;
  font-size: 14px; font-weight: 500; cursor: pointer; display: flex; align-items: center;
  transition: all 0.2s; white-space: nowrap;
}
.b-button:hover { background: #0284c7; }
.b-button:disabled { background: #94a3b8; cursor: not-allowed; }

.b-button-outline {
  background: #ffffff; color: #0f172a; border: 1px solid #e2e8f0; padding: 8px 16px; border-radius: 8px;
  font-size: 14px; font-weight: 500; cursor: pointer; transition: all 0.2s; display: flex; align-items: center;
}
.b-button-outline:hover { background: #f8fafc; border-color: #cbd5e1; }

.empty-state { padding: 80px 40px; text-align: center; color: #94a3b8; background: #ffffff; border-radius: 24px; border: 2px dashed #e2e8f0; }
.empty-state p { margin-top: 16px; font-size: 15px; }

.form-group { margin-bottom: 24px; }
.c-label { display: block; font-size: 13px; font-weight: 500; color: #475569; margin-bottom: 12px; }
.required { color: #f43f5e; }
</style>
