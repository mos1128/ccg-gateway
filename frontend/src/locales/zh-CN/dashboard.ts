export default {
  title: '数据概览',
  refresh: {
    refreshing: '正在刷新',
    paused: '已停止刷新',
    auto: '自动更新'
  },
  date: {
    start: '开始日期',
    end: '结束日期',
    last7: '近七天',
    last14: '近14天',
    thisMonth: '本月',
    lastMonth: '上月',
    maxDays: '日期范围最多 {days} 天'
  },
  agent: {
    manageTip: '管理 Agent 显示',
    modeHelpTitle: '模式说明',
    modeRouteTitle: '路由：',
    modeRoute: '写入网关地址，Agent 请求会经过 CCG Gateway，并按服务商规则路由。',
    modeDirectTitle: '直连：',
    modeDirect: '写入默认服务商配置，Agent 直接请求该服务商，不经过网关路由。',
    modeOfficialTitle: '官方：',
    modeOfficial: '写入官方凭证，Agent 直接连接官方服务。',
    modeDisabledTitle: '停用：',
    modeDisabled: '清除已写入的路由配置，Agent 不受 CCG Gateway 管理。',
    abilitiesView: '查看 {name} 能力',
    abilitiesTitle: '{name} 能力',
    abilitiesFallbackTitle: 'Agent 能力',
    protocols: '支持协议',
    available: '可用',
    unavailable: '不可用',
    visibilityTitle: 'Agent 显示',
    visibilityHint: '关闭后会从常用页面隐藏，不会修改 Agent 的路由或凭证配置。',
    visibilitySwitch: '{name} 显示开关',
    visibilityKeepOne: '至少保留一个 Agent 显示',
    unsupportedRoute: '该 Agent 不支持自动写入网关配置',
    unsupportedDirect: '该 Agent 不支持自动写入服务商直连配置',
    unsupportedOfficial: '该 Agent 不支持托管官方凭证',
    modeUnavailable: '该模式不可用',
    switched: '{name} 已切换至 {mode}',
    switchFailed: '切换失败: {message}'
  },
  kpi: {
    billedInput: '计费 Token · 输入',
    billedOutput: '计费 Token · 输出',
    cacheRead: '缓存 Token · 读取',
    cacheCreate: '缓存 Token · 创建',
    cacheHitRate: '缓存命中率',
    requests: '全部请求数',
    cost: '费用'
  },
  chart: {
    tokens: 'Token',
    requests: '请求数',
    provider: '服务商',
    model: '模型',
    summary: '汇总',
    input: '输入',
    output: '输出',
    cacheRead: '缓存读取',
    cacheCreate: '缓存创建'
  },
  refreshFailed: '数据刷新失败'
}
