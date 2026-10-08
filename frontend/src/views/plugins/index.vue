<template>
  <div>
    <div v-loading="loading || operating" class="v2-cardgrid">
      <div class="v2-addcard" @click="showAddDialog = true">
        <svg width="22" height="22" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>
        <span>{{ t('plugins.addPlugin') }}</span>
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
            <div class="v2-ccard-sub mono"><OverflowText :text="plugin.description || '—'" /></div>
          </div>
          <div class="v2-ccard-acts">
            <el-tooltip :content="t('plugins.update')" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act" :disabled="operatingName === `${plugin.profile}/${plugin.name}`" @click="handleUpdate(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg></button>
            </el-tooltip>
            <el-tooltip :content="t('plugins.uninstall')" placement="top" effect="light" :show-after="250">
              <button class="v2-row-act danger" :disabled="operatingName === `${plugin.profile}/${plugin.name}`" @click="handleUninstall(plugin)"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg></button>
            </el-tooltip>
          </div>
        </div>
        <div v-if="pluginAgent" class="v2-chip-row">
          <span class="v2-chip on pg-badge">
            <CliBrandIcon :type="pluginAgent.id" width="13" height="13" />
            {{ pluginAgent.name }} · {{ t('plugins.activated') }}
          </span>
        </div>
      </div>
    </div>

    <V2Drawer v-model="showAddDialog" :title="t('plugins.addPlugin')" :confirm-text="t('plugins.install')" @confirm="handleInstall">
      <div class="v2-field"><label class="v2-label">Profile <span class="req">*</span></label><input v-model="form.profile" class="v2-input mono" :placeholder="t('plugins.profilePlaceholder')"></div>
      <div class="v2-field"><label class="v2-label">{{ t('plugins.repoUrl') }} <span class="req">*</span></label><input v-model="form.url" class="v2-input mono" :placeholder="t('plugins.repoUrlPlaceholder')"></div>
      <div class="v2-hint">{{ t('plugins.installHint') }}</div>
    </V2Drawer>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { ElNotification } from 'element-plus'
import V2Drawer from '@/components/V2Drawer.vue'
import OverflowText from '@/components/OverflowText.vue'
import PluginIcon from '@/components/PluginIcon.vue'
import CliBrandIcon from '@/components/CliBrandIcon.vue'
import { useAgentStore } from '@/stores/agents'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { pluginsApi } from '@/api/plugins'
import { getErrorMessage } from '@/utils/error'
import type { PluginItem } from '@/types/models'

const { t } = useI18n()
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
  ElNotification({ title: isError ? t('common.operationFailed') : t('plugins.operationResult'), message: output.replace(/\n/g, '<br/>'), type: isError ? 'error' : 'success', duration: 5000, position: 'top-right', dangerouslyUseHTMLString: true })
}

async function fetchInstalled() {
  loading.value = true
  try {
    plugins.value = await pluginsApi.getInstalled()
  } catch (error: any) {
    notify(getErrorMessage(error, t('plugins.loadFailed')), 'error')
  } finally {
    loading.value = false
  }
}

async function handleInstall() {
  const profile = form.value.profile.trim()
  const url = form.value.url.trim()
  if (!profile) {
    notify(t('plugins.profileRequired'), 'error')
    return
  }
  if (!url) {
    notify(t('plugins.repoUrlRequired'), 'error')
    return
  }
  showAddDialog.value = false
  operating.value = true
  try {
    const result = await pluginsApi.install(profile, url)
    showCliOutput(result.cli_output)
    if (!result.cli_output) notify(t('plugins.installSuccess'))
    form.value = { profile: 'web', url: '' }
    await fetchInstalled()
  } catch (error: any) {
    showCliOutput(getErrorMessage(error, t('plugins.installFailed')), true)
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
    showCliOutput(getErrorMessage(error, t('plugins.updateFailed')), true)
  } finally {
    operating.value = false
    operatingName.value = null
  }
}

async function handleUninstall(plugin: PluginItem) {
  try {
    await confirm(t('plugins.uninstallConfirm', { name: plugin.name }), t('plugins.uninstallConfirmTitle'))
    operating.value = true
    operatingName.value = `${plugin.profile}/${plugin.name}`
    try {
      const result = await pluginsApi.uninstall(plugin.profile, plugin.name)
      showCliOutput(result.cli_output)
      await fetchInstalled()
    } catch (error: any) {
      showCliOutput(getErrorMessage(error, t('plugins.uninstallFailed')), true)
    } finally {
      operating.value = false
      operatingName.value = null
    }
  } catch (error: any) {
    if (error !== 'cancel' && error?.toString() !== 'cancel') showCliOutput(getErrorMessage(error, t('plugins.uninstallFailed')), true)
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
