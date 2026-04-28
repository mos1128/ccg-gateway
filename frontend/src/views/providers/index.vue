<template>
  <div class="providers-page">
    <svg style="display:none">
      <defs>
        <symbol id="icon-cloud" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>
        </symbol>
        <symbol id="icon-key" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="7.5" cy="15.5" r="5.5"/>
          <path d="m21 2-9.6 9.6"/>
          <path d="m15.5 7.5 3 3L22 7l-3-3"/>
        </symbol>
        <symbol id="icon-edit" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </symbol>
        <symbol id="icon-copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
        </symbol>
        <symbol id="icon-paste" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>
          <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
          <path d="M9 14h6"/>
          <path d="M12 11v6"/>
        </symbol>
        <symbol id="icon-refresh" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 16h5v5"/>
        </symbol>
        <symbol id="icon-trash" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>
        </symbol>
      </defs>
    </svg>
    
    <!-- Top Level Tabs -->
    <div class="top-tabs">
      <div
        v-for="cli in cliTabs"
        :key="cli.id"
        :class="['tab-item', { active: activeCliType === cli.id }]"
        @click="activeCliType = cli.id"
      >
        {{ cli.label }}
      </div>
    </div>

    <!-- Page Header & Segmented Control -->
    <div class="page-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
      <div class="header-left">
        <div class="b-segmented">
          <div class="b-seg-btn" :class="{ active: viewMode === 'proxy' }" @click="viewMode = 'proxy'">中转模式</div>
          <div class="b-seg-btn" :class="{ active: viewMode === 'direct' }" @click="handleSwitchDirect">官方模式</div>
        </div>

        <div v-if="showProfileControls" class="b-segmented profile-segmented">
          <div
            v-for="profile in profileTabs"
            :key="profile.id"
            class="b-seg-btn"
            :class="{ active: activeProfile === profile.id, disabled: !!profileSwitching }"
            @click="handleProfileSelect(profile.id)"
          >
            {{ profile.label }}
          </div>
        </div>
        <el-tooltip
          v-if="showProfileControls"
          effect="light"
          placement="top"
          :fallback-placements="['bottom', 'top', 'right', 'left']"
          :offset="10"
          :show-after="150"
          :enterable="true"
          popper-class="profile-help-popper"
        >
          <template #content>
            <div class="profile-help-content">
              <div class="tooltip-title">Profile 用法</div>
              <div class="tooltip-item">
                <span>切换到 Profile 时会自动生成对应的配置文件，通过对应启动命令启动的 ClaudeCode 会路由到对应 Profile 配置的服务商</span>
              </div>
              <div class="profile-command-panel">
                <div class="profile-command-header">
                  <strong>{{ profileLabels[activeProfile] }} 启动命令</strong>
                  <button
                    class="profile-command-copy"
                    type="button"
                    :disabled="isCurrentProfileCommandLoading"
                    @click.stop="copyCurrentProfileLaunchCommand"
                    title="复制启动命令"
                  >
                    <svg width="14" height="14"><use href="#icon-copy"/></svg>
                  </button>
                </div>
                <div class="profile-command-text" @click.stop="copyCurrentProfileLaunchCommand">
                  {{ currentProfileLaunchCommand || '正在获取启动命令...' }}
                </div>
              </div>
            </div>
          </template>
          <div class="help-icon-wrapper" @click.stop="copyCurrentProfileLaunchCommand" title="复制当前 Profile 启动命令">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="help-icon">
              <circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </div>
        </el-tooltip>
      </div>
      
      <div v-if="viewMode === 'proxy'" style="display: flex; align-items: center; gap: 8px;">
        <button
          class="action-icon detect-btn"
          @click="showDetectDialog = true"
          title="检测模型可用性"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
          </svg>
        </button>
        <button
          v-if="copiedProvider"
          class="action-icon paste-btn"
          :disabled="pasteLoading"
          @click="handlePasteProvider"
          :title="pasteButtonTitle"
        >
          <svg width="18" height="18"><use href="#icon-paste"/></svg>
        </button>
        <button
          class="action-icon add-btn"
          @click="handleAddProvider"
          title="添加服务商"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 12h14"/><path d="M12 5v14"/>
          </svg>
        </button>
      </div>
      <div v-else>
        <button
          class="action-icon add-btn"
          @click="showAddCredentialDialog = true"
          title="添加凭证"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 12h14"/><path d="M12 5v14"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- PROXY MODE LIST -->
    <div v-if="viewMode === 'proxy'" class="b-card list-container" v-loading="providerStore.loading">
      <div v-if="providerStore.providers.length === 0" class="empty-state">
        <svg width="64" height="64" color="var(--color-border)"><use href="#icon-cloud"/></svg>
        <p>暂无服务商</p>
      </div>
      
      <div v-else class="scroll-area">
        <draggable
          v-model="providerStore.providers"
          item-key="id"
          handle=".drag-handle"
          @end="handleDragEnd"
        >
          <template #item="{ element, index }">
            <div :style="{
              padding: '24px',
              borderBottom: index === providerStore.providers.length - 1 ? 'none' : '1px solid var(--color-bg-subtle)',
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              background: element.is_blacklisted ? 'var(--color-error-2)' : 'var(--color-bg)'
            }">
              <div style="display: flex; align-items: center; gap: 16px; flex: 1; min-width: 0;">
                <div class="drag-handle" aria-label="拖拽排序" style="flex-shrink: 0;">
                  <div class="drag-dot"></div><div class="drag-dot"></div><div class="drag-dot"></div>
                </div>
                
                <div style="flex: 1; min-width: 0;">
                  <div style="display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                    <div class="text-16 fw-medium text-primary" style="white-space: nowrap;" :style="{ color: !element.enabled ? 'var(--color-text-weak)' : 'var(--color-text)' }">
                      {{ element.name }}
                    </div>
                    <div v-if="element.is_blacklisted" class="tag" style="background: var(--color-error-10); color: var(--color-error); white-space: nowrap;">
                      {{ getUnblacklistTime(element) }}
                    </div>
                    <div v-else-if="!element.enabled" class="tag" style="background: var(--color-bg-subtle); color: var(--color-text-muted); white-space: nowrap;">
                      已禁用
                    </div>
                    <div v-if="element.model_maps.length > 0" class="tag" style="background: var(--color-success-10); color: var(--color-success); white-space: nowrap;">
                      {{ element.model_maps.length }}个模型映射
                    </div>
                    <div v-if="element.model_blacklist && element.model_blacklist.length > 0" class="tag" style="background: var(--color-warning-10); color: var(--color-warning); white-space: nowrap;">
                      {{ element.model_blacklist.length }}个黑名单配置
                    </div>
                  </div>
                </div>
              </div>
              
              <div style="display: flex; align-items: center; gap: 40px; flex-shrink: 0; margin-left: 24px;">
                <div style="display: flex; gap: 24px;">
                  <div style="display: flex; flex-direction: column; align-items: center; min-width: 64px;">
                    <div class="text-12 text-muted" style="margin-bottom: 2px; white-space: nowrap;">失败阈值</div>
                    <div class="mono text-16" :style="{ 'font-weight': 'var(--fw-500)', color: element.consecutive_failures >= element.failure_threshold ? 'var(--color-danger)' : 'var(--color-text)' }">
                      {{ element.consecutive_failures }}/{{ element.failure_threshold }}
                    </div>
                  </div>
                </div>
                
                <div style="display: flex; align-items: center; gap: 24px;">
                  <el-switch v-model="element.enabled" @change="handleToggle(element)" />
                  
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <div class="action-icon" @click="handleCopyProvider(element)" title="复制">
                      <svg width="18" height="18"><use href="#icon-copy"/></svg>
                    </div>

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
    </div>

    <!-- DIRECT MODE -->
    <div v-else class="b-card list-container" v-loading="credentialStore.loading">
      <div v-if="credentialStore.credentials.length === 0" class="empty-state">
        <svg width="64" height="64" color="var(--color-border)"><use href="#icon-key"/></svg>
        <p>暂无凭证</p>
      </div>
      
      <div v-else class="scroll-area">
        <draggable
          v-model="credentialStore.credentials"
          item-key="id"
          handle=".drag-handle"
          @end="handleCredentialDragEnd"
        >
          <template #item="{ element, index }">
            <div :style="{
              padding: '24px',
              borderBottom: index === credentialStore.credentials.length - 1 ? 'none' : '1px solid var(--color-bg-subtle)',
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              background: 'var(--color-bg)'
            }">
              <div style="display: flex; align-items: center; gap: 16px;">
                <div class="drag-handle" aria-label="拖拽排序">
                  <div class="drag-dot"></div><div class="drag-dot"></div><div class="drag-dot"></div>
                </div>
                
                <div>
                  <div style="display: flex; align-items: center; gap: 12px;">
                    <div class="text-16 fw-medium text-primary">{{ element.name }}</div>
                    <div v-if="element.is_active" class="tag" style="background: var(--color-success-10); color: var(--color-success);">激活中</div>
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
    </div>

    <!-- Add/Edit Provider Modal -->
    <AppModal v-model="showDialog" :title="editingProvider ? '编辑服务商' : '添加服务商'" width="720px" @confirm="handleSave">
      <div style="display: flex; gap: 32px; margin-bottom: 32px;">
            <div style="flex: 1;">
              <label class="c-label">服务商名称 <span style="color: var(--color-danger);">*</span></label>
              <input type="text" v-model="form.name" class="b-input" placeholder="例如: OpenAI 官方">
            </div>
            <div style="flex: 1;">
              <label class="c-label">Base URL <span style="color: var(--color-danger);">*</span></label>
              <input type="text" v-model="form.base_url" class="b-input" :placeholder="baseUrlPlaceholder">
            </div>
          </div>
          
          <div style="margin-bottom: 40px;">
            <label class="c-label">{{ activeCliType === 'claude_code' ? 'API Token' : 'API Key' }} <span style="color: var(--color-danger);">*</span></label>
            <input type="text" v-model="form.api_key" class="b-input" placeholder="sk-...">
          </div>

          <!-- Advanced Params -->
          <div style="display: flex; gap: 32px; margin-bottom: 40px; padding: 32px 24px; background: var(--color-bg-page); border-radius: 12px; border: 1px solid var(--color-bg-subtle);">
            <div style="flex: 1;">
              <label class="c-label">失败鉴权阈值 (次)</label>
              <input type="number" v-model.number="form.failure_threshold" class="b-input">
            </div>
            <div style="flex: 1;">
              <label class="c-label">拉黑时长 (分钟)</label>
              <input type="number" v-model.number="form.blacklist_minutes" class="b-input">
            </div>
            <div style="flex: 1;">
              <label class="c-label">自定义 UA (选填)</label>
              <input type="text" v-model="form.custom_useragent" class="b-input" placeholder="留空则使用原始">
            </div>
          </div>

          <!-- Model Maps Section -->
          <div style="margin-bottom: 40px;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
              <div>
                <div class="text-16 fw-normal text-primary">模型映射</div>
                <div class="text-12 text-secondary" style="margin-top: 6px;">映射 CLI 请求的源模型名称为服务商模型</div>
              </div>
              <button class="b-button-outline text-14" style="padding: 6px 12px;" @click="addModelMap">+ 添加</button>
            </div>

            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div v-for="(map, index) in form.model_maps" :key="'map-'+index" style="display: flex; gap: 16px; align-items: center;">
                <input type="text" v-model="map.source_model" class="b-input" placeholder="CLI 源模型" style="flex: 1;">
                <div class="text-secondary fw-normal">→</div>
                <input type="text" v-model="map.target_model" class="b-input" placeholder="服务商模型" style="flex: 1;">
                <div class="b-button-icon" @click="removeModelMap(index)">×</div>
              </div>
            </div>
          </div>

          <!-- Model Blacklist Section -->
          <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
              <div>
                <div class="text-16 fw-normal text-primary">模型黑名单</div>
                <div class="text-12 text-secondary" style="margin-top: 6px;">配置服务商不支持的模型名称</div>
              </div>
              <button class="b-button-outline text-14" style="padding: 6px 12px;" @click="addModelBlacklist">+ 添加</button>
            </div>

            <div style="display: flex; flex-direction: column; gap: 20px;">
              <div v-for="(item, index) in form.model_blacklist" :key="'blk-'+index" style="display: flex; gap: 16px; align-items: center;">
                 <input type="text" v-model="item.model_pattern" class="b-input" placeholder="模型规则" style="flex: 1;">
                 <div class="b-button-icon" @click="removeModelBlacklist(index)">×</div>
              </div>
            </div>
          </div>
    </AppModal>
    <!-- / Add Provider Modal -->

    <!-- Add/Edit Credential Modal -->
    <AppModal v-model="showCredentialDialog" :title="editingCredential ? '编辑凭证' : '添加凭证'" width="720px" @confirm="handleSaveCredential">
          <div style="margin-bottom: 32px;">
            <label class="c-label">凭证名称 <span style="color: var(--color-danger);">*</span></label>
            <input type="text" v-model="credentialForm.name" class="b-input" placeholder="例如: 个人主账号">
          </div>

          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
            <div class="text-16 fw-normal text-primary">配置文件</div>
            <button class="b-button-outline text-14" style="padding: 6px 12px;" @click="handleReadFromCli">读取当前 CLI 配置</button>
          </div>

          <template v-if="activeCliType === 'claude_code'">
             <div style="margin-bottom: 24px;">
               <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.claude/settings.json</div>
               <textarea class="b-input mono" rows="10" v-model="credentialForm.claude_settings" placeholder='{"ANTHROPIC_API_KEY": "..."}'></textarea>
             </div>
          </template>

          <template v-if="activeCliType === 'codex'">
            <div style="margin-bottom: 24px;">
               <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.codex/auth.json</div>
               <textarea class="b-input mono" rows="10" v-model="credentialForm.codex_auth"></textarea>
             </div>
          </template>

          <template v-if="activeCliType === 'gemini'">
             <div style="margin-bottom: 24px;">
               <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.gemini/oauth_creds.json</div>
               <textarea class="b-input mono" rows="4" v-model="credentialForm.gemini_oauth"></textarea>
             </div>
             <div style="margin-bottom: 24px;">
               <div class="text-12 text-secondary" style="margin-bottom: 8px;">~/.gemini/google_accounts.json</div>
               <textarea class="b-input mono" rows="3" v-model="credentialForm.gemini_accounts"></textarea>
             </div>
          </template>
    </AppModal>

    <!-- Model Detection Modal -->
    <AppModal v-model="showDetectDialog" title="检测模型可用性" width="800px" :show-footer="true" confirm-text="开始检测" @confirm="handleStartDetect">
      <!-- Model Input -->
      <div style="display: flex; gap: 12px; align-items: flex-end; margin-bottom: 24px;">
        <div style="flex: 1;">
          <label class="c-label">检测模型</label>
          <input type="text" v-model="detectModel" class="b-input" placeholder="输入模型名称">
        </div>
      </div>

      <!-- Provider Checkboxes -->
      <div style="margin-bottom: 24px;">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
          <label class="c-label" style="margin-bottom: 0;">选择服务商</label>
          <span class="text-12 text-info fw-normal" style="cursor: pointer;" @click="toggleAllDetectProviders">
            {{ isAllDetectSelected ? '取消全选' : '全选' }}
          </span>
        </div>
        <div style="display: flex; gap: 10px; flex-wrap: wrap;">
          <label
            v-for="p in detectProviderList"
            :key="p.id"
            class="text-14"
            style="display: flex; align-items: center; gap: 6px; cursor: pointer; padding: 6px 12px; border-radius: 8px; transition: all 0.2s; user-select: none;"
            :style="{
              color: detectSelectedIds.includes(p.id) ? 'var(--color-text)' : 'var(--color-text-weak)',
              border: detectSelectedIds.includes(p.id) ? '1px solid var(--color-primary)' : '1px solid var(--color-border)',
              background: detectSelectedIds.includes(p.id) ? 'var(--color-primary-5)' : 'var(--color-bg)'
            }"
            @click="toggleDetectProvider(p.id)"
          >
            <div
              style="width: 16px; height: 16px; border-radius: 4px; display: flex; align-items: center; justify-content: center; transition: all 0.2s; flex-shrink: 0;"
              :style="{
                border: detectSelectedIds.includes(p.id) ? '2px solid var(--color-primary)' : '2px solid var(--color-border)',
                background: detectSelectedIds.includes(p.id) ? 'var(--color-primary)' : 'transparent'
              }"
            >
              <span v-if="detectSelectedIds.includes(p.id)" class="text-12 fw-bold" style="color: var(--color-bg);">✓</span>
            </div>
            {{ p.name }}
          </label>
        </div>
        <div v-if="detectProviderList.length === 0" class="text-muted text-14" style="padding: 8px 0;">
          当前 CLI 类型无已启用的服务商
        </div>
      </div>

      <!-- Results Table -->
      <div v-if="detectResults.length > 0 || detectLoading" style="border: 1px solid var(--color-border); border-radius: 12px; overflow: hidden; box-shadow: 0 4px 15px var(--color-shadow);">
        <table class="flat-table">
          <colgroup>
            <col style="width: 20%;"><col style="width: 25%;"><col style="width: 12%;"><col style="width: 13%;"><col style="width: 30%;">
          </colgroup>
          <thead>
            <tr>
              <th>服务商</th><th>测试模型</th><th>状态码</th><th>耗时</th><th>响应</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in detectResults" :key="r.provider_id">
              <td class="fw-normal">{{ r.provider_name }}</td>
              <td class="mono">{{ r.actual_model }}</td>
              <td>
                <span v-if="r.status_code === null && r.elapsed_ms === 0" class="pill pill-grey">...</span>
                <span v-else-if="r.status_code !== null" :class="['pill', getDetectPill(r.status_code)]">{{ r.status_code }}</span>
                <span v-else class="pill pill-red">ERR</span>
              </td>
              <td class="mono">
                <span v-if="r.status_code === null && r.elapsed_ms === 0">-</span>
                <span v-else>{{ r.elapsed_ms }}ms</span>
              </td>
              <td :style="{ color: r.status_code !== null && r.status_code >= 200 && r.status_code < 300 ? 'var(--color-text-muted)' : (r.status_code === null && r.elapsed_ms === 0 ? 'var(--color-text-weak)' : 'var(--color-error)') }" style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                <span v-if="r.status_code === null && r.elapsed_ms === 0" style="font-style: italic;">Testing...</span>
                <el-tooltip
                  v-else
                  effect="light"
                  placement="top"
                  :enterable="true"
                  :show-after="200"
                >
                  <template #content>
                    <div class="text-14" style="max-width: 350px; line-height: 1.6; word-break: break-word; user-select: text; color: var(--color-text-dark);">
                      {{ r.response_text }}
                    </div>
                  </template>
                  <span style="cursor: pointer;" @click="copyResponseText(r.response_text)">{{ r.response_text }}</span>
                </el-tooltip>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { confirm } from '@/utils/confirm'
