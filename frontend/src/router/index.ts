import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: MainLayout,
      children: [
        {
          path: '',
          name: 'Dashboard',
          component: () => import('@/views/dashboard/index.vue')
        },
        {
          path: 'providers',
          name: 'Providers',
          component: () => import('@/views/providers/index.vue')
        },
        {
          path: 'sessions',
          name: 'Sessions',
          component: () => import('@/views/sessions/index.vue')
        },
        {
          path: 'config',
          name: 'Config',
          component: () => import('@/views/config/index.vue')
        },
        {
          path: 'logs',
          name: 'Logs',
          component: () => import('@/views/logs/index.vue')
        },
        {
          path: 'mcp',
          name: 'MCP',
          component: () => import('@/views/mcp/index.vue')
        },
        {
          path: 'prompts',
          name: 'Prompts',
          component: () => import('@/views/prompts/index.vue')
        }
      ]
    }
  ]
})

export default router
