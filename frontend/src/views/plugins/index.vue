<template>
  <div>
    <div v-loading="loading || operating" class="v2-cardgrid">
      <div class="v2-addcard" @click="showAddDialog = true">
        <svg width="22" height="22" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>
        <span>添加插件</span>
      </div>
      <div v-for="plugin in plugins" :key="`${plugin.profile}/${plugin.name}`" class="v2-card v2-ccard">
        <div class="v2-ccard-head">
          <div class="v2-ccard-icon"><PluginIcon /></div>
          <div class="v2-ccard-tt">
            <div class="v2-ccard-name">
              {{ plugin.name }}
              <span class="v2-pill v2-pill-neutral mono sk-inline">{{ plugin.profile }}</span>
              <span v-if="plugin.version" class="v2-pill v2-pill-neutral mono sk-inline">{{ plugin.version }}</span>
            </div>
            <div class="v2-ccard-sub mono" :title="plugin.description || ''">{{ plugin.description || '—' }}</div>
          </div>
          <div class="v2-ccard-acts">
            <el-tooltip content="更新" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act" :disabled="operatingName === `${plugin.profile}/${plugin.name}`" @click="handleUpdate(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
            </el-tooltip>
            <el-tooltip content="卸载" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act danger" :disabled="operatingName === `${plugin.profile}/${plugin.name}`" @click="handleUninstall(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
            </el-tooltip>
          </div>
        </div>
        <div v-if="pluginAgent" class="v2-chip-row">
          <span class="v2-chip on pg-badge">
            <CliBrandIcon :type="pluginAgent.id" width="13" height="13" />
            {{ pluginAgent.name }} · 已激活
          </span>
        </div>
      </div>
    </div>

    <V2Drawer v-model="showAddDialog" title="添加插件" confirm-text="安装" @confirm="handleInstall">
      <div class="v2-field"><label class="v2-label">Profile <span class="req">*</span></label><input v-model="form.profile" class="v2-input mono" placeholder="例如：web"></div>
      <div class="v2-field"><label class="v2-label">仓库地址 <span class="req">*</span></label><input v-model="form.url" class="v2-input mono" placeholder="https://github.com/owner/repo 或 github:owner/repo"></div>
      <div class="v2-hint">一个仓库即一个插件，将通过 dsh 安装到指定 profile（需已安装 dsh 和 pnpm）；不存在的 profile 会由 dsh 自动初始化。</div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import { ElNotification } from 'element-plus'
import V2Drawer from '@/components/V2Drawer.vue'
import PluginIcon from '@/components/PluginIcon.vue'
import CliBrandIcon from '@/components/CliBrandIcon.vue'
import { useAgentStore } from '@/stores/agents'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { pluginsApi } from '@/api/plugins'
import { getErrorMessage } from '@/utils/error'
import type { PluginItem } from '@/types/models'

const agentStore = useAgentStore()
// dsh 插件装上即激活，无启停概念：徽标取启用插件功能的 Agent，仅作展示
const pluginAgent = computed(() => agentStore.agentsFor('plugins')[0])

const plugins = ref<PluginItem[]>([])
const loading = ref(false)
const operating = ref(false)
const operatingName = ref<string | null>(null)
const showAddDialog = ref(false)
const form = ref({ profile: 'web', url: '' })

function showCliOutput(output: string, isError = false) {
  if (!output) return
  ElNotification({ title: isError ? '操作失败' : '操作结果', message: output.replace(/\n/g, '<br/>'), type: isError ? 'error' : 'success', duration: 5000, position: 'top-right', dangerouslyUseHTMLString: true })
}

async function fetchInstalled() {
  loading.value = true
  try {
    plugins.value = await pluginsApi.getInstalled()
  } catch (error: any) {
    notify(getErrorMessage(error, '加载失败'), 'error')
  } finally {
    loading.value = false
  }
}

async function handleInstall() {
  const profile = form.value.profile.trim()
  const url = form.value.url.trim()
  if (!profile) {
    notify('请输入 Profile', 'error')
    return
  }
  if (!url) {
    notify('请输入仓库地址', 'error')
    return
  }
  showAddDialog.value = false
  operating.value = true
  try {
    const result = await pluginsApi.install(profile, url)
    showCliOutput(result.cli_output)
    if (!result.cli_output) notify('安装成功')
    form.value = { profile: 'web', url: '' }
    await fetchInstalled()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '安装失败'), true)
  } finally {
    operating.value = false
  }
}

async function handleUpdate(plugin: PluginItem) {
  operating.value = true
  operatingName.value = `${plugin.profile}/${plugin.name}`
  try {
    const result = await pluginsApi.update(plugin.profile, plugin.name)
    showCliOutput(result.cli_output)
    await fetchInstalled()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, '更新失败'), true)
  } finally {
    operating.value = false
    operatingName.value = null
  }
}

async function handleUninstall(plugin: PluginItem) {
  try {
    await confirm(`确定卸载插件 "${plugin.name}"?`, '确认卸载')
    operating.value = true
    operatingName.value = `${plugin.profile}/${plugin.name}`
    try {
      const result = await pluginsApi.uninstall(plugin.profile, plugin.name)
      showCliOutput(result.cli_output)
      await fetchInstalled()
    } catch (error: any) {
      showCliOutput(getErrorMessage(error, '卸载失败'), true)
    } finally {
      operating.value = false
      operatingName.value = null
    }
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') showCliOutput(getErrorMessage(error, '卸载失败'), true)
  }
}

onMounted(() => {
  fetchInstalled()
})
</script>

<style scoped>
.sk-inline { margin-left: 8px; }
.pg-badge { cursor: default; pointer-events: none; }
</style>
