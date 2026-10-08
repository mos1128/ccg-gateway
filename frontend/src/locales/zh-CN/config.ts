export default {
  cli: {
    title: 'Agent 运行配置',
    configDir: 'Agent 目录',
    configDirPlaceholder: 'Agent 配置目录',
    restoreDefault: '恢复默认'
  },
  preset: {
    title: '全局预设',
    clickToEdit: '点击编辑配置',
    editTitle: '编辑全局预设 - {name}',
    format: '格式化',
    writeMode: {
      merge: '增量合并',
      overwrite: '全量写入'
    },
    writeModeTip: {
      title: '配置写入模式',
      mergeDesc: '只写入需要变更的字段，保留配置文件中已有的其他配置（如 MCP / plugin 开关等配置）。',
      overwriteDesc: '每次写入时完全覆盖配置文件。中转路由会备份原始文件，关闭时自动恢复。保持配置干净。'
    }
  },
  backup: {
    title: '备份与同步',
    local: '本地备份',
    export: '导出',
    import: '导入',
    webdavSettings: 'WebDAV 设置',
    exportToWebdav: '导出到 WebDAV',
    importFromWebdav: '从 WebDAV 导入'
  },
  webdav: {
    url: '服务器地址',
    username: '用户名',
    password: '密码',
    showPassword: '显示密码',
    hidePassword: '隐藏密码',
    testConnection: '测试链接',
    testing: '测试中...',
    saving: '保存中...'
  },
  webdavList: {
    title: '管理 WebDAV 备份',
    loading: '读取中',
    count: '共 {count} 个备份文件',
    empty: '暂无备份文件',
    refreshing: '刷新中',
    file: '备份文件',
    modified: '修改时间',
    size: '大小',
    actions: '操作',
    restore: '恢复',
    emptyHint: '暂无备份文件，请先执行「导出到 WebDAV」'
  },
  timeout: {
    title: '请求超时',
    streamFirstByte: '流式首字节超时',
    streamIdle: '流式空闲超时',
    nonStream: '非流式超时',
    unitSeconds: '秒'
  },
  basic: {
    title: '基础配置',
    launchOnStartup: '开机自启',
    launchOnStartupDesc: '登录系统后自动启动 CCG Gateway',
    silentStartup: '静默启动',
    silentStartupDesc: '启动时不显示主窗口，仅在托盘运行',
    minimizeToTray: '关闭时最小化到托盘',
    minimizeToTrayDesc: '点关闭按钮时隐藏窗口，应用继续后台运行'
  },
  msg: {
    presetInvalid: '配置格式错误，请修正后再保存',
    presetSaved: '预设配置已保存',
    saveFailed: '保存失败',
    updateFailed: '更新失败',
    configDirUpdated: 'Agent 目录已更新',
    timeoutsSaved: '请求超时已保存',
    basicSaved: '基础配置已保存',
    webdavSaved: 'WebDAV 配置已保存',
    exportSuccess: '导出成功',
    exportFailed: '导出失败',
    importConfirm: '导入将覆盖当前所有数据，确定继续？',
    importSuccess: '导入成功，应用将自动退出，请重新打开应用',
    importFailed: '导入失败',
    deleteConfirm: '确定要删除远程备份 {filename} 吗？',
    deleted: '已删除',
    deleteFailed: '删除失败',
    connectSuccess: '连接成功',
    connectFailed: '连接失败',
    syncSuccess: '同步成功: {filename}',
    syncFailed: '同步失败',
    listFailed: '读取备份列表失败',
    yamlInvalid: 'YAML 格式错误',
    tomlInvalid: 'TOML 格式错误'
  }
}
