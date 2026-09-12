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
