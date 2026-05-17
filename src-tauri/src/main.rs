mod encoder_detect;
mod ffmpeg;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window was not found".to_string())?;

    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn show_editor(app: AppHandle) -> Result<(), String> {
    show_main_window(&app)
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open_editor = MenuItem::with_id(app, "open_editor", "Open Editor", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_editor, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("Cutdown")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_editor" => {
                if let Err(err) = show_main_window(app) {
                    eprintln!("failed to show editor window: {err}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_editor,
            ffmpeg::probe_video,
            ffmpeg::export_clip,
            encoder_detect::detect_gpu_encoders
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cutdown");
}