import { notify } from '@/utils/notification'
import { getErrorMessage } from '@/utils/error'
import draggable from 'vuedraggable'
import AppModal from '@/components/AppModal.vue'
import { useProviderStore } from '@/stores/providers'
import { useCredentialStore } from '@/stores/credentials'
import { useUiStore } from '@/stores/ui'
import { credentialsApi } from '@/api/credentials'
import { providersApi } from '@/api/providers'
import { settingsApi } from '@/api/settings'
import type { Provider, CliType, ProviderProfile, ClaudeProfileSettingsStatus, OfficialCredential, OfficialCredentialCreate, TestProviderResult } from '@/types/models'

const providerStore = useProviderStore()
const credentialStore = useCredentialStore()
const uiStore = useUiStore()

const cliTabs: { id: CliType; label: string }[] = [
  { id: 'claude_code', label: 'Claude Code' },
  { id: 'codex', label: 'Codex' },
  { id: 'gemini', label: 'Gemini' }
]

const profileTabs: { id: ProviderProfile; label: string }[] = [
  { id: 'default', label: '默认' },
  { id: 'profile1', label: 'Profile 1' },
  { id: 'profile2', label: 'Profile 2' },
  { id: 'profile3', label: 'Profile 3' }
]

const activeCliType = computed({
  get: () => uiStore.providersActiveCliType,
  set: (val) => uiStore.setProvidersActiveCliType(val)
})

