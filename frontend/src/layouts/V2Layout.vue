<template>
  <div class="v2-shell">
    <AppTitleBar />
    <header class="v2-topbar">
      <div class="v2-topbar-inner">
        <div class="v2-brand" @click="go('/')">
          <span class="v2-logo-text">CCG <span>Gateway</span></span>
          <el-tooltip :content="gatewayStatusText" placement="bottom" effect="light" :show-after="150">
            <button
              class="v2-gateway-status"
              :class="[gatewayStatusTone, { 'needs-attention': !gatewayDialogSeen }]"
              type="button"
              :aria-label="gatewayStatusText"
              @click.stop="openGatewayDialog"
            ><span></span></button>
          </el-tooltip>
        </div>

        <nav ref="navEl" class="v2-nav" :class="{ 'has-slider': navSlider.ready }" :style="navSliderStyle">
          <span class="v2-nav-slider" aria-hidden="true"></span>
          <button
            v-for="it in visibleNav"
            :key="it.path"
            :ref="(el) => setNavItemRef(it.path, el)"
            class="v2-nav-item"
            :class="{ active: isActive(it.path) }"
            @click="go(it.path)"
          >{{ it.label }}</button>

          <el-dropdown v-if="overflowNav.length" trigger="click" placement="bottom-end" popper-class="v2-more-pop">
            <button :ref="(el) => setNavItemRef(MORE_NAV_KEY, el)" class="v2-nav-item v2-nav-more" :class="{ active: overflowActive }">
              更多
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item
                  v-for="it in overflowNav"
                  :key="it.path"
                  :class="{ active: isActive(it.path) }"
                  @click="go(it.path)"
                >{{ it.label }}</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </nav>

        <div class="v2-topbar-right">
          <div class="v2-util">
            <el-tooltip :content="themeStore.theme === 'light' ? '切换暗色' : '切换亮色'" placement="bottom" effect="light" :show-after="250">
              <button class="v2-icon-btn" @click="themeStore.toggleTheme()">
                <svg v-if="themeStore.theme === 'light'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
                <svg v-else width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
              </button>
            </el-tooltip>
            <el-tooltip content="开发者工具" placement="bottom" effect="light" :show-after="250">
              <button class="v2-icon-btn" @click="toggleDevtools">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
              </button>
            </el-tooltip>
            <el-tooltip content="GitHub 仓库" placement="bottom" effect="light" :show-after="250">
              <button class="v2-icon-btn" @click="openGithubRepo">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/></svg>
              </button>
            </el-tooltip>
            <el-tooltip content="检查更新" placement="bottom" effect="light" :show-after="250">
              <button class="v2-version mono" :disabled="checkingUpdate" @click="handleCheckUpdate">
                <svg v-if="checkingUpdate" class="spin" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px; display: inline-block; vertical-align: middle;"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/></svg>
                v{{ appVersion }}
              </button>
            </el-tooltip>
          </div>
        </div>
      </div>
    </header>

    <main class="v2-main">
      <div class="v2-container">
        <router-view />
      </div>
    </main>

    <V2Drawer
      v-model="gatewayDialogVisible"
      title="网关设置"
      width="560px"
      :closable="!gatewayEndpointSaving"
    >
      <div class="gateway-listen-status" :class="gatewayStatusTone">
        <el-icon class="gateway-listen-status-icon">
          <CircleCheckFilled v-if="gatewayIsRunning" />
          <CircleCloseFilled v-else-if="gatewayIsError" />
          <Loading v-else />
        </el-icon>
        <div class="gateway-listen-status-copy">
          <div class="gateway-listen-status-title">{{ gatewayListenStatusTitle }}</div>
          <div class="gateway-listen-status-detail">{{ gatewayListenStatusDetail }}</div>
        </div>
      </div>

      <div class="gateway-settings-section">
        <div class="gateway-endpoint-grid">
          <div class="v2-field">
            <label class="v2-label">监听地址</label>
            <el-tooltip
              content="已被环境变量 CCG_GATEWAY_HOST 覆盖，无法编辑"
              placement="top"
              effect="light"
              :show-after="250"
              :disabled="!hostEnvOverride"
            >
              <span class="gateway-control-shell">
                <el-autocomplete
                  v-model="gatewayEndpointForm.gateway_host"
                  :fetch-suggestions="gatewayHostSuggestions"
                  :debounce="0"
                  fit-input-width
                  class="gateway-host-autocomplete mono"
                  popper-class="gateway-host-popper v2-scope"
                  :disabled="hostEnvOverride || dataDirChanging"
                  placeholder="选择或输入监听地址"
                >
                  <template #default="{ item }">
                    <div class="gateway-host-option">
                      <span class="mono">{{ item.value }}</span>
                      <span class="gateway-host-option-label">{{ item.label }}</span>
                    </div>
                  </template>
                </el-autocomplete>
              </span>
            </el-tooltip>
          </div>
          <div class="v2-field">
            <label class="v2-label">监听端口</label>
            <el-tooltip
              content="已被环境变量 CCG_GATEWAY_PORT 覆盖，无法编辑"
              placement="top"
              effect="light"
              :show-after="250"
              :disabled="!portEnvOverride"
            >
              <span class="gateway-control-shell">
                <input
                  v-model.number="gatewayEndpointForm.gateway_port"
                  type="number"
                  min="1"
                  max="65535"
                  step="1"
                  class="v2-input mono"
                  :disabled="portEnvOverride || dataDirChanging"
                  placeholder="7788"
                >
              </span>
            </el-tooltip>
          </div>
        </div>
        <div v-if="exposesGateway" class="gateway-endpoint-warning">
          <el-icon><WarningFilled /></el-icon>
          <span>当前地址允许其他设备访问，请仅在可信网络中使用。</span>
        </div>
      </div>

      <div class="gateway-settings-section">
        <div class="v2-field">
          <label class="v2-label">数据目录</label>
          <el-tooltip
            content="已被环境变量 CCG_DATA_DIR 覆盖，无法编辑"
            placement="top"
            effect="light"
            :show-after="250"
            :disabled="!dataDirEnvOverride"
          >
            <div class="gateway-path-row">
              <input
                v-model="gatewayEndpointForm.data_dir"
                class="v2-input mono"
                :disabled="dataDirEnvOverride"
                placeholder="输入数据目录"
              >
              <el-tooltip content="恢复默认" placement="top" effect="light" :show-after="250" :disabled="dataDirEnvOverride">
                <button
                  type="button"
                  class="v2-row-act gateway-path-reset"
                  :disabled="dataDirEnvOverride || gatewayEndpointSaving"
                  @click="resetGatewayDataDir"
                >
                  <svg :class="{ spinning: gatewayDataDirResetting }" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
                </button>
              </el-tooltip>
            </div>
          </el-tooltip>
          <div v-if="!dataDirEnvOverride && dataDirChanging" class="v2-hint">数据目录需单独保存并关闭应用后，再调整监听地址和端口。</div>
          <div v-else-if="!dataDirEnvOverride" class="v2-hint">修改后重启生效，原目录的数据库、日志和用户配置不会自动迁移。</div>
        </div>

        <div class="gateway-toggle-row">
          <span class="gateway-toggle-copy">
            <span class="gateway-toggle-title">文件日志</span>
            <span class="gateway-toggle-desc">运行日志写入数据目录</span>
          </span>
          <el-tooltip
            content="已被环境变量 CCG_LOG_FILE 覆盖，无法编辑"
            placement="top"
            effect="light"
            :show-after="250"
            :disabled="!logFileEnvOverride"
          >
            <span class="gateway-switch-control">
              <el-switch v-model="gatewayEndpointForm.log_file" :disabled="logFileEnvOverride" />
            </span>
          </el-tooltip>
        </div>
        <div class="v2-field gateway-log-level-field">
          <label class="v2-label">日志级别</label>
          <el-tooltip
            content="已被环境变量 CCG_LOG_LEVEL 覆盖，无法编辑"
            placement="top"
            effect="light"
            :show-after="250"
            :disabled="!logLevelEnvOverride"
          >
            <span class="gateway-control-shell">
              <input
                v-model="gatewayEndpointForm.log_level"
                class="v2-input mono"
                :disabled="logLevelEnvOverride"
                placeholder="info"
              >
            </span>
          </el-tooltip>
        </div>
      </div>

      <template #footer>
        <button class="v2-btn v2-btn-sm v2-btn-ghost" :disabled="gatewayEndpointSaving" @click="gatewayDialogVisible = false">取消</button>
        <button
          class="v2-btn v2-btn-sm v2-btn-primary"
          :disabled="gatewayEndpointSaving || !canSaveGatewayEndpoint"
          @click="saveGatewayEndpoint"
        >{{ gatewayEndpointSaving ? '处理中...' : gatewayEndpointActionText }}</button>
      </template>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { useWindowSize } from '@vueuse/core'
