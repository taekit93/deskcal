# Google Calendar & Tasks 데스크톱 위젯 — 설계 문서

- 작성일: 2026-09-29
- 상태: 검토 대기
- 저장소: GitHub `taekit93/google-calendar-widget` (Private)
- 로컬 경로: `J:\KHT\ProjectList_J\google-calendar-widget`

## 1. 목적

Google Calendar와 Google Tasks는 Windows 위젯을 공식 지원하지 않는다.
바탕화면에 항상 붙어 있는 가벼운 위젯으로 일정과 할 일을 한눈에 확인하고,
할 일은 위젯에서 바로 완료 처리할 수 있게 한다.

### 사용자 / 범위
- 개인용: 본인 PC 1대, Google 계정 1개.
- 외부 배포(구글 앱 검증, 코드 서명, 자동 업데이트)는 범위 밖이다.

### 성공 기준
1. 로그인 후 재부팅해도 추가 로그인 없이 위젯이 바탕화면에 자동으로 뜬다.
2. 월간 달력에 켜 둔 모든 캘린더의 일정이 캘린더 색으로 표시되고, 날짜를 클릭하면 그날 일정 목록이 나온다.
3. 할 일 목록에서 체크하면 Google Tasks에 완료로 반영된다 (웹에서 확인 가능).
4. 보기 모드(캘린더만 / 할 일만 / 둘 다), 창 크기·위치가 재시작 후에도 유지된다.
5. Win+D(바탕화면 보기)를 눌러도 위젯이 보이고, 일반 창들보다 항상 뒤에 있다.
6. 상시 실행 기준 메모리 사용량 약 100MB 이하.

## 2. 요구사항

### 기능
| ID | 내용 |
|----|------|
| F1 | Google OAuth 로그인 (브라우저, PKCE, loopback) / 로그아웃 |
| F2 | 월간 달력(일요일 시작, 6주 격자), 이전/다음 달 이동, 오늘 강조, 일정 있는 날 캘린더 색 점 표시 |
| F3 | 날짜 선택 시 해당 날짜 일정 목록 (종일 일정 우선, 이후 시간순) |
| F4 | 할 일 목록: 켜 둔 Task 목록별로 미완료 할 일 표시, 마감일 표시 |
| F5 | 할 일 완료 체크 (낙관적 업데이트, 3초 내 취소 가능, 실패 시 롤백) |
| F6 | 마감일 있는 할 일은 월간 달력 해당 날짜에도 표시 |
| F7 | 보기 모드 전환: 캘린더만 / 할 일만 / 둘 다 |
| F8 | 창 크기 조절·이동, 설정 영속화 |
| F9 | 설정: 캘린더·Task 목록별 표시 on/off (기본 전부 on) |
| F10 | 바탕화면 고정 (항상 일반 창보다 뒤, 작업표시줄 미표시) |
| F11 | 트레이 아이콘: 새로고침 / 설정 / 위치 잠금 / 로그아웃 / 종료 |
| F12 | Windows 시작 시 자동 실행 (설정에서 on/off) |

### 비기능
- 일정 편집·할 일 추가는 하지 않는다 (필요 시 트레이 메뉴 "브라우저에서 열기").
- 토큰은 Windows 자격 증명 관리자에만 저장하고, 프론트엔드에 노출하지 않는다.
- OAuth 클라이언트 ID/시크릿은 저장소에 커밋하지 않는다 (빌드 시 환경 변수로 주입).

## 3. 기술 스택
- **Tauri 2** (Rust 백엔드 + WebView2)
- 프론트엔드: **TypeScript + Vite**, 프레임워크 없음 (화면 요소가 적어 불필요)
- 테스트: Rust `cargo test` (+ `wiremock`), 프론트 **Vitest**
- 주요 crate: `oauth2`, `reqwest`, `keyring`, `serde`, `chrono`, `windows` (Win32), Tauri 플러그인 `autostart`, `window-state`, `single-instance`

## 4. 아키텍처

```
┌──────────── Tauri 앱 (gcal-widget) ─────────────┐
│  Frontend (TypeScript + Vite)                     │
│   ├─ MonthCalendar   월 격자 + 선택일 일정         │
│   ├─ TaskList        목록별 할 일 + 완료 체크       │
│   ├─ ViewSwitcher    캘린더 / 할 일 / 둘 다         │
│   └─ Settings        캘린더·목록 on/off, 자동 실행  │
│            │ invoke() / event                       │
│  Rust Backend                                       │
│   ├─ auth     OAuth(PKCE) 로그인·토큰 갱신          │
│   │           → keyring (Windows Credential Manager)│
│   ├─ google   Calendar / Tasks API 클라이언트        │
│   ├─ cache    마지막 응답 로컬 캐시                  │
│   ├─ desktop  바탕화면 고정 (Win32)                 │
│   └─ tray     트레이 메뉴                            │
│  설정: %APPDATA%\kr.taekit93.gcalwidget\settings.json│
│  캐시: %APPDATA%\kr.taekit93.gcalwidget\cache.json   │
└──────────────────────────────────────────────────┘
```

