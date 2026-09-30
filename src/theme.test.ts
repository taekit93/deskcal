import { describe, expect, it } from "vitest";
import { ACCENT_PRESETS, THEMES, resolveTheme, type Appearance } from "./theme";

const base: Appearance = { theme: "dark", accent: null, opacity: 0.86, fontScale: "normal" };

describe("resolveTheme", () => {
  it("system follows the OS color scheme", () => {
    expect(resolveTheme({ ...base, theme: "system" }, true).name).toBe("dark");
    expect(resolveTheme({ ...base, theme: "system" }, false).name).toBe("light");
  });

  it("explicit themes ignore the OS color scheme", () => {
    expect(resolveTheme({ ...base, theme: "midnight" }, false).name).toBe("midnight");
    expect(resolveTheme({ ...base, theme: "light" }, true).name).toBe("light");
  });

  it("uses the theme accent unless a custom accent is set", () => {
    expect(resolveTheme(base, true).vars["--accent"]).toBe(THEMES.dark.accent);
    expect(resolveTheme({ ...base, accent: "#34d399" }, true).vars["--accent"]).toBe("#34d399");
  });

  it("picks readable text on the accent color", () => {
    expect(resolveTheme({ ...base, accent: "#fdd663" }, true).vars["--accent-fg"]).toBe("#202124");
    expect(resolveTheme({ ...base, accent: "#1a3a8a" }, true).vars["--accent-fg"]).toBe("#ffffff");
  });

  it("applies opacity only to the widget background", () => {
    const v = resolveTheme({ ...base, opacity: 0.5 }, true).vars;
    const [r, g, b] = THEMES.dark.bg;
    expect(v["--bg"]).toBe(`rgba(${r}, ${g}, ${b}, 0.5)`);
    expect(v["--panel"]).toBe(THEMES.dark.panel);
    expect(v["--fg"]).toBe(THEMES.dark.fg);
  });

  it("maps font scale to a base font size", () => {
    expect(resolveTheme({ ...base, fontScale: "small" }, true).vars["--font-size"]).toBe("12px");
    expect(resolveTheme(base, true).vars["--font-size"]).toBe("13px");
    expect(resolveTheme({ ...base, fontScale: "large" }, true).vars["--font-size"]).toBe("14.5px");
  });

  it("every preset defines every color", () => {
    const keys = Object.keys(resolveTheme(base, true).vars).sort();
    for (const name of Object.keys(THEMES) as (keyof typeof THEMES)[]) {
      const v = resolveTheme({ ...base, theme: name }, true).vars;
      expect(Object.keys(v).sort()).toEqual(keys);
      for (const value of Object.values(v)) expect(value).not.toBe("");
    }
  });

  it("offers eight accent presets as hex colors", () => {
    expect(ACCENT_PRESETS).toHaveLength(8);
    for (const c of ACCENT_PRESETS) expect(c).toMatch(/^#[0-9a-f]{6}$/);
  });
});
