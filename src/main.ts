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
import { applyTheme } from "./theme";

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

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function applyAppearance(s: Settings) {
  applyTheme(app, s, darkQuery.matches);
  app.dataset.header = s.headerMode;
  app.dataset.border = String(s.showBorder);
}

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
  // 이전 계정의 일정·할 일이 설정 창이나 다음 로그인 화면에 남지 않게 비운다.
  state.monthData = null;
  state.tasksData = null;
  completion.reset();
  calEl.innerHTML = "";
  tasksEl.innerHTML = "";
  statusEl.textContent = "";
  overlay.hidden = true;
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
    showDue: s.showDue,
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
    showDayDetail: s.showDayDetail,
    showTaskDots: s.showTaskDots,
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
    if (!state.loggedIn) return;
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
  const { year, month } = state;
  try {
    const [m, t] = await Promise.all([api.getMonth(year, month), api.getTasks()]);
    if (!state.loggedIn) return; // 응답 전에 로그아웃했으면 버린다
    // 응답 전에 다른 달로 이동했으면 그 달 데이터는 loadMonth가 채운다.
    if (state.year === year && state.month === month) state.monthData = m;
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
  for (const el of [header, statusEl, $("hotspot")]) {
    if (locked) el.removeAttribute("data-tauri-drag-region");
    else el.setAttribute("data-tauri-drag-region", "");
  }
  app.dataset.locked = String(locked);
  const pin = $("btn-pin");
  pin.setAttribute("aria-pressed", String(locked));
  pin.title = locked ? "위치·크기 고정 해제" : "위치·크기 고정";
  getCurrentWindow().setResizable(!locked).catch(() => {});
}

let refreshTimer = 0;
function resetTimer() {
  clearInterval(refreshTimer);
  refreshTimer = window.setInterval(() => void refreshAll(), state.settings!.refreshMinutes * 60_000);
}

async function saveSettings(next: Settings) {
  try {
    const saved = await api.saveSettings(next); // 백엔드가 검증한 값을 기준으로 삼는다
    state.settings = saved;
    applyAppearance(saved);
    applyLock(saved.locked);
    resetTimer();
    render();
  } catch (e) {
    toast(`설정 저장 실패: ${errorText(e)}`);
  }
}

function openSettings() {
  if (!state.settings) return;
  const original = state.settings;
  const restore = () => {
    state.settings = original;
    applyAppearance(original);
    render();
  };
  overlay.hidden = false;
  renderSettings(overlay, {
    settings: original,
    onPreview: (draft) => {
      state.settings = draft;
      applyAppearance(draft);
      render();
    },
    calendars: state.monthData?.calendars ?? [],
    lists: state.tasksData?.lists ?? [],
    onSave: async (next) => {
      overlay.hidden = true;
      state.settings = original;
      await saveSettings(next);
      await refreshAll();
    },
    onClose: () => {
      overlay.hidden = true;
      restore();
    },
    onLogout: async () => {
      overlay.hidden = true;
      restore();
      try {
        await api.logout();
        showLogin();
      } catch (e) {
        toast(`로그아웃 실패: ${errorText(e)}`);
      }
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
  applyAppearance(state.settings);
  darkQuery.addEventListener("change", () => state.settings && applyAppearance(state.settings));
  applyLock(state.settings.locked);
  resetTimer();
  loginBtn.onclick = () => void doLogin();
  $("btn-refresh").onclick = () => void refreshAll();
  $("btn-pin").onclick = async () => {
    try {
      const s = await api.toggleLock();
      state.settings = s;
      applyLock(s.locked);
    } catch (e) {
      toast(`고정 상태 변경 실패: ${errorText(e)}`);
    }
  };
  $("btn-settings").onclick = openSettings;

  await listen("refresh", () => void refreshAll());
  await listen("open-settings", openSettings);
  await listen<Settings>("settings-changed", (e) => {
    state.settings = e.payload;
    applyAppearance(e.payload);
    applyLock(e.payload.locked);
    render();
  });
  await listen("logged-out", () => showLogin());
  await listen<string>("logout-failed", (e) => toast(`로그아웃 실패: ${e.payload}`));

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