### 디렉토리 구조 (예정)
```
google-calendar-widget/
├─ src/                    # 프론트엔드
│  ├─ main.ts
│  ├─ api.ts               # invoke 래퍼 (타입 정의)
│  ├─ state.ts             # 앱 상태 + 설정
│  ├─ calendar/month-grid.ts   # 순수 함수: 격자·일정 배치 계산
│  ├─ calendar/MonthCalendar.ts
│  ├─ tasks/TaskList.ts
│  ├─ ViewSwitcher.ts
│  ├─ Settings.ts
│  └─ styles.css
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs / lib.rs
│  │  ├─ auth.rs
│  │  ├─ google/{mod.rs, calendar.rs, tasks.rs, models.rs}
│  │  ├─ cache.rs
│  │  ├─ settings.rs
│  │  ├─ desktop.rs
│  │  └─ tray.rs
│  └─ tauri.conf.json
├─ .env.example            # GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET
└─ docs/
```

### 모듈 경계
- **auth**: `login()`, `logout()`, `access_token() -> Result<String>` (필요 시 자동 갱신). 다른 모듈은 토큰 저장 방식을 모른다.
- **google**: `list_calendars()`, `list_events(cal_id, time_min, time_max)`, `list_tasklists()`, `list_tasks(list_id)`, `set_task_completed(list_id, task_id, bool)`. 응답을 앱 모델(`Calendar`, `Event`, `TaskList`, `Task`)로 변환해 반환한다.
- **Tauri commands** (프론트에 노출): `auth_status`, `login`, `logout`, `get_month(year, month)`, `get_tasks()`, `set_task_completed`, `get_settings`, `save_settings`.
- **month-grid.ts**: DOM 없이 순수 함수로 작성한다 (테스트 대상).

## 5. 데이터 흐름

### 로그인
1. 토큰이 없으면 위젯에 "Google 로그인" 버튼을 표시한다.
2. 앱이 `127.0.0.1:<임의 포트>`에 일회성 HTTP 리스너를 열고, 기본 브라우저로 동의 화면을 연다 (PKCE + state 검증).
3. 인증 코드를 교환한 뒤 refresh token을 keyring에 저장한다. access token은 메모리에만 둔다.
4. 스코프:
   - ~~`https://www.googleapis.com/auth/calendar.readonly`~~ → 공개 전 검토에서 최소 권한으로 변경: `calendar.calendarlist.readonly`(calendarList.list) + `calendar.events.readonly`(events.list)
   - `https://www.googleapis.com/auth/tasks` (완료 처리에 쓰기 권한 필요)

### 데이터 조회
- **캘린더**: `calendarList.list`로 캘린더 목록과 색을 가져온 뒤, 켜 둔 캘린더마다 `events.list(singleEvents=true, orderBy=startTime, timeMin/timeMax=격자 표시 범위)`를 호출한다. 격자에 걸치는 앞뒤 주를 포함한 6주 범위다.
- **할 일**: `tasklists.list`로 목록을 가져온 뒤, 켜 둔 목록마다 `tasks.list(showCompleted=false, showHidden=false)`를 호출한다. 페이지네이션을 처리한다.
- 여러 캘린더와 목록은 병렬로 호출한다.
- **새로고침**: 10분 주기, 월 이동 시, 절전 복귀 시, 트레이 "새로고침" 시.
- **캐시**: 성공한 응답을 `cache.json`에 저장해 두고, 시작 즉시 캐시로 먼저 그린 뒤 최신 데이터로 갱신한다.

### 할 일 완료
1. 체크 즉시 UI에 완료 표시 (취소선).
2. `tasks.patch(status=completed)`를 호출한다.
3. 성공하면 3초 후 목록에서 제거한다. 3초 안에 다시 체크하면 `status=needsAction`으로 되돌린다.
4. 실패하면 체크를 롤백하고 토스트로 오류를 표시한다.

### 시간대
- 모든 날짜 계산은 로컬 시간대 기준이다.
- 종일 일정은 `date` 필드(종료일 exclusive)로 처리하고, 여러 날에 걸친 일정은 격자의 각 날짜에 표시한다.

## 6. 바탕화면 고정 (desktop.rs)
- 창 속성: `decorations: false`, `transparent: true`, `skipTaskbar: true`, `resizable: true`.
- 기본 방식 (Rainmeter "On Desktop" 방식):
  - 창을 서브클래싱해 `WM_WINDOWPOSCHANGING`에서 z-order를 항상 `HWND_BOTTOM`으로 강제한다. 클릭해도 다른 창 위로 올라오지 않는다.
  - `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`로 전경 창을 감시한다. 바탕화면(`Progman`/`WorkerW`)이 전경이 되면(Win+D, 바탕화면 클릭) 위젯을 잠시 최상위로 올리고, 다른 창이 전경이 되면 다시 맨 아래로 보낸다.
