import type { Task, TaskList } from "../api";
import { esc } from "../dom";

export interface TaskViewProps {
  lists: TaskList[];
  tasks: Task[];
  hidden: string[];
  today: string;
  isChecked(id: string): boolean;
  onToggle(task: Task): void;
}

function dueLabel(due: string, today: string): string {
  const text = `${Number(due.slice(5, 7))}/${Number(due.slice(8, 10))}`;
  return `<span class="due ${due < today ? "overdue" : ""}">${text}</span>`;
}

export function renderTasks(el: HTMLElement, p: TaskViewProps): void {
  const groups = p.lists
    .filter((l) => !p.hidden.includes(l.id))
    .map((l) => ({ list: l, tasks: p.tasks.filter((t) => t.listId === l.id) }))
    .filter((g) => g.tasks.length > 0);

  if (groups.length === 0) {
    el.innerHTML = `<p class="empty">할 일이 없습니다</p>`;
    return;
  }

  el.innerHTML = groups.map((g) => `
    <div class="task-group">
      <h3>${esc(g.list.title)}</h3>
      <ul>${g.tasks.map((t) => {
        const checked = p.isChecked(t.id);
        return `<li class="${checked ? "done" : ""}">
          <label><input type="checkbox" data-task="${esc(t.id)}" ${checked ? "checked" : ""}><span class="title">${esc(t.title)}</span></label>
          ${t.due ? dueLabel(t.due, p.today) : ""}
        </li>`;
      }).join("")}</ul>
    </div>`).join("");

  el.querySelectorAll<HTMLInputElement>("input[data-task]").forEach((cb) => {
    cb.onchange = () => {
      const t = p.tasks.find((x) => x.id === cb.dataset.task);
      if (t) p.onToggle(t);
    };
  });
}
