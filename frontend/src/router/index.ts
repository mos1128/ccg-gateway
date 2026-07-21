import { createRouter, createWebHashHistory } from 'vue-router'
import V2Layout from '@/layouts/V2Layout.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      component: V2Layout,
      children: [
        { path: '', name: 'Dashboard', component: () => import('@/views/dashboard/index.vue') },
        { path: 'providers', name: 'Providers', meta: { label: '服务商' }, component: () => import('@/views/providers/index.vue') },
        { path: 'agents', name: 'Agents', meta: { label: 'Agent' }, component: () => import('@/views/agents/index.vue') },
        { path: 'config', name: 'Config', meta: { label: '全局设置' }, component: () => import('@/views/config/index.vue') },
        { path: 'logs', name: 'Logs', meta: { label: '日志记录' }, component: () => import('@/views/logs/index.vue') },
        { path: 'scheduled-tasks', name: 'ScheduledTasks', meta: { label: '定时任务' }, component: () => import('@/views/scheduled-tasks/index.vue') },
        { path: 'sessions', name: 'Sessions', meta: { label: '会话记录' }, component: () => import('@/views/sessions/index.vue') },
        { path: 'mcp', name: 'MCP', meta: { label: 'MCP' }, component: () => import('@/views/mcp/index.vue') },
        { path: 'prompts', name: 'Prompts', meta: { label: '提示词' }, component: () => import('@/views/prompts/index.vue') },
        { path: 'skills', name: 'Skills', meta: { label: 'Skill' }, component: () => import('@/views/skills/index.vue') },
        { path: 'plugins', name: 'Plugins', meta: { label: 'Plugin' }, component: () => import('@/views/plugins/index.vue') },
        { path: ':pathMatch(.*)*', redirect: { name: 'Dashboard' } }
      ]
    }
  ]
})

export default router