const activeProfile = computed({
  get: () => uiStore.providersActiveProfile,
  set: (val) => uiStore.setProvidersActiveProfile(val)
})

type ViewMode = 'proxy' | 'direct'

const viewModes = ref<Record<CliType, ViewMode>>({
  claude_code: 'proxy',
  codex: 'proxy',
  gemini: 'proxy'
})
const viewMode = computed<ViewMode>({
  get: () => activeCliType.value === 'claude_code' ? 'proxy' : viewModes.value[activeCliType.value],
  set: (mode) => {
    if (activeCliType.value === 'claude_code') {
      viewModes.value.claude_code = 'proxy'
      if (mode === 'direct') notify('Claude Code 暂未实现官方模式功能', 'warning')
      return
    }
    viewModes.value[activeCliType.value] = mode
  }
})
const showProfileControls = computed(() => viewMode.value === 'proxy' && activeCliType.value === 'claude_code')
const currentProviderProfile = computed<ProviderProfile>(() =>
  activeCliType.value === 'claude_code' ? activeProfile.value : 'default'
)
const profileSwitching = ref<ProviderProfile | null>(null)
let testResultListener: (() => void) | null = null

const profileLabels: Record<ProviderProfile, string> = {
  default: '默认',
  profile1: 'Profile 1',
  profile2: 'Profile 2',
  profile3: 'Profile 3'
}

