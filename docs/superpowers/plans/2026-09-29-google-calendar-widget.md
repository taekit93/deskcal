# Google Calendar & Tasks 데스크톱 위젯 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Google Calendar(월간 달력)와 Google Tasks(완료 체크)를 바탕화면에 고정된 가벼운 Windows 위젯으로 보여준다.

**Architecture:** Tauri 2 앱. Rust 백엔드가 OAuth(PKCE, loopback), 토큰 저장(Windows 자격 증명 관리자), Google API 호출, 캐시, 설정, 트레이, 바탕화면 고정(Win32)을 맡는다. TypeScript 프론트엔드는 `invoke()`로 데이터를 받아 화면만 그린다. 날짜 계산과 완료 처리 로직은 DOM 없는 순수 모듈로 분리해 Vitest로 테스트한다.

**Tech Stack:** Tauri 2, Rust (reqwest 0.12, tokio, keyring 3, chrono, windows 0.58, wiremock 0.6), TypeScript + Vite, Vitest.

**Spec:** `docs/superpowers/specs/2026-09-29-google-calendar-widget-design.md`

## Global Constraints

- 플랫폼: Windows 11 x64 전용. 비-Windows 코드 경로는 컴파일만 되면 된다 (`#[cfg(windows)]`).
- 프로젝트 루트: `J:\KHT\ProjectList_J\google-calendar-widget` (아래 모든 경로는 이 루트 기준).
- 앱 식별자: `kr.taekit93.gcalwidget`, 제품명 `gcal-widget`.
- 설정: `%APPDATA%\kr.taekit93.gcalwidget\settings.json`, 캐시: 같은 폴더 `cache.json` (Tauri `app_config_dir()`).
- 로그: `%LOCALAPPDATA%\kr.taekit93.gcalwidget\logs\` (tauri-plugin-log, 1MB, KeepOne).
- OAuth 스코프 (정확히 이 문자열): `https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/tasks`
- `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`은 `.env`(git 제외)에서 빌드 시 `build.rs`가 주입한다. 저장소에 커밋 금지.
- refresh token은 keyring(service `gcal-widget`, user `google-refresh-token`)에만 저장한다. access token은 메모리에만 둔다. 프론트엔드로 토큰을 보내지 않는다.
- 월간 격자: 일요일 시작, 항상 42칸(6주).
- 기본 설정: `viewMode: "both"`, `refreshMinutes: 10` (1~120으로 클램프), `autostart: true`, `locked: false`, 숨김 목록 비어 있음.
- 완료 취소 유예: 3000ms. API 재시도: 429/5xx에 최대 3회(지수 백오프, 기본 500ms). 401은 토큰을 무효화하고 1회 재시도.
- 모든 UI 문구는 한국어.
- 커밋 메시지 끝에 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. 각 Task 끝에서 `git push`.

## Review Focus

1. **자정에 끝나는 시간 일정 / 자정을 넘는 일정**: 23:00~24:00 일정은 그날에만, 22:00~다음 날 02:00 일정은 두 날에 표시되어야 한다. → Task 9 테스트
2. **`#`, `@`가 들어간 캘린더 ID** (예: 대한민국 공휴일 캘린더 `ko.south_korea#holiday@group.v.calendar.google.com`): URL 경로에 인코딩되지 않으면 404가 나거나 다른 캘린더를 조회한다. → Task 6 테스트
3. **제목 없는 일정·할 일** (비공개 일정, 빈 제목 할 일): 빈 줄 대신 `(제목 없음)`을 표시해야 한다. → Task 6, Task 7 테스트
4. **빠른 연속 클릭** (체크 직후 서버 응답 전 해제, 여러 할 일 동시 체크): 할 일마다 타이머가 독립적이어야 하고, 해제한 할 일이 목록에서 사라지면 안 된다. → Task 10 테스트
5. **캘린더 일부만 실패** (권한 없는 공유 캘린더 403): 나머지 캘린더는 표시하고, 실패한 ID만 `failed`에 담아야 한다. 네트워크 오류는 전체 실패로 처리해 캐시로 넘어가야 한다. → Task 8 테스트

---

## File Structure

```
google-calendar-widget/
├─ .env.example                 # 클라이언트 ID/시크릿 자리
├─ README.md                    # 설치·Google Cloud 설정 안내
├─ package.json / tsconfig.json / vite.config.ts / vitest.config.ts / index.html
├─ src/
│  ├─ main.ts                   # 상태·이벤트·타이머 연결 (조립만)
│  ├─ api.ts                    # invoke 래퍼 + 타입
│  ├─ errors.ts                 # 백엔드 오류 → 문구 (순수)
│  ├─ dom.ts                    # esc()
│  ├─ ViewSwitcher.ts
│  ├─ Settings.ts
│  ├─ styles.css
│  ├─ calendar/month-grid.ts    # 격자·일정 배치 (순수, 테스트)
│  ├─ calendar/MonthCalendar.ts
│  ├─ tasks/task-completion.ts  # 완료/취소/롤백 상태기계 (순수, 테스트)
│  └─ tasks/TaskList.ts
├─ src-tauri/
│  ├─ Cargo.toml / build.rs / tauri.conf.json / capabilities/default.json
│  └─ src/
│     ├─ main.rs / lib.rs        # 진입점, 플러그인·상태 조립
│     ├─ error.rs                # AppError (직렬화 가능)
│     ├─ settings.rs             # Settings 로드/저장
│     ├─ range.rs                # 월 → 격자 날짜 범위
│     ├─ cache.rs                # JSON 캐시 + 오프라인 폴백
│     ├─ auth.rs                 # OAuth PKCE, loopback, 토큰 갱신, keyring
│     ├─ google/mod.rs           # 공통 HTTP(재시도·401)
│     ├─ google/models.rs        # 프론트로 가는 모델
│     ├─ google/calendar.rs
│     ├─ google/tasks.rs
│     ├─ commands.rs             # AppState + Tauri 명령
│     ├─ tray.rs
│     └─ desktop.rs              # 바탕화면 고정 (Win32)
└─ docs/manual-test-checklist.md
```

---

### Task 1: 개발 환경 설치 (Rust + C++ Build Tools)

**Files:** 없음 (시스템 설치)

**Interfaces:**
- Consumes: 없음
- Produces: `cargo`, `rustc` (stable-x86_64-pc-windows-msvc), MSVC 링커

- [ ] **Step 1: Visual Studio 2022 Build Tools (C++ 워크로드) 설치**

