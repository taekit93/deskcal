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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    #[default]
    Dark,
    Light,
    Midnight,
    Forest,
    Rose,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontScale {
    Small,
    #[default]
    Normal,
    Large,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HeaderMode {
    #[default]
    Always,
    Hover,
}

const DEFAULT_OPACITY: f32 = 0.86;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub view_mode: ViewMode,
    pub hidden_calendars: Vec<String>,
    pub hidden_task_lists: Vec<String>,
    pub refresh_minutes: u32,
    pub autostart: bool,
    pub locked: bool,
    pub theme: Theme,
    /// `#rrggbb`. None이면 테마 기본 강조색.
    pub accent: Option<String>,
    /// 배경 불투명도 (0.3~1.0).
    pub opacity: f32,
    pub font_scale: FontScale,
    pub header_mode: HeaderMode,
    pub show_day_detail: bool,
    pub show_due: bool,
    pub show_task_dots: bool,
    pub show_border: bool,
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
            theme: Theme::Dark,
            accent: None,
            opacity: DEFAULT_OPACITY,
            font_scale: FontScale::Normal,
            header_mode: HeaderMode::Always,
            show_day_detail: true,
            show_due: true,
            show_task_dots: true,
            show_border: true,
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.refresh_minutes = self.refresh_minutes.clamp(1, 120);
        self.opacity = if self.opacity.is_finite() { self.opacity.clamp(0.3, 1.0) } else { DEFAULT_OPACITY };
        // 강조색은 CSS에 그대로 들어가므로 #rrggbb 형식만 허용한다.
        self.accent = self.accent.filter(|a| is_hex_color(a)).map(|a| a.to_ascii_lowercase());
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
    fn appearance_defaults_keep_current_look() {
        let s = Settings::default();
        assert_eq!(s.theme, Theme::Dark);
        assert_eq!(s.accent, None);
        assert!((s.opacity - 0.86).abs() < 1e-6);
        assert_eq!(s.font_scale, FontScale::Normal);
        assert_eq!(s.header_mode, HeaderMode::Always);
        assert!(s.show_day_detail && s.show_due && s.show_task_dots && s.show_border);
    }

    #[test]
    fn old_settings_file_gets_appearance_defaults() {
        let (_d, p) = path();
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, r#"{"viewMode":"tasks","hiddenCalendars":["x"],"refreshMinutes":15,"autostart":false,"locked":true}"#).unwrap();
        let s = load(&p);
        assert_eq!(s.view_mode, ViewMode::Tasks);
        assert!(s.locked);
        assert_eq!(s.theme, Theme::Dark);
        assert_eq!(s.header_mode, HeaderMode::Always);
        assert!(s.show_border);
    }

    #[test]
    fn opacity_is_clamped() {
        assert!((Settings { opacity: 0.1, ..Settings::default() }.normalized().opacity - 0.3).abs() < 1e-6);
        assert!((Settings { opacity: 2.0, ..Settings::default() }.normalized().opacity - 1.0).abs() < 1e-6);
        assert!((Settings { opacity: f32::NAN, ..Settings::default() }.normalized().opacity - 0.86).abs() < 1e-6);
    }

    #[test]
    fn invalid_accent_is_dropped_and_valid_is_lowercased() {
        let bad = Settings { accent: Some("red; background:url(x)".into()), ..Settings::default() }.normalized();
        assert_eq!(bad.accent, None);
        let ok = Settings { accent: Some("#8AB4F8".into()), ..Settings::default() }.normalized();
        assert_eq!(ok.accent.as_deref(), Some("#8ab4f8"));
    }

    #[test]
    fn appearance_serializes_for_frontend() {
        let v = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["fontScale"], "normal");
        assert_eq!(v["headerMode"], "always");
        assert!(v["accent"].is_null());
        assert!(v.get("showDayDetail").is_some());
        assert!(v.get("showTaskDots").is_some());
        let midnight: Settings = serde_json::from_str(r#"{"theme":"midnight","headerMode":"hover","fontScale":"large"}"#).unwrap();
        assert_eq!((midnight.theme, midnight.header_mode, midnight.font_scale), (Theme::Midnight, HeaderMode::Hover, FontScale::Large));
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
        let s = Settings {
            view_mode: ViewMode::Tasks,
            hidden_calendars: vec!["cal-a".into()],
            hidden_task_lists: vec!["list-b".into()],
            ..Settings::default()
        };
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
