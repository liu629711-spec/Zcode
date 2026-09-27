// ============================================================
// persona-json 编解码的可运行检查（G1 存储边界：往返 + 容错）
// ============================================================
// encode/decode 是纯函数：合法快照往返一致；NULL/坏 JSON/形状漂移一律
// undefined + warn，不抛——persona_json 是历史行可缺席的新列，脏数据不能
// 阻断整条会话读取。
//
// 本文件参与 tsc 构建（adapters 的 tsconfig 未排除 *.test.ts），import 一律 .js 后缀。
//
// 运行：npx tsx --test apps/zcode-cli/packages/adapters/src/storage/session-store/persona-json.test.ts

import assert from "node:assert/strict";
import { test } from "node:test";
import { decodeSessionPersonaJson, encodeSessionPersonaJson } from "./persona-json.js";

const persona = {
  name: "code-reviewer",
  systemPrompt: "你是代码审查员",
  memoryScope: "project" as const,
};

test("encode/decode 往返：完整快照（含 memoryScope）一致", () => {
  const raw = encodeSessionPersonaJson(persona);
  assert.ok(raw);
  assert.deepEqual(JSON.parse(raw), persona);
  assert.deepEqual(decodeSessionPersonaJson(raw), persona);
});

test("encode/decode 往返：无 memoryScope 时缺省键不复活", () => {
  const raw = encodeSessionPersonaJson({ name: "小助手", systemPrompt: "你好" });
  assert.ok(raw);
  assert.deepEqual(decodeSessionPersonaJson(raw), { name: "小助手", systemPrompt: "你好" });
});

test("encode：undefined → null（普通会话行值 NULL）", () => {
  assert.equal(encodeSessionPersonaJson(undefined), null);
});

test("decode：null/空串 → undefined，不抛", () => {
  assert.equal(decodeSessionPersonaJson(null), undefined);
  assert.equal(decodeSessionPersonaJson(""), undefined);
  assert.equal(decodeSessionPersonaJson(undefined), undefined);
});

test("decode：坏 JSON → undefined + warn，不抛", () => {
  const warnings: unknown[][] = [];
  const original = console.warn;
  console.warn = (...args: unknown[]) => warnings.push(args);
  try {
    assert.equal(decodeSessionPersonaJson("{not-json"), undefined);
  } finally {
    console.warn = original;
  }
  assert.equal(warnings.length, 1);
});

test("decode：形状漂移（缺 name/systemPrompt/非对象）→ undefined，旧行照常降级", () => {
  for (const raw of [
    JSON.stringify({ systemPrompt: "只有正文" }),
    JSON.stringify({ name: "只有名字" }),
    JSON.stringify("stranger"),
    JSON.stringify(null),
    "null",
  ]) {
    assert.equal(decodeSessionPersonaJson(raw), undefined, raw);
  }
});
