import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { normalizeAgentId } from "@zcode/shared";
import { resolveUserSubagentRoot } from "../src/subagents/subagentStorage.js";
import { createSubagentsService } from "../src/subagents/subagentsService.js";

// ============================================================
// 一键补号（D26 收尾，2026-10-08）的可运行检查。
// 无号老档案走 updateAgent 同款链补号（读旧档 → previous 在场才补 → 搬家 →
// 落盘）；已有号的档案与坏 frontmatter 原地不动。测试夹具用 memory:"project"
// 的档案 + 无 workspacePath → 搬家计划缺席（不触碰真实用户目录，保持密闭）。
// 运行：npx tsx --test packages/services/test/subagentAgentIdBackfill.test.ts
// ============================================================

async function makeFixture(): Promise<{ root: string; agentsRoot: string }> {
  const root = await mkdtemp(join(tmpdir(), "zcode-agent-backfill-"));
  const agentsRoot = await resolveUserSubagentRoot({ homeDir: join(root, "home") });
  await mkdir(agentsRoot, { recursive: true });
  return { root, agentsRoot };
}

async function cleanup(root: string): Promise<void> {
  await rm(root, { recursive: true, force: true });
}

function serviceOf(root: string) {
  return createSubagentsService({ homeDir: join(root, "home") });
}

test("一键补号：无号档案上号且内容保真，已有号与坏档不动", async () => {
  const fixture = await makeFixture();
  try {
    await writeFile(
      join(fixture.agentsRoot, "legacy-agent.md"),
      [
        "---",
        'name: "legacy-agent"',
        'description: "老档案，没号"',
        // 引号形式（与序列化器一致）：parse 不认裸值是既有行为，非本批修理对象。
        'color: "orange"',
        "---",
        "",
        "先把目录摸清再动手。",
      ].join("\n"),
      "utf-8",
    );
    await writeFile(
      join(fixture.agentsRoot, "numbered-agent.md"),
      [
        "---",
        "name: numbered-agent",
        "description: d",
        'agentId: "11111111-2222-4333-8444-555555555555"',
        "---",
        "",
        "p",
      ].join("\n"),
      "utf-8",
    );
    await writeFile(join(fixture.agentsRoot, "broken.md"), "没有 frontmatter 的正文", "utf-8");

    const result = await serviceOf(fixture.root).backfillAgentIds();

    assert.equal(result.assigned.length, 1, "只有无号档案上号");
    assert.equal(result.assigned[0]?.name, "legacy-agent");
    const agentId = result.assigned[0]?.agentId;
    assert.ok(normalizeAgentId(agentId), "补发的必须是合法 uuid");

    const backfilled = await readFile(join(fixture.agentsRoot, "legacy-agent.md"), "utf-8");
    assert.ok(backfilled.includes(`agentId: ${agentId}`));
    // 序列化值会带引号（字符串 frontmatter 统一引号化），查值本身不查原文行。
    assert.ok(backfilled.includes("老档案，没号"), "内容保真：description 不丢");
    assert.ok(backfilled.includes("orange"), "内容保真：color 不丢");
    assert.ok(backfilled.includes("先把目录摸清再动手。"), "内容保真：正文不丢");

    const untouched = await readFile(join(fixture.agentsRoot, "numbered-agent.md"), "utf-8");
    assert.ok(untouched.includes("11111111-2222-4333-8444-555555555555"), "已有号原样");
    const broken = await readFile(join(fixture.agentsRoot, "broken.md"), "utf-8");
    assert.equal(broken, "没有 frontmatter 的正文", "坏 frontmatter 原地不动");
  } finally {
    await cleanup(fixture.root);
  }
});

test("一键补号：二次运行幂等（都已有号，assigned 为空）", async () => {
  const fixture = await makeFixture();
  try {
    await writeFile(
      join(fixture.agentsRoot, "legacy-agent.md"),
      '---\nname: legacy-agent\ndescription: d\n---\n\np',
      "utf-8",
    );
    const service = serviceOf(fixture.root);
    const first = await service.backfillAgentIds();
    assert.equal(first.assigned.length, 1);
    const second = await service.backfillAgentIds();
    assert.equal(second.assigned.length, 0, "二次运行没有可补的档案");
  } finally {
    await cleanup(fixture.root);
  }
});
