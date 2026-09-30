use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Local;
use futures::future::join_all;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::auth::Auth;
use crate::cache::{with_fallback, Cache};
use crate::error::AppError;
use crate::google::models::{MonthData, TasksData};
use crate::google::Google;
use crate::range::{grid_range, to_rfc3339_local};
use crate::settings::{self, Settings};

pub const TASKS_KEY: &str = "tasks";

pub struct AppState {
    pub auth: Arc<Auth>,
    pub google: Google,
    pub cache: Cache,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
}

pub fn month_key(year: i32, month: u32) -> String {
    format!("month:{year}-{month:02}")
}

fn is_fatal(e: &AppError) -> bool {
    matches!(e, AppError::AuthExpired | AppError::NotLoggedIn | AppError::Network(_))
}

pub fn merge_results<T>(
    ids: Vec<String>,
    results: Vec<Result<Vec<T>, AppError>>,
) -> Result<(Vec<T>, Vec<String>), AppError> {
    let mut items = Vec::new();
    let mut failed = Vec::new();
    for (id, result) in ids.into_iter().zip(results) {
        match result {
            Ok(v) => items.extend(v),
            Err(e) if is_fatal(&e) => return Err(e),
            Err(e) => {
                log::warn!("{id} 조회 실패: {e}");
                failed.push(id);
            }
        }
    }
    Ok((items, failed))
}

#[tauri::command]
pub fn auth_status(state: State<'_, AppState>) -> bool {
    state.auth.is_logged_in()
}

#[tauri::command]
pub async fn login(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .auth
        .login(|url| app.opener().open_url(url, None::<&str>).map_err(|e| AppError::Login(e.to_string())))
        .await
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), AppError> {
    sign_out(&state).await
}

/// 로그아웃: 토큰과 함께 이전 계정의 캐시 데이터도 지운다.
pub async fn sign_out(state: &AppState) -> Result<(), AppError> {
    state.auth.logout().await?;
    state.cache.clear()
}

async fn fetch_month(state: &AppState, year: i32, month: u32) -> Result<MonthData, AppError> {
    let (start, end) = grid_range(year, month).ok_or_else(|| AppError::Config(format!("잘못된 월: {year}-{month}")))?;
    let (time_min, time_max) = (to_rfc3339_local(start), to_rfc3339_local(end));
    let calendars = state.google.list_calendars().await?;
    let hidden = state.settings.lock().unwrap().hidden_calendars.clone();
    let visible: Vec<_> = calendars.iter().filter(|c| !hidden.contains(&c.id)).collect();
    let results = join_all(visible.iter().map(|c| state.google.list_events(&c.id, &time_min, &time_max))).await;
    let (events, failed) = merge_results(visible.iter().map(|c| c.id.clone()).collect(), results)?;
    Ok(MonthData { calendars, events, failed, fetched_at: Local::now().to_rfc3339(), stale: false })
}

#[tauri::command]
pub async fn get_month(state: State<'_, AppState>, year: i32, month: u32) -> Result<MonthData, AppError> {
    let result = fetch_month(&state, year, month).await;
    with_fallback(&state.cache, &month_key(year, month), result, |d| d.stale = true)
}

#[tauri::command]
pub fn peek_month(state: State<'_, AppState>, year: i32, month: u32) -> Option<MonthData> {
    state.cache.get(&month_key(year, month))
}

async fn fetch_tasks(state: &AppState) -> Result<TasksData, AppError> {
    let lists = state.google.list_tasklists().await?;
    let hidden = state.settings.lock().unwrap().hidden_task_lists.clone();
    let visible: Vec<_> = lists.iter().filter(|l| !hidden.contains(&l.id)).collect();
    let results = join_all(visible.iter().map(|l| state.google.list_tasks(&l.id))).await;
    let (tasks, failed) = merge_results(visible.iter().map(|l| l.id.clone()).collect(), results)?;
    Ok(TasksData { lists, tasks, failed, fetched_at: Local::now().to_rfc3339(), stale: false })
}

#[tauri::command]
pub async fn get_tasks(state: State<'_, AppState>) -> Result<TasksData, AppError> {
    let result = fetch_tasks(&state).await;
    with_fallback(&state.cache, TASKS_KEY, result, |d| d.stale = true)
}

