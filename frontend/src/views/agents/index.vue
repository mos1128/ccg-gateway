<template>
  <div class="agent-page" v-loading="agentStore.loading">
    <section>
      <div class="section-head">
        <div>
          <h2>内置 Agent</h2>
          <span class="section-count mono">{{ agentStore.agents.length }}</span>
        </div>
        <el-tooltip content="刷新" placement="top" effect="light" :show-after="250">
          <button class="v2-row-act" type="button" @click="refresh">
            <el-icon><Refresh /></el-icon>
          </button>
        </el-tooltip>
      </div>

      <div class="agent-grid">
        <article v-for="agent in agentStore.agents" :key="agent.id" class="v2-card agent-card">
          <header class="agent-head">
            <div class="agent-title">
              <CliBrandIcon :type="agent.id" width="18" height="18" />
              <div>
                <h3>{{ agent.name }}</h3>
                <span class="mono agent-id">{{ agent.id }}</span>
              </div>
            </div>
            <span class="v2-pill v2-pill-neutral mono">schema {{ agent.schema_version }}</span>
          </header>

          <div class="agent-meta">
            <div class="meta-label">User-Agent</div>
            <div class="pill-row">
              <span v-for="pattern in agent.user_agent" :key="pattern" class="v2-pill v2-pill-neutral mono">{{ pattern }}</span>
            </div>
            <div class="meta-label">端点类型</div>
            <div class="pill-row">
              <span v-for="protocol in agent.protocols" :key="protocol" class="v2-pill v2-pill-info mono">{{ protocolLabel(protocol) }}</span>
            </div>
          </div>

          <div class="feature-list">
            <div v-for="feature in displayFeatures(agent)" :key="feature.key" class="feature-row">
              <div class="feature-main">
                <span class="feature-name">{{ feature.label }}</span>
              </div>

              <span class="v2-pill" :class="featureStatusClass(feature)">{{ featureStatusLabel(feature) }}</span>
            </div>
          </div>
        </article>
      </div>
    </section>

    <section v-if="agentStore.definitionErrors.length" class="v2-card diagnostic-block error-block">
      <div class="block-head">
        <h2>定义加载错误</h2>
        <span class="v2-pill v2-pill-danger mono">{{ agentStore.definitionErrors.length }}</span>
      </div>
      <div v-for="error in agentStore.definitionErrors" :key="error.source" class="error-row">
        <span class="mono">{{ error.source }}</span>
        <span>{{ error.message }}</span>
      </div>
    </section>

    <section class="v2-card diagnostic-block">
      <div class="block-head">
        <h2>最近未知 User-Agent</h2>
        <span class="v2-pill v2-pill-neutral mono">{{ unknownDiagnostics.length }}</span>
      </div>
      <V2Empty v-if="unknownDiagnostics.length === 0" title="暂无未知 Agent" />
      <div v-else class="diagnostic-table-wrap">
        <table class="v2-table diagnostic-table">
          <thead>
            <tr>
              <th>User-Agent</th>
              <th>最近出现</th>
              <th>次数</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in unknownDiagnostics" :key="item.id">
              <td class="mono ua-cell">{{ diagnosticUserAgent(item) }}</td>
              <td class="mono">{{ formatTime(item.last_seen) }}</td>
              <td class="mono">{{ item.occurrence_count }}</td>
              <td>
                <el-tooltip content="复制 User-Agent" placement="top" effect="light" :show-after="250">
                  <button class="v2-row-act" type="button" @click="copyUa(item)">
                    <el-icon><CopyDocument /></el-icon>
                  </button>
                </el-tooltip>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { CopyDocument, Refresh } from '@element-plus/icons-vue'
import CliBrandIcon from '@/components/CliBrandIcon.vue'
import V2Empty from '@/components/V2Empty.vue'
import { useAgentStore } from '@/stores/agents'
import { notify } from '@/utils/notification'
import type {
  AgentDiagnostic,
  AgentFeatureName,
  AgentInfo,
  Protocol,
} from '@/types/models'

const agentStore = useAgentStore()
const featureLabels: Record<AgentFeatureName, string> = {
  provider_config: 'Provider 配置',
  global_preset: '全局预设',
  profiles: 'Profile',
  official_login: '官方凭证',
  model_mapping: '模型映射',
  token_usage: 'Token 统计',
  skills: 'Skill',
  mcp: 'MCP',
  sessions: 'Session',
  plugins: 'Plugin',
  prompts: '提示词',
}
const featureKeys = Object.keys(featureLabels) as AgentFeatureName[]

