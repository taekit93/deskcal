import type { CalEvent, Task } from "../api";

export interface GridDay { key: string; date: Date; inMonth: boolean; isToday: boolean }

export const pad = (n: number) => String(n).padStart(2, "0");

export function dayKey(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function parseDateOnly(s: string): Date {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d);
}

function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
}

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

/** month는 1~12. 일요일 시작 42칸. */
export function buildMonthGrid(year: number, month: number, today: Date): GridDay[] {
  const first = new Date(year, month - 1, 1);
  const start = addDays(first, -first.getDay());
  const todayKey = dayKey(today);
  return Array.from({ length: 42 }, (_, i) => {
    const date = addDays(start, i);
    const key = dayKey(date);
    return { key, date, inMonth: date.getMonth() === month - 1, isToday: key === todayKey };
  });
}

/** 일정이 걸치는 [첫날, 마지막 날] (둘 다 포함, 로컬 자정). */
export function eventDayRange(ev: CalEvent): [Date, Date] {
  if (ev.allDay) {
    const s = parseDateOnly(ev.start);
    const last = addDays(parseDateOnly(ev.end), -1);
    return [s, last < s ? s : last];
  }
  const s = new Date(ev.start);
  const e = new Date(ev.end);
  const first = startOfDay(s);
  let last = startOfDay(e);
  if (e > s && e.getTime() === last.getTime()) last = addDays(last, -1);
  return [first, last < first ? first : last];
}

function startMs(e: CalEvent): number {
  return e.allDay ? parseDateOnly(e.start).getTime() : new Date(e.start).getTime();
}

export function compareEvents(a: CalEvent, b: CalEvent): number {
  if (a.allDay !== b.allDay) return a.allDay ? -1 : 1;
  return startMs(a) - startMs(b) || a.title.localeCompare(b.title);
}

export function eventsByDay(events: CalEvent[], grid: GridDay[]): Map<string, CalEvent[]> {
  const map = new Map<string, CalEvent[]>();
  if (grid.length === 0) return map;
  const gridFirst = grid[0].date;
  const gridLast = grid[grid.length - 1].date;
  for (const ev of events) {
    let [d, end] = eventDayRange(ev);
    if (d < gridFirst) d = gridFirst;
    if (end > gridLast) end = gridLast;
    for (; d <= end; d = addDays(d, 1)) {
      const k = dayKey(d);
      const list = map.get(k);
      if (list) list.push(ev);
      else map.set(k, [ev]);
    }
  }
  for (const list of map.values()) list.sort(compareEvents);
  return map;
}

export function tasksByDay(tasks: Task[]): Map<string, Task[]> {
  const map = new Map<string, Task[]>();
  for (const t of tasks) {
    if (!t.due) continue;
    const list = map.get(t.due);
    if (list) list.push(t);
    else map.set(t.due, [t]);
  }
  return map;
}

export function formatEventTime(ev: CalEvent): string {
  if (ev.allDay) return "종일";
  const s = new Date(ev.start);
  return `${pad(s.getHours())}:${pad(s.getMinutes())}`;
}
