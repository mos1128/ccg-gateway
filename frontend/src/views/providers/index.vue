<template>
  <div class="providers-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-refresh" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
      </defs>
    </svg>
    
    <!-- Tab Headers (Top Line) -->
    <div style="display: flex; gap: 32px; border-bottom: 1px solid rgba(226, 232, 240, 0.6); margin-bottom: 24px; padding-top: 8px;">
      <div 
        v-for="cli in [{id: 'claude_code', label: 'Claude Code'}, {id: 'codex', label: 'Codex'}, {id: 'gemini', label: 'Gemini'}]"
        :key="cli.id"
        @click="activeCliType = cli.id"
        :style="activeCliType === cli.id ? 'padding-bottom: 12px; color: #0f172a; font-weight: 600; font-size: 15px; border-bottom: 2px solid #0f172a; cursor: pointer;' : 'padding-bottom: 12px; color: #94a3b8; font-weight: 500; font-size: 15px; cursor: pointer;'"
      >
        {{ cli.label }}
      </div>
    </div>

    <!-- Page Header & Segmented Control -->
    <div class="page-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
      <div class="b-segmented">
        <div class="b-seg-btn" :class="{ active: viewMode === 'proxy' }" @click="viewMode = 'proxy'">中转模式</div>
        <div class="b-seg-btn" :class="{ active: viewMode === 'direct' }" @click="handleSwitchDirect">官方模式</div>
      </div>
      
      <button 
        v-if="viewMode === 'proxy'" 
        class="b-button" 
        style="padding: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center; transform: translateY(-2px); box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3);" 
        @click="showAddDialog = true"
        title="添加服务商"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </svg>
      </button>
      <button 
        v-else 
        class="b-button" 
        style="padding: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center; transform: translateY(-2px); box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3);" 
        @click="showAddCredentialDialog = true"
        title="添加凭证"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/><path d="M12 5v14"/>
        </svg>
      </button>
    </div>

    <!-- PROXY MODE LIST -->
    <div v-if="viewMode === 'proxy'" class="b-card" style="padding: 0; overflow: hidden;" v-loading="providerStore.loading">
      <div v-if="providerStore.providers.length === 0" style="padding: 40px; text-align: center; color: #94a3b8;">
        暂无服务商
      </div>
      
      <draggable
        v-model="providerStore.providers"
        item-key="id"
        handle=".drag-handle"
        @end="handleDragEnd"
      >
        <template #item="{ element }">
          <div :style="{
            padding: '24px',
            borderBottom: '1px solid #f1f5f9',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            background: element.is_blacklisted ? 'rgba(244, 63, 94, 0.02)' : '#ffffff'
          }">
            <div style="display: flex; align-items: center; gap: 16px; flex: 1; min-width: 0;">
              <div class="drag-handle" aria-label="拖拽排序" style="flex-shrink: 0;">
                 <div class="drag-dot"></div><div class="drag-dot"></div><div class="drag-dot"></div>
              </div>
              
              <div style="flex: 1; min-width: 0;">
                <div style="display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                  <div style="font-weight: 500; font-size: 16px; white-space: nowrap;" :style="{ color: !element.enabled ? '#94a3b8' : '#0f172a' }">
                    {{ element.name }}
                  </div>
                  <div v-if="element.is_blacklisted" class="tag" style="background: rgba(244, 63, 94, 0.1); color: #f43f5e; white-space: nowrap;">
                    {{ getUnblacklistTime(element) }}
                  </div>
                  <div v-else-if="!element.enabled" class="tag" style="background: #f1f5f9; color: #64748b; white-space: nowrap;">
                    已禁用
                  </div>
                  <div v-if="element.model_maps.length > 0" class="tag" style="background: rgba(16, 185, 129, 0.1); color: #10b981; white-space: nowrap;">
                    {{ element.model_maps.length }}个模型映射
                  </div>
                  <div v-if="element.model_blacklist && element.model_blacklist.length > 0" class="tag" style="background: rgba(245, 158, 11, 0.1); color: #f59e0b; white-space: nowrap;">
                    {{ element.model_blacklist.length }}个黑名单配置
                  </div>
                </div>
              </div>
            </div>
            
            <div style="display: flex; align-items: center; gap: 40px; flex-shrink: 0; margin-left: 24px;">
               <div style="display: flex; gap: 24px;">
                 <div style="display: flex; flex-direction: column; align-items: center; min-width: 50px;">
                   <div style="font-size: 12px; margin-bottom: 2px; white-space: nowrap;" :style="{ color: element.consecutive_failures >= element.failure_threshold ? '#ef4444' : '#94a3b8' }">失败次数</div>
                   <div :style="{ color: element.consecutive_failures >= element.failure_threshold ? '#ef4444' : '#0f172a', fontWeight: 500, fontSize: '15px' }">
                     {{ element.consecutive_failures }}
                   </div>
                 </div>
                 <div style="display: flex; flex-direction: column; align-items: center; min-width: 50px;">
                   <div style="font-size: 12px; color: #94a3b8; margin-bottom: 2px; white-space: nowrap;">失败阈值</div>
                   <div style="color: #64748b; font-weight: 500; font-size: 15px;">{{ element.failure_threshold }}</div>
                 </div>
               </div>
               
               <div style="display: flex; align-items: center; gap: 24px;">
                 <el-switch v-model="element.enabled" @change="handleToggle(element)" />
                 
                 <div style="display: flex; align-items: center; gap: 8px;">
                   <div class="action-icon" @click="handleEdit(element)" title="编辑">
                     <svg width="18" height="18"><use href="#icon-edit"/></svg>
                   </div>
                   
                   <div class="action-icon" @click="handleReset(element)" title="重置并解除拉黑">
                     <svg width="18" height="18"><use href="#icon-refresh"/></svg>
                   </div>

                   <div class="action-icon delete" @click="handleCommand('delete', element)" title="删除">
                     <svg width="18" height="18"><use href="#icon-trash"/></svg>
                   </div>
                 </div>
               </div>
            </div>
          </div>
        </template>
      </draggable>
    </div>

    <!-- DIRECT MODE -->
    <div v-else class="b-card" style="padding: 0; overflow: hidden;" v-loading="credentialStore.loading">
      <div v-if="credentialStore.credentials.length === 0" style="padding: 40px; text-align: center; color: #94a3b8;">
        暂无凭证
      </div>
      
      <draggable
        v-else
        v-model="credentialStore.credentials"
        item-key="id"
        handle=".drag-handle"
        @end="handleCredentialDragEnd"
      >
        <template #item="{ element }">
          <div style="padding: 24px; border-bottom: 1px solid #f1f5f9; display: flex; justify-content: space-between; align-items: center; background: #ffffff;">
            <div style="display: flex; align-items: center; gap: 16px;">
              <div class="drag-handle" aria-label="拖拽排序">
                 <div class="drag-dot"></div><div class="drag-dot"></div><div class="drag-dot"></div>
              </div>
              
              <div>
                <div style="display: flex; align-items: center; gap: 12px;">
                  <div style="font-weight: 500; font-size: 16px; color: #0f172a;">{{ element.name }}</div>
                  <div v-if="element.is_active" class="tag" style="background: rgba(16, 185, 129, 0.1); color: #10b981;">激活中</div>
                </div>
              </div>
            </div>
            
            <div style="display: flex; align-items: center; gap: 12px;">
               <div class="action-icon" @click="handleEditCredential(element)" title="编辑">
                 <svg width="18" height="18"><use href="#icon-edit"/></svg>
               </div>
               <div class="action-icon delete" @click="handleDeleteCredential(element)" title="删除">
                 <svg width="18" height="18"><use href="#icon-trash"/></svg>
               </div>
            </div>
          </div>
        </template>
      </draggable>
    </div>

    <!-- Add/Edit Provider Modal -->
    <AppModal v-model="showDialog" :title="editingProvider ? '编辑服务商' : '添加服务商'" width="720px" :show-footer="false">
      <div style="display: flex; gap: 32px; margin-bottom: 32px;">
            <div style="flex: 1;">
              <label class="c-label">服务商名称 <span style="color: #ef4444;">*</span></label>
              <input type="text" v-model="form.name" class="c-input" placeholder="例如: OpenAI 官方">
            </div>
            <div style="flex: 1;">
              <label class="c-label">Base URL <span style="color: #ef4444;">*</span></label>
              <input type="text" v-model="form.base_url" class="c-input" :placeholder="baseUrlPlaceholder">
            </div>
          </div>
          
          <div style="margin-bottom: 40px;">
            <label class="c-label">{{ activeCliType === 'claude_code' ? 'API Token' : 'API Key' }} <span style="color: #ef4444;">*</span></label>
            <input type="password" v-model="form.api_key" class="c-input" placeholder="sk-...">
          </div>

          <!-- Advanced Params -->
          <div style="display: flex; gap: 32px; margin-bottom: 40px; padding: 32px 24px; background: #f8fafc; border-radius: 12px; border: 1px solid #f1f5f9;">
            <div style="flex: 1;">
              <label class="c-label">失败鉴权阈值 (次)</label>
              <input type="number" v-model.number="form.failure_threshold" class="c-input">
              <div style="font-size: 11px; color: #94a3b8; margin-top: 10px;">连续失败次数达到此值后拉黑。</div>
            </div>
            <div style="flex: 1;">
              <label class="c-label">拉黑时长 (分钟)</label>
              <input type="number" v-model.number="form.blacklist_minutes" class="c-input">
              <div style="font-size: 11px; color: #94a3b8; margin-top: 10px;">被拉黑后这段时间不再转发请求。</div>
            </div>
            <div style="flex: 1;">
              <label class="c-label">自定义 UA (选填)</label>
              <input type="text" v-model="form.custom_useragent" class="c-input" placeholder="留空则使用原始">
              <div style="font-size: 11px; color: #94a3b8; margin-top: 10px;">强制替换转发请求的 User-Agent。</div>
            </div>
          </div>

          <!-- Model Maps Section -->
          <div style="margin-bottom: 40px;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
              <div>
                <!-- FONT-WEIGHT REDUCED TO 500 TO PREVENT BLURRINESS -->
                <div style="font-weight: 500; font-size: 15px; color: #0f172a;">模型转发配置 (映射)</div>
                <div style="font-size: 12px; color: #64748b; margin-top: 6px;">将 CLI 请求的源模型名称，转译为该服务商真正的目标模型名称。</div>
              </div>
              <button class="b-button-outline" style="font-size: 13px; padding: 6px 12px;" @click="addModelMap">+ 添加映射</button>
            </div>
            
            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div v-for="(map, index) in form.model_maps" :key="'map-'+index" style="display: flex; gap: 16px; align-items: center;">
                <input type="text" v-model="map.source_model" class="c-input" placeholder="CLI 源模型" style="flex: 1;">
                <div style="color: #cbd5e1; font-weight: 500;">→</div>
                <input type="text" v-model="map.target_model" class="c-input" placeholder="服务商模型" style="flex: 1;">
                <div class="b-button-icon" @click="removeModelMap(index)">×</div>
              </div>
            </div>
          </div>

          <!-- Model Blacklist Section -->
          <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
              <div>
                <!-- FONT-WEIGHT REDUCED TO 500 TO PREVENT BLURRINESS -->
                <div style="font-weight: 500; font-size: 15px; color: #0f172a;">模型黑名单</div>
                <div style="font-size: 12px; color: #64748b; margin-top: 6px;">配置该服务商不支持的模型正则/通配符 (如: claude-opus-*)。</div>
              </div>
              <button class="b-button-outline" style="font-size: 13px; padding: 6px 12px;" @click="addModelBlacklist">+ 加黑名单</button>
            </div>

            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div v-for="(item, index) in form.model_blacklist" :key="'blk-'+index" style="display: flex; gap: 16px; align-items: center;">
                 <input type="text" v-model="item.model_pattern" class="c-input" placeholder="模型规则" style="flex: 1;">
                 <div class="b-button-icon" @click="removeModelBlacklist(index)">×</div>
              </div>
            </div>
          </div>

      <template #footer>
        <button class="b-button-outline" @click="showDialog = false">取消修改</button>
        <button class="b-button" @click="handleSave">保存配置</button>
      </template>
    </AppModal>
    <!-- / Add Provider Modal -->

    <!-- Add/Edit Credential Modal -->
    <AppModal v-model="showCredentialDialog" :title="editingCredential ? '编辑凭证' : '添加凭证'" width="720px" :show-footer="false">
          <div style="margin-bottom: 32px;">
            <label class="c-label">凭证名称 <span style="color: #ef4444;">*</span></label>
            <input type="text" v-model="credentialForm.name" class="c-input" placeholder="例如: 个人主账号">
          </div>

          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
            <div style="font-weight: 500; font-size: 15px; color: #0f172a;">配置文件</div>
            <button class="b-button-outline" style="font-size: 13px; padding: 6px 12px;" @click="handleReadFromCli">读取当前 CLI 配置</button>
          </div>

          <template v-if="activeCliType === 'claude_code'">
             <div style="margin-bottom: 24px;">
               <div style="font-size: 12px; color: #64748b; margin-bottom: 8px;">~/.claude/settings.json</div>
               <el-input type="textarea" :rows="10" v-model="credentialForm.claude_settings" placeholder='{"ANTHROPIC_API_KEY": "..."}' />
             </div>
          </template>

          <template v-if="activeCliType === 'codex'">
            <div style="margin-bottom: 24px;">
               <div style="font-size: 12px; color: #64748b; margin-bottom: 8px;">~/.codex/auth.json</div>
               <el-input type="textarea" :rows="10" v-model="credentialForm.codex_auth" />
             </div>
          </template>

          <template v-if="activeCliType === 'gemini'">
             <div style="margin-bottom: 24px;">
               <div style="font-size: 12px; color: #64748b; margin-bottom: 8px;">~/.gemini/oauth_creds.json</div>
               <el-input type="textarea" :rows="4" v-model="credentialForm.gemini_oauth" />
             </div>
             <div style="margin-bottom: 24px;">
               <div style="font-size: 12px; color: #64748b; margin-bottom: 8px;">~/.gemini/google_accounts.json</div>
               <el-input type="textarea" :rows="3" v-model="credentialForm.gemini_accounts" />
             </div>
             <div style="margin-bottom: 24px;">
               <div style="font-size: 12px; color: #64748b; margin-bottom: 8px;">~/.gemini/settings.json</div>
               <el-input type="textarea" :rows="4" v-model="credentialForm.gemini_settings" />
             </div>
          </template>

      <template #footer>
        <button class="b-button-outline" @click="showCredentialDialog = false">取消</button>
        <button class="b-button" @click="handleSaveCredential">保存凭证</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import { notify } from '@/utils/notification'