import { checkForUpdates } from '@/utils/updater'
import { useThemeStore } from '@/stores/theme'
import { useSettingsStore } from '@/stores/settings'
import { useAgentStore } from '@/stores/agents'
import { settingsApi } from '@/api/settings'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import AppTitleBar from '@/components/AppTitleBar.vue'
import V2Drawer from '@/components/V2Drawer.vue'
import { CircleCheckFilled, CircleCloseFilled, Loading, WarningFilled } from '@element-plus/icons-vue'
import type { ComponentPublicInstance } from 'vue'

const route = useRoute()
const router = useRouter()
const themeStore = useThemeStore()
const settingsStore = useSettingsStore()
const agentStore = useAgentStore()

const nav = [
  { label: '仪表盘', path: '/' },
  { label: '服务商', path: '/providers' },
  { label: '全局设置', path: '/config' },
  { label: '日志记录', path: '/logs' },
  { label: '定时任务', path: '/scheduled-tasks' },
  { label: '会话记录', path: '/sessions' },
  { label: 'MCP', path: '/mcp' },
  { label: '提示词', path: '/prompts' },
  { label: 'Skill', path: '/skills' },
  { label: 'Plugin', path: '/plugins' }
]
const MORE_NAV_KEY = '__more'

const { width } = useWindowSize()
const maxVisible = computed(() => {
  const w = width.value
  if (w >= 1180) return nav.length
  if (w >= 1040) return 8
  if (w >= 900) return 6
  if (w >= 780) return 4
  return 3
})
const visibleNav = computed(() => nav.slice(0, maxVisible.value))
const overflowNav = computed(() => nav.slice(maxVisible.value))
const overflowActive = computed(() => overflowNav.value.some(it => isActive(it.path)))
const activeNavKey = computed(() => visibleNav.value.find(it => isActive(it.path))?.path || (overflowActive.value ? MORE_NAV_KEY : ''))
const navEl = ref<HTMLElement | null>(null)
const navItemEls = new Map<string, HTMLElement>()
const navSlider = reactive({ x: 0, width: 0, ready: false })
const navSliderStyle = computed(() => ({
  '--v2-nav-slider-x': `${navSlider.x}px`,
  '--v2-nav-slider-w': `${navSlider.width}px`
}))

function setNavItemRef(path: string, el: Element | ComponentPublicInstance | null) {
  if (el instanceof HTMLElement) {
    navItemEls.set(path, el)
    return
  }
  if (el && '$el' in el && el.$el instanceof HTMLElement) {
    navItemEls.set(path, el.$el)
    return
  }
  navItemEls.delete(path)
}

function updateNavSlider() {
  const navNode = navEl.value
  const activeNode = activeNavKey.value ? navItemEls.get(activeNavKey.value) : null
  if (!navNode || !activeNode) {
    navSlider.ready = false
    return
  }
  const navRect = navNode.getBoundingClientRect()
  const activeRect = activeNode.getBoundingClientRect()
  navSlider.x = activeRect.left - navRect.left
  navSlider.width = activeRect.width
  navSlider.ready = true
}

function scheduleNavSliderUpdate() {
  nextTick(() => requestAnimationFrame(updateNavSlider))
}

watch(() => [route.path, width.value, visibleNav.value.length, overflowNav.value.length], scheduleNavSliderUpdate)

function isActive(path: string) {
  return path === '/' ? route.path === '/' : route.path.startsWith(path)
}
function go(path: string) {
  if (route.path !== path) router.push(path)
}

const appVersion = ref('0.0.0')
const checkingUpdate = ref(false)
const gatewayDialogVisible = ref(false)
const GATEWAY_DIALOG_SEEN_KEY = 'ccg-gateway-endpoint-dialog-seen'

function readGatewayDialogSeen() {
  try {
    return localStorage.getItem(GATEWAY_DIALOG_SEEN_KEY) === '1'
  } catch {
    return true
  }
}

