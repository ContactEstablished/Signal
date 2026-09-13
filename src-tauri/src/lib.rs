mod attachments;
mod clock;
mod db;
mod exit_guard;
mod links;
mod timers;
mod tray;
mod window;
mod workspace;
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
        .manage(clock::Clock::default())
        .manage(window::GeometryReady::default())
        .manage(exit_guard::EditGuard::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(database, db::migrations())
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            timers::initialize_timers,
            timers::get_timers,
            timers::get_task_time,
            timers::start_timer,
            timers::pause_timer,
            timers::resume_timer,
            timers::stop_timer,
            timers::log_time,
            db::runtime_config,
            db::load_tray_preference,
            db::set_tray_preference,
            db::seed_database,
            workspace::list_projects,
            workspace::get_board,
            workspace::get_task_detail,
            workspace::create_project,
            workspace::update_project,
            workspace::move_project,
            workspace::create_task,
            workspace::update_task,
            workspace::move_task,
            workspace::set_subtasks,
            workspace::set_task_tags,
            workspace::set_task_alerts,
            workspace::preview_deletion,
            workspace::delete_entity,
            attachments::initialize_workspace,
            attachments::stage_attachments,
            attachments::discard_staged_attachments,
            attachments::add_attachments,
            attachments::remove_attachment,
            attachments::open_attachment,
            attachments::install_fixture_attachments,
            links::open_external_url,
            links::open_task_link,
            exit_guard::set_edit_guard,
            exit_guard::resolve_exit_request
        ])
        .setup(move |app| {
            let root = app.path().app_data_dir()?.join(if requested_seed {
                "attachments-dev"
            } else {
                "attachments"
            });
            app.manage(attachments::Files::new(root).map_err(|e| e.message)?);
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
