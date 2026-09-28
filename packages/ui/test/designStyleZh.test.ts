import assert from "node:assert/strict";
import test from "node:test";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import { DESIGN_STYLE_ZH, resolveDesignStyleZhName } from "../src/asset-library/catalog/designStyleZh.js";

// ============================================================
// 设计风格中文名对照（V4.6 用户反馈：Airbnb 这类原名不好理解）
// ============================================================
// 钉的是：目录里每一件设计风格货都有中文名条目——新增收录货时必须补中文，
// 否则面板/推荐条/chip 会回落英文原名（测试直接炸出来提醒补表）。

test("设计风格中文对照：目录里每件 design-style 货都有中文名", () => {
  const designStyleAssets = ASSET_CATALOG.filter((asset) => asset.category === "design-style");
  assert.ok(designStyleAssets.length > 0, "目录应存在设计风格货");
  const missing = designStyleAssets
    .filter((asset) => !DESIGN_STYLE_ZH[asset.id.replace(/^design-/, "")])
    .map((asset) => asset.id);
  assert.deepEqual(missing, [], `以下设计风格缺中文名：${missing.join(", ")}`);
});

test("中文名解析：命中对照表返回中文名，未命中回落英文原名", () => {
  assert.equal(resolveDesignStyleZhName("design-airbnb", "Airbnb"), "Airbnb 民宿风");
  assert.equal(resolveDesignStyleZhName("design-notion", "Notion"), "Notion 文档风");
  assert.equal(resolveDesignStyleZhName("design-unknown-style", "Unknown Style"), "Unknown Style");
});