import draggable from 'vuedraggable'
import AppModal from '@/components/AppModal.vue'
import { useProviderStore } from '@/stores/providers'
import { useCredentialStore } from '@/stores/credentials'
import { useUiStore } from '@/stores/ui'
import { credentialsApi } from '@/api/credentials'
import type { Provider, ModelMap, ModelBlacklist, CliType, OfficialCredential, OfficialCredentialCreate } from '@/types/models'

const providerStore = useProviderStore()
const credentialStore = useCredentialStore()
const uiStore = useUiStore()

const activeCliType = computed({
  get: () => uiStore.providersActiveCliType,
  set: (val) => uiStore.setProvidersActiveCliType(val)
})

const viewMode = ref<'proxy' | 'direct'>('proxy')

function handleSwitchDirect() {
  if (activeCliType.value === 'claude_code') {
    notify('Claude Code 暂未实现官方模式功能', 'warning')
    return
  }
  viewMode.value = 'direct'
}

const showAddDialog = ref(false)
const showAddCredentialDialog = ref(false)
const editingProvider = ref<Provider | null>(null)
const editingCredential = ref<OfficialCredential | null>(null)

const showDialog = computed({
  get: () => showAddDialog.value || !!editingProvider.value,
  set: (val) => {
    if (!val) {
      showAddDialog.value = false
      editingProvider.value = null
    }
  }
})

