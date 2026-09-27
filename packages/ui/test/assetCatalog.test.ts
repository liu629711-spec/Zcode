import assert from "node:assert/strict";
import test from "node:test";
import { ASSET_SANDBOX } from "../src/asset-library/AssetPreviewFrame.js";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import { assertSandboxSafe, validateCatalog } from "../src/asset-library/catalog/catalogCheck.js";
import type { AssetCategory, AssetManifest } from "../src/asset-library/catalog/types.js";

// ============================================================
// 素材库货架的可运行检查（技术设计 §6）
// ============================================================
// 钉的是 catalogCheck 的体检语义：坏货逐项报对错误、好货零误报、
// 沙箱常量恒等 "allow-scripts"（铁律见 AssetPreviewFrame.tsx）。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/assetCatalog.test.ts

function pick(id: string): AssetManifest {
  const manifest = ASSET_CATALOG.find((item) => item.id === id);
  assert.ok(manifest, `种子货 ${id} 必须在目录里`);
  return manifest;
}

/** 以真实好货为底，覆写出一件坏货。 */
function broken(overrides: Partial<AssetManifest>): AssetManifest {
  return { ...pick("jelly-toggle"), ...overrides };
}

test("真实种子目录：validateCatalog 全绿", () => {
  assert.deepEqual(validateCatalog(ASSET_CATALOG), []);
});

test("坏货逐项体检：每类坏法都报出对应错误", () => {
  // 重复 id
  const duplicated = validateCatalog([
    pick("jelly-toggle"),
    { ...pick("typewriter-text"), id: "jelly-toggle" },
  ]);
  assert.ok(duplicated.some((m) => m.includes("jelly-toggle") && m.includes("重复")), duplicated.join("；"));

  // 坏 id 形状（非 kebab-case）
  const badShape = validateCatalog([broken({ id: "Jelly_Toggle" })]);
  assert.ok(badShape.some((m) => m.includes("kebab-case")), badShape.join("；"));

  // 缺必填字段
  const missing = validateCatalog([broken({ title: " " })]);
  assert.ok(missing.some((m) => m.includes("必填字段 title")), missing.join("；"));

  // 非 prompt 类 files 为空
  const noFiles = validateCatalog([broken({ files: [] })]);
  assert.ok(noFiles.some((m) => m.includes("files 不能为空")), noFiles.join("；"));

  // previewHtml 外链
  const external = validateCatalog([
    broken({ previewHtml: '<script src="https://cdn.example.com/x.js"></script>' }),
  ]);
  assert.ok(external.some((m) => m.includes("外链")), external.join("；"));

  // previewHtml localStorage
  const persistent = validateCatalog([
    broken({ previewHtml: "<script>localStorage.setItem('k','v');</script>" }),
  ]);
  assert.ok(persistent.some((m) => m.includes("localStorage")), persistent.join("；"));
});

test("files 规则按类别分流：prompt 类允许空图纸，control 类不允许", () => {
  assert.ok(validateCatalog([broken({ category: "prompt", files: [] })]).every((m) => !m.includes("files")));
  assert.ok(validateCatalog([broken({ category: "prompt", files: [] })]).length === 0);
  assert.ok(validateCatalog([broken({ category: "control", files: [] })]).some((m) => m.includes("files")));
});

test("assertSandboxSafe：恒等 allow-scripts 通过，混入其它 token 一律 throw", () => {
  assert.doesNotThrow(() => assertSandboxSafe("allow-scripts"));
  assert.doesNotThrow(() => assertSandboxSafe(ASSET_SANDBOX));
  for (const poisoned of [
    "allow-scripts allow-same-origin",
    "allow-scripts allow-popups",
    "allow-scripts allow-top-navigation",
    "allow-same-origin",
    "",
  ]) {
    assert.throws(() => assertSandboxSafe(poisoned), poisoned);
  }
});

test("30 件货架：五类都有货且 id 唯一", () => {
  // S5 备货批次定编 30（8 件种子 + 22 件自制原创，配比见任务书素材库 S5 卡）
  assert.equal(ASSET_CATALOG.length, 30);
  const byCategory = new Map<AssetCategory, number>();
  for (const manifest of ASSET_CATALOG) {
    byCategory.set(manifest.category, (byCategory.get(manifest.category) ?? 0) + 1);
  }
  for (const category of ["control", "text-animation", "background", "block", "prompt"] as const) {
    assert.ok(
      (byCategory.get(category) ?? 0) >= 1,
      `类别 ${category} 至少要有一件种子货`,
    );
  }
  const ids = ASSET_CATALOG.map((manifest) => manifest.id);
  assert.equal(new Set(ids).size, ids.length, "种子货 id 必须全局唯一");
});
