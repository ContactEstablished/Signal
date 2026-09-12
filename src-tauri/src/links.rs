use crate::workspace::models::*;
use tauri_plugin_opener::OpenerExt;
pub fn validate(value: &str) -> Result<url::Url> {
    let url = url::Url::parse(value).map_err(|_| AppError::validation("Enter a valid web link"))?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::validation(
            "Only HTTP or HTTPS links without credentials can be opened",
        ));
    }
    Ok(url)
}
#[tauri::command]
pub async fn open_external_url(app: tauri::AppHandle, url: String) -> Result<()> {
    let url = validate(&url)?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| AppError::new("File", e.to_string()))
}
#[tauri::command]
pub async fn open_task_link(app: tauri::AppHandle, task_id: String) -> Result<()> {
    let pool = crate::db::pool(&app)
        .await
        .map_err(|e| AppError::new("Database", e))?;
    let task = one(
        &mut *pool.acquire().await?,
        "SELECT external_url FROM tasks WHERE id=?",
        vec![serde_json::json!(task_id)],
    )
    .await?;
    open_external_url(app, text(&task, "external_url")?.into()).await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_web_and_credentials() {
        for v in [
            "javascript:alert(1)",
            "file:///C:/secret",
            "https://user:pass@example.com",
            "data:text/html,test",
        ] {
            assert!(validate(v).is_err())
        }
        assert!(validate("https://example.com/task/1").is_ok());
    }
}
