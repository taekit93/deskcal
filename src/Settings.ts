import type { Calendar, Settings, TaskList } from "./api";
import { esc } from "./dom";
import { ACCENT_PRESETS, THEMES, THEME_LABELS, type FontScale, type ThemeName } from "./theme";

export interface SettingsViewProps {
  settings: Settings;
  calendars: Calendar[];
  lists: TaskList[];
  /** 모양 변경을 저장 전에 미리 보여준다. */
  onPreview(draft: Settings): void;
  onSave(next: Settings): void;
  /** 취소. 호출한 쪽이 미리보기를 원래 설정으로 되돌린다. */
  onClose(): void;
  onLogout(): void;
}

type Tab = "look" | "data" | "general";
const TABS: [Tab, string][] = [["look", "모양"], ["data", "캘린더·할 일"], ["general", "일반"]];
const REFRESH_OPTIONS = [5, 10, 15, 30];
const FONT_OPTIONS: [FontScale, string][] = [["small", "작게"], ["normal", "보통"], ["large", "크게"]];
const HEADER_OPTIONS: [Settings["headerMode"], string][] = [["always", "항상 보이기"], ["hover", "마우스 올리면"]];
const TOGGLES: [keyof Settings, string][] = [
  ["showDayDetail", "선택한 날 일정 목록"],
  ["showDue", "할 일 마감일"],
  ["showTaskDots", "달력에 할 일 표시"],
  ["showBorder", "위젯 테두리"],
];

function swatch(name: ThemeName): string {
  if (name === "system") {
    const d = THEMES.dark, l = THEMES.light;
    return `linear-gradient(135deg, rgb(${d.bg.join(",")}) 50%, rgb(${l.bg.join(",")}) 50%)`;
  }
  const t = THEMES[name];
  return `linear-gradient(135deg, rgb(${t.bg.join(",")}) 60%, ${t.accent} 60%)`;
}

function segmented<T extends string>(name: string, options: [T, string][], current: T): string {
  return `<div class="seg">${options.map(([v, label]) =>
    `<button class="${v === current ? "active" : ""}" data-${name}="${v}">${label}</button>`).join("")}</div>`;
}

