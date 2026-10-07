/**
 * 代码主题解析回归测试。
 *
 * 背景（真机实证）：设置里 darkTheme 曾被存成 "github-light"（浅色主题），
 * 深色界面于是渲染出 rgb(36,41,46) 的深色文字画在 #202020 的深色卡片上——
 * 正文看起来"消失"。全仓八个渲染面都无条件信任这对设置，所以修复放在数据边界：
 * 归一化 + 统一解析。这个文件锁住那两条保证。
 */
import assert from "node:assert/strict";
import test from "node:test";
import {
  CODE_PREVIEW_THEME_OPTIONS,
  getCodePreviewTheme,
  getCodePreviewThemeOptions,
  isDarkCodePreviewTheme,
  normalizeCodePreviewThemeSettings,
  resolveCodePreviewTheme,
} from "../src/lib/codePreviewPreferences.js";
import { DEFAULT_CODE_PREVIEW_SETTINGS } from "../src/lib/codePreviewSettings.js";

test("归一化：深色槽位里的浅色主题被拉回深色默认值（隐形文字的元凶）", () => {
  const fixed = normalizeCodePreviewThemeSettings({
    lightTheme: "github-light",
    darkTheme: "github-light", // ← 真机上就是这个错配
  });
  assert.equal(fixed.darkTheme, DEFAULT_CODE_PREVIEW_SETTINGS.darkTheme);
  assert.equal(isDarkCodePreviewTheme(fixed.darkTheme), true);
  // 合法的浅色槽位保持不动
  assert.equal(fixed.lightTheme, "github-light");
});

test("归一化：浅色槽位里的深色主题同样被拉回（两个方向都堵）", () => {
  const fixed = normalizeCodePreviewThemeSettings({
    lightTheme: "catppuccin-mocha",
    darkTheme: "vitesse-dark",
  });
  assert.equal(fixed.lightTheme, DEFAULT_CODE_PREVIEW_SETTINGS.lightTheme);
  assert.equal(isDarkCodePreviewTheme(fixed.lightTheme), false);
  assert.equal(fixed.darkTheme, "vitesse-dark");
});

test("归一化：已正确的设置原样返回（引用不变，避免多余重渲染）", () => {
  const settings = { lightTheme: "github-light", darkTheme: "github-dark" } as const;
  assert.equal(normalizeCodePreviewThemeSettings(settings), settings);
});

test("归一化：保留主题以外的字段（字号/行号等不被吞掉）", () => {
  const fixed = normalizeCodePreviewThemeSettings({
    lightTheme: "github-light",
    darkTheme: "github-light",
    fontSizePx: 16,
    showLineNumbers: false,
    wrapLongLines: true,
  });
  assert.equal(fixed.fontSizePx, 16);
  assert.equal(fixed.showLineNumbers, false);
  assert.equal(fixed.wrapLongLines, true);
});

test("解析：暗色主题拿到深色代码主题，亮色拿到浅色", () => {
  const settings = { lightTheme: "github-light", darkTheme: "catppuccin-mocha" } as const;
  for (const dark of ["dark", "zai-dark"] as const) {
    assert.equal(resolveCodePreviewTheme(dark, settings), "catppuccin-mocha", dark);
  }
  for (const light of ["light", "zai-light"] as const) {
    assert.equal(resolveCodePreviewTheme(light, settings), "github-light", light);
  }
});

test("解析：system 跟随 OS 偏好（浅色 OS 不得拿到深色主题的深底）", () => {
  const settings = { lightTheme: "github-light", darkTheme: "catppuccin-mocha" } as const;
  assert.equal(resolveCodePreviewTheme("system", settings, { prefersDark: false }), "github-light");
  assert.equal(resolveCodePreviewTheme("system", settings, { prefersDark: true }), "catppuccin-mocha");
  // theme 缺省时同 system 口径
  assert.equal(resolveCodePreviewTheme(undefined, settings, { prefersDark: false }), "github-light");
});

test("解析：错配设置经过解析后依然是合法明暗（防御性闭环）", () => {
  const broken = { lightTheme: "github-light", darkTheme: "github-light" } as const;
  assert.equal(isDarkCodePreviewTheme(resolveCodePreviewTheme("dark", broken)), true);
  assert.equal(isDarkCodePreviewTheme(resolveCodePreviewTheme("light", broken)), false);
});

test("解析：错配值任何主题下都不会产出浅色主题给深色模式", () => {
  const broken = { lightTheme: "github-dark", darkTheme: "github-light" } as const;
  const darkTheme = resolveCodePreviewTheme("zai-dark", broken);
  assert.equal(isDarkCodePreviewTheme(darkTheme), true);
});

test("下拉分组：深浅两档只各出同类主题，且合并后无遗漏", () => {
  const light = getCodePreviewThemeOptions("light");
  const dark = getCodePreviewThemeOptions("dark");
  assert.ok(light.length > 0 && dark.length > 0);
  for (const option of light) assert.equal(isDarkCodePreviewTheme(option.value), false, option.value);
  for (const option of dark) assert.equal(isDarkCodePreviewTheme(option.value), true, option.value);
  assert.equal(light.length + dark.length, CODE_PREVIEW_THEME_OPTIONS.length);
});

test("默认设置本身自洽（浅槽浅主题、深槽深主题）", () => {
  assert.equal(isDarkCodePreviewTheme(DEFAULT_CODE_PREVIEW_SETTINGS.darkTheme), true);
  assert.equal(isDarkCodePreviewTheme(DEFAULT_CODE_PREVIEW_SETTINGS.lightTheme), false);
  assert.equal(
    normalizeCodePreviewThemeSettings(DEFAULT_CODE_PREVIEW_SETTINGS),
    DEFAULT_CODE_PREVIEW_SETTINGS,
  );
});

test("getCodePreviewTheme 与 resolveCodePreviewTheme 口径一致", () => {
  const settings = { lightTheme: "min-light", darkTheme: "min-dark" } as const;
  assert.equal(getCodePreviewTheme("dark", settings), resolveCodePreviewTheme("zai-dark", settings));
  assert.equal(
    getCodePreviewTheme("light", settings),
    resolveCodePreviewTheme("zai-light", settings),
  );
});
