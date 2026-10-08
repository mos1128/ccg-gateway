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
        { path: 'providers', name: 'Providers', component: () => import('@/views/providers/index.vue') },
        { path: 'config', name: 'Config', component: () => import('@/views/config/index.vue') },
        { path: 'logs', name: 'Logs', component: () => import('@/views/logs/index.vue') },
        { path: 'scheduled-tasks', name: 'ScheduledTasks', component: () => import('@/views/scheduled-tasks/index.vue') },
        { path: 'sessions', name: 'Sessions', component: () => import('@/views/sessions/index.vue') },
        { path: 'mcp', name: 'MCP', component: () => import('@/views/mcp/index.vue') },
        { path: 'prompts', name: 'Prompts', component: () => import('@/views/prompts/index.vue') },
        { path: 'skills', name: 'Skills', component: () => import('@/views/skills/index.vue') },
        { path: 'plugins', name: 'Plugins', component: () => import('@/views/plugins/index.vue') },
        { path: ':pathMatch(.*)*', redirect: { name: 'Dashboard' } }
      ]
    }
  ]
})

export default router