export function renderSettings(el: HTMLElement, p: SettingsViewProps): void {
  const draft: Settings = { ...p.settings };
  let tab: Tab = "look";

  const lookTab = () => `
    <h3>테마</h3>
    <div class="themes">${(Object.keys(THEME_LABELS) as ThemeName[]).map((n) => `
      <button class="theme-opt ${draft.theme === n ? "active" : ""}" data-theme-opt="${n}">
        <i class="swatch" style="background:${swatch(n)}"></i>${THEME_LABELS[n]}
      </button>`).join("")}</div>
    <h3>강조색</h3>
    <div class="accents">
      <button class="accent-opt default ${draft.accent === null ? "active" : ""}" data-accent="" title="테마 기본">기본</button>
      ${ACCENT_PRESETS.map((c) =>
        `<button class="accent-opt ${draft.accent === c ? "active" : ""}" data-accent="${c}" style="background:${c}" title="${c}"></button>`).join("")}
      <label class="accent-custom" title="직접 고르기"><input type="color" id="accent-custom" value="${esc(draft.accent ?? "#8ab4f8")}"></label>
    </div>
    <h3>배경 투명도 <span id="opacity-val" class="muted">${Math.round(draft.opacity * 100)}%</span></h3>
    <input type="range" id="opacity" min="30" max="100" step="1" value="${Math.round(draft.opacity * 100)}">
    <h3>글자 크기</h3>${segmented("font", FONT_OPTIONS, draft.fontScale)}
    <h3>상단 바</h3>${segmented("header", HEADER_OPTIONS, draft.headerMode)}
    <h3>표시 항목</h3>
    ${TOGGLES.map(([k, label]) =>
      `<label class="row"><input type="checkbox" data-toggle="${k}" ${draft[k] ? "checked" : ""}>${label}</label>`).join("")}`;

  const dataTab = () => {
    const calRows = p.calendars.map((c) => `
      <label class="row"><input type="checkbox" data-cal="${esc(c.id)}" ${draft.hiddenCalendars.includes(c.id) ? "" : "checked"}>
      <i class="dot" style="background:${esc(c.color)}"></i><span class="title">${esc(c.summary)}</span></label>`).join("");
    const listRows = p.lists.map((l) => `
      <label class="row"><input type="checkbox" data-list="${esc(l.id)}" ${draft.hiddenTaskLists.includes(l.id) ? "" : "checked"}>
      <span class="title">${esc(l.title)}</span></label>`).join("");
    return `
      <h3>캘린더</h3>${calRows || `<p class="empty">불러온 캘린더가 없습니다</p>`}
      <h3>할 일 목록</h3>${listRows || `<p class="empty">불러온 목록이 없습니다</p>`}`;
  };

  const generalTab = () => `
    <label class="row">새로고침 간격
      <select id="refresh">${REFRESH_OPTIONS.map((m) =>
        `<option value="${m}" ${m === draft.refreshMinutes ? "selected" : ""}>${m}분</option>`).join("")}</select>
    </label>
    <label class="row"><input type="checkbox" id="autostart" ${draft.autostart ? "checked" : ""}>Windows 시작 시 자동 실행</label>
    <p class="muted hint">위젯 종료·위치 잠금은 트레이 아이콘 메뉴에서 할 수 있습니다.</p>
    <button id="logout" class="ghost danger">로그아웃</button>`;

  const preview = () => p.onPreview({ ...draft });

  function draw() {
    const body = tab === "look" ? lookTab() : tab === "data" ? dataTab() : generalTab();
    el.innerHTML = `
      <div class="panel">
        <div class="panel-head">
          <h2>설정</h2>
          <div class="tabs">${TABS.map(([t, label]) =>
            `<button class="${t === tab ? "active" : ""}" data-tab="${t}">${label}</button>`).join("")}</div>
        </div>
        <div class="panel-body">${body}</div>
        <div class="actions">
          <span class="spacer"></span>
          <button id="cancel" class="ghost">취소</button>
          <button id="save" class="primary">저장</button>
        </div>
      </div>`;
    wire();
  }

  const all = <T extends HTMLElement>(sel: string) => [...el.querySelectorAll<T>(sel)];

  function wire() {
    all<HTMLButtonElement>("[data-tab]").forEach((b) => (b.onclick = () => { tab = b.dataset.tab as Tab; draw(); }));
    all<HTMLButtonElement>("[data-theme-opt]").forEach((b) => (b.onclick = () => {
      draft.theme = b.dataset.themeOpt as ThemeName; preview(); draw();
    }));
    all<HTMLButtonElement>("[data-accent]").forEach((b) => (b.onclick = () => {
      draft.accent = b.dataset.accent || null; preview(); draw();
    }));
    const custom = el.querySelector<HTMLInputElement>("#accent-custom");
    if (custom) custom.oninput = () => { draft.accent = custom.value.toLowerCase(); preview(); };
    if (custom) custom.onchange = () => draw();
    const opacity = el.querySelector<HTMLInputElement>("#opacity");
    if (opacity) opacity.oninput = () => {
      draft.opacity = Number(opacity.value) / 100;
      el.querySelector("#opacity-val")!.textContent = `${opacity.value}%`;
      preview();
    };
    all<HTMLButtonElement>("[data-font]").forEach((b) => (b.onclick = () => {
      draft.fontScale = b.dataset.font as FontScale; preview(); draw();
    }));
    all<HTMLButtonElement>("[data-header]").forEach((b) => (b.onclick = () => {
      draft.headerMode = b.dataset.header as Settings["headerMode"]; preview(); draw();
    }));
    all<HTMLInputElement>("[data-toggle]").forEach((cb) => (cb.onchange = () => {
      (draft as unknown as Record<string, boolean>)[cb.dataset.toggle!] = cb.checked; preview();
    }));
    all<HTMLInputElement>("[data-cal]").forEach((cb) => (cb.onchange = () => {
      draft.hiddenCalendars = all<HTMLInputElement>("input[data-cal]").filter((i) => !i.checked).map((i) => i.dataset.cal!);
    }));
    all<HTMLInputElement>("[data-list]").forEach((cb) => (cb.onchange = () => {
      draft.hiddenTaskLists = all<HTMLInputElement>("input[data-list]").filter((i) => !i.checked).map((i) => i.dataset.list!);
    }));
    const refresh = el.querySelector<HTMLSelectElement>("#refresh");
    if (refresh) refresh.onchange = () => (draft.refreshMinutes = Number(refresh.value));
    const autostart = el.querySelector<HTMLInputElement>("#autostart");
    if (autostart) autostart.onchange = () => (draft.autostart = autostart.checked);
    const logout = el.querySelector<HTMLButtonElement>("#logout");
    if (logout) logout.onclick = () => p.onLogout();
    el.querySelector<HTMLButtonElement>("#cancel")!.onclick = () => p.onClose();
    el.querySelector<HTMLButtonElement>("#save")!.onclick = () => p.onSave({ ...draft });
  }

  draw();
}
