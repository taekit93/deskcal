use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::commands::AppState;
use crate::error::AppError;
use crate::settings::Settings;

/// 트레이의 "위치·크기 고정" 체크 항목. 상단 바 핀 버튼에서 바꿔도 체크 표시를 맞추려고 보관한다.
pub struct TrayLockItem(pub CheckMenuItem<tauri::Wry>);

/// 고정 상태를 뒤집고, 트레이 체크 표시와 화면(settings-changed)을 함께 맞춘다.
pub fn toggle_lock_everywhere(app: &AppHandle) -> Result<Settings, AppError> {
    let result = crate::commands::toggle_lock(&app.state::<AppState>());
    let locked = app.state::<AppState>().settings.lock().unwrap().locked;
    if let Some(item) = app.try_state::<TrayLockItem>() {
        let _ = item.0.set_checked(locked);
    }
    if let Ok(updated) = &result {
        let _ = app.emit("settings-changed", updated);
    }
    result
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let locked = app.state::<AppState>().settings.lock().unwrap().locked;
    let lock_item = CheckMenuItemBuilder::with_id("lock", "위치·크기 고정").checked(locked).build(app)?;
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
        .tooltip("DeskCal")
        .menu(&menu)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .build(app)?;
    app.manage(TrayLockItem(lock_item));
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
            if let Err(e) = toggle_lock_everywhere(app) {
                log::error!("설정 저장 실패: {e}");
            }
        }
        "logout" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::commands::sign_out(&app.state::<AppState>()).await {
                    log::error!("로그아웃 실패: {e}");
                    let _ = app.emit("logout-failed", e.to_string());
                    return;
                }
                let _ = app.emit("logged-out", ());
            });
        }
        "quit" => app.exit(0),
        _ => {}
    }
}
