// ============================================================
// 排班草案（团队看板批3b）的可运行检查。
// 端口四动作依赖 runtime 全家桶，这里测纯判定与快照两条腿：
//  1. validatePlanTasks：assignee 解析/依赖引用/重复 key/环；
//  2. extractStagedPlanDrafts：只列 staged 的，approved/discarded 不进看板草案区。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/team-plan.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { AgentProfile } from "@zcode/core";
import type { TeamPlanTask } from "@zcode/contracts";
import {
  extractStagedPlanDrafts,
  hasPlanDependencyCycle,
  validatePlanTasks,
  type TeamPlanLedgerRow,
} from "../src/zcode-protocol/team-plan.js";

// 工号是 uuid 形状（isAgentIdLike 据此走工号匹配；不像 uuid 的串按名字匹配）。
const PROFILES: AgentProfile[] = [
  { name: "code-plus", agentId: "0b9e6c5e-1111-4c11-8111-111111111111", systemPrompt: "", source: "project" },
  { name: "doc-writer", agentId: "0b9e6c5e-2222-4c22-8222-222222222222", systemPrompt: "", source: "user" },
] as AgentProfile[];

function task(input: { taskKey: string; assignee?: string; dependsOn?: string[] }): TeamPlanTask {
  return {
    taskKey: input.taskKey,
    task: `任务 ${input.taskKey}`,
    assignee: input.assignee ?? "code-plus",
    ...(input.dependsOn ? { dependsOn: input.dependsOn } : {}),
  };
}

test("validatePlanTasks：通过口径（assignee 可解析、依赖引用本草案、无环）", () => {
  const errors = validatePlanTasks(
    [
      task({ taskKey: "req", assignee: "0b9e6c5e-1111-4c11-8111-111111111111" }),
      task({ taskKey: "impl", assignee: "doc-writer", dependsOn: ["req"] }),
      task({ taskKey: "review", assignee: "doc-writer", dependsOn: ["impl"] }),
    ],
    PROFILES,
  );
  assert.deepEqual(errors, []);
});

test("validatePlanTasks：未知 assignee / 悬空依赖 / 重复 key / 环 各自报错", () => {
  const unknown = validatePlanTasks([task({ taskKey: "a", assignee: "不存在" })], PROFILES);
  assert.equal(unknown.length, 1);
  assert.ok(unknown[0]!.error.includes("不在名册里"));

  const dangling = validatePlanTasks([task({ taskKey: "a", dependsOn: ["没定义"] })], PROFILES);
  assert.ok(dangling[0]!.error.includes("不是本草案里定义的任务符号名"));

  const dup = validatePlanTasks([task({ taskKey: "a" }), task({ taskKey: "a" })], PROFILES);
  assert.ok(dup[0]!.error.includes("重复"));

  const cyclic: TeamPlanTask[] = [
    task({ taskKey: "a", dependsOn: ["b"] }),
    task({ taskKey: "b", dependsOn: ["a"] }),
  ];
  assert.equal(hasPlanDependencyCycle(cyclic), true);
  const blocked = validatePlanTasks(cyclic, PROFILES);
  assert.ok(blocked[0]!.error.includes("依赖成环"));
});

function planRow(input: {
  planId: string;
  status: "staged" | "approved" | "discarded";
  title?: string;
}): TeamPlanLedgerRow {
  const title = input.title ?? "登录页改造";
  return {
    id: `agentWorkOrderTeamPlan:sess:${input.planId}`,
    kind: "agentWorkOrderTeamPlan",
    status: "admitted",
    payload: {
      text: title,
      planId: input.planId,
      title,
      status: input.status,
      tasks: [task({ taskKey: "req" })],
      createdAt: 1,
    },
  };
}

test("快照草案区：只列 staged 的；approved/discarded 不进草案区", () => {
  const drafts = extractStagedPlanDrafts([
    planRow({ planId: "p1", status: "staged" }),
    planRow({ planId: "p2", status: "approved" }),
    planRow({ planId: "p3", status: "discarded" }),
  ]);
  assert.deepEqual(
    drafts.map((plan) => plan.planId),
    ["p1"],
  );
  assert.equal(drafts[0]!.status, "staged");
});