const gatewayDialogSeen = ref(readGatewayDialogSeen())
const gatewayEndpointForm = ref({
  gateway_host: '127.0.0.1',
  gateway_port: 7788,
  data_dir: '',
  log_file: false,
  log_level: 'info',
})
const gatewayHostSuggestions = [
  { value: '127.0.0.1', label: '仅本机' },
  { value: '0.0.0.0', label: '局域网' },
]
const gatewayEndpointSaving = ref(false)
const gatewayDataDirResetting = ref(false)
const gatewayStatus = computed(() => settingsStore.gatewayStatus)
const gatewayStatusTone = computed(() => settingsStore.gatewayStatus?.status ?? 'starting')
const gatewayIsRunning = computed(() => gatewayStatus.value?.status === 'running')
const gatewayIsError = computed(() => gatewayStatus.value?.status === 'error')
function formatGatewayEndpoint(host: string, port: number) {
  const normalizedHost = host.includes(':') && !host.startsWith('[') ? `[${host}]` : host
  return `${normalizedHost}:${port}`
}
const gatewayListenStatusTitle = computed(() => {
  if (gatewayIsRunning.value) return '监听正常'
  if (gatewayIsError.value) return '监听失败'
  return '正在启动监听'
})
const gatewayListenStatusDetail = computed(() => {
  const status = gatewayStatus.value
  if (!status) return '正在获取监听状态...'
  if (status.status === 'error') {
    return status.error_message || `无法监听 ${formatGatewayEndpoint(status.host, status.port)}`
  }
  const endpoint = formatGatewayEndpoint(status.host, status.port)
  return status.status === 'running' ? `已监听 ${endpoint}` : `正在尝试监听 ${endpoint}`
})
const gatewayStatusText = computed(() => {
  const status = settingsStore.gatewayStatus
  const action = gatewayDialogSeen.value ? '点击修改网关配置' : '点击查看网关配置'
  if (!status || status.status === 'starting') return `网关启动中 · ${action}`
  if (status.status === 'running') {
    const host = status.host.includes(':') && !status.host.startsWith('[') ? `[${status.host}]` : status.host
    return `网关运行中 · 监听于 ${host}:${status.port} · ${action}`
  }
  return `${status.error_message || '网关启动失败'} · ${action}`
})
const hostEnvOverride = computed(() => gatewayStatus.value?.host_env_override === true)
const portEnvOverride = computed(() => gatewayStatus.value?.port_env_override === true)
const dataDirEnvOverride = computed(() => gatewayStatus.value?.data_dir_env_override === true)
const logFileEnvOverride = computed(() => gatewayStatus.value?.log_file_env_override === true)
const logLevelEnvOverride = computed(() => gatewayStatus.value?.log_level_env_override === true)
const dataDirChanging = computed(() => (
  !!gatewayStatus.value
  && !dataDirEnvOverride.value
  && gatewayEndpointForm.value.data_dir.trim() !== gatewayStatus.value.data_dir
))
const gatewayListenerDirty = computed(() => {
  const saved = settingsStore.settings?.gateway
  const savedHost = saved?.gateway_host ?? gatewayStatus.value?.host
  const savedPort = saved?.gateway_port ?? gatewayStatus.value?.port
  return savedHost !== undefined && savedPort !== undefined && (
    (!hostEnvOverride.value && gatewayEndpointForm.value.gateway_host.trim() !== savedHost)
    || (!portEnvOverride.value && Number(gatewayEndpointForm.value.gateway_port) !== savedPort)
  )
})
const bootstrapDirty = computed(() => {
  const status = gatewayStatus.value
  return !!status && (
    (!dataDirEnvOverride.value && gatewayEndpointForm.value.data_dir.trim() !== status.data_dir)
    || (!logFileEnvOverride.value && gatewayEndpointForm.value.log_file !== status.log_file)
    || (!logLevelEnvOverride.value && gatewayEndpointForm.value.log_level.trim() !== status.log_level)
  )
})
const gatewayEndpointDirty = computed(() => (
  gatewayListenerDirty.value || bootstrapDirty.value
))
const canSaveGatewayEndpoint = computed(() => gatewayEndpointDirty.value || gatewayStatus.value?.status === 'error')
const gatewayEndpointActionText = computed(() => {
  if (!gatewayEndpointDirty.value) {
    return gatewayStatus.value?.status === 'error' ? '关闭应用' : '无更改'
  }
  return '保存并关闭'
})
const exposesGateway = computed(() => {
  const host = (hostEnvOverride.value
    ? gatewayStatus.value?.host
    : gatewayEndpointForm.value.gateway_host)?.trim().toLowerCase()
  return !!host && host !== 'localhost' && host !== '::1' && !/^127(?:\.\d{1,3}){3}$/.test(host)
})
let gatewayStatusTimer: ReturnType<typeof setInterval> | null = null
let gatewayErrorAutoOpened = false

function markGatewayDialogSeen() {
  if (gatewayDialogSeen.value) return
  gatewayDialogSeen.value = true
  try {
    localStorage.setItem(GATEWAY_DIALOG_SEEN_KEY, '1')
  } catch {
    // Keep the in-memory state when localStorage is unavailable.
  }
}

async function openGatewayDialog() {
  try {
    const status = gatewayStatus.value ?? await settingsStore.fetchGatewayStatus()
    if (!status) return
    gatewayEndpointForm.value = {
      gateway_host: status.host,
      gateway_port: status.port,
      data_dir: status.data_dir,
      log_file: status.log_file,
      log_level: status.log_level,
    }
    gatewayDialogVisible.value = true
    markGatewayDialogSeen()
    void settingsStore.fetchGatewayStatus().catch(() => undefined)
  } catch (error) {
    notify(getErrorMessage(error, '网关设置加载失败'), 'error')
  }
}

function resetGatewayDataDir() {
  if (gatewayDataDirResetting.value) return
  const defaultDataDir = gatewayStatus.value?.default_data_dir
  if (defaultDataDir) gatewayEndpointForm.value.data_dir = defaultDataDir
  gatewayDataDirResetting.value = true
  setTimeout(() => {
    gatewayDataDirResetting.value = false
  }, 600)
}

async function closeGatewayApp(message: string) {
  gatewayEndpointSaving.value = true
  try {
    await settingsApi.closeApp()
    notify(message)
  } catch (error) {
    gatewayEndpointSaving.value = false
    notify(getErrorMessage(error, '应用关闭失败'), 'error')
  }
}

async function saveGatewayEndpoint() {
  if (!gatewayEndpointDirty.value) {
    if (gatewayStatus.value?.status !== 'error') return
    await closeGatewayApp('应用将关闭，请手动重新启动')
    return
  }

  const host = gatewayEndpointForm.value.gateway_host.trim()
  const port = Number(gatewayEndpointForm.value.gateway_port)
  const dataDir = gatewayEndpointForm.value.data_dir.trim()
  const logLevel = gatewayEndpointForm.value.log_level.trim()

  if (gatewayListenerDirty.value) {
    if (!host) {
      notify('监听地址不能为空', 'error')
      return
    }
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      notify('监听端口必须是 1 到 65535 之间的整数', 'error')
      return
    }
  }
  if (bootstrapDirty.value && !dataDirEnvOverride.value && !dataDir) {
    notify('数据目录不能为空', 'error')
    return
  }
  if (dataDirChanging.value && gatewayListenerDirty.value) {
    notify('修改数据目录时请先单独保存并关闭应用，再调整监听地址和端口', 'error')
    return
  }
  if (bootstrapDirty.value && !logLevelEnvOverride.value && !logLevel) {
    notify('日志级别不能为空', 'error')
    return
  }

  if (gatewayListenerDirty.value) {
    const effectiveHost = hostEnvOverride.value ? gatewayStatus.value?.host ?? host : host
    const effectivePort = portEnvOverride.value ? gatewayStatus.value?.port ?? port : port
    try {
      await settingsApi.validateGatewayBind(effectiveHost, effectivePort)
    } catch (error) {
      try {
        await confirm(
          `当前监听地址暂时无法绑定：${getErrorMessage(error, '未知错误')}\n\n这可能由旧监听仍被当前进程占用、端口冲突或地址尚不可用导致。仍可继续保存；保存后应用会关闭，请确认端口可用后手动重新启动。`,
          '监听地址检查失败',
          { confirmText: '继续保存' },
        )
      } catch (confirmError) {
        if (confirmError === 'cancel') return
        throw confirmError
      }
    }
  }

  gatewayEndpointSaving.value = true
  try {
    if (bootstrapDirty.value) {
      const bootstrapUpdate: { data_dir?: string; log_file?: boolean; log_level?: string } = {}
      if (!dataDirEnvOverride.value) bootstrapUpdate.data_dir = dataDir
      if (!logFileEnvOverride.value) bootstrapUpdate.log_file = gatewayEndpointForm.value.log_file
      if (!logLevelEnvOverride.value) bootstrapUpdate.log_level = logLevel
      await settingsStore.updateBootstrap(bootstrapUpdate)
    }
    if (gatewayListenerDirty.value) {
      const listenerUpdate: { gateway_host?: string; gateway_port?: number } = {}
      if (!hostEnvOverride.value) listenerUpdate.gateway_host = host
      if (!portEnvOverride.value) listenerUpdate.gateway_port = port
      await settingsStore.updateGateway(listenerUpdate)
    }
    await settingsApi.closeApp()
    notify('配置已保存，应用将关闭，请手动重新启动')
  } catch (error) {
    gatewayEndpointSaving.value = false
    notify(getErrorMessage(error, '网关设置保存失败'), 'error')
  }
}

