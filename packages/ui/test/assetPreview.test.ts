import assert from "node:assert/strict";
import test from "node:test";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import {
  ASSET_SANDBOX,
  assertSandboxSafe,
  validateCatalog,
} from "../src/asset-library/catalog/catalogCheck.js";
import { REACT_PREVIEW_HTML } from "../src/asset-library/catalog/preview-html.js";

// ============================================================
// react 货预编译产物的断言（技术设计 §6：build-asset-previews 产物断言）
// ============================================================
// 钉的是：每件 react 货都有产物、产物自包含（无外链/无 localStorage）、
// react 确实内联进了产物。运行：
// npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/assetPreview.test.ts
// 产物缺失/过期时：仓库根 node scripts/build-asset-previews.mjs 重新生成。

/** react 货判据，与 scripts/build-asset-previews.mjs 同源：files[0] 是 tsx。 */
const reactAssets = ASSET_CATALOG.filter((asset) => asset.files[0]?.language === "tsx");

test("react 货都有非空预编译产物，且产物表不收录目录外 id", () => {
  assert.ok(reactAssets.length >= 2, "至少要有 2 件 react 种子货");
  for (const asset of reactAssets) {
    const html = REACT_PREVIEW_HTML[asset.id];
    assert.ok(
      typeof html === "string" && html.trim() !== "",
      `${asset.id} 缺预编译产物（跑 node scripts/build-asset-previews.mjs）`,
    );
  }
  for (const id of Object.keys(REACT_PREVIEW_HTML)) {
    assert.ok(
      ASSET_CATALOG.some((asset) => asset.id === id),
      `产物表收录了目录外的 id：${id}`,
    );
  }
});

test("产物自包含：无外链 script、无 localStorage，react 确实内联", () => {
  for (const [id, html] of Object.entries(REACT_PREVIEW_HTML)) {
    assert.ok(!html.includes('<script src="http'), `${id}: 产物含外链 script`);
    assert.ok(!/src\s*=\s*["']https?:/i.test(html), `${id}: 产物含外链资源`);
    assert.ok(!html.includes("localStorage"), `${id}: 产物含 localStorage（沙箱无源环境会抛）`);
    // react 打包标记由构建脚本在 bundle 成功后写入；"Minified React error" 是
    // react-dom production 里的自然字符串，两道一起钉"react 真被打进去了"
    assert.ok(html.includes("__ASSET_REACT_BUNDLE__"), `${id}: 缺 react 打包标记`);
    assert.ok(html.includes("Minified React error"), `${id}: 缺 react-dom 产物字符串`);
    assert.ok(html.length > 100_000, `${id}: 产物过小（${html.length}B），react 可能没打进去`);
  }
});

test("validateCatalog 对含 react 货的全目录仍全绿，且 previewHtml 已接线", () => {
  assert.deepEqual(validateCatalog(ASSET_CATALOG), []);
  for (const asset of reactAssets) {
    assert.equal(
      asset.previewHtml,
      REACT_PREVIEW_HTML[asset.id],
      `${asset.id}: previewHtml 应从 REACT_PREVIEW_HTML 接线（catalog/index.ts）`,
    );
  }
});

test("assertSandboxSafe 负例照旧：混入其它 token 一律 throw", () => {
  assert.equal(ASSET_SANDBOX, "allow-scripts");
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
