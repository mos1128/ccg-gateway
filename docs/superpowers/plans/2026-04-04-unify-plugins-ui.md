# Plugin Management UI Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify the Plugin Management page (`frontend/src/views/plugins/index.vue`) UI and interaction logic to completely match the Skill Management page, while preserving the existing plugin backend business logic.

**Architecture:** 
1. **Tabs Reorganization**: Adopt a 3-tab layout: "插件" (Installed Plugins), "市场" (Marketplaces), and "收藏" (Favorites).
2. **State Drilling**: The Marketplaces tab will implement a two-level drill-down (Market List -> Market Plugins) purely via frontend filtering of the existing `allPlugins` array using the `marketplace_name` property.
3. **Card UI Standardization**: Plugin cards will adopt the Top-Down structure of Skill cards (Icon, Info, Top-Right Actions) and a bottom `cli-toggles` area repurposed for the "Enable/Disable" plugin toggle switch.
4. **CSS Consistency**: Import and adapt the CSS classes from `skills/index.vue` (`skill-grid`, `card-top`, `discover-list`, etc.) to guarantee an identical visual experience.

**Tech Stack:** Vue 3 (Composition API), Element Plus, TypeScript

---

### Task 1: Update State Variables and Computed Properties

**Files:**
- Modify: `frontend/src/views/plugins/index.vue` (Script block)

- [ ] **Step 1: Add new reactive state variables**
  Add state for the current market selection and loading indicators to mimic skills.
  ```typescript
  const currentMarket = ref<MarketplaceInfo | null>(null)
  const operationLoading = ref(false)
  const installingPluginId = ref<string | null>(null)
  ```

- [ ] **Step 2: Update computed properties for the new tab logic**
  Replace `sortedPlugins` and `filteredPlugins` with new computed logic tailored to the separate tabs.
  ```typescript
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
  ```

- [ ] **Step 3: Add interaction methods for Marketplaces**
  ```typescript
  function handleMarketClick(market: MarketplaceInfo) {
    currentMarket.value = market
    pluginSearchQuery.value = ''
  }

  function handleBackToMarkets() {
    currentMarket.value = null
    pluginSearchQuery.value = ''
  }
  ```

- [ ] **Step 4: Update Plugin Action Methods to use `operationLoading`**
  Modify `handleEnable`, `handleDisable`, `handleUpdate`, `handleUninstall`, and `handleInstall` to use `operationLoading.value = true/false` and `installingPluginId.value = pluginId / null` around the API calls. For toggling enable/disable, create a single method `handleToggleEnable(plugin, enabled)`.

  ```typescript
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
  ```

### Task 2: Rebuild Tab 1 - Installed Plugins UI

**Files:**
- Modify: `frontend/src/views/plugins/index.vue` (Template & Style blocks)

- [ ] **Step 1: Replace Tab 1 Template Structure**
  Replace the `<div v-if="activeTab === 'plugins'">` block with the `skill-grid` and `skill-card` structure from `skills/index.vue`, but iterate over `installedPlugins`.

  ```vue
  <div v-if="activeTab === 'plugins'" class="tab-pane">
    <div v-loading="loading || operationLoading" class="list-container">
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
                  <svg width="18" height="18"><use href="#icon-star"/></svg>
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
                <span class="toggle-label">启用状态</span>
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
  ```

- [ ] **Step 2: Add requisite CSS classes**
  Ensure CSS classes like `.skill-grid`, `.skill-card`, `.card-top`, `.skill-icon`, `.skill-info`, `.skill-name`, `.skill-market`, `.card-actions`, `.action-icon`, `.cli-toggles`, `.toggle-item`, `.toggle-label` are present in the `<style scoped>` section (copied exactly from `skills/index.vue`). Remove old `.plugin-card`, `.plugin-grid` CSS if no longer used.

### Task 3: Rebuild Tab 2 - Marketplaces (Drill-down UI)

**Files:**
- Modify: `frontend/src/views/plugins/index.vue` (Template block)

- [ ] **Step 1: Replace Marketplaces Tab Template**
  Replace the `<div v-else-if="activeTab === 'marketplaces'">` block with the two-level structure (Market List vs Market Plugins) mirroring `repos` tab from skills.

  ```vue
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
  ```

- [ ] **Step 2: Add requisite CSS classes and Icons**
  Ensure CSS classes `.repo-list-view`, `.repo-grid`, `.repo-card`, `.repo-icon-box`, `.repo-info-main`, `.repo-name-title`, `.repo-source-subtitle`, `.repo-actions-overlay`, `.repo-skills-view`, `.discover-list`, `.discover-item`, `.discover-info`, `.discover-name-row`, `.discover-name`, `.discover-desc`, `.discover-actions` are present in `<style scoped>`. Ensure `<svg>` definition for `#icon-back` exists at the top.

### Task 4: Rebuild Tab 3 - Favorites UI

**Files:**
- Modify: `frontend/src/views/plugins/index.vue` (Template & Style blocks)

- [ ] **Step 1: Replace Favorites Tab Template**
  Update the Favorites tab content to match the clean style of `skills/index.vue`.

  ```vue
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
                <div class="fav-market">来自市场: {{ fav.marketplace_name }}</div>
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
  ```

- [ ] **Step 2: Add requisite CSS classes**
  Ensure CSS classes `.favorite-grid`, `.fav-card`, `.fav-main`, `.fav-info`, `.fav-name`, `.fav-market`, `.fav-actions` are present and match `skills/index.vue`. Remove old unused favorite styles.

### Task 5: Final Review & Cleanup

- [ ] **Step 1: Check SVG Definitions**
  Copy `<symbol id="icon-back">` and `<symbol id="icon-edit">` (if needed) from `skills/index.vue` to `plugins/index.vue`'s `<defs>`.

- [ ] **Step 2: Clean up unused code**
  Remove unused functions, variables (e.g., `sortedPlugins`, `filteredPlugins` if they are fully replaced by the new computed properties) and unused old CSS (`.plugin-card`, `.market-card`, etc.).
