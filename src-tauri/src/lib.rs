#![cfg_attr(target_os = "macos", allow(unexpected_cfgs))]

mod config;
mod db;
#[cfg(target_os = "windows")]
mod floating;
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
fn set_floating_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("floating") {
        #[cfg(target_os = "windows")]
        let _ = win.set_focusable(false);

        if visible {
            #[cfg(target_os = "windows")]
            floating::apply_circle(&win)?;
            win.show().map_err(|error| error.to_string())?;
        } else {
            win.hide().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            let database = db::Database::new().expect("Failed to initialize database");
            app.manage(database);

            // ----- Create floating window dynamically -----------------------
            // macOS uses alpha compositing. Windows uses an opaque WebView2 host
            // with a circular native region, avoiding the previous DWM/color-key issues.
            {
                let use_transparent = cfg!(target_os = "macos");

                let floating = tauri::WebviewWindowBuilder::new(
                    app,
                    "floating",
                    tauri::WebviewUrl::App("/".into()),
                )
                .title("")
                .inner_size(76.0, 76.0)
                .position(48.0, 188.0)
                .visible(false)
                .decorations(false)
                .always_on_top(true)
                .transparent(use_transparent)
                .background_color(tauri::window::Color(
                    218,
                    230,
                    243,
                    if use_transparent { 0 } else { 255 },
                ))
                .shadow(false)
                .resizable(false)
                .skip_taskbar(true)
                .build()?;

                if let Some(icon) = app.default_window_icon().cloned() {
                    let _ = floating.set_icon(icon);
                }
            }

            // ----- System tray -----
            let show_item = MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
            let hide_item = MenuItemBuilder::with_id("hide", "隐藏主窗口").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .items(&[&show_item, &hide_item, &quit_item])
                .build()?;

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .expect("missing default app icon");

            if let Some(icon) = app.default_window_icon().cloned() {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.set_icon(icon);
                }
            }

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

            // ----- Platform-specific floating window setup ------------------
            #[cfg(target_os = "windows")]
            if let Some(win) = app.get_webview_window("floating") {
                let _ = win.set_focusable(false);
                floating::apply_circle(&win).map_err(std::io::Error::other)?;
                let floating_window = win.clone();
                win.on_window_event(move |event| {
                    if matches!(
                        event,
                        tauri::WindowEvent::Resized(_)
                            | tauri::WindowEvent::ScaleFactorChanged { .. }
                    ) {
                        if let Err(error) = floating::apply_circle(&floating_window) {
                            eprintln!("Failed to resize floating window region: {error}");
                        }
                    }
                });
            }

            #[cfg(target_os = "macos")]
            #[allow(unexpected_cfgs)]
            {
                use objc::{class, msg_send, sel, sel_impl};

                if let Some(win) = app.get_webview_window("floating") {
                    unsafe {
                        if let Ok(ns_win) = win.ns_window() {
                            let ns_win = ns_win as *mut objc::runtime::Object;
                            let clear: *mut objc::runtime::Object =
                                msg_send![class!(NSColor), clearColor];
                            let _: () = msg_send![ns_win, setBackgroundColor: clear];
                            let _: () = msg_send![ns_win, setOpaque: objc::runtime::NO];
                            let _: () = msg_send![ns_win, setHasShadow: objc::runtime::NO];

                            let content_view: *mut objc::runtime::Object =
                                msg_send![ns_win, contentView];
                            if !content_view.is_null() {
                                let _: () =
                                    msg_send![content_view, setWantsLayer: objc::runtime::YES];
                                let layer: *mut objc::runtime::Object =
                                    msg_send![content_view, layer];
                                if !layer.is_null() {
                                    let radius: f64 = 38.0;
                                    let _: () = msg_send![layer, setCornerRadius: radius];
                                    let _: () =
                                        msg_send![layer, setMasksToBounds: objc::runtime::YES];
                                }
                            }
                        }
                    }
                    let _ = win.with_webview(|webview| {
                        #[allow(unexpected_cfgs)]
                        unsafe {
                            use objc::{class, msg_send, sel, sel_impl};
                            let wv = webview.inner() as *mut objc::runtime::Object;
                            let no = objc::runtime::NO;
                            let ns_no: *mut objc::runtime::Object =
                                msg_send![class!(NSNumber), numberWithBool: no];
                            let ns_key: *mut objc::runtime::Object =
                                msg_send![class!(NSString), stringWithUTF8String:
                                    b"drawsBackground\0".as_ptr()];
                            let _: () = msg_send![wv, setValue: ns_no forKey: ns_key];
                        }
                    });
                }
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