- 채택하지 않은 방식: WorkerW 자식으로 붙이는 방식. Windows 11 24H2 이후 WorkerW가 Progman 안, 아이콘 레이어(SHELLDLL_DefView) 아래로 이동해서 위젯이 클릭을 받지 못할 가능성이 높다.
- 이동·크기 조절: 상단 드래그 영역 + 가장자리 리사이즈. 트레이 "위치 잠금" 시 비활성화한다.
- 구현 단계에서 Windows 11 26200에서 수동 체크리스트로 검증한다.

## 7. 설정 (`%APPDATA%\kr.taekit93.gcalwidget\settings.json`)
```json
{
  "viewMode": "both",            // "calendar" | "tasks" | "both"
  "hiddenCalendars": [],         // calendar id 목록
  "hiddenTaskLists": [],
  "refreshMinutes": 10,
  "autostart": true,
  "locked": false
}
```
(주석은 설명용이다. 실제 파일은 순수 JSON이다.)
- 창 크기·위치는 `tauri-plugin-window-state`가 관리한다.
- 설정은 기본값 전부 표시로 시작하고, 새로 생긴 캘린더는 자동으로 표시한다 ("숨긴 목록" 방식이라서).

## 8. 오류 처리
| 상황 | 동작 |
|---|---|
| 네트워크 끊김 | 캐시 데이터 유지, 상단에 "오프라인 · 마지막 갱신 HH:MM" 표시, 연결 복구 시 자동 재시도 |
| access token 만료 | refresh token으로 조용히 갱신 |
| refresh 실패 (`invalid_grant`: 권한 철회, 만료) | keyring 삭제 → 로그인 화면 |
| 429 / 5xx | 지수 백오프, 최대 3회 재시도 |
| 캘린더 1개 조회 실패 | 나머지는 표시하고 해당 캘린더만 경고 아이콘 표시 |
| 바탕화면 고정 실패 | HWND_BOTTOM 방식으로 대체하고 로그 기록 |
| 중복 실행 | single-instance 플러그인으로 기존 창에 포커스 |

로그: `%LOCALAPPDATA%\kr.taekit93.gcalwidget\logs\` (1MB 초과 시 교체, 1개 보관).

## 9. Google Cloud 설정 (사전 준비)
1. Google Cloud Console에서 프로젝트를 생성하고 Calendar API, Tasks API를 활성화한다.
2. OAuth 동의 화면: User type "외부", 테스트 사용자에 본인 계정을 추가한다.
3. 사용자 인증 정보에서 OAuth 클라이언트 ID(유형: **데스크톱 앱**)를 만들고, 값을 `.env`에 넣는다 (git 제외).
4. **테스트 모드의 7일 만료 문제**: 외부·테스트 상태의 OAuth 앱은 refresh token이 7일 후 만료된다. 해결책은 둘 중 하나다.
   - 계정이 Google Workspace 조직 계정이면 User type을 **"내부"**로 설정한다. 검증과 만료가 없다.
   - 개인 Gmail이면 앱을 **"프로덕션으로 게시"**하고 검증을 받지 않는다. 로그인 시 "확인되지 않은 앱" 경고가 한 번 뜨지만 본인 사용에는 문제가 없고, 토큰이 만료되지 않는다.

## 10. 테스트
- **Rust 단위 테스트**
  - 토큰 갱신 흐름 (만료 → 갱신 → 재시도, `invalid_grant` → 로그아웃 상태)
  - API 응답 → 모델 변환 (종일/시간 일정, 페이지네이션)
  - 월 → 조회 범위(timeMin/timeMax) 계산
  - `wiremock`으로 Google API를 모킹한다.
- **프론트 Vitest**
  - `month-grid`: 월 시작 요일, 5주/6주 달, 종일·다일 일정 배치, 윤년
  - 완료 체크: 성공 / 실패 롤백 / 3초 내 취소
- **수동 체크리스트**: 첫 로그인, 재부팅 후 자동 실행, Win+D, 절전 복귀, 네트워크 차단, 보기 모드·크기 복원, 캘린더 on/off.

## 11. 범위 밖 (YAGNI)
- 일정 생성·수정, 할 일 추가·편집
- 여러 Google 계정
- 외부 배포 (구글 앱 검증, 코드 서명, 자동 업데이트, 설치 페이지)
- 알림 / 리마인더
- 주간·일간 뷰

## 12. 개발 환경 준비
- 설치 확인됨: Node 22, npm, git, WebView2, gh (로그인됨)
- 설치 필요: **rustup (stable, MSVC)**, **Visual Studio 2022 Build Tools (C++ 데스크톱 워크로드)**