const profileSettingsStatusMap = ref<Partial<Record<ProviderProfile, ClaudeProfileSettingsStatus>>>({})
const profileCommandLoading = ref<ProviderProfile | null>(null)
let profileCommandRequestId = 0

const currentProfileSettingsStatus = computed(() => profileSettingsStatusMap.value[activeProfile.value])
const currentProfileLaunchCommand = computed(() => currentProfileSettingsStatus.value?.launch_command || '')
const isCurrentProfileCommandLoading = computed(() => profileCommandLoading.value === activeProfile.value)

function handleSwitchDirect() {
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
      resetForm()
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
interface ProviderDraft {
  name: string
  base_url: string
  api_key: string
  enabled: boolean
  failure_threshold: number
  blacklist_minutes: number
  custom_useragent: string
  model_maps: FormModelMap[]
  model_blacklist: FormModelBlacklist[]
}

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
const copiedProvider = ref<ProviderDraft | null>(null)
const pasteLoading = ref(false)

const credentialForm = ref({
  name: '',
  claude_settings: '',
  codex_auth: '',
  gemini_oauth: '',
  gemini_accounts: ''
})

const baseUrlPlaceholder = computed(() => {
  if (activeCliType.value === 'codex') return 'https://api.example.com/v1'
  return 'https://api.example.com'
})

const pasteButtonTitle = computed(() => {
  if (!copiedProvider.value) return '粘贴服务商'
  return `粘贴服务商：${copiedProvider.value.name}`
})

function resetForm() {
  form.value = {
    name: '', base_url: '', api_key: '', failure_threshold: 3, blacklist_minutes: 10,
    custom_useragent: '', model_maps: [], model_blacklist: []
  }
}
function resetCredentialForm() {
  credentialForm.value = { name: '', claude_settings: '', codex_auth: '', gemini_oauth: '', gemini_accounts: '' }
}

function cloneProviderDraft(draft: ProviderDraft): ProviderDraft {
  return {
    ...draft,
    model_maps: draft.model_maps.map(m => ({ ...m })),
    model_blacklist: draft.model_blacklist.map(b => ({ ...b }))
  }
}

function createProviderDraft(provider: Provider): ProviderDraft {
  return {
    name: provider.name,
    base_url: provider.base_url,
    api_key: provider.api_key,
    enabled: provider.enabled,
    failure_threshold: provider.failure_threshold,
    blacklist_minutes: provider.blacklist_minutes,
    custom_useragent: provider.custom_useragent || '',
    model_maps: provider.model_maps.map(({ source_model, target_model, enabled }) => ({ source_model, target_model, enabled })),
    model_blacklist: provider.model_blacklist.map(({ model_pattern }) => ({ model_pattern }))
  }
}

function makeUniqueProviderName(name: string): string {
  const trimmedName = name.trim() || '未命名服务商'
  const existingNames = new Set(providerStore.providers.map(p => p.name.trim().toLowerCase()))
  if (!existingNames.has(trimmedName.toLowerCase())) return trimmedName

  const baseName = `${trimmedName} 副本`
  let candidate = baseName
  let index = 2
  while (existingNames.has(candidate.toLowerCase())) {
    candidate = `${baseName} ${index}`
    index += 1
  }
  return candidate
}

// ==================== Model Detection ====================
const DEFAULT_DETECT_MODELS: Record<string, string> = {
  claude_code: 'claude-opus-4-7',
  codex: 'gpt-5.5',
  gemini: 'gemini-3.1-pro-preview',
}

const showDetectDialog = ref(false)
const detectLoading = ref(false)
const detectModel = ref('')
const detectSelectedIds = ref<number[]>([])
const detectResults = ref<TestProviderResult[]>([])

const detectProviderList = computed(() =>
  providerStore.providers.filter(p => p.enabled)
)

const isAllDetectSelected = computed(() =>
  detectProviderList.value.length > 0 && detectSelectedIds.value.length === detectProviderList.value.length
)

function toggleDetectProvider(id: number) {
  const idx = detectSelectedIds.value.indexOf(id)
  if (idx >= 0) detectSelectedIds.value.splice(idx, 1)
  else detectSelectedIds.value.push(id)
}

function toggleAllDetectProviders() {
  if (isAllDetectSelected.value) {
    detectSelectedIds.value = []
  } else {
    detectSelectedIds.value = detectProviderList.value.map(p => p.id)
  }
}

watch(showDetectDialog, (open) => {
  if (open) {
    const key = `detect_model_${activeCliType.value}`
    detectModel.value = localStorage.getItem(key) || DEFAULT_DETECT_MODELS[activeCliType.value] || ''
    detectSelectedIds.value = detectProviderList.value.map(p => p.id)
    detectResults.value = []
    detectLoading.value = false
  } else {
    // 关闭对话框时清理监听器
    if (testResultListener) {
      testResultListener()
      testResultListener = null
    }
  }
})

async function handleStartDetect() {
  if (!detectModel.value.trim()) {
    notify('请输入检测模型名称', 'error')
    return
  }
  if (detectSelectedIds.value.length === 0) {
    notify('请至少选择一个服务商', 'error')
    return
  }

  localStorage.setItem(`detect_model_${activeCliType.value}`, detectModel.value.trim())

  detectResults.value = detectSelectedIds.value.map(id => {
    const p = providerStore.providers.find(x => x.id === id)
    return {
      provider_id: id,
      provider_name: p?.name || 'Unknown',
      actual_model: '...',
      status_code: null,
      elapsed_ms: 0,
      response_text: '',
      request_url: '',
      request_headers: '',
      request_body: '',
      response_headers: '',
      response_body: '',
    }
  })
  detectLoading.value = true

  // 清理之前的监听器
  if (testResultListener) {
    testResultListener()
    testResultListener = null
  }

  // 监听测试结果事件
  testResultListener = await providersApi.listenTestResults((result) => {
    const idx = detectResults.value.findIndex(r => r.provider_id === result.provider_id)
    if (idx >= 0) {
      detectResults.value[idx] = result
    }
    // 检查是否所有结果都已返回（response_text 非空表示有结果）
    if (detectResults.value.every(r => r.response_text !== '')) {
      detectLoading.value = false
    }
  })

  try {
    await providersApi.startTestModels(detectModel.value.trim(), detectSelectedIds.value)
  } catch (e: any) {
    notify(getErrorMessage(e, '检测失败'), 'error')
    detectLoading.value = false
  }
}

function getDetectPill(code: number | null): string {
  if (!code) return 'pill-grey'
  if (code >= 200 && code < 300) return 'pill-green'
  if (code >= 400 && code < 500) return 'pill-grey'
  if (code >= 500) return 'pill-red'
  return 'pill-grey'
}

async function copyResponseText(text: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    notify('响应已复制到剪贴板')
  } catch {
    notify('复制失败', 'error')
  }
}

