// 素材库复验驱动（2026-10-01，队列⑤）：既有体检重跑 + 新鲜眼光全模式扫描。
// 只读：不改任何目录文件；结论打到 stdout（VERDICT 行收口）。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json scripts/asset-library-reverify.mts
import assert from "node:assert/strict";
import { ASSET_CATALOG } from "../packages/ui/src/asset-library/catalog/index.js";
import { BULK_AUTO_ASSETS } from "../packages/ui/src/asset-library/catalog/generated/index.js";
import {
  ASSET_SANDBOX,
  assertSandboxSafe,
  validateCatalog,
} from "../packages/ui/src/asset-library/catalog/catalogCheck.js";

let problems = 0;
const report = (label: string, detail: string) => {
  problems += 1;
  console.log(`FAIL ${label}: ${detail}`);
};

// ── 1. 既有体检：validateCatalog 全绿 + 沙箱常量恒等 ──────────────────
const catalogErrors = validateCatalog(ASSET_CATALOG);
if (catalogErrors.length > 0) {
  for (const error of catalogErrors.slice(0, 20)) report("validateCatalog", error);
  if (catalogErrors.length > 20) {
    console.log(`FAIL validateCatalog: …另 ${catalogErrors.length - 20} 条省略`);
    problems += 0; // 上面已计
  }
}
try {
  assertSandboxSafe(ASSET_SANDBOX);
} catch (error) {
  report("assertSandboxSafe", String(error));
}

// ── 2. 盘点：按 category 与来源批次计数 ───────────────────────────────
const byCategory = new Map<string, number>();
const bySource = new Map<string, number>();
for (const manifest of ASSET_CATALOG) {
  byCategory.set(manifest.category, (byCategory.get(manifest.category) ?? 0) + 1);
  const site = manifest.source?.site ?? "(自制)";
  bySource.set(site, (bySource.get(site) ?? 0) + 1);
}
console.log(`COUNT total=${ASSET_CATALOG.length}`);
for (const [category, count] of [...byCategory.entries()].sort()) {
  console.log(`COUNT category ${category}=${count}`);
}
for (const [site, count] of [...bySource.entries()].sort((a, b) => b[1] - a[1]).slice(0, 10)) {
  console.log(`COUNT source ${site}=${count}`);
}

