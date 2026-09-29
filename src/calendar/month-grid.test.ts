import { describe, expect, it } from "vitest";
import type { CalEvent, Task } from "../api";
import { buildMonthGrid, dayKey, eventsByDay, formatEventTime, tasksByDay } from "./month-grid";

const ev = (p: Partial<CalEvent>): CalEvent => ({
  id: "e", calendarId: "c1", title: "t", allDay: false,
  start: "2026-09-15T10:00:00+09:00", end: "2026-09-15T11:00:00+09:00", htmlLink: null, ...p,
});
const today = new Date(2026, 8, 29);

describe("buildMonthGrid", () => {
  it("has 42 days starting on the Sunday before the 1st", () => {
    const g = buildMonthGrid(2026, 9, today);
    expect(g).toHaveLength(42);
    expect(g[0].key).toBe("2026-08-30");
    expect(g[41].key).toBe("2026-10-10");
    expect(g[0].date.getDay()).toBe(0);
  });

  it("starts on the 1st when the month begins on Sunday", () => {
    expect(buildMonthGrid(2026, 2, today)[0].key).toBe("2026-02-01");
  });

  it("marks in-month days and today", () => {
    const g = buildMonthGrid(2026, 9, today);
    expect(g.filter((d) => d.inMonth)).toHaveLength(30);
    expect(g.filter((d) => d.isToday).map((d) => d.key)).toEqual(["2026-09-29"]);
  });

  it("handles leap-year February", () => {
    const g = buildMonthGrid(2028, 2, today);
    expect(g.filter((d) => d.inMonth)).toHaveLength(29);
  });

  it("handles December → January rollover", () => {
    expect(buildMonthGrid(2026, 12, today)[41].key).toBe("2027-01-09");
  });
});

describe("eventsByDay", () => {
  const grid = buildMonthGrid(2026, 9, today);

  it("places a timed event on its day", () => {
    const m = eventsByDay([ev({ id: "a" })], grid);
    expect(m.get("2026-09-15")?.map((e) => e.id)).toEqual(["a"]);
  });

  it("treats all-day end date as exclusive", () => {
    const m = eventsByDay([ev({ id: "v", allDay: true, start: "2026-09-10", end: "2026-09-12" })], grid);
    expect(m.has("2026-09-10")).toBe(true);
    expect(m.has("2026-09-11")).toBe(true);
    expect(m.has("2026-09-12")).toBe(false);
  });

  it("does not spill an event ending exactly at midnight into the next day", () => {
    const m = eventsByDay([ev({ id: "late", start: "2026-09-15T23:00:00+09:00", end: "2026-09-16T00:00:00+09:00" })], grid);
    expect(m.has("2026-09-15")).toBe(true);
    expect(m.has("2026-09-16")).toBe(false);
  });

  it("shows an overnight event on both days", () => {
    const m = eventsByDay([ev({ id: "night", start: "2026-09-15T22:00:00+09:00", end: "2026-09-16T02:00:00+09:00" })], grid);
    expect(m.has("2026-09-15")).toBe(true);
    expect(m.has("2026-09-16")).toBe(true);
  });

  it("converts UTC times to local days", () => {
    const m = eventsByDay([ev({ id: "utc", start: "2026-09-15T16:00:00Z", end: "2026-09-15T17:00:00Z" })], grid);
    expect(m.has("2026-09-16")).toBe(true);
  });

  it("clips multi-day events to the grid", () => {
    const m = eventsByDay([ev({ id: "long", allDay: true, start: "2026-08-01", end: "2026-12-01" })], grid);
    expect(m.size).toBe(42);
  });

  it("sorts all-day first, then by start time", () => {
    const m = eventsByDay([
      ev({ id: "b", start: "2026-09-15T14:00:00+09:00", end: "2026-09-15T15:00:00+09:00" }),
      ev({ id: "a", start: "2026-09-15T09:00:00+09:00", end: "2026-09-15T10:00:00+09:00" }),
      ev({ id: "d", allDay: true, start: "2026-09-15", end: "2026-09-16" }),
    ], grid);
    expect(m.get("2026-09-15")?.map((e) => e.id)).toEqual(["d", "a", "b"]);
  });
});

describe("tasksByDay / formatEventTime / dayKey", () => {
  it("groups tasks by due date and skips undated tasks", () => {
    const tasks: Task[] = [
      { id: "t1", listId: "L", title: "a", due: "2026-09-30", notes: null },
      { id: "t2", listId: "L", title: "b", due: null, notes: null },
    ];
    const m = tasksByDay(tasks);
    expect([...m.keys()]).toEqual(["2026-09-30"]);
  });

  it("formats time or 종일", () => {
    expect(formatEventTime(ev({}))).toBe("10:00");
    expect(formatEventTime(ev({ allDay: true, start: "2026-09-15", end: "2026-09-16" }))).toBe("종일");
  });

  it("zero-pads day keys", () => {
    expect(dayKey(new Date(2026, 0, 5))).toBe("2026-01-05");
  });
});
