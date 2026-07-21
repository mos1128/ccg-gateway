use crate::db::models::{
    Provider, ProviderKeepalivePayload, ScheduledTask, ScheduledTaskCreate, ScheduledTaskResponse,
    ScheduledTaskRun, ScheduledTaskRunItem, ScheduledTaskRunListResponse, ScheduledTaskUpdate,
};
use crate::services::provider as provider_service;
use crate::services::provider_profile::{provider_profile_exists_if_supported, validate_cli_type};
use crate::services::routing::{normalize_profile, DEFAULT_PROFILE};
use crate::time::now_timestamp;
use chrono::{Duration, Local, NaiveTime, TimeZone};
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

const TASK_TYPE_PROVIDER_KEEPALIVE: &str = "provider_keepalive";
const SCHEDULE_TYPE_INTERVAL: &str = "interval";
const SCHEDULE_TYPE_DAILY: &str = "daily";
const SCHEDULER_POLL_SECONDS: u64 = 5;
const SCHEDULER_DUE_TASK_LIMIT: i64 = 10;
const RUNNING_STALE_SECONDS: i64 = 60 * 60;
const RUN_RETENTION_PER_TASK: i64 = 500;
const STATUS_PENDING: &str = "pending";
const STATUS_RUNNING: &str = "running";
const STATUS_SUCCESS: &str = "success";
const STATUS_FAILED: &str = "failed";
const STATUS_PARTIAL_FAILED: &str = "partial_failed";
const STATUS_RETRYING: &str = "retrying";
const STATUS_SKIPPED: &str = "skipped";
const TRIGGER_MANUAL: &str = "manual";
const TRIGGER_SCHEDULED: &str = "scheduled";
const EVENT_SCHEDULED_TASK_CHANGED: &str = "scheduled-task-changed";

type TaskLocks = Arc<Mutex<HashSet<i64>>>;

static TASK_LOCKS: OnceLock<TaskLocks> = OnceLock::new();

#[derive(Clone, Serialize)]
pub struct ScheduledTaskChangeEvent {
    task_id: Option<i64>,
    run_id: Option<i64>,
}

#[derive(Deserialize)]
struct DailyScheduleExpr {
    days: i64,
    hour: u32,
    minute: u32,
}

pub fn emit_task_changed(app_handle: &AppHandle, task_id: Option<i64>, run_id: Option<i64>) {
    let event = ScheduledTaskChangeEvent { task_id, run_id };
    if let Err(e) = app_handle.emit(EVENT_SCHEDULED_TASK_CHANGED, event) {
        tracing::error!(error = %e, "Failed to emit scheduled task event");
    }
}

fn emit_task_changed_if_present(
    app_handle: Option<&AppHandle>,
    task_id: Option<i64>,
    run_id: Option<i64>,
) {
    if let Some(app_handle) = app_handle {
        emit_task_changed(app_handle, task_id, run_id);
    }
}

#[derive(Debug)]
struct KeepaliveTarget {
    provider_id: Option<i64>,
    provider_name: String,
    provider: Option<Provider>,
    skip_reason: Option<String>,
}

#[derive(Debug)]
struct RunOutcome {
    total_count: i64,
    success_count: i64,
    failure_count: i64,
    skipped_count: i64,
    errors: Vec<String>,
}

impl RunOutcome {
    fn empty() -> Self {
        Self {
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            skipped_count: 0,
            errors: Vec::new(),
        }
    }

    fn status(&self) -> &'static str {
        if self.total_count == 0 {
            STATUS_FAILED
        } else if self.skipped_count == self.total_count {
            // 全部跳过：单独状态，不重试
            STATUS_SKIPPED
        } else {
            // 跳过不参与决策，只看成功和失败
            let effective_count = self.success_count + self.failure_count;
            if effective_count == 0 {
                // 理论上不会走到这里（已排除全部跳过），兜底
                STATUS_SKIPPED
            } else if self.failure_count == 0 {
                // 无失败（可能有跳过）：成功
                STATUS_SUCCESS
            } else if self.success_count == 0 {
                // 无成功（可能有跳过）：失败
                STATUS_FAILED
            } else {
                // 有成功有失败：部分失败
                STATUS_PARTIAL_FAILED
            }
        }
    }

    fn error_message(&self) -> Option<String> {
        if self.errors.is_empty() {
            None
        } else {
            Some(truncate_text(&self.errors.join("；"), 2000))
        }
    }

    fn merge(&mut self, other: Self) {
        self.total_count += other.total_count;
        self.success_count += other.success_count;
        self.failure_count += other.failure_count;
        self.skipped_count += other.skipped_count;
        self.errors.extend(other.errors);
    }
}

pub fn start_scheduler(db: SqlitePool, log_db: SqlitePool, app_handle: AppHandle) {
    tokio::spawn(async move {
        // 启动时恢复因崩溃而卡在 running 状态的任务
        if let Err(e) = recover_stuck_tasks(&db, &log_db, None).await {
            tracing::error!(error = %e, "Failed to recover stuck tasks");
        }

        loop {
            if let Err(e) = recover_stuck_tasks(&db, &log_db, Some(RUNNING_STALE_SECONDS)).await {
                tracing::error!(error = %e, "Failed to recover stale running tasks");
            }
            if let Err(e) = run_due_tasks(&db, &log_db, &app_handle).await {
                tracing::error!(error = %e, "Scheduled task tick failed");
            }
            tokio::time::sleep(std::time::Duration::from_secs(SCHEDULER_POLL_SECONDS)).await;
        }
    });
}

