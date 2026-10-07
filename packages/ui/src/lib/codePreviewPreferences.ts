import type { BundledTheme } from "shiki";
import type { Theme } from "../useTheme.js";
import {
  DEFAULT_CODE_PREVIEW_SETTINGS,
  type CodePreviewSettings,
} from "./codePreviewSettings.js";

interface CodePreviewThemeOption {
  value: BundledTheme;
  label: string;
  /** 明暗归属：设置页两个下拉按它分组，深色槽位不再出现浅色主题。 */
  mode: "light" | "dark";
}

export const CODE_PREVIEW_THEME_OPTIONS: CodePreviewThemeOption[] = [
  { value: "github-light", label: "GitHub Light", mode: "light" },
  { value: "github-dark", label: "GitHub Dark", mode: "dark" },
  { value: "vitesse-light", label: "Vitesse Light", mode: "light" },
  { value: "vitesse-dark", label: "Vitesse Dark", mode: "dark" },
  { value: "min-light", label: "Minimal Light", mode: "light" },
  { value: "min-dark", label: "Minimal Dark", mode: "dark" },
  { value: "github-light-high-contrast", label: "GitHub HC Light", mode: "light" },
  { value: "github-dark-high-contrast", label: "GitHub HC Dark", mode: "dark" },
  { value: "catppuccin-latte", label: "Catppuccin Latte", mode: "light" },
  { value: "catppuccin-mocha", label: "Catppuccin Mocha", mode: "dark" },
];

/** 按明暗取下拉项：深色代码主题只列深色主题，反之亦然。 */
export function getCodePreviewThemeOptions(mode: "light" | "dark"): CodePreviewThemeOption[] {
  return CODE_PREVIEW_THEME_OPTIONS.filter((option) => option.mode === mode);
}

const DARK_CODE_PREVIEW_THEMES: readonly BundledTheme[] = [
  "github-dark",
  "vitesse-dark",
  "min-dark",
  "github-dark-high-contrast",
  "catppuccin-mocha",
];

export function isDarkCodePreviewTheme(theme: BundledTheme): boolean {
  // 代码主题的明暗语义必须跟随设置页支持的显式选项，不能用主题名正则猜测。
  return DARK_CODE_PREVIEW_THEMES.includes(theme);
}

export function isLightCodePreviewTheme(theme: BundledTheme): boolean {
  return !isDarkCodePreviewTheme(theme);
}

/** 归一化只关心这一对主题：消费方常只持有主题（不必凑齐字号/行号等字段）。 */
export type CodePreviewThemePair = Pick<CodePreviewSettings, "lightTheme" | "darkTheme">;

/**
 * 数据边界归一化：代码主题槽位必须与界面明暗匹配，错配就把该槽位拉回默认。
 *
 * 背景：浅色主题（如 github-light）画在深色卡片上就是深底深字，正文看起来"消失"。
 * 老数据或手工改过的 localStorage 里出现过 darkTheme=github-light 这种错配，
 * 全仓八个消费方（聊天代码块/文件查看器/diff/手机页/内联 diff…）都无条件信任这对设置，
 * 于是同一类隐形会在各处反复出现。统一在读写入口收口，消费方不必各自防御。
 */
export function normalizeCodePreviewThemeSettings<T extends CodePreviewThemePair>(settings: T): T {
  const lightTheme = isLightCodePreviewTheme(settings.lightTheme)
    ? settings.lightTheme
    : DEFAULT_CODE_PREVIEW_SETTINGS.lightTheme;
  const darkTheme = isDarkCodePreviewTheme(settings.darkTheme)
    ? settings.darkTheme
    : DEFAULT_CODE_PREVIEW_SETTINGS.darkTheme;
  if (lightTheme === settings.lightTheme && darkTheme === settings.darkTheme) {
    return settings;
  }
  return { ...settings, lightTheme, darkTheme };
}

export const SETTINGS_PREVIEW_CODE = `const themePreview: ThemeConfig = {
  surface: "sidebar",
  accent: "#339CFF",
  contrast: 45,
};`;

export function getCodePreviewTheme(
  mode: "light" | "dark",
  settings: CodePreviewThemePair,
): BundledTheme {
  const normalized = normalizeCodePreviewThemeSettings(settings);
  return mode === "dark" ? normalized.darkTheme : normalized.lightTheme;
}

/**
 * 由**应用主题**（含 system 跟随 OS）解析出该用哪套代码高亮主题。
 *
 * 八个渲染面（聊天代码块、文件查看器、富 diff、轻量 diff、内联 diff、手机页…）
 * 此前各自写同一段三元表达式，还各自处理 system：写法分散，任一处漏掉 system
 * 或写反明暗，就是一类"深底深字"。统一走这里，消费方只传 theme。
 */
export function resolveCodePreviewTheme(
  theme: Theme | undefined,
  settings: CodePreviewThemePair,
  options?: { prefersDark?: boolean },
): BundledTheme {
  const normalized = normalizeCodePreviewThemeSettings(settings);
  let mode: "light" | "dark";
  if (theme === "system" || theme === undefined) {
    const prefersDark =
      options?.prefersDark ??
      (typeof window !== "undefined" && typeof window.matchMedia === "function"
        ? window.matchMedia("(prefers-color-scheme: dark)").matches
        : false);
    mode = prefersDark ? "dark" : "light";
  } else {
    mode = theme === "dark" || theme === "zai-dark" ? "dark" : "light";
  }
  return mode === "dark" ? normalized.darkTheme : normalized.lightTheme;
}
