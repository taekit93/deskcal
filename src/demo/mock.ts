// 소개 페이지 데모: Tauri 백엔드 대신 예시 데이터로 응답한다.
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Settings } from "../api";
import { DEMO_SETTINGS, sampleMonth, sampleTasks } from "./sample-data";

let settings: Settings = { ...DEMO_SETTINGS };
const completed = new Set<string>();
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

mockWindows("main");
mockIPC(async (cmd, args) => {
  const a = (args ?? {}) as Record<string, unknown>;
  switch (cmd) {
    case "auth_status": return true;
    case "get_settings": return settings;
    case "save_settings": settings = a.settings as Settings; return settings;
    case "get_month": await wait(150); return sampleMonth(a.year as number, a.month as number, new Date());
    case "get_tasks": await wait(150); return sampleTasks(new Date(), completed);
    case "set_task_completed":
      await wait(250);
      if (a.completed) completed.add(a.taskId as string);
      else completed.delete(a.taskId as string);
      return null;
    default: return null; // peek_*, login, logout, plugin:window|* 등
  }
}, { shouldMockEvents: true });

// 소개 페이지(부모 창)의 테마 버튼 → 위젯 설정 변경
const THEMES = new Set(["system", "dark", "light", "midnight", "forest", "rose"]);
window.addEventListener("message", (e: MessageEvent) => {
  if (e.source !== window.parent) return; // 소개 페이지(부모 창)에서 온 메시지만
  const data = e.data as { type?: string; patch?: { theme?: unknown; opacity?: unknown } } | null;
  if (data?.type !== "deskcal:settings" || !data.patch) return;
  const { theme, opacity } = data.patch;
  const patch: Partial<Settings> = {};
  if (typeof theme === "string" && THEMES.has(theme)) patch.theme = theme as Settings["theme"];
  if (typeof opacity === "number" && opacity >= 0.3 && opacity <= 1) patch.opacity = opacity;
  settings = { ...settings, ...patch };
  void emit("settings-changed", settings);
});
