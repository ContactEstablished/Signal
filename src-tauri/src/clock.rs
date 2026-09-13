use crate::workspace::models::{AppError, Result};
use std::sync::RwLock;
use tauri::{AppHandle, Manager};
#[derive(Default)]
pub struct Clock {
    pub offset: RwLock<Option<i64>>,
    pub initialize_gate: tokio::sync::Mutex<()>,
    pub tray_gate: tokio::sync::Mutex<()>,
}
pub fn millis(value: &str) -> Result<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|v| v.timestamp_millis())
        .map_err(|_| AppError::validation("Invalid UTC instant."))
}
pub fn iso(ms: i64) -> Result<String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|v| v.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .ok_or_else(|| AppError::validation("Time is outside the supported range."))
}
pub fn sample(app: &AppHandle) -> Result<(String, i64)> {
    let state = app.state::<Clock>();
    let offset = state
        .offset
        .read()
        .map_err(|_| AppError::new("Database", "Clock lock failed."))?
        .ok_or_else(|| {
            AppError::new("Database", "Initialize the workspace clock before editing.")
        })?;
    let ms = chrono::Utc::now()
        .timestamp_millis()
        .checked_add(offset)
        .ok_or_else(|| AppError::validation("Clock overflow."))?;
    Ok((iso(ms)?, offset))
}
