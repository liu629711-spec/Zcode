// ============================================================
// 岗位禁令进图纸（2026-10-01 拍板）的可运行检查：员工工具限制与
// 图纸安全底线的合成规则。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/src/app/workflow-actor-tools.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { workflowActorToolPolicy } from "./workflow-actor-tools.js";

test("没点名员工：行为与从前逐字节一致（底线减法表，narrowed=false）", () => {
  const policy = workflowActorToolPolicy();
  assert.equal(policy.narrowed, false);
  assert.equal(policy.toolAllowlist, undefined);
  assert.equal(policy.toolDisallowlist.length, 7);
  assert.ok(policy.toolDisallowlist.includes("CreateWorkflow"));
  assert.deepEqual(workflowActorToolPolicy({}).toolDisallowlist, policy.toolDisallowlist);
});

test("岗位禁令并进减法表：员工被禁的 Bash/Write 在图纸里照样禁", () => {
  const policy = workflowActorToolPolicy({
    disallowedTools: ["Bash", "Write", "Bash"],
  });
  assert.equal(policy.narrowed, true);
  assert.ok(policy.toolDisallowlist.includes("Bash"));
  assert.ok(policy.toolDisallowlist.includes("Write"));
  // 去重：底线 + 岗位禁令并集无重复。
  assert.equal(new Set(policy.toolDisallowlist).size, policy.toolDisallowlist.length);
});

test("岗位白名单在场：整个面收紧成「白名单 − 安全底线」", () => {
  const policy = workflowActorToolPolicy({
    tools: ["Read", "Grep", "AskUserQuestion"],
  });
  assert.equal(policy.narrowed, true);
  assert.deepEqual(policy.toolAllowlist, ["Read", "Grep"]);
  // 白名单里混进底线工具（AskUserQuestion 会悬挂在无人应答的会话里）：底线赢。
  assert.ok(!policy.toolAllowlist?.includes("AskUserQuestion"));
  // 减法表保持完整（白名单语义下它仍是第二道闸）。
  assert.equal(policy.toolDisallowlist.length, 7);
});

test("白名单与禁令同给：两面同时生效", () => {
  const policy = workflowActorToolPolicy({
    tools: ["Read", "Bash"],
    disallowedTools: ["Bash"],
  });
  assert.equal(policy.narrowed, true);
  assert.deepEqual(policy.toolAllowlist, ["Read", "Bash"]);
  assert.ok(policy.toolDisallowlist.includes("Bash"));
});

test("空数组等价于没给：不误触发收窄日志", () => {
  assert.equal(workflowActorToolPolicy({ tools: [] }).narrowed, false);
  assert.equal(workflowActorToolPolicy({ disallowedTools: [] }).narrowed, false);
});
