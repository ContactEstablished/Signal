pub mod delete;
pub mod models;
pub mod projects;
pub mod tasks;
use models::*;
use serde_json::Value;
use tauri::{AppHandle, Manager};
async fn pool(app: &AppHandle) -> Result<sqlx::SqlitePool> {
    crate::db::pool(app)
        .await
        .map_err(|e| AppError::new("Database", e))
}
fn clock(app: &AppHandle) -> String {
    now(app.state::<crate::db::RuntimeState>().seeded)
}
#[tauri::command]
pub async fn list_projects(app: AppHandle) -> Result<Value> {
    Ok(serde_json::json!(projects::list(&pool(&app).await?).await?))
}
#[tauri::command]
pub async fn get_board(
    app: AppHandle,
    project_id: String,
    day_start_utc: String,
    day_end_utc: String,
) -> Result<Value> {
    tasks::board(
        &pool(&app).await?,
        &project_id,
        &day_start_utc,
        &day_end_utc,
    )
    .await
}
#[tauri::command]
pub async fn get_task_detail(app: AppHandle, task_id: String) -> Result<Value> {
    tasks::get(&pool(&app).await?, &task_id).await
}
#[tauri::command]
pub async fn create_project(app: AppHandle, input: ProjectInput) -> Result<Value> {
    projects::save(&pool(&app).await?, None, input).await
}
#[tauri::command]
pub async fn update_project(app: AppHandle, id: String, input: ProjectInput) -> Result<Value> {
    projects::save(&pool(&app).await?, Some(id), input).await
}
#[tauri::command]
pub async fn move_project(app: AppHandle, id: String, direction: String) -> Result<Value> {
    Ok(serde_json::json!(
        projects::reorder(&pool(&app).await?, &id, &direction).await?
    ))
}
#[tauri::command]
pub async fn create_task(app: AppHandle, input: Value) -> Result<Value> {
    let files = app.state::<crate::attachments::Files>();
    let _gate = files.gate.lock().await;
    tasks::create(&pool(&app).await?, input, &clock(&app)).await
}
#[tauri::command]
pub async fn update_task(
    app: AppHandle,
    id: String,
    patch: Value,
    expected_revision: i64,
) -> Result<Value> {
    tasks::update(
        &pool(&app).await?,
        &id,
        patch,
        expected_revision,
        &clock(&app),
    )
    .await
}
#[tauri::command]
pub async fn move_task(
    app: AppHandle,
    id: String,
    status: String,
    before_task_id: Option<String>,
    expected_revision: i64,
) -> Result<Value> {
    tasks::move_task(
        &pool(&app).await?,
        &id,
        &status,
        before_task_id.as_deref(),
        expected_revision,
        &clock(&app),
    )
    .await
}
#[tauri::command]
pub async fn set_subtasks(
    app: AppHandle,
    id: String,
    inputs: Value,
    expected_revision: i64,
) -> Result<Value> {
    tasks::children(
        &pool(&app).await?,
        &id,
        "subtasks",
        inputs,
        expected_revision,
        &clock(&app),
    )
    .await
}
#[tauri::command]
pub async fn set_task_tags(
    app: AppHandle,
    id: String,
    inputs: Value,
    expected_revision: i64,
) -> Result<Value> {
    tasks::children(
        &pool(&app).await?,
        &id,
        "tags",
        inputs,
        expected_revision,
        &clock(&app),
    )
    .await
}
#[tauri::command]
pub async fn set_task_alerts(
    app: AppHandle,
    id: String,
    offsets: Value,
    expected_revision: i64,
) -> Result<Value> {
    tasks::children(
        &pool(&app).await?,
        &id,
        "alerts",
        offsets,
        expected_revision,
        &clock(&app),
    )
    .await
}
#[tauri::command]
pub async fn preview_deletion(app: AppHandle, target: DeletionTarget) -> Result<Value> {
    delete::preview(&pool(&app).await?, &target).await
}
#[tauri::command]
pub async fn delete_entity(
    app: AppHandle,
    target: DeletionTarget,
    fingerprint: String,
) -> Result<Value> {
    let pool = pool(&app).await?;
    let files = app.state::<crate::attachments::Files>();
    let _gate = files.gate.lock().await;
    delete::remove(&pool, &target, &fingerprint, &clock(&app)).await?;
    Ok(
        serde_json::json!({"deleted":true,"cleanupPending":files.cleanup(&pool,&clock(&app)).await.unwrap_or(true)}),
    )
}
#[cfg(test)]
mod tests;
