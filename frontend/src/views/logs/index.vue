<template>
  <div class="logs-page">
    <!-- Settings Card -->
    <el-card class="settings-card">
      <template #header>
        <div class="card-header">
          <span>日志设置</span>
        </div>
      </template>
      <el-form :inline="true">
        <el-form-item label="启用日志记录">
          <el-switch v-model="logEnabled" @change="updateLogSettings" />
        </el-form-item>
        <el-form-item>
          <span class="tip">开启后记录请求/响应详情到数据库</span>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- Tabs -->
    <el-card style="margin-top: 20px">
      <el-tabs v-model="activeTab">
        <el-tab-pane label="请求日志" name="request">
          <!-- Filters -->
          <el-form :inline="true" class="filter-form">
            <el-form-item label="CLI类型">
              <el-select v-model="requestFilters.cli_type" clearable placeholder="全部" style="width: 130px">
                <el-option label="ClaudeCode" value="claude_code" />
                <el-option label="Codex" value="codex" />
                <el-option label="Gemini" value="gemini" />
              </el-select>
            </el-form-item>
            <el-form-item label="服务商">
              <el-select v-model="requestFilters.provider_name" clearable filterable placeholder="全部" style="width: 150px">
                <el-option v-for="p in providerOptions" :key="p" :label="p" :value="p" />
              </el-select>
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="requestFilters.success" clearable placeholder="全部" style="width: 100px">
                <el-option label="成功" :value="true" />
                <el-option label="失败" :value="false" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchRequestLogs">查询</el-button>
              <el-button @click="resetRequestFilters">重置</el-button>
              <el-button type="danger" @click="clearRequestLogs">清空日志</el-button>
            </el-form-item>
          </el-form>

          <!-- Table -->
          <el-table :data="requestLogs" v-loading="requestLoading" stripe>
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column label="时间" width="170">
              <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
            </el-table-column>
            <el-table-column prop="cli_type" label="CLI" width="130" />
            <el-table-column prop="provider_name" label="服务商" width="150" show-overflow-tooltip />
            <el-table-column prop="model_id" label="模型" width="220" show-overflow-tooltip />
            <el-table-column label="状态" width="80">
              <template #default="{ row }">
                <el-tag :type="row.success ? 'success' : 'danger'" size="small">
                  {{ row.success ? '成功' : '失败' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status_code" label="状态码" width="80" />
            <el-table-column label="耗时" width="90">
              <template #default="{ row }">{{ row.elapsed_ms }}ms</template>
            </el-table-column>
            <el-table-column prop="client_method" label="方法" width="70" />
            <el-table-column prop="client_path" label="路径" show-overflow-tooltip />
            <el-table-column label="操作" width="80" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link @click="showRequestDetail(row.id)">详情</el-button>
              </template>
            </el-table-column>
          </el-table>

          <!-- Pagination -->
          <div class="pagination-wrapper">
            <span class="total-text">总数量 {{ requestTotal }}</span>
            <el-pagination
              v-model:current-page="requestPage"
              v-model:page-size="requestPageSize"
              :total="requestTotal"
              :page-sizes="[20, 50, 100]"
              layout="sizes, prev, pager, next"
              @size-change="fetchRequestLogs"
              @current-change="fetchRequestLogs"
            />
          </div>
        </el-tab-pane>

        <el-tab-pane label="系统日志" name="system">
          <!-- Filters -->
          <el-form :inline="true" class="filter-form">
            <el-form-item label="级别">
              <el-select v-model="systemFilters.level" clearable placeholder="全部" style="width: 100px">
                <el-option label="INFO" value="INFO" />
                <el-option label="WARN" value="WARN" />
                <el-option label="ERROR" value="ERROR" />
              </el-select>
            </el-form-item>
            <el-form-item label="事件类型">
              <el-select v-model="systemFilters.event_type" clearable placeholder="全部" style="width: 150px">
                <el-option label="服务商失败" value="服务商失败" />
                <el-option label="服务商黑名单" value="服务商黑名单" />
                <el-option label="服务商切换" value="服务商切换" />
                <el-option label="失败重置" value="失败重置" />
              </el-select>
            </el-form-item>
            <el-form-item label="服务商">
              <el-select v-model="systemFilters.provider_name" clearable filterable placeholder="全部" style="width: 150px">
                <el-option v-for="p in providerOptions" :key="p" :label="p" :value="p" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchSystemLogs">查询</el-button>
              <el-button @click="resetSystemFilters">重置</el-button>
              <el-button type="danger" @click="clearSystemLogs">清空日志</el-button>
            </el-form-item>
          </el-form>

          <!-- Table -->
          <el-table :data="systemLogs" v-loading="systemLoading" stripe>
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column label="时间" width="170">
              <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
            </el-table-column>
            <el-table-column label="级别" width="80">
              <template #default="{ row }">
                <el-tag :type="getLevelType(row.level)" size="small">{{ row.level }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="event_type" label="事件类型" width="130" />
            <el-table-column prop="provider_name" label="服务商" width="150" show-overflow-tooltip />
            <el-table-column prop="message" label="消息" show-overflow-tooltip />
            <el-table-column label="详情" width="80">
              <template #default="{ row }">
                <el-button v-if="row.details" type="primary" link @click="showSystemDetail(row)">查看</el-button>
              </template>
            </el-table-column>
          </el-table>

          <!-- Pagination -->
          <div class="pagination-wrapper">
            <span class="total-text">总数量 {{ systemTotal }}</span>
            <el-pagination
              v-model:current-page="systemPage"
              v-model:page-size="systemPageSize"
              :total="systemTotal"
              :page-sizes="[20, 50, 100]"
              layout="sizes, prev, pager, next"
              @size-change="fetchSystemLogs"
              @current-change="fetchSystemLogs"
            />
          </div>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- Request Detail Dialog -->
    <el-dialog v-model="requestDetailVisible" title="请求详情" width="900px" destroy-on-close>
      <div v-if="requestDetail" class="detail-content">
        <!-- Summary -->
        <el-descriptions :column="3" border size="small">
          <el-descriptions-item label="ID">{{ requestDetail.id }}</el-descriptions-item>
          <el-descriptions-item label="时间">{{ formatTime(requestDetail.created_at) }}</el-descriptions-item>
          <el-descriptions-item label="耗时">{{ requestDetail.elapsed_ms }}ms</el-descriptions-item>
          <el-descriptions-item label="CLI类型">{{ requestDetail.cli_type }}</el-descriptions-item>
          <el-descriptions-item label="服务商">{{ requestDetail.provider_name }}</el-descriptions-item>
          <el-descriptions-item label="模型">{{ requestDetail.model_id || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="requestDetail.success ? 'success' : 'danger'" size="small">
              {{ requestDetail.success ? '成功' : '失败' }}
            </el-tag>
          </el-descriptions-item>
        </el-descriptions>

        <!-- Error Message -->
        <el-alert v-if="requestDetail.error_message" :title="requestDetail.error_message" type="error" :closable="false" style="margin-top: 16px" />

        <!-- Request/Response Cards -->
        <div class="cards-container">
          <!-- CLI Request -->
          <el-card class="detail-card" shadow="hover">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">CLI 请求</span>
                <el-tag size="small" type="info">{{ requestDetail.client_method }}</el-tag>
              </div>
            </template>
            <div class="url-line">{{ getFullClientUrl() }}</div>
            <el-collapse>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Headers</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.client_headers)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.client_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Body</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.client_body)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.client_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>

          <!-- Gateway Forward Request -->
          <el-card class="detail-card" shadow="hover">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">网关转发请求</span>
                <el-tag size="small" type="info">{{ requestDetail.client_method }}</el-tag>
              </div>
            </template>
            <div class="url-line">{{ requestDetail.forward_url }}</div>
            <el-collapse>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Headers</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.forward_headers)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.forward_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Body</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.forward_body)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.forward_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>

          <!-- Provider Response -->
          <el-card class="detail-card" shadow="hover">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">服务商响应</span>
                <el-tag size="small" :type="getStatusType(requestDetail.provider_status)">
                  {{ requestDetail.provider_status || '-' }}
                </el-tag>
              </div>
            </template>
            <el-collapse>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Headers</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.provider_headers)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.provider_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Body</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.provider_body)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.provider_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>

          <!-- Gateway Forward Response -->
          <el-card class="detail-card" shadow="hover">
            <template #header>
              <div class="detail-card-header">
                <span class="card-title">网关转发响应</span>
                <el-tag size="small" :type="getStatusType(requestDetail.response_status)">
                  {{ requestDetail.response_status || '-' }}
                </el-tag>
              </div>
            </template>
            <el-collapse>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Headers</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.response_headers)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.response_headers) }}</pre>
              </el-collapse-item>
              <el-collapse-item>
                <template #title>
                  <div class="collapse-title">
                    <span>Body</span>
                    <el-button :icon="CopyDocument" size="small" text @click.stop="handleCopy(requestDetail.response_body)" />
                  </div>
                </template>
                <pre class="code-block">{{ formatJson(requestDetail.response_body) }}</pre>
              </el-collapse-item>
            </el-collapse>
          </el-card>
        </div>
      </div>
    </el-dialog>

    <!-- System Detail Dialog -->
    <el-dialog v-model="systemDetailVisible" title="详情" width="600px">
      <pre class="code-block">{{ systemDetailContent }}</pre>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { CopyDocument } from '@element-plus/icons-vue'