const showCredentialDialog = computed({
  get: () => showAddCredentialDialog.value || !!editingCredential.value,
  set: (val) => {
    if (!val) {
      showAddCredentialDialog.value = false
      editingCredential.value = null
    }
  }
})

interface FormModelMap { source_model: string; target_model: string; enabled: boolean }
interface FormModelBlacklist { model_pattern: string }

const form = ref({
  name: '',
  base_url: '',
  api_key: '',
  failure_threshold: 3,
  blacklist_minutes: 10,
  custom_useragent: '',
  model_maps: [] as FormModelMap[],
  model_blacklist: [] as FormModelBlacklist[]
})

const credentialForm = ref({
  name: '',
  claude_settings: '',
  codex_auth: '',
  gemini_oauth: '',
  gemini_accounts: '',
  gemini_settings: ''
})

const baseUrlPlaceholder = computed(() => {
  if (activeCliType.value === 'codex') return 'https://api.example.com/v1'
  return 'https://api.example.com'
})

function resetForm() {
  form.value = {
    name: '', base_url: '', api_key: '', failure_threshold: 3, blacklist_minutes: 10,
    custom_useragent: '', model_maps: [], model_blacklist: []
  }
}
function resetCredentialForm() {
  credentialForm.value = { name: '', claude_settings: '', codex_auth: '', gemini_oauth: '', gemini_accounts: '', gemini_settings: '' }
}

