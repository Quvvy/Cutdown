#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clip_history;
mod command_util;
mod encoder_detect;
mod ffmpeg;
mod ffmpeg_install;
mod launch;
mod obs;
mod presets;
mod project;
mod secret_store;
mod settings;
mod source_session;
mod upload;
mod upload_providers;
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
    let path = std::path::PathBuf::from(path);
    if !is_supported_user_file(&path) {
        return false;
    }
    path.exists()
}

fn is_supported_user_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("mp4" | "mkv" | "mov" | "webm" | "ts" | "avi" | "flv")
            | Some(project::PROJECT_EXTENSION)
    )
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
fn save_app_settings(app: AppHandle, params: UpdateSettingsParams) -> Result<AppSettings, String> {
    let settings = settings::update_settings(params)?;
    watch_folder::restart_watcher(app)?;
    Ok(settings)
}

#[tauri::command]
fn save_editor_settings(
    app: AppHandle,
    params: settings::SaveEditorSettingsParams,
) -> Result<AppSettings, String> {
    let settings = settings::apply_editor_settings(params)?;
    watch_folder::restart_watcher(app)?;
    Ok(settings)
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open_editor = MenuItem::with_id(app, "open_editor", "Open Editor", true, None::<&str>)?;
    let open_watch_folder = MenuItem::with_id(
        app,
        "open_watch_folder",
        "Open Watch Folder",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_editor, &open_watch_folder, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Cutdown")
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.on_menu_event(|app, event| match event.id().as_ref() {
        "open_editor" => {
            if let Err(err) = show_editor_window(app) {
                eprintln!("failed to show editor window: {err}");
            }
        }
        "open_watch_folder" => {
            let folder = settings::load_settings()
                .watch_folder
                .filter(|path| !path.trim().is_empty());
            if let Some(folder) = folder {
                if let Err(err) = obs::open_watch_folder_in_explorer(folder) {
                    eprintln!("failed to open watch folder: {err}");
                }
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
    if std::env::args().any(|arg| arg == "--install-dependencies") {
        let code = if ffmpeg_install::run_headless_install().is_ok() {
            0
        } else {
            1
        };
        std::process::exit(code);
    }

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

            if settings::load_settings().start_minimized_to_tray {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_editor,
            path_exists,
            update_watch_folder,
            save_app_settings,
            save_editor_settings,
            launch::get_launch_path,
            ffmpeg::probe_video,
            ffmpeg::extract_waveform,
            ffmpeg::export_clip,
            ffmpeg::cancel_export,
            ffmpeg::check_ffmpeg,
            ffmpeg_install::install_ffmpeg,
            ffmpeg::reveal_in_explorer,
            ffmpeg::prepare_preview,
            ffmpeg::cleanup_preview,
            settings::get_settings,
            settings::push_recent_source,
            settings::set_last_export_dir,
            settings::set_last_preset_id,
            presets::list_presets,
            encoder_detect::detect_gpu_encoders,
            windows_integration::set_run_at_startup,
            clip_history::list_clip_history,
            clip_history::clear_clip_history,
            clip_history::remove_clip_history_entry,
            clip_history::update_clip_history_share_url,
            upload::upload_file,
            upload::list_upload_providers,
            upload::save_upload_providers,
            upload::get_upload_providers_for_editor,
            upload::copy_text_to_clipboard,
            source_session::get_source_session,
            source_session::save_source_session,
            project::save_project_file,
            project::load_project_file,
            obs::find_latest_replay_in_folder,
            obs::open_watch_folder_in_explorer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cutdown");
}