UAC 창이 뜰 수 있다. 사용자에게 승인을 요청한다.

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --accept-package-agreements --accept-source-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```
Expected: `Successfully installed` (10~20분 걸릴 수 있음)

- [ ] **Step 2: rustup 설치**

```powershell
winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements
```
그다음 **새 터미널**에서 실행한다:
```powershell
rustup default stable-msvc
cargo --version; rustc --version
```
Expected: `cargo 1.8x.x`, `rustc 1.8x.x` 출력

- [ ] **Step 3: 링커 확인**

```powershell
cd $env:TEMP; cargo new hello-check --quiet; cd hello-check; cargo run --quiet; cd ..; Remove-Item -Recurse -Force hello-check
```
Expected: `Hello, world!`

---

### Task 2: 프로젝트 스캐폴드 (Tauri 2 + Vite + Vitest)

**Files:**
- Create: `package.json`, `vitest.config.ts`, `.env.example`, `README.md`
- Create (스캐폴드에서 복사): `index.html`, `tsconfig.json`, `vite.config.ts`, `src/*`, `src-tauri/*`
- Overwrite: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: 크레이트 이름 `gcal_widget_lib`, `pub fn run()`. 빌드 시 환경 변수 `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` (`option_env!`로 읽음).

- [ ] **Step 1: 임시 폴더에 스캐폴드 생성 후 복사**

기존 `.gitignore`와 `docs/`는 덮어쓰지 않는다(`cp -rn`).
```bash
SCRATCH=$(mktemp -d)
cd "$SCRATCH" && npm create tauri-app@latest gcal-scaffold -- --template vanilla-ts --manager npm --yes
cp -rn "$SCRATCH/gcal-scaffold/." "/j/KHT/ProjectList_J/google-calendar-widget/"
cd "/j/KHT/ProjectList_J/google-calendar-widget" && ls
```
Expected: `index.html package.json src src-tauri tsconfig.json vite.config.ts docs` 등이 보인다.

- [ ] **Step 2: `package.json` 덮어쓰기**

```json
{
  "name": "google-calendar-widget",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run"
  },
  "dependencies": {
    "@tauri-apps/api": "^2"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "typescript": "~5.6.2",
    "vite": "^6.0.3",
    "vitest": "^3.0.0"
  }
}
```

- [ ] **Step 3: `vitest.config.ts` 생성**

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    env: { TZ: "Asia/Seoul" },
  },
});
```

- [ ] **Step 4: `src-tauri/Cargo.toml` 덮어쓰기**

```toml
[package]
name = "gcal-widget"
version = "0.1.0"
edition = "2021"

[lib]
name = "gcal_widget_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-autostart = "2"
tauri-plugin-window-state = "2"
tauri-plugin-single-instance = "2"
tauri-plugin-log = "2"
log = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["net", "io-util", "time", "macros", "rt-multi-thread", "sync"] }
futures = "0.3"
chrono = { version = "0.4", features = ["serde"] }
keyring = { version = "3", features = ["windows-native"] }
sha2 = "0.10"
base64 = "0.22"
rand = "0.8"
url = "2"
urlencoding = "2"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
  "Win32_UI_Shell",
  "Win32_UI_Accessibility",
] }

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
```

- [ ] **Step 5: `src-tauri/build.rs` 덮어쓰기**

```rust
fn main() {
    println!("cargo:rerun-if-changed=../.env");
    if let Ok(text) = std::fs::read_to_string("../.env") {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                println!("cargo:rustc-env={}={}", key.trim(), value.trim());
            }
        }
    }
    tauri_build::build()
}
```

- [ ] **Step 6: `src-tauri/tauri.conf.json` 덮어쓰기**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "gcal-widget",
  "version": "0.1.0",
  "identifier": "kr.taekit93.gcalwidget",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "gcal-widget",
        "width": 420,
        "height": 640,
        "minWidth": 280,
        "minHeight": 240,
        "decorations": false,
        "transparent": true,
        "shadow": false,
        "skipTaskbar": true,
        "resizable": true,
        "visible": false
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 7: `src-tauri/capabilities/default.json` 덮어쓰기**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "main window permissions",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-set-resizable"
  ]
}
```

- [ ] **Step 8: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` 덮어쓰기**

`main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gcal_widget_lib::run()
}
```
`lib.rs` (이후 Task에서 확장):
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 9: `.env.example`, `README.md` 생성**

`.env.example`:
```
# Google Cloud Console > API 및 서비스 > 사용자 인증 정보 > OAuth 클라이언트 ID (데스크톱 앱)
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
```
`README.md`:
````markdown
# gcal-widget

Google Calendar(월간 달력)와 Google Tasks를 바탕화면에 고정해 보여주는 개인용 Windows 위젯.

## 준비

1. Rust (stable-msvc), Visual Studio 2022 Build Tools (C++), Node 20+
2. Google Cloud 설정
   1. https://console.cloud.google.com 에서 새 프로젝트를 만든다.
   2. "API 및 서비스 > 라이브러리"에서 **Google Calendar API**, **Google Tasks API**를 사용 설정한다.
   3. "OAuth 동의 화면":
      - Workspace 계정이면 User type **내부**를 고른다 (검증 불필요, 토큰 만료 없음).
      - 개인 Gmail이면 **외부**를 고르고 테스트 사용자에 본인을 추가한다. 이후 "앱 게시(프로덕션)"를 누른다. 테스트 상태로 두면 7일마다 다시 로그인해야 한다. 검증 신청은 하지 않아도 된다.
      - 스코프: `.../auth/calendar.readonly`, `.../auth/tasks`
   4. "사용자 인증 정보 > OAuth 클라이언트 ID > 애플리케이션 유형: 데스크톱 앱"을 만든다.
3. `.env.example`을 `.env`로 복사하고 클라이언트 ID/시크릿을 넣는다.

## 실행

```bash
npm install
npm run tauri dev      # 개발 실행
npm test               # 프론트 테스트
cd src-tauri && cargo test   # 백엔드 테스트
npm run tauri build    # 설치 파일: src-tauri/target/release/bundle/nsis/
```
````

- [ ] **Step 10: 설치 및 빌드 확인**

```bash
cd "/j/KHT/ProjectList_J/google-calendar-widget" && npm install && npx vitest run --passWithNoTests && cd src-tauri && cargo check
```
Expected: vitest `No test files found, exiting with code 0`, cargo `Finished`

- [ ] **Step 11: 커밋**

```bash
git add -A && git commit -m "chore: scaffold Tauri 2 + Vite project" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 3: AppError, Settings, 날짜 범위

**Files:**
- Create: `src-tauri/src/error.rs`, `src-tauri/src/settings.rs`, `src-tauri/src/range.rs`
- Modify: `src-tauri/src/lib.rs` (모듈 선언 추가)

**Interfaces:**
- Produces:
  - `error::AppError` enum: `NotLoggedIn`, `AuthExpired`, `Network(String)`, `Api { status: u16, message: String }`, `Config(String)`, `Login(String)`, `Storage(String)`. JSON은 `{"kind":"...","message":...}` 형태. `From<reqwest::Error>`.
  - `settings::{ViewMode, Settings, load(&Path) -> Settings, save(&Path, &Settings) -> Result<(), AppError>}`, `Settings::normalized(self) -> Settings`
  - `range::{grid_range(year: i32, month: u32) -> Option<(NaiveDate, NaiveDate)>, to_rfc3339_local(NaiveDate) -> String}` (끝 날짜는 exclusive)

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/error.rs`:
```rust
use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_variant_serializes_with_kind_only() {
        let v = serde_json::to_value(AppError::NotLoggedIn).unwrap();
        assert_eq!(v, serde_json::json!({ "kind": "NotLoggedIn" }));
    }

    #[test]
    fn api_variant_serializes_status_and_message() {
        let v = serde_json::to_value(AppError::Api { status: 404, message: "nf".into() }).unwrap();
        assert_eq!(v, serde_json::json!({ "kind": "Api", "message": { "status": 404, "message": "nf" } }));
    }
}
```
`src-tauri/src/settings.rs`:
```rust
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
```
`src-tauri/src/range.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn september_2026_starts_on_previous_sunday() {
        assert_eq!(grid_range(2026, 9), Some((d(2026, 8, 30), d(2026, 10, 11))));
    }

    #[test]
    fn month_starting_on_sunday_starts_same_day() {
        assert_eq!(grid_range(2026, 2), Some((d(2026, 2, 1), d(2026, 3, 15))));
    }

    #[test]
    fn invalid_month_is_none() {
        assert_eq!(grid_range(2026, 0), None);
        assert_eq!(grid_range(2026, 13), None);
    }

    #[test]
    fn rfc3339_is_local_midnight() {
        let s = to_rfc3339_local(d(2026, 9, 1));
        assert!(s.starts_with("2026-09-01T00:00:00"), "{s}");
    }
}
```
`lib.rs` 맨 위에 추가:
```rust
mod error;
mod range;
mod settings;
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib`
Expected: 컴파일 실패 (`cannot find type AppError`, `load`, `grid_range` 등)

- [ ] **Step 3: 구현**

`error.rs` 상단(테스트 위)에 추가:
```rust
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("로그인이 필요합니다")]
    NotLoggedIn,
    #[error("로그인이 만료되었습니다")]
    AuthExpired,
    #[error("네트워크 오류: {0}")]
    Network(String),
    #[error("Google API 오류 ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("설정 오류: {0}")]
    Config(String),
    #[error("로그인 실패: {0}")]
    Login(String),
    #[error("저장소 오류: {0}")]
    Storage(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() || e.is_request() {
            AppError::Network(e.to_string())
        } else {
            AppError::Api {
                status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                message: e.to_string(),
            }
        }
    }
}
```
`settings.rs` 상단:
```rust
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
```
`range.rs` 상단:
```rust
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};

/// 월간 격자(일요일 시작, 42칸)의 [시작일, 끝일) 범위.
pub fn grid_range(year: i32, month: u32) -> Option<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let start = first - Duration::days(first.weekday().num_days_from_sunday() as i64);
    Some((start, start + Duration::days(42)))
}

/// 로컬 시간대 자정을 RFC3339 문자열로.
pub fn to_rfc3339_local(date: NaiveDate) -> String {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .unwrap_or_else(|| Local.from_utc_datetime(&naive))
        .to_rfc3339()
}
```

- [ ] **Step 4: 통과 확인**

Run: `cd src-tauri && cargo test --lib`
Expected: `test result: ok. 12 passed`

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "feat: add AppError, settings persistence and month grid range" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 4: 캐시와 오프라인 폴백

**Files:**
- Create: `src-tauri/src/cache.rs`
- Modify: `src-tauri/src/lib.rs` (`mod cache;`)

**Interfaces:**
- Consumes: `AppError` (Task 3)
- Produces:
  - `cache::Cache::new(path: PathBuf) -> Cache`
  - `Cache::get<T: DeserializeOwned>(&self, key: &str) -> Option<T>`
  - `Cache::put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), AppError>`
  - `cache::with_fallback<T: Serialize + DeserializeOwned>(cache: &Cache, key: &str, result: Result<T, AppError>, mark_stale: impl FnOnce(&mut T)) -> Result<T, AppError>`: 성공하면 캐시에 저장하고 반환한다. `Network` 오류면 캐시 값에 `mark_stale`을 적용해 반환한다. 그 밖의 오류는 그대로 반환한다.

- [ ] **Step 1: 실패하는 테스트 작성** (`cache.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Snap {
        items: Vec<String>,
        stale: bool,
    }

    fn snap(items: &[&str]) -> Snap {
        Snap { items: items.iter().map(|s| s.to_string()).collect(), stale: false }
    }

    fn cache() -> (tempfile::TempDir, Cache) {
        let dir = tempfile::tempdir().unwrap();
        let c = Cache::new(dir.path().join("sub").join("cache.json"));
        (dir, c)
    }

    #[test]
    fn missing_key_is_none() {
        let (_d, c) = cache();
        assert_eq!(c.get::<Snap>("month:2026-09"), None);
    }

    #[test]
    fn put_then_get_roundtrip_and_keys_are_independent() {
        let (_d, c) = cache();
        c.put("a", &snap(&["1"])).unwrap();
        c.put("b", &snap(&["2"])).unwrap();
        assert_eq!(c.get::<Snap>("a"), Some(snap(&["1"])));
        assert_eq!(c.get::<Snap>("b"), Some(snap(&["2"])));
    }

    #[test]
    fn corrupt_file_reads_none_and_put_recovers() {
        let (_d, c) = cache();
        std::fs::create_dir_all(c.path.parent().unwrap()).unwrap();
        std::fs::write(&c.path, "garbage").unwrap();
        assert_eq!(c.get::<Snap>("a"), None);
        c.put("a", &snap(&["x"])).unwrap();
        assert_eq!(c.get::<Snap>("a"), Some(snap(&["x"])));
    }

    #[test]
    fn fallback_success_is_cached() {
        let (_d, c) = cache();
        let out = with_fallback(&c, "k", Ok(snap(&["new"])), |s| s.stale = true).unwrap();
        assert_eq!(out, snap(&["new"]));
        assert_eq!(c.get::<Snap>("k"), Some(snap(&["new"])));
    }

    #[test]
    fn fallback_network_error_returns_stale_cache() {
        let (_d, c) = cache();
        c.put("k", &snap(&["old"])).unwrap();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::Network("down".into())), |s| s.stale = true).unwrap();
        assert_eq!(out.items, vec!["old".to_string()]);
        assert!(out.stale);
    }

    #[test]
    fn fallback_network_error_without_cache_is_error() {
        let (_d, c) = cache();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::Network("down".into())), |s| s.stale = true);
        assert!(matches!(out, Err(AppError::Network(_))));
    }

    #[test]
    fn fallback_auth_error_is_not_masked_by_cache() {
        let (_d, c) = cache();
        c.put("k", &snap(&["old"])).unwrap();
        let out = with_fallback::<Snap>(&c, "k", Err(AppError::AuthExpired), |s| s.stale = true);
        assert!(matches!(out, Err(AppError::AuthExpired)));
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib cache`
Expected: 컴파일 실패 (`cannot find struct Cache`)

- [ ] **Step 3: 구현** (`cache.rs` 상단)

```rust
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::error::AppError;

pub struct Cache {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Cache {
    pub fn new(path: PathBuf) -> Self {
        Self { path, lock: Mutex::new(()) }
    }

    fn read_map(&self) -> HashMap<String, Value> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let _guard = self.lock.lock().unwrap();
        self.read_map().remove(key).and_then(|v| serde_json::from_value(v).ok())
    }

    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), AppError> {
        let _guard = self.lock.lock().unwrap();
        let mut map = self.read_map();
        let value = serde_json::to_value(value).map_err(|e| AppError::Storage(e.to_string()))?;
        map.insert(key.to_string(), value);
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| AppError::Storage(e.to_string()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string(&map).map_err(|e| AppError::Storage(e.to_string()))?;
        fs::write(&tmp, text).map_err(|e| AppError::Storage(e.to_string()))?;
        fs::rename(&tmp, &self.path).map_err(|e| AppError::Storage(e.to_string()))
    }
}

pub fn with_fallback<T: Serialize + DeserializeOwned>(
    cache: &Cache,
    key: &str,
    result: Result<T, AppError>,
    mark_stale: impl FnOnce(&mut T),
) -> Result<T, AppError> {
    match result {
        Ok(value) => {
            if let Err(e) = cache.put(key, &value) {
                log::warn!("cache write failed: {e}");
            }
            Ok(value)
        }
        Err(AppError::Network(msg)) => match cache.get::<T>(key) {
            Some(mut value) => {
                mark_stale(&mut value);
                Ok(value)
            }
            None => Err(AppError::Network(msg)),
        },
        Err(e) => Err(e),
    }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cd src-tauri && cargo test --lib cache`
Expected: `7 passed`

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "feat: add JSON cache with offline fallback" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 5: OAuth 인증 (PKCE, loopback, 토큰 갱신, keyring)

**Files:**
- Create: `src-tauri/src/auth.rs`
- Modify: `src-tauri/src/lib.rs` (`mod auth;`)

**Interfaces:**
- Consumes: `AppError`
- Produces:
  - `auth::SCOPES: &str`
  - `auth::OAuthConfig { client_id, client_secret, auth_url, token_url }` (모두 `String`), `OAuthConfig::from_build_env()`
  - `auth::TokenStore` trait (`get/set/delete`), `auth::KeyringStore::new() -> Result<KeyringStore, AppError>`, 테스트 전용 `auth::MemoryStore::{with(&str), empty()}`
  - `auth::Auth::new(cfg: OAuthConfig, store: Box<dyn TokenStore>, http: reqwest::Client) -> Auth`
  - `Auth::is_logged_in(&self) -> bool`, `Auth::invalidate(&self)`, `Auth::logout(&self) -> Result<(), AppError>`
  - `Auth::access_token(&self) -> impl Future<Output = Result<String, AppError>>`
  - `Auth::login<F: FnOnce(&str) -> Result<(), AppError>>(&self, open_browser: F) -> Result<(), AppError>`

- [ ] **Step 1: 실패하는 테스트 작성** (`auth.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn cfg_for(uri: &str) -> OAuthConfig {
        OAuthConfig {
            client_id: "cid".into(),
            client_secret: "csecret".into(),
            auth_url: format!("{uri}/auth"),
            token_url: format!("{uri}/token"),
        }
    }

    fn query(url: &str) -> HashMap<String, String> {
        url::Url::parse(url).unwrap().query_pairs().into_owned().collect()
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_example() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn pkce_pair_is_consistent() {
        let (verifier, challenge) = pkce_pair();
        assert!(verifier.len() >= 43);
        assert_eq!(challenge_for(&verifier), challenge);
    }

    #[test]
    fn auth_url_has_required_params() {
        let url = build_auth_url(&cfg_for("https://x"), "http://127.0.0.1:5000", "chal", "st");
        let q = query(&url);
        assert_eq!(q["client_id"], "cid");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["scope"], SCOPES);
        assert_eq!(q["code_challenge"], "chal");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["state"], "st");
        assert_eq!(q["access_type"], "offline");
        assert_eq!(q["prompt"], "consent");
    }

    #[test]
    fn callback_with_code_and_matching_state() {
        let r = parse_callback("GET /?state=abc&code=4%2F0Ab HTTP/1.1", "abc");
        assert!(matches!(r, CallbackResult::Done(Ok(ref c)) if c == "4/0Ab"), "{r:?}");
    }

    #[test]
    fn callback_with_wrong_state_fails() {
        let r = parse_callback("GET /?state=evil&code=x HTTP/1.1", "abc");
        assert!(matches!(r, CallbackResult::Done(Err(AppError::Login(_)))), "{r:?}");
    }

    #[test]
    fn callback_access_denied_fails() {
        let r = parse_callback("GET /?error=access_denied&state=abc HTTP/1.1", "abc");
        assert!(matches!(r, CallbackResult::Done(Err(AppError::Login(ref m))) if m == "access_denied"), "{r:?}");
    }

    #[test]
    fn favicon_request_is_ignored() {
        assert!(matches!(parse_callback("GET /favicon.ico HTTP/1.1", "abc"), CallbackResult::Ignore));
    }

    #[tokio::test]
    async fn access_token_is_cached_until_expiry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at1", "expires_in": 3600})))
            .expect(1)
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::with("r1")), reqwest::Client::new());
        assert_eq!(auth.access_token().await.unwrap(), "at1");
        assert_eq!(auth.access_token().await.unwrap(), "at1");
    }

    #[tokio::test]
    async fn invalidate_forces_refresh() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at", "expires_in": 3600})))
            .expect(2)
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::with("r1")), reqwest::Client::new());
        auth.access_token().await.unwrap();
        auth.invalidate();
        auth.access_token().await.unwrap();
    }

    #[tokio::test]
    async fn not_logged_in_without_refresh_token() {
        let auth = Auth::new(cfg_for("http://127.0.0.1:1"), Box::new(MemoryStore::empty()), reqwest::Client::new());
        assert!(!auth.is_logged_in());
        assert!(matches!(auth.access_token().await, Err(AppError::NotLoggedIn)));
    }

    #[tokio::test]
    async fn invalid_grant_clears_store_and_reports_expired() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_grant"})))
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::with("r1")), reqwest::Client::new());
        assert!(matches!(auth.access_token().await, Err(AppError::AuthExpired)));
        assert!(!auth.is_logged_in());
    }

    #[tokio::test]
    async fn login_exchanges_code_via_loopback_and_stores_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=abc"))
            .and(body_string_contains("code_verifier="))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"access_token": "at", "expires_in": 3600, "refresh_token": "r2"}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let auth = Auth::new(cfg_for(&server.uri()), Box::new(MemoryStore::empty()), reqwest::Client::new());
        auth.login(|url| {
            let q = query(url);
            let target = format!("{}/?code=abc&state={}", q["redirect_uri"], q["state"]);
            tokio::spawn(async move {
                let _ = reqwest::get(target).await;
            });
            Ok(())
        })
        .await
        .unwrap();
        assert!(auth.is_logged_in());
        assert_eq!(auth.access_token().await.unwrap(), "at");
    }

    #[tokio::test]
    async fn login_without_client_id_is_config_error() {
        let mut cfg = cfg_for("http://127.0.0.1:1");
        cfg.client_id.clear();
        let auth = Auth::new(cfg, Box::new(MemoryStore::empty()), reqwest::Client::new());
        assert!(matches!(auth.login(|_| Ok(())).await, Err(AppError::Config(_))));
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib auth`
Expected: 컴파일 실패 (`cannot find struct Auth`)

- [ ] **Step 3: 구현** (`auth.rs` 상단)

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::error::AppError;

pub const SCOPES: &str =
    "https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/tasks";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
}

impl OAuthConfig {
    pub fn from_build_env() -> Self {
        Self {
            client_id: option_env!("GOOGLE_CLIENT_ID").unwrap_or("").to_string(),
            client_secret: option_env!("GOOGLE_CLIENT_SECRET").unwrap_or("").to_string(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
        }
    }
}

pub trait TokenStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, AppError>;
    fn set(&self, token: &str) -> Result<(), AppError>;
    fn delete(&self) -> Result<(), AppError>;
}

pub struct KeyringStore {
    entry: keyring::Entry,
}

impl KeyringStore {
    pub fn new() -> Result<Self, AppError> {
        keyring::Entry::new("gcal-widget", "google-refresh-token")
            .map(|entry| Self { entry })
            .map_err(|e| AppError::Storage(e.to_string()))
    }
}

impl TokenStore for KeyringStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        match self.entry.get_password() {
            Ok(t) => Ok(Some(t)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::Storage(e.to_string())),
        }
    }
    fn set(&self, token: &str) -> Result<(), AppError> {
        self.entry.set_password(token).map_err(|e| AppError::Storage(e.to_string()))
    }
    fn delete(&self) -> Result<(), AppError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Storage(e.to_string())),
        }
    }
}

#[cfg(test)]
pub struct MemoryStore(Mutex<Option<String>>);

#[cfg(test)]
impl MemoryStore {
    pub fn with(token: &str) -> Self {
        Self(Mutex::new(Some(token.to_string())))
    }
    pub fn empty() -> Self {
        Self(Mutex::new(None))
    }
}

#[cfg(test)]
impl TokenStore for MemoryStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn set(&self, token: &str) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(token.to_string());
        Ok(())
    }
    fn delete(&self) -> Result<(), AppError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
}

pub struct Auth {
    cfg: OAuthConfig,
    store: Box<dyn TokenStore>,
    http: reqwest::Client,
    cached: Mutex<Option<(String, Instant)>>,
}

impl Auth {
    pub fn new(cfg: OAuthConfig, store: Box<dyn TokenStore>, http: reqwest::Client) -> Self {
        Self { cfg, store, http, cached: Mutex::new(None) }
    }

    pub fn is_logged_in(&self) -> bool {
        matches!(self.store.get(), Ok(Some(_)))
    }

    pub fn invalidate(&self) {
        *self.cached.lock().unwrap() = None;
    }

    pub fn logout(&self) -> Result<(), AppError> {
        self.invalidate();
        self.store.delete()
    }

    pub async fn access_token(&self) -> Result<String, AppError> {
        let cached = self.cached.lock().unwrap().clone();
        if let Some((token, expires_at)) = cached {
            if Instant::now() < expires_at {
                return Ok(token);
            }
        }
        let refresh = self.store.get()?.ok_or(AppError::NotLoggedIn)?;
        let params = [
            ("client_id", self.cfg.client_id.as_str()),
            ("client_secret", self.cfg.client_secret.as_str()),
            ("refresh_token", refresh.as_str()),
            ("grant_type", "refresh_token"),
        ];
        Ok(self.request_token(&params).await?.access_token)
    }

    pub async fn login<F>(&self, open_browser: F) -> Result<(), AppError>
    where
        F: FnOnce(&str) -> Result<(), AppError>,
    {
        if self.cfg.client_id.is_empty() {
            return Err(AppError::Config("GOOGLE_CLIENT_ID가 설정되지 않았습니다 (.env 확인)".into()));
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| AppError::Login(e.to_string()))?;
        let port = listener.local_addr().map_err(|e| AppError::Login(e.to_string()))?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}");
        let (verifier, challenge) = pkce_pair();
        let state = random_token(16);

        open_browser(&build_auth_url(&self.cfg, &redirect_uri, &challenge, &state))?;

        let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_code(&listener, &state))
            .await
            .map_err(|_| AppError::Login("시간 초과 (5분)".into()))??;

        let params = [
            ("client_id", self.cfg.client_id.as_str()),
            ("client_secret", self.cfg.client_secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ];
        let token = self.request_token(&params).await?;
        if token.refresh_token.is_none() {
            return Err(AppError::Login("refresh token을 받지 못했습니다".into()));
        }
        Ok(())
    }

    async fn request_token(&self, params: &[(&str, &str)]) -> Result<TokenResponse, AppError> {
        let resp = self.http.post(&self.cfg.token_url).form(params).send().await?;
        let status = resp.status();
        if status.is_success() {
            let token: TokenResponse = resp.json().await?;
            if let Some(refresh) = &token.refresh_token {
                self.store.set(refresh)?;
            }
            let expires_at = Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60));
            *self.cached.lock().unwrap() = Some((token.access_token.clone(), expires_at));
            return Ok(token);
        }
        let body = resp.text().await.unwrap_or_default();
        let code = serde_json::from_str::<TokenError>(&body).map(|e| e.error).unwrap_or_default();
        if code == "invalid_grant" {
            self.invalidate();
            self.store.delete()?;
            return Err(AppError::AuthExpired);
        }
        Err(AppError::Api { status: status.as_u16(), message: body })
    }
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub(crate) fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub(crate) fn pkce_pair() -> (String, String) {
    let verifier = random_token(32);
    let challenge = challenge_for(&verifier);
    (verifier, challenge)
}

pub(crate) fn build_auth_url(cfg: &OAuthConfig, redirect_uri: &str, challenge: &str, state: &str) -> String {
    url::Url::parse_with_params(
        &cfg.auth_url,
        &[
            ("client_id", cfg.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
    .expect("auth_url is a valid URL")
    .to_string()
}

#[derive(Debug)]
pub(crate) enum CallbackResult {
    Ignore,
    Done(Result<String, AppError>),
}

pub(crate) fn parse_callback(request_line: &str, expected_state: &str) -> CallbackResult {
    let mut parts = request_line.split_whitespace();
    if parts.next() != Some("GET") {
        return CallbackResult::Ignore;
    }
    let target = parts.next().unwrap_or("");
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{target}")) else {
        return CallbackResult::Ignore;
    };
    if url.path() != "/" {
        return CallbackResult::Ignore;
    }
    let q: HashMap<String, String> = url.query_pairs().into_owned().collect();
    if let Some(err) = q.get("error") {
        return CallbackResult::Done(Err(AppError::Login(err.clone())));
    }
    if q.get("state").map(String::as_str) != Some(expected_state) {
        return CallbackResult::Done(Err(AppError::Login("state 불일치".into())));
    }
    match q.get("code") {
        Some(code) => CallbackResult::Done(Ok(code.clone())),
        None => CallbackResult::Ignore,
    }
}

async fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String, AppError> {
    loop {
        let (mut sock, _) = listener.accept().await.map_err(|e| AppError::Login(e.to_string()))?;
        let mut buf = vec![0u8; 4096];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let line = request.lines().next().unwrap_or("");
        match parse_callback(line, state) {
            CallbackResult::Ignore => {
                let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            }
            CallbackResult::Done(result) => {
                let msg = if result.is_ok() {
                    "로그인 완료. 이 창을 닫아도 됩니다."
                } else {
                    "로그인 실패. 위젯에서 다시 시도해 주세요."
                };
                let body = format!("<!doctype html><meta charset=\"utf-8\"><title>gcal-widget</title><p style=\"font:16px sans-serif\">{msg}</p>");
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                return result;
            }
        }
    }
}
```

- [ ] **Step 4: 통과 확인**

Run: `cd src-tauri && cargo test --lib auth`
Expected: `13 passed`

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "feat: add OAuth PKCE login, token refresh and keyring storage" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 6: Google API 공통 클라이언트 + Calendar

**Files:**
- Create: `src-tauri/src/google/mod.rs`, `src-tauri/src/google/models.rs`, `src-tauri/src/google/calendar.rs`
- Modify: `src-tauri/src/lib.rs` (`mod google;`)

**Interfaces:**
- Consumes: `Auth::access_token`, `Auth::invalidate`, `AppError`, 테스트에서 `MemoryStore`, `OAuthConfig`
- Produces:
  - `google::Google::new(http: reqwest::Client, auth: Arc<Auth>) -> Google`
  - 테스트 전용 `google::test_support::{setup() -> (MockServer, Google), mount_token(&MockServer)}`
  - `Google::list_calendars(&self) -> Result<Vec<Calendar>, AppError>`
  - `Google::list_events(&self, calendar_id: &str, time_min: &str, time_max: &str) -> Result<Vec<Event>, AppError>`
  - `google::enc(&str) -> String` (경로 세그먼트 인코딩)
  - `models::{Calendar{id, summary, color, primary}, Event{id, calendar_id, title, all_day, start, end, html_link}, TaskList{id, title}, Task{id, list_id, title, due, notes}, MonthData{calendars, events, failed, fetched_at, stale}, TasksData{lists, tasks, failed, fetched_at, stale}}`. 모두 `Serialize + Deserialize + Clone + Debug + PartialEq`, camelCase. `Event.start/end`: 종일이면 `YYYY-MM-DD`(end exclusive), 아니면 RFC3339.

- [ ] **Step 1: 모델과 테스트 지원 코드 작성**

`google/models.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: String,
    pub summary: String,
    pub color: String,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub all_day: bool,
    pub start: String,
    pub end: String,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskList {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub due: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthData {
    pub calendars: Vec<Calendar>,
    pub events: Vec<Event>,
    pub failed: Vec<String>,
    pub fetched_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasksData {
    pub lists: Vec<TaskList>,
    pub tasks: Vec<Task>,
    pub failed: Vec<String>,
    pub fetched_at: String,
    pub stale: bool,
}

pub const UNTITLED: &str = "(제목 없음)";
```
`google/mod.rs` (구현은 Step 3에서 채움, 먼저 테스트 지원만):
```rust
pub mod calendar;
pub mod models;

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::Google;
    use crate::auth::{Auth, MemoryStore, OAuthConfig};

    pub async fn mount_token(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at", "expires_in": 3600})))
            .mount(server)
            .await;
    }

    /// 토큰 mock은 붙이지 않는다. 호출하는 테스트가 mount_token 또는 직접 mock을 붙인다.
    pub async fn setup() -> (MockServer, Google) {
        let server = MockServer::start().await;
        let cfg = OAuthConfig {
            client_id: "cid".into(),
            client_secret: "cs".into(),
            auth_url: format!("{}/auth", server.uri()),
            token_url: format!("{}/token", server.uri()),
        };
        let auth = Arc::new(Auth::new(cfg, Box::new(MemoryStore::with("r1")), reqwest::Client::new()));
        let google = Google::with_base(reqwest::Client::new(), auth, &server.uri());
        (server, google)
    }
}
```

- [ ] **Step 2: 실패하는 테스트 작성** (`google/calendar.rs`)

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use crate::error::AppError;
    use crate::google::models::Calendar;
    use crate::google::test_support::{mount_token, setup};

    #[tokio::test]
    async fn list_calendars_maps_fields() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .and(header("authorization", "Bearer at"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "me@x.com", "summary": "나", "backgroundColor": "#9fe1e7", "primary": true},
                    {"id": "c2", "summary": "원래 이름", "summaryOverride": "별칭"}
                ]
            })))
            .mount(&server)
            .await;
        let cals = g.list_calendars().await.unwrap();
        assert_eq!(
            cals,
            vec![
                Calendar { id: "me@x.com".into(), summary: "나".into(), color: "#9fe1e7".into(), primary: true },
                Calendar { id: "c2".into(), summary: "별칭".into(), color: "#4285f4".into(), primary: false },
            ]
        );
    }

    #[tokio::test]
    async fn list_events_follows_pagination_and_skips_cancelled() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .and(query_param("pageToken", "p2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "e3", "summary": "회의",
                           "start": {"dateTime": "2026-09-15T10:00:00+09:00"},
                           "end": {"dateTime": "2026-09-15T11:00:00+09:00"}}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "e1", "summary": "휴가", "start": {"date": "2026-09-10"}, "end": {"date": "2026-09-12"},
                     "htmlLink": "https://calendar.google.com/e1"},
                    {"id": "e2", "status": "cancelled"}
                ],
                "nextPageToken": "p2"
            })))
            .mount(&server)
            .await;
        let events = g.list_events("c1", "a", "b").await.unwrap();
        let ids: Vec<_> = events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e1", "e3"]);
        assert!(events[0].all_day);
        assert_eq!(events[0].start, "2026-09-10");
        assert_eq!(events[0].end, "2026-09-12");
        assert_eq!(events[0].calendar_id, "c1");
        assert_eq!(events[0].html_link.as_deref(), Some("https://calendar.google.com/e1"));
        assert!(!events[1].all_day);
        assert_eq!(events[1].start, "2026-09-15T10:00:00+09:00");
    }

    #[tokio::test]
    async fn list_events_encodes_calendar_id_and_sends_range() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/ko.south_korea%23holiday%40group.v.calendar.google.com/events"))
            .and(query_param("timeMin", "2026-08-30T00:00:00+09:00"))
            .and(query_param("timeMax", "2026-10-11T00:00:00+09:00"))
            .and(query_param("singleEvents", "true"))
            .and(query_param("orderBy", "startTime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
            .expect(1)
            .mount(&server)
            .await;
        let events = g
            .list_events(
                "ko.south_korea#holiday@group.v.calendar.google.com",
                "2026-08-30T00:00:00+09:00",
                "2026-10-11T00:00:00+09:00",
            )
            .await
            .unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn event_without_title_gets_placeholder() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "e1", "start": {"date": "2026-09-10"}, "end": {"date": "2026-09-11"}},
                          {"id": "e2", "summary": "   ", "start": {"date": "2026-09-10"}, "end": {"date": "2026-09-11"}}]
            })))
            .mount(&server)
            .await;
        let events = g.list_events("c1", "a", "b").await.unwrap();
        assert_eq!(events[0].title, "(제목 없음)");
        assert_eq!(events[1].title, "(제목 없음)");
    }

    #[tokio::test]
    async fn retries_on_503_then_succeeds() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
            .mount(&server)
            .await;
        assert!(g.list_calendars().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn gives_up_after_three_retries() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(503))
            .expect(4)
            .mount(&server)
            .await;
        assert!(matches!(g.list_calendars().await, Err(AppError::Api { status: 503, .. })));
    }

    #[tokio::test]
    async fn refreshes_token_once_on_401() {
        let (server, g) = setup().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "at", "expires_in": 3600})))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(401))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
            .mount(&server)
            .await;
        assert!(g.list_calendars().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn not_found_is_api_error() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/calendar/v3/calendars/c1/events"))
            .respond_with(ResponseTemplate::new(404).set_body_string("nope"))
            .mount(&server)
            .await;
        assert!(matches!(g.list_events("c1", "a", "b").await, Err(AppError::Api { status: 404, .. })));
    }
}
```

- [ ] **Step 3: 실패 확인**

Run: `cd src-tauri && cargo test --lib google`
Expected: 컴파일 실패 (`cannot find struct Google`)

- [ ] **Step 4: 구현**

`google/mod.rs`에서 `pub mod calendar; pub mod models;` 아래, `test_support` 위에 추가:
```rust
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;

use crate::auth::Auth;
use crate::error::AppError;

const MAX_RETRIES: u32 = 3;

pub struct Google {
    http: reqwest::Client,
    auth: Arc<Auth>,
    calendar_base: String,
    tasks_base: String,
    backoff: Duration,
}

impl Google {
    pub fn new(http: reqwest::Client, auth: Arc<Auth>) -> Self {
        Self {
            http,
            auth,
            calendar_base: "https://www.googleapis.com/calendar/v3".into(),
            tasks_base: "https://tasks.googleapis.com/tasks/v1".into(),
            backoff: Duration::from_millis(500),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_base(http: reqwest::Client, auth: Arc<Auth>, base: &str) -> Self {
        Self {
            http,
            auth,
            calendar_base: format!("{base}/calendar/v3"),
            tasks_base: format!("{base}/tasks/v1"),
            backoff: Duration::from_millis(1),
        }
    }

    pub(crate) async fn send_json<T: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        query: &[(&str, String)],
        body: Option<&serde_json::Value>,
    ) -> Result<T, AppError> {
        let mut auth_retried = false;
        let mut attempt = 0u32;
        loop {
            let token = self.auth.access_token().await?;
            let mut req = self.http.request(method.clone(), url).bearer_auth(&token).query(query);
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await?;
            let status = resp.status();
            if status.is_success() {
                return Ok(resp.json::<T>().await?);
            }
            if status == StatusCode::UNAUTHORIZED && !auth_retried {
                auth_retried = true;
                self.auth.invalidate();
                continue;
            }
            if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()) && attempt < MAX_RETRIES {
                tokio::time::sleep(self.backoff * 2u32.pow(attempt)).await;
                attempt += 1;
                continue;
            }
            let message = resp.text().await.unwrap_or_default();
            return Err(AppError::Api { status: status.as_u16(), message });
        }
    }
}

pub(crate) fn enc(segment: &str) -> String {
    urlencoding::encode(segment).into_owned()
}
```
`google/calendar.rs` 상단:
```rust
use reqwest::Method;
use serde::Deserialize;

use super::models::{Calendar, Event, UNTITLED};
use super::{enc, Google};
use crate::error::AppError;

const DEFAULT_COLOR: &str = "#4285f4";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarListPage {
    #[serde(default)]
    items: Vec<RawCalendar>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCalendar {
    id: String,
    summary: Option<String>,
    summary_override: Option<String>,
    background_color: Option<String>,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventsPage {
    #[serde(default)]
    items: Vec<RawEvent>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvent {
    id: String,
    summary: Option<String>,
    status: Option<String>,
    start: Option<RawTime>,
    end: Option<RawTime>,
    html_link: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTime {
    date: Option<String>,
    date_time: Option<String>,
}

impl Google {
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>, AppError> {
        let url = format!("{}/users/me/calendarList", self.calendar_base);
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![("maxResults", "250".to_string())];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: CalendarListPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(page.items.into_iter().map(|c| Calendar {
                id: c.id,
                summary: c.summary_override.or(c.summary).unwrap_or_default(),
                color: c.background_color.unwrap_or_else(|| DEFAULT_COLOR.into()),
                primary: c.primary,
            }));
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn list_events(&self, calendar_id: &str, time_min: &str, time_max: &str) -> Result<Vec<Event>, AppError> {
        let url = format!("{}/calendars/{}/events", self.calendar_base, enc(calendar_id));
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![
                ("timeMin", time_min.to_string()),
                ("timeMax", time_max.to_string()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
                ("maxResults", "2500".to_string()),
            ];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: EventsPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(page.items.into_iter().filter_map(|e| convert_event(calendar_id, e)));
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }
}

fn convert_event(calendar_id: &str, e: RawEvent) -> Option<Event> {
    if e.status.as_deref() == Some("cancelled") {
        return None;
    }
    let (start, end) = (e.start?, e.end?);
    let (all_day, start, end) = match (start.date, start.date_time, end.date, end.date_time) {
        (Some(s), _, Some(en), _) => (true, s, en),
        (_, Some(s), _, Some(en)) => (false, s, en),
        _ => return None,
    };
    Some(Event {
        id: e.id,
        calendar_id: calendar_id.to_string(),
        title: e.summary.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| UNTITLED.into()),
        all_day,
        start,
        end,
        html_link: e.html_link,
    })
}
```

- [ ] **Step 5: 통과 확인**

Run: `cd src-tauri && cargo test --lib google`
Expected: `8 passed`

- [ ] **Step 6: 커밋**

```bash
git add -A && git commit -m "feat: add Google API client with retry and Calendar endpoints" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 7: Google Tasks

**Files:**
- Create: `src-tauri/src/google/tasks.rs`
- Modify: `src-tauri/src/google/mod.rs` (`pub mod tasks;` 추가)

**Interfaces:**
- Consumes: `Google::send_json`, `enc`, `models::{Task, TaskList, UNTITLED}`, `test_support`
- Produces:
  - `Google::list_tasklists(&self) -> Result<Vec<TaskList>, AppError>`
  - `Google::list_tasks(&self, list_id: &str) -> Result<Vec<Task>, AppError>` (미완료만, `due`는 `YYYY-MM-DD`)
  - `Google::set_task_completed(&self, list_id: &str, task_id: &str, completed: bool) -> Result<(), AppError>`

- [ ] **Step 1: 실패하는 테스트 작성** (`google/tasks.rs`)

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use crate::google::models::{Task, TaskList};
    use crate::google::test_support::{mount_token, setup};

    #[tokio::test]
    async fn list_tasklists_maps_titles() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/users/@me/lists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "L1", "title": "내 할 일"}, {"id": "L2"}]
            })))
            .mount(&server)
            .await;
        assert_eq!(
            g.list_tasklists().await.unwrap(),
            vec![
                TaskList { id: "L1".into(), title: "내 할 일".into() },
                TaskList { id: "L2".into(), title: "(제목 없음)".into() },
            ]
        );
    }

    #[tokio::test]
    async fn list_tasks_requests_open_tasks_trims_due_and_handles_empty_title() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/lists/L1/tasks"))
            .and(query_param("showCompleted", "false"))
            .and(query_param("showHidden", "false"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "t1", "title": "보고서", "due": "2026-09-30T00:00:00.000Z", "status": "needsAction", "notes": "초안"},
                    {"id": "t2", "title": "", "status": "needsAction"},
                    {"id": "t3", "title": "끝남", "status": "completed"}
                ]
            })))
            .mount(&server)
            .await;
        assert_eq!(
            g.list_tasks("L1").await.unwrap(),
            vec![
                Task { id: "t1".into(), list_id: "L1".into(), title: "보고서".into(), due: Some("2026-09-30".into()), notes: Some("초안".into()) },
                Task { id: "t2".into(), list_id: "L1".into(), title: "(제목 없음)".into(), due: None, notes: None },
            ]
        );
    }

    #[tokio::test]
    async fn list_tasks_follows_pagination() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/lists/L1/tasks"))
            .and(query_param("pageToken", "n2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": [{"id": "b", "title": "B"}]})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/tasks/v1/lists/L1/tasks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": [{"id": "a", "title": "A"}], "nextPageToken": "n2"})))
            .mount(&server)
            .await;
        let ids: Vec<_> = g.list_tasks("L1").await.unwrap().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn set_completed_patches_status() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("PATCH"))
            .and(path("/tasks/v1/lists/L1/tasks/T1"))
            .and(body_json(json!({"status": "completed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "T1"})))
            .expect(1)
            .mount(&server)
            .await;
        g.set_task_completed("L1", "T1", true).await.unwrap();
    }

    #[tokio::test]
    async fn set_uncompleted_clears_completed_date() {
        let (server, g) = setup().await;
        mount_token(&server).await;
        Mock::given(method("PATCH"))
            .and(path("/tasks/v1/lists/L1/tasks/T1"))
            .and(body_json(json!({"status": "needsAction", "completed": null})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "T1"})))
            .expect(1)
            .mount(&server)
            .await;
        g.set_task_completed("L1", "T1", false).await.unwrap();
    }
}
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib google::tasks`
Expected: 컴파일 실패 (`no method named list_tasklists`)

- [ ] **Step 3: 구현** (`google/tasks.rs` 상단)

```rust
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;

use super::models::{Task, TaskList, UNTITLED};
use super::{enc, Google};
use crate::error::AppError;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListsPage {
    #[serde(default)]
    items: Vec<RawList>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct RawList {
    id: String,
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TasksPage {
    #[serde(default)]
    items: Vec<RawTask>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct RawTask {
    id: String,
    title: Option<String>,
    due: Option<String>,
    notes: Option<String>,
    status: Option<String>,
}

fn title_or_placeholder(title: Option<String>) -> String {
    title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| UNTITLED.into())
}

impl Google {
    pub async fn list_tasklists(&self) -> Result<Vec<TaskList>, AppError> {
        let url = format!("{}/users/@me/lists", self.tasks_base);
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![("maxResults", "100".to_string())];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: ListsPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(page.items.into_iter().map(|l| TaskList { id: l.id, title: title_or_placeholder(l.title) }));
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn list_tasks(&self, list_id: &str) -> Result<Vec<Task>, AppError> {
        let url = format!("{}/lists/{}/tasks", self.tasks_base, enc(list_id));
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut q = vec![
                ("showCompleted", "false".to_string()),
                ("showHidden", "false".to_string()),
                ("maxResults", "100".to_string()),
            ];
            if let Some(t) = &page_token {
                q.push(("pageToken", t.clone()));
            }
            let page: TasksPage = self.send_json(Method::GET, &url, &q, None).await?;
            out.extend(
                page.items
                    .into_iter()
                    .filter(|t| t.status.as_deref() != Some("completed"))
                    .map(|t| Task {
                        id: t.id,
                        list_id: list_id.to_string(),
                        title: title_or_placeholder(t.title),
                        due: t.due.and_then(|d| d.get(..10).map(str::to_string)),
                        notes: t.notes,
                    }),
            );
            match page.next_page_token {
                Some(t) => page_token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn set_task_completed(&self, list_id: &str, task_id: &str, completed: bool) -> Result<(), AppError> {
        let url = format!("{}/lists/{}/tasks/{}", self.tasks_base, enc(list_id), enc(task_id));
        let body = if completed {
            json!({ "status": "completed" })
        } else {
            json!({ "status": "needsAction", "completed": null })
        };
        let _: serde_json::Value = self.send_json(Method::PATCH, &url, &[], Some(&body)).await?;
        Ok(())
    }
}
```
`google/mod.rs` 상단 모듈 선언을 `pub mod calendar; pub mod models; pub mod tasks;`로 바꾼다.

- [ ] **Step 4: 통과 확인**

Run: `cd src-tauri && cargo test --lib`
Expected: 전체 `45 passed` (Task 3~7 합계)

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "feat: add Google Tasks list and completion endpoints" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 8: Tauri 명령, 앱 조립, 트레이

**Files:**
- Create: `src-tauri/src/commands.rs`, `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs` (전체 교체)

**Interfaces:**
- Consumes: Task 3~7의 모든 공개 항목
- Produces:
  - `commands::AppState { auth: Arc<Auth>, google: Google, cache: Cache, settings: Mutex<Settings>, settings_path: PathBuf }`
  - 명령 (JS 이름 → 인자 → 반환):
    - `auth_status` → `boolean`
    - `login` → `void` (에러: `AppError`)
    - `logout` → `void`
    - `get_month {year, month}` → `MonthData`
    - `peek_month {year, month}` → `MonthData | null`
    - `get_tasks` → `TasksData`
    - `peek_tasks` → `TasksData | null`
    - `set_task_completed {listId, taskId, completed}` → `void`
    - `get_settings` → `Settings`
    - `save_settings {settings}` → `void`
  - 프론트로 가는 이벤트: `refresh`(payload 없음), `open-settings`, `settings-changed`(`Settings`), `logged-out`
  - `commands::merge_results<T>(ids: Vec<String>, results: Vec<Result<Vec<T>, AppError>>) -> Result<(Vec<T>, Vec<String>), AppError>`

- [ ] **Step 1: 실패하는 테스트 작성** (`commands.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

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
```
`lib.rs`에 `mod commands;` 추가.

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib commands`
Expected: 컴파일 실패 (`cannot find function merge_results`)

- [ ] **Step 3: `commands.rs` 구현** (테스트 위)

```rust
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
pub fn logout(state: State<'_, AppState>) -> Result<(), AppError> {
    state.auth.logout()
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
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), AppError> {
    let settings = settings.normalized();
    settings::save(&state.settings_path, &settings)?;
    *state.settings.lock().unwrap() = settings.clone();
    apply_autostart(&app, settings.autostart);
    Ok(())
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    if let Err(e) = result {
        log::warn!("자동 실행 설정 실패: {e}");
    }
}
```

- [ ] **Step 4: `tray.rs` 구현**

```rust
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
            if let Err(e) = app.state::<AppState>().auth.logout() {
                log::error!("로그아웃 실패: {e}");
            }
            let _ = app.emit("logged-out", ());
        }
        "quit" => app.exit(0),
        _ => {}
    }
}
```

- [ ] **Step 5: `lib.rs` 전체 교체**

```rust
mod auth;
mod cache;
mod commands;
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
```

- [ ] **Step 6: 테스트와 빌드 확인**

Run: `cd src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings`
Expected: `49 passed`, clippy 경고 없음. 경고가 있으면 수정한다. `dead_code` 경고는 해당 항목을 쓰는 Task가 끝나지 않았다면 `#[allow(dead_code)]` 대신 코드를 다시 확인한다.

- [ ] **Step 7: 커밋**

```bash
git add -A && git commit -m "feat: wire Tauri commands, app state and tray menu" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 9: 월간 격자 계산 (프론트, 순수 함수)

**Files:**
- Create: `src/api.ts`, `src/calendar/month-grid.ts`, `src/calendar/month-grid.test.ts`

**Interfaces:**
- Consumes: Task 8의 명령 이름과 JSON 모양
- Produces:
  - `api.ts`: 타입 `ViewMode`, `Calendar`, `CalEvent`, `TaskList`, `Task`, `MonthData`, `TasksData`, `Settings`, 객체 `api` (`authStatus, login, logout, getMonth, peekMonth, getTasks, peekTasks, setTaskCompleted, getSettings, saveSettings`)
  - `month-grid.ts`: `GridDay {key, date, inMonth, isToday}`, `pad(n)`, `dayKey(Date)`, `parseDateOnly(string)`, `buildMonthGrid(year, month, today) -> GridDay[]`, `eventDayRange(ev) -> [Date, Date]`, `eventsByDay(events, grid) -> Map<string, CalEvent[]>`, `tasksByDay(tasks) -> Map<string, Task[]>`, `formatEventTime(ev) -> string`

- [ ] **Step 1: `src/api.ts` 작성**

```ts
import { invoke } from "@tauri-apps/api/core";

export type ViewMode = "calendar" | "tasks" | "both";

export interface Calendar { id: string; summary: string; color: string; primary: boolean }
export interface CalEvent {
  id: string; calendarId: string; title: string; allDay: boolean;
  start: string; end: string; htmlLink: string | null;
}
export interface TaskList { id: string; title: string }
export interface Task { id: string; listId: string; title: string; due: string | null; notes: string | null }
export interface MonthData { calendars: Calendar[]; events: CalEvent[]; failed: string[]; fetchedAt: string; stale: boolean }
export interface TasksData { lists: TaskList[]; tasks: Task[]; failed: string[]; fetchedAt: string; stale: boolean }
export interface Settings {
  viewMode: ViewMode; hiddenCalendars: string[]; hiddenTaskLists: string[];
  refreshMinutes: number; autostart: boolean; locked: boolean;
}

export const api = {
  authStatus: () => invoke<boolean>("auth_status"),
  login: () => invoke<void>("login"),
  logout: () => invoke<void>("logout"),
  getMonth: (year: number, month: number) => invoke<MonthData>("get_month", { year, month }),
  peekMonth: (year: number, month: number) => invoke<MonthData | null>("peek_month", { year, month }),
  getTasks: () => invoke<TasksData>("get_tasks"),
  peekTasks: () => invoke<TasksData | null>("peek_tasks"),
  setTaskCompleted: (listId: string, taskId: string, completed: boolean) =>
    invoke<void>("set_task_completed", { listId, taskId, completed }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
};
```

- [ ] **Step 2: 실패하는 테스트 작성** (`src/calendar/month-grid.test.ts`)

```ts
import { describe, expect, it } from "vitest";
import type { CalEvent, Task } from "../api";
import { buildMonthGrid, dayKey, eventsByDay, formatEventTime, tasksByDay } from "./month-grid";

const ev = (p: Partial<CalEvent>): CalEvent => ({
  id: "e", calendarId: "c1", title: "t", allDay: false,
  start: "2026-09-15T10:00:00+09:00", end: "2026-09-15T11:00:00+09:00", htmlLink: null, ...p,
});
const today = new Date(2026, 8, 29);

describe("buildMonthGrid", () => {
  it("has 42 days starting on the Sunday before the 1st", () => {
    const g = buildMonthGrid(2026, 9, today);
    expect(g).toHaveLength(42);
    expect(g[0].key).toBe("2026-08-30");
    expect(g[41].key).toBe("2026-10-10");
    expect(g[0].date.getDay()).toBe(0);
  });

  it("starts on the 1st when the month begins on Sunday", () => {
    expect(buildMonthGrid(2026, 2, today)[0].key).toBe("2026-02-01");
  });

  it("marks in-month days and today", () => {
    const g = buildMonthGrid(2026, 9, today);
    expect(g.filter((d) => d.inMonth)).toHaveLength(30);
    expect(g.filter((d) => d.isToday).map((d) => d.key)).toEqual(["2026-09-29"]);
  });

  it("handles leap-year February", () => {
    const g = buildMonthGrid(2028, 2, today);
    expect(g.filter((d) => d.inMonth)).toHaveLength(29);
  });

  it("handles December → January rollover", () => {
    expect(buildMonthGrid(2026, 12, today)[41].key).toBe("2027-01-09");
  });
});

describe("eventsByDay", () => {
  const grid = buildMonthGrid(2026, 9, today);

  it("places a timed event on its day", () => {
    const m = eventsByDay([ev({ id: "a" })], grid);
    expect(m.get("2026-09-15")?.map((e) => e.id)).toEqual(["a"]);
  });

  it("treats all-day end date as exclusive", () => {
    const m = eventsByDay([ev({ id: "v", allDay: true, start: "2026-09-10", end: "2026-09-12" })], grid);
    expect(m.has("2026-09-10")).toBe(true);
    expect(m.has("2026-09-11")).toBe(true);
    expect(m.has("2026-09-12")).toBe(false);
  });

  it("does not spill an event ending exactly at midnight into the next day", () => {
    const m = eventsByDay([ev({ id: "late", start: "2026-09-15T23:00:00+09:00", end: "2026-09-16T00:00:00+09:00" })], grid);
    expect(m.has("2026-09-15")).toBe(true);
    expect(m.has("2026-09-16")).toBe(false);
  });

  it("shows an overnight event on both days", () => {
    const m = eventsByDay([ev({ id: "night", start: "2026-09-15T22:00:00+09:00", end: "2026-09-16T02:00:00+09:00" })], grid);
    expect(m.has("2026-09-15")).toBe(true);
    expect(m.has("2026-09-16")).toBe(true);
  });

  it("converts UTC times to local days", () => {
    const m = eventsByDay([ev({ id: "utc", start: "2026-09-15T16:00:00Z", end: "2026-09-15T17:00:00Z" })], grid);
    expect(m.has("2026-09-16")).toBe(true);
  });

  it("clips multi-day events to the grid", () => {
    const m = eventsByDay([ev({ id: "long", allDay: true, start: "2026-08-01", end: "2026-12-01" })], grid);
    expect(m.size).toBe(42);
  });

  it("sorts all-day first, then by start time", () => {
    const m = eventsByDay([
      ev({ id: "b", start: "2026-09-15T14:00:00+09:00", end: "2026-09-15T15:00:00+09:00" }),
      ev({ id: "a", start: "2026-09-15T09:00:00+09:00", end: "2026-09-15T10:00:00+09:00" }),
      ev({ id: "d", allDay: true, start: "2026-09-15", end: "2026-09-16" }),
    ], grid);
    expect(m.get("2026-09-15")?.map((e) => e.id)).toEqual(["d", "a", "b"]);
  });
});

describe("tasksByDay / formatEventTime / dayKey", () => {
  it("groups tasks by due date and skips undated tasks", () => {
    const tasks: Task[] = [
      { id: "t1", listId: "L", title: "a", due: "2026-09-30", notes: null },
      { id: "t2", listId: "L", title: "b", due: null, notes: null },
    ];
    const m = tasksByDay(tasks);
    expect([...m.keys()]).toEqual(["2026-09-30"]);
  });

  it("formats time or 종일", () => {
    expect(formatEventTime(ev({}))).toBe("10:00");
    expect(formatEventTime(ev({ allDay: true, start: "2026-09-15", end: "2026-09-16" }))).toBe("종일");
  });

  it("zero-pads day keys", () => {
    expect(dayKey(new Date(2026, 0, 5))).toBe("2026-01-05");
  });
});
```

- [ ] **Step 3: 실패 확인**

Run: `npm test`
Expected: FAIL (`Failed to resolve import "./month-grid"`)

- [ ] **Step 4: 구현** (`src/calendar/month-grid.ts`)

```ts
import type { CalEvent, Task } from "../api";

export interface GridDay { key: string; date: Date; inMonth: boolean; isToday: boolean }

export const pad = (n: number) => String(n).padStart(2, "0");

export function dayKey(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function parseDateOnly(s: string): Date {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d);
}

function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
}

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

/** month는 1~12. 일요일 시작 42칸. */
export function buildMonthGrid(year: number, month: number, today: Date): GridDay[] {
  const first = new Date(year, month - 1, 1);
  const start = addDays(first, -first.getDay());
  const todayKey = dayKey(today);
  return Array.from({ length: 42 }, (_, i) => {
    const date = addDays(start, i);
    const key = dayKey(date);
    return { key, date, inMonth: date.getMonth() === month - 1, isToday: key === todayKey };
  });
}