function cacheClaudeProfileSettingsStatus(status: ClaudeProfileSettingsStatus) {
  profileSettingsStatusMap.value = {
    ...profileSettingsStatusMap.value,
    [status.profile]: status
  }
}

async function loadClaudeProfileSettingsStatus(profile: ProviderProfile, silent = false): Promise<ClaudeProfileSettingsStatus | null> {
  const requestId = ++profileCommandRequestId
  profileCommandLoading.value = profile
  try {
    const { data } = await settingsApi.getClaudeProfileSettingsStatus(profile)
    cacheClaudeProfileSettingsStatus(data)
    return data
  } catch (e: any) {
    if (!silent) notify(getErrorMessage(e, '获取启动命令失败'), 'error')
    return null
  } finally {
    if (requestId === profileCommandRequestId) {
      profileCommandLoading.value = null
    }
  }
}

async function copyCurrentProfileLaunchCommand() {
  if (!showProfileControls.value) return

  const profile = activeProfile.value
  const status = profileSettingsStatusMap.value[profile] || await loadClaudeProfileSettingsStatus(profile)
  const command = status?.launch_command
  if (!command) return

  try {
    await navigator.clipboard.writeText(command)
    notify(`已复制 ${profileLabels[profile]} 启动命令`)
  } catch {
    notify('复制失败', 'error')
  }
}

