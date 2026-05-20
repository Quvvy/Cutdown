mod clip_history;
mod encoder_detect;
mod ffmpeg;
mod launch;
mod presets;
mod settings;
mod upload;
mod watch_folder;
mod windows_integration;

use launch::LaunchState;
use settings::{AppSettings, UpdateSettingsParams};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WindowEvent};

pub fn show_editor_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window was not found".to_string())?;

    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn show_editor(app: AppHandle) -> Result<(), String> {
    show_editor_window(&app)
}

#[tauri::command]
fn path_exists(path: String) -> bool {
    std::path::PathBuf::from(path).exists()
}

#[tauri::command]
fn update_watch_folder(
    app: AppHandle,
    path: Option<String>,
    enabled: bool,
) -> Result<AppSettings, String> {
    let settings = settings::update_watch_folder_settings(path, enabled)?;
    watch_folder::restart_watcher(app)?;
    Ok(settings)
}

#[tauri::command]
fn save_app_settings(
    app: AppHandle,
    params: UpdateSettingsParams,
) -> Result<AppSettings, String> {
    let settings = settings::update_settings(params)?;
    watch_folder::restart_watcher(app)?;
    Ok(settings)
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open_editor = MenuItem::with_id(app, "open_editor", "Open Editor", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_editor, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Cutdown")
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_editor" => {
                if let Err(err) = show_editor_window(app) {
                    eprintln!("failed to show editor window: {err}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(err) = show_editor_window(tray.app_handle()) {
                    eprintln!("failed to show editor window: {err}");
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(LaunchState::new())
        .setup(|app| {
            setup_tray(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let window_for_close = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                    }
                });
            }

            watch_folder::manage_state(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_editor,
            path_exists,
            update_watch_folder,
            save_app_settings,
            launch::get_launch_path,
            ffmpeg::probe_video,
            ffmpeg::export_clip,
            ffmpeg::check_ffmpeg,
            ffmpeg::reveal_in_explorer,
            ffmpeg::prepare_preview,
            ffmpeg::cleanup_preview,
            settings::get_settings,
            settings::set_last_export_dir,
            settings::set_last_preset_id,
            presets::list_presets,
            encoder_detect::detect_gpu_encoders,
            windows_integration::set_run_at_startup,
            clip_history::list_clip_history,
            clip_history::clear_clip_history,
            clip_history::remove_clip_history_entry,
            upload::upload_to_catbox,
            upload::copy_text_to_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cutdown");
}