/** 일정이 걸치는 [첫날, 마지막 날] (둘 다 포함, 로컬 자정). */
export function eventDayRange(ev: CalEvent): [Date, Date] {
  if (ev.allDay) {
    const s = parseDateOnly(ev.start);
    const last = addDays(parseDateOnly(ev.end), -1);
    return [s, last < s ? s : last];
  }
  const s = new Date(ev.start);
  const e = new Date(ev.end);
  const first = startOfDay(s);
  let last = startOfDay(e);
  if (e > s && e.getTime() === last.getTime()) last = addDays(last, -1);
  return [first, last < first ? first : last];
}

function startMs(e: CalEvent): number {
  return e.allDay ? parseDateOnly(e.start).getTime() : new Date(e.start).getTime();
}

export function compareEvents(a: CalEvent, b: CalEvent): number {
  if (a.allDay !== b.allDay) return a.allDay ? -1 : 1;
  return startMs(a) - startMs(b) || a.title.localeCompare(b.title);
}

export function eventsByDay(events: CalEvent[], grid: GridDay[]): Map<string, CalEvent[]> {
  const map = new Map<string, CalEvent[]>();
  if (grid.length === 0) return map;
  const gridFirst = grid[0].date;
  const gridLast = grid[grid.length - 1].date;
  for (const ev of events) {
    let [d, end] = eventDayRange(ev);
    if (d < gridFirst) d = gridFirst;
    if (end > gridLast) end = gridLast;
    for (; d <= end; d = addDays(d, 1)) {
      const k = dayKey(d);
      const list = map.get(k);
      if (list) list.push(ev);
      else map.set(k, [ev]);
    }
  }
  for (const list of map.values()) list.sort(compareEvents);
  return map;
}

