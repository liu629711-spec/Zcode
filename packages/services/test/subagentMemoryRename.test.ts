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

async function renameToFfixture(fixture: Fixture, agentId?: string): Promise<void> {
  const service = createSubagentsService({ homeDir: join(fixture.root, "home") });
  await service.updateAgent({
    agentId: "old-name",
    config: {
      name: "new-name",
      description: "d",
      systemPrompt: "prompt",
      memory: "project",
      ...(agentId ? { agentId } : {}),
    },
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

test("改名搬家（D26 后落点是号目录）：目标缺席，记忆随改名迁到新 key，旧目录消失", async () => {
  const fixture = await makeFixture();
  try {
    await renameToFfixture(fixture, RENAME_ID);
    assert.equal(await exists(fixture.oldMemoryDir), false);
    assert.equal(
      await readFile(join(idMemoryDir(fixture, RENAME_ID), "MEMORY.md"), "utf-8"),
      MEMORY_NOTE,
    );
    assert.equal(await exists(join(fixture.agentsRoot, "old-name.md")), false);
    assert.equal(await exists(join(fixture.agentsRoot, "new-name.md")), true);
    assert.equal(
      await exists(fixture.newMemoryDir),
      false,
      "改名不再按新名字开目录：号在场即以号归位（同一次写入补发的号）",
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("改名搬家：目标是空壳目录（D25 幻影），删壳照搬，记忆不再被留在旧 key 下", async () => {
  const fixture = await makeFixture();
  try {
    // 读路径曾为改过名的旧会话现造这种空目录（core 已修，存量场景仍可出现）。
    await mkdir(idMemoryDir(fixture, RENAME_ID), { recursive: true });
    await renameToFfixture(fixture, RENAME_ID);
    assert.equal(await exists(fixture.oldMemoryDir), false, "旧目录必须搬走");
    assert.equal(
      await readFile(join(idMemoryDir(fixture, RENAME_ID), "MEMORY.md"), "utf-8"),
      MEMORY_NOTE,
      "空壳目标不挡搬家，内容必须落到新 key",
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("改名搬家：目标有内容，维持不覆盖不合并（保住两边数据），档案改名照常完成", async () => {
  const fixture = await makeFixture();
  try {
    await mkdir(idMemoryDir(fixture, RENAME_ID), { recursive: true });
    await writeFile(
      join(idMemoryDir(fixture, RENAME_ID), "MEMORY.md"),
      "号目录原件，不能动\n",
      "utf-8",
    );
    await renameToFfixture(fixture, RENAME_ID);
    assert.equal(await exists(fixture.oldMemoryDir), true, "旧目录原地保留");
    assert.equal(
      await readFile(join(idMemoryDir(fixture, RENAME_ID), "MEMORY.md"), "utf-8"),
      "号目录原件，不能动\n",
      "目标已有内容不被覆盖",
    );
    assert.equal(await exists(join(fixture.agentsRoot, "new-name.md")), true, "档案改名不受影响");
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

// ============================================================
// D26 片三「记事本按号归位」：上号那一次从名字目录搬到号目录，此后改名永不动目录。
// 号目录键 = `a-<uuid>`，与 shared/node resolveAgentMemoryKey 同一规则（两处推导
// 漂移就是「面板空白而员工在别处写」）。
// ============================================================

function idMemoryDir(fixture: Fixture, agentId: string): string {
  return join(fixture.memoryRoot, `a-${agentId}`);
}

const MEMORY_NOTE = "- [note](note.md) — 记了点事\n";

// 用例自带的号：搬家目标必须可预知，才能在测试里先造出「空壳目标 / 有内容目标」。
const RENAME_ID = "5b0e2a7c-1d3f-4c8a-9e2b-7a6d5c4b3a21";

test("上号即归位：老档案改名这次写入补号，记忆从名字目录搬到号目录", async () => {
  const fixture = await makeFixture();
  try {
    const service = createSubagentsService({ homeDir: join(fixture.root, "home") });
    const renamed = await service.updateAgent({
      agentId: "old-name",
      config: { name: "new-name", description: "d", systemPrompt: "prompt", memory: "project" },
      oldFilePath: fixture.oldProfilePath,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    const agentId = renamed.agent.agentId!;
    assert.equal(
      await readFile(join(idMemoryDir(fixture, agentId), "MEMORY.md"), "utf-8"),
      MEMORY_NOTE,
      "记事本跟着号走",
    );
    assert.equal(await exists(fixture.oldMemoryDir), false, "老名字目录搬空");
    assert.equal(
      await exists(join(fixture.memoryRoot, "new-name")),
      false,
      "不许再按新名字另开一个目录（两份记事本的事故源头）",
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test("有号档案再改名：号不变、目录不动，记忆原地（断链事故的正解）", async () => {
  const fixture = await makeFixture();
  try {
    const service = createSubagentsService({ homeDir: join(fixture.root, "home") });
    const first = await service.updateAgent({
      agentId: "old-name",
      config: { name: "new-name", description: "d", systemPrompt: "prompt", memory: "project" },
      oldFilePath: fixture.oldProfilePath,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    const issuedId = first.agent.agentId!;

    const second = await service.updateAgent({
      agentId: first.agent.id,
      config: { name: "third-name", description: "d", systemPrompt: "prompt", memory: "project" },
      oldFilePath: join(fixture.agentsRoot, "new-name.md"),
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });

    assert.equal(second.agent.agentId, issuedId, "改名不换号");
    assert.equal(
      await readFile(join(idMemoryDir(fixture, issuedId), "MEMORY.md"), "utf-8"),
      MEMORY_NOTE,
      "目录原地不动",
    );
    assert.equal(
      await exists(join(fixture.memoryRoot, "third-name")),
      false,
      "有号的档案不再按名字落目录",
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});
