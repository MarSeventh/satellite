mod config;
mod db;
mod remote;
mod upload;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};

#[tauri::command]
fn toggle_main_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn hide_main_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn set_floating_visible(app: tauri::AppHandle, visible: bool) {
    if let Some(win) = app.get_webview_window("floating") {
        if visible {
            let _ = win.show();
        } else {
            let _ = win.hide();
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            let database =
                db::Database::new().expect("Failed to initialize database");
            app.manage(database);

            // Check config for floating window visibility
            let cfg = config::load_config();
            if !cfg.show_floating {
                // Floating window starts hidden (visible: false in config),
                // FloatingBall.svelte calls appWindow.show() on mount,
                // but we suppress that by keeping it hidden via config.
                // We'll handle this by emitting a config event.
            }

            // ----- System tray -----
            let show_item =
                MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
            let hide_item =
                MenuItemBuilder::with_id("hide", "隐藏主窗口").build(app)?;
            let quit_item =
                MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .items(&[&show_item, &hide_item, &quit_item])
                .build()?;

            let tray_icon = create_tray_icon();

            TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip("Satellite")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Prevent main window close — hide instead
            if let Some(main_win) = app.get_webview_window("main") {
                let main_win_clone = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            upload::upload_files,
            db::get_history,
            db::get_history_count,
            db::delete_history,
            config::get_config,
            config::save_config,
            toggle_main_window,
            show_main_window,
            hide_main_window,
            set_floating_visible,
            remote::list_remote_files,
            remote::delete_remote_file,
            remote::download_remote_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Generate a simple 32×32 blue circle icon for the tray (RGBA).
fn create_tray_icon() -> tauri::image::Image<'static> {
    let size: u32 = 32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let center = size as f32 / 2.0;
    let radius = center - 1.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                // Smooth edge (anti-alias 1px)
                let alpha = if dist > radius - 1.0 {
                    ((radius - dist).max(0.0) * 255.0) as u8
                } else {
                    255u8
                };
                // Blue accent color: #89b4fa
                rgba.extend_from_slice(&[0x89, 0xB4, 0xFA, alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    tauri::image::Image::new_owned(rgba, size, size)
}