export function tasksByDay(tasks: Task[]): Map<string, Task[]> {
  const map = new Map<string, Task[]>();
  for (const t of tasks) {
    if (!t.due) continue;
    const list = map.get(t.due);
    if (list) list.push(t);
    else map.set(t.due, [t]);
  }
  return map;
}

export function formatEventTime(ev: CalEvent): string {
  if (ev.allDay) return "종일";
  const s = new Date(ev.start);
  return `${pad(s.getHours())}:${pad(s.getMinutes())}`;
}
```

- [ ] **Step 5: 통과 확인**

Run: `npm test`
Expected: `15 passed`

- [ ] **Step 6: 커밋**

```bash
git add -A && git commit -m "feat: add month grid and event placement logic" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 10: 할 일 완료 처리 로직 + 오류 문구

**Files:**
- Create: `src/errors.ts`, `src/errors.test.ts`, `src/tasks/task-completion.ts`, `src/tasks/task-completion.test.ts`

**Interfaces:**
- Consumes: 없음 (순수)
- Produces:
  - `errors.ts`: `isAuthError(e: unknown): boolean`, `errorText(e: unknown): string`
  - `task-completion.ts`: `createCompletion(deps: CompletionDeps)` → `{ isChecked(id): boolean; toggle(task: {id, listId}): Promise<void> }`. `CompletionDeps = { setCompleted(listId, taskId, completed): Promise<void>; onChange(): void; onRemove(taskId): void; onError(message): void; delayMs?: number }`

