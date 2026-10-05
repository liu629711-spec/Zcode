// ============================================================
// 团队调度器（团队看板批2 2026-10-05）的可运行检查。
// 驱动真实纯判定函数（执行器只在 bootstrap 触发，判定可测）：
//  1. parseReviewVerdict（core）：格式行/自由文本/位置窗口；
//  2. computeReadyHeldWorkOrders：held 行前置全 completed 才释放；
//  3. computeAutoRepairAction：verdict 门/开关门/在飞挡/上限停手；
//  4. computeReReviewAction：找原评审人、落回评审批次。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/team-scheduler.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { parseReviewVerdict } from "@zcode/core";
import {
  computeAutoRepairAction,
  computeReadyHeldWorkOrders,
  computeReReviewAction,
  isTeamAutoFlowEnabled,
} from "../src/zcode-protocol/team-scheduler.js";
import {
  computeCompletedTaskKeys,
  type TeamBoardInputRow,
} from "../src/zcode-protocol/team-board.js";

let seq = 0;

function row(input: {
  id?: string;
  kind: string;
  status?: string;
  payload: Record<string, unknown>;
}): TeamBoardInputRow {
  seq += 1;
  return {
    id: input.id ?? `row-${seq}`,
    kind: input.kind,
    status: input.status ?? "admitted",
    admittedSequence: seq,
    time: { created: seq, updated: seq },
    payload: input.payload as TeamBoardInputRow["payload"],
  };
}

function envelope(extra: Record<string, unknown>): Record<string, unknown> {
  return {
    workOrderId: "wo",
    fromAgentName: "boss",
    fromSessionId: "sess-boss",
    task: "任务",
    ...extra,
  };
}

test("parseReviewVerdict：格式行三值解析，自由文本与位置窗口不猜", () => {
  assert.equal(parseReviewVerdict("评审结论：不通过\n1. xxx"), "fail");
  assert.equal(parseReviewVerdict("评审结论: 通过"), "pass");
  assert.equal(parseReviewVerdict("评审结论：有条件通过"), "conditional");
  assert.equal(parseReviewVerdict("我觉得还行"), undefined);
  // 结论行在第四行（窗口外）：不猜。
  assert.equal(parseReviewVerdict("前话一\n前话二\n前话三\n评审结论：通过"), undefined);
});

function heldRow(input: {
  workOrderId: string;
  taskKey?: string;
  dependsOn?: string[];
  agentName?: string;
}): TeamBoardInputRow {
  return row({
    kind: "agentWorkOrderDispatch",
    payload: {
      text: "任务",
      workOrderId: input.workOrderId,
      agentName: input.agentName ?? "code-plus",
      agentId: "agent-1",
      targetSessionId: `desk-${input.workOrderId}`,
      held: { pendingKeys: input.dependsOn ?? [] },
      envelope: envelope({
        workOrderId: input.workOrderId,
        task: "任务",
        ...(input.taskKey ? { taskKey: input.taskKey } : {}),
        ...(input.dependsOn ? { dependsOn: input.dependsOn } : {}),
      }),
    },
  });
}

function receiptRow(input: {
  workOrderId: string;
  status: "completed" | "failed" | "cancelled";
  response?: string;
}): TeamBoardInputRow {
  return row({
    kind: "agentWorkOrderReceipt",
    payload: {
      text: "回执",
      workOrderId: input.workOrderId,
      outcome: { status: input.status, ...(input.response ? { response: input.response } : {}) },
      envelope: envelope({ workOrderId: input.workOrderId }),
    },
  });
}

test("持派释放：前置全部 completed 才就绪；部分完成继续挂", () => {
  const rows: TeamBoardInputRow[] = [
    row({ kind: "agentWorkOrderDispatch", payload: { text: "任务", workOrderId: "wo-1", agentName: "code-plus", envelope: envelope({ workOrderId: "wo-1", task: "任务", taskKey: "impl" }) } }),
    receiptRow({ workOrderId: "wo-1", status: "completed" }),
    heldRow({ workOrderId: "wo-2", dependsOn: ["impl"] }),
    heldRow({ workOrderId: "wo-3", dependsOn: ["impl", "docs"] }),
    row({ kind: "agentWorkOrderDispatch", payload: { text: "t", workOrderId: "wo-4", envelope: envelope({ workOrderId: "wo-4", task: "t", taskKey: "docs" }) } }),
    receiptRow({ workOrderId: "wo-4", status: "failed" }),
  ];
  assert.deepEqual(computeCompletedTaskKeys(rows), new Set(["impl"]));
  const ready = computeReadyHeldWorkOrders(rows);
  assert.deepEqual(ready.map((item) => item.workOrderId), ["wo-2"], "wo-3 还等 docs");
  assert.equal(ready[0]!.envelope.workOrderId, "wo-2");
  assert.equal(ready[0]!.agentId, "agent-1");
});