/// 应用启动时将卡在 running 状态的任务重置为 failed，并重新计算 next_run_at
async fn recover_stuck_tasks(
    db: &SqlitePool,
    log_db: &SqlitePool,
    stale_after_seconds: Option<i64>,
) -> Result<(), String> {
    let now = now_timestamp();
    let error_message = "应用异常退出，任务中断";
    let mut sql = "SELECT * FROM scheduled_tasks WHERE last_status = ?".to_string();
    if stale_after_seconds.is_some() {
        sql.push_str(" AND last_run_at IS NOT NULL AND last_run_at <= ?");
    }

    let mut query = sqlx::query_as::<_, ScheduledTask>(&sql).bind(STATUS_RUNNING);
    if let Some(seconds) = stale_after_seconds {
        query = query.bind(now - seconds);
    }

    let stuck_tasks = query.fetch_all(db).await.map_err(|e| e.to_string())?;

    if stuck_tasks.is_empty() {
        return Ok(());
    }

    let mut recovered_count = 0;
    for task in &stuck_tasks {
        if stale_after_seconds.is_some() && is_task_locked(task.id).await {
            continue;
        }

        let next_run_at = if task.next_run_at <= now {
            next_run_after(&task.schedule_type, &task.schedule_expr, now).unwrap_or(now + 60)
        } else {
            task.next_run_at
        };

        let result = sqlx::query(
            r#"
            UPDATE scheduled_tasks
            SET last_status = ?, last_error = ?, next_run_at = ?, updated_at = ?
            WHERE id = ? AND last_status = ?
            "#,
        )
        .bind(STATUS_FAILED)
        .bind(error_message)
        .bind(next_run_at)
        .bind(now)
        .bind(task.id)
        .bind(STATUS_RUNNING)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;

        if result.rows_affected() == 0 {
            continue;
        }

        sqlx::query(
            r#"
            UPDATE scheduled_task_runs
            SET status = ?, finished_at = ?, error_message = ?
            WHERE task_id = ? AND status = ? AND finished_at IS NULL
            "#,
        )
        .bind(STATUS_FAILED)
        .bind(now)
        .bind(error_message)
        .bind(task.id)
        .bind(STATUS_RUNNING)
        .execute(log_db)
        .await
        .map_err(|e| e.to_string())?;

        recovered_count += 1;
    }

    if recovered_count > 0 {
        tracing::warn!(count = recovered_count, "Recovered stuck scheduled tasks");
    }
    Ok(())
}

pub async fn list_tasks(db: &SqlitePool) -> Result<Vec<ScheduledTaskResponse>, String> {
    let tasks = sqlx::query_as::<_, ScheduledTask>("SELECT * FROM scheduled_tasks ORDER BY id")
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tasks.into_iter().map(Into::into).collect())
}

pub async fn get_task(db: &SqlitePool, id: i64) -> Result<ScheduledTaskResponse, String> {
    let task = load_task(db, id).await?;
    Ok(task.into())
}

pub async fn create_task(
    db: &SqlitePool,
    input: ScheduledTaskCreate,
) -> Result<ScheduledTaskResponse, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("任务名称不能为空".to_string());
    }

    validate_task_type(&input.task_type)?;
    validate_schedule_type(&input.schedule_type)?;
    validate_payload(&input.task_type, &input.payload_json)?;

    let retry_limit = input.retry_limit.unwrap_or(3);
    let retry_interval_minutes = input.retry_interval_minutes.unwrap_or(10);
    validate_retry(retry_limit, retry_interval_minutes)?;

    let now = now_timestamp();
    let next_run_at = next_run_after(&input.schedule_type, &input.schedule_expr, now)?;
    let enabled = if input.enabled.unwrap_or(true) {
        1i64
    } else {
        0i64
    };

    let result = sqlx::query(
        r#"
        INSERT INTO scheduled_tasks
            (name, task_type, enabled, schedule_type, schedule_expr, payload_json, retry_limit,
             retry_interval_minutes, retry_count, last_run_at, next_run_at, last_status,
             last_error, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?, NULL, ?, ?)
        "#,
    )
    .bind(name)
    .bind(&input.task_type)
    .bind(enabled)
    .bind(&input.schedule_type)
    .bind(input.schedule_expr.trim())
    .bind(input.payload_json.trim())
    .bind(retry_limit)
    .bind(retry_interval_minutes)
    .bind(next_run_at)
    .bind(STATUS_PENDING)
    .bind(now)
    .bind(now)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    get_task(db, result.last_insert_rowid()).await
}