async function handleCheckUpdate() {
  checkingUpdate.value = true
  try {
    await checkForUpdates(false)
  } finally {
    checkingUpdate.value = false
  }
}
async function openGithubRepo() {
  await open('https://github.com/mos1128/ccg-gateway')
}
async function toggleDevtools() {
  await invoke('toggle_devtools')
}

watch(() => gatewayStatus.value?.status, (status) => {
  if (status === 'running') {
    gatewayErrorAutoOpened = false
    return
  }
  if (status !== 'error' || gatewayErrorAutoOpened) return
  gatewayErrorAutoOpened = true
  if (!gatewayDialogVisible.value) void openGatewayDialog()
}, { immediate: true })

onMounted(async () => {
  scheduleNavSliderUpdate()
  gatewayStatusTimer = setInterval(() => {
    void settingsStore.fetchGatewayStatus().catch(() => undefined)
  }, 2000)
  void settingsStore.fetchGatewayStatus().catch(() => undefined)
  appVersion.value = await getVersion()
  await Promise.all([
    agentStore.agents.length ? Promise.resolve() : agentStore.fetchAgents(),
    settingsStore.settings ? Promise.resolve() : settingsStore.fetchSettings(),
  ])
  checkForUpdates(true)
})

onUnmounted(() => {
  if (gatewayStatusTimer) clearInterval(gatewayStatusTimer)
})
</script>

<style>
/* ========== v2 设计系统（令牌挂 :root 以便浮层取色；样式仅作用 .v2-shell，不影响现有界面） ========== */
:root {
  --v2-bg-base: var(--el-bg-color-page);
  --v2-mask-bg: var(--el-mask-color);
  --v2-surface: var(--el-bg-color);
  --v2-surface-2: var(--el-fill-color-light);
  --v2-surface-3: var(--el-border-color-extra-light);
  --v2-row-hover-bg: var(--el-fill-color);
  --v2-tab-divider: #dfe3e9;
  --v2-text: var(--el-text-color-primary);
  --v2-text-2: var(--el-text-color-regular);
  --v2-text-3: var(--el-text-color-placeholder);
  --v2-text-btn: var(--v2-text-2);
  --v2-accent: #2b70c9;
  --v2-accent-press: #245ea8;
  --v2-on-accent: #ffffff;
  --v2-success: var(--el-color-success);
  --v2-danger: var(--el-color-danger);
  --v2-on-danger: #ffffff;
  --v2-warning: var(--el-color-warning);
  --v2-info: var(--v2-accent);
  --v2-selected-bg: color-mix(in srgb, var(--v2-accent) 12%, var(--v2-surface));
  --v2-danger-bg: color-mix(in srgb, var(--v2-danger) 12%, transparent);
  --v2-warning-bg: color-mix(in srgb, var(--v2-warning) 12%, transparent);
  --v2-brand-bg: color-mix(in srgb, var(--v2-brand-current, var(--v2-accent)) 12%, var(--v2-surface));
  --v2-chart-purple: #8b5cf6;
  --v2-chart-cyan: #06b6d4;
  --v2-shadow-card: var(--el-box-shadow-light);
  --v2-shadow-pop: var(--el-box-shadow);
  --v2-r-sm: 4px;
  --v2-r: 8px;
  --v2-r-lg: 12px;
  --v2-fs-xs: 12px;
  --v2-fs-sm: 14px;
  --v2-fs-base: 14px;
  --v2-fs-md: 16px;
  --v2-fs-lg: 20px;
  --v2-fs-xl: 24px;
  --v2-fw-regular: 400;
  --v2-fw-medium: 500;
  --v2-fw-semibold: 600;
  --v2-fw-bold: 700;
  --v2-space-1: 4px;
  --v2-space-2: 8px;
  --v2-space-3: 12px;
  --v2-space-4: 16px;
  --v2-space-5: 20px;
  --v2-space-6: 24px;
  --v2-space-7: 28px;
  --v2-space-8: 32px;
  --v2-space-12: 48px;
}

.v2-shell {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--v2-bg-base);
  color: var(--v2-text);
  font-size: var(--v2-fs-base);
  font-family: var(--font-ui);
}

html.dark {
  --v2-tab-divider: #2b2b2c;
  --v2-accent: #5aa2ff;
  --v2-accent-press: #8cc2ff;
  --v2-on-accent: #0d1b22;
}

/* ===== 顶栏 ===== */
.v2-topbar {
  flex-shrink: 0;
  background: var(--v2-surface);
  border-bottom: 1px solid var(--v2-surface-2);
}
.v2-topbar-inner {
  max-width: 1240px;
  margin: 0 auto;
  height: 54px;
  padding: 0 28px;
  display: flex;
  align-items: center;
  gap: 28px;
}
.v2-brand {
  display: flex;
  align-items: center;
  gap: 9px;
  cursor: pointer;
  flex-shrink: 0;
}

.v2-logo-text {
  font-size: var(--v2-fs-base);
  font-weight: var(--v2-fw-semibold);
  letter-spacing: 0;
  color: var(--v2-text);
}
.v2-logo-text span {
  color: var(--v2-text-3);
  font-weight: var(--v2-fw-semibold);
}
.v2-gateway-status {
  position: relative;
  width: 20px;
  height: 20px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
}
.v2-gateway-status span {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--v2-text-3);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--v2-text-3) 12%, transparent);
}
.v2-gateway-status.running span {
  background: var(--v2-success);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--v2-success) 14%, transparent);
}
.v2-gateway-status.error span {
  background: var(--v2-danger);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--v2-danger) 14%, transparent);
}
.v2-gateway-status.starting span {
  animation: gateway-status-pulse 1.4s ease-in-out infinite;
}
.v2-gateway-status.needs-attention::after {
  content: "";
  position: absolute;
  inset: 1px;
  border: 1px solid color-mix(in srgb, var(--v2-accent) 70%, transparent);
  border-radius: 50%;
  animation: gateway-status-guide 1.8s ease-out infinite;
  pointer-events: none;
}
@keyframes gateway-status-pulse {
  50% { opacity: 0.35; }
}
@keyframes gateway-status-guide {
  0% { opacity: 0.8; transform: scale(0.5); }
  80%, 100% { opacity: 0; transform: scale(1.45); }
}

