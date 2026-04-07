# Plugin Cache Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the Plugin management system to use an on-demand, lazy loading architecture instead of a monolithic file cache.

**Architecture:** We will delete the `PluginsCache` structure and `plugins_cache.json` backend logic. We will replace `get_all_plugins` with `get_installed_plugins` and `get_marketplace_plugins`. The frontend will be updated to call these specific endpoints only when the corresponding tab or view is active.

**Tech Stack:** Rust (Tauri Backend), TypeScript (Vue 3 Frontend)

---

### Task 1: Clean up Backend Cache and Core Logic

**Files:**
- Modify: `src-tauri/src/services/plugin.rs`
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Remove cache structures from models**

Remove `PluginActionResult` and `MarketplaceActionResult` (if they exist) as we won't be returning the global cache anymore. We will redefine them or use `Vec<PluginItem>` directly. Let's just remove the `plugins` field from `PluginActionResult` and `MarketplaceActionResult` in `models.rs` or `plugin.rs`. Wait, let's look at `plugin.rs` where `PluginActionResult` is defined.

```rust
// Remove from src-tauri/src/services/plugin.rs
/*
pub struct PluginActionResult {
    pub cli_output: String,
    pub plugins: Vec<PluginItem>,
}
*/
```
Actually, we'll redefine `PluginActionResult` to just contain the `cli_output`, as the frontend will re-fetch data.

```rust
// Modify in src-tauri/src/services/plugin.rs
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PluginActionResult {
    pub cli_output: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MarketplaceActionResult {
    pub cli_output: String,
}
```

- [ ] **Step 2: Delete Cache Functions**

Delete the following from `src-tauri/src/services/plugin.rs`:
- `CACHE_LOCK`
- `PluginsCache`
- `get_cache_path`
- `write_cache`
- `read_cache`
- `generate_cache`
- `update_installed_status`
- `update_favorite_status`
- `get_plugins` (the old global one)
- `refresh_plugins`
- `refresh_cache`

- [ ] **Step 3: Create `get_installed_plugins`**

Add this to `src-tauri/src/services/plugin.rs`:

```rust
pub async fn get_installed_plugins(
    config_dir: &std::path::Path,
    favorite_ids: &HashSet<String>,
) -> Result<Vec<PluginItem>> {
    let installed_map = get_installed_plugins_sync()?;
    let marketplaces = get_marketplaces_from_known_json(config_dir)?;
    
    let mut installed_plugins = Vec::new();
    
    for market in &marketplaces {
        if let Ok(plugins) = read_marketplace_plugins(&market.name, config_dir) {
            for plugin in plugins {
                let key = format!("{}@{}", plugin.name, plugin.marketplace_name);
                if let Some((version, enabled)) = installed_map.get(&key) {
                    installed_plugins.push(PluginItem {
                        name: plugin.name,
                        version: version.clone(),
                        description: plugin.description,
                        marketplace_name: plugin.marketplace_name,
                        is_installed: true,
                        is_enabled: Some(*enabled),
                        is_favorited: favorite_ids.contains(&key),
                    });
                }
            }
        }
    }
    
    installed_plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(installed_plugins)
}
```

- [ ] **Step 4: Create `get_marketplace_plugins`**

Add this to `src-tauri/src/services/plugin.rs`:

```rust
pub async fn get_marketplace_plugins(
    market_name: &str,
    config_dir: &std::path::Path,
    favorite_ids: &HashSet<String>,
) -> Result<Vec<PluginItem>> {
    let installed_map = get_installed_plugins_sync()?;
    let mut market_plugins = Vec::new();
    
    if let Ok(plugins) = read_marketplace_plugins(market_name, config_dir) {
        for plugin in plugins {
            let key = format!("{}@{}", plugin.name, plugin.marketplace_name);
            let (is_installed, version, is_enabled) = if let Some((ver, enabled)) = installed_map.get(&key) {
                (true, ver.clone(), Some(*enabled))
            } else {
                (false, plugin.version.clone(), None)
            };
            
            market_plugins.push(PluginItem {
                name: plugin.name,
                version,
                description: plugin.description,
                marketplace_name: plugin.marketplace_name,
                is_installed,
                is_enabled,
                is_favorited: favorite_ids.contains(&key),
            });
        }
    }
    
    market_plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(market_plugins)
}
```