- [ ] **Step 1: 실패하는 테스트 작성**

`src/errors.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { errorText, isAuthError } from "./errors";

describe("errors", () => {
  it("detects auth errors", () => {
    expect(isAuthError({ kind: "NotLoggedIn" })).toBe(true);
    expect(isAuthError({ kind: "AuthExpired" })).toBe(true);
    expect(isAuthError({ kind: "Network", message: "x" })).toBe(false);
    expect(isAuthError(null)).toBe(false);
  });

  it("formats backend errors in Korean", () => {
    expect(errorText({ kind: "Network", message: "dns" })).toBe("네트워크 오류");
    expect(errorText({ kind: "Login", message: "access_denied" })).toBe("access_denied");
    expect(errorText({ kind: "Api", message: { status: 403, message: "forbidden" } })).toBe("forbidden");
    expect(errorText("plain")).toBe("plain");
    expect(errorText(new Error("boom"))).toBe("boom");
  });
});
```
`src/tasks/task-completion.test.ts`:
```ts
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createCompletion } from "./task-completion";

const task = (id: string) => ({ id, listId: "L" });

function setup(setCompleted = vi.fn().mockResolvedValue(undefined)) {
  const deps = { setCompleted, onChange: vi.fn(), onRemove: vi.fn(), onError: vi.fn(), delayMs: 3000 };
  return { deps, c: createCompletion(deps) };
}

describe("createCompletion", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("checks immediately, calls API, removes after delay", async () => {
    const { deps, c } = setup();
    const p = c.toggle(task("a"));
    expect(c.isChecked("a")).toBe(true);
    expect(deps.onChange).toHaveBeenCalled();
    await p;
    expect(deps.setCompleted).toHaveBeenCalledWith("L", "a", true);
    vi.advanceTimersByTime(2999);
    expect(deps.onRemove).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(deps.onRemove).toHaveBeenCalledWith("a");
    expect(c.isChecked("a")).toBe(false);
  });

  it("undo within delay cancels removal and reverts on server", async () => {
    const { deps, c } = setup();
    await c.toggle(task("a"));
    vi.advanceTimersByTime(1000);
    await c.toggle(task("a"));
    expect(deps.setCompleted).toHaveBeenLastCalledWith("L", "a", false);
    vi.advanceTimersByTime(5000);
    expect(deps.onRemove).not.toHaveBeenCalled();
    expect(c.isChecked("a")).toBe(false);
  });

  it("rolls back and reports when API fails", async () => {
    const { deps, c } = setup(vi.fn().mockRejectedValue({ kind: "Network", message: "x" }));
    await c.toggle(task("a"));
    expect(c.isChecked("a")).toBe(false);
    expect(deps.onError).toHaveBeenCalledWith("네트워크 오류");
    vi.advanceTimersByTime(5000);
    expect(deps.onRemove).not.toHaveBeenCalled();
  });

  it("unchecking before the first request resolves does not remove the task", async () => {
    let resolveFirst!: () => void;
    const setCompleted = vi
      .fn()
      .mockImplementationOnce(() => new Promise<void>((r) => (resolveFirst = r)))
      .mockResolvedValue(undefined);
    const { deps, c } = setup(setCompleted);
    const first = c.toggle(task("a"));
    const second = c.toggle(task("a"));
    resolveFirst();
    await first;
    await second;
    vi.advanceTimersByTime(5000);
    expect(deps.onRemove).not.toHaveBeenCalled();
    expect(c.isChecked("a")).toBe(false);
  });

  it("keeps independent timers per task", async () => {
    const { deps, c } = setup();
    await c.toggle(task("a"));
    vi.advanceTimersByTime(2000);
    await c.toggle(task("b"));
    vi.advanceTimersByTime(1000);
    expect(deps.onRemove).toHaveBeenCalledTimes(1);
    expect(deps.onRemove).toHaveBeenCalledWith("a");
    vi.advanceTimersByTime(2000);
    expect(deps.onRemove).toHaveBeenCalledWith("b");
  });

  it("failed undo keeps the task completed and still removes it", async () => {
    const setCompleted = vi.fn().mockResolvedValueOnce(undefined).mockRejectedValueOnce({ kind: "Network" });
    const { deps, c } = setup(setCompleted);
    await c.toggle(task("a"));
    await c.toggle(task("a"));
    expect(c.isChecked("a")).toBe(true);
    expect(deps.onError).toHaveBeenCalled();
    vi.advanceTimersByTime(3000);
    expect(deps.onRemove).toHaveBeenCalledWith("a");
  });
});
```