.gateway-listen-status {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  margin-bottom: 20px;
  border: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r);
  background: var(--v2-surface-2);
}
.gateway-listen-status.running {
  border-color: color-mix(in srgb, var(--v2-success) 24%, transparent);
  background: color-mix(in srgb, var(--v2-success) 9%, transparent);
}
.gateway-listen-status.error {
  border-color: color-mix(in srgb, var(--v2-danger) 24%, transparent);
  background: var(--v2-danger-bg);
}
.gateway-listen-status-icon {
  margin-top: 1px;
  flex-shrink: 0;
  color: var(--v2-text-3);
  font-size: 16px;
}
.gateway-listen-status.running .gateway-listen-status-icon {
  color: var(--v2-success);
}
.gateway-listen-status.error .gateway-listen-status-icon {
  color: var(--v2-danger);
}
.gateway-listen-status.starting .gateway-listen-status-icon {
  animation: gateway-reset-spin 1s linear infinite;
}
.gateway-listen-status-copy {
  min-width: 0;
}
.gateway-listen-status-title {
  color: var(--v2-text);
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-medium);
  line-height: 1.4;
}
.gateway-listen-status.running .gateway-listen-status-title {
  color: var(--v2-success);
}
.gateway-listen-status.error .gateway-listen-status-title {
  color: var(--v2-danger);
}
.gateway-listen-status-detail {
  margin-top: 3px;
  color: var(--v2-text-2);
  font-size: var(--v2-fs-xs);
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.gateway-endpoint-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 180px;
  gap: 12px;
}
.gateway-settings-section + .gateway-settings-section {
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid var(--v2-surface-3);
}
.gateway-endpoint-grid .v2-field {
  margin-bottom: 0;
}
.gateway-path-row {
  position: relative;
  display: flex;
  align-items: center;
}
.gateway-path-row .v2-input {
  flex: 1;
  padding-right: 36px;
}
.gateway-path-row .v2-row-act {
  position: absolute;
  right: 4px;
  flex-shrink: 0;
}
.gateway-path-reset:disabled {
  color: var(--v2-text-3);
  cursor: not-allowed;
  opacity: 0.45;
}
.gateway-path-reset svg.spinning {
  animation: gateway-reset-spin 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}
@keyframes gateway-reset-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.gateway-control-shell {
  display: block;
  width: 100%;
  min-width: 0;
}
.gateway-switch-control {
  display: inline-flex;
  width: auto;
}
.gateway-toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  margin: 0 0 16px;
  padding: 12px 0;
  border-top: 1px solid var(--v2-surface-2);
}
.gateway-toggle-copy {
  min-width: 0;
}
.gateway-toggle-title {
  display: block;
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text);
}
.gateway-toggle-desc {
  display: block;
  margin-top: 3px;
  font-size: var(--v2-fs-xs);
  line-height: 1.4;
  color: var(--v2-text-3);
}
.gateway-log-level-field {
  margin-bottom: 0;
}
.gateway-host-autocomplete {
  width: 100%;
}
.gateway-host-autocomplete .el-input__wrapper {
  height: 32px;
  padding: 0 16px;
  border-radius: var(--v2-r);
  background: var(--v2-bg-base);
  box-shadow: none !important;
}
.gateway-host-autocomplete .el-input__inner {
  color: var(--v2-text);
  font-family: var(--font-mono);
  font-size: 13px;
  font-weight: var(--v2-fw-medium);
}
.gateway-host-option {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.gateway-host-option-label {
  color: var(--v2-text-3);
  font-size: var(--v2-fs-xs);
}
.gateway-endpoint-warning {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 9px 11px;
  margin-top: 16px;
  border-radius: var(--v2-r-sm);
  background: var(--v2-warning-bg);
  color: var(--v2-text-2);
  font-size: var(--v2-fs-xs);
  line-height: 1.5;
}
.gateway-endpoint-warning .el-icon {
  margin-top: 2px;
  flex-shrink: 0;
  color: var(--v2-warning);
}
@media (max-width: 520px) {
  .gateway-endpoint-grid {
    grid-template-columns: 1fr;
  }
}

.v2-nav {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2px;
  flex: 1;
  min-width: 0;
  position: relative;
}
.v2-nav-slider {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: var(--v2-nav-slider-w, 0);
  border-radius: var(--v2-r-sm);
  background: var(--v2-surface-2);
  opacity: 0;
  transform: translateX(var(--v2-nav-slider-x, 0));
  transition: transform 0.22s ease, width 0.22s ease, opacity 0.12s ease;
  pointer-events: none;
}
.v2-nav.has-slider .v2-nav-slider {
  opacity: 1;
}
.v2-nav-item {
  appearance: none;
  border: none;
  background: transparent;
  color: var(--v2-text-2);
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-regular);
  padding: 7px 12px;
  line-height: 20px;
  border-radius: var(--v2-r-sm);
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s, color 0.15s;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  position: relative;
  z-index: 1;
}
.v2-nav-item:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-nav-item.active {
  background: var(--v2-surface-2);
  color: var(--v2-text);
  font-weight: var(--v2-fw-medium);
}
.v2-nav.has-slider .v2-nav-item.active,
.v2-nav.has-slider .v2-nav-item.active:hover {
  background: transparent;
}
.v2-nav-more svg {
  opacity: 0.6;
}

.v2-topbar-right {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-shrink: 0;
}
.v2-util {
  display: flex;
  align-items: center;
  gap: 2px;
}
.v2-icon-btn {
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--v2-text-3);
  border-radius: var(--v2-r-sm);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.v2-icon-btn:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-icon-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.v2-version {
  margin-left: 4px;
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  background: transparent;
  border: none;
  padding: 2px 6px;
  border-radius: var(--v2-r-sm);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  display: inline-flex;
  align-items: center;
}
.v2-version:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-version:disabled {
  cursor: not-allowed;
  opacity: 0.7;
}
.v2-shell button:focus,
.v2-scope button:focus {
  outline: none;
}
.v2-shell .spin {
  animation: v2-spin 1.6s linear infinite;
}
@keyframes v2-spin {
  to { transform: rotate(360deg); }
}

/* ===== 主区 ===== */
.v2-main {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.v2-container {
  padding: 28px max(28px, calc((100% - 1184px) / 2)) 48px;
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

/* ========== 可复用基元 ========== */
.v2-shell .mono, .v2-scope .mono {
  font-family: var(--font-mono);
  font-feature-settings: "tnum" 1, "cv01" 1;
  letter-spacing: 0;
}

/* 页头 */
.v2-page-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: var(--v2-space-6);
}
.v2-page-title {
  font-size: var(--v2-fs-lg);
  font-weight: var(--v2-fw-semibold);
  letter-spacing: 0;
  color: var(--v2-text);
  margin: 0;
}
.v2-page-sub {
  font-size: var(--v2-fs-sm);
  color: var(--v2-text-3);
  margin: 5px 0 0;
}
.v2-head-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

/* 卡片 */
.v2-card {
  background: var(--v2-surface);
  border: 1px solid rgba(0, 0, 0, 0.045);
  border-radius: var(--v2-r-lg);
  box-shadow: none;
}
html.dark .v2-card {
  border-color: rgba(255, 255, 255, 0.04);
}
.v2-card-pad {
  padding: 18px 20px;
}
.v2-card-title {
  font-size: var(--v2-fs-base);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text);
}

