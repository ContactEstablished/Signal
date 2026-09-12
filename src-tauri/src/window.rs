use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{LogicalSize, Manager, PhysicalPosition, WebviewWindow, Window, WindowEvent};
use tauri_plugin_store::StoreExt;

#[derive(Default)]
pub struct GeometryReady(pub AtomicBool, pub AtomicU64);
#[derive(Serialize, Deserialize)]
struct Geometry {
    x: i32,
    y: i32,
    width: f64,
    height: f64,
    maximized: bool,
}
fn store_file(window: &impl Manager<tauri::Wry>) -> &'static str {
    if window.state::<crate::db::RuntimeState>().seeded {
        "window-dev.json"
    } else {
        "window.json"
    }
}
pub fn restore(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let store = window.app_handle().store(store_file(window))?;
    let has_saved_geometry = store.get("geometry").is_some();
    if let Some(value) = store.get("geometry") {
        if let Ok(g) = serde_json::from_value::<Geometry>(value) {
            if g.width.is_finite()
                && g.height.is_finite()
                && (1200.0..=10000.0).contains(&g.width)
                && (760.0..=10000.0).contains(&g.height)
            {
                window.set_size(LogicalSize::new(g.width, g.height))?;
                // A saved titlebar must still be reachable after monitor changes.
                let visible = window.available_monitors()?.iter().any(|m| {
                    let p = m.position();
                    let s = m.size();
                    g.x >= p.x
                        && g.x < p.x + s.width as i32 - 100
                        && g.y >= p.y
                        && g.y < p.y + s.height as i32 - 50
                });
                if visible {
                    window.set_position(PhysicalPosition::new(g.x, g.y))?;
                } else {
                    window.center()?;
                }
                if g.maximized {
                    window.maximize()?;
                }
            }
        }
    } else {
        window.center()?;
    }
    window
        .state::<GeometryReady>()
        .0
        .store(true, Ordering::SeqCst);
    window.show()?;
    if !has_saved_geometry {
        save(&window.as_ref().window(), true)?;
    }
    Ok(())
}
pub fn save(window: &Window, flush: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !window.state::<GeometryReady>().0.load(Ordering::SeqCst) || window.is_minimized()? {
        return Ok(());
    }
    let store = window.app_handle().store(store_file(window))?;
    let maximized = window.is_maximized()?;
    if maximized {
        if let Some(mut value) = store.get("geometry") {
            value["maximized"] = true.into();
            store.set("geometry", value);
        }
    } else {
        let position = window.outer_position()?;
        let size = window
            .inner_size()?
            .to_logical::<f64>(window.scale_factor()?);
        if size.width >= 1200.0 && size.height >= 760.0 {
            store.set(
                "geometry",
                serde_json::to_value(Geometry {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                    maximized,
                })?,
            );
        }
    }
    if flush {
        store.save()?;
    }
    Ok(())
}
pub fn handle(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if crate::exit_guard::intercept(window.app_handle(), "close") {
            api.prevent_close();
            return;
        }
    }
    if matches!(event, WindowEvent::CloseRequested { .. }) {
        window
            .state::<GeometryReady>()
            .1
            .fetch_add(1, Ordering::SeqCst);
        if let Err(error) = save(window, true) {
            eprintln!("Window geometry: {error}");
        }
    }
    if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
        // Windows emits transient normal-sized events during maximize/restore.
        // Sample once the native state settles, retaining the last normal bounds.
        let generation = window
            .state::<GeometryReady>()
            .1
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        let window = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if window.state::<GeometryReady>().1.load(Ordering::SeqCst) == generation {
                if let Err(error) = save(&window, false) {
                    eprintln!("Window geometry: {error}");
                }
            }
        });
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window
            .state::<crate::db::RuntimeState>()
            .close_to_tray
            .load(Ordering::SeqCst)
            && window.app_handle().tray_by_id("signal-tray").is_some()
        {
            // Only prevent closing once hiding succeeds.
            match window.hide() {
                Ok(()) => api.prevent_close(),
                Err(e) => eprintln!("Could not hide Signal: {e}"),
            }
        }
    }
}