// ── 3. 新鲜眼光扫描（审计盲区扩展）───────────────────────────────────
// EXTERNAL_REF（catalogCheck）只认 src/href/srcset/poster/@import/url() 的外链形态；
// 这里扫它没覆盖的请求/跳转/统计通道。previewHtml 惰性 getter 逐件物化（测试既有先例）。
const FRESH_PATTERNS: ReadonlyArray<{ name: string; re: RegExp }> = [
  { name: "meta-refresh", re: /<meta[^>]*http-equiv\s*=\s*["']?refresh/i },
  { name: "form-action-external", re: /<form[^>]*\baction\s*=\s*["']?(?:https?:)?\/\//i },
  { name: "javascript-url", re: /(?:href|src|action|location\.href)\s*=\s*["']?\s*javascript:/i },
  { name: "fetch-absolute", re: /\bfetch\s*\(\s*[`"'](?:https?:)?\/\/[^`"')]+/i },
  { name: "xhr-absolute", re: /XMLHttpRequest[\s\S]{0,200}open\s*\(\s*["'][^"']*["']\s*,\s*["'](?:https?:)?\/\//i },
  { name: "websocket-absolute", re: /new\s+WebSocket\s*\(\s*[`"'](?:wss?:)?\/\/[^`"')]+/i },
  { name: "sendBeacon", re: /sendBeacon\s*\(/i },
  { name: "eventsource-absolute", re: /new\s+EventSource\s*\(\s*[`"'](?:https?:)?\/\//i },
  { name: "dynamic-import-absolute", re: /import\s*\(\s*[`"'](https?:)?\/\//i },
  { name: "base-href-external", re: /<base[^>]+href\s*=\s*["']?(?:https?:)?\/\//i },
  { name: "window-open", re: /window\.open\s*\(/i },
  { name: "top-navigation", re: /(?:top|parent)\.location(?:\s*=|\.href\s*=)/i },
];

// 图纸原文（files）允许上游原文带外链（台账口径：清洗只发生在预览副本），
// 这里只盘点数量作质量信号，不算违规。
let filesExternalCount = 0;
let filesExternalSamples = 0;

for (const manifest of ASSET_CATALOG) {
  // previewHtml 惰性物化（uiverse 3793 件逐件首读，测试同款走法）。
  const previewHtml: string = manifest.previewHtml;
  for (const pattern of FRESH_PATTERNS) {
    if (pattern.re.test(previewHtml)) {
      report(
        `preview[${manifest.id}] ${pattern.name}`,
        previewHtml.match(pattern.re)?.[0]?.slice(0, 120) ?? "(match)",
      );
    }
  }
  for (const file of manifest.files) {
    if (/(?:https?:)?\/\/[^\s"'`)>]+\.[a-z]{2,}/i.test(file.content)) {
      filesExternalCount += 1;
      if (filesExternalSamples < 5) {
        filesExternalSamples += 1;
        console.log(`INFO files-external ${manifest.id}/${file.name}`);
      }
    }
  }
}

// files 里的外链是交付质量信号（上游原文口径允许），单独汇报不计违规。
console.log(`COUNT files-with-external-refs=${filesExternalCount}`);

// ── 4. uiverse 自动收录大袋（3,793 件）同一套体检与新鲜扫描 ────────────
assert.ok(BULK_AUTO_ASSETS.length >= 3_000, "大袋应 ≥3000 件");
const bulkErrors = validateCatalog(BULK_AUTO_ASSETS);
if (bulkErrors.length > 0) {
  for (const error of bulkErrors.slice(0, 20)) report("bulk validateCatalog", error);
}
const bulkIds = new Set<string>();
let bulkDuplicate = 0;
let bulkFresh = 0;
const bulkFreshSamples = new Map<string, string>();
for (const manifest of BULK_AUTO_ASSETS) {
  if (bulkIds.has(manifest.id)) {
    bulkDuplicate += 1;
    if (bulkDuplicate <= 5) report("bulk duplicate id", manifest.id);
  }
  bulkIds.add(manifest.id);
  const previewHtml: string = manifest.previewHtml;
  for (const pattern of FRESH_PATTERNS) {
    if (pattern.re.test(previewHtml)) {
      bulkFresh += 1;
      if (bulkFreshSamples.size < 10) {
        bulkFreshSamples.set(
          `${manifest.id} ${pattern.name}`,
          previewHtml.match(pattern.re)?.[0]?.slice(0, 120) ?? "(match)",
        );
      }
    }
  }
}
console.log(`COUNT bulk-total=${BULK_AUTO_ASSETS.length}`);
console.log(`COUNT bulk-duplicate-ids=${bulkDuplicate}`);
for (const [label, sample] of bulkFreshSamples) {
  report(`bulk fresh-pattern ${label}`, sample);
}
if (bulkFresh === 0) console.log("COUNT bulk-fresh-pattern-hits=0");

// 静态货架与大袋 id 互不撞车（测试既有口径，复验重申）。
let crossCollision = 0;
const staticIds = new Set(ASSET_CATALOG.map((manifest) => manifest.id));
for (const id of bulkIds) {
  if (staticIds.has(id)) {
    crossCollision += 1;
    report("bulk/static id collision", id);
  }
}
console.log(`COUNT bulk-static-id-collisions=${crossCollision}`);

// ── 5. 瘦身拆分对账（2026-10-05）：galaxy meta/bodies 必须与原大袋逐件等值 ──
// 拆分产物（galaxy-meta.ts + galaxy-bodies/*.ts）是 uiverse-*.ts 的派生物，
// 重跑生成器后忘了重跑拆分，这里当场抓住。
const { GALAXY_META } = await import(
  "../packages/ui/src/asset-library/catalog/generated/galaxy-meta.js"
);
if (GALAXY_META.length !== BULK_AUTO_ASSETS.length) {
  report("galaxy meta count", `meta=${GALAXY_META.length} bulk=${BULK_AUTO_ASSETS.length}`);
}
const CHUNKS = [
  "buttons",
  "cards",
  "checkbox",
  "forms",
  "inputs",
  "loaders",
  "notifications",
  "patterns",
  "radio-button",
  "toggle-switch",
  "tooltips",
] as const;
const splitBodies = new Map<string, { previewHtml: string; files: unknown }>();
for (const chunk of CHUNKS) {
  const mod = await import(
    `../packages/ui/src/asset-library/catalog/generated/galaxy-bodies/${chunk}.js`
  );
  for (const [id, body] of Object.entries(
    mod.GALAXY_BODIES as Record<string, { previewHtml: string; files: unknown }>,
  )) {
    if (splitBodies.has(id)) report("galaxy body duplicate id", id);
    splitBodies.set(id, body);
  }
}
let metaMismatch = 0;
for (const [index, original] of BULK_AUTO_ASSETS.entries()) {
  const entry = GALAXY_META[index];
  if (!entry || entry.id !== original.id) {
    report("galaxy meta order/id", `#${index}: ${entry?.id} vs ${original.id}`);
    metaMismatch += 1;
    if (metaMismatch > 5) break;
    continue;
  }
  const strip = (manifest: Record<string, unknown>) => {
    const { previewHtml: _p, files: _f, filesCount: _c, bodyFrom: _b, ...rest } = manifest;
    return rest;
  };
  if (
    JSON.stringify(strip(entry as unknown as Record<string, unknown>)) !==
    JSON.stringify(strip(original as unknown as Record<string, unknown>))
  ) {
    report("galaxy meta fields differ", original.id);
    metaMismatch += 1;
  }
  const body = splitBodies.get(original.id);
  if (!body) {
    report("galaxy body missing", original.id);
    metaMismatch += 1;
  } else if (
    body.previewHtml !== original.previewHtml ||
    JSON.stringify(body.files) !== JSON.stringify(original.files)
  ) {
    report("galaxy body differs from source", original.id);
    metaMismatch += 1;
  }
}
console.log(`COUNT galaxy-meta=${GALAXY_META.length} bodies=${splitBodies.size}`);

// 静态 react 货（bodyFrom="preview-html"）的预览 HTML 移出了 ASSET_CATALOG，
// 沙箱扫描在这里补位——真身照样过全模式扫描。
const { REACT_PREVIEW_HTML } = await import(
  "../packages/ui/src/asset-library/catalog/preview-html.js"
);
let reactScan = 0;
for (const [id, previewHtml] of Object.entries(REACT_PREVIEW_HTML)) {
  for (const pattern of FRESH_PATTERNS) {
    if (pattern.re.test(previewHtml)) {
      report(
        `react-preview[${id}] ${pattern.name}`,
        previewHtml.match(pattern.re)?.[0]?.slice(0, 120) ?? "(match)",
      );
    }
  }
  reactScan += 1;
}
console.log(`COUNT react-preview-html-scanned=${reactScan}`);

console.log(problems === 0 ? "VERDICT ALL-GREEN" : `VERDICT ${problems} PROBLEM(S)`);
if (problems > 0) process.exitCode = 1;
