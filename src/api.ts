import { invoke } from "@tauri-apps/api/core";
import type { FontScale, ThemeName } from "./theme";

export type ViewMode = "calendar" | "tasks" | "both";

export interface Calendar { id: string; summary: string; color: string; primary: boolean }
export interface CalEvent {
  id: string; calendarId: string; title: string; allDay: boolean;
  start: string; end: string; htmlLink: string | null;
}
export interface TaskList { id: string; title: string }
export interface Task { id: string; listId: string; title: string; due: string | null; notes: string | null }
export interface MonthData { calendars: Calendar[]; events: CalEvent[]; failed: string[]; fetchedAt: string; stale: boolean }
export interface TasksData { lists: TaskList[]; tasks: Task[]; failed: string[]; fetchedAt: string; stale: boolean }
export interface Settings {
  viewMode: ViewMode; hiddenCalendars: string[]; hiddenTaskLists: string[];
  refreshMinutes: number; autostart: boolean; locked: boolean;
  theme: ThemeName; accent: string | null; opacity: number; fontScale: FontScale;
  headerMode: "always" | "hover";
  showDayDetail: boolean; showDue: boolean; showTaskDots: boolean; showBorder: boolean;
}

export const api = {
  authStatus: () => invoke<boolean>("auth_status"),
  login: () => invoke<void>("login"),
  logout: () => invoke<void>("logout"),
  getMonth: (year: number, month: number) => invoke<MonthData>("get_month", { year, month }),
  peekMonth: (year: number, month: number) => invoke<MonthData | null>("peek_month", { year, month }),
  getTasks: () => invoke<TasksData>("get_tasks"),
  peekTasks: () => invoke<TasksData | null>("peek_tasks"),
  setTaskCompleted: (listId: string, taskId: string, completed: boolean) =>
    invoke<void>("set_task_completed", { listId, taskId, completed }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
};
