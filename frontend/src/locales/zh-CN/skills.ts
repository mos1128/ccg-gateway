export default {
  tabs: {
    skills: '技能',
    repos: '仓库',
    favorites: '收藏'
  },
  invalid: '失效',
  favorite: '收藏',
  unfavorite: '取消收藏',
  removeFavorite: '移除收藏',
  install: '安装',
  installing: '安装中…',
  reinstall: '重装',
  uninstall: '卸载',
  searchPlaceholder: '搜索…',
  repo: {
    add: '添加仓库',
    addTitle: '添加技能仓库',
    url: '仓库地址',
    urlPlaceholder: 'owner/repo 或本地路径',
    urlHint: '支持 GitHub 仓库 URL 或本地目录路径。',
    back: '返回'
  },
  empty: {
    installedTitle: '还没有已安装技能',
    installedDesc: '到「仓库」或「收藏」里安装技能，启用后即可应用到各 Agent',
    noMatch: '无匹配结果',
    repoEmpty: '该仓库暂无 Skills',
    favoritesTitle: '还没有收藏技能',
    favoritesDesc: '收藏会保留来源仓库，方便随时一键安装'
  },
  confirm: {
    uninstallTitle: '确认卸载',
    uninstallMsg: '确定卸载技能 "{name}"?',
    reinstallTitle: '确认重装',
    reinstallMsg: '确定重装 "{name}"? (将更新为最新版本)',
    reinstallSkillMsg: '确定重装技能 "{name}"?',
    removeRepoTitle: '确认删除',
    removeRepoMsg: '确定删除仓库 "{name}" 并卸载该仓库下所有已安装技能？'
  },
  msg: {
    loadFailed: '加载失败',
    installSuccess: '安装成功',
    installFailed: '安装失败',
    reinstallSuccess: '重装成功',
    reinstallFailed: '重装失败',
    missingRepo: '缺少仓库信息，无法重装',
    uninstalled: '已卸载',
    uninstallFailed: '卸载失败',
    listRefreshed: '已获取最新列表',
    refreshFailed: '刷新失败',
    updated: '已更新',
    updateFailed: '更新失败',
    favorited: '已收藏',
    unfavorited: '已取消收藏',
    removed: '已移除',
    descriptionCopied: '描述已复制',
    copyFailed: '复制失败',
    repoUrlRequired: '请输入仓库地址',
    added: '添加成功',
    addFailed: '添加失败',
    deleted: '已删除',
    deleteFailed: '删除失败'
  }
}