- [ ] **Step 2: 실패 확인**

Run: `npm test`
Expected: FAIL (`Failed to resolve import "./errors"`, `"./task-completion"`)

- [ ] **Step 3: 구현**

`src/errors.ts`:
```ts
interface ErrorPayload { kind?: unknown; message?: unknown }

export function isAuthError(e: unknown): boolean {
  const kind = (e as ErrorPayload | null)?.kind;
  return kind === "NotLoggedIn" || kind === "AuthExpired";
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  const p = e as ErrorPayload | null;
  if (p && typeof p.kind === "string") {
    if (p.kind === "NotLoggedIn") return "로그인이 필요합니다";
    if (p.kind === "AuthExpired") return "로그인이 만료되었습니다";
    if (p.kind === "Network") return "네트워크 오류";
    if (typeof p.message === "string") return p.message;
    if (p.message && typeof p.message === "object" && "message" in p.message) {
      return String((p.message as { message: unknown }).message);
    }
    return p.kind;
  }
  return String(e);
}
```
`src/tasks/task-completion.ts`:
```ts
import { errorText } from "../errors";

export interface CompletionDeps {
  setCompleted(listId: string, taskId: string, completed: boolean): Promise<void>;
  onChange(): void;
  onRemove(taskId: string): void;
  onError(message: string): void;
  delayMs?: number;
}

interface TaskRef { id: string; listId: string }

export function createCompletion(deps: CompletionDeps) {
  const delay = deps.delayMs ?? 3000;
  const checked = new Set<string>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function cancelRemoval(id: string) {
    const t = timers.get(id);
    if (t !== undefined) {
      clearTimeout(t);
      timers.delete(id);
    }
  }

  function scheduleRemoval(id: string) {
    cancelRemoval(id);
    timers.set(id, setTimeout(() => {
      timers.delete(id);
      checked.delete(id);
      deps.onRemove(id);
    }, delay));
  }

  return {
    isChecked: (id: string) => checked.has(id),

    async toggle(task: TaskRef): Promise<void> {
      const nowChecked = !checked.has(task.id);
      if (nowChecked) checked.add(task.id);
      else checked.delete(task.id);
      cancelRemoval(task.id);
      deps.onChange();
      try {
        await deps.setCompleted(task.listId, task.id, nowChecked);
      } catch (e) {
        // 서버 상태는 요청 전 상태 그대로이므로 UI를 되돌린다.
        if (nowChecked) checked.delete(task.id);
        else checked.add(task.id);
        deps.onChange();
        deps.onError(errorText(e));
        if (!nowChecked) scheduleRemoval(task.id);
        return;
      }
      if (nowChecked && checked.has(task.id)) scheduleRemoval(task.id);
    },
  };
}
```

- [ ] **Step 4: 통과 확인**

Run: `npm test`
Expected: `23 passed`

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "feat: add task completion state machine and error text" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 11: 화면 구성 및 첫 실제 실행

**Files:**
- Create: `src/dom.ts`, `src/calendar/MonthCalendar.ts`, `src/tasks/TaskList.ts`, `src/ViewSwitcher.ts`, `src/Settings.ts`
- Overwrite: `index.html`, `src/main.ts`, `src/styles.css`
- Delete: 스캐폴드의 `src/assets/`와 사용하지 않는 파일 (`src/vite-env.d.ts`는 유지)

**Interfaces:**
- Consumes: `api`, 타입들 (Task 9), `month-grid` (Task 9), `createCompletion`, `errorText`, `isAuthError` (Task 10), 이벤트 `refresh` / `open-settings` / `settings-changed` / `logged-out` (Task 8)
- Produces: 실행 가능한 위젯 UI

- [ ] **Step 1: `index.html` 덮어쓰기**

```html
<!doctype html>
<html lang="ko">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>gcal-widget</title>
    <script type="module" src="/src/main.ts" defer></script>
  </head>
  <body>
    <div id="app" data-view="both" data-auth="out">
      <header id="header" data-tauri-drag-region>
        <div id="views" class="views"></div>
        <span id="status" class="status" data-tauri-drag-region></span>
        <button id="btn-refresh" class="icon" title="새로고침">⟳</button>
        <button id="btn-settings" class="icon" title="설정">⚙</button>
      </header>
      <section id="login">
        <p>Google 계정으로 로그인하면<br />일정과 할 일을 볼 수 있습니다.</p>
        <button id="btn-login">Google 로그인</button>
        <p id="login-msg" class="muted"></p>
      </section>
      <main id="content">
        <section id="calendar"></section>
        <section id="tasks"></section>
      </main>
      <div id="overlay" hidden></div>
      <div id="toast" hidden></div>
    </div>
  </body>
</html>
```

- [ ] **Step 2: `src/dom.ts`, `src/ViewSwitcher.ts` 작성**

`src/dom.ts`:
```ts
const ENTITIES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

export function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ENTITIES[c]);
}
```
`src/ViewSwitcher.ts`:
```ts
import type { ViewMode } from "./api";

const MODES: [ViewMode, string][] = [["calendar", "캘린더"], ["both", "모두"], ["tasks", "할 일"]];

export function renderViewSwitcher(el: HTMLElement, current: ViewMode, onChange: (m: ViewMode) => void): void {
  el.innerHTML = MODES.map(([m, label]) =>
    `<button class="${m === current ? "active" : ""}" data-mode="${m}">${label}</button>`).join("");
  el.querySelectorAll<HTMLButtonElement>("[data-mode]").forEach((b) => {
    b.onclick = () => onChange(b.dataset.mode as ViewMode);
  });
}
```

- [ ] **Step 3: `src/calendar/MonthCalendar.ts` 작성**

```ts
import type { CalEvent, Task } from "../api";
import { esc } from "../dom";
import { formatEventTime, type GridDay } from "./month-grid";

export interface MonthViewProps {
  year: number;
  month: number;
  grid: GridDay[];
  events: Map<string, CalEvent[]>;
  tasks: Map<string, Task[]>;
  colors: Map<string, string>;
  selected: string;
  onSelect(key: string): void;
  onNav(delta: -1 | 0 | 1): void;
}

const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];
const FALLBACK_COLOR = "#4285f4";

export function renderMonth(el: HTMLElement, p: MonthViewProps): void {
  const color = (e: CalEvent) => esc(p.colors.get(e.calendarId) ?? FALLBACK_COLOR);

  const cells = p.grid.map((d, i) => {
    const evs = p.events.get(d.key) ?? [];
    const dots = [...new Set(evs.map(color))].slice(0, 3).map((c) => `<i class="dot" style="background:${c}"></i>`).join("");
    const taskDot = p.tasks.has(d.key) ? `<i class="dot task-dot"></i>` : "";
    const cls = ["cell", `wd-${i % 7}`, d.inMonth ? "" : "out", d.isToday ? "today" : "", d.key === p.selected ? "selected" : ""]
      .filter(Boolean).join(" ");
    return `<button class="${cls}" data-day="${d.key}"><span class="num">${d.date.getDate()}</span><span class="dots">${dots}${taskDot}</span></button>`;
  }).join("");

  const [, sm, sd] = p.selected.split("-").map(Number);
  const items = [
    ...(p.events.get(p.selected) ?? []).map((e) =>
      `<li><i class="bar" style="background:${color(e)}"></i><span class="time">${formatEventTime(e)}</span><span class="title">${esc(e.title)}</span></li>`),
    ...(p.tasks.get(p.selected) ?? []).map((t) =>
      `<li><i class="bar task-bar"></i><span class="time">할 일</span><span class="title">${esc(t.title)}</span></li>`),
  ];

  el.innerHTML = `
    <div class="month-head">
      <button class="nav" data-nav="-1" title="이전 달">‹</button>
      <span class="month-title">${p.year}년 ${p.month}월</span>
      <button class="nav" data-nav="1" title="다음 달">›</button>
      <button class="nav today-btn" data-nav="0">오늘</button>
    </div>
    <div class="grid">
      ${WEEKDAYS.map((w, i) => `<div class="wd wd-${i}">${w}</div>`).join("")}
      ${cells}
    </div>
    <div class="day-detail">
      <h4>${sm}월 ${sd}일</h4>
      ${items.length ? `<ul>${items.join("")}</ul>` : `<p class="empty">일정 없음</p>`}
    </div>`;

  el.querySelectorAll<HTMLButtonElement>("[data-nav]").forEach((b) => {
    b.onclick = () => p.onNav(Number(b.dataset.nav) as -1 | 0 | 1);
  });
  el.querySelectorAll<HTMLButtonElement>("[data-day]").forEach((b) => {
    b.onclick = () => p.onSelect(b.dataset.day!);
  });
}
```

- [ ] **Step 4: `src/tasks/TaskList.ts` 작성**

```ts
import type { Task, TaskList } from "../api";
import { esc } from "../dom";

export interface TaskViewProps {
  lists: TaskList[];
  tasks: Task[];
  hidden: string[];
  today: string;
  isChecked(id: string): boolean;
  onToggle(task: Task): void;
}

function dueLabel(due: string, today: string): string {
  const text = `${Number(due.slice(5, 7))}/${Number(due.slice(8, 10))}`;
  return `<span class="due ${due < today ? "overdue" : ""}">${text}</span>`;
}

export function renderTasks(el: HTMLElement, p: TaskViewProps): void {
  const groups = p.lists
    .filter((l) => !p.hidden.includes(l.id))
    .map((l) => ({ list: l, tasks: p.tasks.filter((t) => t.listId === l.id) }))
    .filter((g) => g.tasks.length > 0);

  if (groups.length === 0) {
    el.innerHTML = `<p class="empty">할 일이 없습니다</p>`;
    return;
  }

  el.innerHTML = groups.map((g) => `
    <div class="task-group">
      <h3>${esc(g.list.title)}</h3>
      <ul>${g.tasks.map((t) => {
        const checked = p.isChecked(t.id);
        return `<li class="${checked ? "done" : ""}">
          <label><input type="checkbox" data-task="${esc(t.id)}" ${checked ? "checked" : ""}><span class="title">${esc(t.title)}</span></label>
          ${t.due ? dueLabel(t.due, p.today) : ""}
        </li>`;
      }).join("")}</ul>
    </div>`).join("");

  el.querySelectorAll<HTMLInputElement>("input[data-task]").forEach((cb) => {
    cb.onchange = () => {
      const t = p.tasks.find((x) => x.id === cb.dataset.task);
      if (t) p.onToggle(t);
    };
  });
}
```

