// ============================================================
// 员工（persona）会话不吃老板项目记忆的可运行检查：
// projectMemoryEnabled=false 只关老板记忆根，不影响自家记事本柜的判定输入。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/runtime/helpers/project-memory.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { AgentRuntimeConfig } from "../types.ts";
import { resolveEnabledProjectMemoryRoot } from "./project-memory.ts";

const base = {
  enabled: true,
  use: true,
  cliStorageRoot: "C:/fake/cli-storage",
  storageRoot: "C:/fake/storage",
} as const;

test("projectMemoryEnabled=false → 老板项目记忆根解析为 undefined", () => {
  const root = resolveEnabledProjectMemoryRoot(
    { memory: { ...base, projectMemoryEnabled: false } } as AgentRuntimeConfig,
    "D:/some/workspace",
  );
  assert.equal(root, undefined);
});

test("缺省（未传 flag）→ 行为不变，仍解析出 memories/projects 根", () => {
  const root = resolveEnabledProjectMemoryRoot(
    { memory: { ...base } } as AgentRuntimeConfig,
    "D:/some/workspace",
  );
  assert.ok(root);
  assert.ok(root.includes("memories"), root);
  assert.ok(root.includes("projects"), root);
});

test("enabled=false（总闸关）→ 依旧 undefined（既有行为钉住）", () => {
  const root = resolveEnabledProjectMemoryRoot(
    { memory: { ...base, enabled: false } } as AgentRuntimeConfig,
    "D:/some/workspace",
  );
  assert.equal(root, undefined);
});
