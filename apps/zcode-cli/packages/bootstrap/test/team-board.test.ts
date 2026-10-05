// ============================================================
// 团队看板快照聚合（团队看板批 2026-10-05）的可运行检查。
// 驱动真实 buildTeamBoardSnapshotFromRows（协议层纯函数）：
//  1. 派单行按信封 batchId 分组成队；散单（无 batchId）不进看板；
//  2. 回执行按 workOrderId 对账终态（admittedSequence 最新赢）；
//  3. 解锁判定：dependsOn 全部 completed 才 unlocked；无依赖恒 true；
//  4. 质检灯台账口径：skipped > ran(promoted) > pending(admitted)；
//  5. 成员名册去重（agentId 优先），工位 live 状态来自注册表查询；
//  6. 评审批判定与质检触发同规则：同批全是评审单才算。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/team-board.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildTeamBoardSnapshotFromRows,
  liveStatusFromSessions,
  type TeamBoardInputRow,
} from "../src/zcode-protocol/team-board.js";

let seq = 0;

function row(input: {
  kind: string;
  status?: string;
  payload: Record<string, unknown>;
  created?: number;
}): TeamBoardInputRow {
  seq += 1;
  const created = input.created ?? seq;
  return {
    kind: input.kind,
    status: input.status ?? "admitted",
    admittedSequence: seq,
    time: { created, updated: created },
    payload: input.payload as TeamBoardInputRow["payload"],
  };
}

function dispatchRow(input: {
  workOrderId: string;
  batchId: string;
  batchTitle?: string;
  agentName: string;
  agentId?: string;
  taskKey?: string;
  dependsOn?: string[];
  review?: boolean;
  model?: string;
  created?: number;
}): TeamBoardInputRow {
  return row({
    kind: "agentWorkOrderDispatch",
    payload: {
      text: `任务 ${input.workOrderId}`,
      workOrderId: input.workOrderId,
      agentName: input.agentName,
      ...(input.agentId ? { agentId: input.agentId } : {}),
      targetSessionId: `desk-${input.workOrderId}`,
      ...(input.model ? { model: input.model } : {}),
      envelope: {
        workOrderId: input.workOrderId,
        fromAgentName: "boss",
        fromSessionId: "sess-boss",
        task: `任务 ${input.workOrderId}`,
        batchId: input.batchId,
        ...(input.batchTitle ? { batchTitle: input.batchTitle } : {}),
        ...(input.taskKey ? { taskKey: input.taskKey } : {}),
        ...(input.dependsOn ? { dependsOn: input.dependsOn } : {}),
        ...(input.review ? { review: true } : {}),
      },
    },
    ...(input.created === undefined ? {} : { created: input.created }),
  });
}

function receiptRow(input: {
  workOrderId: string;
  batchId: string;
  status: "completed" | "failed" | "cancelled";
}): TeamBoardInputRow {
  return row({
    kind: "agentWorkOrderReceipt",
    payload: {
      text: "回执",
      workOrderId: input.workOrderId,
      outcome: { status: input.status, toolCallCount: 3 },
      envelope: {
        workOrderId: input.workOrderId,
        fromAgentName: "boss",
        fromSessionId: "sess-boss",
        task: "任务",
        batchId: input.batchId,
      },
    },
  });
}

const LIVE_IDLE = () => "idle" as const;
const LIVE_RUNNING = (desk: string) => (desk === "desk-wo-2" ? ("running" as const) : ("idle" as const));