- [ ] **Step 5: `src/Settings.ts` 작성**

```ts
import type { Calendar, Settings, TaskList } from "./api";
import { esc } from "./dom";

export interface SettingsViewProps {
  settings: Settings;
  calendars: Calendar[];
  lists: TaskList[];
  onSave(next: Settings): void;
  onClose(): void;
  onLogout(): void;
}

const REFRESH_OPTIONS = [5, 10, 15, 30];

export function renderSettings(el: HTMLElement, p: SettingsViewProps): void {
  const s = p.settings;
  const calRows = p.calendars.map((c) => `
    <label class="row"><input type="checkbox" data-cal="${esc(c.id)}" ${s.hiddenCalendars.includes(c.id) ? "" : "checked"}>
    <i class="dot" style="background:${esc(c.color)}"></i><span class="title">${esc(c.summary)}</span></label>`).join("");
  const listRows = p.lists.map((l) => `
    <label class="row"><input type="checkbox" data-list="${esc(l.id)}" ${s.hiddenTaskLists.includes(l.id) ? "" : "checked"}>
    <span class="title">${esc(l.title)}</span></label>`).join("");

  el.innerHTML = `
    <div class="panel">
      <h2>설정</h2>
      <h3>캘린더</h3>${calRows || `<p class="empty">불러온 캘린더가 없습니다</p>`}
      <h3>할 일 목록</h3>${listRows || `<p class="empty">불러온 목록이 없습니다</p>`}
      <h3>일반</h3>
      <label class="row">새로고침 간격
        <select id="refresh">${REFRESH_OPTIONS.map((m) =>
          `<option value="${m}" ${m === s.refreshMinutes ? "selected" : ""}>${m}분</option>`).join("")}</select>
      </label>
      <label class="row"><input type="checkbox" id="autostart" ${s.autostart ? "checked" : ""}>Windows 시작 시 자동 실행</label>
      <div class="actions">
        <button id="logout" class="ghost">로그아웃</button>
        <span class="spacer"></span>
        <button id="cancel" class="ghost">취소</button>
        <button id="save">저장</button>
      </div>
    </div>`;

  const unchecked = (attr: string) =>
    [...el.querySelectorAll<HTMLInputElement>(`input[data-${attr}]`)]
      .filter((i) => !i.checked)
      .map((i) => i.dataset[attr]!);

  el.querySelector<HTMLButtonElement>("#save")!.onclick = () => p.onSave({
    ...s,
    hiddenCalendars: unchecked("cal"),
    hiddenTaskLists: unchecked("list"),
    refreshMinutes: Number(el.querySelector<HTMLSelectElement>("#refresh")!.value),
    autostart: el.querySelector<HTMLInputElement>("#autostart")!.checked,
  });
  el.querySelector<HTMLButtonElement>("#cancel")!.onclick = () => p.onClose();
  el.querySelector<HTMLButtonElement>("#logout")!.onclick = () => p.onLogout();
}
```

- [ ] **Step 6: `src/main.ts` 덮어쓰기**

```ts
import "./styles.css";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type MonthData, type Settings, type TasksData, type ViewMode } from "./api";
import { errorText, isAuthError } from "./errors";
import { buildMonthGrid, dayKey, eventsByDay, pad, tasksByDay } from "./calendar/month-grid";
import { renderMonth } from "./calendar/MonthCalendar";
import { renderTasks } from "./tasks/TaskList";
import { createCompletion } from "./tasks/task-completion";
import { renderViewSwitcher } from "./ViewSwitcher";
import { renderSettings } from "./Settings";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const app = $("app");
const header = $("header");
const viewsEl = $("views");
const statusEl = $("status");
const calEl = $("calendar");
const tasksEl = $("tasks");
const overlay = $("overlay");
const toastEl = $("toast");
const loginMsg = $("login-msg");
const loginBtn = $<HTMLButtonElement>("btn-login");

const now = new Date();
const state = {
  settings: null as Settings | null,
  year: now.getFullYear(),
  month: now.getMonth() + 1,
  selected: dayKey(now),
  monthData: null as MonthData | null,
  tasksData: null as TasksData | null,
  loggedIn: false,
  loading: false,
};

let toastTimer = 0;
function toast(msg: string) {
  toastEl.textContent = msg;
  toastEl.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (toastEl.hidden = true), 4000);
}

const completion = createCompletion({
  setCompleted: (listId, taskId, completed) => api.setTaskCompleted(listId, taskId, completed),
  onChange: () => renderTasksView(),
  onRemove: (id) => {
    if (state.tasksData) state.tasksData.tasks = state.tasksData.tasks.filter((t) => t.id !== id);
    render();
  },
  onError: (m) => toast(`완료 처리 실패: ${m}`),
});

function hhmm(iso: string): string {
  const d = new Date(iso);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function updateStatus() {
  const parts: string[] = [];
  const stale = state.monthData?.stale || state.tasksData?.stale;
  const fetched = state.monthData?.fetchedAt ?? state.tasksData?.fetchedAt;
  if (stale && fetched) parts.push(`오프라인 · 마지막 갱신 ${hhmm(fetched)}`);
  const failed = (state.monthData?.failed.length ?? 0) + (state.tasksData?.failed.length ?? 0);
  if (failed > 0) parts.push("일부 목록을 불러오지 못했습니다");
  statusEl.textContent = parts.join(" · ");
}

function showLogin(message = "") {
  state.loggedIn = false;
  app.dataset.auth = "out";
  loginMsg.textContent = message;
}

function handleError(e: unknown) {
  if (isAuthError(e)) showLogin("다시 로그인해 주세요");
  else toast(errorText(e));
}

function renderTasksView() {
  const s = state.settings!;
  renderTasks(tasksEl, {
    lists: state.tasksData?.lists ?? [],
    tasks: state.tasksData?.tasks ?? [],
    hidden: s.hiddenTaskLists,
    today: dayKey(new Date()),
    isChecked: completion.isChecked,
    onToggle: (t) => void completion.toggle(t),
  });
}

function onNav(delta: -1 | 0 | 1) {
  if (delta === 0) {
    const t = new Date();
    state.year = t.getFullYear();
    state.month = t.getMonth() + 1;
    state.selected = dayKey(t);
  } else {
    const d = new Date(state.year, state.month - 1 + delta, 1);
    state.year = d.getFullYear();
    state.month = d.getMonth() + 1;
    state.selected = dayKey(d);
  }
  render();
  void loadMonth();
}

function render() {
  const s = state.settings;
  if (!s) return;
  app.dataset.view = s.viewMode;
  renderViewSwitcher(viewsEl, s.viewMode, (mode: ViewMode) => void saveSettings({ ...s, viewMode: mode }));
  if (!state.loggedIn) return;
  const grid = buildMonthGrid(state.year, state.month, new Date());
  const visibleTasks = (state.tasksData?.tasks ?? []).filter((t) => !s.hiddenTaskLists.includes(t.listId));
  renderMonth(calEl, {
    year: state.year,
    month: state.month,
    grid,
    events: eventsByDay(state.monthData?.events ?? [], grid),
    tasks: tasksByDay(visibleTasks),
    colors: new Map((state.monthData?.calendars ?? []).map((c) => [c.id, c.color])),
    selected: state.selected,
    onSelect: (k) => {
      state.selected = k;
      render();
    },
    onNav,
  });
  renderTasksView();
}

async function loadMonth() {
  const { year, month } = state;
  try {
    const data = await api.getMonth(year, month);
    if (state.year === year && state.month === month) state.monthData = data;
    render();
  } catch (e) {
    handleError(e);
  }
  updateStatus();
}

let lastRefresh = 0;
async function refreshAll() {
  if (!state.loggedIn || state.loading) return;
  state.loading = true;
  try {
    const [m, t] = await Promise.all([api.getMonth(state.year, state.month), api.getTasks()]);
    state.monthData = m;
    state.tasksData = t;
    render();
  } catch (e) {
    handleError(e);
  } finally {
    state.loading = false;
    lastRefresh = Date.now();
    updateStatus();
  }
}

function applyLock(locked: boolean) {
  for (const el of [header, statusEl]) {
    if (locked) el.removeAttribute("data-tauri-drag-region");
    else el.setAttribute("data-tauri-drag-region", "");
  }
  app.dataset.locked = String(locked);
  getCurrentWindow().setResizable(!locked).catch(() => {});
}

let refreshTimer = 0;
function resetTimer() {
  clearInterval(refreshTimer);
  refreshTimer = window.setInterval(() => void refreshAll(), state.settings!.refreshMinutes * 60_000);
}

async function saveSettings(next: Settings) {
  try {
    await api.saveSettings(next);
    state.settings = next;
    applyLock(next.locked);
    resetTimer();
    render();
  } catch (e) {
    toast(`설정 저장 실패: ${errorText(e)}`);
  }
}

function openSettings() {
  if (!state.settings) return;
  overlay.hidden = false;
  renderSettings(overlay, {
    settings: state.settings,
    calendars: state.monthData?.calendars ?? [],
    lists: state.tasksData?.lists ?? [],
    onSave: async (next) => {
      overlay.hidden = true;
      await saveSettings(next);
      await refreshAll();
    },
    onClose: () => (overlay.hidden = true),
    onLogout: async () => {
      overlay.hidden = true;
      await api.logout().catch(() => {});
      showLogin();
    },
  });
}

async function doLogin() {
  loginBtn.disabled = true;
  loginMsg.textContent = "브라우저에서 로그인을 완료해 주세요…";
  try {
    await api.login();
    state.loggedIn = true;
    app.dataset.auth = "in";
    loginMsg.textContent = "";
    await refreshAll();
  } catch (e) {
    loginMsg.textContent = errorText(e);
  } finally {
    loginBtn.disabled = false;
  }
}

async function init() {
  state.settings = await api.getSettings();
  applyLock(state.settings.locked);
  resetTimer();
  loginBtn.onclick = () => void doLogin();
  $("btn-refresh").onclick = () => void refreshAll();
  $("btn-settings").onclick = openSettings;

  await listen("refresh", () => void refreshAll());
  await listen("open-settings", openSettings);
  await listen<Settings>("settings-changed", (e) => {
    state.settings = e.payload;
    applyLock(e.payload.locked);
    render();
  });
  await listen("logged-out", () => showLogin());

  // 절전 복귀 감지: 30초 틱이 90초 넘게 밀리면 새로고침.
  let lastTick = Date.now();
  window.setInterval(() => {
    const t = Date.now();
    if (t - lastTick > 90_000 && t - lastRefresh > 60_000) void refreshAll();
    lastTick = t;
  }, 30_000);
  window.addEventListener("online", () => void refreshAll());

  render();
  if (await api.authStatus()) {
    state.loggedIn = true;
    app.dataset.auth = "in";
    const [m, t] = await Promise.all([api.peekMonth(state.year, state.month), api.peekTasks()]);
    state.monthData = m;
    state.tasksData = t;
    render();
    await refreshAll();
  } else {
    showLogin();
  }
}

init().catch((e) => toast(errorText(e)));
```

- [ ] **Step 7: `src/styles.css` 덮어쓰기**

