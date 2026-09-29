use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Calendar,
    Tasks,
    #[default]
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub view_mode: ViewMode,
    pub hidden_calendars: Vec<String>,
    pub hidden_task_lists: Vec<String>,
    pub refresh_minutes: u32,
    pub autostart: bool,
    pub locked: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Both,
            hidden_calendars: Vec::new(),
            hidden_task_lists: Vec::new(),
            refresh_minutes: 10,
            autostart: true,
            locked: false,
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.refresh_minutes = self.refresh_minutes.clamp(1, 120);
        self
    }
}

pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default()
        .normalized()
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), AppError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| AppError::Storage(e.to_string()))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| AppError::Storage(e.to_string()))?;
    fs::write(path, text).map_err(|e| AppError::Storage(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn path() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("nested").join("settings.json");
        (dir, p)
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (_d, p) = path();
        let s = load(&p);
        assert_eq!(s, Settings::default());
        assert_eq!(s.view_mode, ViewMode::Both);
        assert_eq!(s.refresh_minutes, 10);
        assert!(s.autostart);
        assert!(!s.locked);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let (_d, p) = path();
        let mut s = Settings::default();
        s.view_mode = ViewMode::Tasks;
        s.hidden_calendars = vec!["cal-a".into()];
        s.hidden_task_lists = vec!["list-b".into()];
        save(&p, &s).unwrap();
        assert_eq!(load(&p), s);
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let (_d, p) = path();
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "{not json").unwrap();
        assert_eq!(load(&p), Settings::default());
    }

    #[test]
    fn partial_file_fills_missing_fields() {
        let (_d, p) = path();
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, r#"{"viewMode":"calendar"}"#).unwrap();
        let s = load(&p);
        assert_eq!(s.view_mode, ViewMode::Calendar);
        assert_eq!(s.refresh_minutes, 10);
        assert!(s.autostart);
    }

    #[test]
    fn refresh_minutes_is_clamped() {
        let (_d, p) = path();
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, r#"{"refreshMinutes":0}"#).unwrap();
        assert_eq!(load(&p).refresh_minutes, 1);
        fs::write(&p, r#"{"refreshMinutes":9999}"#).unwrap();
        assert_eq!(load(&p).refresh_minutes, 120);
    }

    #[test]
    fn serializes_camel_case_for_frontend() {
        let v = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(v["viewMode"], "both");
        assert!(v.get("hiddenCalendars").is_some());
        assert!(v.get("refreshMinutes").is_some());
    }
}
