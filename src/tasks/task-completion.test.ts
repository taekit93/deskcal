import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createCompletion } from "./task-completion";

const task = (id: string) => ({ id, listId: "L" });

function setup(setCompleted = vi.fn().mockResolvedValue(undefined)) {
  const deps = { setCompleted, onChange: vi.fn(), onRemove: vi.fn(), onError: vi.fn(), delayMs: 3000 };
  return { deps, c: createCompletion(deps) };
}

describe("createCompletion", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("checks immediately, calls API, removes after delay", async () => {
    const { deps, c } = setup();
    const p = c.toggle(task("a"));
    expect(c.isChecked("a")).toBe(true);
    expect(deps.onChange).toHaveBeenCalled();
    await p;
    expect(deps.setCompleted).toHaveBeenCalledWith("L", "a", true);
    vi.advanceTimersByTime(2999);
    expect(deps.onRemove).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(deps.onRemove).toHaveBeenCalledWith("a");
    expect(c.isChecked("a")).toBe(false);
  });

  it("undo within delay cancels removal and reverts on server", async () => {
    const { deps, c } = setup();
    await c.toggle(task("a"));
    vi.advanceTimersByTime(1000);
    await c.toggle(task("a"));
    expect(deps.setCompleted).toHaveBeenLastCalledWith("L", "a", false);
    vi.advanceTimersByTime(5000);
    expect(deps.onRemove).not.toHaveBeenCalled();
    expect(c.isChecked("a")).toBe(false);
  });

  it("rolls back and reports when API fails", async () => {
    const { deps, c } = setup(vi.fn().mockRejectedValue({ kind: "Network", message: "x" }));
    await c.toggle(task("a"));
    expect(c.isChecked("a")).toBe(false);
    expect(deps.onError).toHaveBeenCalledWith("네트워크 오류");
    vi.advanceTimersByTime(5000);
    expect(deps.onRemove).not.toHaveBeenCalled();
  });

  it("unchecking before the first request resolves does not remove the task", async () => {
    let resolveFirst!: () => void;
    const setCompleted = vi
      .fn()
      .mockImplementationOnce(() => new Promise<void>((r) => (resolveFirst = r)))
      .mockResolvedValue(undefined);
    const { deps, c } = setup(setCompleted);
    const first = c.toggle(task("a"));
    const second = c.toggle(task("a"));
    resolveFirst();
    await first;
    await second;
    vi.advanceTimersByTime(5000);
    expect(deps.onRemove).not.toHaveBeenCalled();
    expect(c.isChecked("a")).toBe(false);
  });

  it("keeps independent timers per task", async () => {
    const { deps, c } = setup();
    await c.toggle(task("a"));
    vi.advanceTimersByTime(2000);
    await c.toggle(task("b"));
    vi.advanceTimersByTime(1000);
    expect(deps.onRemove).toHaveBeenCalledTimes(1);
    expect(deps.onRemove).toHaveBeenCalledWith("a");
    vi.advanceTimersByTime(2000);
    expect(deps.onRemove).toHaveBeenCalledWith("b");
  });

  it("failed undo keeps the task completed and still removes it", async () => {
    const setCompleted = vi.fn().mockResolvedValueOnce(undefined).mockRejectedValueOnce({ kind: "Network" });
    const { deps, c } = setup(setCompleted);
    await c.toggle(task("a"));
    await c.toggle(task("a"));
    expect(c.isChecked("a")).toBe(true);
    expect(deps.onError).toHaveBeenCalled();
    vi.advanceTimersByTime(3000);
    expect(deps.onRemove).toHaveBeenCalledWith("a");
  });
});
