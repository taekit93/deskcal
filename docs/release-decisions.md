# 공개·승인 과정의 결정 기록

공개(GitHub)부터 Google OAuth 검증 신청까지 진행하면서 내린 결정과 이유를 적습니다. 작성일 2026-09-30.

| # | 결정 | 이유 |
|---|---|---|
| 1 | Google Cloud 프로젝트를 개인 계정(taekit93@gmail.com)에 새로 만들고, 조직 없이 둠 (프로젝트 ID `deskcal-510213`) | 공개 앱은 회사 Workspace 정책의 영향을 받지 않아야 함. "조직 없음"이어야 사용자 유형을 "외부"로 쓸 수 있음 |
| 2 | 캘린더 권한을 `calendar.calendarlist.readonly` + `calendar.events.readonly`로 좁힘 | 앱이 호출하는 API(calendarList.list, events.list)를 모두 쓸 수 있는 최소 권한. `calendarlist.readonly`는 민감하지 않은 권한으로 분류되어 검증 부담이 줄어듦 |
| 3 | Tasks 권한은 `tasks` 유지 | 완료 처리(tasks.patch)는 `tasks` 권한에서만 됨. `tasks.readonly`로는 핵심 기능이 안 됨 |
| 4 | OAuth 클라이언트 유형: 데스크톱 앱, 이름 "DeskCal Windows" | 설치형 앱의 표준 유형. PKCE + loopback 리디렉션을 씀 |
| 5 | 이전 클라이언트 값은 `.env.previous`로 보관 (git 제외) | 되돌려야 할 때를 대비 |
| 6 | 테스트 사용자에 taekit93@gmail.com, htkim@nanoit.kr 추가 | 프로덕션 게시 전에도 두 계정으로 새 클라이언트를 쓸 수 있게 |
| 7 | 이전 클라이언트 토큰이 `unauthorized_client`로 거절되면 로그인 만료로 처리 | 클라이언트를 바꾼 뒤에도 사용자가 로그인 화면으로 자연스럽게 돌아오게 |
