use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, Manager,
};
pub fn setup(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::with_id("signal-tray")
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/icon.png"
        ))?)
        .tooltip("Signal")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window
                        .unminimize()
                        .and_then(|_| window.show())
                        .and_then(|_| window.set_focus())
                    {
                        eprintln!("Could not restore Signal: {e}");
                    }
                }
            }
            "quit" => {
                if crate::exit_guard::intercept(app, "quit") {
                    return;
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = crate::window::save(&window.as_ref().window(), true);
                }
                app.exit(0);
            }
            _ => (),
        })
        .build(app)?;
    Ok(())
}

/// Read current committed counts under one presentation gate so older updates cannot win.
pub async fn refresh(app: &tauri::AppHandle) -> crate::workspace::models::Result<()> {
    use crate::workspace::models::{rows, AppError};
    let state = app.state::<crate::clock::Clock>();
    let _gate = state.tray_gate.lock().await;
    let pool = crate::db::pool(app)
        .await
        .map_err(|e| AppError::new("Database", e))?;
    let mut conn = pool.acquire().await?;
    let counts = rows(
        &mut conn,
        "SELECT state,COUNT(*) AS n FROM timer_sessions WHERE state!='stopped' GROUP BY state",
        vec![],
    )
    .await?;
    let mut parts = vec!["Signal".to_string()];
    for state in ["running", "paused"] {
        if let Some(row) = counts.iter().find(|v| v["state"] == state) {
            let n = row["n"].as_i64().unwrap_or(0);
            if n > 0 {
                parts.push(format!(
                    "{n} timer{} {state}",
                    if n == 1 { "" } else { "s" }
                ));
            }
        }
    }
    app.tray_by_id("signal-tray")
        .ok_or_else(|| AppError::new("Database", "Tray unavailable."))?
        .set_tooltip(Some(parts.join(" · ")))
        .map_err(|e| AppError::new("Database", e.to_string()))
}