import { logsApi } from '@/api/logs'
import { providersApi } from '@/api/providers'
import type { RequestLogListItem, RequestLogDetail, SystemLogItem } from '@/types/models'

const activeTab = ref('request')
const logEnabled = ref(false)
const providerOptions = ref<string[]>([])

// Request logs
const requestLogs = ref<RequestLogListItem[]>([])
const requestLoading = ref(false)
const requestPage = ref(1)
const requestPageSize = ref(20)
const requestTotal = ref(0)
const requestFilters = ref({
  cli_type: '',
  provider_name: '',
  success: undefined as boolean | undefined
})
const requestDetailVisible = ref(false)
const requestDetail = ref<RequestLogDetail | null>(null)

// System logs
const systemLogs = ref<SystemLogItem[]>([])
const systemLoading = ref(false)
const systemPage = ref(1)
const systemPageSize = ref(20)
const systemTotal = ref(0)
const systemFilters = ref({
  level: '',
  event_type: '',
  provider_name: ''
})
const systemDetailVisible = ref(false)
const systemDetailContent = ref('')

async function fetchProviders() {
  try {
    const res = await providersApi.list()
    const names = new Set<string>()
    res.data.forEach((p: any) => names.add(p.name))
    providerOptions.value = Array.from(names)
  } catch {}
}

