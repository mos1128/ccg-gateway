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
    <div class="view-content-wrapper">
      
      <!-- TAB: INSTALLED PLUGINS -->
      <div v-if="activeTab === 'plugins'" class="tab-pane">
        <div class="plugins-view">
          <div v-loading="loadingInstalled || loadingInstalledOperation" class="list-container">
          <template v-if="installedPlugins.length === 0">
            <div class="empty-state">
              <svg width="64" height="64" class="empty-icon"><use href="#icon-puzzle"/></svg>
              <p>暂无已安装插件</p>
            </div>
          </template>
          <div v-else class="scroll-area">
            <div class="skill-grid">
              <InstalledPluginCard
                v-for="plugin in installedPlugins"
                :key="getPluginId(plugin)"
                :plugin="plugin"
                :is-favorite="favoriteIds.has(getPluginId(plugin))"
                :operation-disabled="loadingInstalledOperation"
                @favorite="handleInstalledPluginFavorite"
                @update="handleUpdate"
                @uninstall="handleUninstall"
                @toggle-enable="handleToggleEnable"
              />
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
            <button class="action-icon primary" @click="showAddMarketDialog = true" title="添加市场">
              <svg width="20" height="20"><use href="#icon-plus"/></svg>
            </button>
          </div>

          <div v-loading="loadingMarketplaces" class="list-container">
            <div v-if="marketplaceList.length === 0" class="empty-state">
               <svg width="64" height="64" class="empty-icon"><use href="#icon-store"/></svg>
               <p>暂无插件市场</p>
            </div>
            <div v-else class="scroll-area">
              <div class="repo-grid">
                <MarketplaceCard
                  v-for="market in marketplaceList"
                  :key="market.name"
                  :marketplace="market"
                  @open="handleMarketClick"
                  @update="handleUpdateMarketplace"
                  @remove="handleRemoveMarketplace"
                />
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
                <h2 class="page-title text-20">{{ currentMarket.name }}</h2>
                <div class="mono text-14 text-muted">{{ currentMarket.marketplace_source || '内建市场' }}</div>
              </div>
            </div>
            <div style="display: flex; gap: 12px; align-items: center;">
              <div class="search-box" style="width: 240px; position: relative;">
                <svg class="search-icon" width="16" height="16" style="position: absolute; left: 12px; top: 50%; transform: translateY(-50%); pointer-events: none; z-index: 1;"><use href="#icon-search"/></svg>
                <input type="text" v-model="pluginSearchQuery" class="b-input search-input" placeholder="搜索...">
              </div>
              <button class="action-icon" :disabled="loadingMarketPlugins" @click="handleUpdateMarketplace(currentMarket)" title="刷新市场">
                <svg width="18" height="18"><use href="#icon-refresh"/></svg>
              </button>
            </div>
          </div>

          <div v-loading="loadingMarketPlugins || loadingMarketPluginsOperation" class="list-container">
            <template v-if="filteredMarketPlugins.length === 0">
              <div class="empty-state">
                <svg width="64" height="64" color="var(--color-border)"><use href="#icon-puzzle"/></svg>
                <p>{{ pluginSearchQuery ? '无匹配结果' : '该市场暂无插件' }}</p>
              </div>
            </template>
            <div v-else class="scroll-area">
              <div class="discover-list">
                <MarketplacePluginItem
                  v-for="plugin in filteredMarketPlugins"
                  :key="getPluginId(plugin)"
                  :plugin="plugin"
                  :installing="installingPluginId === getPluginId(plugin)"
                  @update="handleUpdate"
                  @install="handleInstall"
                  @copy-description="copyDescription"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB: FAVORITES -->
      <div v-else class="tab-pane">
        <div class="favorites-view">
          <div class="page-header">
            <p class="page-subtitle">收藏的插件会保留市场信息，方便后续快速安装</p>
          </div>

          <div v-loading="loadingFavorites || loadingFavoritesOperation" class="list-container">
          <div v-if="favoriteList.length === 0" class="empty-state">
            <svg width="64" height="64" class="empty-icon"><use href="#icon-star"/></svg>
            <p>暂无收藏插件</p>
          </div>
          <div v-else class="scroll-area">
            <div class="favorite-grid">
              <PluginFavoriteCard
                v-for="fav in favoriteList"
                :key="fav.plugin_id"
                :favorite="fav"
                :installing="installingPluginId === fav.plugin_id"
                @remove="handleRemoveFavoriteById"
                @install="handleInstallFavorite"
              />
            </div>
          </div>
        </div>
        </div>
      </div>

    </div>

    <AddMarketplaceModal
      v-model="showAddMarketDialog"
      v-model:url="marketForm.url"
      @confirm="handleAddMarketplace"
    />

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElNotification } from 'element-plus'
import { confirm } from '@/utils/confirm'
import InstalledPluginCard from './components/InstalledPluginCard.vue'
import MarketplaceCard from './components/MarketplaceCard.vue'
import MarketplacePluginItem from './components/MarketplacePluginItem.vue'
import PluginFavoriteCard from './components/PluginFavoriteCard.vue'
import AddMarketplaceModal from './components/AddMarketplaceModal.vue'
import { pluginsApi } from '@/api/plugins'
import { getErrorMessage } from '@/utils/error'
import type { MarketplaceInfo, PluginItem, PluginFavoriteItem } from '@/types/models'

