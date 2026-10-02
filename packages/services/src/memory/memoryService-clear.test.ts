// ============================================================
// 清空整本记事本（2026-10-02 拍板的「彻底失忆正门」）的可运行检查：
// 整目录删除（含面板看不见的非 .md 杂页）、目录不存在时幂等成功。
// 运行：npx tsx --test packages/services/src/memory/memoryService-clear.test.ts
// ============================================================

import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { resolveAgentMemoryRoot } from "@zcode/shared/node";
import { createMemoryService } from "./memoryService.js";

test("clearAgentMemoryFiles：整目录删除（含非 .md 杂页），再清一次幂等成功", async () => {
  const workspacePath = mkdtempSync(join(tmpdir(), "agent-memory-clear-"));
  const service = createMemoryService();
  const params = {
    agentName: "小明",
    agentId: "00000000-0000-4000-8000-000000000001",
    scope: "project" as const,
    workspacePath,
  };
  const rootDir = resolveAgentMemoryRoot({
    agentName: params.agentName,
    agentId: params.agentId,
    scope: "project",
    storageRoot: "",
    workspaceRoot: workspacePath,
  });
  mkdirSync(rootDir, { recursive: true });
  writeFileSync(join(rootDir, "MEMORY.md"), "# 索引\n");
  writeFileSync(join(rootDir, "hotpot.md"), "辣度中辣\n");
  writeFileSync(join(rootDir, "junk.txt"), "员工塞进来的非 md 杂页\n");

  await service.clearAgentMemoryFiles(params);
  assert.equal(existsSync(rootDir), false);

  // 再清一次：目录不存在按幂等成功，不抛错。
  await service.clearAgentMemoryFiles(params);
  assert.equal(existsSync(rootDir), false);
});
