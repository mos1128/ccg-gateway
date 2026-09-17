pub mod models;
pub mod schema_definition;
pub mod schema_diff;
pub mod schema_inspector;
pub mod schema_migrator;

use crate::services::{agent, skill};
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
        } else {
            recover_unfinished_request_logs(&pool).await?;
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
        if is_log_db {
            recover_unfinished_request_logs(&pool).await?;
        } else {
            init_default_data(&pool).await?;
        }
        tracing::info!("数据库已是最新版本，跳过迁移");
        return Ok(pool);
    }

    // 9. 需要迁移
    tracing::info!("检测到数据库版本过旧，开始自动迁移...");

    // 10. 阶梯熔断上线（v42）：providers 表的旧熔断列会被 rebuild 删除，
    // 旧值必须先搬进档位表，回填必须发生在结构迁移之前。
    // current_version < 42 的库才存在这些旧列，之后版本再跑会因列不存在而报错。
    if !is_log_db && current_version < 42 {
        backfill_blacklist_tiers_before_rebuild(&pool, &expected_schema).await?;
    }

    // 11. 读取实际结构
    let actual_tables = inspector.get_tables().await?;

    // 12. 对比差异（通过 SQL 比较）
    let diff = SchemaDiff::compare_async(&expected_schema, actual_tables, &inspector).await?;

    // 13. 应用变更
    if diff.has_changes() {
        tracing::info!("检测到 {} 个结构变更，开始迁移...", diff.change_count());
        let migrator = SchemaMigrator::new(&pool, &expected_schema);
        migrator.apply(diff).await?;
        tracing::info!("数据库迁移完成");
    }

    if !is_log_db && current_version < 32 {
        migrate_provider_protocols(&pool).await?;
    }

    create_schema_indexes(&pool, &expected_schema).await?;

    // 13. 更新版本
    update_version(&pool, expected_schema.version).await?;

    // 14. 插入默认数据（仅主数据库）
    if !is_log_db {
        init_default_data(&pool).await?;
    } else {
        recover_unfinished_request_logs(&pool).await?;
    }

    tracing::info!("数据库迁移完成");
    Ok(pool)
}

