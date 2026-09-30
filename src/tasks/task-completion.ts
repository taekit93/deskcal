import { errorText } from "../errors";

export interface CompletionDeps {
  setCompleted(listId: string, taskId: string, completed: boolean): Promise<void>;
  onChange(): void;
  onRemove(taskId: string): void;
  onError(message: string): void;
  delayMs?: number;
}

interface TaskRef { id: string; listId: string }

/** 할 일 하나의 상태. confirmed = 서버에 반영된 값, desired = 사용자가 마지막으로 고른 값. */
interface Entry { confirmed: boolean; desired: boolean; running: boolean }

/**
 * 할 일마다 요청을 한 번에 하나만 보낸다(순서 보장). 요청 중에 사용자가 여러 번 눌러도
 * 마지막 선택만 남기고, 응답이 오면 그 선택이 서버와 다를 때만 다음 요청을 보낸다.
 */
export function createCompletion(deps: CompletionDeps) {
  const delay = deps.delayMs ?? 3000;
  const entries = new Map<string, Entry>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  const entry = (id: string): Entry => {
    let e = entries.get(id);
    if (!e) entries.set(id, (e = { confirmed: false, desired: false, running: false }));
    return e;
  };

  function cancelRemoval(id: string) {
    const t = timers.get(id);
    if (t !== undefined) {
      clearTimeout(t);
      timers.delete(id);
    }
  }

  function scheduleRemoval(id: string) {
    cancelRemoval(id);
    timers.set(id, setTimeout(() => {
      timers.delete(id);
      entries.delete(id);
      deps.onRemove(id);
    }, delay));
  }

  async function sync(task: TaskRef, e: Entry): Promise<void> {
    if (e.running) return;
    e.running = true;
    try {
      while (e.desired !== e.confirmed) {
        const target = e.desired;
        try {
          await deps.setCompleted(task.listId, task.id, target);
          e.confirmed = target;
        } catch (err) {
          // 실패: 화면을 서버 상태로 되돌린다. (요청 중에 사용자가 다른 값을 골랐다면 그 값도 버린다.)
          e.desired = e.confirmed;
          deps.onChange();
          deps.onError(errorText(err));
          break;
        }
      }
    } finally {
      e.running = false;
    }
    if (entries.get(task.id) === e && e.confirmed && e.desired) scheduleRemoval(task.id);
  }

  return {
    isChecked: (id: string) => entries.get(id)?.desired ?? false,

    toggle(task: TaskRef): Promise<void> {
      const e = entry(task.id);
      e.desired = !e.desired;
      cancelRemoval(task.id);
      deps.onChange();
      return sync(task, e);
    },

    /** 로그아웃 등으로 목록이 사라질 때: 예약된 제거와 상태를 모두 잊는다. */
    reset(): void {
      for (const t of timers.values()) clearTimeout(t);
      timers.clear();
      entries.clear();
    },
  };
}
