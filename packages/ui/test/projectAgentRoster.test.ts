import assert from "node:assert/strict";
import test from "node:test";
import {
  PROJECT_AGENT_PRESET_ROSTER,
  planProjectAgentRosterInstall,
} from "../src/WorkspaceSidebar/projectAgentRoster.js";
import {
  toProjectAgentCreateConfig,
  validateProjectAgentDraft,
} from "../src/WorkspaceSidebar/projectAgentsModel.js";

// ============================================================
// D23 预置班底：名单本身的合法性 + "请进项目"的清单纯函数。
// 班底档案会真的落进用户项目（<ws>/.zcode/agents/*.md），所以名单里任何一条
// 不合法都会被建档时的校验弹回来——这里提前炸，别等用户点按钮。
//
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/projectAgentRoster.test.ts
// ============================================================

test("班底名单：名字合法、介绍与人设齐备、条数在 3~6 之间", () => {
  assert.ok(
    PROJECT_AGENT_PRESET_ROSTER.length >= 3 && PROJECT_AGENT_PRESET_ROSTER.length <= 6,
    `班底人数 ${PROJECT_AGENT_PRESET_ROSTER.length}，要的是 3~6 位`,
  );
  const names = new Set<string>();
  for (const member of PROJECT_AGENT_PRESET_ROSTER) {
    assert.deepEqual(validateProjectAgentDraft(member), [], `${member.name} 的档案不合法`);
    const lower = member.name.trim().toLowerCase();
    assert.equal(names.has(lower), false, `${member.name} 与名单里另一位重名`);
    names.add(lower);
    assert.ok(member.role.trim().length > 0, `${member.name} 缺大白话职业名`);
    // 人设必须写成"职责 + 工作方式 + 合格标准"，光一句"你是 XX 专家"是空壳。
    assert.ok(member.systemPrompt.length > 200, `${member.name} 的人设太短`);
    assert.ok(member.systemPrompt.includes("合格标准"), `${member.name} 的人设缺合格标准`);
  }
});

test("班底名单：不撞 ZCode 内置子代理名（建档会被 assertNotBuiltInName 拒）", () => {
  const reserved = new Set(["general-purpose", "explore"]);
  for (const member of PROJECT_AGENT_PRESET_ROSTER) {
    assert.equal(
      reserved.has(member.name.trim().toLowerCase()),
      false,
      `${member.name} 与内置子代理同名，建档会抛错`,
    );
  }
});

test("班底建档配置：记忆一律跟项目走（每个项目各一本记事本）", () => {
  for (const member of PROJECT_AGENT_PRESET_ROSTER) {
    assert.equal(toProjectAgentCreateConfig(member).memory, "project", member.name);
  }
});

test("planProjectAgentRosterInstall：跳过同名档案，用户改过的不覆盖", () => {
  const planned = planProjectAgentRosterInstall(["doc-writer"]);
  assert.deepEqual(
    planned.map((member) => member.name),
    PROJECT_AGENT_PRESET_ROSTER.map((member) => member.name).filter(
      (name) => name !== "doc-writer",
    ),
  );
  assert.deepEqual(
    planProjectAgentRosterInstall(PROJECT_AGENT_PRESET_ROSTER.map((member) => member.name)),
    [],
    "班底到齐就不再请人",
  );
});

test("planProjectAgentRosterInstall：同名判定大小写不敏感（档案文件名会小写化）", () => {
  // services 建档写盘是 `<name 小写>.md`，『Doc-Writer』与『doc-writer』是同一个文件。
  assert.equal(
    planProjectAgentRosterInstall(["Doc-Writer"]).some((m) => m.name === "doc-writer"),
    false,
  );
  assert.equal(
    planProjectAgentRosterInstall(["  DOC-WRITER  "]).some((m) => m.name === "doc-writer"),
    false,
    "带余空格也要认出来（档案名 trim 后比较）",
  );
});

test("planProjectAgentRosterInstall：空项目请进整支班底，坏名字不参与", () => {
  assert.equal(planProjectAgentRosterInstall([]).length, PROJECT_AGENT_PRESET_ROSTER.length);
  assert.equal(planProjectAgentRosterInstall(["", "   "]).length, PROJECT_AGENT_PRESET_ROSTER.length);
});
