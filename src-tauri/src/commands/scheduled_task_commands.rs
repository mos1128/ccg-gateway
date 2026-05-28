use super::*;

#[tauri::command]
pub async fn get_scheduled_tasks(db: State<'_, SqlitePool>) -> Result<Vec<ScheduledTaskResponse>> {
    crate::services::scheduler::list_tasks(db.inner()).await
}

#[tauri::command]
pub async fn get_scheduled_task(
    db: State<'_, SqlitePool>,
    id: i64,
) -> Result<ScheduledTaskResponse> {
    crate::services::scheduler::get_task(db.inner(), id).await
}

#[tauri::command]
pub async fn create_scheduled_task(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    input: ScheduledTaskCreate,
) -> Result<ScheduledTaskResponse> {
    let task = crate::services::scheduler::create_task(db.inner(), input).await?;
    crate::services::scheduler::emit_task_changed(&app, Some(task.id), None);
    Ok(task)
}

#[tauri::command]
pub async fn update_scheduled_task(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    id: i64,
    input: ScheduledTaskUpdate,
) -> Result<ScheduledTaskResponse> {
    let task = crate::services::scheduler::update_task(db.inner(), id, input).await?;
    crate::services::scheduler::emit_task_changed(&app, Some(task.id), None);
    Ok(task)
}

#[tauri::command]
pub async fn delete_scheduled_task(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<()> {
    crate::services::scheduler::delete_task(db.inner(), &log_db.0, id).await?;
    crate::services::scheduler::emit_task_changed(&app, Some(id), None);
    Ok(())
}

#[tauri::command]
pub async fn run_scheduled_task_now(
    app: tauri::AppHandle,
    db: State<'_, SqlitePool>,
    log_db: State<'_, LogDb>,
    id: i64,
) -> Result<ScheduledTaskRun> {
    crate::services::scheduler::run_task_now(db.inner(), &log_db.0, id, Some(&app)).await
}

#[tauri::command]
pub async fn get_scheduled_task_runs(
    log_db: State<'_, LogDb>,
    task_id: Option<i64>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<ScheduledTaskRunListResponse> {
    crate::services::scheduler::list_runs(&log_db.0, task_id, page, page_size).await
}

#[tauri::command]
pub async fn get_scheduled_task_run_items(
    log_db: State<'_, LogDb>,
    run_id: i64,
) -> Result<Vec<ScheduledTaskRunItem>> {
    crate::services::scheduler::list_run_items(&log_db.0, run_id).await
}