- [ ] **Step 5: Update `get_favorites` logic**

Modify `get_favorites` in `src-tauri/src/services/plugin.rs` to not use the cache:

```rust
pub async fn get_favorites(
    config_dir: &std::path::Path,
    favorites: Vec<(String, String, String, Option<String>)>,
) -> Result<Vec<PluginFavoriteItem>> {
    let installed_map = get_installed_plugins_sync()?;
    
    Ok(favorites
        .into_iter()
        .map(|(plugin_id, plugin_name, marketplace_name, marketplace_source)| {
            let is_installed = installed_map.contains_key(&plugin_id);
            PluginFavoriteItem {
                plugin_id,
                plugin_name,
                marketplace_name,
                marketplace_source,
                is_installed,
            }
        })
        .collect())
}
```

- [ ] **Step 6: Update Action functions**

Modify `plugin_action` and `marketplace_action` in `src-tauri/src/services/plugin.rs` to return the simplified Result.

```rust
pub async fn plugin_action(
    action: &str,
    plugin_id: &str,
    config_dir: &std::path::Path,
    _favorite_ids: &HashSet<String>, // can remove _favorite_ids usage
) -> Result<PluginActionResult> {
    let output = match action {
        "install" => run_claude(&["plugin", "install", plugin_id]),
        "uninstall" => run_claude(&["plugin", "uninstall", plugin_id]),
        "enable" => run_claude(&["plugin", "enable", plugin_id]),
        "disable" => run_claude(&["plugin", "disable", plugin_id]),
        "update" => run_claude(&["plugin", "update", plugin_id]),
        _ => return Err(format!("未知的插件操作: {}", action)),
    }?;

    Ok(PluginActionResult {
        cli_output: output,
    })
}

pub async fn marketplace_action(
    action: &str,
    param: &str,
    config_dir: &std::path::Path,
    _favorite_ids: &HashSet<String>,
) -> Result<MarketplaceActionResult> {
    let output = match action {
        "add" => run_claude(&["plugin", "marketplace", "add", param]),
        "remove" => run_claude(&["plugin", "marketplace", "remove", param]),
        "update" => run_claude(&["plugin", "marketplace", "update", param]),
        _ => return Err(format!("未知的市场操作: {}", action)),
    }?;

    Ok(MarketplaceActionResult {
        cli_output: output,
    })
}
```

Modify `install_favorite_plugin` similarly.
Remove `update_cache_favorite_status` function.

### Task 2: Update Tauri Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Update API handlers**

```rust
#[tauri::command]
pub async fn get_installed_plugins(
    config_dir: State<'_, AppConfigDir>,
    favorite_ids: State<'_, Arc<Mutex<HashSet<String>>>>,
) -> Result<Vec<PluginItem>> {
    let favorite_ids = favorite_ids.lock().unwrap().clone();
    crate::services::plugin::get_installed_plugins(&config_dir, &favorite_ids).await
}

#[tauri::command]
pub async fn get_marketplace_plugins(
    market_name: String,
    config_dir: State<'_, AppConfigDir>,
    favorite_ids: State<'_, Arc<Mutex<HashSet<String>>>>,
) -> Result<Vec<PluginItem>> {
    let favorite_ids = favorite_ids.lock().unwrap().clone();
    crate::services::plugin::get_marketplace_plugins(&market_name, &config_dir, &favorite_ids).await
}
```

Remove `get_all_plugins` and `refresh_plugins`.

- [ ] **Step 2: Clean up favorite commands**

In `add_plugin_favorite` and `remove_plugin_favorite`, remove the call to `crate::services::plugin::update_cache_favorite_status`.

- [ ] **Step 3: Update action commands**

Update `plugin_action` and `marketplace_action` in `commands.rs` to match the new signatures (they no longer need `favorite_ids` if you removed it, but fine if kept).

- [ ] **Step 4: Register Commands**

In `src-tauri/src/main.rs`, update the `invoke_handler`:
Replace `get_all_plugins` and `refresh_plugins` with `get_installed_plugins` and `get_marketplace_plugins`.

### Task 3: Update Frontend API

