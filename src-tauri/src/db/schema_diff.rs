use super::schema_definition::{DatabaseSchema, TableDefinition};
use super::schema_inspector::SchemaInspector;
use std::collections::HashSet;

/// 结构变更类型
#[derive(Debug)]
pub enum SchemaChange {
    /// 删除表
    DropTable { name: String },

    /// 创建表
    CreateTable { definition: TableDefinition },

    /// 重建表（表结构有变化）
    RebuildTable { name: String },
}

/// 结构差异
pub struct SchemaDiff {
    pub changes: Vec<SchemaChange>,
}

impl SchemaDiff {
    /// 对比新旧结构，生成变更清单（异步版本）
    pub async fn compare_async(
        expected: &DatabaseSchema,
        actual_tables: HashSet<String>,
        inspector: &SchemaInspector<'_>,
    ) -> Result<Self, sqlx::Error> {
        let mut changes = Vec::new();

        // 1. 找出需要删除的表（实际存在但期望中不存在）
        for actual_table in &actual_tables {
            if !expected.tables.contains_key(actual_table) {
                tracing::info!("表 {} 将被删除", actual_table);
                changes.push(SchemaChange::DropTable {
                    name: actual_table.clone(),
                });
            }
        }

        // 2. 处理每个期望的表
        for (table_name, expected_table) in &expected.tables {
            if !actual_tables.contains(table_name) {
                // 表不存在，需要创建
                tracing::info!("表 {} 将被创建", table_name);
                changes.push(SchemaChange::CreateTable {
                    definition: expected_table.clone(),
                });
            } else {
                // 表存在，通过比较 CREATE TABLE SQL 检查结构是否有变化
                let expected_sql = expected_table.to_create_sql();
                let actual_sql = inspector.get_create_table_sql(table_name).await?;

                if let Some(actual_sql) = actual_sql {
                    if Self::table_structure_differs(&expected_sql, &actual_sql) {
                        tracing::info!(
                            "表 {} 的结构有变化，将被重建\n期望: {}\n实际: {}",
                            table_name,
                            Self::normalize_sql(&expected_sql),
                            Self::normalize_sql(&actual_sql)
                        );
                        changes.push(SchemaChange::RebuildTable {
                            name: table_name.clone(),
                        });
                    }
                }
            }
        }

        Ok(Self { changes })
    }

    /// 获取变更数量
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// 是否有变更
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// SQL 标准化：去除引号、标准化空白字符
    fn normalize_sql(sql: &str) -> String {
        sql.replace('"', "") // 1. 去除双引号
            .replace("IF NOT EXISTS", "") // 2. 去除 IF NOT EXISTS
            .split_whitespace() // 3. 标准化空白字符
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    /// 比较两个 CREATE TABLE SQL 是否不同（忽略大小写）
    fn table_structure_differs(expected_sql: &str, actual_sql: &str) -> bool {
        let normalized_expected = Self::normalize_sql(expected_sql);
        let normalized_actual = Self::normalize_sql(actual_sql);

        // 忽略大小写比较
        !normalized_expected.eq_ignore_ascii_case(&normalized_actual)
    }
}
