mod auth;
mod cache;
mod commands;
mod desktop;
mod error;
mod google;
mod range;
mod settings;
mod tray;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;
use tauri_plugin_window_state::StateFlags;

use crate::auth::{Auth, KeyringStore, OAuthConfig};
use crate::cache::Cache;
use crate::commands::AppState;
use crate::google::Google;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout))
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                    file_name: Some("gcal-widget".into()),
                }))
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings_path = config_dir.join("settings.json");
            let loaded = settings::load(&settings_path);
            let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build()?;
            let auth = Arc::new(Auth::new(OAuthConfig::from_build_env(), Box::new(KeyringStore::new()?), http.clone()));
            let google = Google::new(http, auth.clone());
            let cache = Cache::new(config_dir.join("cache.json"));
            if loaded.autostart {
                commands::apply_autostart(app.handle(), true);
            }
            app.manage(AppState { auth, google, cache, settings: Mutex::new(loaded), settings_path });
            tray::setup_tray(app.handle())?;
            let window = app.get_webview_window("main").expect("main window is configured");
            desktop::pin(&window);
            window.show()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth_status,
            commands::login,
            commands::logout,
            commands::get_month,
            commands::peek_month,
            commands::get_tasks,
            commands::peek_tasks,
            commands::set_task_completed,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
