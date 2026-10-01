# Google OAuth 앱 검증 준비

## 현재 상태 (2026-09-30)

| 항목 | 상태 |
|---|---|
| Google Cloud 프로젝트 | `deskcal-510213` (taekit93@gmail.com, 조직 없음) |
| API | Google Calendar API, Google Tasks API 사용 설정 완료 |
| OAuth 동의 화면 | 외부, **프로덕션 게시 완료**, 앱 이름 DeskCal, 로고 등록 |
| 스코프 | `calendar.calendarlist.readonly`(민감하지 않음), `calendar.events.readonly`(민감), `tasks`(민감) |
| OAuth 클라이언트 | 데스크톱 앱 "DeskCal Windows" (값은 `.env`, git 제외) |
| 홈페이지 / 개인정보처리방침 | https://taekit93.github.io/deskcal/ , https://taekit93.github.io/deskcal/privacy.html |
| 승인된 도메인 | taekit93.github.io |
| Search Console | `https://taekit93.github.io/`(HTML 파일, 저장소 `taekit93/taekit93.github.io`)와 `https://taekit93.github.io/deskcal/` 소유권 확인 완료 |
| 브랜드 검증 | **대기**: 2026-10-01 재시도도 같은 사유로 거절. `/deskcal/` 직접 확인(HTML 파일)을 추가했으므로 2026-10-03 00:00 이후 재시도 |
| 데이터 액세스 검증 | **대기**: 브랜드 검증 통과 후 신청 가능, 시연 영상 준비됨: https://youtu.be/tBsj6BP2uBM |

## 남은 순서

1. **2026-10-01 이후** Google Cloud 콘솔 → Google 인증 플랫폼 → **브랜딩** → 인증 상태의 "문제 보기" → "문제를 해결함" → 계속 (브랜딩 재인증 요청)
2. ~~시연 영상 녹화·업로드~~ 완료: https://youtu.be/tBsj6BP2uBM (공개)
3. **인증 센터** → 데이터 액세스 인증 신청: 3장의 스코프별 설명과 데이터 처리 요약을 붙여 넣고 영상 링크 입력
4. 심사 메일에 답변 (보통 며칠~몇 주)

---


DeskCal은 민감한 스코프(`calendar.calendarlist.readonly`, `calendar.events.readonly`, `tasks`)를 쓰므로, 사용자 수 제한(100명)과 "확인되지 않은 앱" 경고를 없애려면 Google 검증이 필요합니다. 이 문서는 신청에 필요한 자료와 순서를 정리합니다.

## 전제

- 저장소와 소개 페이지가 공개되어 있어야 합니다.
  - 홈페이지: `https://taekit93.github.io/deskcal/`
  - 개인정보처리방침: `https://taekit93.github.io/deskcal/privacy.html`
- OAuth 동의 화면 사용자 유형: **외부**

## 1. 도메인 소유 확인 (Search Console)

