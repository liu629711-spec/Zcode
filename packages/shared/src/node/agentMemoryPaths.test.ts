// ============================================================
// agentMemoryPaths 的可运行检查（tsx --test）：core 记忆注入与 services
// 记忆面板共用的目录推导，key 清洗与三档 scope 落点漂移即失忆/空面板。
//
// 运行：npx tsx --test packages/shared/src/node/agentMemoryPaths.test.ts
// ============================================================

import assert from "node:assert/strict";
import { join, resolve } from "node:path";
import { test } from "node:test";

import { resolveAgentMemoryRoot, sanitizeAgentMemoryKey } from "./agentMemoryPaths.js";

test("key 清洗：非法字符换成连字符，清空兜底 unknown，合法名原样保留", () => {
  assert.equal(sanitizeAgentMemoryKey("code-reviewer"), "code-reviewer");
  assert.equal(sanitizeAgentMemoryKey("Code Reviewer!"), "Code-Reviewer-");
  // 非法字符是逐个替换成连字符，长度不变；只有空名才落到 unknown 兜底。
  assert.equal(sanitizeAgentMemoryKey("代码评审员"), "-----");
  assert.equal(sanitizeAgentMemoryKey(""), "unknown");
});

test("project 落点：<ws>/.zcode/agent-memory/<key>/，相对 workspace 先转绝对", () => {
  const root = resolveAgentMemoryRoot({
    agentName: "code-reviewer",
    scope: "project",
    storageRoot: "",
    workspaceRoot: "/tmp/ws",
  });
  assert.equal(root, join(resolve("/tmp/ws"), ".zcode", "agent-memory", "code-reviewer"));
});

test("local 落点：<ws>/.zcode/agent-memory-local/<key>/（与 project 不同目录）", () => {
  const root = resolveAgentMemoryRoot({
    agentName: "code-reviewer",
    scope: "local",
    storageRoot: "",
    workspaceRoot: "/tmp/ws",
  });
  assert.equal(root, join(resolve("/tmp/ws"), ".zcode", "agent-memory-local", "code-reviewer"));
});

test("user 落点：<storageRoot>/agent-memory/<key>/，与工作区无关", () => {
  const root = resolveAgentMemoryRoot({
    agentName: "code-reviewer",
    scope: "user",
    storageRoot: "/home/u/.zcode",
    workspaceRoot: "/tmp/ws",
  });
  assert.equal(root, join("/home/u/.zcode", "agent-memory", "code-reviewer"));
});
