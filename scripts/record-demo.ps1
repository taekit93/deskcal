# Google OAuth 검증용 시연 영상 녹화 (전체 화면, 30fps, mp4)
# 사용법: powershell -ExecutionPolicy Bypass -File scripts\record-demo.ps1
# 녹화를 멈추려면 이 창에서 q 를 누르세요. 결과: .\deskcal-demo.mp4
# 필요: ffmpeg (https://www.gyan.dev/ffmpeg/builds/)

$out = Join-Path (Get-Location) "deskcal-demo.mp4"
Write-Host "녹화를 시작합니다. 촬영 순서는 docs/google-verification.md 4장을 참고하세요. 멈추려면 q"
ffmpeg -y -f gdigrab -framerate 30 -draw_mouse 1 -i desktop `
  -c:v libx264 -preset veryfast -crf 23 -pix_fmt yuv420p $out
Write-Host "저장됨: $out  → YouTube에 '일부 공개'로 업로드한 뒤 링크를 인증 신청서에 넣으세요."
