// 소개 페이지 라이브 데모용 예시 데이터. 실제 사용자 데이터는 쓰지 않는다.
import type { CalEvent, Calendar, MonthData, Settings, Task, TasksData } from "../api";

export const DEMO_CALENDARS: Calendar[] = [
  { id: "personal", summary: "개인", color: "#7986cb", primary: true },
  { id: "work", summary: "업무", color: "#33b679", primary: false },
  { id: "holiday", summary: "대한민국 공휴일", color: "#e67c73", primary: false },
];

export const DEMO_SETTINGS: Settings = {
  viewMode: "both", hiddenCalendars: [], hiddenTaskLists: [], refreshMinutes: 10, autostart: true, locked: false,
  theme: "dark", accent: null, opacity: 0.92, fontScale: "normal", headerMode: "always",
  showDayDetail: true, showDue: true, showTaskDots: true, showBorder: true,
};

const HOLIDAYS: Record<string, string> = {
  "01-01": "신정", "03-01": "삼일절", "05-05": "어린이날", "06-06": "현충일",
  "08-15": "광복절", "10-03": "개천절", "10-09": "한글날", "12-25": "성탄절",
};

const pad = (n: number) => String(n).padStart(2, "0");
const ymd = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);

export function sampleMonth(year: number, month: number, today: Date): MonthData {
  const events: CalEvent[] = [];
  let seq = 0;
  const day = (d: number) => `${year}-${pad(month)}-${pad(d)}`;
  const timed = (cal: string, d: number, title: string, h: number, min: number, minutes: number) => {
    const end = new Date(year, month - 1, d, h, min + minutes);
    events.push({
      id: `demo-${year}${pad(month)}-${seq++}`, calendarId: cal, title, allDay: false,
      start: `${day(d)}T${pad(h)}:${pad(min)}:00+09:00`,
      end: `${ymd(end)}T${pad(end.getHours())}:${pad(end.getMinutes())}:00+09:00`, htmlLink: null,
    });
  };
  const allDay = (cal: string, d: number, title: string, days = 1) => {
    events.push({
      id: `demo-${year}${pad(month)}-${seq++}`, calendarId: cal, title, allDay: true,
      start: day(d), end: ymd(new Date(year, month - 1, d + days)), htmlLink: null,
    });
  };

  const last = new Date(year, month, 0).getDate();
  for (let d = 1; d <= last; d++) {
    const holiday = HOLIDAYS[`${pad(month)}-${pad(d)}`];
    if (holiday) allDay("holiday", d, holiday);
    const wd = new Date(year, month - 1, d).getDay();
    if (wd === 1) timed("work", d, "팀 주간회의", 10, 0, 60);
    if (wd === 3) timed("personal", d, "헬스", 19, 0, 60);
  }
  timed("work", 5, "디자인 리뷰", 14, 0, 90);
  timed("personal", 12, "치과 예약", 11, 30, 30);
  timed("work", 15, "월간 보고", 9, 30, 60);
  timed("personal", 15, "점심 약속", 12, 0, 60);
  timed("work", 15, "1:1 미팅", 16, 0, 30);
  timed("personal", Math.min(20, last), "가족 저녁", 18, 30, 120);
  allDay("work", Math.min(26, last - 1), "워크숍", 2);

  if (today.getFullYear() === year && today.getMonth() + 1 === month) {
    const t = today.getDate();
    timed("work", t, "스탠드업", 9, 0, 15);
    timed("work", t, "고객 미팅", 15, 0, 60);
    timed("personal", t, "요가", 20, 0, 60);
  }

  return { calendars: DEMO_CALENDARS, events, failed: [], fetchedAt: new Date().toISOString(), stale: false };
}

export function sampleTasks(today: Date, completed: Set<string>): TasksData {
  const due = (n: number) => ymd(addDays(today, n));
  const tasks: Task[] = [
    { id: "t-report", listId: "my", title: "주간 보고서 작성", due: due(1), notes: null },
    { id: "t-insurance", listId: "my", title: "자동차 보험 갱신", due: due(-2), notes: null },
    { id: "t-flight", listId: "my", title: "항공권 예매", due: null, notes: null },
    { id: "t-book", listId: "my", title: "도서관 책 반납", due: due(5), notes: null },
    { id: "t-banner", listId: "space", title: "[디자인팀] 배너 시안 검토", due: due(0), notes: null },
    { id: "t-notice", listId: "space", title: "[운영] 서버 점검 공지 확인", due: null, notes: null },
  ];
  return {
    lists: [{ id: "my", title: "내 할 일" }, { id: "space", title: "팀 스페이스" }],
    tasks: tasks.filter((t) => !completed.has(t.id)),
    failed: [], fetchedAt: new Date().toISOString(), stale: false,
  };
}
