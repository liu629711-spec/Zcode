import assert from "node:assert/strict";
import test from "node:test";
import { filterAssets } from "../src/asset-library/assetFilter.js";
import type { AssetManifest } from "../src/asset-library/catalog/types.js";

// ============================================================
// 展厅搜索/分类过滤纯函数的可运行检查（技术设计 §6）
// ============================================================
// 钉的是 filterAssets 的匹配语义：分类过滤、各字段命中、大小写、
// 空查询全量、无结果空数组、En 字段缺失回退（titleEn ?? title）。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/assetLibraryFilter.test.ts

/** 最小合法 manifest，只带测试关心的字段。 */
function asset(
  overrides: Partial<AssetManifest> & Pick<AssetManifest, "id" | "title" | "category">,
): AssetManifest {
  return {
    description: "",
    tags: [],
    previewHtml: "<!doctype html>",
    files: [],
    prompt: "",
    ...overrides,
  };
}

const FIXTURES: AssetManifest[] = [
  asset({
    id: "jelly-toggle",
    title: "果冻开关",
    titleEn: "Jelly Toggle",
    description: "纯 CSS 回弹开关",
    descriptionEn: "A springy pure-CSS toggle",
    category: "control",
    tags: ["css", "控件"],
  }),
  asset({
    id: "typewriter-text",
    title: "打字机文本",
    category: "text-animation",
    tags: ["文字", "typewriter"],
  }),
  asset({
    id: "meteor-background",
    title: "流星背景",
    titleEn: "Meteor Background",
    description: "canvas 星野衬底",
    category: "background",
    tags: ["canvas", "暗色"],
  }),
  asset({
    id: "frosted-glass-login-prompt",
    title: "毛玻璃登录页",
    description: "成套口令卡",
    category: "prompt",
    tags: ["口令"],
  }),
];

test("逐类过滤：五类各归各类，all 返回全量", () => {
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "", category: "control" }).map((m) => m.id),
    ["jelly-toggle"],
  );
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "", category: "text-animation" }).map((m) => m.id),
    ["typewriter-text"],
  );
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "", category: "background" }).map((m) => m.id),
    ["meteor-background"],
  );
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "", category: "prompt" }).map((m) => m.id),
    ["frosted-glass-login-prompt"],
  );
  assert.equal(filterAssets(FIXTURES, { query: "", category: "block" }).length, 0);
  assert.equal(filterAssets(FIXTURES, { query: "", category: "all" }).length, FIXTURES.length);
});

test("查询命中各字段：title / titleEn / tag / description / descriptionEn", () => {
  // title
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "打字机", category: "all" }).map((m) => m.id),
    ["typewriter-text"],
  );
  // titleEn（未设 En 的货也能靠 title 命中，见 locale 回退用例）
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "meteor", category: "all" }).map((m) => m.id),
    ["meteor-background"],
  );
  // tag
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "canvas", category: "all" }).map((m) => m.id),
    ["meteor-background"],
  );
  // description
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "星野", category: "all" }).map((m) => m.id),
    ["meteor-background"],
  );
  // descriptionEn
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "springy", category: "all" }).map((m) => m.id),
    ["jelly-toggle"],
  );
});

test("大小写不敏感且首尾空白忽略", () => {
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "  JELLY  ", category: "all" }).map((m) => m.id),
    ["jelly-toggle"],
  );
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "TYPEWRITER", category: "all" }).map((m) => m.id),
    ["typewriter-text"],
  );
});

test("空查询/纯空白查询返回全量", () => {
  assert.equal(filterAssets(FIXTURES, { query: "", category: "all" }).length, 4);
  assert.equal(filterAssets(FIXTURES, { query: "   ", category: "all" }).length, 4);
});

test("无结果返回空数组（不抛错）", () => {
  assert.deepEqual(filterAssets(FIXTURES, { query: "不存在的词xyz", category: "all" }), []);
  // 命中关键词但分类不匹配 → 同样空
  assert.deepEqual(filterAssets(FIXTURES, { query: "果冻", category: "background" }), []);
});

test("locale 回退：titleEn 缺失时用 title（titleEn ?? title）", () => {
  // typewriter-text 没有 titleEn：英文风格的 En 查询只能靠 title 兜底命中
  assert.deepEqual(
    filterAssets(FIXTURES, { query: "打字机文本", category: "all" }).map((m) => m.id),
    ["typewriter-text"],
  );
  // 有 titleEn 的货：原字段与 En 字段都命中
  assert.ok(
    filterAssets(FIXTURES, { query: "流星背景", category: "all" })
      .map((m) => m.id)
      .includes("meteor-background"),
  );
  assert.ok(
    filterAssets(FIXTURES, { query: "Meteor", category: "all" })
      .map((m) => m.id)
      .includes("meteor-background"),
  );
});
