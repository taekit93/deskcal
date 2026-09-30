use tauri::menu::{CheckMenuItemBuilder, MenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::commands::AppState;
use crate::settings;

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let locked = app.state::<AppState>().settings.lock().unwrap().locked;
    let lock_item = CheckMenuItemBuilder::with_id("lock", "위치 잠금").checked(locked).build(app)?;
    let menu = MenuBuilder::new(app)
        .text("refresh", "새로고침")
        .text("settings", "설정")
        .text("open_calendar", "브라우저에서 캘린더 열기")
        .separator()
        .item(&lock_item)
        .separator()
        .text("logout", "로그아웃")
        .text("quit", "종료")
        .build()?;
    let icon = app.default_window_icon().cloned().expect("bundle icon is configured");
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Google 캘린더 위젯")
        .menu(&menu)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "refresh" => {
            let _ = app.emit("refresh", ());
        }
        "settings" => {
            let _ = app.emit("open-settings", ());
        }
        "open_calendar" => {
            let _ = app.opener().open_url("https://calendar.google.com", None::<&str>);
        }
        "lock" => {
            let state = app.state::<AppState>();
            let updated = {
                let mut s = state.settings.lock().unwrap();
                s.locked = !s.locked;
                s.clone()
            };
            if let Err(e) = settings::save(&state.settings_path, &updated) {
                log::error!("설정 저장 실패: {e}");
            }
            let _ = app.emit("settings-changed", &updated);
        }
        "logout" => {
            if let Err(e) = crate::commands::sign_out(&app.state::<AppState>()) {
                log::error!("로그아웃 실패: {e}");
            }
            let _ = app.emit("logged-out", ());
        }
        "quit" => app.exit(0),
        _ => {}
    }
}