pub async fn update_task(
    db: &SqlitePool,
    id: i64,
    input: ScheduledTaskUpdate,
) -> Result<ScheduledTaskResponse, String> {
    let current = load_task(db, id).await?;

    let name = input.name.unwrap_or_else(|| current.name.clone());
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("任务名称不能为空".to_string());
    }

    let schedule_type = input
        .schedule_type
        .unwrap_or_else(|| current.schedule_type.clone());
    let schedule_expr = input
        .schedule_expr
        .unwrap_or_else(|| current.schedule_expr.clone());
    let payload_json = input
        .payload_json
        .unwrap_or_else(|| current.payload_json.clone());
    let retry_limit = input.retry_limit.unwrap_or(current.retry_limit);
    let retry_interval_minutes = input
        .retry_interval_minutes
        .unwrap_or(current.retry_interval_minutes);

    validate_schedule_type(&schedule_type)?;
    validate_payload(&current.task_type, &payload_json)?;
    validate_retry(retry_limit, retry_interval_minutes)?;

    let now = now_timestamp();
    let enabled = input
        .enabled
        .map(|v| if v { 1i64 } else { 0i64 })
        .unwrap_or(current.enabled);

    let schedule_changed = schedule_type != current.schedule_type
        || schedule_expr != current.schedule_expr
        || payload_json != current.payload_json
        || (current.enabled == 0 && enabled == 1 && current.next_run_at <= now);
    let next_run_at = if schedule_changed {
        next_run_after(&schedule_type, &schedule_expr, now)?
    } else {
        current.next_run_at
    };
    let retry_count = if schedule_changed {
        0
    } else {
        current.retry_count
    };
    let last_status = if schedule_changed && current.last_status == STATUS_RETRYING {
        STATUS_PENDING.to_string()
    } else {
        current.last_status
    };

    sqlx::query(
        r#"
        UPDATE scheduled_tasks
        SET name = ?, enabled = ?, schedule_type = ?, schedule_expr = ?, payload_json = ?,
            retry_limit = ?, retry_interval_minutes = ?, retry_count = ?, next_run_at = ?,
            last_status = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(name)
    .bind(enabled)
    .bind(schedule_type)
    .bind(schedule_expr.trim())
    .bind(payload_json.trim())
    .bind(retry_limit)
    .bind(retry_interval_minutes)
    .bind(retry_count)
    .bind(next_run_at)
    .bind(last_status)
    .bind(now)
    .bind(id)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    get_task(db, id).await
}

pub async fn delete_task(db: &SqlitePool, log_db: &SqlitePool, id: i64) -> Result<(), String> {
    if !try_lock_task(id).await {
        return Err("任务正在执行，稍后再删除".to_string());
    }

    let result = delete_task_locked(db, log_db, id).await;
    unlock_task(id).await;
    result
}

