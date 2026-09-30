import { defineConfig } from "vite";

// 소개 페이지의 라이브 데모: 위젯 화면을 예시 데이터로 빌드해 site/demo/ 에 둔다.
export default defineConfig({
  root: "demo",
  base: "./",
  build: { outDir: "../site/demo", emptyOutDir: true },
});
