/// <reference types="node" />
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { DEMO_CALENDARS, sampleMonth, sampleTasks } from "./sample-data";

const today = new Date(2026, 8, 30); // 2026-09-30 (수)

describe("sampleMonth", () => {
  it("only returns events inside the requested month", () => {
    const m = sampleMonth(2026, 9, today);
    expect(m.events.length).toBeGreaterThan(10);
    for (const e of m.events) expect(e.start.slice(0, 7)).toBe("2026-09");
  });

  it("uses only the demo calendars", () => {
    const ids = new Set(DEMO_CALENDARS.map((c) => c.id));
    for (const e of sampleMonth(2026, 11, today).events) expect(ids.has(e.calendarId)).toBe(true);
  });

  it("puts several events on today so the day list is filled", () => {
    const todays = sampleMonth(2026, 9, today).events.filter((e) => e.start.startsWith("2026-09-30"));
    expect(todays.length).toBeGreaterThanOrEqual(3);
  });

  it("includes fixed-date public holidays as all-day events", () => {
    const oct = sampleMonth(2026, 10, today).events.filter((e) => e.calendarId === "holiday");
    expect(oct.map((e) => [e.start, e.title])).toEqual([["2026-10-03", "개천절"], ["2026-10-09", "한글날"]]);
    expect(oct.every((e) => e.allDay)).toBe(true);
  });

  it("is deterministic", () => {
    expect(sampleMonth(2026, 9, today)).toEqual(sampleMonth(2026, 9, today));
  });
});

describe("sampleTasks", () => {
  it("returns two lists with due dates relative to today, one overdue", () => {
    const t = sampleTasks(today, new Set());
    expect(t.lists.map((l) => l.id)).toEqual(["my", "space"]);
    expect(t.tasks.some((x) => x.due === "2026-10-01")).toBe(true);
    expect(t.tasks.some((x) => x.due !== null && x.due < "2026-09-30")).toBe(true);
  });

  it("leaves out tasks completed in the demo", () => {
    const first = sampleTasks(today, new Set()).tasks[0];
    const after = sampleTasks(today, new Set([first.id])).tasks;
    expect(after.find((x) => x.id === first.id)).toBeUndefined();
  });
});

describe("demo/index.html", () => {
  it("matches the app's index.html except for the entry script", () => {
    const app = readFileSync("index.html", "utf8");
    const demo = readFileSync("demo/index.html", "utf8");
    const strip = (s: string) => s.replace(/<script[^>]*src="[^"]*"[^>]*><\/script>/, "<script/>").replace(/\r\n/g, "\n");
    expect(strip(demo)).toBe(strip(app));
    expect(demo).toContain('src="../src/demo/main.ts"');
  });
});