async function fetchLogSettings() {
  try {
    const res = await logsApi.getSettings()
    logEnabled.value = res.data.debug_log
  } catch {}
}

async function updateLogSettings() {
  try {
    await logsApi.updateSettings({ debug_log: logEnabled.value })
    ElMessage.success('日志设置已更新')
  } catch {}
}

async function fetchRequestLogs() {
  requestLoading.value = true
  try {
    const params: any = {
      page: requestPage.value,
      page_size: requestPageSize.value
    }
    if (requestFilters.value.cli_type) params.cli_type = requestFilters.value.cli_type
    if (requestFilters.value.provider_name) params.provider_name = requestFilters.value.provider_name
    if (requestFilters.value.success !== undefined) params.success = requestFilters.value.success

    const res = await logsApi.listRequestLogs(params)
    requestLogs.value = res.data.items
    requestTotal.value = res.data.total
  } finally {
    requestLoading.value = false
  }
}

function resetRequestFilters() {
  requestFilters.value = { cli_type: '', provider_name: '', success: undefined }
  requestPage.value = 1
  fetchRequestLogs()
}

async function clearRequestLogs() {
  try {
    await ElMessageBox.confirm('确定要清空所有请求日志吗？', '确认', { type: 'warning' })
    await logsApi.clearRequestLogs()
    ElMessage.success('请求日志已清空')
    fetchRequestLogs()
  } catch {}
}

