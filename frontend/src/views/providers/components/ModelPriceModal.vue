<template>
  <V2Drawer v-model="visible" :title="t('providers.price.title')" width="72%" :show-footer="false">
    <div class="mp-toolbar">
      <div class="mp-status">
        <span :class="{ error: !!syncState?.last_error }">{{ statusText }}</span>
        <span v-if="syncState?.last_success_at" class="mp-status-time">{{ new Date(syncState.last_success_at * 1000).toLocaleString() }}</span>
      </div>
      <div class="mp-actions">
        <input v-model="keyword" class="v2-input mp-search" :placeholder="t('providers.price.searchPlaceholder')">
        <button class="v2-btn v2-btn-sm v2-btn-outline" :disabled="syncLoading" @click="emit('sync')">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/></svg>
          {{ syncLoading ? t('providers.price.syncing') : t('providers.price.sync') }}
        </button>
      </div>
    </div>

    <div class="mp-hint">{{ t('providers.price.hint') }}</div>

    <div v-loading="loading" class="mp-table-card">
      <div class="mp-table-scroll">
        <table class="v2-table">
          <thead>
            <tr>
              <th class="mp-col-name">{{ t('providers.price.colModel') }}</th>
              <th class="mp-col-src">{{ t('providers.price.colSource') }}</th>
              <th class="mp-col-tier">{{ t('providers.price.colThreshold') }}</th>
              <th class="mp-col-price">{{ t('providers.price.colInput') }}</th>
              <th class="mp-col-price">{{ t('providers.price.colOutput') }}</th>
              <th class="mp-col-price">{{ t('providers.price.colCacheRead') }}</th>
              <th class="mp-col-price">{{ t('providers.price.colCacheCreation') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in rows" :key="row.entry.model_key">
              <td class="mp-col-name mono"><OverflowText :text="row.entry.model_key" /></td>
              <td><OverflowText :text="row.entry.source_provider || '—'" /></td>
              <td class="mp-col-tier">
                <div v-for="line in row.lines" :key="line.label" class="mp-line" :class="{ 'mp-line-tier': line.tier }">{{ line.label }}</div>
              </td>
              <td class="mono">
                <div v-for="line in row.lines" :key="line.label" class="mp-line" :class="{ 'mp-line-tier': line.tier }">{{ formatPrice(line.input) }}</div>
              </td>
              <td class="mono">
                <div v-for="line in row.lines" :key="line.label" class="mp-line" :class="{ 'mp-line-tier': line.tier }">{{ formatPrice(line.output) }}</div>
              </td>
              <td class="mono">
                <div v-for="line in row.lines" :key="line.label" class="mp-line" :class="{ 'mp-line-tier': line.tier }">{{ formatPrice(line.cacheRead) }}</div>
              </td>
              <td class="mono">
                <div v-for="line in row.lines" :key="line.label" class="mp-line" :class="{ 'mp-line-tier': line.tier }">{{ formatPrice(line.cacheCreation) }}</div>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-if="!loading && !entries.length" class="mp-empty">
          {{ keyword.trim() ? t('providers.price.noMatch') : t('providers.price.emptyCatalog') }}
        </div>
      </div>
    </div>
  </V2Drawer>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import V2Drawer from '@/components/V2Drawer.vue'
import OverflowText from '@/components/OverflowText.vue'
import { providersApi } from '@/api/providers'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import type { ModelPriceCatalogEntry, ModelPriceTier, PriceSyncState } from '@/types/models'

const { t } = useI18n()
const CATALOG_LIMIT = 4000

interface PriceLine {
  label: string
  tier: boolean
  input: number
  output: number
  cacheRead: number
  cacheCreation: number
}

const props = defineProps<{
  modelValue: boolean
  syncState: PriceSyncState | null
  syncLoading?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  sync: []
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const entries = ref<ModelPriceCatalogEntry[]>([])
const loading = ref(false)
const keyword = ref('')
let searchTimer: ReturnType<typeof setTimeout> | null = null
let requestVersion = 0

const statusText = computed(() => {
  if (props.syncState?.last_error) return t('providers.price.lastSyncFailed', { error: props.syncState.last_error })
  const total = props.syncState?.model_count ?? 0
  const shown = entries.value.length
  if (!total) return t('providers.price.emptyStatus')
  return shown === total
    ? t('providers.price.total', { total })
    : t('providers.price.totalShown', { total, shown })
})

const rows = computed(() => entries.value.map((entry) => {
  const tiers = parseTiers(entry.tiers)
  const lines: PriceLine[] = [{
    label: tiers.length ? t('providers.price.baseline') : '—',
    tier: false,
    input: entry.input_price_per_m,
    output: entry.output_price_per_m,
    cacheRead: entry.cache_read_price_per_m,
    cacheCreation: entry.cache_creation_price_per_m
  }]
  for (const tier of tiers) {
    lines.push({
      label: `>${formatThreshold(tier.threshold_tokens)}`,
      tier: true,
      input: tier.input_price_per_m,
      output: tier.output_price_per_m,
      cacheRead: tier.cache_read_price_per_m,
      cacheCreation: tier.cache_creation_price_per_m
    })
  }
  return { entry, lines }
}))

function parseTiers(raw: string | null): ModelPriceTier[] {
  if (!raw) return []
  try {
    const parsed = JSON.parse(raw) as ModelPriceTier[]
    return parsed.slice().sort((a, b) => a.threshold_tokens - b.threshold_tokens)
  } catch {
    return []
  }
}

function formatThreshold(tokens: number) {
  return tokens >= 1000 ? `${Math.round(tokens / 1000)}K` : String(tokens)
}

function formatPrice(value: number) {
  if (!value) return '—'
  return `$${Number(value.toFixed(4))}`
}

async function loadCatalog() {
  const version = ++requestVersion
  loading.value = true
  try {
    const { data } = await providersApi.getPriceCatalog(keyword.value.trim(), CATALOG_LIMIT)
    if (version !== requestVersion) return
    entries.value = data
  } catch (e: any) {
    if (version === requestVersion) notify(getErrorMessage(e, t('providers.price.loadFailed')), 'error')
  } finally {
    if (version === requestVersion) loading.value = false
  }
}

watch(() => props.modelValue, (open) => {
  if (!open) return
  keyword.value = ''
  loadCatalog()
})
watch(keyword, () => {
  if (!props.modelValue) return
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(loadCatalog, 250)
})
watch(() => props.syncState?.updated_at, () => {
  if (props.modelValue) loadCatalog()
})
onUnmounted(() => {
  if (searchTimer) clearTimeout(searchTimer)
})
</script>

<style scoped>
.mp-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.mp-status { display: flex; flex-direction: column; gap: 2px; min-width: 0; font-size: var(--v2-fs-sm); color: var(--v2-text-2); }
.mp-status .error { color: var(--v2-danger); overflow-wrap: anywhere; }
.mp-status-time { font-size: var(--v2-fs-xs); color: var(--v2-text-3); }
.mp-actions { display: flex; align-items: center; gap: 8px; }
.mp-search { width: 200px; height: 28px; }
.mp-hint { margin: 10px 0 14px; font-size: var(--v2-fs-xs); color: var(--v2-text-3); line-height: 1.5; }
.mp-table-card { flex: 1; min-height: 120px; display: flex; flex-direction: column; overflow: hidden; border: 1px solid var(--v2-surface-3); border-radius: var(--v2-r); }
.mp-table-scroll { flex: 1; overflow: auto; }
.mp-table-scroll .v2-table { table-layout: fixed; }
.mp-table-scroll thead th { position: sticky; top: 0; z-index: 1; }
.mp-table-scroll .v2-table th,
.mp-table-scroll .v2-table td { padding: 12px 14px; text-align: center; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mp-table-scroll .v2-table .mp-col-name { text-align: left; }
.mp-table-scroll .v2-table .mp-col-src { width: 136px; }
.mp-table-scroll .v2-table .mp-col-tier { width: 92px; }
.mp-table-scroll .v2-table .mp-col-price { width: 100px; }
.mp-line + .mp-line { margin-top: 4px; }
.mp-line-tier { color: var(--v2-text-3); font-size: var(--v2-fs-xs); }
.mp-empty { padding: 28px 0; text-align: center; font-size: var(--v2-fs-sm); color: var(--v2-text-3); }
</style>