**Files:**
- Modify: `frontend/src/api/plugins.ts`
- Modify: `frontend/src/types/models.ts`

- [ ] **Step 1: Update Models**

In `frontend/src/types/models.ts`, update `PluginActionResult` and `MarketplaceActionResult` to remove `plugins` and `marketplaces` arrays.

```typescript
export interface PluginActionResult {
  cli_output: string
}

export interface MarketplaceActionResult {
  cli_output: string
}
```

- [ ] **Step 2: Update API functions**

In `frontend/src/api/plugins.ts`:
Replace `getAll()` and `refresh()` with:

```typescript
  getInstalled: async (): Promise<PluginItem[]> => {
    return await invoke<PluginItem[]>('get_installed_plugins')
  },
  getMarketplacePlugins: async (marketName: string): Promise<PluginItem[]> => {
    return await invoke<PluginItem[]>('get_marketplace_plugins', { marketName })
  },
```

### Task 4: Update Frontend Vue Component

**Files:**
- Modify: `frontend/src/views/plugins/index.vue`

- [ ] **Step 1: Replace State**

Remove `allPlugins`.
Add `installedPlugins = ref<PluginItem[]>([])`
Add `marketPlugins = ref<PluginItem[]>([])`

- [ ] **Step 2: Update `loadAll`**

Rename to `loadInitial` or just fetch specific data based on the active tab.

```typescript
async function loadInstalled() {
  loading.value = true
  try {
    installedPlugins.value = await pluginsApi.getInstalled()
    favoriteList.value = await pluginsApi.getFavorites()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loading.value = false
  }
}

async function loadMarketplaces() {
  loadingMarketplaces.value = true
  try {
    marketplaceList.value = await pluginsApi.getMarketplaces()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loadingMarketplaces.value = false
  }
}

async function loadMarketplacePlugins(name: string) {
  loading.value = true
  try {
    marketPlugins.value = await pluginsApi.getMarketplacePlugins(name)
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loading.value = false
  }
}
```

- [ ] **Step 3: Update Tab Watcher**

```typescript
watch(activeTab, (newTab) => {
  if (newTab === 'plugins') {
    loadInstalled()
  } else if (newTab === 'marketplaces') {
    if (!currentMarket.value) {
      loadMarketplaces()
    } else {
      loadMarketplacePlugins(currentMarket.value.name)
    }
  } else if (newTab === 'favorites') {
    loadFavorites()
  }
}, { immediate: true })

// Add loadFavorites
async function loadFavorites() {
  try {
    favoriteList.value = await pluginsApi.getFavorites()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  }
}
```

- [ ] **Step 4: Update Actions**

When an action (like install, uninstall) is performed, re-fetch the list for the current context.

For example, `handleInstall`:
```typescript
  try {
    const result = await pluginsApi.install(pluginId)
    showCliOutput(result.cli_output)
    
    // Refresh current view
    if (activeTab.value === 'marketplaces' && currentMarket.value) {
      await loadMarketplacePlugins(currentMarket.value.name)
    } else if (activeTab.value === 'plugins') {
      await loadInstalled()
    }
    await loadFavorites()
  } catch ...
```

Do this for `handleUninstall`, `handleToggleEnable`, `handleAddFavorite`, `handleRemoveFavorite`, etc.

- [ ] **Step 5: Check computed properties**

The `installedPlugins` and `marketPlugins` in the template are now `ref`s instead of computed properties filtering `allPlugins`. Ensure the template binds correctly to them (remove the `computed` definitions). Re-implement search filtering for `marketPlugins` using a computed wrapper `filteredMarketPlugins`.

```typescript
const filteredMarketPlugins = computed(() => {
  if (!pluginSearchQuery.value) return marketPlugins.value
  const q = pluginSearchQuery.value.toLowerCase()
  return marketPlugins.value.filter(p =>
    p.name.toLowerCase().includes(q) ||
    p.description?.toLowerCase().includes(q)
  )
})
```
Update template to use `filteredMarketPlugins`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/plugin.rs src-tauri/src/commands.rs src-tauri/src/db/models.rs src-tauri/src/main.rs frontend/src/api/plugins.ts frontend/src/types/models.ts frontend/src/views/plugins/index.vue
git commit -m "refactor: migrate plugin management to on-demand loading"
```
