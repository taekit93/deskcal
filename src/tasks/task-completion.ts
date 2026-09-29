import { errorText } from "../errors";

export interface CompletionDeps {
  setCompleted(listId: string, taskId: string, completed: boolean): Promise<void>;
  onChange(): void;
  onRemove(taskId: string): void;
  onError(message: string): void;
  delayMs?: number;
}

interface TaskRef { id: string; listId: string }

export function createCompletion(deps: CompletionDeps) {
  const delay = deps.delayMs ?? 3000;
  const checked = new Set<string>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

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
      checked.delete(id);
      deps.onRemove(id);
    }, delay));
  }

  return {
    isChecked: (id: string) => checked.has(id),

    async toggle(task: TaskRef): Promise<void> {
      const nowChecked = !checked.has(task.id);
      if (nowChecked) checked.add(task.id);
      else checked.delete(task.id);
      cancelRemoval(task.id);
      deps.onChange();
      try {
        await deps.setCompleted(task.listId, task.id, nowChecked);
      } catch (e) {
        // 서버 상태는 요청 전 상태 그대로이므로 UI를 되돌린다.
        if (nowChecked) checked.delete(task.id);
        else checked.add(task.id);
        deps.onChange();
        deps.onError(errorText(e));
        if (!nowChecked) scheduleRemoval(task.id);
        return;
      }
      if (nowChecked && checked.has(task.id)) scheduleRemoval(task.id);
    },
  };
}