function addModelMap() { form.value.model_maps.push({ source_model: '', target_model: '', enabled: true }) }
function removeModelMap(index: number) { form.value.model_maps.splice(index, 1) }
function addModelBlacklist() { form.value.model_blacklist.push({ model_pattern: '' }) }
function removeModelBlacklist(index: number) { form.value.model_blacklist.splice(index, 1) }

async function ensureClaudeProfileReady(profile: ProviderProfile): Promise<boolean> {
  if (activeCliType.value !== 'claude_code' || viewMode.value !== 'proxy' || profile === 'default') {
    return true
  }

  profileSwitching.value = profile
  try {
    const { data: status } = await settingsApi.getClaudeProfileSettingsStatus(profile)
    cacheClaudeProfileSettingsStatus(status)
    if (status.uses_gateway) return true

    const { data: ensured } = await settingsApi.ensureClaudeProfileSettings(profile)
    cacheClaudeProfileSettingsStatus(ensured)
    if (!ensured.uses_gateway) {
      notify(`写入后仍未检测到有效配置：${ensured.path}`, 'error')
      return false
    }

    notify(`已写入 ${ensured.path}`)
    return true
  } catch (e: any) {
    notify(getErrorMessage(e, '写入 Profile 配置失败'), 'error')
    return false
  } finally {
    profileSwitching.value = null
  }
}

async function handleProfileSelect(profile: ProviderProfile) {
  if (profile === activeProfile.value || profileSwitching.value) return

  const ok = await ensureClaudeProfileReady(profile)
  if (!ok) return

  activeProfile.value = profile
}

async function ensureCurrentClaudeProfileOrFallback(): Promise<ProviderProfile> {
  if (activeCliType.value !== 'claude_code') return 'default'

  const profile = activeProfile.value
  if (await ensureClaudeProfileReady(profile)) return profile

  activeProfile.value = 'default'
  return 'default'
}

// Listen for tab changes
watch(() => activeCliType.value, async (cliType) => {
  const profile = await ensureCurrentClaudeProfileOrFallback()
  providerStore.fetchProviders(cliType as CliType, profile)
  credentialStore.fetchCredentials(cliType as CliType)
})

watch(() => activeProfile.value, (profile) => {
  if (activeCliType.value !== 'claude_code') return
  providerStore.fetchProviders(activeCliType.value as CliType, profile)
})

watch([showProfileControls, activeProfile], ([visible, profile]) => {
  if (!visible || profileSettingsStatusMap.value[profile]) return
  loadClaudeProfileSettingsStatus(profile, true)
}, { immediate: true })

function handleAddProvider() {
  editingProvider.value = null
  resetForm()
  showAddDialog.value = true
}

function handleCopyProvider(provider: Provider) {
  copiedProvider.value = createProviderDraft(provider)
  notify(`已复制服务商：${provider.name}`)
}