interface DisplayFeature {
  key: AgentFeatureName
  label: string
  enabled: boolean
}

const unknownDiagnostics = computed(() =>
  agentStore.diagnostics.filter((item) => item.kind === 'unknown_agent'),
)

function protocolLabel(protocol: Protocol) {
  const labels: Record<Protocol, string> = {
    anthropic_messages: 'Anthropic Messages',
    openai_chat: 'OpenAI Chat',
    openai_responses: 'OpenAI Responses',
    gemini_generate_content: 'Gemini GenerateContent',
  }
  return labels[protocol]
}

function displayFeatures(agent: AgentInfo): DisplayFeature[] {
  return featureKeys.map((key) => ({
    key,
    label: featureLabels[key],
    enabled: agent.features[key].enabled,
  }))
}

function featureStatusLabel(feature: DisplayFeature) {
  return feature.enabled ? '可用' : '不可用'
}

function featureStatusClass(feature: DisplayFeature) {
  return feature.enabled ? 'v2-pill-success' : 'v2-pill-neutral'
}

function diagnosticUserAgent(item: AgentDiagnostic) {
  try {
    const payload = JSON.parse(item.payload_json) as { user_agent?: string }
    return payload.user_agent || item.key
  } catch {
    return item.key
  }
}

function formatTime(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString()
}

async function copyUa(item: AgentDiagnostic) {
  await navigator.clipboard.writeText(diagnosticUserAgent(item))
  notify('已复制 User-Agent')
}

async function refresh() {
  await Promise.all([agentStore.fetchAgents(), agentStore.fetchDiagnostics()])
}

onMounted(refresh)
</script>

<style scoped>
.agent-page { display: flex; flex-direction: column; gap: 20px; }
.section-head, .section-head > div, .block-head, .agent-head, .agent-title, .feature-row { display: flex; align-items: center; }
.section-head, .block-head, .agent-head, .feature-row { justify-content: space-between; }
.section-head { margin-bottom: 12px; }
.section-head h2, .block-head h2 { margin: 0; font-size: var(--v2-fs-base); font-weight: var(--v2-fw-semibold); }
.section-head > div { gap: 8px; }
.section-count { color: var(--v2-text-3); font-size: var(--v2-fs-xs); }
.agent-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(420px, 1fr)); gap: 14px; }
.agent-card { padding: 16px; min-width: 0; }
.agent-title { gap: 10px; min-width: 0; }
.agent-title h3 { margin: 0 0 2px; font-size: var(--v2-fs-base); font-weight: var(--v2-fw-semibold); }
.agent-id { color: var(--v2-text-3); font-size: 11px; }
.agent-meta { display: grid; grid-template-columns: 84px minmax(0, 1fr); gap: 9px 12px; padding: 14px 0; border-bottom: 1px solid var(--v2-surface-2); }
.meta-label { color: var(--v2-text-3); font-size: var(--v2-fs-xs); padding-top: 2px; }
.pill-row { display: flex; flex-wrap: wrap; gap: 5px; min-width: 0; }
.pill-row .v2-pill { max-width: 100%; overflow-wrap: anywhere; }
.feature-list { padding-top: 5px; }
.feature-row { min-height: 42px; gap: 12px; border-bottom: 1px solid var(--v2-surface-2); }
.feature-row:last-child { border-bottom: 0; }
.feature-main { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.feature-name { font-size: var(--v2-fs-sm); color: var(--v2-text); }
.diagnostic-block { overflow: hidden; }
.block-head { height: 48px; padding: 0 16px; border-bottom: 1px solid var(--v2-surface-2); }
.diagnostic-table-wrap { overflow-x: auto; }
.diagnostic-table th:last-child, .diagnostic-table td:last-child { width: 42px; padding-left: 4px; padding-right: 8px; }
.ua-cell { max-width: 680px; white-space: normal !important; overflow-wrap: anywhere; }
.error-row { display: grid; grid-template-columns: 180px 1fr; gap: 16px; padding: 11px 16px; border-bottom: 1px solid var(--v2-surface-2); color: var(--v2-danger); font-size: var(--v2-fs-sm); }
.error-row:last-child { border-bottom: 0; }
@media (max-width: 760px) {
  .agent-grid { grid-template-columns: 1fr; }
  .agent-meta { grid-template-columns: 1fr; gap: 5px; }
  .feature-row { padding: 10px 0; }
}
</style>
