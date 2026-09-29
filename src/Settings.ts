import type { Calendar, Settings, TaskList } from "./api";
import { esc } from "./dom";

export interface SettingsViewProps {
  settings: Settings;
  calendars: Calendar[];
  lists: TaskList[];
  onSave(next: Settings): void;
  onClose(): void;
  onLogout(): void;
}

const REFRESH_OPTIONS = [5, 10, 15, 30];

export function renderSettings(el: HTMLElement, p: SettingsViewProps): void {
  const s = p.settings;
  const calRows = p.calendars.map((c) => `
    <label class="row"><input type="checkbox" data-cal="${esc(c.id)}" ${s.hiddenCalendars.includes(c.id) ? "" : "checked"}>
    <i class="dot" style="background:${esc(c.color)}"></i><span class="title">${esc(c.summary)}</span></label>`).join("");
  const listRows = p.lists.map((l) => `
    <label class="row"><input type="checkbox" data-list="${esc(l.id)}" ${s.hiddenTaskLists.includes(l.id) ? "" : "checked"}>
    <span class="title">${esc(l.title)}</span></label>`).join("");

  el.innerHTML = `
    <div class="panel">
      <h2>설정</h2>
      <h3>캘린더</h3>${calRows || `<p class="empty">불러온 캘린더가 없습니다</p>`}
      <h3>할 일 목록</h3>${listRows || `<p class="empty">불러온 목록이 없습니다</p>`}
      <h3>일반</h3>
      <label class="row">새로고침 간격
        <select id="refresh">${REFRESH_OPTIONS.map((m) =>
          `<option value="${m}" ${m === s.refreshMinutes ? "selected" : ""}>${m}분</option>`).join("")}</select>
      </label>
      <label class="row"><input type="checkbox" id="autostart" ${s.autostart ? "checked" : ""}>Windows 시작 시 자동 실행</label>
      <div class="actions">
        <button id="logout" class="ghost">로그아웃</button>
        <span class="spacer"></span>
        <button id="cancel" class="ghost">취소</button>
        <button id="save">저장</button>
      </div>
    </div>`;

  const unchecked = (attr: string) =>
    [...el.querySelectorAll<HTMLInputElement>(`input[data-${attr}]`)]
      .filter((i) => !i.checked)
      .map((i) => i.dataset[attr]!);

  el.querySelector<HTMLButtonElement>("#save")!.onclick = () => p.onSave({
    ...s,
    hiddenCalendars: unchecked("cal"),
    hiddenTaskLists: unchecked("list"),
    refreshMinutes: Number(el.querySelector<HTMLSelectElement>("#refresh")!.value),
    autostart: el.querySelector<HTMLInputElement>("#autostart")!.checked,
  });
  el.querySelector<HTMLButtonElement>("#cancel")!.onclick = () => p.onClose();
  el.querySelector<HTMLButtonElement>("#logout")!.onclick = () => p.onLogout();
}
