import type zhCN from '../zh-CN'
import common from './common'
import layout from './layout'
import dict from './dict'
import dashboard from './dashboard'
import providers from './providers'
import config from './config'
import logs from './logs'
import sessions from './sessions'
import skills from './skills'
import scheduledTasks from './scheduledTasks'
import mcp from './mcp'
import prompts from './prompts'
import plugins from './plugins'
import agents from './agents'
import updater from './updater'

// `satisfies` 保证英文词条与中文词条的键结构完全一致（缺键/多键都会在类型检查时报错）
export default {
  common,
  layout,
  dict,
  dashboard,
  providers,
  config,
  logs,
  sessions,
  skills,
  scheduledTasks,
  mcp,
  prompts,
  plugins,
  agents,
  updater
} satisfies typeof zhCN
