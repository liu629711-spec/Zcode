import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { normalizeAgentId, ZCODE_AGENT_PROVIDER } from "@zcode/shared";
import {
  parseSubagentMarkdown,
  serializeSubagentMarkdown,
} from "../src/subagents/subagentMarkdown.js";
import { createSubagentsService } from "../src/subagents/subagentsService.js";

// ============================================================
// D26 片一「员工工号」：档案 frontmatter `agentId` 的形状判据 + 发号/保号规矩。
// 病根：会话↔档案↔记事本过去全靠名字对号，改名必断一处（真机连出三次事故）。
// 号一旦落盘就是这个人的一生凭证，任何调用方都不得改写。
//
// 运行：npx tsx --test packages/services/test/subagentAgentId.test.ts
// ============================================================

const AGENT_ID_A = "11111111-2222-4333-8444-555555555555";
const AGENT_ID_B = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

function profileMarkdown(fields: string, body = "prompt\n"): string {
  return `---\n${fields}---\n\n${body}`;
}

function parseProfile(content: string, path = "/tmp/an-agent.md") {
  return parseSubagentMarkdown({ content, path, scope: "workspace" });
}

interface Fixture {
  root: string;
  workspacePath: string;
  agentsRoot: string;
}

async function makeFixture(): Promise<Fixture> {
  const root = await mkdtemp(join(tmpdir(), "zcode-agent-id-"));
  const workspacePath = join(root, "ws");
  const agentsRoot = join(workspacePath, ".zcode", "agents");
  await mkdir(agentsRoot, { recursive: true });
  return { root, workspacePath, agentsRoot };
}

async function cleanup(fixture: Fixture): Promise<void> {
  await rm(fixture.root, { recursive: true, force: true });
}

function serviceOf(fixture: Fixture) {
  return createSubagentsService({ homeDir: join(fixture.root, "home") });
}

async function readProfileAt(path: string): Promise<string> {
  return readFile(path, "utf-8");
}

test("形状判据：uuid 收（大小写归一），其余一律当无号", () => {
  assert.equal(normalizeAgentId(AGENT_ID_A), AGENT_ID_A);
  assert.equal(normalizeAgentId(AGENT_ID_A.toUpperCase()), AGENT_ID_A);
  assert.equal(normalizeAgentId(`  ${AGENT_ID_A}  `), AGENT_ID_A, "手写 frontmatter 的余空格");
  assert.equal(normalizeAgentId("not-a-uuid"), undefined);
  assert.equal(normalizeAgentId("11111111-2222-4333-8444-55555555555"), undefined, "半截 uuid");
  assert.equal(normalizeAgentId(undefined), undefined);
  assert.equal(normalizeAgentId(42), undefined);
});

test("parse / serialize 往返回号：号写在名字下面一行，无号不凭空造行", () => {
  const withId = parseProfile(
    profileMarkdown(`name: "an-agent"\ndescription: "d"\nagentId: "${AGENT_ID_A}"\n`),
  );
  assert.equal(withId.diagnostic, undefined);
  assert.equal(withId.agent?.agentId, AGENT_ID_A);

  const reserialized = serializeSubagentMarkdown(withId.agent!);
  assert.ok(reserialized.includes(`agentId: ${AGENT_ID_A}`), "序列化写号（appendScalar 不加引号）");
  assert.equal(parseProfile(reserialized).agent?.agentId, AGENT_ID_A);

  const legacy = parseProfile(profileMarkdown('name: "an-agent"\ndescription: "d"\n'));
  assert.equal(legacy.agent?.agentId, undefined, "老档案没有号这一行照样能读");
  assert.ok(
    !serializeSubagentMarkdown(legacy.agent!).includes("agentId"),
    "无号档案序列化不出 agentId 行",
  );
});

test("坏号当无号：非法形状不落进 AgentSummary，序列化也不带回（防半截 uuid 传染）", () => {
  const broken = parseProfile(
    profileMarkdown('name: "an-agent"\ndescription: "d"\nagentId: "我是工号"\n'),
  );
  assert.equal(broken.agent?.agentId, undefined);
  assert.ok(!serializeSubagentMarkdown(broken.agent!).includes("agentId"));
});