function addModelMap() { form.value.model_maps.push({ source_model: '', target_model: '', enabled: true }) }
function removeModelMap(index: number) { form.value.model_maps.splice(index, 1) }
function addModelBlacklist() { form.value.model_blacklist.push({ model_pattern: '' }) }
function removeModelBlacklist(index: number) { form.value.model_blacklist.splice(index, 1) }

// Listen for tab changes
watch(() => activeCliType.value, (cliType) => {
  providerStore.fetchProviders(cliType as CliType)
  credentialStore.fetchCredentials(cliType as CliType)
})

function handleEdit(provider: Provider) {
  editingProvider.value = provider
  form.value = {
    name: provider.name, base_url: provider.base_url, api_key: provider.api_key,
    failure_threshold: provider.failure_threshold, blacklist_minutes: provider.blacklist_minutes,
    custom_useragent: provider.custom_useragent || '',
    model_maps: provider.model_maps.map(m => ({ ...m })),
    model_blacklist: provider.model_blacklist.map(b => ({ ...b }))
  }
}

async function handleSave() {
  if (!form.value.name.trim() || !form.value.base_url.trim() || !form.value.api_key.trim()) {
    notify('请填写完整的必填项', 'error')
    return
  }
  const data = {
    cli_type: activeCliType.value,
    ...form.value,
    model_maps: form.value.model_maps.filter(m => m.source_model && m.target_model),
    model_blacklist: form.value.model_blacklist.filter(b => b.model_pattern)
  }
  
  if (editingProvider.value) {
    await providerStore.updateProvider(editingProvider.value.id, data)
    notify('更新成功')
  } else {
    await providerStore.createProvider(data as any)
    notify('添加成功')
  }
  showDialog.value = false
  resetForm()
  providerStore.fetchProviders(activeCliType.value as CliType)
}

