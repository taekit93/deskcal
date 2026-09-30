export type ThemeName = "system" | "dark" | "light" | "midnight" | "forest" | "rose";
export type FontScale = "small" | "normal" | "large";
type ConcreteTheme = Exclude<ThemeName, "system">;

export interface Appearance {
  theme: ThemeName;
  accent: string | null;
  opacity: number;
  fontScale: FontScale;
}

interface Palette {
  /** 위젯 배경 RGB. 불투명도는 설정값으로 따로 적용한다. */
  bg: [number, number, number];
  panel: string;
  fg: string;
  muted: string;
  line: string;
  hover: string;
  accent: string;
  sun: string;
  sat: string;
  task: string;
}

export const THEMES: Record<ConcreteTheme, Palette> = {
  dark: {
    bg: [22, 24, 30], panel: "#202329", fg: "#e8eaed", muted: "#9aa0a6",
    line: "rgba(255, 255, 255, 0.08)", hover: "rgba(255, 255, 255, 0.08)",
    accent: "#8ab4f8", sun: "#f28b82", sat: "#8ab4f8", task: "#fdd663",
  },
  light: {
    bg: [250, 250, 252], panel: "#ffffff", fg: "#202124", muted: "#5f6368",
    line: "rgba(0, 0, 0, 0.08)", hover: "rgba(0, 0, 0, 0.05)",
    accent: "#1a73e8", sun: "#d93025", sat: "#1a73e8", task: "#f9ab00",
  },
  midnight: {
    bg: [15, 23, 42], panel: "#1e293b", fg: "#e2e8f0", muted: "#94a3b8",
    line: "rgba(148, 163, 184, 0.15)", hover: "rgba(148, 163, 184, 0.12)",
    accent: "#818cf8", sun: "#fb7185", sat: "#7dd3fc", task: "#fbbf24",
  },
  forest: {
    bg: [20, 32, 26], panel: "#1c2b23", fg: "#e3efe7", muted: "#93a89b",
    line: "rgba(227, 239, 231, 0.10)", hover: "rgba(227, 239, 231, 0.08)",
    accent: "#6ee7b7", sun: "#fca5a5", sat: "#93c5fd", task: "#fde68a",
  },
  rose: {
    bg: [253, 242, 244], panel: "#fff7f8", fg: "#3b1f27", muted: "#8a6570",
    line: "rgba(59, 31, 39, 0.10)", hover: "rgba(59, 31, 39, 0.06)",
    accent: "#e11d74", sun: "#e11d48", sat: "#6366f1", task: "#d97706",
  },
};

export const THEME_LABELS: Record<ThemeName, string> = {
  system: "시스템", dark: "다크", light: "라이트", midnight: "미드나잇", forest: "포레스트", rose: "로즈",
};

export const ACCENT_PRESETS = [
  "#8ab4f8", "#1a73e8", "#818cf8", "#a78bfa", "#f472b6", "#f87171", "#fb923c", "#34d399",
];

const FONT_SIZES: Record<FontScale, string> = { small: "12px", normal: "13px", large: "14.5px" };

/** 강조색 위에 올릴 글자색 (밝은 강조색엔 어두운 글자). */
function textOn(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.55 ? "#202124" : "#ffffff";
}

export function resolveTheme(a: Appearance, prefersDark: boolean): { name: ConcreteTheme; vars: Record<string, string> } {
  const name: ConcreteTheme = a.theme === "system" ? (prefersDark ? "dark" : "light") : a.theme;
  const p = THEMES[name];
  const accent = a.accent ?? p.accent;
  const [r, g, b] = p.bg;
  return {
    name,
    vars: {
      "--bg": `rgba(${r}, ${g}, ${b}, ${a.opacity})`,
      "--panel": p.panel,
      "--fg": p.fg,
      "--muted": p.muted,
      "--line": p.line,
      "--hover": p.hover,
      "--accent": accent,
      "--accent-fg": textOn(accent),
      "--sun": p.sun,
      "--sat": p.sat,
      "--task": p.task,
      "--font-size": FONT_SIZES[a.fontScale],
    },
  };
}

export function applyTheme(el: HTMLElement, a: Appearance, prefersDark: boolean): void {
  const { name, vars } = resolveTheme(a, prefersDark);
  el.dataset.theme = name;
  for (const [k, v] of Object.entries(vars)) el.style.setProperty(k, v);
}
