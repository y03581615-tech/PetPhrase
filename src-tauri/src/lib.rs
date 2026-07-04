mod pet_loader;
mod storage;

use pet_loader::PetInfo;
use std::path::PathBuf;
use storage::{PhraseData, Settings};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").expect("APPDATA 环境变量缺失")).join("PetPhrase")
}

#[tauri::command]
fn get_phrases() -> PhraseData {
    storage::load_phrases(&data_dir())
}

#[tauri::command]
fn save_phrases(app: AppHandle, data: PhraseData) -> Result<(), String> {
    storage::save_phrases(&data_dir(), &data).map_err(|e| e.to_string())?;
    app.emit("data-changed", ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings() -> Settings {
    storage::load_settings(&data_dir())
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    storage::save_settings(&data_dir(), &settings).map_err(|e| e.to_string())?;
    app.emit("settings-changed", &settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_pets(app: AppHandle) -> Vec<PetInfo> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(resources) = app.path().resource_dir() {
        roots.push(resources.join("pets"));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        roots.push(PathBuf::from(home).join(".codex").join("pets"));
    }
    if let Some(custom) = storage::load_settings(&data_dir()).custom_pet_dir {
        roots.push(PathBuf::from(custom));
    }
    let refs: Vec<&std::path::Path> = roots.iter().map(|p| p.as_path()).collect();
    pet_loader::scan_pets(&refs)
}

#[tauri::command]
fn export_phrases(path: String) -> Result<(), String> {
    storage::export_phrases(&data_dir(), &PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_phrases(app: AppHandle, path: String) -> Result<PhraseData, String> {
    let data =
        storage::import_phrases(&data_dir(), &PathBuf::from(path)).map_err(|e| e.to_string())?;
    app.emit("data-changed", ()).map_err(|e| e.to_string())?;
    Ok(data)
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle-pet", "显示/隐藏宠物", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "open-settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("缺少应用图标").clone())
        .tooltip("PetPhrase")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle-pet" => {
                if let Some(pet) = app.get_webview_window("pet") {
                    if pet.is_visible().unwrap_or(false) {
                        let _ = pet.hide();
                        if let Some(panel) = app.get_webview_window("panel") {
                            let _ = panel.hide();
                        }
                    } else {
                        let _ = pet.show();
                    }
                }
            }
            "open-settings" => {
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(pet) = app.get_webview_window("pet") {
                let _ = pet.show();
                let _ = pet.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            storage::backup_phrases(&data_dir());
            setup_tray(app)?;

            if let Some(panel) = app.get_webview_window("panel") {
                // acrylic 失败 → 通知前端退化实底主题
                if window_vibrancy::apply_acrylic(&panel, Some((255, 255, 255, 140))).is_err() {
                    let _ = app.emit("vibrancy-failed", ());
                }
            }
            if let Some(preview) = app.get_webview_window("preview") {
                let _ = preview.set_ignore_cursor_events(true);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_phrases,
            save_phrases,
            get_settings,
            save_settings,
            list_pets,
            export_phrases,
            import_phrases
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // 窗口全隐藏时保持常驻;仅托盘「退出」(app.exit) 真正退出
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
