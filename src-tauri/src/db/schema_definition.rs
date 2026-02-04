use std::collections::HashMap;

/// 列定义
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
}

/// 表定义
#[derive(Debug, Clone)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: Vec<String>,
    pub unique_constraints: Vec<Vec<String>>,
}

impl TableDefinition {
    /// 生成 CREATE TABLE SQL
    pub fn to_create_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", self.name);

        // 列定义
        let column_defs: Vec<String> = self.columns
            .iter()
            .map(|col| {
                let mut parts = vec![col.name.clone(), col.data_type.clone()];

                if !col.nullable {
                    parts.push("NOT NULL".to_string());
                }

                if let Some(ref default) = col.default_value {
                    parts.push(format!("DEFAULT {}", default));
                }

                format!("    {}", parts.join(" "))
            })
            .collect();

        sql.push_str(&column_defs.join(",\n"));

        // 主键
        if !self.primary_key.is_empty() {
            sql.push_str(",\n    PRIMARY KEY (");
            sql.push_str(&self.primary_key.join(", "));
            sql.push(')');
        }

        // 唯一约束
        for unique_cols in &self.unique_constraints {
            sql.push_str(",\n    UNIQUE(");
            sql.push_str(&unique_cols.join(", "));
            sql.push(')');
        }

        sql.push_str("\n)");
        sql
    }
}

/// 数据库 Schema
#[derive(Debug, Clone)]
pub struct DatabaseSchema {
    pub version: i64,
    pub tables: HashMap<String, TableDefinition>,
}

impl DatabaseSchema {
    /// 获取当前主数据库 Schema
    pub fn current() -> Self {
        Self {
            version: 6,
            tables: Self::define_main_tables(),
        }
    }

    /// 获取日志数据库 Schema
    pub fn log_schema() -> Self {
        Self {
            version: 5,
            tables: Self::define_log_tables(),
        }
    }

    /// 生成所有表的 CREATE SQL
    pub fn to_create_all_sql(&self) -> Vec<String> {
        self.tables.values().map(|table| table.to_create_sql()).collect()
    }

    /// 定义主数据库表
    fn define_main_tables() -> HashMap<String, TableDefinition> {
        let mut tables = HashMap::new();

        // providers 表
        tables.insert(
            "providers".to_string(),
            TableDefinition {
                name: "providers".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "cli_type".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: Some("'claude_code'".to_string()),
                    },
                    ColumnDefinition {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "base_url".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "api_key".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "enabled".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("1".to_string()),
                    },
                    ColumnDefinition {
                        name: "failure_threshold".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("3".to_string()),
                    },
                    ColumnDefinition {
                        name: "blacklist_minutes".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("10".to_string()),
                    },
                    ColumnDefinition {
                        name: "consecutive_failures".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "blacklisted_until".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "sort_order".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "created_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![vec!["cli_type".to_string(), "name".to_string()]],
            },
        );

        // provider_model_map 表
        tables.insert(
            "provider_model_map".to_string(),
            TableDefinition {
                name: "provider_model_map".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "provider_id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "source_model".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "target_model".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "enabled".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("1".to_string()),
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![vec![
                    "provider_id".to_string(),
                    "source_model".to_string(),
                ]],
            },
        );

        // gateway_settings 表
        tables.insert(
            "gateway_settings".to_string(),
            TableDefinition {
                name: "gateway_settings".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("1".to_string()),
                    },
                    ColumnDefinition {
                        name: "debug_log".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![],
            },
        );

        // timeout_settings 表
        tables.insert(
            "timeout_settings".to_string(),
            TableDefinition {
                name: "timeout_settings".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("1".to_string()),
                    },
                    ColumnDefinition {
                        name: "stream_first_byte_timeout".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("30".to_string()),
                    },
                    ColumnDefinition {
                        name: "stream_idle_timeout".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("60".to_string()),
                    },
                    ColumnDefinition {
                        name: "non_stream_timeout".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("120".to_string()),
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![],
            },
        );

        // cli_settings 表
        tables.insert(
            "cli_settings".to_string(),
            TableDefinition {
                name: "cli_settings".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "cli_type".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "default_json_config".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["cli_type".to_string()],
                unique_constraints: vec![],
            },
        );

        // mcp_configs 表
        tables.insert(
            "mcp_configs".to_string(),
            TableDefinition {
                name: "mcp_configs".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "config_json".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![vec!["name".to_string()]],
            },
        );

        // prompt_presets 表
        tables.insert(
            "prompt_presets".to_string(),
            TableDefinition {
                name: "prompt_presets".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "content".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![vec!["name".to_string()]],
            },
        );

        // webdav_settings 表
        tables.insert(
            "webdav_settings".to_string(),
            TableDefinition {
                name: "webdav_settings".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("1".to_string()),
                    },
                    ColumnDefinition {
                        name: "url".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "username".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "password".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "path".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "enabled".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "updated_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![],
            },
        );

        // skill_repos 表（Skill 仓库列表）
        tables.insert(
            "skill_repos".to_string(),
            TableDefinition {
                name: "skill_repos".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "owner".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "branch".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: Some("'main'".to_string()),
                    },
                ],
                primary_key: vec!["owner".to_string(), "name".to_string()],
                unique_constraints: vec![],
            },
        );

        // useragent_map 表（全局 User-Agent 映射）
        tables.insert(
            "useragent_map".to_string(),
            TableDefinition {
                name: "useragent_map".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "source_pattern".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "target_value".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "enabled".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("1".to_string()),
                    },
                    ColumnDefinition {
                        name: "sort_order".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![vec!["source_pattern".to_string()]],
            },
        );

        // skill_configs 表（已安装的 Skills）
        tables.insert(
            "skill_configs".to_string(),
            TableDefinition {
                name: "skill_configs".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "description".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "directory".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "repo_owner".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "repo_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "repo_branch".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "readme_url".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "installed_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![vec!["directory".to_string()]],
            },
        );

        tables
    }

    /// 定义日志数据库表
    fn define_log_tables() -> HashMap<String, TableDefinition> {
        let mut tables = HashMap::new();

        // request_logs 表
        tables.insert(
            "request_logs".to_string(),
            TableDefinition {
                name: "request_logs".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "created_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "cli_type".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "provider_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "model_id".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "status_code".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "elapsed_ms".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "input_tokens".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "output_tokens".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "client_method".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "client_path".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "client_headers".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "client_body".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "forward_url".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "forward_headers".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "forward_body".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "provider_headers".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "provider_body".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "error_message".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: true,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![],
            },
        );

        // system_logs 表
        tables.insert(
            "system_logs".to_string(),
            TableDefinition {
                name: "system_logs".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "created_at".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "event_type".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "message".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique_constraints: vec![],
            },
        );

        // usage_daily 表
        tables.insert(
            "usage_daily".to_string(),
            TableDefinition {
                name: "usage_daily".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "usage_date".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "provider_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "cli_type".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "request_count".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "success_count".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "failure_count".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "input_tokens".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                    ColumnDefinition {
                        name: "output_tokens".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: Some("0".to_string()),
                    },
                ],
                primary_key: vec![
                    "usage_date".to_string(),
                    "provider_name".to_string(),
                    "cli_type".to_string(),
                ],
                unique_constraints: vec![],
            },
        );

        tables
    }
}
