import assert from "node:assert/strict";
import test from "node:test";
import { ASSET_SANDBOX } from "../src/asset-library/AssetPreviewFrame.js";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import {
  assertSandboxSafe,
  countDeliverableExternalRefs,
  validateCatalog,
} from "../src/asset-library/catalog/catalogCheck.js";
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

  // 跳转/伪协议通道（复验 2026-10-01 加宽：审计记录的三条盲区收进体检）
  const redirect = validateCatalog([
    broken({ previewHtml: '<meta http-equiv="refresh" content="0;url=https://evil.example">' }),
    broken({ previewHtml: '<form action="https://evil.example/collect"></form>' }),
    broken({ previewHtml: '<a href="javascript:fetch(location)">x</a>' }),
  ]);
  assert.equal(
    redirect.filter((m) => m.includes("跳转/伪协议")).length,
    3,
    redirect.join("；"),
  );

  // previewHtml localStorage
  const persistent = validateCatalog([
    broken({ previewHtml: "<script>localStorage.setItem('k','v');</script>" }),
  ]);
  assert.ok(persistent.some((m) => m.includes("localStorage")), persistent.join("；"));

  // source 台账：有 source 时三字段缺一报错
  const noLicense = validateCatalog([
    broken({ source: { site: "React Bits", url: "https://reactbits.dev", license: " " } }),
  ]);
  assert.ok(noLicense.some((m) => m.includes("source.license")), noLicense.join("；"));
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
  // 图纸口径是 30~50（技术设计 §8）：这里只钉"至少 30"，后续备货片加货不该炸。
  assert.ok(ASSET_CATALOG.length >= 30, `货架应至少 30 件，实际 ${ASSET_CATALOG.length}`);
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

// ============================================================
// V2-4 四站首批的台账钉子（技术设计 §10）：逐件必须有完整 source
// ============================================================
const SOURCED_BATCH = [
  // React Bits 6
  "split-text", "blur-text", "count-up", "silk-background", "click-spark", "ribbons-background",
  // Beautiful UI 6
  "ai-loading-dots", "ai-thinking-trace", "ai-tool-call-row", "ai-approval-card", "ai-task-row", "ai-streamed-text",
  // RareUI 4
  "animated-tooltip", "spotlight-card", "flip-card", "animated-list",
  // UIverse 4
  "day-night-toggle", "orbit-loader", "checkmark-checkbox", "glow-focus-input",
] as const;

test("V2-4 四站首批 20 件：逐件 source 三字段齐全（许可台账）", () => {
  for (const id of SOURCED_BATCH) {
    const manifest = pick(id);
    assert.ok(manifest.source, `${id}: 缺 source（台账必填）`);
    for (const field of ["site", "url", "license"] as const) {
      assert.ok(
        manifest.source[field].trim() !== "",
        `${id}: source.${field} 为空`,
      );
    }
    assert.match(manifest.source.url, /^https:\/\//, `${id}: source.url 应为 https`);
  }
});

// ============================================================
// V5 备货扩容：UIverse 自动收录大袋（动态 chunk）+ React Bits 灵感批次
// ============================================================

test("UIverse 自动收录大袋：3,000+ 件全绿且与静态货架 id 不撞", async () => {
  const { BULK_AUTO_ASSETS } = await import("../src/asset-library/catalog/generated/index.js");
  assert.ok(BULK_AUTO_ASSETS.length >= 3_000, `大袋应 ≥3000 件，实际 ${BULK_AUTO_ASSETS.length}`);
  assert.deepEqual(validateCatalog(BULK_AUTO_ASSETS), []);

  const staticIds = new Set(ASSET_CATALOG.map((manifest) => manifest.id));
  const bulkIds = BULK_AUTO_ASSETS.map((manifest) => manifest.id);
  assert.equal(new Set(bulkIds).size, bulkIds.length, "大袋内 id 必须唯一");
  for (const id of bulkIds) {
    assert.ok(!staticIds.has(id), `大袋 id 与静态货架撞车：${id}`);
  }
});

test("UIverse 自动收录大袋：预览惰性可算且沙箱清洗生效", async () => {
  const { BULK_AUTO_ASSETS } = await import("../src/asset-library/catalog/generated/index.js");
  // 惰性 getter：首读能算出完整预览文档
  const sample = BULK_AUTO_ASSETS[0];
  assert.ok(sample.previewHtml.startsWith("<!doctype html>"), "预览应为完整文档");
  assert.ok(sample.previewHtml.includes(sample.files[0].content), "预览应内嵌图纸原文");
  // 抽查全袋：预览（清洗后）不再有外链 src，图纸保持上游逐字
  for (const manifest of BULK_AUTO_ASSETS) {
    assert.ok(
      !/src\s*=\s*["']https?:/i.test(manifest.previewHtml),
      `${manifest.id}: 预览仍含外链 src`,
    );
  }
});

test("React Bits 灵感批次：效果自实现口径逐件带 source", async () => {
  const reactBits = ASSET_CATALOG.filter((manifest) => manifest.id.startsWith("reactbits-"));
  assert.ok(reactBits.length >= 20, `React Bits 批次应 ≥20 件，实际 ${reactBits.length}`);
  for (const manifest of reactBits) {
    assert.ok(manifest.source, `${manifest.id}: 缺 source`);
    assert.match(manifest.source.license, /效果自实现/, `${manifest.id}: 许可字段应标注自实现口径`);
    assert.match(manifest.source.url, /^https:\/\/reactbits\.dev/, `${manifest.id}: 出处应为 reactbits.dev`);
  }
});

test("设计风格卡全量：152 张齐且全为 design-style 类", () => {
  const styles = ASSET_CATALOG.filter((manifest) => manifest.category === "design-style");
  assert.ok(styles.length >= 152, `设计风格应 ≥152 张，实际 ${styles.length}`);
  const ids = ASSET_CATALOG.map((manifest) => manifest.id);
  assert.equal(new Set(ids).size, ids.length, "全目录 id 必须全局唯一");
});

test("countDeliverableExternalRefs：只数资源类加载，署名 <a href> 与 xmlns 不算（复验拍板）", () => {
  assert.equal(
    countDeliverableExternalRefs(
      '<svg xmlns="http://www.w3.org/2000/svg"></svg>' +
        '<img src="https://api.dicebear.com/9/x/svg?seed=a">' +
        '<img src="//api.dicebear.com/9/x/svg?seed=b">' +
        '<a href="https://github.com/uiverse-io/galaxy">署名</a>' +
        '<link href="https://fonts.googleapis.com/css2?family=x" rel="stylesheet">' +
        '<a href="/relative/path">站内</a>',
    ),
    3,
  );
  assert.equal(countDeliverableExternalRefs(""), 0);
  assert.equal(countDeliverableExternalRefs("<div>没有外链的图纸</div>"), 0);
});
