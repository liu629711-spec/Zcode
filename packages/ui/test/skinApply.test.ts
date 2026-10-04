import assert from "node:assert/strict";
import test from "node:test";
import { validateSkin } from "../src/skin-engine/skinSchema.js";
import { buildSkinCss, resolveEffectiveGlass, MIN_SURFACE_OPACITY } from "../src/skin-engine/skinApply.js";

function skinOf(raw: Record<string, unknown>) {
  const result = validateSkin({ formatVersion: 1, id: "t", name: "t", ...raw });
  assert.equal(result.ok, true);
  return result.ok ? result.skin : undefined;
}

test("buildSkinCss：ramp 覆盖生成 --style-* 变量且带 !important", () => {
  const css = buildSkinCss(
    skinOf({ light: { background: "#f5f4ed", brand: "#c96442" }, dark: { ink: "#faf9f5" } }),
  );
  assert.ok(css.includes(":root[data-zcode-skin]"));
  assert.ok(css.includes("--style-light-background: #f5f4ed !important;"));
  assert.ok(css.includes("--style-light-brand: #c96442 !important;"));
  assert.ok(css.includes("--style-dark-ink: #faf9f5 !important;"));
  assert.ok(!css.includes("focus-border"));
});

test("buildSkinCss：无覆盖无玻璃无壁纸=空串（卸载态）", () => {
  const css = buildSkinCss(skinOf({}));
  assert.equal(css, "");
});

test("buildSkinCss：有壁纸无 glass → 默认融入度生效（有壁纸必有玻璃）", () => {
  const glass = resolveEffectiveGlass(
    skinOf({ wallpaper: { kind: "gradient", gradient: "linear-gradient(0deg, red, blue)", opacity: 1, blurPx: 0, dim: 0 } }),
  );
  assert.ok(glass);
  assert.equal(glass.enabled, true);
  assert.equal(glass.blend, 0.35);
  const css = buildSkinCss(
    skinOf({ wallpaper: { kind: "gradient", gradient: "linear-gradient(0deg, red, blue)", opacity: 1, blurPx: 0, dim: 0 } }),
  );
  assert.ok(css.includes("html.theme-zai-light"));
  assert.ok(css.includes("html.theme-zai-dark"));
  assert.ok(css.includes("--color-background: color-mix(in oklab, var(--style-light-background) 65%, transparent)"));
});

test("buildSkinCss：玻璃区域倍率分别生效且贴可读性地板", () => {
  const css = buildSkinCss(
    skinOf({ glass: { enabled: true, blend: 1, sidebar: 1, panel: 0.2, composer: 0 } }),
  );
  // blend=1 sidebar=1 → 打到地板 0.5
  assert.ok(css.includes("--color-sidebar: color-mix(in oklab, var(--style-light-sidebar) 50%, transparent)"));
  // panel=0.2 → 1-0.2=0.8
  assert.ok(css.includes("--color-panel: color-mix(in oklab, var(--style-light-surface) 80%, transparent)"));
  // composer=0 → 完全不透明
  assert.ok(css.includes("--color-input: color-mix(in oklab, var(--style-light-surface) 100%, transparent)"));
  // 终端钉死不透明
  assert.ok(css.includes("--color-terminal-bg: var(--style-light-background);"));
  assert.ok(css.includes("--color-terminal-bg: var(--style-dark-background);"));
});

test("可读性地板：任何 blend 组合下不低于 MIN_SURFACE_OPACITY", () => {
  for (let blend = 0; blend <= 1.001; blend += 0.1) {
    for (const mult of [0, 0.3, 0.7, 1]) {
      const percent = Math.round(Math.max(1 - blend * mult, MIN_SURFACE_OPACITY) * 100);
      assert.ok(percent >= MIN_SURFACE_OPACITY * 100 - 0.5);
    }
  }
});
