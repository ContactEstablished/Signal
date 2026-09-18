mod data;
mod engine;
mod settings;
use crate::workspace::models::*;
pub use engine::Voice;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn voice_settings(app: AppHandle) -> Result<Value> {
    let root = settings::root(&app)?;
    let config = settings::load(&app).await?;
    let mut models = Vec::new();
    for name in ["base.en", "small.en"] {
        let (file, bytes) = engine::model(name)?;
        models.push(json!({"id":name,"bytes":bytes,"installed":std::fs::metadata(root.join("models").join(file)).map(|m| m.len()==bytes).unwrap_or(false)}));
    }
    Ok(json!({"settings":config,"has_key":root.join("credential.bin").is_file(),"models":models}))
}
#[tauri::command]
pub async fn save_voice_settings(
    app: AppHandle,
    settings: settings::Settings,
    key: Option<String>,
) -> Result<()> {
    settings::save(&app, settings, key).await
}
#[tauri::command]
pub async fn download_voice_model(app: AppHandle, model: String) -> Result<()> {
    engine::download(&app, &model).await
}
#[tauri::command]
pub fn start_voice_capture(app: AppHandle) -> Result<String> {
    let state = app.state::<Voice>();
    let mut runtime = state.0.lock().unwrap();
    if runtime.capture.is_some() || runtime.job.is_some() {
        return Err(AppError::validation(
            "A voice operation is already running.",
        ));
    }
    let id = uid();
    runtime.capture = Some(engine::Capture {
        id: id.clone(),
        next: 0,
        samples: Vec::new(),
    });
    Ok(id)
}
#[tauri::command]
pub fn voice_frame(
    app: AppHandle,
    capture_id: String,
    sequence: u32,
    samples: Vec<i16>,
) -> Result<()> {
    let state = app.state::<Voice>();
    let mut runtime = state.0.lock().unwrap();
    let capture = runtime
        .capture
        .as_mut()
        .filter(|c| c.id == capture_id)
        .ok_or_else(|| AppError::validation("Recording is no longer active."))?;
    capture.push(sequence, samples)
}
#[tauri::command]
pub async fn stop_voice_capture(app: AppHandle, capture_id: String) -> Result<String> {
    let (samples, task) = {
        let state = app.state::<Voice>();
        let mut runtime = state.0.lock().unwrap();
        if runtime.capture.as_ref().is_none_or(|c| c.id != capture_id) {
            return Err(AppError::validation("Recording is no longer active."));
        }
        let samples = runtime.capture.take().unwrap().samples;
        (samples, engine::reserve(&app, &mut runtime)?)
    };
    engine::transcribe(&app, samples, task).await
}
#[tauri::command]
pub async fn cancel_voice(app: AppHandle) -> Result<()> {
    let state = app.state::<Voice>();
    let original = {
        let mut runtime = state.0.lock().unwrap();
        runtime.capture = None;
        runtime.job.as_ref().map(|(id, flag)| {
            flag.store(true, Ordering::SeqCst);
            id.clone()
        })
    };
    // A normal close waits for the worker to exit and clean scratch audio.
    for _ in 0..200 {
        if state.0.lock().unwrap().job.as_ref().map(|j| &j.0) != original.as_ref()
            || original.is_none()
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    Err(AppError::new(
        "Busy",
        "Voice cleanup is still finishing. Retry Cancel before closing.",
    ))
}
#[tauri::command]
pub async fn load_voice_draft(app: AppHandle) -> Result<Option<data::Draft>> {
    data::load(&crate::workspace::pool(&app).await?).await
}
#[tauri::command]
pub async fn save_voice_draft(app: AppHandle, draft: data::Draft) -> Result<data::Draft> {
    data::save(&crate::workspace::pool(&app).await?, draft).await
}
#[tauri::command]
pub async fn discard_voice_draft(app: AppHandle, id: String, revision: i64) -> Result<()> {
    data::discard(&crate::workspace::pool(&app).await?, &id, revision).await
}
#[tauri::command]
pub async fn suggest_voice_tasks(app: AppHandle, id: String, revision: i64) -> Result<data::Draft> {
    engine::bounded_suggestions(
        async {
            let draft = data::load(&crate::workspace::pool(&app).await?)
                .await?
                .filter(|d| d.id == id && d.revision == revision)
                .ok_or_else(AppError::conflict)?;

            engine::suggest(&app, draft).await
        },
        std::time::Duration::from_secs(90),
    )
    .await
}
#[tauri::command]
pub async fn accept_voice_tasks(app: AppHandle, input: data::Accept) -> Result<Value> {
    data::accept(
        &crate::workspace::pool(&app).await?,
        input,
        &crate::clock::sample(&app)?.0,
    )
    .await
}
#[tauri::command]
pub async fn voice_recovery(app: AppHandle) -> Result<Option<data::Accept>> {
    let pool = crate::workspace::pool(&app).await?;
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key='voice_pending'")
            .fetch_optional(&pool)
            .await?;
    raw.map(|v| {
        serde_json::from_str(&v)
            .map_err(|_| AppError::validation("Voice recovery data is invalid."))
    })
    .transpose()
}
#[tauri::command]
pub async fn set_voice_recovery(app: AppHandle, input: Option<data::Accept>) -> Result<()> {
    let pool = crate::workspace::pool(&app).await?;
    if let Some(input) = input {
        sqlx::query("INSERT INTO settings(key,value) VALUES('voice_pending',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(serde_json::to_string(&input).unwrap()).execute(&pool).await?;
    } else {
        sqlx::query("DELETE FROM settings WHERE key='voice_pending'")
            .execute(&pool)
            .await?;
    }
    Ok(())
}
pub fn initialize(app: &AppHandle) -> Result<()> {
    let root = settings::root(app)?;
    // Only this app-owned scratch directory; never recordings or user attachments.
    let temp = root.join("temp");
    if temp.is_dir() {
        std::fs::remove_dir_all(&temp)?;
    }
    std::fs::create_dir_all(temp)?;
    Ok(())
}
#[cfg(test)]
mod tests;
