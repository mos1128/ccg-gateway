use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

/// 列信息
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
}

/// 数据库结构检查器
pub struct SchemaInspector<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SchemaInspector<'a> {
    /// 创建新的检查器
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取数据库版本
    /// 如果版本表不存在或没有记录，返回 0
    pub async fn get_version(&self) -> Result<i64, sqlx::Error> {
        // 检查版本表是否存在
        let has_version_table: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='_schema_version'",
        )
        .fetch_optional(self.pool)
        .await?;

        if has_version_table.is_none() {
            return Ok(0); // 版本表不存在，视为版本 0
        }

        // 读取版本号
        let version: Option<(i64,)> =
            sqlx::query_as("SELECT version FROM _schema_version ORDER BY version DESC LIMIT 1")
                .fetch_optional(self.pool)
                .await?;

        Ok(version.map(|v| v.0).unwrap_or(0))
    }

    /// 检查是否是全新数据库（没有任何用户表）
    pub async fn is_empty_database(&self) -> Result<bool, sqlx::Error> {
        // 检查是否有版本表，如果有说明不是全新数据库
        let has_version_table: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='_schema_version'",
        )
        .fetch_optional(self.pool)
        .await?;

        if has_version_table.is_some() {
            return Ok(false); // 版本表存在，不是全新数据库
        }

        // 检查是否有其他用户表
        let tables = self.get_tables().await?;
        Ok(tables.is_empty())
    }

    /// 获取所有用户表名（排除系统表和版本表）
    pub async fn get_tables(&self) -> Result<HashSet<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master 
             WHERE type='table' 
             AND name NOT LIKE 'sqlite_%' 
             AND name NOT GLOB '_*'
             ORDER BY name",
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 获取指定表的列信息
    pub async fn get_table_columns(
        &self,
        table_name: &str,
    ) -> Result<Vec<ColumnInfo>, sqlx::Error> {
        // 使用 PRAGMA table_info 获取列信息
        // 返回格式: cid, name, type, notnull, dflt_value, pk
        let query = format!("PRAGMA table_info({})", table_name);
        let rows = sqlx::query(&query).fetch_all(self.pool).await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.get(1); // name
            let data_type: String = row.get(2); // type
            let not_null: i64 = row.get(3); // notnull (0 or 1)
            let default_value: Option<String> = row.try_get(4).ok(); // dflt_value

            columns.push(ColumnInfo {
                name,
                data_type: data_type.to_uppercase(),
                nullable: not_null == 0,
                default_value,
            });
        }

        Ok(columns)
    }

    /// 获取表的主键信息
    #[allow(dead_code)]
    pub async fn get_primary_key(&self, table_name: &str) -> Result<Vec<String>, sqlx::Error> {
        // 使用 PRAGMA table_info 获取主键信息
        // pk 字段：0 表示不是主键，>0 表示主键序号
        let query = format!("PRAGMA table_info({})", table_name);
        let rows = sqlx::query(&query).fetch_all(self.pool).await?;

        let mut primary_keys: Vec<(i64, String)> = Vec::new();
        for row in rows {
            let name: String = row.get(1); // name
            let pk: i64 = row.get(5); // pk
            if pk > 0 {
                primary_keys.push((pk, name));
            }
        }

        // 按主键序号排序
        primary_keys.sort_by_key(|k| k.0);

        Ok(primary_keys.into_iter().map(|k| k.1).collect())
    }

    /// 获取表的 CREATE TABLE SQL 语句
    pub async fn get_create_table_sql(
        &self,
        table_name: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT sql FROM sqlite_master WHERE type='table' AND name=?")
                .bind(table_name)
                .fetch_optional(self.pool)
                .await?;

        Ok(row.map(|r| r.0))
    }
}