async function handleToggle(provider: Provider) {
  try {
    await providerStore.updateProvider(provider.id, { enabled: provider.enabled })
    notify(provider.enabled ? '已启用' : '已停用')
  } catch {
    provider.enabled = !provider.enabled
  }
}

async function handleDragEnd() {
  const ids = providerStore.providers.map(p => p.id)
  await providerStore.reorderProviders(ids)
  notify('排序已保存')
}

async function handleReset(provider: Provider) {
  await providerStore.resetFailures(provider.id)
  if (provider.is_blacklisted) {
    await providerStore.unblacklist(provider.id)
  }
  notify('重置成功')
}

async function handleCommand(command: string, provider: Provider) {
  if (command === 'reset') {
    await providerStore.resetFailures(provider.id)
    notify('已重置')
  } else if (command === 'unblacklist') {
    await providerStore.unblacklist(provider.id)
    notify('已解除拉黑')
  } else if (command === 'delete') {
    await ElMessageBox.confirm('确定删除该服务商？', '确认')
    await providerStore.deleteProvider(provider.id)
    notify('已删除')
  }
}

function handleEditCredential(credential: OfficialCredential) {
  editingCredential.value = credential
  credentialForm.value.name = credential.name
  try {
    const filesData = JSON.parse(credential.credential_json)
    if (Array.isArray(filesData)) {
      filesData.forEach(file => {
        const path = file.path || ''; const content = file.content || ''
        if (path.includes('.claude') && path.includes('settings.json')) credentialForm.value.claude_settings = content
        else if (path.includes('auth.json')) credentialForm.value.codex_auth = content
        else if (path.includes('oauth_creds.json')) credentialForm.value.gemini_oauth = content
        else if (path.includes('google_accounts.json')) credentialForm.value.gemini_accounts = content
        else if (path.includes('.gemini') && path.includes('settings.json')) credentialForm.value.gemini_settings = content
      })
    }
  } catch (e) {}
}