test("自动返修：verdict 不通过 + 开关开 + 被审单可寻址 → 派返修；在飞挡；上限停", () => {
  const base: TeamBoardInputRow[] = [
    row({ id: "row-impl", kind: "agentWorkOrderDispatch", payload: { text: "实现登录页", workOrderId: "wo-impl", agentName: "code-plus", agentId: "agent-1", envelope: envelope({ workOrderId: "wo-impl", task: "实现登录页", taskKey: "impl", batchId: "b-build", batchTitle: "登录页" }) } }),
    row({ id: "row-review", kind: "agentWorkOrderDispatch", payload: { text: "评审", workOrderId: "wo-review", agentName: "code-reviewer", envelope: envelope({ workOrderId: "wo-review", task: "评审", review: true, reviews: "wo-impl", batchId: "b-review" }) } }),
  ];
  // 开关关：不修。
  assert.equal(
    computeAutoRepairAction(base, { reviewsWorkOrderId: "wo-impl", reviewResponse: "评审结论：不通过\n1. 样式", reviewBatchId: "b-review" }),
    undefined,
  );
  // 开关开（评审批或工地批任一）：修。
  const withFlow: TeamBoardInputRow[] = [
    ...base,
    row({ kind: "agentWorkOrderTeamFlow", payload: { batchId: "b-review", enabled: true } }),
  ];
  const action = computeAutoRepairAction(withFlow, {
    reviewsWorkOrderId: "wo-impl",
    reviewResponse: "评审结论：不通过\n1. 样式不对",
    reviewBatchId: "b-review",
  });
  assert.equal(action?.kind, "repair");
  if (action?.kind === "repair") {
    assert.equal(action.agent, "agent-1", "按工号派回原员工");
    assert.equal(action.batchId, "b-build", "返修单落回原工地卡");
    assert.equal(action.repairRound, 1);
    assert.ok(action.task.includes("实现登录页"), "带原任务原文");
    assert.ok(action.task.includes("样式不对"), "带评审意见");
  }
  // 返修在飞：不重复触发。
  const withInflight: TeamBoardInputRow[] = [
    ...withFlow,
    row({ kind: "agentWorkOrderDispatch", payload: { text: "返修", workOrderId: "wo-repair", envelope: envelope({ workOrderId: "wo-repair", task: "返修", repairOf: "wo-impl", repairRound: 1 }) } }),
  ];
  assert.equal(
    computeAutoRepairAction(withInflight, { reviewsWorkOrderId: "wo-impl", reviewResponse: "评审结论：不通过", reviewBatchId: "b-review" }),
    undefined,
  );
  // 已有 2 轮返修（都不在飞）再来一个不通过 → 停手。
  const capped: TeamBoardInputRow[] = [
    ...withFlow,
    row({ kind: "agentWorkOrderDispatch", status: "discarded", payload: { text: "返修1", workOrderId: "wo-repair-1", envelope: envelope({ workOrderId: "wo-repair-1", task: "返修1", repairOf: "wo-impl", repairRound: 1 }) } }),
    row({ kind: "agentWorkOrderDispatch", status: "discarded", payload: { text: "返修2", workOrderId: "wo-repair-2", envelope: envelope({ workOrderId: "wo-repair-2", task: "返修2", repairOf: "wo-impl", repairRound: 2 }) } }),
  ];
  const stopped = computeAutoRepairAction(capped, {
    reviewsWorkOrderId: "wo-impl",
    reviewResponse: "评审结论：不通过",
    reviewBatchId: "b-review",
  });
  assert.equal(stopped?.kind, "stop");
  // 通过/有条件通过不触发。
  assert.equal(
    computeAutoRepairAction(withFlow, { reviewsWorkOrderId: "wo-impl", reviewResponse: "评审结论：通过", reviewBatchId: "b-review" }),
    undefined,
  );
});

test("复审：返修单 completed → 找原评审人，落回评审批次，reviews 指向返修单", () => {
  const rows: TeamBoardInputRow[] = [
    row({ kind: "agentWorkOrderDispatch", payload: { text: "实现", workOrderId: "wo-impl", agentName: "code-plus", envelope: envelope({ workOrderId: "wo-impl", task: "实现", batchId: "b-build" }) } }),
    row({ kind: "agentWorkOrderDispatch", payload: { text: "评审", workOrderId: "wo-review", agentName: "code-reviewer", agentId: "agent-9", envelope: envelope({ workOrderId: "wo-review", task: "评审", review: true, reviews: "wo-impl", batchId: "b-review", batchTitle: "评审会" }) } }),
    row({ kind: "agentWorkOrderDispatch", payload: { text: "返修", workOrderId: "wo-repair", envelope: envelope({ workOrderId: "wo-repair", task: "返修", repairOf: "wo-impl", repairRound: 1 }) } }),
    receiptRow({ workOrderId: "wo-repair", status: "completed" }),
  ];
  const action = computeReReviewAction(rows, { repairWorkOrderId: "wo-repair" });
  assert.ok(action);
  assert.equal(action!.agent, "agent-9");
  assert.equal(action!.batchId, "b-review");
  assert.equal(action!.reviews, "wo-repair");
  assert.ok(action!.task.includes("第 1 轮复审"));
});

test("开关查询：任一相关批次开着即算开", () => {
  const rows: TeamBoardInputRow[] = [
    row({ kind: "agentWorkOrderTeamFlow", payload: { batchId: "b-build", enabled: true } }),
  ];
  assert.equal(isTeamAutoFlowEnabled(rows, ["b-build"]), true);
  assert.equal(isTeamAutoFlowEnabled(rows, ["b-review", "b-build"]), true);
  assert.equal(isTeamAutoFlowEnabled(rows, ["b-review"]), false);
});
