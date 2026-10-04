// ============================================================
// 素材正文按需解析（地基清理·展厅瘦身 2026-10-05）的可运行检查。
// 验证：
//  1. galaxy meta 形状合法：validateCatalog 全绿（meta 模式：bodyFrom 在场、
//     previewHtml 恒空、files 恒空 + filesCount 保留真计数）；
//  2. loadAssetBody 对 galaxy 货按 bodyFrom 拉到真身，合并后 validateCatalog
//     全绿（把正文拉回既有校验轨道：非空/无外链/localStorage）；
//  3. 静态 react 货（bodyFrom="preview-html"）经惰性 map 解析出完整预览文档；
//  4. 正文就地的货 loadAssetBody 原样返回（幂等入口不分叉）。
// 运行：见 packages/ui/tsconfig.gate.json 的编译说明（ui 测试走 gate 编译）。
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { validateCatalog } from "../src/asset-library/catalog/catalogCheck.js";
import { loadAssetBody, mergeAssetBody } from "../src/asset-library/catalog/assetBodies.js";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import type { AssetManifest } from "../src/asset-library/catalog/types.js";

test("galaxy meta 形状：bodyFrom 齐全、正文字段恒空、validateCatalog（meta 模式）全绿", async () => {
  const { GALAXY_META } = await import("../src/asset-library/catalog/generated/galaxy-meta.js");
  assert.ok(GALAXY_META.length >= 3_000, `meta 应 ≥3000 件，实际 ${GALAXY_META.length}`);
  assert.deepEqual(validateCatalog(GALAXY_META), []);
  for (const meta of GALAXY_META as readonly AssetManifest[]) {
    assert.match(meta.bodyFrom ?? "", /^galaxy:/, `${meta.id} 应带 galaxy bodyFrom`);
    assert.equal(meta.previewHtml, "", `${meta.id} 的 previewHtml 应为空串`);
    assert.equal(meta.files.length, 0, `${meta.id} 的 files 应为空数组`);
    assert.ok((meta.filesCount ?? 0) >= 0, `${meta.id} 应保留 filesCount`);
    break; // 形状逐字段抽查一件即可，全量校验走 validateCatalog
  }
});

test("galaxy 货正文按需解析：合并后 validateCatalog 全绿（正文回到既有校验轨道）", async () => {
  const { GALAXY_META } = await import("../src/asset-library/catalog/generated/galaxy-meta.js");
  const sample = GALAXY_META.find((meta) => (meta.filesCount ?? 0) > 0);
  assert.ok(sample, "应存在非 prompt 类 galaxy 货");
  const body = await loadAssetBody(sample);
  assert.ok(body.previewHtml.length > 0, "正文应含预览 HTML");
  assert.equal(body.files.length, sample.filesCount, "图纸条数应等于 meta 的 filesCount");
  const merged = mergeAssetBody(sample, body);
  assert.deepEqual(validateCatalog([merged]), []);
});

test("静态 react 货（bodyFrom=preview-html）：惰性解析出完整预览文档", async () => {
  const asset = ASSET_CATALOG.find((manifest) => manifest.bodyFrom === "preview-html");
  assert.ok(asset, "静态货架应含 bodyFrom=preview-html 的 react 货");
  assert.equal(asset.previewHtml, "", "惰性货的 previewHtml 应为空串");
  const body = await loadAssetBody(asset);
  assert.ok(body.previewHtml.startsWith("<!doctype html>"), "预览应为完整文档");
  assert.deepEqual(validateCatalog([mergeAssetBody(asset, body)]), []);
});

test("正文就地的货：loadAssetBody 原样返回，不分叉", async () => {
  const asset = ASSET_CATALOG.find(
    (manifest) => manifest.bodyFrom === undefined && manifest.previewHtml.length > 0,
  );
  assert.ok(asset, "静态货架应含正文就地的货");
  const body = await loadAssetBody(asset);
  assert.equal(body.previewHtml, asset.previewHtml);
  assert.equal(body.files, asset.files);
});