const activeTab = ref('plugins')

// Installed plugins (Tab 1)
const installedPlugins = ref<PluginItem[]>([])
const loadingInstalled = ref(false)
const loadingInstalledOperation = ref(false)

// Marketplaces (Tab 2)
const marketplaceList = ref<MarketplaceInfo[]>([])
const loadingMarketplaces = ref(false)
const currentMarket = ref<MarketplaceInfo | null>(null)
const marketPlugins = ref<PluginItem[]>([])
const loadingMarketPlugins = ref(false)
const loadingMarketPluginsOperation = ref(false)
const pluginSearchQuery = ref('')
const showAddMarketDialog = ref(false)
const marketForm = ref({ url: '' })

// Favorites (Tab 3)
const favoriteList = ref<PluginFavoriteItem[]>([])
const loadingFavorites = ref(false)
const loadingFavoritesOperation = ref(false)

// Operation state
const installingPluginId = ref<string | null>(null)

// Computed
const favoriteIds = computed(() => new Set(favoriteList.value.map(f => f.plugin_id)))

const filteredMarketPlugins = computed(() => {
  if (!pluginSearchQuery.value) return marketPlugins.value
  const q = pluginSearchQuery.value.toLowerCase()
  return marketPlugins.value.filter(p =>
    p.name.toLowerCase().includes(q) ||
    p.description?.toLowerCase().includes(q)
  )
})

// Utils
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

function notify(message: string, type: 'success' | 'error' | 'warning' | 'info' = 'success') {
  ElNotification({
    title: type === 'success' ? '成功' : type === 'error' ? '错误' : '提示',
    message,
    type,
    duration: 3000,
    position: 'top-right'
  })
}

// --- Fetch functions ---

async function fetchInstalled() {
  loadingInstalled.value = true
  try {
    installedPlugins.value = await pluginsApi.getInstalled()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingInstalled.value = false
  }
}

async function fetchMarketplaces() {
  loadingMarketplaces.value = true
  try {
    marketplaceList.value = await pluginsApi.getMarketplaces()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingMarketplaces.value = false
  }
}

async function fetchMarketplacePlugins() {
  if (!currentMarket.value) return
  loadingMarketPlugins.value = true
  try {
    marketPlugins.value = await pluginsApi.getMarketplacePlugins(currentMarket.value.name)
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingMarketPlugins.value = false
  }
}

async function fetchFavorites() {
  loadingFavorites.value = true
  try {
    favoriteList.value = await pluginsApi.getFavorites()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingFavorites.value = false
  }
}

// --- Marketplace navigation ---

function handleMarketClick(market: MarketplaceInfo) {
  currentMarket.value = market
  pluginSearchQuery.value = ''
  fetchMarketplacePlugins()
}

function handleBackToMarkets() {
  currentMarket.value = null
  marketPlugins.value = []
  pluginSearchQuery.value = ''
}

// --- Plugin actions ---

async function handleToggleEnable(plugin: PluginItem, enabled: boolean) {
  loadingInstalledOperation.value = true
  const pluginId = getPluginId(plugin)
  try {
    const result = enabled
      ? await pluginsApi.enable(pluginId)
      : await pluginsApi.disable(pluginId)
    showCliOutput(result.cli_output)
    await fetchInstalled()
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '操作失败'), true)
  } finally {
    loadingInstalledOperation.value = false
  }
}

async function handleInstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  loadingMarketPluginsOperation.value = true
  installingPluginId.value = pluginId
  try {
    const result = await pluginsApi.install(pluginId)
    showCliOutput(result.cli_output)
    await Promise.all([fetchInstalled(), fetchFavorites()])
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '安装失败'), true)
  } finally {
    loadingMarketPluginsOperation.value = false
    installingPluginId.value = null
  }
}

async function handleUninstall(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  try {
    await confirm(`确定卸载插件 "${plugin.name}"?`, '确认卸载')
    loadingInstalledOperation.value = true
    const result = await pluginsApi.uninstall(pluginId)
    showCliOutput(result.cli_output)
    await Promise.all([fetchInstalled(), fetchFavorites()])
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      showCliOutput(getErrorMessage(error, '卸载失败'), true)
    }
  } finally {
    loadingInstalledOperation.value = false
  }
}