async fn delete_task_locked(db: &SqlitePool, log_db: &SqlitePool, id: i64) -> Result<(), String> {
    let task = load_task(db, id).await?;
    if task.last_status == STATUS_RUNNING {
        return Err("任务正在执行，稍后再删除".to_string());
    }

    sqlx::query("DELETE FROM scheduled_task_run_items WHERE run_id IN (SELECT id FROM scheduled_task_runs WHERE task_id = ?)")
        .bind(id)
        .execute(log_db)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM scheduled_task_runs WHERE task_id = ?")
        .bind(id)
        .execute(log_db)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM scheduled_tasks WHERE id = ?")
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn run_task_now(
    db: &SqlitePool,
    log_db: &SqlitePool,
    id: i64,
    app_handle: Option<&AppHandle>,
) -> Result<ScheduledTaskRun, String> {
    execute_task(db, log_db, id, TRIGGER_MANUAL, app_handle).await
}

pub async fn list_runs(
    log_db: &SqlitePool,
    task_id: Option<i64>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<ScheduledTaskRunListResponse, String> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let (items, total) = if let Some(task_id) = task_id {
        let (total,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scheduled_task_runs WHERE task_id = ?")
                .bind(task_id)
                .fetch_one(log_db)
                .await
                .map_err(|e| e.to_string())?;

        let items = sqlx::query_as::<_, ScheduledTaskRun>(
            r#"
            SELECT * FROM scheduled_task_runs
            WHERE task_id = ?
            ORDER BY started_at DESC, id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(task_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(log_db)
        .await
        .map_err(|e| e.to_string())?;

        (items, total)
    } else {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scheduled_task_runs")
            .fetch_one(log_db)
            .await
            .map_err(|e| e.to_string())?;

        let items = sqlx::query_as::<_, ScheduledTaskRun>(
            r#"
            SELECT * FROM scheduled_task_runs
            ORDER BY started_at DESC, id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(log_db)
        .await
        .map_err(|e| e.to_string())?;

        (items, total)
    };

    Ok(ScheduledTaskRunListResponse { items, total })
}

pub async fn list_run_items(
    log_db: &SqlitePool,
    run_id: i64,
) -> Result<Vec<ScheduledTaskRunItem>, String> {
    sqlx::query_as::<_, ScheduledTaskRunItem>(
        "SELECT * FROM scheduled_task_run_items WHERE run_id = ? ORDER BY id",
    )
    .bind(run_id)
    .fetch_all(log_db)
    .await
    .map_err(|e| e.to_string())
}

async fn run_due_tasks(
    db: &SqlitePool,
    log_db: &SqlitePool,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let now = now_timestamp();
    let task_ids = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT id FROM scheduled_tasks
        WHERE enabled = 1 AND schedule_type IN (?, ?) AND next_run_at <= ? AND last_status != ?
        ORDER BY next_run_at, id
        LIMIT ?
        "#,
    )
    .bind(SCHEDULE_TYPE_INTERVAL)
    .bind(SCHEDULE_TYPE_DAILY)
    .bind(now)
    .bind(STATUS_RUNNING)
    .bind(SCHEDULER_DUE_TASK_LIMIT)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    for (task_id,) in task_ids {
        if is_task_locked(task_id).await {
            continue;
        }

        let db = db.clone();
        let log_db = log_db.clone();
        let app_handle = app_handle.clone();
        tokio::spawn(async move {
            if let Err(e) =
                execute_task(&db, &log_db, task_id, TRIGGER_SCHEDULED, Some(&app_handle)).await
            {
                if e != "任务正在执行" {
                    tracing::error!(task_id = task_id, error = %e, "Scheduled task execution failed");
                }
            }
        });
    }

    Ok(())
}

async fn execute_task(
    db: &SqlitePool,
    log_db: &SqlitePool,
    task_id: i64,
    trigger_type: &str,
    app_handle: Option<&AppHandle>,
) -> Result<ScheduledTaskRun, String> {
    if !try_lock_task(task_id).await {
        return Err("任务正在执行".to_string());
    }

    let result = execute_task_locked(db, log_db, task_id, trigger_type, app_handle).await;
    if let Err(e) = &result {
        cleanup_failed_execution(db, log_db, task_id, e, app_handle).await;
    }
    unlock_task(task_id).await;
    result
}

async fn execute_task_locked(
    db: &SqlitePool,
    log_db: &SqlitePool,
    task_id: i64,
    trigger_type: &str,
    app_handle: Option<&AppHandle>,
) -> Result<ScheduledTaskRun, String> {
    let task = load_task(db, task_id).await?;
    if task.enabled == 0 && trigger_type == TRIGGER_SCHEDULED {
        return Err("任务已停用".to_string());
    }

    let started_at = now_timestamp();
    let timer = std::time::Instant::now();

    sqlx::query(
        "UPDATE scheduled_tasks SET last_status = ?, last_run_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(STATUS_RUNNING)
    .bind(started_at)
    .bind(started_at)
    .bind(task.id)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    let running_updated_at = started_at;

    let retry_provider_ids = if trigger_type == TRIGGER_SCHEDULED
        && task.retry_count > 0
        && task.last_status == STATUS_RETRYING
    {
        Some(latest_failed_provider_ids(log_db, task.id).await?)
    } else {
        None
    };

    let run_id = insert_run(log_db, &task, trigger_type, started_at).await?;
    emit_task_changed_if_present(app_handle, Some(task.id), Some(run_id));

    let outcome = match task.task_type.as_str() {
        TASK_TYPE_PROVIDER_KEEPALIVE => {
            execute_keepalive_task(db, log_db, run_id, &task, retry_provider_ids.as_deref()).await
        }
        _ => Err(format!("不支持的任务类型: {}", task.task_type)),
    };

    let finished_at = now_timestamp();
    let elapsed_ms = timer.elapsed().as_millis() as i64;

    let (status, outcome) = match outcome {
        Ok(outcome) => (outcome.status().to_string(), outcome),
        Err(e) => {
            let mut outcome = RunOutcome::empty();
            outcome.errors.push(e);
            (STATUS_FAILED.to_string(), outcome)
        }
    };
    let error_message = outcome.error_message();

    finish_run(
        log_db,
        run_id,
        &status,
        finished_at,
        elapsed_ms,
        &outcome,
        error_message.as_deref(),
    )
    .await?;

    update_task_after_run(
        db,
        log_db,
        &task,
        running_updated_at,
        &status,
        &outcome,
        error_message.as_deref(),
        trigger_type,
    )
    .await?;

    prune_old_runs(log_db, task.id).await?;

    let run = load_run(log_db, run_id).await?;
    emit_task_changed_if_present(app_handle, Some(task.id), Some(run_id));
    Ok(run)
}

async fn execute_keepalive_task(
    db: &SqlitePool,
    log_db: &SqlitePool,
    run_id: i64,
    task: &ScheduledTask,
    retry_provider_ids: Option<&[i64]>,
) -> Result<RunOutcome, String> {
    let payload: ProviderKeepalivePayload = serde_json::from_str(&task.payload_json)
        .map_err(|e| format!("服务商调用参数解析失败: {}", e))?;
    let model_name = payload.model_name.trim().to_string();
    let test_text = payload.test_text.clone();
    let targets = resolve_keepalive_targets(db, &payload, retry_provider_ids).await?;
    let mut outcome = RunOutcome::empty();

    if targets.is_empty() {
        let reason = "没有可执行的服务商";
        outcome.total_count = 1;
        outcome.skipped_count = 1;
        outcome.errors.push(reason.to_string());
        insert_run_item(
            log_db,
            run_id,
            None,
            "任务",
            &model_name,
            STATUS_SKIPPED,
            None,
            0,
            Some(reason),
        )
        .await?;
        return Ok(outcome);
    }

    let results = join_all(targets.into_iter().map(|target| {
        execute_keepalive_target(
            db,
            log_db,
            run_id,
            target,
            &model_name,
            test_text.as_deref(),
        )
    }))
    .await;

    for result in results {
        outcome.merge(result?);
    }

    Ok(outcome)
}

async fn execute_keepalive_target(
    db: &SqlitePool,
    log_db: &SqlitePool,
    run_id: i64,
    target: KeepaliveTarget,
    model_name: &str,
    test_text: Option<&str>,
) -> Result<RunOutcome, String> {
    let mut outcome = RunOutcome::empty();
    outcome.total_count = 1;

    if let Some(reason) = target.skip_reason {
        outcome.skipped_count = 1;
        outcome
            .errors
            .push(format!("{}: {}", target.provider_name, reason));
        insert_run_item(
            log_db,
            run_id,
            target.provider_id,
            &target.provider_name,
            model_name,
            "skipped",
            None,
            0,
            Some(&reason),
        )
        .await?;
        return Ok(outcome);
    }

    let Some(provider) = target.provider else {
        return Ok(outcome);
    };

    let timeout_secs = provider_service::get_stream_first_byte_timeout(db).await;
    let result =
        provider_service::test_provider_model(db, provider.id, model_name, test_text, timeout_secs)
            .await;
    let ok = result
        .status_code
        .map(|code| (200..300).contains(&code))
        .unwrap_or(false)
        && result.response_text == "请求成功";

    if ok {
        outcome.success_count = 1;
    } else {
        outcome.failure_count = 1;
        outcome.errors.push(format!(
            "{}: {}",
            result.provider_name,
            truncate_text(&result.response_text, 300)
        ));
    }

    let error_message = if ok {
        None
    } else {
        Some(truncate_text(&result.response_text, 2000))
    };

    insert_run_item(
        log_db,
        run_id,
        Some(result.provider_id),
        &result.provider_name,
        &result.actual_model,
        if ok { STATUS_SUCCESS } else { STATUS_FAILED },
        result.status_code.map(|code| code as i64),
        result.elapsed_ms as i64,
        error_message.as_deref(),
    )
    .await?;

    Ok(outcome)
}

async fn resolve_keepalive_targets(
    db: &SqlitePool,
    payload: &ProviderKeepalivePayload,
    retry_provider_ids: Option<&[i64]>,
) -> Result<Vec<KeepaliveTarget>, String> {
    if let Some(ids) = retry_provider_ids {
        return resolve_provider_ids(db, ids).await;
    }

    match payload.target_mode.as_str() {
        "all" => {
            let cli_type = payload
                .cli_type
                .as_deref()
                .ok_or_else(|| "全选模式缺少 cli_type".to_string())?;
            let profile = normalize_profile(payload.profile.as_deref())
                .unwrap_or_else(|| DEFAULT_PROFILE.to_string());
            if !provider_profile_exists_if_supported(db, cli_type, &profile).await? {
                return Ok(vec![KeepaliveTarget {
                    provider_id: None,
                    provider_name: format!("{}:{}", cli_type, profile),
                    provider: None,
                    skip_reason: Some("Profile 不存在".to_string()),
                }]);
            }

            let providers = sqlx::query_as::<_, Provider>(
                r#"
                SELECT * FROM providers
                WHERE enabled = 1 AND cli_type = ? AND profile = ?
                ORDER BY sort_order, id
                "#,
            )
            .bind(cli_type)
            .bind(&profile)
            .fetch_all(db)
            .await
            .map_err(|e| e.to_string())?;

            if providers.is_empty() {
                return Ok(vec![KeepaliveTarget {
                    provider_id: None,
                    provider_name: format!("{}:{}", cli_type, profile),
                    provider: None,
                    skip_reason: Some("没有可用服务商".to_string()),
                }]);
            }

            Ok(providers
                .into_iter()
                .map(|provider| KeepaliveTarget {
                    provider_id: Some(provider.id),
                    provider_name: provider.name.clone(),
                    provider: Some(provider),
                    skip_reason: None,
                })
                .collect())
        }
        "selected" => {
            let ids = payload
                .provider_ids
                .as_deref()
                .ok_or_else(|| "指定服务商模式缺少 provider_ids".to_string())?;
            resolve_provider_ids(db, ids).await
        }
        _ => Err("执行对象只能是 all 或 selected".to_string()),
    }
}

async fn resolve_provider_ids(
    db: &SqlitePool,
    provider_ids: &[i64],
) -> Result<Vec<KeepaliveTarget>, String> {
    if provider_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Build WHERE IN query with placeholders
    let placeholders: Vec<&str> = provider_ids.iter().map(|_| "?").collect();
    let query = format!(
        "SELECT * FROM providers WHERE id IN ({})",
        placeholders.join(", ")
    );

    let mut q = sqlx::query_as::<_, Provider>(&query);
    for id in provider_ids {
        q = q.bind(id);
    }

    let providers = q.fetch_all(db).await.map_err(|e| e.to_string())?;
    let provider_map: HashMap<i64, Provider> = providers.into_iter().map(|p| (p.id, p)).collect();

    Ok(provider_ids
        .iter()
        .map(|id| match provider_map.get(id) {
            Some(provider) if provider.enabled == 1 => KeepaliveTarget {
                provider_id: Some(*id),
                provider_name: provider.name.clone(),
                provider: Some(provider.clone()),
                skip_reason: None,
            },
            Some(provider) => KeepaliveTarget {
                provider_id: Some(*id),
                provider_name: provider.name.clone(),
                provider: None,
                skip_reason: Some("服务商已停用".to_string()),
            },
            None => KeepaliveTarget {
                provider_id: Some(*id),
                provider_name: format!("Provider#{}", id),
                provider: None,
                skip_reason: Some("服务商不存在".to_string()),
            },
        })
        .collect())
}

async fn update_task_after_run(
    db: &SqlitePool,
    log_db: &SqlitePool,
    task: &ScheduledTask,
    running_updated_at: i64,
    status: &str,
    outcome: &RunOutcome,
    error_message: Option<&str>,
    trigger_type: &str,
) -> Result<(), String> {
    let now = now_timestamp();
    let is_success = status == STATUS_SUCCESS;
    let is_manual = trigger_type == TRIGGER_MANUAL;
    let should_retry = !is_manual
        && !is_success
        && outcome.failure_count > 0
        && task.retry_count < task.retry_limit;

    let (last_status, retry_count, next_run_at) = if is_manual {
        // 手动执行不打断自动重试链
        let keep_retrying = !is_success && task.last_status == STATUS_RETRYING;
        (
            if keep_retrying {
                STATUS_RETRYING.to_string()
            } else if is_success {
                STATUS_SUCCESS.to_string()
            } else {
                status.to_string()
            },
            if is_success { 0 } else { task.retry_count },
            if task.next_run_at <= now {
                next_run_after(&task.schedule_type, &task.schedule_expr, now)?
            } else {
                task.next_run_at
            },
        )
    } else if is_success {
        (
            STATUS_SUCCESS.to_string(),
            0,
            next_run_after(&task.schedule_type, &task.schedule_expr, now)?,
        )
    } else if should_retry {
        (
            STATUS_RETRYING.to_string(),
            task.retry_count + 1,
            now + task.retry_interval_minutes.max(1) * 60,
        )
    } else {
        (
            status.to_string(),
            0,
            next_run_after(&task.schedule_type, &task.schedule_expr, now)?,
        )
    };

    let update_result = sqlx::query(
        r#"
        UPDATE scheduled_tasks
        SET retry_count = ?, next_run_at = ?, last_status = ?, last_error = ?, updated_at = ?
        WHERE id = ? AND last_status = ? AND updated_at = ? AND enabled = ?
          AND schedule_type = ? AND schedule_expr = ? AND payload_json = ?
          AND retry_limit = ? AND retry_interval_minutes = ?
        "#,
    )
    .bind(retry_count)
    .bind(next_run_at)
    .bind(&last_status)
    .bind(error_message)
    .bind(now)
    .bind(task.id)
    .bind(STATUS_RUNNING)
    .bind(running_updated_at)
    .bind(task.enabled)
    .bind(&task.schedule_type)
    .bind(&task.schedule_expr)
    .bind(&task.payload_json)
    .bind(task.retry_limit)
    .bind(task.retry_interval_minutes)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    if update_result.rows_affected() == 0 {
        sqlx::query(
            r#"
            UPDATE scheduled_tasks
            SET last_status = ?, last_error = ?, updated_at = ?
            WHERE id = ? AND last_status = ?
            "#,
        )
        .bind(&last_status)
        .bind(error_message)
        .bind(now)
        .bind(task.id)
        .bind(STATUS_RUNNING)
        .execute(db)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())?;
    }

    if !is_manual && !is_success && !should_retry && status != STATUS_SKIPPED {
        let message = format!(
            "定时任务 {} 执行失败: {}",
            task.name,
            error_message.unwrap_or("未知错误")
        );
        let _ =
            crate::services::stats::record_system_log(log_db, "scheduled_task_failed", &message)
                .await;
    }

    Ok(())
}

async fn insert_run(
    db: &SqlitePool,
    task: &ScheduledTask,
    trigger_type: &str,
    started_at: i64,
) -> Result<i64, String> {
    let result = sqlx::query(
        r#"
        INSERT INTO scheduled_task_runs
            (task_id, task_name, task_type, trigger_type, status, started_at, finished_at,
             elapsed_ms, total_count, success_count, failure_count, skipped_count, error_message)
        VALUES (?, ?, ?, ?, ?, ?, NULL, 0, 0, 0, 0, 0, NULL)
        "#,
    )
    .bind(task.id)
    .bind(&task.name)
    .bind(&task.task_type)
    .bind(trigger_type)
    .bind(STATUS_RUNNING)
    .bind(started_at)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.last_insert_rowid())
}

async fn finish_run(
    db: &SqlitePool,
    run_id: i64,
    status: &str,
    finished_at: i64,
    elapsed_ms: i64,
    outcome: &RunOutcome,
    error_message: Option<&str>,
) -> Result<(), String> {
    sqlx::query(
        r#"
        UPDATE scheduled_task_runs
        SET status = ?, finished_at = ?, elapsed_ms = ?, total_count = ?, success_count = ?,
            failure_count = ?, skipped_count = ?, error_message = ?
        WHERE id = ?
        "#,
    )
    .bind(status)
    .bind(finished_at)
    .bind(elapsed_ms)
    .bind(outcome.total_count)
    .bind(outcome.success_count)
    .bind(outcome.failure_count)
    .bind(outcome.skipped_count)
    .bind(error_message)
    .bind(run_id)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn insert_run_item(
    db: &SqlitePool,
    run_id: i64,
    provider_id: Option<i64>,
    provider_name: &str,
    model_name: &str,
    status: &str,
    status_code: Option<i64>,
    elapsed_ms: i64,
    error_message: Option<&str>,
) -> Result<(), String> {
    let now = now_timestamp();
    sqlx::query(
        r#"
        INSERT INTO scheduled_task_run_items
            (run_id, provider_id, provider_name, model_name, status, status_code, elapsed_ms,
             error_message, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(run_id)
    .bind(provider_id)
    .bind(provider_name)
    .bind(model_name)
    .bind(status)
    .bind(status_code)
    .bind(elapsed_ms)
    .bind(error_message)
    .bind(now)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn latest_failed_provider_ids(db: &SqlitePool, task_id: i64) -> Result<Vec<i64>, String> {
    let run_id = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT id FROM scheduled_task_runs
        WHERE task_id = ? AND trigger_type = ? AND failure_count > 0
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .bind(task_id)
    .bind(TRIGGER_SCHEDULED)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .map(|(id,)| id);

    let Some(run_id) = run_id else {
        return Ok(Vec::new());
    };

    let rows = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT provider_id FROM scheduled_task_run_items
        WHERE run_id = ? AND status = ? AND provider_id IS NOT NULL
        ORDER BY id
        "#,
    )
    .bind(run_id)
    .bind(STATUS_FAILED)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

async fn prune_old_runs(log_db: &SqlitePool, task_id: i64) -> Result<(), String> {
    sqlx::query(
        r#"
        DELETE FROM scheduled_task_run_items
        WHERE run_id IN (
            SELECT id FROM scheduled_task_runs
            WHERE task_id = ?
            ORDER BY started_at DESC, id DESC
            LIMIT -1 OFFSET ?
        )
        "#,
    )
    .bind(task_id)
    .bind(RUN_RETENTION_PER_TASK)
    .execute(log_db)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        r#"
        DELETE FROM scheduled_task_runs
        WHERE id IN (
            SELECT id FROM scheduled_task_runs
            WHERE task_id = ?
            ORDER BY started_at DESC, id DESC
            LIMIT -1 OFFSET ?
        )
        "#,
    )
    .bind(task_id)
    .bind(RUN_RETENTION_PER_TASK)
    .execute(log_db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn cleanup_failed_execution(
    db: &SqlitePool,
    log_db: &SqlitePool,
    task_id: i64,
    error: &str,
    app_handle: Option<&AppHandle>,
) {
    let now = now_timestamp();
    let next_run_at = match load_task(db, task_id).await {
        Ok(task) if task.last_status == STATUS_RUNNING => {
            next_run_after(&task.schedule_type, &task.schedule_expr, now).unwrap_or(now + 60)
        }
        _ => now + 60,
    };

    if let Err(e) = sqlx::query(
        r#"
        UPDATE scheduled_tasks
        SET last_status = ?, last_error = ?, next_run_at = ?, updated_at = ?
        WHERE id = ? AND last_status = ?
        "#,
    )
    .bind(STATUS_FAILED)
    .bind(truncate_text(error, 2000))
    .bind(next_run_at)
    .bind(now)
    .bind(task_id)
    .bind(STATUS_RUNNING)
    .execute(db)
    .await
    {
        tracing::error!(task_id = task_id, error = %e, "Failed to cleanup scheduled task state");
    }

    if let Err(e) = sqlx::query(
        r#"
        UPDATE scheduled_task_runs
        SET status = ?, finished_at = ?, error_message = ?
        WHERE task_id = ? AND status = ? AND finished_at IS NULL
        "#,
    )
    .bind(STATUS_FAILED)
    .bind(now)
    .bind(truncate_text(error, 2000))
    .bind(task_id)
    .bind(STATUS_RUNNING)
    .execute(log_db)
    .await
    {
        tracing::error!(task_id = task_id, error = %e, "Failed to cleanup scheduled task run state");
    }

    emit_task_changed_if_present(app_handle, Some(task_id), None);
}

async fn load_task(db: &SqlitePool, id: i64) -> Result<ScheduledTask, String> {
    sqlx::query_as::<_, ScheduledTask>("SELECT * FROM scheduled_tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "任务不存在".to_string())
}

async fn load_run(db: &SqlitePool, id: i64) -> Result<ScheduledTaskRun, String> {
    sqlx::query_as::<_, ScheduledTaskRun>("SELECT * FROM scheduled_task_runs WHERE id = ?")
        .bind(id)
        .fetch_one(db)
        .await
        .map_err(|e| e.to_string())
}

async fn is_task_locked(task_id: i64) -> bool {
    let locks = TASK_LOCKS
        .get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
        .clone();
    let guard = locks.lock().await;
    guard.contains(&task_id)
}

async fn try_lock_task(task_id: i64) -> bool {
    let locks = TASK_LOCKS
        .get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
        .clone();
    let mut guard = locks.lock().await;
    guard.insert(task_id)
}

async fn unlock_task(task_id: i64) {
    let locks = TASK_LOCKS
        .get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
        .clone();
    let mut guard = locks.lock().await;
    guard.remove(&task_id);
}

fn validate_task_type(task_type: &str) -> Result<(), String> {
    if task_type == TASK_TYPE_PROVIDER_KEEPALIVE {
        Ok(())
    } else {
        Err("只支持服务商调用任务".to_string())
    }
}

fn validate_schedule_type(schedule_type: &str) -> Result<(), String> {
    if schedule_type == SCHEDULE_TYPE_INTERVAL || schedule_type == SCHEDULE_TYPE_DAILY {
        Ok(())
    } else {
        Err("只支持间隔执行或定期执行".to_string())
    }
}

fn validate_payload(task_type: &str, payload_json: &str) -> Result<(), String> {
    if task_type != TASK_TYPE_PROVIDER_KEEPALIVE {
        return validate_task_type(task_type);
    }

    let payload: ProviderKeepalivePayload =
        serde_json::from_str(payload_json).map_err(|e| format!("任务参数不是合法 JSON: {}", e))?;

    if payload.model_name.trim().is_empty() {
        return Err("模型名不能为空".to_string());
    }

    match payload.target_mode.as_str() {
        "all" => {
            let cli_type = payload.cli_type.as_deref().unwrap_or_default();
            validate_cli_type(cli_type)?;
            if normalize_profile(payload.profile.as_deref()).is_none() {
                return Err("profile 参数无效".to_string());
            }
        }
        "selected" => {
            if payload.provider_ids.as_ref().map_or(true, Vec::is_empty) {
                return Err("指定服务商模式必须选择服务商".to_string());
            }
        }
        _ => return Err("执行对象只能是 all 或 selected".to_string()),
    }

    Ok(())
}

fn validate_retry(retry_limit: i64, retry_interval_minutes: i64) -> Result<(), String> {
    if retry_limit < 0 {
        return Err("重试次数不能小于 0".to_string());
    }
    if retry_interval_minutes <= 0 {
        return Err("重试间隔必须大于 0".to_string());
    }
    Ok(())
}

fn next_run_after(schedule_type: &str, schedule_expr: &str, after_ts: i64) -> Result<i64, String> {
    validate_schedule_type(schedule_type)?;
    match schedule_type {
        SCHEDULE_TYPE_INTERVAL => next_interval_run_after(schedule_expr, after_ts),
        SCHEDULE_TYPE_DAILY => next_daily_run_after(schedule_expr, after_ts),
        _ => unreachable!(),
    }
}

fn next_interval_run_after(schedule_expr: &str, after_ts: i64) -> Result<i64, String> {
    let minutes = schedule_expr
        .trim()
        .parse::<i64>()
        .map_err(|_| "执行间隔必须是整数分钟".to_string())?;
    if minutes <= 0 {
        return Err("执行间隔必须大于 0 分钟".to_string());
    }
    minutes
        .checked_mul(60)
        .and_then(|seconds| after_ts.checked_add(seconds))
        .ok_or_else(|| "执行间隔过大".to_string())
}

fn next_daily_run_after(schedule_expr: &str, after_ts: i64) -> Result<i64, String> {
    let (days, time) = parse_daily_schedule(schedule_expr)?;
    let after = Local
        .timestamp_opt(after_ts, 0)
        .single()
        .unwrap_or_else(Local::now);
    let mut date = after.date_naive();

    for _ in 0..5 {
        let naive = date.and_time(time);
        let candidate = Local
            .from_local_datetime(&naive)
            .single()
            .or_else(|| Local.from_local_datetime(&naive).earliest());

        if let Some(candidate) = candidate {
            if candidate.timestamp() > after_ts {
                return Ok(candidate.timestamp());
            }
        }

        date += Duration::days(days);
    }

    Err("无法计算下次定期执行时间".to_string())
}

fn parse_daily_schedule(schedule_expr: &str) -> Result<(i64, NaiveTime), String> {
    let value = schedule_expr.trim();
    if value.is_empty() {
        return Err("定期执行时间不能为空".to_string());
    }

    if let Ok(expr) = serde_json::from_str::<DailyScheduleExpr>(value) {
        validate_daily_parts(expr.days, expr.hour, expr.minute)?;
        let time = NaiveTime::from_hms_opt(expr.hour, expr.minute, 0)
            .ok_or_else(|| "定期执行时间无效".to_string())?;
        return Ok((expr.days, time));
    }

    let time = NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| "定期执行格式必须是 HH:mm 或 JSON".to_string())?;
    Ok((1, time))
}

fn validate_daily_parts(days: i64, hour: u32, minute: u32) -> Result<(), String> {
    if !(1..=365).contains(&days) {
        return Err("执行周期必须是 1 到 365 天".to_string());
    }
    if hour > 23 {
        return Err("执行小时必须是 0 到 23".to_string());
    }
    if minute > 59 {
        return Err("执行分钟必须是 0 到 59".to_string());
    }
    Ok(())
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for ch in text.chars().take(max_chars) {
        result.push(ch);
    }
    if text.chars().count() > max_chars {
        result.push_str("...");
    }
    result
}