async function handlePasteProvider() {
  if (!copiedProvider.value || pasteLoading.value) return

  const targetCliType = activeCliType.value as CliType
  const targetProfile = currentProviderProfile.value
  pasteLoading.value = true
  try {
    await providerStore.fetchProviders(targetCliType, targetProfile)
    const draft = cloneProviderDraft(copiedProvider.value)
    const data = {
      cli_type: targetCliType,
      profile: targetProfile,
      ...draft,
      name: makeUniqueProviderName(draft.name),
      model_maps: draft.model_maps.filter(m => m.source_model && m.target_model),
      model_blacklist: draft.model_blacklist.filter(b => b.model_pattern)
    }
    await providerStore.createProvider(data)
    notify(`已粘贴服务商：${data.name}`)
  } catch (e: any) {
    notify(getErrorMessage(e, '粘贴失败'), 'error')
  } finally {
    pasteLoading.value = false
  }
}

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
    profile: currentProviderProfile.value,
    ...form.value,
    model_maps: form.value.model_maps.filter(m => m.source_model && m.target_model),
    model_blacklist: form.value.model_blacklist.filter(b => b.model_pattern)
  }
  
  try {
    if (editingProvider.value) {
      await providerStore.updateProvider(editingProvider.value.id, data)
      notify('更新成功')
    } else {
      await providerStore.createProvider(data as any)
      notify('添加成功')
    }
    showDialog.value = false
    resetForm()
    providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
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
    try {
      await confirm('确定删除该服务商？', '确认')
      await providerStore.deleteProvider(provider.id)
      notify('已删除')
    } catch (e) {
      if (e !== 'cancel') notify(getErrorMessage(e, '删除失败'), 'error')
    }
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
      })
    }
  } catch (e) {}
}

async function handleDeleteCredential(credential: OfficialCredential) {
  try {
    await confirm('确定删除该凭证？', '确认')
    await credentialStore.deleteCredential(credential.id)
    notify('已删除')
  } catch (e) {
    if (e !== 'cancel') notify(getErrorMessage(e, '删除失败'), 'error')
  }
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
        })
      }
    } catch {}
    notify('读取成功')
  } catch (e: any) {
    notify(getErrorMessage(e, '读取失败'), 'error')
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

  try {
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
  } catch (e: any) {
    notify(getErrorMessage(e, '保存失败'), 'error')
  }
}

async function handleCredentialDragEnd() {
  const ids = credentialStore.credentials.map(c => c.id)
  await credentialStore.reorderCredentials(ids)
  notify('排序已保存')
}

const now = ref(Date.now())
let timer: any = null

function getUnblacklistTime(provider: Provider): string {
  if (!provider.is_blacklisted || !provider.blacklisted_until) return '已拉黑'
  const diffSeconds = provider.blacklisted_until - (now.value / 1000)
  if (diffSeconds <= 0) return '已解除'
  const mins = Math.floor(diffSeconds / 60)
  return mins === 0 ? `${Math.ceil(diffSeconds)}秒后解除` : `${mins}分${Math.ceil(diffSeconds % 60)}秒后解除`
}

onMounted(async () => {
  const profile = await ensureCurrentClaudeProfileOrFallback()
  providerStore.fetchProviders(activeCliType.value as CliType, profile)
  credentialStore.fetchCredentials(activeCliType.value as CliType)
  
  // 每秒更新一次时间，触发倒计时重绘
  timer = setInterval(() => {
    const oldNow = now.value
    now.value = Date.now()
    
    // 检查是否有服务商的拉黑时间刚刚到期
    const hasExpired = providerStore.providers.some(p => {
      if (p.is_blacklisted && p.blacklisted_until) {
        return p.blacklisted_until > (oldNow / 1000) && p.blacklisted_until <= (now.value / 1000)
      }
      return false
    })
    
    if (hasExpired) {
      providerStore.fetchProviders(activeCliType.value as CliType, currentProviderProfile.value)
    }
  }, 1000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
  if (testResultListener) {
    testResultListener()
    testResultListener = null
  }
})
</script>

<style scoped>
.providers-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Tab Underlines */
.top-tabs { display: flex; gap: 32px; border-bottom: 1px solid var(--color-border-light); margin: 0 40px 24px 40px; padding-top: 8px; flex-shrink: 0; }
.tab-item { padding-bottom: 12px; color: var(--color-text-weak); font-weight: var(--fw-400); font-size: var(--fs-14); cursor: pointer; position: relative; transition: color 0.2s; }
.tab-item:hover { color: var(--color-text-secondary); }
.tab-item.active { color: var(--color-primary); font-weight: var(--fw-600); border-bottom: 2px solid var(--color-primary); }

.page-header { flex-shrink: 0; margin: 0 40px 32px 40px; }
.header-left { display: flex; align-items: center; gap: 12px; min-width: 0; }
.profile-segmented .b-seg-btn { padding: 6px 12px; }
.profile-segmented .b-seg-btn.disabled { opacity: 0.55; pointer-events: none; }

.help-icon-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  cursor: help;
}

.help-icon {
  color: var(--color-text-weak);
  transition: color 0.2s;
}

.help-icon-wrapper:hover .help-icon { color: var(--color-text-muted); }

:global(.profile-help-popper.el-popper) {
  border-radius: 12px;
  padding: 16px;
  box-shadow: 0 8px 24px var(--color-shadow-lg);
}

.profile-help-content {
  width: 320px;
}

.tooltip-title {
  font-size: var(--fs-14);
  font-weight: var(--fw-700);
  color: var(--color-text);
  margin-bottom: 10px;
}

.tooltip-item {
  margin-bottom: 8px;
  font-size: var(--fs-12);
  line-height: 1.5;
  color: var(--color-text-muted);
}

.tooltip-item:last-child { margin-bottom: 0; }

.tooltip-item strong {
  display: block;
  color: var(--color-text-dark);
  font-weight: var(--fw-600);
  margin-bottom: 2px;
}

.profile-command-panel {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--color-border-light);
}