async function handleUpdate(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  const inMarketDetail = currentMarket.value !== null
  if (inMarketDetail) {
    loadingMarketPluginsOperation.value = true
  } else {
    loadingInstalledOperation.value = true
  }
  installingPluginId.value = pluginId
  try {
    const result = await pluginsApi.update(pluginId)
    showCliOutput(result.cli_output)
    await fetchInstalled()
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '更新失败'), true)
  } finally {
    if (inMarketDetail) {
      loadingMarketPluginsOperation.value = false
    } else {
      loadingInstalledOperation.value = false
    }
    installingPluginId.value = null
  }
}

// --- Favorite actions ---

async function handleAddFavorite(plugin: PluginItem) {
  loadingInstalledOperation.value = true
  const pluginId = getPluginId(plugin)
  try {
    await pluginsApi.addFavorite(pluginId, plugin.name, plugin.marketplace_name)
    await fetchFavorites()
    notify('已收藏')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    loadingInstalledOperation.value = false
  }
}

async function handleInstalledPluginFavorite(plugin: PluginItem) {
  if (favoriteIds.value.has(getPluginId(plugin))) {
    await handleRemoveFavorite(plugin)
  } else {
    await handleAddFavorite(plugin)
  }
}

async function handleRemoveFavorite(plugin: PluginItem) {
  loadingInstalledOperation.value = true
  const pluginId = getPluginId(plugin)
  try {
    await pluginsApi.removeFavorite(pluginId)
    await fetchFavorites()
    notify('已取消收藏')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    loadingInstalledOperation.value = false
  }
}

async function handleInstallFavorite(favorite: PluginFavoriteItem) {
  loadingFavoritesOperation.value = true
  installingPluginId.value = favorite.plugin_id
  try {
    let result: { cli_output: string }
    if (favorite.is_installed) {
      result = await pluginsApi.update(favorite.plugin_id)
    } else {
      result = await pluginsApi.installFavorite(
        favorite.plugin_id,
        favorite.marketplace_name,
        favorite.marketplace_source ?? undefined
      )
    }
    showCliOutput(result.cli_output)
    await Promise.all([fetchInstalled(), fetchFavorites(), fetchMarketplaces()])
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, favorite.is_installed ? '更新失败' : '安装失败'), true)
  } finally {
    loadingFavoritesOperation.value = false
    installingPluginId.value = null
  }
}

async function handleRemoveFavoriteById(favorite: PluginFavoriteItem) {
  loadingFavoritesOperation.value = true
  try {
    await pluginsApi.removeFavorite(favorite.plugin_id)
    await fetchFavorites()
    notify('已移除')
  } catch (error: any) {
    notify(getErrorMessage(error, '操作失败'), 'error')
  } finally {
    loadingFavoritesOperation.value = false
  }
}

// --- Marketplace actions ---

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
    showCliOutput(result.cli_output)
    await fetchMarketplaces()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '添加失败'), true)
  } finally {
    loadingMarketplaces.value = false
  }
}

async function handleRemoveMarketplace(market: MarketplaceInfo) {
  try {
    await confirm(`确定删除市场 "${market.name}"?`, '确认删除')
    loadingMarketplaces.value = true
    const result = await pluginsApi.removeMarketplace(market.name)
    showCliOutput(result.cli_output)
    if (currentMarket.value?.name === market.name) {
      handleBackToMarkets()
    }
    // CLI 删除市场时会自动卸载其中的插件
    await Promise.all([fetchMarketplaces(), fetchInstalled(), fetchFavorites()])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') {
      showCliOutput(getErrorMessage(error, '删除失败'), true)
    }
  } finally {
    loadingMarketplaces.value = false
  }
}

async function handleUpdateMarketplace(market: MarketplaceInfo) {
  const inMarketDetail = currentMarket.value?.name === market.name
  if (inMarketDetail) {
    loadingMarketPlugins.value = true
  } else {
    loadingMarketplaces.value = true
  }
  try {
    const result = await pluginsApi.updateMarketplace(market.name)
    showCliOutput(result.cli_output)
    await fetchMarketplaces()
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '更新失败'), true)
  } finally {
    if (inMarketDetail) {
      loadingMarketPlugins.value = false
    } else {
      loadingMarketplaces.value = false
    }
  }
}

// --- Misc ---

async function copyDescription(text: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    notify('描述已复制')
  } catch {
    notify('复制失败', 'error')
  }
}

onMounted(() => {
  fetchInstalled()
  fetchMarketplaces()
  fetchFavorites()
})
</script>

<style scoped>
.plugins-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid var(--color-border); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }

.view-content-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tab-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.repo-list-view, .repo-skills-view, .favorites-view, .plugins-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* Header */
.page-title.text-20 { margin: 0; }

.skill-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 24px; }

.repo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }

.discover-list { background: var(--color-bg); border-radius: 16px; overflow: hidden; border: 1px solid var(--color-bg-subtle); }

.favorite-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); gap: 20px; }

.search-box { position: relative; }
.search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--color-text-weak); }


.empty-icon { color: var(--color-border); }

</style>