/* 按钮 */
.v2-btn {
  appearance: none;
  border: 1px solid transparent;
  border-radius: var(--v2-r-sm);
  padding: 8px 14px;
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-medium);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
  white-space: nowrap;
}
.v2-btn:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}
.v2-btn-sm {
  padding: 6px 10px;
  font-size: 13px;
  gap: 5px;
}
.v2-btn-sm svg {
  width: 14px;
  height: 14px;
}
.v2-btn-primary {
  background: var(--v2-accent);
  color: var(--v2-on-accent);
}
.v2-btn-primary:hover {
  background: var(--v2-accent-press);
}
.v2-btn-outline {
  background: var(--v2-surface);
  border-color: var(--v2-surface-3);
  color: var(--v2-text-btn);
}
.v2-btn-outline:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-btn-ghost {
  background: transparent;
  color: var(--v2-text-2);
}
.v2-btn-ghost:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}

/* 分段控件 */
.v2-seg {
  display: inline-flex;
  padding: 4px;
  background: var(--v2-bg-base);
  border: 1px solid transparent;
  border-radius: 999px;
  gap: 2px;
}
.v2-seg-btn {
  border: none;
  background: transparent;
  color: var(--v2-text-2);
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-medium);
  padding: 5px 14px;
  line-height: 20px;
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  white-space: nowrap;
}
.v2-seg-btn.active {
  background: var(--v2-selected-bg);
  color: var(--v2-text-2);
  font-weight: var(--v2-fw-medium);
  box-shadow: none;
}

/* 滑块动画全局支持 */
.v2-seg:has(.v2-seg-slider) {
  position: relative;
  display: inline-grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 0;
}
.v2-seg-slider {
  position: absolute;
  top: 4px;
  bottom: 4px;
  left: 4px;
  background: var(--v2-surface);
  border: none;
  border-radius: 999px;
  box-shadow: none;
  transition: transform 0.22s cubic-bezier(0.2, 0.8, 0.2, 1);
  z-index: 0;
}
.v2-seg:has(.v2-seg-slider) .v2-seg-btn {
  position: relative;
  z-index: 1;
  min-width: 86px;
  background: transparent !important;
  box-shadow: none !important;
  font-weight: var(--v2-fw-medium);
  text-align: center;
}
.v2-seg:has(.v2-seg-slider) .v2-seg-btn.active {
  background: transparent !important;
  color: var(--v2-text-2);
  box-shadow: none !important;
  font-weight: var(--v2-fw-medium) !important;
}

html.dark .v2-seg {
  background: var(--v2-bg-base);
}

html.dark .v2-seg-slider {
  background: var(--el-bg-color-overlay);
  box-shadow: var(--el-box-shadow-light);
}

/* 标签 */
.v2-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: var(--v2-fs-xs);
  line-height: 16px;
  font-weight: var(--v2-fw-medium);
  border: 1px solid transparent;
  flex-shrink: 0;
}
.v2-pill-neutral { background: var(--v2-surface-2); color: var(--v2-text-2); border-color: transparent; }
.v2-pill-success { background: color-mix(in srgb, var(--v2-success) 12%, transparent); color: var(--v2-success); border-color: transparent; }
.v2-pill-danger { background: var(--v2-danger-bg); color: var(--v2-danger); border-color: transparent; }
.v2-pill-accent { background: var(--v2-selected-bg); color: var(--v2-accent); border-color: transparent; }
.v2-pill-warn { background: color-mix(in srgb, var(--v2-warning) 12%, transparent); color: var(--v2-warning); border-color: transparent; }
.v2-pill-info { background: color-mix(in srgb, var(--v2-info) 12%, transparent); color: var(--v2-info); border-color: transparent; }
.v2-pill.dot::before { content: ""; width: 5px; height: 5px; border-radius: 50%; background: currentColor; flex-shrink: 0; }

/* 列表行（可复用基元） */
.v2-row { display: flex; align-items: center; gap: 12px; padding: 13px 16px; border-bottom: 1px solid var(--v2-surface-2); }
.v2-row:last-child { border-bottom: none; }
.v2-row.blacklisted { background: var(--v2-danger-bg); }
.v2-row-drag { display: flex; flex-direction: column; gap: 3px; cursor: grab; padding: 4px; opacity: 0.3; flex-shrink: 0; transition: opacity 0.15s; }
.v2-row-drag:hover { opacity: 0.7; }
.v2-row-drag span { width: 3px; height: 3px; border-radius: 50%; background: var(--v2-text-3); }
.v2-row-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; background: var(--v2-text-3); }
.v2-row-dot.ok { background: var(--v2-success); }
.v2-row-dot.danger { background: var(--v2-danger); }
.v2-row-name { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-semibold); color: var(--v2-text); white-space: nowrap; flex-shrink: 0; }
.v2-row-name.off { color: var(--v2-text-3); }
.v2-row-gap { flex: 1; min-width: 8px; }
.v2-row-acts { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
.v2-row-act { width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; border: none; background: transparent; color: var(--v2-text-btn); border-radius: 4px; cursor: pointer; transition: background 0.15s, color 0.15s; }
.v2-row-act:hover { background: var(--v2-surface-2); color: var(--v2-text); }
.v2-row-act.danger:hover { background: var(--v2-danger-bg); color: var(--v2-danger); }
.v2-row-act.off { color: var(--v2-text-3); opacity: 0.4; pointer-events: none; }
.v2-row-act svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }

/* Element Plus 开关适配 Ink */
.v2-shell .el-switch.is-checked .el-switch__core { background-color: var(--v2-accent); border-color: var(--v2-accent); }
.v2-shell .el-switch.is-checked .el-switch__core .el-switch__action { background-color: var(--v2-on-accent); }

