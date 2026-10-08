export default {
  title: 'Overview',
  refresh: {
    refreshing: 'Refreshing',
    paused: 'Refresh paused',
    auto: 'Auto refresh'
  },
  date: {
    start: 'Start date',
    end: 'End date',
    last7: 'Last 7 days',
    last14: 'Last 14 days',
    thisMonth: 'This month',
    lastMonth: 'Last month',
    maxDays: 'Date range is limited to {days} days'
  },
  agent: {
    manageTip: 'Manage agent visibility',
    modeHelpTitle: 'Mode guide',
    modeRouteTitle: 'Route:',
    modeRoute: 'Writes the gateway address; agent requests go through CCG Gateway and are routed by provider rules.',
    modeDirectTitle: 'Direct:',
    modeDirect: 'Writes the default provider configuration; the agent calls that provider directly without gateway routing.',
    modeOfficialTitle: 'Official:',
    modeOfficial: 'Writes official credentials; the agent connects to the official service directly.',
    modeDisabledTitle: 'Off:',
    modeDisabled: 'Clears the written routing configuration; the agent is no longer managed by CCG Gateway.',
    abilitiesView: 'View {name} capabilities',
    abilitiesTitle: '{name} Capabilities',
    abilitiesFallbackTitle: 'Agent Capabilities',
    protocols: 'Supported protocols',
    available: 'Available',
    unavailable: 'Unavailable',
    visibilityTitle: 'Agent Visibility',
    visibilityHint: 'Hidden agents disappear from common pages; their routing and credential configuration is not modified.',
    visibilitySwitch: '{name} visibility switch',
    visibilityKeepOne: 'At least one agent must stay visible',
    unsupportedRoute: 'This agent does not support automatic gateway configuration',
    unsupportedDirect: 'This agent does not support automatic provider-direct configuration',
    unsupportedOfficial: 'This agent does not support managed official credentials',
    modeUnavailable: 'This mode is unavailable',
    switched: '{name} switched to {mode}',
    switchFailed: 'Switch failed: {message}'
  },
  kpi: {
    billedInput: 'Billed Tokens · Input',
    billedOutput: 'Billed Tokens · Output',
    cacheRead: 'Cache Tokens · Read',
    cacheCreate: 'Cache Tokens · Write',
    cacheHitRate: 'Cache Hit Rate',
    requests: 'Total Requests',
    cost: 'Cost'
  },
  chart: {
    tokens: 'Tokens',
    requests: 'Requests',
    provider: 'Provider',
    model: 'Model',
    summary: 'Total',
    input: 'Input',
    output: 'Output',
    cacheRead: 'Cache read',
    cacheCreate: 'Cache write'
  },
  refreshFailed: 'Failed to refresh data'
}
