export default {
  cli: {
    title: 'Agent Runtime Configuration',
    configDir: 'Agent Directory',
    configDirPlaceholder: 'Agent config directory',
    restoreDefault: 'Restore Defaults'
  },
  preset: {
    title: 'Global Preset',
    clickToEdit: 'Click to edit configuration',
    editTitle: 'Edit Global Preset - {name}',
    format: 'Format',
    writeMode: {
      merge: 'Merge',
      overwrite: 'Overwrite'
    },
    writeModeTip: {
      title: 'Config Write Mode',
      mergeDesc: 'Only writes the fields that need to change, keeping the existing configuration in the file (such as MCP / plugin toggles).',
      overwriteDesc: 'Fully overwrites the config file on every write. The proxy backs up the original file and restores it automatically when disabled. Keeps the config clean.'
    }
  },
  backup: {
    title: 'Backup & Sync',
    local: 'Local Backup',
    export: 'Export',
    import: 'Import',
    webdavSettings: 'WebDAV Settings',
    exportToWebdav: 'Export to WebDAV',
    importFromWebdav: 'Import from WebDAV'
  },
  webdav: {
    url: 'Server URL',
    username: 'Username',
    password: 'Password',
    showPassword: 'Show password',
    hidePassword: 'Hide password',
    testConnection: 'Test Connection',
    testing: 'Testing...',
    saving: 'Saving...'
  },
  webdavList: {
    title: 'Manage WebDAV Backups',
    loading: 'Loading',
    count: '{count} backup file | {count} backup files',
    empty: 'No backup files',
    refreshing: 'Refreshing',
    file: 'Backup File',
    modified: 'Modified',
    size: 'Size',
    actions: 'Actions',
    restore: 'Restore',
    emptyHint: 'No backup files yet. Run "Export to WebDAV" first.'
  },
  timeout: {
    title: 'Request Timeouts',
    streamFirstByte: 'Streaming first byte timeout',
    streamIdle: 'Streaming idle timeout',
    nonStream: 'Non-streaming timeout',
    unitSeconds: 's'
  },
  basic: {
    title: 'Basic Settings',
    launchOnStartup: 'Launch on Startup',
    launchOnStartupDesc: 'Start CCG Gateway automatically after signing in',
    silentStartup: 'Silent Startup',
    silentStartupDesc: 'Start without showing the main window, running in the tray only',
    minimizeToTray: 'Minimize to Tray on Close',
    minimizeToTrayDesc: 'Hide the window when the close button is clicked, keeping the app running in the background'
  },
  msg: {
    presetInvalid: 'Invalid configuration format. Fix it before saving.',
    presetSaved: 'Preset configuration saved',
    saveFailed: 'Failed to save',
    updateFailed: 'Failed to update',
    configDirUpdated: 'Agent directory updated',
    timeoutsSaved: 'Request timeouts saved',
    basicSaved: 'Basic settings saved',
    webdavSaved: 'WebDAV configuration saved',
    exportSuccess: 'Exported',
    exportFailed: 'Failed to export',
    importConfirm: 'Importing will overwrite all current data. Continue?',
    importSuccess: 'Imported. The app will exit automatically, please reopen it.',
    importFailed: 'Failed to import',
    deleteConfirm: 'Delete the remote backup {filename}?',
    deleted: 'Deleted',
    deleteFailed: 'Failed to delete',
    connectSuccess: 'Connected',
    connectFailed: 'Connection failed',
    syncSuccess: 'Synced: {filename}',
    syncFailed: 'Failed to sync',
    listFailed: 'Failed to load the backup list',
    yamlInvalid: 'YAML format error',
    tomlInvalid: 'TOML format error'
  }
}
