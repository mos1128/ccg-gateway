pub mod models;
pub mod schema_definition;
pub mod schema_diff;
pub mod schema_inspector;
pub mod schema_migrator;

use crate::services::skill;
use crate::time::now_timestamp;
use schema_definition::DatabaseSchema;
use schema_diff::SchemaDiff;
use schema_inspector::SchemaInspector;
use schema_migrator::SchemaMigrator;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

fn sqlite_pool_options() -> SqlitePoolOptions {
    SqlitePoolOptions::new()
        .max_connections(5)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 2000")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
}

pub async fn init_db(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    // 1. 确保父目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // 2. 连接数据库
    let db_url = format!("sqlite:{}?mode=rwc", path.display());
    let pool = sqlite_pool_options().connect(&db_url).await?;

    // 3. 判断数据库类型
    let is_log_db = path.ends_with("ccg_logs.db") || path.ends_with("ccg_logs");

    // 4. 获取期望的 schema
    let expected_schema = if is_log_db {
        DatabaseSchema::log_schema()
    } else {
        DatabaseSchema::current()
    };

    // 5. 创建检查器
    let inspector = SchemaInspector::new(&pool);

    // 6. 检查是否是全新数据库
    if inspector.is_empty_database().await? {
        tracing::info!("检测到全新数据库，创建表结构...");
        create_fresh_database(&pool, &expected_schema).await?;

        // 插入默认数据（仅主数据库）
        if !is_log_db {
            init_default_data(&pool).await?;
        }

        return Ok(pool);
    }

    // 7. 检查版本
    let current_version = inspector.get_version().await?;
    tracing::info!(
        "数据库当前版本: {}, 期望版本: {}",
        current_version,
        expected_schema.version
    );

    // 8. 版本检查
    if current_version >= expected_schema.version {
        create_schema_indexes(&pool, &expected_schema).await?;
        tracing::info!("数据库已是最新版本，跳过迁移");
        return Ok(pool);
    }

    // 9. 需要迁移
    tracing::info!("检测到数据库版本过旧，开始自动迁移...");

    // 10. 读取实际结构
    let actual_tables = inspector.get_tables().await?;

    // 11. 对比差异（通过 SQL 比较）
    let diff = SchemaDiff::compare_async(&expected_schema, actual_tables, &inspector).await?;

    // 12. 应用变更
    if diff.has_changes() {
        tracing::info!("检测到 {} 个结构变更，开始迁移...", diff.change_count());
        let migrator = SchemaMigrator::new(&pool, &expected_schema);
        migrator.apply(diff).await?;
        tracing::info!("数据库迁移完成");
    }

    create_schema_indexes(&pool, &expected_schema).await?;

    // 13. 更新版本
    update_version(&pool, expected_schema.version).await?;

    // 14. 插入默认数据（仅主数据库）
    if !is_log_db {
        init_default_data(&pool).await?;
    }

    tracing::info!("数据库迁移完成");
    Ok(pool)
}

pub async fn init_stats_db(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db_url = format!("sqlite:{}?mode=rwc", path.display());
    let pool = sqlite_pool_options().connect(&db_url).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usage_daily_model (
            usage_date TEXT NOT NULL,
            cli_type TEXT NOT NULL,
            provider_name TEXT NOT NULL,
            model_id TEXT NOT NULL,
            request_count INTEGER NOT NULL DEFAULT 0,
            success_count INTEGER NOT NULL DEFAULT 0,
            failure_count INTEGER NOT NULL DEFAULT 0,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read_input_tokens INTEGER NOT NULL DEFAULT 0,
            cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            elapsed_ms INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (usage_date, cli_type, provider_name, model_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query("DROP TABLE IF EXISTS stats_meta")
        .execute(&pool)
        .await?;

    create_version_table(&pool).await?;
    update_version(&pool, 3).await?;

    Ok(pool)
}

/// 创建全新数据库
async fn create_fresh_database(
    pool: &SqlitePool,
    schema: &DatabaseSchema,
) -> Result<(), sqlx::Error> {
    // 创建所有表
    for sql in schema.to_create_all_sql() {
        sqlx::query(&sql).execute(pool).await?;
    }

    // 创建版本表
    create_version_table(pool).await?;

    // 记录版本
    update_version(pool, schema.version).await?;

    tracing::info!("全新数据库创建完成，版本: {}", schema.version);
    Ok(())
}

async fn create_schema_indexes(
    pool: &SqlitePool,
    schema: &DatabaseSchema,
) -> Result<(), sqlx::Error> {
    for index in &schema.indexes {
        sqlx::query(&index.to_create_sql()).execute(pool).await?;
    }
    Ok(())
}

/// 创建版本表
async fn create_version_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _schema_version (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新版本号
async fn update_version(pool: &SqlitePool, version: i64) -> Result<(), sqlx::Error> {
    // 先创建版本表（如果不存在）
    create_version_table(pool).await?;

    let now = now_timestamp();
    sqlx::query("INSERT OR REPLACE INTO _schema_version (version, applied_at) VALUES (?, ?)")
        .bind(version)
        .bind(now)
        .execute(pool)
        .await?;

    tracing::info!("数据库版本已更新为: {}", version);
    Ok(())
}

/// 插入默认配置数据
async fn init_default_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = now_timestamp();

    // gateway_settings
    sqlx::query(
        "INSERT OR IGNORE INTO gateway_settings (id, debug_log, updated_at) VALUES (1, 0, ?)",
    )
    .bind(now)
    .execute(pool)
    .await?;

    // timeout_settings
    sqlx::query(
        "INSERT OR IGNORE INTO timeout_settings (id, stream_first_byte_timeout, stream_idle_timeout, non_stream_timeout, updated_at) VALUES (1, 30, 60, 120, ?)"
    )
    .bind(now)
    .execute(pool)
    .await?;

    // cli_settings（插入默认配置）
    sqlx::query("INSERT OR IGNORE INTO cli_settings (cli_type, default_json_config, updated_at) VALUES ('claude_code', '{\n  \"env\": {},\n  \"permissions\": {}\n}', ?)")
        .bind(now)
        .execute(pool)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO cli_settings (cli_type, default_json_config, updated_at) VALUES ('codex', 'model_reasoning_effort = \"high\"\nmodel_reasoning_summary = \"detailed\"', ?)")
        .bind(now)
        .execute(pool)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO cli_settings (cli_type, default_json_config, updated_at) VALUES ('gemini', '{\n  \"theme\": \"dark\"\n}', ?)")
        .bind(now)
        .execute(pool)
        .await?;

    let _ = skill::ensure_default_skill_repos();

    Ok(())
}