async fn recover_unfinished_request_logs(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = now_timestamp();
    sqlx::query(
        "UPDATE request_logs
         SET finished_at = created_at + ((elapsed_ms + 999) / 1000)
         WHERE finished_at IS NULL
           AND (elapsed_ms > 0 OR status_code IS NOT NULL OR error_message IS NOT NULL
                OR input_tokens > 0 OR cache_read_input_tokens > 0
                OR cache_creation_input_tokens > 0 OR output_tokens > 0)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE request_logs
         SET finished_at = ?, elapsed_ms = MAX(0, (? - created_at) * 1000),
             error_message = COALESCE(error_message, 'Gateway stopped before request completed')
         WHERE finished_at IS NULL",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
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
            total_cost REAL,
            PRIMARY KEY (usage_date, cli_type, provider_name, model_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // 这张表不走 schema 差异迁移，加列要自己来。费用快照是后加的，升级前的历史行
    // 没有价格可回溯，直接按 0 处理（用户只关心当前和未来的费用）。
    let has_total_cost: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('usage_daily_model') WHERE name = 'total_cost'",
    )
    .fetch_one(&pool)
    .await?;
    if has_total_cost == 0 {
        sqlx::query("ALTER TABLE usage_daily_model ADD COLUMN total_cost REAL")
            .execute(&pool)
            .await?;
    }
    sqlx::query("UPDATE usage_daily_model SET total_cost = 0 WHERE total_cost IS NULL")
        .execute(&pool)
        .await?;

    sqlx::query("DROP TABLE IF EXISTS stats_meta")
        .execute(&pool)
        .await?;

    create_version_table(&pool).await?;
    update_version(&pool, 4).await?;

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
        "INSERT OR IGNORE INTO gateway_settings (id, debug_log, log_detail_mode, launch_on_startup, silent_startup, minimize_to_tray_on_close, updated_at) VALUES (1, 0, 'failure_only', 0, 0, 1, ?)",
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

    // 为内置和用户提供的有效 Agent 定义补齐设置行。
    for definition in agent::definitions() {
        sqlx::query(
            "INSERT OR IGNORE INTO cli_settings
                (cli_type, default_json_config, updated_at)
             VALUES (?, ?, ?)",
        )
        .bind(&definition.id)
        .bind("")
        .bind(now)
        .execute(pool)
        .await?;
    }

    // 服务商子表的孤儿行清理：删服务商时若漏掉某张子表，残留行会一直留着，
    // 而 SQLite 复用 rowid（新 id = MAX(id) + 1），下一个新服务商拿到同一个 id
    // 就撞上残留行的唯一约束，从此再也建不出服务商。孤儿行没有任何引用价值，
    // 每次启动清一遍。必须排在下面补档位之前，否则残留档位会被当成已有档位。
    //
    // TODO(v2.2+): 这是为了清理 v2.1.0 之前遗留的孤儿数据（删 Profile 时漏删
    // provider_blacklist_tier 导致的历史垃圾）。v2.1.0 已修复删除逻辑，v2.2 发布
    // 3 个月后，假定所有用户已升级并清理完毕，可以删除这段启动清理代码，仅保留
    // 删除路径的完整性。如果后续发现其他删除路径仍有遗漏，可保留此防御机制。
    for table in [
        "provider_model_map",
        "provider_model_blacklist",
        "provider_blacklist_tier",
        "provider_models",
        "provider_model_sync_state",
    ] {
        sqlx::query(&format!(
            "DELETE FROM {} WHERE provider_id NOT IN (SELECT id FROM providers)",
            table
        ))
        .execute(pool)
        .await?;
    }

    // 阶梯熔断兜底：为还没有档位的服务商补默认一档（5 次拉黑 10 分钟）。
    // 档位正常由创建/编辑服务商维护，这里只兜异常情况（如手动改库清空档位）。
    // NOT IN 保证幂等，每次启动自愈。
    sqlx::query(
        r#"
        INSERT INTO provider_blacklist_tier
            (provider_id, failure_count, blacklist_minutes, created_at, updated_at)
        SELECT id, 5, 10, ?, ?
        FROM providers
        WHERE id NOT IN (SELECT provider_id FROM provider_blacklist_tier)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    let _ = skill::ensure_default_skill_repos();

    Ok(())
}

async fn migrate_provider_protocols(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for definition in agent::definitions() {
        let Some(protocol) = definition.protocols.first() else {
            continue;
        };
        sqlx::query("UPDATE providers SET protocol = ? WHERE cli_type = ?")
            .bind(protocol.as_str())
            .bind(&definition.id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// v42 阶梯熔断上线：providers 表的旧熔断列（failure_threshold/
/// blacklist_minutes）会被结构迁移的 rebuild 删除，删掉的值无法找回，
/// 所以在迁移前先把旧值搬进 provider_blacklist_tier 作为默认一档。
/// 表不存在时按期望结构先建（to_create_sql 自带 IF NOT EXISTS）。
async fn backfill_blacklist_tiers_before_rebuild(
    pool: &SqlitePool,
    expected_schema: &DatabaseSchema,
) -> Result<(), sqlx::Error> {
    let now = now_timestamp();

    if let Some(table) = expected_schema.tables.get("provider_blacklist_tier") {
        sqlx::query(&table.to_create_sql()).execute(pool).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO provider_blacklist_tier
            (provider_id, failure_count, blacklist_minutes, created_at, updated_at)
        SELECT id, MAX(failure_threshold, 1), MAX(blacklist_minutes, 1), ?, ?
        FROM providers
        WHERE id NOT IN (SELECT provider_id FROM provider_blacklist_tier)
        "#,
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}