async function showRequestDetail(id: number) {
  try {
    const res = await logsApi.getRequestLog(id)
    requestDetail.value = res.data
    requestDetailVisible.value = true
  } catch {}
}

async function fetchSystemLogs() {
  systemLoading.value = true
  try {
    const params: any = {
      page: systemPage.value,
      page_size: systemPageSize.value
    }
    if (systemFilters.value.level) params.level = systemFilters.value.level
    if (systemFilters.value.event_type) params.event_type = systemFilters.value.event_type
    if (systemFilters.value.provider_name) params.provider_name = systemFilters.value.provider_name

    const res = await logsApi.listSystemLogs(params)
    systemLogs.value = res.data.items
    systemTotal.value = res.data.total
  } finally {
    systemLoading.value = false
  }
}

function resetSystemFilters() {
  systemFilters.value = { level: '', event_type: '', provider_name: '' }
  systemPage.value = 1
  fetchSystemLogs()
}

async function clearSystemLogs() {
  try {
    await ElMessageBox.confirm('确定要清空所有系统日志吗？', '确认', { type: 'warning' })
    await logsApi.clearSystemLogs()
    ElMessage.success('系统日志已清空')
    fetchSystemLogs()
  } catch {}
}

function showSystemDetail(row: SystemLogItem) {
  systemDetailContent.value = formatJson(row.details)
  systemDetailVisible.value = true
}

function formatTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString('zh-CN')
}

function formatJson(str: string | null): string {
  if (!str) return ''
  try {
    return JSON.stringify(JSON.parse(str), null, 2)
  } catch {
    return str
  }
}

function getLevelType(level: string): string {
  switch (level) {
    case 'ERROR': return 'danger'
    case 'WARN': return 'warning'
    default: return 'info'
  }
}

function getStatusType(status: number | null): string {
  if (!status) return 'info'
  if (status >= 200 && status < 300) return 'success'
  if (status >= 400) return 'danger'
  return 'warning'
}

function getFullClientUrl(): string {
  if (!requestDetail.value) return ''
  const path = requestDetail.value.client_path
  return `http://localhost:7788/${path.startsWith('/') ? path.slice(1) : path}`
}

async function handleCopy(content: string | null) {
  if (!content) return
  try {
    await navigator.clipboard.writeText(formatJson(content))
    ElMessage.success('已复制')
  } catch {
    ElMessage.error('复制失败')
  }
}

watch(activeTab, (tab) => {
  if (tab === 'request') fetchRequestLogs()
  else fetchSystemLogs()
})

onMounted(() => {
  fetchLogSettings()
  fetchProviders()
  fetchRequestLogs()
})
</script>

<style scoped>
.settings-card {
  margin-bottom: 0;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.tip {
  color: #909399;
  font-size: 12px;
}
.filter-form {
  margin-bottom: 16px;
}
.pagination-wrapper {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
}
.total-text {
  color: #606266;
  font-size: 13px;
}
.detail-content {
  max-height: 70vh;
  overflow-y: auto;
  padding-right: 12px;
  box-sizing: border-box;
}
.cards-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-top: 16px;
}
.detail-card {
  margin: 0;
}
.detail-card :deep(.el-card__header) {
  padding: 12px 16px;
  background: #f5f7fa;
}
.detail-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.card-title {
  font-weight: 500;
  font-size: 14px;
}
.url-line {
  font-family: monospace;
  font-size: 12px;
  color: #409eff;
  word-break: break-all;
  margin-bottom: 12px;
  padding: 8px;
  background: #f0f9ff;
  border-radius: 4px;
}
.code-block {
  background: #f5f7fa;
  padding: 12px;
  border-radius: 4px;
  font-family: monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow-y: auto;
  margin: 0;
}
.collapse-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding-right: 8px;
}
</style>
