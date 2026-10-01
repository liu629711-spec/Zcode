// ============================================================
// matchImportedActor 的 profile 关闸（班底进图纸，评审 A1）可运行检查。
// 运行：npx tsx --test apps/zcode-cli/packages/dynamic-workflow/src/engine/imported-cache.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { ImportedRunCache } from "./imported-cache-types.js";
import { matchImportedActor } from "./imported-cache.js";

const CACHE: ImportedRunCache = {
  actors: new Map([
    [
      "审查",
      {
        persona: { name: "审查" },
        entries: [],
        transcriptSourceSessionId: "sess-prev",
      },
    ],
  ]),
  world: new Map(),
};

test("点名员工的 actor 永不命中修订缓存：引用串相同也全新重跑（活引用防陈旧身份）", () => {
  // 引用串与前驱一致（都是没有 profile 的比较对象不存在——前驱根本没记 profile 的解析结果，
  // 同引用不同名册内容无法证同）：直接关闸。
  assert.equal(matchImportedActor(CACHE, { name: "审查", profile: "code-reviewer" }), undefined);
});

test("没点名的 actor 照旧命中（基线回归）", () => {
  const state = matchImportedActor(CACHE, { name: "审查" });
  assert.ok(state !== undefined, "同名同 persona 应命中");
});

test("persona 不一致照旧弃候选（基线回归）", () => {
  assert.equal(matchImportedActor(CACHE, { name: "审查", system: "换人设" }), undefined);
});

test("无缓存/匿名照旧返回 undefined（基线回归）", () => {
  assert.equal(matchImportedActor(undefined, { name: "审查" }), undefined);
  assert.equal(matchImportedActor(CACHE, {}), undefined);
});