```css
:root {
  --bg: rgba(22, 24, 30, 0.86);
  --panel: #202329;
  --fg: #e8eaed;
  --muted: #9aa0a6;
  --line: rgba(255, 255, 255, 0.08);
  --hover: rgba(255, 255, 255, 0.08);
  --accent: #8ab4f8;
  --sun: #f28b82;
  --sat: #8ab4f8;
  --task: #fdd663;
}
* { box-sizing: border-box; }
html, body {
  margin: 0; height: 100%; overflow: hidden; background: transparent; color: var(--fg);
  font: 13px/1.4 "Segoe UI", "Malgun Gothic", sans-serif; user-select: none;
}
button { font: inherit; color: inherit; background: none; border: 0; cursor: pointer; border-radius: 6px; }
input[type="checkbox"] { accent-color: var(--accent); margin: 0; }
select { background: #2b2f36; color: var(--fg); border: 1px solid var(--line); border-radius: 4px; }

#app {
  position: relative; height: 100%; display: flex; flex-direction: column;
  background: var(--bg); border: 1px solid var(--line); border-radius: 12px; overflow: hidden;
}
header { display: flex; align-items: center; gap: 6px; padding: 8px 10px; border-bottom: 1px solid var(--line); cursor: grab; }
[data-locked="true"] header { cursor: default; }
.views { display: flex; gap: 2px; padding: 2px; border-radius: 8px; background: rgba(255, 255, 255, 0.06); }
.views button { padding: 3px 8px; color: var(--muted); }
.views button.active { background: rgba(255, 255, 255, 0.14); color: var(--fg); }
.status { flex: 1; min-width: 0; font-size: 11px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
button.icon { width: 26px; height: 26px; font-size: 15px; color: var(--muted); }
button.icon:hover, .nav:hover, .cell:hover { background: var(--hover); color: var(--fg); }

#login { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 16px; text-align: center; }
#btn-login, #save { padding: 7px 16px; background: var(--accent); color: #202124; font-weight: 600; }
#btn-login:disabled { opacity: 0.6; cursor: default; }
[data-auth="out"] #content, [data-auth="in"] #login { display: none; }
.muted, .empty { color: var(--muted); }

#content { flex: 1; min-height: 0; display: flex; flex-direction: column; }
#calendar, #tasks { min-height: 0; overflow-y: auto; padding: 8px 10px; }
#calendar { flex: 0 0 auto; }
#tasks { flex: 1; border-top: 1px solid var(--line); }
[data-view="calendar"] #tasks, [data-view="tasks"] #calendar { display: none; }
[data-view="calendar"] #calendar { flex: 1; }
[data-view="tasks"] #tasks { border-top: 0; }
@media (min-width: 640px) {
  [data-view="both"] #content { flex-direction: row; }
  [data-view="both"] #calendar { flex: 1.3; }
  [data-view="both"] #tasks { border-top: 0; border-left: 1px solid var(--line); }
}

.month-head { display: flex; align-items: center; gap: 4px; margin-bottom: 6px; }
.month-title { margin: 0 4px; font-size: 14px; font-weight: 600; }
.nav { padding: 2px 8px; color: var(--muted); }
.today-btn { margin-left: auto; font-size: 12px; }
.grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; }
.wd { padding-bottom: 2px; text-align: center; font-size: 11px; color: var(--muted); }
.wd.wd-0 { color: var(--sun); }
.wd.wd-6 { color: var(--sat); }
.cell { display: flex; flex-direction: column; align-items: center; min-height: 34px; padding: 3px 0; }
.cell.out { opacity: 0.35; }
.cell.wd-0 .num { color: var(--sun); }
.cell.wd-6 .num { color: var(--sat); }
.cell.selected { background: rgba(138, 180, 248, 0.16); }
.cell.today .num { width: 22px; height: 22px; line-height: 22px; border-radius: 50%; text-align: center; background: var(--accent); color: #202124; }
.dots { display: flex; gap: 2px; height: 6px; margin-top: 2px; }
.dot { display: inline-block; flex: none; width: 5px; height: 5px; border-radius: 50%; }
.task-dot { background: var(--task); }

.day-detail { margin-top: 8px; padding-top: 6px; border-top: 1px solid var(--line); }
.day-detail h4 { margin: 0 0 4px; font-size: 12px; font-weight: 600; color: var(--muted); }
.day-detail ul, .task-group ul { list-style: none; margin: 0; padding: 0; }
.day-detail li, .task-group li { display: flex; align-items: center; gap: 6px; padding: 3px 0; }
.bar { align-self: stretch; width: 3px; border-radius: 2px; }
.task-bar { background: var(--task); }
.time { min-width: 38px; font-size: 11px; color: var(--muted); }
.title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.task-group h3 { margin: 4px 0; font-size: 12px; font-weight: 600; color: var(--muted); }
.task-group label { display: flex; flex: 1; min-width: 0; align-items: center; gap: 8px; cursor: pointer; }
.task-group li.done .title { text-decoration: line-through; color: var(--muted); }
.due { font-size: 11px; color: var(--muted); }
.due.overdue { color: var(--sun); }

#overlay { position: absolute; inset: 0; display: flex; align-items: flex-start; justify-content: center; padding: 12px; background: rgba(0, 0, 0, 0.45); }
#overlay[hidden], #toast[hidden] { display: none; }
.panel { width: 100%; max-height: 100%; overflow-y: auto; padding: 12px 14px; background: var(--panel); border: 1px solid var(--line); border-radius: 10px; }
.panel h2 { margin: 0 0 8px; font-size: 15px; }
.panel h3 { margin: 12px 0 4px; font-size: 12px; color: var(--muted); }
.row { display: flex; align-items: center; gap: 8px; padding: 3px 0; }
.actions { display: flex; gap: 6px; margin-top: 14px; }
.spacer { flex: 1; }
.ghost { padding: 6px 12px; background: rgba(255, 255, 255, 0.06); }
#toast { position: absolute; left: 10px; right: 10px; bottom: 10px; padding: 8px 10px; border-radius: 8px; background: #3c4043; font-size: 12px; }
```

- [ ] **Step 8: 스캐폴드 잔여 파일 정리 및 타입 확인**

```bash
rm -rf src/assets && ls src && npx tsc --noEmit && npm test
```
Expected: 타입 오류 없음, 테스트 `23 passed`

- [ ] **Step 9: (사용자) Google Cloud 설정과 `.env` 작성**

README "준비 > 2~3"을 따라 사용자가 직접 진행한다. 구현자는 사용자에게 요청하고 완료를 기다린다. 완료 후 `.env`에 두 값이 들어 있는지 **값은 출력하지 않고** 확인한다:
```bash
grep -c "^GOOGLE_CLIENT_ID=.\+" .env && grep -c "^GOOGLE_CLIENT_SECRET=.\+" .env
```
Expected: `1`, `1`

- [ ] **Step 10: 첫 실행 확인**

Run: `npm run tauri dev`
확인 항목 (모두 통과해야 한다):
1. 테두리 없는 반투명 창이 뜨고, 작업표시줄에는 아이콘이 없고 트레이에 아이콘이 있다.
2. "Google 로그인" → 브라우저 동의 화면 → "로그인 완료" 페이지 → 위젯에 이번 달 달력과 할 일이 나온다.
3. 날짜를 클릭하면 아래에 그날 일정이 나온다. ‹ › 오늘 버튼이 동작한다.
4. 할 일을 체크하면 취소선이 그어지고 3초 뒤 사라진다. calendar.google.com의 Tasks에서도 완료로 표시된다.
5. 보기 모드 버튼 3개가 동작한다. 창 크기를 바꾸고 앱을 재시작하면 모드와 크기, 위치가 유지된다.
6. 설정에서 캘린더 하나를 끄고 저장하면 그 캘린더의 점이 사라진다.

- [ ] **Step 11: 커밋**

```bash
git add -A && git commit -m "feat: add widget UI (month calendar, tasks, view switcher, settings)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 12: 바탕화면 고정

**Files:**
- Create: `src-tauri/src/desktop.rs`
- Modify: `src-tauri/src/lib.rs` (`mod desktop;`, setup에서 `desktop::pin(&window);`를 `window.show()?` 앞에 호출)

**Interfaces:**
- Consumes: `tauri::WebviewWindow::hwnd()`
- Produces: `desktop::pin(window: &tauri::WebviewWindow)` (메인 스레드에서 호출. Tauri `setup`은 메인 스레드에서 실행된다)

- [ ] **Step 1: `desktop.rs` 작성**

windows 0.58 기준 시그니처다. 컴파일 오류가 나면 해당 버전 문서(`docs.rs/windows/0.58`)에서 함수 시그니처를 확인해 맞춘다. 동작 로직은 바꾸지 않는다.
```rust
//! 위젯을 바탕화면 층에 고정한다.
//! - WM_WINDOWPOSCHANGING을 가로채 항상 HWND_BOTTOM에 둔다 (클릭해도 다른 창 위로 올라오지 않음).
//! - 바탕화면(Progman/WorkerW)이 전경이 되면(Win+D, 바탕화면 클릭) 잠시 최상위로 올려 보이게 한다.

pub fn pin(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    match window.hwnd() {
        Ok(hwnd) => imp::pin(hwnd.0 as isize),
        Err(e) => log::error!("HWND를 얻지 못했습니다: {e}"),
    }
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, SetWindowPos, EVENT_SYSTEM_FOREGROUND, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WINDOWPOS, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        WM_WINDOWPOSCHANGING,
    };

    static WIDGET: AtomicIsize = AtomicIsize::new(0);
    static RAISED: AtomicBool = AtomicBool::new(false);

    fn widget() -> HWND {
        HWND(WIDGET.load(Ordering::Relaxed) as *mut _)
    }

    pub fn pin(raw: isize) {
        WIDGET.store(raw, Ordering::Relaxed);
        let hwnd = widget();
        unsafe {
            if !SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0).as_bool() {
                log::warn!("SetWindowSubclass 실패");
            }
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                HMODULE::default(),
                Some(on_foreground),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
            if hook.is_invalid() {
                log::warn!("SetWinEventHook 실패");
            }
            send_to_bottom(hwnd);
        }
    }

    unsafe fn send_to_bottom(hwnd: HWND) {
        let _ = SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_WINDOWPOSCHANGING && !RAISED.load(Ordering::Relaxed) {
            let pos = &mut *(lparam.0 as *mut WINDOWPOS);
            pos.hwndInsertAfter = HWND_BOTTOM;
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    unsafe fn is_desktop(hwnd: HWND) -> bool {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut buf);
        if n <= 0 {
            return false;
        }
        let class = String::from_utf16_lossy(&buf[..n as usize]);
        class == "Progman" || class == "WorkerW"
    }

    unsafe extern "system" fn on_foreground(
        _hook: HWINEVENTHOOK,
        _event: u32,
        foreground: HWND,
        _id_object: i32,
        _id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        let hwnd = widget();
        if is_desktop(foreground) {
            RAISED.store(true, Ordering::Relaxed);
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        } else if RAISED.swap(false, Ordering::Relaxed) {
            let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            send_to_bottom(hwnd);
        }
    }
}
```

- [ ] **Step 2: `lib.rs` 수정**

`mod desktop;`를 추가하고 setup 끝부분을 다음처럼 바꾼다:
```rust
            let window = app.get_webview_window("main").expect("main window is configured");
            desktop::pin(&window);
            window.show()?;
            Ok(())
```

- [ ] **Step 3: 빌드 확인**

Run: `cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test --lib`
Expected: 경고 없음, `49 passed`

- [ ] **Step 4: 수동 검증** (`npm run tauri dev`)

1. 메모장 등 다른 창을 위젯 위로 옮기면 위젯이 **뒤에** 가려진다.
2. 위젯을 클릭하거나 할 일을 체크해도 위젯이 다른 창 **위로 올라오지 않는다**.
3. Win+D를 누르면 위젯이 **보인다**. 다시 Win+D를 누르거나 다른 창을 클릭하면 위젯이 다시 뒤로 간다.
4. 위젯 상단을 드래그해 옮길 수 있고, 가장자리로 크기를 바꿀 수 있다.
5. 트레이 "위치 잠금"을 켜면 드래그와 크기 조절이 안 된다. 끄면 다시 된다.

3번이 실패하면 로그(`%LOCALAPPDATA%\kr.taekit93.gcalwidget\logs\gcal-widget.log`)를 확인한다. Win+D 직후 전경 창의 클래스 이름을 `log::info!`로 임시 출력해 `is_desktop` 조건을 보정한다. 보정 후 임시 로그는 지운다.

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "feat: pin widget to desktop layer" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```

---

### Task 13: 수동 체크리스트, 릴리스 빌드

**Files:**
- Create: `docs/manual-test-checklist.md`

**Interfaces:**
- Consumes: 완성된 앱
- Produces: 설치 파일 `src-tauri/target/release/bundle/nsis/gcal-widget_0.1.0_x64-setup.exe`

- [ ] **Step 1: `docs/manual-test-checklist.md` 작성**

```markdown
# 수동 테스트 체크리스트

릴리스 빌드를 설치한 상태에서 확인한다.

## 로그인
- [ ] 첫 실행 시 로그인 화면이 뜬다.
- [ ] 로그인 도중 브라우저에서 "취소"하면 위젯에 오류 문구가 나오고 다시 시도할 수 있다.
- [ ] 로그인 후 재부팅해도 다시 로그인하지 않는다.
- [ ] 트레이 "로그아웃" → 로그인 화면으로 돌아간다. Windows 자격 증명 관리자에서 `gcal-widget` 항목이 사라진다.

## 자동 실행
- [ ] 재부팅 후 위젯이 자동으로 바탕화면에 뜬다.
- [ ] 설정에서 자동 실행을 끄고 재부팅하면 뜨지 않는다.

## 바탕화면 고정
- [ ] 다른 창보다 항상 뒤에 있다. 클릭해도 올라오지 않는다.
- [ ] Win+D에서 보인다.
- [ ] 작업표시줄에 아이콘이 없다. 트레이에만 있다.

## 데이터
- [ ] 공휴일 캘린더를 구독 중이면 공휴일이 달력에 표시된다.
- [ ] 여러 날 종일 일정이 각 날짜에 표시된다.
- [ ] 마감일이 있는 할 일이 달력 해당 날짜에 노란 점으로 표시된다.
- [ ] 할 일 완료 → 3초 안에 다시 클릭하면 취소되고 목록에 남는다.

## 오프라인·복귀
- [ ] Wi-Fi를 끄고 새로고침하면 "오프라인 · 마지막 갱신 HH:MM"이 표시되고 데이터는 그대로 있다.
- [ ] Wi-Fi를 켜면 자동으로 갱신된다.
- [ ] 절전 모드에서 복귀하면 1분 안에 갱신된다.

## 설정 유지
- [ ] 보기 모드, 창 크기와 위치, 숨긴 캘린더, 위치 잠금이 재시작 후에도 유지된다.
- [ ] 이미 실행 중일 때 다시 실행하면 두 번째 창이 뜨지 않는다.

## 자원
- [ ] 작업 관리자에서 gcal-widget + WebView2 메모리 합계가 약 100MB 이하다.
```

- [ ] **Step 2: 전체 테스트**

Run: `npm test && cd src-tauri && cargo test --lib && cargo clippy --all-targets -- -D warnings`
Expected: 프론트 `23 passed`, Rust `49 passed`, 경고 없음

- [ ] **Step 3: 릴리스 빌드**

Run: `npm run tauri build`
Expected: `src-tauri/target/release/bundle/nsis/gcal-widget_0.1.0_x64-setup.exe` 생성

- [ ] **Step 4: 설치 후 체크리스트 수행**

설치 파일을 실행해 설치하고, 개발 실행으로 등록된 자동 실행 항목이 있으면 설정에서 자동 실행을 껐다 켜서 설치된 exe 경로로 다시 등록한다. 그다음 `docs/manual-test-checklist.md`를 모두 확인하고, 실패 항목은 사용자에게 보고한다.

- [ ] **Step 5: 커밋**

```bash
git add -A && git commit -m "docs: add manual test checklist" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" && git push
```
