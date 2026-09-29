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
