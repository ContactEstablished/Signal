use crate::workspace::models::*;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
#[derive(Default)]
pub struct EditGuard(Mutex<GuardState>);
#[derive(Default)]
struct GuardState {
    active: bool,
    pending: Option<(String, String)>,
}
impl GuardState {
    fn request(&mut self, intent: &str) -> Option<(String, String)> {
        if !self.active {
            return None;
        }
        Some(
            self.pending
                .get_or_insert_with(|| (uid(), intent.to_string()))
                .clone(),
        )
    }
    fn resolve(&mut self, request_id: &str, proceed: bool) -> Result<Option<String>> {
        let (id, intent) = self
            .pending
            .as_ref()
            .ok_or_else(AppError::conflict)?
            .clone();
        if id != request_id {
            return Err(AppError::conflict());
        }
        self.pending = None;
        if proceed {
            self.active = false;
            Ok(Some(intent))
        } else {
            Ok(None)
        }
    }
}
pub fn intercept(app: &AppHandle, intent: &str) -> bool {
    let state = app.state::<EditGuard>();
    let mut guard = state.0.lock().unwrap();
    let Some(request) = guard.request(intent) else {
        return false;
    };
    drop(guard);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        if let Err(e) = window.emit(
            "signal://exit-request",
            serde_json::json!({"requestId":request.0,"intent":request.1}),
        ) {
            eprintln!("Could not request editor closure: {e}");
        }
    }
    true
}
#[tauri::command]
pub fn set_edit_guard(app: AppHandle, active: bool) -> Result<()> {
    let state = app.state::<EditGuard>();
    let mut guard = state.0.lock().unwrap();
    if guard.pending.is_some() && !active {
        return Err(AppError::conflict());
    }
    guard.active = active;
    Ok(())
}
#[tauri::command]
pub fn resolve_exit_request(app: AppHandle, request_id: String, proceed: bool) -> Result<()> {
    let state = app.state::<EditGuard>();
    let mut guard = state.0.lock().unwrap();
    let Some(intent) = guard.resolve(&request_id, proceed)? else {
        return Ok(());
    };
    drop(guard);
    let window = app
        .get_webview_window("main")
        .ok_or_else(AppError::missing)?;
    if intent == "quit" {
        let _ = crate::window::save(&window.as_ref().window(), true);
        app.exit(0);
    } else {
        window
            .close()
            .map_err(|e| AppError::new("File", e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_exit_keeps_original_intent_and_stale_answers_cannot_clear_a_draft() {
        let mut guard = GuardState::default();
        assert!(guard.request("close").is_none());
        guard.active = true;
        let close = guard.request("close").unwrap();
        assert_eq!(guard.request("quit").unwrap(), close);
        assert!(guard.resolve("stale", true).is_err());
        assert!(guard.active);
        assert_eq!(guard.resolve(&close.0, false).unwrap(), None);
        assert!(guard.active);
        let quit = guard.request("quit").unwrap();
        assert_ne!(quit.0, close.0);
        assert!(guard.resolve(&close.0, true).is_err());
        assert_eq!(guard.resolve(&quit.0, true).unwrap(), Some("quit".into()));
        assert!(!guard.active);
        assert!(guard.request("close").is_none());
    }
}