1. [Google Search Console](https://search.google.com/search-console)에서 **URL 접두어** 속성으로 `https://taekit93.github.io/`를 추가합니다.
   - GitHub Pages 프로젝트 사이트(`/deskcal/`)는 `taekit93.github.io` 도메인 아래에 있으므로, 도메인 루트를 확인해야 합니다.
2. 확인 방법으로 **HTML 파일**을 고르고, 받은 `google….html` 파일을 사용자 사이트 저장소 `taekit93/taekit93.github.io`의 루트에 올립니다.
   - 루트(`/`)에 파일을 두려면 사용자 사이트 저장소가 필요합니다. 저장소 이름이 정확히 `taekit93.github.io`여야 합니다.
   - 또는 **HTML 태그** 방식으로 사용자 사이트의 `index.html` `<head>`에 메타 태그를 넣어도 됩니다.
3. 확인이 끝나면 Google Cloud 콘솔 → OAuth 동의 화면 → **승인된 도메인**에 `taekit93.github.io`를 추가합니다.
   - 같은 Google 계정으로 Search Console과 Cloud 콘솔에 로그인해야 합니다.

## 2. 동의 화면(브랜딩) 입력값

| 항목 | 값 |
|---|---|
| 앱 이름 | DeskCal |
| 사용자 지원 이메일 | (본인 이메일) |
| 앱 로고 | `assets/icon.png` (120×120 이상 정사각형 PNG. 로고를 올리면 추가 검토가 붙을 수 있음) |
| 애플리케이션 홈페이지 | https://taekit93.github.io/deskcal/ |
| 개인정보처리방침 링크 | https://taekit93.github.io/deskcal/privacy.html |
| 승인된 도메인 | taekit93.github.io |
| 개발자 연락처 | (본인 이메일) |

## 3. 스코프와 사용 이유 (영문, 신청 양식에 그대로 사용)

**`https://www.googleapis.com/auth/calendar.calendarlist.readonly`**

> DeskCal is a Windows desktop widget that shows the user's Google Calendar as a month grid pinned to their desktop. We use calendar.calendarlist.readonly only to call calendarList.list, which gives the names and colors of the calendars the user subscribes to. The widget uses the colors to mark each event's calendar and lets the user show or hide individual calendars in its settings. The app never adds or removes calendars.

**`https://www.googleapis.com/auth/calendar.events.readonly`**

> We use calendar.events.readonly only to call events.list for each visible calendar and the month currently shown, so the widget can mark days that have events and list the selected day's events in time order. The app never creates, modifies or deletes events. We deliberately request these two narrow read-only scopes instead of calendar.readonly.

**`https://www.googleapis.com/auth/tasks`**

> DeskCal shows the user's Google Tasks lists next to the calendar and lets the user check off a task directly from the widget. Checking a task sends a PATCH request that sets its status to "completed" (and back to "needsAction" if the user undoes it within three seconds). This write access is the core interaction of the widget, so tasks.readonly is not sufficient. The app does not create or delete tasks.

**데이터 처리 요약 (Data handling)**

> DeskCal has no backend server. All requests go directly from the user's PC to Google APIs. The OAuth refresh token is stored in Windows Credential Manager, and fetched events and tasks are cached only on the user's PC (%APPDATA%) for offline display. No Google user data is transmitted to the developer or any third party, and the app contains no analytics or ads. DeskCal's use of information received from Google APIs adheres to the Google API Services User Data Policy, including the Limited Use requirements.

## 4. 시연 영상 (YouTube 일부 공개로 업로드)

길이 2~3분. 화면은 영어 UI가 가장 무난하지만, 한국어 UI라면 자막이나 내레이션으로 설명을 붙입니다.

1. **앱 소개 (10초):** 바탕화면의 DeskCal 위젯을 보여줍니다.
2. **OAuth 과정 (40초):** 로그아웃 상태에서 "Google 로그인"을 누르고 브라우저 동의 화면을 보여줍니다.
   - 동의 화면의 **앱 이름 DeskCal**과 **요청 스코프 두 가지**가 영상에 또렷이 보여야 합니다.
   - 주소창의 URL이 보이면 `client_id`가 신청한 앱과 같은지 심사자가 확인할 수 있습니다.
3. **캘린더 권한 사용 장면 (40초):** 달력에 일정 점이 표시되고, 날짜를 누르면 일정 목록이 나오는 장면. 설정에서 캘린더를 켜고 끄는 장면.
4. **`tasks` 사용 장면 (40초):** 할 일을 체크하면 3초 뒤 사라지고, 브라우저의 Google Tasks에서 완료로 바뀐 것을 보여줍니다. 체크 후 다시 눌러 취소하는 장면도 넣습니다.
5. **데이터 삭제 (20초):** 트레이 메뉴 → 로그아웃. myaccount.google.com/permissions에서 권한을 철회할 수 있다고 안내합니다.

## 5. 신청

Google Cloud 콘솔 → **Google 인증 플랫폼** → **데이터 액세스**에서 스코프를 확인하고, **인증 센터(Verification Center)**에서 신청합니다. 심사 중 이메일로 추가 질문이 올 수 있습니다. 보통 며칠에서 몇 주 걸립니다.

## 검증 전에 할 수 있는 것

- 동의 화면을 **프로덕션으로 게시**하면, 검증 전이라도 최대 100명이 경고 화면을 넘기고 로그인할 수 있습니다. 게시하면 refresh token이 7일 만에 만료되는 문제도 없어집니다.
