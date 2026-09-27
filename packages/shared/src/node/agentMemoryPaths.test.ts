// ============================================================
// agentMemoryPaths 的可运行检查（tsx --test）：core 记忆注入与 services
// 记忆面板共用的目录推导，key 清洗与三档 scope 落点漂移即失忆/空面板。
//
// 运行：npx tsx --test packages/shared/src/node/agentMemoryPaths.test.ts
// ============================================================

import assert from "node:assert/strict";
import { join, resolve } from "node:path";
import { test } from "node:test";

import {
  planAgentMemoryDirectoryRename,
  resolveAgentMemoryRoot,
  sanitizeAgentMemoryKey,
} from "./agentMemoryPaths.js";

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

// 改名迁移（G6）：档案改名后记事本目录跟着走。触发条件是 key 变化（不是文件路径
// 变化——文件名小写化而 key 保大小写），scope 或落点参数不足以定位根目录就不动。
test("改名迁移：project 档改名后目录跟到新 key", () => {
  const plan = planAgentMemoryDirectoryRename({
    previousAgentName: "code-reviewer",
    nextAgentName: "code-reviewer-v2",
    memoryScope: "project",
    workspacePath: "/tmp/ws",
  });
  assert.equal(plan?.fromDir, join(resolve("/tmp/ws"), ".zcode", "agent-memory", "code-reviewer"));
  assert.equal(
    plan?.toDir,
    join(resolve("/tmp/ws"), ".zcode", "agent-memory", "code-reviewer-v2"),
  );
});

test("改名迁移：纯大小写改名也触发（文件路径不变但 key 变了）", () => {
  const plan = planAgentMemoryDirectoryRename({
    previousAgentName: "Alpha",
    nextAgentName: "alpha",
    memoryScope: "project",
    workspacePath: "/tmp/ws",
  });
  assert.equal(plan?.fromDir, join(resolve("/tmp/ws"), ".zcode", "agent-memory", "Alpha"));
  assert.equal(plan?.toDir, join(resolve("/tmp/ws"), ".zcode", "agent-memory", "alpha"));
});

test("改名迁移：sanitize 后 key 相同（只是非法字符写法变化）不迁", () => {
  assert.equal(
    planAgentMemoryDirectoryRename({
      previousAgentName: "code reviewer",
      nextAgentName: "code*reviewer",
      memoryScope: "project",
      workspacePath: "/tmp/ws",
    }),
    undefined,
  );
});

test("改名迁移：user 档跟 userMemoryRoot，缺根不迁；workspace 档缺工作区不迁", () => {
  const userPlan = planAgentMemoryDirectoryRename({
    previousAgentName: "Alpha",
    nextAgentName: "beta",
    memoryScope: "user",
    userMemoryRoot: "/home/u/.zcode",
  });
  assert.equal(userPlan?.fromDir, join("/home/u/.zcode", "agent-memory", "Alpha"));
  assert.equal(userPlan?.toDir, join("/home/u/.zcode", "agent-memory", "beta"));
  assert.equal(
    planAgentMemoryDirectoryRename({
      previousAgentName: "Alpha",
      nextAgentName: "beta",
      memoryScope: "user",
    }),
    undefined,
  );
  assert.equal(
    planAgentMemoryDirectoryRename({
      previousAgentName: "Alpha",
      nextAgentName: "beta",
      memoryScope: "project",
      workspacePath: "  ",
    }),
    undefined,
  );
});
