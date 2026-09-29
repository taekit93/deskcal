import type { CalEvent, Task } from "../api";
import { esc } from "../dom";
import { formatEventTime, type GridDay } from "./month-grid";

export interface MonthViewProps {
  year: number;
  month: number;
  grid: GridDay[];
  events: Map<string, CalEvent[]>;
  tasks: Map<string, Task[]>;
  colors: Map<string, string>;
  selected: string;
  onSelect(key: string): void;
  onNav(delta: -1 | 0 | 1): void;
}

const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];
const FALLBACK_COLOR = "#4285f4";

export function renderMonth(el: HTMLElement, p: MonthViewProps): void {
  const color = (e: CalEvent) => esc(p.colors.get(e.calendarId) ?? FALLBACK_COLOR);

  const cells = p.grid.map((d, i) => {
    const evs = p.events.get(d.key) ?? [];
    const dots = [...new Set(evs.map(color))].slice(0, 3).map((c) => `<i class="dot" style="background:${c}"></i>`).join("");
    const taskDot = p.tasks.has(d.key) ? `<i class="dot task-dot"></i>` : "";
    const cls = ["cell", `wd-${i % 7}`, d.inMonth ? "" : "out", d.isToday ? "today" : "", d.key === p.selected ? "selected" : ""]
      .filter(Boolean).join(" ");
    return `<button class="${cls}" data-day="${d.key}"><span class="num">${d.date.getDate()}</span><span class="dots">${dots}${taskDot}</span></button>`;
  }).join("");

  const [, sm, sd] = p.selected.split("-").map(Number);
  const items = [
    ...(p.events.get(p.selected) ?? []).map((e) =>
      `<li><i class="bar" style="background:${color(e)}"></i><span class="time">${formatEventTime(e)}</span><span class="title">${esc(e.title)}</span></li>`),
    ...(p.tasks.get(p.selected) ?? []).map((t) =>
      `<li><i class="bar task-bar"></i><span class="time">할 일</span><span class="title">${esc(t.title)}</span></li>`),
  ];

  el.innerHTML = `
    <div class="month-head">
      <button class="nav" data-nav="-1" title="이전 달">‹</button>
      <span class="month-title">${p.year}년 ${p.month}월</span>
      <button class="nav" data-nav="1" title="다음 달">›</button>
      <button class="nav today-btn" data-nav="0">오늘</button>
    </div>
    <div class="grid">
      ${WEEKDAYS.map((w, i) => `<div class="wd wd-${i}">${w}</div>`).join("")}
      ${cells}
    </div>
    <div class="day-detail">
      <h4>${sm}월 ${sd}일</h4>
      ${items.length ? `<ul>${items.join("")}</ul>` : `<p class="empty">일정 없음</p>`}
    </div>`;

  el.querySelectorAll<HTMLButtonElement>("[data-nav]").forEach((b) => {
    b.onclick = () => p.onNav(Number(b.dataset.nav) as -1 | 0 | 1);
  });
  el.querySelectorAll<HTMLButtonElement>("[data-day]").forEach((b) => {
    b.onclick = () => p.onSelect(b.dataset.day!);
  });
}