test("createAgent：新建即发号，两次新建的号互不相同", async () => {
  const fixture = await makeFixture();
  try {
    const service = serviceOf(fixture);
    const first = await service.createAgent({
      config: { name: "first-agent", description: "d", systemPrompt: "p", memory: "project" },
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.ok(normalizeAgentId(first.agent.agentId), "返回的号必须是合法 uuid");
    const onDisk = await readProfileAt(join(fixture.agentsRoot, "first-agent.md"));
    assert.ok(onDisk.includes(`agentId: ${first.agent.agentId}`), "号必须写在档案里");

    const second = await service.createAgent({
      config: { name: "second-agent", description: "d", systemPrompt: "p", memory: "project" },
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.notEqual(second.agent.agentId, first.agent.agentId);
  } finally {
    await cleanup(fixture);
  }
});

test("createAgent 自带号就采信（内置班底包档案自带号，装上全机共用同一凭证）", async () => {
  const fixture = await makeFixture();
  try {
    const created = await serviceOf(fixture).createAgent({
      config: {
        name: "preset-agent",
        description: "d",
        systemPrompt: "p",
        agentId: AGENT_ID_B,
        memory: "project",
      },
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.equal(created.agent.agentId, AGENT_ID_B);
  } finally {
    await cleanup(fixture);
  }
});

test("改名保号：盘上的号是一等公民，调用方带什么都不作数", async () => {
  const fixture = await makeFixture();
  try {
    const service = serviceOf(fixture);
    const created = await service.createAgent({
      config: { name: "old-name", description: "d", systemPrompt: "p", memory: "project" },
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    const issued = created.agent.agentId!;

    // 改名走的是同一条 update：号必须原地不动，这是 D26 的全部意义。
    const renamed = await service.updateAgent({
      agentId: created.agent.id,
      config: { name: "new-name", description: "d", systemPrompt: "p", memory: "project" },
      oldFilePath: join(fixture.agentsRoot, "old-name.md"),
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.equal(renamed.agent.agentId, issued, "改名丢号 = 记事本断链的重演");

    // 调用方塞来别人的号（或乱码）也一律以盘上为准。
    const hijacked = await service.updateAgent({
      agentId: renamed.agent.id,
      config: {
        name: "new-name",
        description: "d",
        systemPrompt: "p",
        memory: "project",
        agentId: AGENT_ID_B,
      },
      oldFilePath: join(fixture.agentsRoot, "new-name.md"),
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.equal(hijacked.agent.agentId, issued);
    assert.equal(
      normalizeAgentId(await readProfileAt(join(fixture.agentsRoot, "new-name.md"))
        .then((content) => parseProfile(content).agent?.agentId)),
      issued,
    );
  } finally {
    await cleanup(fixture);
  }
});

test("老档案补号：编辑一次即上号；上过的号任何保存都丢不了", async () => {
  const fixture = await makeFixture();
  try {
    const legacyPath = join(fixture.agentsRoot, "legacy-agent.md");
    await writeFile(
      legacyPath,
      profileMarkdown('name: "legacy-agent"\ndescription: "d"\nmemory: project\n'),
      "utf-8",
    );
    const service = serviceOf(fixture);
    const updated = await service.updateAgent({
      agentId: "user:workspace:legacy-agent",
      config: { name: "legacy-agent", description: "d", systemPrompt: "prompt", memory: "project" },
      oldFilePath: legacyPath,
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.ok(normalizeAgentId(updated.agent.agentId), "读到旧档案 → 这次写入补发号");
    const issued = updated.agent.agentId!;

    // 没带 oldFilePath 的同名保存：兜底读将要覆盖的目标文件，号原样留下。
    const anchorless = await service.updateAgent({
      agentId: "user:workspace:legacy-agent",
      config: { name: "legacy-agent", description: "d", systemPrompt: "prompt" },
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.equal(anchorless.agent.agentId, issued, "上过的号不能被一次保存写丢");
  } finally {
    await cleanup(fixture);
  }
});

test("无锚点不发号：旧档案与目标都读不到号时保持无号（防每次保存换一张脸）", async () => {
  const fixture = await makeFixture();
  try {
    const created = await serviceOf(fixture).updateAgent({
      agentId: "user:workspace:brand-new",
      config: { name: "brand-new", description: "d", systemPrompt: "prompt" },
      provider: ZCODE_AGENT_PROVIDER,
      scope: "workspace",
      workspacePath: fixture.workspacePath,
    });
    assert.equal(created.agent.agentId, undefined);
    assert.ok(
      !(await readProfileAt(join(fixture.agentsRoot, "brand-new.md"))).includes("agentId"),
      "凭空发号会让记事本目录跟着号一路搬家",
    );
  } finally {
    await cleanup(fixture);
  }
});