/* Element Plus Pagination Override */
.v2-shell .el-pagination {
  --el-color-primary: var(--v2-accent) !important;
  --el-pagination-hover-color: var(--v2-accent-press);
}
.v2-shell .el-pagination .el-select__wrapper,
.v2-shell .el-pagination .el-input__wrapper,
.v2-shell .el-pagination .el-select__wrapper:hover,
.v2-shell .el-pagination .el-input__wrapper:hover,
.v2-shell .el-pagination .el-select__wrapper.is-focused,
.v2-shell .el-pagination .el-input__wrapper.is-focused {
  box-shadow: none !important;
  border-color: transparent !important;
  background-color: var(--v2-bg-base) !important;
}
.v2-shell .el-pagination .el-select__placeholder,
.v2-shell .el-pagination .el-select__selected-item {
  color: var(--v2-text) !important;
}
.v2-shell .el-pagination button,
.v2-shell .el-pagination .el-pager li,
.v2-shell .el-pagination button:hover,
.v2-shell .el-pagination .el-pager li:hover,
.v2-shell .el-pagination button:disabled {
  background-color: transparent !important;
  border: none !important;
  box-shadow: none !important;
}
.v2-shell .el-pagination .el-pager li {
  color: var(--v2-text-3) !important;
  font-weight: var(--v2-fw-medium);
}
.v2-shell .el-pagination .el-pager li.is-active {
  color: var(--v2-accent) !important;
  font-weight: var(--v2-fw-semibold);
}
.v2-shell .el-pagination .el-pager li:hover:not(.is-active) {
  color: var(--v2-text) !important;
}
.v2-shell .el-pagination button {
  color: var(--v2-text-3) !important;
}
.v2-shell .el-pagination button:hover:not(:disabled) {
  color: var(--v2-text) !important;
}

/* v2 浮层作用域（teleport 到 body 的抽屉/气泡继承 v2 字体） */
.v2-scope { font-family: var(--font-ui); }

/* 表单 */
.v2-field { margin-bottom: 16px; }
.v2-label { display: block; font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); color: var(--v2-text); margin-bottom: 7px; }
.v2-label .req { color: var(--v2-danger); margin-left: 4px; }
.v2-input {
  width: 100%;
  background: var(--v2-bg-base);
  color: var(--v2-text);
  border: 1px solid transparent;
  border-radius: var(--v2-r);
  font-size: 13px;
  font-weight: var(--v2-fw-medium);
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
}
input.v2-input {
  height: 32px;
  padding: 0 16px;
}
textarea.v2-input {
  padding: 8px 16px;
  resize: vertical;
  line-height: 1.55;
}
.v2-input:focus {
  outline: none;
}
.v2-input-surface {
  background: var(--v2-surface);
}
.v2-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.v2-input::placeholder {
  color: var(--v2-text-3);
}
.v2-input::-webkit-outer-spin-button,
.v2-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.v2-input[type="number"] {
  -moz-appearance: textfield;
}
.v2-hint { font-size: var(--v2-fs-xs); line-height: 1.5; color: var(--v2-text-3); margin-top: 6px; }
.v2-grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: var(--v2-space-4); }

.v2-file-editor {
  border-right: 1px solid var(--v2-surface-3);
  border-bottom: 1px solid var(--v2-surface-3);
  border-radius: var(--v2-r-lg, 8px);
  background: var(--v2-bg-base);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.v2-file-editor-header {
  height: 42px;
  padding: 0 14px;
  background: var(--v2-bg-base);
  border-bottom: 1px solid var(--v2-surface-3);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.v2-file-editor-title {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--v2-text);
  font-size: var(--v2-fs-xs);
  font-weight: var(--v2-fw-medium);
}
.v2-file-editor-title .file-icon {
  flex-shrink: 0;
  color: var(--v2-text-3);
  opacity: 0.8;
  margin-top: 1px;
}
.v2-file-editor-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-weight: var(--v2-fw-semibold);
  font-size: 13px;
}

.v2-file-editor-action {
  flex-shrink: 0;
  appearance: none;
  background: var(--v2-surface);
  border: none;
  border-radius: var(--v2-r-lg, 8px);
  padding: 5px 14px;
  font-size: 11px;
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text-2);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  box-shadow: none;
}
.v2-file-editor-action svg {
  color: var(--v2-text-3);
}
.v2-file-editor-body {
  position: relative;
  background: transparent;
}
.v2-file-editor-textarea {
  width: 100%;
  padding: 14px;
  line-height: 1.6;
  font-family: var(--font-mono);
  font-size: 13px;
  tab-size: 2;
  background: transparent;
  border: none;
  outline: none;
  color: var(--v2-text);
}
.v2-file-editor-textarea::placeholder {
  color: var(--v2-text-3);
}
/* 选项卡 */
.v2-tabs { display: flex; border-bottom: 1px solid var(--v2-tab-divider); margin-bottom: var(--v2-space-4); }
.v2-tab { padding: 9px 4px; margin-right: 12px; font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); color: var(--v2-text-2); cursor: pointer; margin-bottom: -1px; transition: color 0.15s; display: inline-flex; align-items: center; gap: 6px; position: relative; }
.v2-tab:hover { color: var(--v2-text); }
.v2-tab.active { color: var(--v2-accent); font-weight: var(--v2-fw-semibold); }
.v2-tab::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 50%;
  width: 0;
  height: 2px;
  background: var(--v2-accent);
  border-radius: 99px;
  transform: translateX(-50%);
  transition: width 0.25s cubic-bezier(0.25, 1, 0.5, 1);
}
.v2-tab.active::after {
  width: 14px;
}
.tab-brand-icon { display: inline-flex; align-items: center; justify-content: center; width: 14px; height: 14px; flex-shrink: 0; }

/* 移除按钮 */
.v2-x { width: 32px; height: 32px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; border: 1px solid var(--v2-surface-2); border-radius: var(--v2-r-sm); background: var(--v2-surface); color: var(--v2-text-3); cursor: pointer; transition: background 0.15s, border-color 0.15s, color 0.15s; }
.v2-x:hover { background: var(--v2-danger-bg); border-color: color-mix(in srgb, var(--v2-danger) 12%, transparent); color: var(--v2-danger); }
.v2-x svg { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; }

/* CLI 徽章（跨 CLI 启用控件，自动换行可扩展） */
.v2-chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.v2-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px 6px 10px;
  border-radius: 999px;
  border: 1px solid transparent;
  background: var(--v2-surface-2);
  color: var(--v2-text-2);
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-medium);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s, box-shadow 0.15s;
}
.v2-chip:hover {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--v2-surface-3);
  transition: background 0.15s;
}
.v2-chip.on {
  background: color-mix(in srgb, var(--v2-accent) 18%, var(--v2-surface));
  border-color: transparent;
  color: var(--v2-text-2);
  box-shadow: none;
}
.v2-chip.on .v2-chip-dot { background: var(--v2-accent); }