#[tauri::command]
pub fn peek_tasks(state: State<'_, AppState>) -> Option<TasksData> {
    state.cache.get(TASKS_KEY)
}

#[tauri::command]
pub async fn set_task_completed(
    state: State<'_, AppState>,
    list_id: String,
    task_id: String,
    completed: bool,
) -> Result<(), AppError> {
    state.google.set_task_completed(&list_id, &task_id, completed).await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<Settings, AppError> {
    let saved = persist_settings(&state, settings)?;
    apply_autostart(&app, saved.autostart);
    Ok(saved)
}

/// 설정을 검증·저장하고, 실제로 저장된 값을 돌려준다 (화면은 이 값을 기준으로 삼는다).
pub fn persist_settings(state: &AppState, settings: Settings) -> Result<Settings, AppError> {
    let settings = settings.normalized();
    settings::save(&state.settings_path, &settings)?;
    *state.settings.lock().unwrap() = settings.clone();
    Ok(settings)
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    if let Err(e) = result {
        log::warn!("자동 실행 설정 실패: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_cache(dir: &std::path::Path) -> AppState {
        use crate::auth::{MemoryStore, OAuthConfig};
        let cfg = OAuthConfig {
            client_id: "cid".into(),
            client_secret: "cs".into(),
            auth_url: "http://127.0.0.1:1/auth".into(),
            token_url: "http://127.0.0.1:1/token".into(),
            revoke_url: "http://127.0.0.1:1/revoke".into(),
        };
        let auth = Arc::new(Auth::new(cfg, Box::new(MemoryStore::with("r1")), reqwest::Client::new()));
        AppState {
            google: Google::new(reqwest::Client::new(), auth.clone()),
            auth,
            cache: Cache::new(dir.join("cache.json")),
            settings: Mutex::new(Settings::default()),
            settings_path: dir.join("settings.json"),
        }
    }

    #[test]
    fn persist_settings_returns_and_stores_the_normalized_value() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_cache(dir.path());
        let saved = persist_settings(&state, Settings { refresh_minutes: 0, accent: Some("bad".into()), ..Settings::default() }).unwrap();
        assert_eq!(saved.refresh_minutes, 1);
        assert_eq!(saved.accent, None);
        assert_eq!(*state.settings.lock().unwrap(), saved);
        assert_eq!(settings::load(&state.settings_path), saved);
    }

    #[tokio::test]
    async fn sign_out_clears_token_and_cached_account_data() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_cache(dir.path());
        state.cache.put(TASKS_KEY, &vec!["이전 계정 할 일".to_string()]).unwrap();
        state.cache.put(&month_key(2026, 9), &vec!["이전 계정 일정".to_string()]).unwrap();
        assert!(state.auth.is_logged_in());

        sign_out(&state).await.unwrap();

        assert!(!state.auth.is_logged_in());
        assert_eq!(state.cache.get::<Vec<String>>(TASKS_KEY), None);
        assert_eq!(state.cache.get::<Vec<String>>(&month_key(2026, 9)), None);
    }

    #[test]
    fn merge_collects_successes_and_failed_ids() {
        let r = merge_results(
            vec!["a".into(), "b".into(), "c".into()],
            vec![Ok(vec![1, 2]), Err(AppError::Api { status: 403, message: "forbidden".into() }), Ok(vec![3])],
        )
        .unwrap();
        assert_eq!(r, (vec![1, 2, 3], vec!["b".to_string()]));
    }

    #[test]
    fn merge_propagates_network_error_so_cache_fallback_applies() {
        let r = merge_results::<i32>(vec!["a".into(), "b".into()], vec![Ok(vec![1]), Err(AppError::Network("down".into()))]);
        assert!(matches!(r, Err(AppError::Network(_))));
    }

    #[test]
    fn merge_propagates_auth_errors() {
        let r = merge_results::<i32>(vec!["a".into()], vec![Err(AppError::AuthExpired)]);
        assert!(matches!(r, Err(AppError::AuthExpired)));
    }

    #[test]
    fn month_key_is_zero_padded() {
        assert_eq!(month_key(2026, 9), "month:2026-09");
    }
}
