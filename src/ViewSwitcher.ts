import type { ViewMode } from "./api";

const MODES: [ViewMode, string][] = [["calendar", "캘린더"], ["both", "모두"], ["tasks", "할 일"]];

export function renderViewSwitcher(el: HTMLElement, current: ViewMode, onChange: (m: ViewMode) => void): void {
  el.innerHTML = MODES.map(([m, label]) =>
    `<button class="${m === current ? "active" : ""}" data-mode="${m}">${label}</button>`).join("");
  el.querySelectorAll<HTMLButtonElement>("[data-mode]").forEach((b) => {
    b.onclick = () => onChange(b.dataset.mode as ViewMode);
  });
}