test("看板：派单行分组成队，散单不进看板；任务带符号名/依赖/模型预览", () => {
  const snapshot = buildTeamBoardSnapshotFromRows({
    sessionId: "sess-boss",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", batchTitle: "登录页改造", agentName: "code-plus", taskKey: "impl", model: "glm-5.3" }),
      dispatchRow({ workOrderId: "wo-2", batchId: "b1", agentName: "doc-writer", taskKey: "docs", dependsOn: ["impl"] }),
      dispatchRow({ workOrderId: "wo-3", batchId: "b2", agentName: "prd-engineer" }),
      row({ kind: "agentWorkOrderDispatch", payload: { text: "散单", workOrderId: "wo-9", envelope: { task: "散单", fromSessionId: "sess-boss", workOrderId: "wo-9" } } }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  assert.equal(snapshot.teams.length, 2);
  const b1 = snapshot.teams.find((team) => team.batchId === "b1")!;
  assert.equal(b1.title, "登录页改造");
  assert.equal(b1.orders.length, 2);
  const impl = b1.orders.find((order) => order.taskKey === "impl")!;
  assert.equal(impl.model, "glm-5.3");
  assert.equal(impl.taskPreview, "任务 wo-1");
  const docs = b1.orders.find((order) => order.taskKey === "docs")!;
  assert.deepEqual(docs.dependsOn, ["impl"]);
  assert.equal(docs.unlocked, false, "前置未完成 = 锁死");
  assert.equal(impl.unlocked, true, "无依赖恒解锁");
  assert.equal(snapshot.teams.find((team) => team.batchId === "wo-9"), undefined, "散单不进看板");
});

test("看板：回执终态对账 + 解锁随 completed 解锁", () => {
  const snapshot = buildTeamBoardSnapshotFromRows({
    sessionId: "sess-boss",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "a", taskKey: "impl" }),
      dispatchRow({ workOrderId: "wo-2", batchId: "b1", agentName: "b", taskKey: "docs", dependsOn: ["impl"] }),
      receiptRow({ workOrderId: "wo-1", batchId: "b1", status: "completed" }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  const b1 = snapshot.teams[0]!;
  assert.equal(b1.orders.find((order) => order.workOrderId === "wo-1")!.status, "completed");
  assert.equal(b1.orders.find((order) => order.workOrderId === "wo-2")!.unlocked, true, "前置 completed → 解锁");
});

test("看板：同单多张回执取 admittedSequence 最新（重试翻盘成立）", () => {
  const snapshot = buildTeamBoardSnapshotFromRows({
    sessionId: "sess-boss",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "a" }),
      receiptRow({ workOrderId: "wo-1", batchId: "b1", status: "failed" }),
      receiptRow({ workOrderId: "wo-1", batchId: "b1", status: "completed" }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  assert.equal(snapshot.teams[0]!.orders[0]!.status, "completed");
});

test("看板：质检灯台账口径——免检最强，其次已开跑，最后待跑", () => {
  const skipped = buildTeamBoardSnapshotFromRows({
    sessionId: "s",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "a" }),
      row({ kind: "agentWorkOrderBatchQc", payload: { text: "", batchId: "b1", skipped: true } }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  assert.equal(skipped.teams[0]!.qc, "skipped");

  const ran = buildTeamBoardSnapshotFromRows({
    sessionId: "s",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "a" }),
      row({ kind: "agentWorkOrderBatchQc", status: "promoted", payload: { text: "", batchId: "b1" } }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  assert.equal(ran.teams[0]!.qc, "ran");

  const pending = buildTeamBoardSnapshotFromRows({
    sessionId: "s",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "a" }),
      row({ kind: "agentWorkOrderBatchQc", payload: { text: "", batchId: "b1" } }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  assert.equal(pending.teams[0]!.qc, "pending");
});

test("看板：成员名册按工号去重、live 来自注册表查询；评审批判定同质检触发规则", () => {
  const snapshot = buildTeamBoardSnapshotFromRows({
    sessionId: "sess-boss",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "小陈", agentId: "agent-1", created: 1 }),
      dispatchRow({ workOrderId: "wo-2", batchId: "b1", agentName: "小陈改名后", agentId: "agent-1", created: 2 }),
      dispatchRow({ workOrderId: "wo-3", batchId: "b1", agentName: "小李", agentId: "agent-2", created: 3 }),
    ],
    liveStatusOf: LIVE_RUNNING,
  });
  const b1 = snapshot.teams[0]!;
  assert.equal(b1.members.length, 2, "同工号改名 = 同一位员工");
  const chen = b1.members.find((member) => member.agentId === "agent-1")!;
  assert.equal(chen.name, "小陈改名后", "工位与署名取台账最新一行");
  assert.equal(chen.live, "running", "desk-wo-2 在跑");
  assert.equal(b1.review, undefined, "混批或全普通批不是评审批");

  const review = buildTeamBoardSnapshotFromRows({
    sessionId: "sess-boss",
    rows: [
      dispatchRow({ workOrderId: "wo-1", batchId: "b2", agentName: "a", review: true }),
      dispatchRow({ workOrderId: "wo-2", batchId: "b2", agentName: "b", review: true }),
    ],
    liveStatusOf: LIVE_IDLE,
  });
  assert.equal(review.teams[0]!.review, true);
});

test("看板：工位 live 查询炸了退 unknown，不炸快照", () => {
  assert.equal(liveStatusFromSessions(undefined), "unknown");
  assert.equal(
    liveStatusFromSessions({
      hasActiveOrQueuedTurnWork: () => {
        throw new Error("boom");
      },
    }),
    "unknown",
  );
  assert.equal(liveStatusFromSessions({ hasActiveOrQueuedTurnWork: () => false }), "idle");
});
