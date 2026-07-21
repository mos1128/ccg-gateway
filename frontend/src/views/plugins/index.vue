<template>
  <div class="pg-page">
    <div class="v2-tabs pg-tabs">
      <div class="v2-tab" :class="{ active: activeTab === 'plugins' }" @click="activeTab = 'plugins'">插件</div>
      <div class="v2-tab" :class="{ active: activeTab === 'marketplaces' }" @click="activeTab = 'marketplaces'">市场</div>
      <div class="v2-tab" :class="{ active: activeTab === 'favorites' }" @click="activeTab = 'favorites'">收藏</div>
    </div>

    <!-- 插件 -->
    <template v-if="activeTab === 'plugins'">
      <div v-loading="loadingInstalled || loadingInstalledOperation" class="pg-body">
        <V2Empty class="v2-card" v-if="installedPlugins.length === 0" title="还没有已安装插件" description="到「市场」里安装插件，安装后可在此启用/停用">
          <template #icon><PluginIcon :size="40" :stroke-width="1.6" /></template>
        </V2Empty>
        <div v-else class="v2-cardgrid">
          <div v-for="plugin in installedPlugins" :key="getPluginId(plugin)" class="v2-card v2-ccard">
            <div class="v2-ccard-head">
              <div class="v2-ccard-icon"><PluginIcon /></div>
              <div class="v2-ccard-tt">
                <div class="v2-ccard-name">{{ plugin.name }}</div>
                <div class="v2-ccard-sub mono">{{ plugin.marketplace_name || '—' }}</div>
              </div>
              <div class="v2-ccard-acts">
                <el-tooltip :content="favoriteIds.has(getPluginId(plugin)) ? '取消收藏' : '收藏'" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ 'pg-star-on': favoriteIds.has(getPluginId(plugin)) }" @click="handleInstalledPluginFavorite(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" :fill="favoriteIds.has(getPluginId(plugin)) ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg></button>
                </el-tooltip>
                <el-tooltip content="更新" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" @click="handleUpdate(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
                </el-tooltip>
                <el-tooltip content="卸载" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act danger" @click="handleUninstall(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                </el-tooltip>
              </div>
            </div>
            <V2CliChips :flags="{ claude_code: !!plugin.is_enabled }" feature="plugins" @toggle="(_, e) => handleToggleEnable(plugin, e)" />
          </div>
        </div>
      </div>
    </template>

    <!-- 市场 -->
    <template v-else-if="activeTab === 'marketplaces'">
      <template v-if="!currentMarket">
        <div v-loading="loadingMarketplaces" class="pg-body">
          <div class="v2-cardgrid">
            <div class="v2-addcard v2-addcard-row" @click="showAddMarketDialog = true">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></svg>
              <span>添加市场</span>
            </div>
            <div v-for="market in marketplaceList" :key="market.name" class="v2-card v2-ccard pg-mcard" @click="handleMarketClick(market)">
              <div class="v2-ccard-head">
                <div class="v2-ccard-icon"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9 12 3l9 6v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><polyline points="9 22 9 12 15 12 15 22"/></svg></div>
                <div class="v2-ccard-tt"><div class="v2-ccard-name">{{ market.name }}</div><div class="v2-ccard-sub mono">{{ market.marketplace_source || '内建市场' }}</div></div>
                <div class="v2-ccard-acts">
                  <el-tooltip content="更新" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act" @click.stop="handleUpdateMarketplace(market)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
                  </el-tooltip>
                  <el-tooltip content="删除" placement="top" effect="light" :show-after="250">
                    <button class="v2-row-act danger" @click.stop="handleRemoveMarketplace(market)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                  </el-tooltip>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>
      <template v-else>
        <div class="pg-mhead">
          <div class="pg-mhead-l">
            <el-tooltip content="返回" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act" @click="handleBackToMarkets"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg></button>
            </el-tooltip>
            <div><div class="pg-mhead-t">{{ currentMarket.name }}</div><div class="v2-hint mono">{{ currentMarket.marketplace_source || '内建市场' }}</div></div>
          </div>
          <div class="pg-mhead-r">
            <input v-model="pluginSearchQuery" class="v2-input v2-input-surface pg-search" placeholder="搜索…">
            <el-tooltip content="刷新" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act" :disabled="loadingMarketPlugins" @click="handleUpdateMarketplace(currentMarket)"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
            </el-tooltip>
          </div>
        </div>
        <div v-loading="loadingMarketPlugins || loadingMarketPluginsOperation" class="pg-body">
          <V2Empty class="v2-card" v-if="filteredMarketPlugins.length === 0" :title="pluginSearchQuery ? '无匹配结果' : '该市场暂无插件'" />
          <div v-else class="v2-card pg-dlist">
            <div v-for="plugin in filteredMarketPlugins" :key="getPluginId(plugin)" class="pg-drow">
              <div class="pg-drow-icon">
                <PluginIcon :size="15" />
              </div>
              <div class="pg-dinfo">
                <div class="pg-dname">
                  {{ plugin.name }}
                  <span v-if="plugin.version && plugin.version !== 'unknown'" class="v2-pill v2-pill-neutral mono sk-inline">{{ plugin.version }}</span>
                </div>
                <div v-if="plugin.description" class="pg-ddesc" @click="copyDescription(plugin.description)">{{ plugin.description }}</div>
              </div>
              <button class="v2-btn v2-btn-sm pg-install" :class="plugin.is_installed ? 'v2-btn-outline' : 'v2-btn-primary'" :disabled="installingPluginId === getPluginId(plugin)" @click="plugin.is_installed ? handleUpdate(plugin) : handleInstall(plugin)">{{ installingPluginId === getPluginId(plugin) ? '处理中…' : (plugin.is_installed ? '更新' : '安装') }}</button>
            </div>
          </div>
        </div>
      </template>
    </template>

    <!-- 收藏 -->
    <template v-else>
      <div v-loading="loadingFavorites || loadingFavoritesOperation" class="pg-body">
        <V2Empty class="v2-card" v-if="favoriteList.length === 0" title="还没有收藏插件" description="收藏会保留来源市场，方便随时一键安装">
          <template #icon><svg width="40" height="40" viewBox="0 0 24 24"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg></template>
        </V2Empty>
        <div v-else class="v2-cardgrid">
          <div v-for="fav in favoriteList" :key="fav.plugin_id" class="v2-card v2-ccard">
            <div class="v2-ccard-head">
              <div class="v2-ccard-icon"><PluginIcon /></div>
              <div class="v2-ccard-tt">
                <div class="v2-ccard-name">{{ fav.plugin_name }}</div>
                <div class="v2-ccard-sub mono">{{ fav.marketplace_name }}</div>
              </div>
              <div class="v2-ccard-acts">
                <el-tooltip :content="fav.is_installed ? '更新' : '安装'" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" :class="{ off: installingPluginId === fav.plugin_id }" @click="handleInstallFavorite(fav)">
                    <svg v-if="fav.is_installed" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg>
                    <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                  </button>
                </el-tooltip>
                <el-tooltip content="移除收藏" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act danger" @click="handleRemoveFavoriteById(fav)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
                </el-tooltip>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <V2Drawer v-model="showAddMarketDialog" title="添加插件市场" @confirm="handleAddMarketplace">
      <div class="v2-field"><label class="v2-label">市场地址 <span class="req">*</span></label><input v-model="marketForm.url" class="v2-input mono" placeholder="owner/repo 或本地路径"></div>
      <div class="v2-hint">支持 GitHub 仓库 URL 或本地目录路径。</div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import { ElNotification } from 'element-plus'
import V2Drawer from '@/components/V2Drawer.vue'
import V2Empty from '@/components/V2Empty.vue'
import V2CliChips from '@/components/V2CliChips.vue'
import PluginIcon from '@/components/PluginIcon.vue'
import { confirm } from '@/utils/confirm'
import { pluginsApi } from '@/api/plugins'
import { getErrorMessage } from '@/utils/error'
import type { MarketplaceInfo, PluginItem, PluginFavoriteItem } from '@/types/models'

const activeTab = ref('plugins')

const installedPlugins = ref<PluginItem[]>([])
const loadingInstalled = ref(false)
const loadingInstalledOperation = ref(false)

const marketplaceList = ref<MarketplaceInfo[]>([])
const loadingMarketplaces = ref(false)
const currentMarket = ref<MarketplaceInfo | null>(null)
const marketPlugins = ref<PluginItem[]>([])
const loadingMarketPlugins = ref(false)
const loadingMarketPluginsOperation = ref(false)
const pluginSearchQuery = ref('')
const showAddMarketDialog = ref(false)
const marketForm = ref({ url: '' })

const favoriteList = ref<PluginFavoriteItem[]>([])
const loadingFavorites = ref(false)
const loadingFavoritesOperation = ref(false)

const installingPluginId = ref<string | null>(null)

const favoriteIds = computed(() => new Set(favoriteList.value.map(f => f.plugin_id)))
const filteredMarketPlugins = computed(() => {
  if (!pluginSearchQuery.value) return marketPlugins.value
  const q = pluginSearchQuery.value.toLowerCase()
  return marketPlugins.value.filter(p => p.name.toLowerCase().includes(q) || p.description?.toLowerCase().includes(q))
})

function getPluginId(plugin: PluginItem): string {
  return plugin.marketplace_name ? `${plugin.name}@${plugin.marketplace_name}` : plugin.name
}
function showCliOutput(output: string, isError = false) {
  if (!output) return
  ElNotification({ title: isError ? '操作失败' : '操作结果', message: output.replace(/\n/g, '<br/>'), type: isError ? 'error' : 'success', duration: 5000, position: 'top-right', dangerouslyUseHTMLString: true })
}
function notify(message: string, type: 'success' | 'error' | 'warning' | 'info' = 'success') {
  ElNotification({ title: type === 'success' ? '成功' : type === 'error' ? '错误' : '提示', message, type, duration: 3000, position: 'top-right' })
}

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
async function handleToggleEnable(plugin: PluginItem, enabled: boolean) {
  loadingInstalledOperation.value = true
  const pluginId = getPluginId(plugin)
  try {
    const result = enabled ? await pluginsApi.enable(pluginId) : await pluginsApi.disable(pluginId)
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
    if (error !== 'cancel' && error?.toString() !== 'cancel') showCliOutput(getErrorMessage(error, '卸载失败'), true)
  } finally {
    loadingInstalledOperation.value = false
  }
}
async function handleUpdate(plugin: PluginItem) {
  const pluginId = getPluginId(plugin)
  const inMarketDetail = currentMarket.value !== null
  if (inMarketDetail) loadingMarketPluginsOperation.value = true
  else loadingInstalledOperation.value = true
  installingPluginId.value = pluginId
  try {
    const result = await pluginsApi.update(pluginId)
    showCliOutput(result.cli_output)
    await fetchInstalled()
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '更新失败'), true)
  } finally {
    if (inMarketDetail) loadingMarketPluginsOperation.value = false
    else loadingInstalledOperation.value = false
    installingPluginId.value = null
  }
}
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
  if (favoriteIds.value.has(getPluginId(plugin))) await handleRemoveFavorite(plugin)
  else await handleAddFavorite(plugin)
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
    if (favorite.is_installed) result = await pluginsApi.update(favorite.plugin_id)
    else result = await pluginsApi.installFavorite(favorite.plugin_id, favorite.marketplace_name, favorite.marketplace_source ?? undefined)
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
    if (currentMarket.value?.name === market.name) handleBackToMarkets()
    await Promise.all([fetchMarketplaces(), fetchInstalled(), fetchFavorites()])
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') showCliOutput(getErrorMessage(error, '删除失败'), true)
  } finally {
    loadingMarketplaces.value = false
  }
}
async function handleUpdateMarketplace(market: MarketplaceInfo) {
  const inMarketDetail = currentMarket.value?.name === market.name
  if (inMarketDetail) loadingMarketPlugins.value = true
  else loadingMarketplaces.value = true
  try {
    const result = await pluginsApi.updateMarketplace(market.name)
    showCliOutput(result.cli_output)
    await fetchMarketplaces()
    if (currentMarket.value) await fetchMarketplacePlugins()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '更新失败'), true)
  } finally {
    if (inMarketDetail) loadingMarketPlugins.value = false
    else loadingMarketplaces.value = false
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

onMounted(() => {
  fetchInstalled()
  fetchMarketplaces()
  fetchFavorites()
})
</script>

<style scoped>
.pg-page { height: 100%; display: flex; flex-direction: column; min-height: 0; margin-top: -16px; }
.pg-tabs { margin-bottom: 16px; flex-shrink: 0; }
.pg-toolbar { display: flex; justify-content: flex-end; margin-bottom: 14px; flex-shrink: 0; }
.pg-body { flex: 1; overflow-y: auto; min-height: 0; }
.pg-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 9px; padding: 60px; color: var(--v2-text-3); }
.pg-empty-t { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); }
.sk-inline { margin-left: 8px; }
.pg-star-on { color: var(--v2-warning); }

.pg-mcard { cursor: pointer; transition: border-color 0.15s; }
.pg-mcard:hover { border-color: var(--v2-surface-3); }
.pg-mhead { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 14px; flex-shrink: 0; }
.pg-mhead-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
.pg-mhead-t { font-size: var(--v2-fs-md); font-weight: var(--v2-fw-medium); color: var(--v2-text); }
.pg-mhead-r { display: flex; align-items: center; gap: 8px; }
.pg-search { width: 220px; }

.pg-dlist { padding: 4px 0; }
.pg-drow { display: flex; align-items: center; gap: 16px; padding: 13px 16px; border-bottom: 1px solid var(--v2-surface-2); }
.pg-drow:last-child { border-bottom: none; }
.pg-drow-icon { width: 32px; height: 32px; border-radius: 6px; background: var(--v2-surface-2); color: var(--v2-text-2); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.pg-dinfo { flex: 1; min-width: 0; }
.pg-dname { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); color: var(--v2-text); display: flex; align-items: center; }
.pg-ddesc { font-size: var(--v2-fs-sm); color: var(--v2-text-3); margin-top: 3px; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pg-install { flex-shrink: 0; }
</style>
