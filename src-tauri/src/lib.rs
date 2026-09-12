mod db;
mod tray;
mod window;
use std::sync::atomic::AtomicBool;
use tauri::Manager;
pub fn run() {
    let requested_seed = std::env::args().any(|arg| arg == "--seed");
    if requested_seed && !cfg!(debug_assertions) {
        eprintln!("--seed is only available in debug builds.");
        std::process::exit(2);
    }
    let database = if requested_seed {
        db::SEED_DB
    } else {
        db::USER_DB
    };
    tauri::Builder::default()
        .manage(db::RuntimeState {
            seeded: requested_seed,
            close_to_tray: AtomicBool::new(false),
        })
        .manage(window::GeometryReady::default())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(database, db::migrations())
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            db::runtime_config,
            db::load_tray_preference,
            db::set_tray_preference,
            db::seed_database
        ])
        .setup(move |app| {
            tray::setup(app)?;
            let window = app
                .get_webview_window("main")
                .ok_or("Main window missing")?;
            if requested_seed {
                window.set_title("Signal · September 2025 fixture")?;
            }
            window::restore(&window)?;
            Ok(())
        })
        .on_window_event(window::handle)
        .run(tauri::generate_context!())
        .expect("Signal could not start");
}