async function handleDeleteCredential(credential: OfficialCredential) {
  await ElMessageBox.confirm('确定删除该凭证？', '确认')
  await credentialStore.deleteCredential(credential.id)
  notify('已删除')
}

async function handleReadFromCli() {
  try {
    const { data } = await credentialsApi.readCliCredential(activeCliType.value as CliType)
    try {
      const filesData = JSON.parse(data)
      if (Array.isArray(filesData)) {
        filesData.forEach(file => {
          const path = file.path || ''; const content = file.content || ''
          if (path.includes('.claude') && path.includes('settings.json')) credentialForm.value.claude_settings = content
          else if (path.includes('auth.json')) credentialForm.value.codex_auth = content
          else if (path.includes('oauth_creds.json')) credentialForm.value.gemini_oauth = content
          else if (path.includes('google_accounts.json')) credentialForm.value.gemini_accounts = content
          else if (path.includes('.gemini') && path.includes('settings.json')) credentialForm.value.gemini_settings = content
        })
      }
    } catch {}
    notify('读取成功')
  } catch (e: any) {
    notify(e.message || '读取失败', 'error')
  }
}

async function handleSaveCredential() {
  if (!credentialForm.value.name) {
    notify('请输入凭证名称', 'error')
    return
  }
  const files: Array<{ path: string; content: string }> = []
  if (activeCliType.value === 'claude_code') {
    if (credentialForm.value.claude_settings) files.push({ path: '~/.claude/settings.json', content: credentialForm.value.claude_settings })
  } else if (activeCliType.value === 'codex') {
    if (credentialForm.value.codex_auth) files.push({ path: '~/.codex/auth.json', content: credentialForm.value.codex_auth })
  } else if (activeCliType.value === 'gemini') {
    if (credentialForm.value.gemini_oauth) files.push({ path: '~/.gemini/oauth_creds.json', content: credentialForm.value.gemini_oauth })
    if (credentialForm.value.gemini_accounts) files.push({ path: '~/.gemini/google_accounts.json', content: credentialForm.value.gemini_accounts })
    if (credentialForm.value.gemini_settings) files.push({ path: '~/.gemini/settings.json', content: credentialForm.value.gemini_settings })
  }
  if (files.length === 0) {
    notify('请至少填写一个文件内容', 'error')
    return
  }

  const data: OfficialCredentialCreate = {
    cli_type: activeCliType.value as CliType,
    name: credentialForm.value.name.trim(),
    credential_json: JSON.stringify(files)
  }

  if (editingCredential.value) {
    await credentialStore.updateCredential(editingCredential.value.id, { name: data.name, credential_json: data.credential_json })
    notify('更新成功')
  } else {
    await credentialStore.createCredential(data)
    notify('添加成功')
  }
  showCredentialDialog.value = false
  resetCredentialForm()
  credentialStore.fetchCredentials(activeCliType.value as CliType)
}

