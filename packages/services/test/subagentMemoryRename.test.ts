import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { createSubagentsService } from "../src/subagents/subagentsService.js";

// ============================================================
// G6 改名搬家 × D25 空壳目标：updateAgent 全流程的可运行检查。
// 真实文件系统（mkdtemp），workspace 作用域自包含、不依赖全局数据目录。
//
// 运行：npx tsx --test packages/services/test/subagentMemoryRename.test.ts
// ============================================================

const OLD_PROFILE_MD = `---
name: "old-name"
description: "d"
memory: project
---

prompt
`;

interface Fixture {
  root: string;
  workspacePath: string;
  agentsRoot: string;
  memoryRoot: string;
  oldProfilePath: string;
  oldMemoryDir: string;
  newMemoryDir: string;
}

async function makeFixture(): Promise<Fixture> {
  const root = await mkdtemp(join(tmpdir(), "zcode-agent-rename-"));
  const workspacePath = join(root, "ws");
  const agentsRoot = join(workspacePath, ".zcode", "agents");
  const memoryRoot = join(workspacePath, ".zcode", "agent-memory");
  const oldProfilePath = join(agentsRoot, "old-name.md");
  const oldMemoryDir = join(memoryRoot, "old-name");
  const newMemoryDir = join(memoryRoot, "new-name");
  await mkdir(agentsRoot, { recursive: true });
  await mkdir(oldMemoryDir, { recursive: true });
  await writeFile(join(oldMemoryDir, "MEMORY.md"), "- [note](note.md) — 记了点事\n", "utf-8");
  await writeFile(oldProfilePath, OLD_PROFILE_MD, "utf-8");
  return { root, workspacePath, agentsRoot, memoryRoot, oldProfilePath, oldMemoryDir, newMemoryDir };
}

async function renameToFfixture(fixture: Fixture): Promise<void> {
  const service = createSubagentsService({ homeDir: join(fixture.root, "home") });
  await service.updateAgent({
    agentId: "old-name",
    config: { name: "new-name", description: "d", systemPrompt: "prompt", memory: "project" },
    oldFilePath: fixture.oldProfilePath,
    scope: "workspace",
    workspacePath: fixture.workspacePath,
  });
}

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

test("改名搬家：目标缺席，记忆随改名迁到新 key，旧目录消失", async () => {
  const fixture = await makeFixture();
  try {
    await renameToFfixture(fixture);
    assert.equal(await exists(fixture.oldMemoryDir), false);
    assert.equal(
      await readFile(join(fixture.newMemoryDir, "MEMORY.md"), "utf-8"),
      "- [note](note.md) — 记了点事\n",
    );
    assert.equal(await exists(join(fixture.agentsRoot, "old-name.md")), false);
    assert.equal(await exists(join(fixture.agentsRoot, "new-name.md")), true);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("改名搬家：目标是空壳目录（D25 幻影），删壳照搬，记忆不再被留在旧 key 下", async () => {
  const fixture = await makeFixture();
  try {
    // 读路径曾为改过名的旧会话现造这种空目录（core 已修，存量场景仍可出现）。
    await mkdir(fixture.newMemoryDir, { recursive: true });
    await renameToFfixture(fixture);
    assert.equal(await exists(fixture.oldMemoryDir), false, "旧目录必须搬走");
    assert.equal(
      await readFile(join(fixture.newMemoryDir, "MEMORY.md"), "utf-8"),
      "- [note](note.md) — 记了点事\n",
      "空壳目标不挡搬家，内容必须落到新 key",
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("改名搬家：目标有内容，维持不覆盖不合并（保住两边数据），档案改名照常完成", async () => {
  const fixture = await makeFixture();
  try {
    await mkdir(fixture.newMemoryDir, { recursive: true });
    await writeFile(join(fixture.newMemoryDir, "other.md"), "已有内容，不能动\n", "utf-8");
    await renameToFfixture(fixture);
    assert.equal(await exists(fixture.oldMemoryDir), true, "旧目录原地保留");
    assert.equal(
      await readFile(join(fixture.newMemoryDir, "other.md"), "utf-8"),
      "已有内容，不能动\n",
      "目标已有内容不被覆盖",
    );
    assert.equal(await exists(join(fixture.agentsRoot, "new-name.md")), true, "档案改名不受影响");
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});
