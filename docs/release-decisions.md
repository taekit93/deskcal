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
| 8 | 저장소 이름을 `deskcal`로 바꾸고 Public 전환, 설명·홈페이지 설정 | 앱 이름과 주소를 일치시킴. 공개 전 기록 전체에서 비밀값이 없음을 확인함 |
| 9 | GitHub Pages는 Actions 방식으로 켜고 수동 워크플로로 배포 | 데모를 빌드해야 해서 브랜치 정적 배포로는 안 됨 |
| 10 | 첫 릴리스 `v0.1.0`에 NSIS 설치 파일만 첨부, 코드 서명 없음 | 인증서는 유료. SmartScreen 경고 대처법을 릴리스 노트와 페이지에 안내 |
| 11 | Search Console은 "URL 접두어" 속성 `https://taekit93.github.io/`를 HTML 파일로 확인 | github.io 하위 도메인은 DNS(도메인 속성) 확인이 불가능. 루트에 파일을 두려고 사용자 사이트 저장소 `taekit93.github.io`를 만들고 `/deskcal/`로 이동하는 index를 둠 |
| 12 | `https://taekit93.github.io/deskcal/` 속성도 추가(상위 속성으로 자동 확인), DeskCal 사이트에도 같은 확인 파일 배치 | 브랜드 검증이 홈페이지 경로와 정확히 맞는 속성을 찾을 수 있게 |
| 13 | 동의 화면 로고로 120×120 PNG 업로드 | Google 권장 크기. 로고가 있으면 브랜드 검증이 필요하지만, 민감한 스코프 때문에 어차피 검증을 받아야 함 |
| 14 | 서비스 약관 URL은 비워 둠 | 필수 항목이 아니고, 서버 없는 무료 앱이라 별도 약관 대신 MIT 라이선스와 개인정보처리방침으로 충분 |
| 15 | 프로덕션 게시 진행 | 게시해야 100명 한도 안에서 외부 사용자가 쓸 수 있고, 7일 토큰 만료가 없어짐 |
| 16 | 브랜드 재인증은 24시간 뒤로 미룸 | Google이 소유권 확인 후 24시간 대기를 요구함. 지금 재시도해도 같은 사유로 거절됨 |
| 17 | 시연 영상 녹화·YouTube 업로드는 사용자에게 남김 | 로그인은 기본 브라우저의 새 창에서 이뤄져 자동화 범위 밖이고, 심사자는 실제 사용자 흐름을 봐야 함. 녹화 스크립트(`scripts/record-demo.ps1`)와 촬영 순서를 준비함 |
| 18 | 이 PC의 DeskCal을 새 클라이언트로 빌드한 버전으로 재설치 | 이전 토큰은 `unauthorized_client`로 거절되므로 앱을 켜면 로그인 화면이 나오고, 다시 로그인하면 새 권한으로 동의하게 됨 |