async function handleCredentialDragEnd() {
  const ids = credentialStore.credentials.map(c => c.id)
  await credentialStore.reorderCredentials(ids)
  notify('排序已保存')
}

function getUnblacklistTime(provider: Provider): string {
  if (!provider.is_blacklisted || !provider.blacklisted_until) return '已拉黑'
  const diffSeconds = provider.blacklisted_until - (Date.now() / 1000)
  if (diffSeconds <= 0) return '已拉黑'
  const mins = Math.floor(diffSeconds / 60)
  return mins === 0 ? `${Math.ceil(diffSeconds)}秒后解除` : `${mins}分${Math.ceil(diffSeconds % 60)}秒后解除`
}

onMounted(() => {
  providerStore.fetchProviders(activeCliType.value as CliType)
  credentialStore.fetchCredentials(activeCliType.value as CliType)
})
</script>

<style scoped>
.b-card { background: #ffffff; border-radius: 16px; box-shadow: 0 4px 12px rgba(0,0,0,0.03); margin-bottom: 24px; transition: border-color 0.2s; border: 1px solid transparent; }
.b-card:hover { border-color: #e2e8f0; }

.b-segmented { display: inline-flex; background: #e2e8f0; padding: 4px; border-radius: 10px; }
.b-seg-btn { text-align: center; padding: 6px 16px; font-size: 14px; color: #475569; border-radius: 8px; cursor: pointer; font-weight: 500; transition: all 0.2s ease; }
.b-seg-btn.active { background: #ffffff; color: #0f172a; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }

.b-button { background: #0ea5e9; color: white; border: none; padding: 8px 16px; border-radius: 8px; font-size: 14px; font-weight: 500; cursor: pointer; transition: background 0.2s; }
.b-button:hover { background: #0284c7; }

.b-button-outline { background: white; color: #0f172a; border: 1px solid #e2e8f0; padding: 8px 16px; border-radius: 8px; font-size: 14px; font-weight: 500; cursor: pointer; transition: background 0.2s; }
.b-button-outline:hover { background: #f8fafc; }

.b-button-icon { background: white; border: 1px solid #e2e8f0; color: #64748b; width: 32px; height: 32px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: 0.2s; }
.b-button-icon:hover { background: #fee2e2; color: #ef4444; border-color: #fca5a5; }

.tag { padding: 4px 10px; border-radius: 999px; font-size: 12px; font-weight: 500; }

.c-input { width: 100%; padding: 10px 14px; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 14px; outline: none; transition: border-color 0.2s; }
.c-input:focus { border-color: #0ea5e9; }
.c-label { font-size: 13px; font-weight: 500; color: #475569; margin-bottom: 12px; display: block; }

.drag-handle { display: flex; flex-direction: column; gap: 3px; cursor: grab; padding: 8px; margin-left: -8px; opacity: 0.3; transition: opacity 0.2s; }
.drag-handle:hover { opacity: 0.8; }
.drag-dot { width: 4px; height: 4px; border-radius: 50%; background: #64748b; }

.action-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
  background: transparent;
}
.action-icon:hover {
  background: #f1f5f9;
  color: #0f172a;
}
.action-icon.delete:hover {
  background: #fee2e2;
  color: #ef4444;
}
</style>