.profile-command-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 6px;
  font-size: var(--fs-12);
  color: var(--color-text-dark);
}

.profile-command-copy {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-bg);
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
}

.profile-command-copy:hover:not(:disabled) {
  background: var(--color-bg-page);
  color: var(--color-primary);
  border-color: var(--color-primary);
}

.profile-command-copy:disabled {
  cursor: wait;
  opacity: 0.6;
}

.profile-command-text {
  padding: 8px;
  border: 1px solid var(--color-border-light);
  border-radius: 6px;
  background: var(--color-bg-page);
  color: var(--color-text-secondary);
  font-family: ui-monospace, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace;
  font-size: var(--fs-12);
  line-height: 1.45;
  word-break: break-all;
  cursor: pointer;
  user-select: text;
}

.list-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: 0 40px 0 40px;
}

.scroll-area {
  flex: 1;
  overflow-y: auto;
}

.b-card { background: var(--color-bg); border-radius: 16px; box-shadow: 0 4px 12px var(--color-shadow); margin-bottom: 24px; border: 1px solid transparent; overflow: hidden; }

.b-segmented { display: inline-flex; background: var(--color-border); padding: 4px; border-radius: 10px; flex-shrink: 0; }
.b-seg-btn { text-align: center; padding: 6px 16px; font-size: var(--fs-14); color: var(--color-text-secondary); border-radius: 8px; cursor: pointer; font-weight: var(--fw-500); transition: all 0.2s ease; }
.b-seg-btn.active { background: var(--color-bg); color: var(--color-primary); box-shadow: 0 1px 3px var(--color-shadow-lg); }

.b-button-outline { background: var(--color-bg); color: var(--color-text); border: 1px solid var(--color-border); padding: 8px 16px; border-radius: 8px; font-size: var(--fs-14); font-weight: var(--fw-400); cursor: pointer; transition: background 0.2s; }
.b-button-outline:hover { background: var(--color-bg-page); }

.b-button-icon { background: var(--color-bg); border: 1px solid var(--color-border); color: var(--color-text-muted); width: 32px; height: 32px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: 0.2s; }
.b-button-icon:hover { background: var(--color-danger-light); color: var(--color-danger); border-color: var(--color-danger-muted); }

.tag { padding: 4px 10px; border-radius: 999px; font-size: var(--fs-12); font-weight: var(--fw-400); }

.c-label { font-size: var(--fs-14); font-weight: var(--fw-400); color: var(--color-text-secondary); margin-bottom: 12px; display: block; }

.drag-handle { display: flex; flex-direction: column; gap: 3px; cursor: grab; padding: 8px; margin-left: -8px; opacity: 0.3; transition: opacity 0.2s; }
.drag-handle:hover { opacity: 0.8; }
.drag-dot { width: 4px; height: 4px; border-radius: 50%; background: var(--color-text-muted); }

.action-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.2s;
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
}
.action-icon:hover {
  background: var(--color-bg-subtle);
  color: var(--color-text);
}
.action-icon:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  pointer-events: none;
}
.action-icon.delete:hover {
  background: var(--color-danger-light);
  color: var(--color-danger);
}

/* Detection Table */
.flat-table { width: 100%; border-collapse: separate; border-spacing: 0; text-align: left; table-layout: fixed; }
.flat-table th, .flat-table td { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; box-sizing: border-box; text-align: left; }
.flat-table th { padding: 12px 20px; font-size: var(--fs-12); font-weight: var(--fw-600); color: var(--color-text-muted); text-transform: uppercase; background: var(--color-bg-page); border-bottom: 1px solid var(--color-border); position: sticky; top: 0; z-index: 10; }
.flat-table td { padding: 12px 20px; font-size: var(--fs-14); color: var(--color-text); border-bottom: 1px solid var(--color-bg-subtle); }
.flat-table tr:last-child td { border-bottom: none; }
.flat-table tr:hover td { background: var(--color-bg-page); }
.pill { padding: 4px 10px; border-radius: 999px; font-size: var(--fs-12); font-weight: var(--fw-600); display: inline-flex; align-items: center; letter-spacing: 0.3px; }
.pill-green { background: var(--color-success-light); color: var(--color-success); }
.pill-red { background: var(--color-error-light); color: var(--color-error); }
.pill-grey { background: var(--color-bg-subtle); color: var(--color-text-muted); font-weight: var(--fw-400); }
.code-block { background: var(--color-bg-page); padding: 12px; border-radius: 6px; font-size: var(--fs-12); white-space: pre-wrap; word-break: break-all; max-height: 300px; overflow-y: auto; margin: 0; cursor: pointer; border: 1px solid transparent; transition: border-color 0.2s; }
.code-block:hover { border-color: var(--color-border-hover); }

.action-icon.detect-btn {
  width: 36px;
  height: 36px;
  color: var(--color-primary);
  background: var(--color-primary-10);
}
.action-icon.detect-btn:hover {
  background: var(--color-primary-20);
  color: var(--color-primary);
}

.action-icon.paste-btn {
  width: 36px;
  height: 36px;
  color: var(--color-success);
  background: var(--color-success-10);
}
.action-icon.paste-btn:hover {
  background: var(--color-success-light);
  color: var(--color-success);
}

.action-icon.add-btn {
  width: 36px;
  height: 36px;
  color: var(--color-primary);
  background: var(--color-primary-10);
}
.action-icon.add-btn:hover {
  background: var(--color-primary-20);
  color: var(--color-primary);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: var(--color-text-weak);
}
.empty-state p {
  margin-top: 16px;
  font-size: var(--fs-14);
}
</style>
>