/* 配置卡片墙（MCP / 提示词等） */
.v2-cardgrid { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: var(--v2-space-4); }
.v2-ccard { padding: 15px 16px; display: flex; flex-direction: column; gap: 13px; }
.v2-ccard-head { display: flex; align-items: center; gap: 10px; }
.v2-ccard-icon { width: 34px; height: 34px; border-radius: 6px; background: var(--v2-surface-2); display: flex; align-items: center; justify-content: center; color: var(--v2-text-2); flex-shrink: 0; }
.v2-ccard-icon svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.v2-ccard-tt { flex: 1; min-width: 0; }
.v2-ccard-name { font-size: var(--v2-fs-base); font-weight: var(--v2-fw-medium); color: var(--v2-text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-left: 2px; }
.v2-ccard-sub { font-size: var(--v2-fs-xs); color: var(--v2-text-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px; padding-left: 2px; }
.v2-ccard-acts { display: flex; gap: 2px; flex-shrink: 0; }
.v2-ccard .v2-chip-row { border-top: 1px solid var(--v2-surface-2); padding-top: 12px; }
.v2-addcard { border: 1px dashed var(--v2-text-3); border-radius: var(--v2-r-lg); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--v2-text-3); cursor: pointer; min-height: 122px; transition: border-color 0.15s, color 0.15s, background 0.15s; }
.v2-addcard:hover { border-color: var(--v2-accent); color: var(--v2-text); background: var(--v2-surface); }
.v2-addcard svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.v2-addcard span { font-size: var(--v2-fs-sm); font-weight: var(--v2-fw-medium); }
.v2-addcard-row { flex-direction: row; min-height: 66px; gap: 8px; }
.v2-addcard-row svg { width: 17px; height: 17px; }

/* v2 纤细滚动条（覆盖全局粗滚动条，内部滚动区不抢眼） */
.v2-shell ::-webkit-scrollbar { width: 10px; height: 10px; }
.v2-shell ::-webkit-scrollbar-track { background: transparent; }
.v2-shell ::-webkit-scrollbar-thumb { background-color: rgba(0, 0, 0, 0.15); border: 3px solid transparent; background-clip: padding-box; border-radius: 999px; }
.v2-shell ::-webkit-scrollbar-thumb:hover { background-color: rgba(0, 0, 0, 0.3); }
.v2-shell ::-webkit-scrollbar-corner { background: transparent; }
.v2-scope ::-webkit-scrollbar { width: 10px; height: 10px; }
.v2-scope ::-webkit-scrollbar-track { background: transparent; }
.v2-scope ::-webkit-scrollbar-thumb { background-color: rgba(0, 0, 0, 0.15); border: 3px solid transparent; background-clip: padding-box; border-radius: 999px; }
.v2-scope ::-webkit-scrollbar-thumb:hover { background-color: rgba(0, 0, 0, 0.3); }
.v2-scope ::-webkit-scrollbar-corner { background: transparent; }

html.dark .v2-shell ::-webkit-scrollbar-thumb,
html.dark .v2-scope ::-webkit-scrollbar-thumb {
  background-color: rgba(255, 255, 255, 0.15);
}
html.dark .v2-shell ::-webkit-scrollbar-thumb:hover,
html.dark .v2-scope ::-webkit-scrollbar-thumb:hover {
  background-color: rgba(255, 255, 255, 0.3);
}

/* KPI */
.v2-kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(148px, 1fr));
  gap: 12px;
}
.v2-kpi {
  background: var(--v2-surface);
  border: 1px solid rgba(0, 0, 0, 0.045);
  border-radius: var(--v2-r);
  padding: 15px 16px;
  box-shadow: none;
}
html.dark .v2-kpi {
  border-color: rgba(255, 255, 255, 0.04);
}
.v2-kpi-label {
  font-size: var(--v2-fs-xs);
  color: var(--v2-text-3);
  margin-bottom: 9px;
}
.v2-kpi-value {
  font-size: var(--v2-fs-xl);
  font-weight: var(--v2-fw-bold);
  letter-spacing: 0;
  line-height: 1.1;
  color: var(--v2-text);
}

/* 自适应卡片栅格 */
.v2-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--v2-space-4);
}

/* 表格 */
.v2-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
}
.v2-table th {
  text-align: left;
  font-size: var(--v2-fs-xs);
  font-weight: var(--v2-fw-medium);
  color: var(--v2-text-2);
  padding: 12px 16px;
  border-bottom: 1px solid var(--v2-surface-3);
  white-space: nowrap;
  background: var(--v2-surface-2);
}
.v2-table td {
  font-size: var(--v2-fs-sm);
  color: var(--v2-text);
  padding: 12px 16px;
  border-bottom: 1px solid var(--v2-surface-3);
  white-space: nowrap;
}
.v2-table tbody tr:last-child td {
  border-bottom: none;
}
.v2-table tbody tr:hover td {
  background: var(--v2-row-hover-bg);
}

/* 更多菜单 popper */
.v2-more-pop.el-popper {
  border-radius: var(--v2-r) !important;
  border: 1px solid var(--v2-surface-2) !important;
  box-shadow: var(--v2-shadow-pop) !important;
}
.v2-more-pop .el-dropdown-menu__item.active {
  color: var(--v2-accent);
  font-weight: var(--v2-fw-semibold);
}
.el-popper.is-light {
  --el-popper-bg-color-light: var(--v2-surface);
  --el-border-color-light: var(--v2-surface-2);
  --el-bg-color-overlay: var(--v2-surface);
  color: var(--v2-text);
  box-shadow: var(--v2-shadow-pop);
}

/* 通用帮助提示 */
.v2-help {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: help;
  color: var(--v2-text-3);
}
.v2-help:hover {
  color: var(--v2-text-2);
}
.v2-help .el-icon {
  width: 14px;
  height: 14px;
  font-size: 14px;
}
.v2-profile-pop.el-popper {
  border-radius: 10px;
  border: 1px solid var(--v2-surface-2);
  box-shadow: var(--v2-shadow-pop);
  padding: 14px;
  background: var(--v2-surface);
}
.v2-profile-pop .profile-help {
  width: 300px;
}
.v2-profile-pop .profile-help .tooltip-title {
  font-size: var(--v2-fs-sm);
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text);
  margin-bottom: 8px;
}
.v2-profile-pop .profile-help .tooltip-item {
  font-size: var(--v2-fs-xs);
  line-height: 1.5;
  color: var(--v2-text-2);
}
.v2-profile-pop .profile-help .tooltip-item + .tooltip-item {
  margin-top: 8px;
}
.v2-profile-pop .profile-help .tooltip-item strong {
  font-weight: var(--v2-fw-semibold);
  color: var(--v2-text-2);
}
.v2-profile-pop .profile-help .tooltip-item span {
  color: var(--v2-text-2);
}

/* Unify global dropdown popups (Select & Dropdown menu) */
.el-select-dropdown__item,
.el-dropdown-menu__item {
  font-family: var(--font-ui) !important;
  font-size: 13px !important;
  font-weight: var(--v2-fw-medium) !important;
  height: 32px !important;
  line-height: 32px !important;
  color: var(--v2-text-2) !important;
}

.el-select-dropdown__item {
  padding: 0 32px 0 16px !important;
}

.el-dropdown-menu__item {
  padding: 0 16px !important;
}

/* Selected item styling */
.el-select-dropdown__item.is-selected,
.el-dropdown-menu__item.selected,
.el-dropdown-menu__item.active {
  color: var(--v2-accent) !important;
  font-weight: var(--v2-fw-semibold) !important;
  background-color: transparent !important;
}

/* Hover and focus item styling */
.el-select-dropdown__item:hover:not(.is-disabled),
.el-dropdown-menu__item:hover:not(.is-disabled),
.el-select-dropdown__item.is-hovering,
.el-dropdown-menu__item:focus:not(.is-disabled) {
  background-color: var(--v2-surface-2) !important;
  color: var(--v2-text) !important;
}

</style>
