// ============================================================
// 点将（D24）纯规则的可运行检查：正文解析、现役对号、派遣归属裁决、告示文案。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/subagent/agent-call.test.ts
// （node 原生 --test 会在 @zcode/shared 的 .js→.ts 解析上报错，统一走 tsx。）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  MAX_AGENT_CALLS_PER_TURN,
  buildAgentCallReminderBody,
  extractAgentReferences,
  resolveAgentCallTargets,
  resolvePinnedAgentType,
} from "./agent-call.ts";
import type { AgentProfile } from "./profile.ts";

function profile(name: string): AgentProfile {
  return { name, description: `${name} 的活`, source: "project", systemPrompt: "p" };
}

test("extractAgentReferences：认 agent:// 链接，按出现顺序去重", () => {
  const text =
    "先 [点将 @doc-writer](agent://doc-writer) 写文档，再 " +
    "[复核](agent://code-reviewer)，然后 [重复点名](agent://doc-writer) 一次";
  assert.deepEqual(extractAgentReferences(text).names, ["doc-writer", "code-reviewer"]);
});

test("extractAgentReferences：协议名与形状都从严，非 agent 链接一概不动", () => {
  const cases = [
    "[偷渡](Agent://doc-writer)",
    "[半截](agent://)",
    "[非法字符](agent://中文名)",
    "[太长](agent://" + "a".repeat(51) + ")",
    "[普通文件](file://a.ts)",
    "[裸文本] agent://doc-writer 没有链接壳不算",
    "[查询串](agent://doc-writer?x=1)",
  ];
  for (const text of cases) {
    assert.deepEqual(extractAgentReferences(text).names, [], text);
  }
  // 意图命中但形状非法 → invalidCount 如实记账（别把坏引用当没看见）。
  assert.equal(extractAgentReferences("[坏](Agent://doc-writer)").invalidCount, 1);
});

test("extractAgentReferences：超过单轮上限截断并计数", () => {
  const text = Array.from(
    { length: MAX_AGENT_CALLS_PER_TURN + 2 },
    (_, index) => `[${index}](agent://agent-${index})`,
  ).join(" ");
  const result = extractAgentReferences(text);
  assert.equal(result.names.length, MAX_AGENT_CALLS_PER_TURN);
  assert.equal(result.truncatedCount, 2);
});

test("resolveAgentCallTargets：与现役档案大小写不敏感对号，点空的人如实带出", () => {
  const profiles = [profile("doc-writer"), profile("code-reviewer")];
  const result = resolveAgentCallTargets(["DOC-writer", "ghost"], profiles);
  assert.deepEqual(
    result.resolved.map((item) => item.name),
    ["doc-writer"],
  );
  assert.deepEqual(result.unresolved, ["ghost"]);
});

test("resolvePinnedAgentType：名单为空行为不变；名单在场只能派给名单内", () => {
  assert.equal(resolvePinnedAgentType("explore", []), "explore", "没点将 → 原样");
  assert.equal(
    resolvePinnedAgentType("general-purpose", ["doc-writer"]),
    "doc-writer",
    "模型省略 subagent_type 时工具已填 general-purpose → 照样改派点名的人",
  );
  assert.equal(
    resolvePinnedAgentType("code-reviewer", ["doc-writer", "code-reviewer"]),
    "code-reviewer",
  );
  assert.equal(
    resolvePinnedAgentType("CODE-REVIEWER", ["doc-writer", "code-reviewer"]),
    "code-reviewer",
    "模型写的大小写不同也算名单内",
  );
  assert.equal(
    resolvePinnedAgentType("explore", ["doc-writer"]),
    "doc-writer",
    "用户点名了就派给他，模型想派别人也不许越名单",
  );
});

test("buildAgentCallReminderBody：告示写清名单与点空的人，两者都空就不注入", () => {
  assert.equal(
    buildAgentCallReminderBody({ resolved: [], unresolved: [] }),
    undefined,
    "没点将是绝对主路径：零注入",
  );
  const body = buildAgentCallReminderBody({
    resolved: [{ name: "doc-writer", description: "文档文员" }],
    unresolved: ["ghost"],
  })!;
  assert.ok(body.includes("doc-writer"));
  assert.ok(body.includes("subagent_type"), "要教模型怎么派");
  assert.ok(body.includes("ghost"), "点空了要当场说，不能静默");
});
