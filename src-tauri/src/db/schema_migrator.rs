use super::schema_definition::{DatabaseSchema, TableDefinition};
use super::schema_diff::{SchemaChange, SchemaDiff};
use super::schema_inspector::SchemaInspector;
use sqlx::SqlitePool;

/// 迁移执行器
pub struct SchemaMigrator<'a> {
    pool: &'a SqlitePool,
    expected_schema: &'a DatabaseSchema,
}

impl<'a> SchemaMigrator<'a> {
    /// 创建新的迁移执行器
    pub fn new(pool: &'a SqlitePool, expected_schema: &'a DatabaseSchema) -> Self {
        Self {
            pool,
            expected_schema,
        }
    }

    /// 应用所有变更（使用事务确保原子性）
    pub async fn apply(&self, diff: SchemaDiff) -> Result<(), sqlx::Error> {
        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 处理所有变更
        for change in diff.changes {
            match change {
                SchemaChange::DropTable { name } => {
                    self.drop_table_tx(&mut tx, &name).await?;
                }
                SchemaChange::CreateTable { definition } => {
                    self.create_table_tx(&mut tx, &definition).await?;
                }
                SchemaChange::RebuildTable { name } => {
                    self.rebuild_table_tx(&mut tx, &name).await?;
                }
            }
        }

        // 提交事务
        tx.commit().await?;
        Ok(())
    }

    /// 删除表（事务版本）
    async fn drop_table_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        name: &str,
    ) -> Result<(), sqlx::Error> {
        tracing::info!("删除表: {}", name);
        let sql = format!("DROP TABLE IF EXISTS {}", name);
        sqlx::query(&sql).execute(&mut **tx).await?;
        Ok(())
    }

    /// 创建表（事务版本）
    async fn create_table_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        definition: &TableDefinition,
    ) -> Result<(), sqlx::Error> {
        tracing::info!("创建表: {}", definition.name);
        let sql = definition.to_create_sql();
        sqlx::query(&sql).execute(&mut **tx).await?;
        Ok(())
    }

    /// 重建表（事务版本）
    /// 用于处理列变更（新增或删除），确保表结构完全符合新定义
    /// 注意：字段重命名会导致数据丢失，字段类型变更可能不符合预期
    async fn rebuild_table_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        table: &str,
    ) -> Result<(), sqlx::Error> {
        tracing::info!("重建表: {}", table);

        // 1. 获取期望的表定义
        let expected_table =
            self.expected_schema.tables.get(table).ok_or_else(|| {
                sqlx::Error::Protocol(format!("表 {} 不在期望结构中", table).into())
            })?;

        // 2. 获取当前表的列信息（用于数据复制）
        let inspector = SchemaInspector::new(self.pool);
        let actual_columns = inspector.get_table_columns(table).await?;

        // 3. 找出新旧表都存在的列（用于数据复制）
        let expected_column_names: Vec<String> = expected_table
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let keep_columns: Vec<String> = actual_columns
            .iter()
            .filter(|c| expected_column_names.contains(&c.name))
            .map(|c| c.name.clone())
            .collect();

        if keep_columns.is_empty() {
            return Err(sqlx::Error::Protocol(
                format!("表 {} 新旧结构没有共同列，无法迁移数据", table).into(),
            ));
        }

        // 4. 重建表
        // 4.1 重命名旧表
        let rename_sql = format!("ALTER TABLE {} RENAME TO {}_old", table, table);
        sqlx::query(&rename_sql).execute(&mut **tx).await?;

        // 4.2 创建新表（使用期望的结构）
        self.create_table_tx(tx, expected_table).await?;

        // 4.3 复制数据（只复制共同列）
        let column_list = keep_columns.join(", ");
        let copy_sql = format!(
            "INSERT INTO {} ({}) SELECT {} FROM {}_old",
            table, column_list, column_list, table
        );
        sqlx::query(&copy_sql).execute(&mut **tx).await?;

        // 4.4 删除旧表
        let drop_sql = format!("DROP TABLE {}_old", table);
        sqlx::query(&drop_sql).execute(&mut **tx).await?;

        tracing::info!("表 {} 重建完成", table);
        Ok(())
    }
}
