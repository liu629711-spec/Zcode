import assert from "node:assert/strict";
import test from "node:test";
import type { TurnHeaderRow } from "@zcode/shared/zcode-protocol-v4";
import type { ConversationTurnRenderUnit } from "../src/v4/conversationTurnRenderUnits.js";
import {
  selectWorkOrderBatchRenderInfo,
  selectWorkOrderBatches,
} from "../src/v4/agentWorkOrderBatch.js";

// ============================================================
// 批次派单（工地卡）纯规则的可运行检查：派单行 + 回执轮头按 batchId 聚拢、
// 回执更新单据状态、散单照旧、只有回执的批次不崩。
//
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/agentWorkOrderBatch.test.ts
// ============================================================

function turnHeaderRow(overrides: Partial<TurnHeaderRow>): TurnHeaderRow {
  return {
    rowId: 1,
    turnId: "turn-1",
    createdAt: 0,
    createdAtSeq: 0,
    kind: "turnHeader",
    origin: "userInput",
    state: "completedSuccess",
    startedAt: 0,
    ...overrides,
  } as TurnHeaderRow;
}

function unit(key: string, overrides: Partial<ConversationTurnRenderUnit>): ConversationTurnRenderUnit {
  return {
    key,
    turnId: key,
    visibleUserInputs: [],
    assistantWorkRows: [],
    assistantHistoryRows: [],
    assistantFollowingRows: [],
    assistantTailRows: [],
    browserTurnEndRows: [],
    hookInvocations: [],
    assistantTextRows: [],
    leadingBoundaryRows: [],
    flowItems: [],
    renderRows: [],
    isLastTurn: true,
    isRunning: false,
    assistantHistoryDefaultOpen: false,
    timelineOnly: false,
    ...overrides,
  };
}

function dispatchRow(toolCallId: string, input: object, output: object) {
  return {
    kind: "toolCall" as const,
    rowId: 1,
    turnId: "turn-1",
    createdAt: 0,
    createdAtSeq: 0,
    toolCallId,
    toolName: "AgentDispatch",
    status: "success" as const,
    inputText: JSON.stringify(input),
    input,
    output: { text: JSON.stringify(output) },
  };
}

function receiptTurn(
  key: string,
  batch: { batchId: string; batchTitle?: string },
  workOrderId: string,
  title: string,
  answer = "",
): ConversationTurnRenderUnit {
  return unit(key, {
    header: turnHeaderRow({
      turnId: key,
      origin: "backgroundResult",
      state: "completedSuccess",
      originMeta: {
        backgroundSource: "agent_work_order_receipt",
        workId: workOrderId,
        title,
        batchId: batch.batchId,
        ...(batch.batchTitle ? { batchTitle: batch.batchTitle } : {}),
      },
    }),
    assistantTextRows: answer
      ? [{ kind: "assistantText" as const, rowId: 2, turnId: key, createdAt: 0, createdAtSeq: 0, text: answer, state: "complete" as const }]
      : [],
    latestAssistantTextRow: answer
      ? { kind: "assistantText" as const, rowId: 2, turnId: key, createdAt: 0, createdAtSeq: 0, text: answer, state: "complete" as const }
      : undefined,
  });
}

const BATCH = { batchId: "batch-1", batchTitle: "登录页改造" };

function dispatchTurn(key: string, rows: ReturnType<typeof dispatchRow>[]): ConversationTurnRenderUnit {
  return unit(key, { assistantWorkRows: rows });
}

test("selectWorkOrderBatches：同轮两张派单 + 后续一张回执补单聚成一批（2+1）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow(
        "call-1",
        { agent: "code-plus", task: "拆组件", batch_title: "登录页改造" },
        { targetSessionId: "s1", agentName: "code-plus", delivery: "started", createdSession: false, workOrderId: "wo-1", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
      dispatchRow(
        "call-2",
        { agent: "doc-writer", task: "写文档", batch_title: "登录页改造" },
        { targetSessionId: "s2", agentName: "doc-writer", delivery: "queued", createdSession: false, workOrderId: "wo-2", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
    ]),
    receiptTurn("receipt-3", BATCH, "wo-3", "prd-engineer 交活", "PRD 已定稿"),
  ];

  const batches = selectWorkOrderBatches(units);
  assert.equal(batches.length, 1);
  const batch = batches[0]!;
  assert.equal(batch.batchId, "batch-1");
  assert.equal(batch.title, "登录页改造");
  // host 跟最新证据走：工地卡贴活边（最新回执轮），不钉在派单轮上。
  assert.equal(batch.hostUnitKey, "receipt-3");
  assert.deepEqual(
    batch.orders.map((order) => [order.agentName, order.status]),
    [
      ["code-plus", "dispatched"],
      ["doc-writer", "queued"],
      ["prd-engineer", "completed"],
    ],
  );
  assert.equal(batch.orders[2]!.receiptText, "PRD 已定稿");
});

test("selectWorkOrderBatches：回执把派单行记的 dispatched 更新成 completed（按 workOrderId 对账）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow(
        "call-1",
        { agent: "code-plus", task: "A", batch_title: "登录页改造" },
        { agentName: "code-plus", delivery: "started", workOrderId: "wo-1", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
      dispatchRow(
        "call-2",
        { agent: "code-reviewer", task: "B", batch_title: "登录页改造" },
        { agentName: "code-reviewer", delivery: "started", workOrderId: "wo-2", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
    ]),
    receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "改完了"),
  ];

  const batch = selectWorkOrderBatches(units)[0]!;
  assert.equal(batch.orders.length, 2);
  const first = batch.orders.find((order) => order.key === "wo-1")!;
  assert.equal(first.status, "completed");
  assert.equal(first.receiptTitle, "code-plus 交活");
  assert.equal(first.receiptText, "改完了");
  assert.equal(first.agentName, "code-plus");
  assert.equal(batch.orders.find((order) => order.key === "wo-2")!.status, "dispatched");
  // 回执轮既是 member（散卡压制）也是 host（工地卡贴最新回执轮）；
  // 派单轮只是 member——卡片不再钉在派单轮上被后续内容越顶越远。
  const info = selectWorkOrderBatchRenderInfo(units);
  assert.equal(info.get("receipt-1")![0]!.isHost, true);
  assert.equal(info.get("receipt-1")![0]!.isMember, true);
  assert.equal(info.get("dispatch-turn")![0]!.isHost, false);
});

test("selectWorkOrderBatches：单张工单不成批（派单行与回执对上是同一单）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow(
        "call-1",
        { agent: "code-plus", task: "发个你好", batch_title: "登录页改造" },
        { agentName: "code-plus", delivery: "started", workOrderId: "wo-1", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
    ]),
    receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "你好已发"),
  ];

  assert.deepEqual(selectWorkOrderBatches(units), []);
  assert.equal(selectWorkOrderBatchRenderInfo(units).size, 0);
});

test("selectWorkOrderBatches：只有回执的批次（无派单行证据）不崩，失败回执按标题解署名", () => {
  const units = [
    receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "第一单完成"),
    receiptTurn("receipt-2", BATCH, "wo-2", "doc-writer 的工单未完成"),
  ];

  const batch = selectWorkOrderBatches(units)[0]!;
  assert.equal(batch.batchId, "batch-1");
  assert.equal(batch.hostUnitKey, "receipt-2");
  assert.equal(batch.orders[0]!.status, "completed");
  assert.equal(batch.orders[1]!.status, "failed");
  assert.equal(batch.orders[1]!.agentName, "doc-writer");
});

// ── 失败回执线索进批次模型（2026-10-01 员工可靠性批）─────────────────

test("selectWorkOrderBatches：失败回执的任务原文与失败线索进单据行（一键重派数据源）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow(
        "call-1",
        { agent: "code-builder", task: "拆组件", batch_title: "登录页改造" },
        { targetSessionId: "s1", agentName: "code-builder", delivery: "started", workOrderId: "wo-1", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
      dispatchRow(
        "call-2",
        { agent: "code-reviewer", task: "写测试", batch_title: "登录页改造" },
        { targetSessionId: "s2", agentName: "code-reviewer", delivery: "queued", workOrderId: "wo-2", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
    ]),
    unit("receipt-fail", {
      header: turnHeaderRow({
        turnId: "receipt-fail",
        origin: "backgroundResult",
        state: "completedError",
        originMeta: {
          backgroundSource: "agent_work_order_receipt",
          workId: "wo-1",
          title: "code-builder 的工单未完成",
          batchId: "batch-1",
          batchTitle: "登录页改造",
          task: "拆组件",
          failureCode: "invalid_model_request",
          failureModelId: "glm-5.3-flash-local",
          failureReason: "Provider rejected the model request.",
          retried: true,
          agentId: "a-12345678-1234-1234-1234-123456789abc",
        },
      }),
    }),
  ];

  const batches = selectWorkOrderBatches(units);
  assert.equal(batches.length, 1);
  const order = batches[0]!.orders.find((candidate) => candidate.key === "wo-1");
  assert.ok(order);
  assert.equal(order.status, "failed");
  assert.equal(order.agentName, "code-builder");
  assert.equal(order.task, "拆组件");
  assert.equal(order.failureCode, "invalid_model_request");
  assert.equal(order.failureModelId, "glm-5.3-flash-local");
  assert.equal(order.failureReason, "Provider rejected the model request.");
  assert.equal(order.retried, true);
  assert.equal(order.agentId, "a-12345678-1234-1234-1234-123456789abc");
  // 失败行没有可转交的成果。
  assert.equal(order.receiptAnswer, undefined);
});

test("selectWorkOrderBatches：被中断的回执是 cancelled，不再画成红色失败（audit D P2-4）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow(
        "call-1",
        { agent: "code-plus", task: "拆组件", batch_title: "登录页改造" },
        { targetSessionId: "s1", agentName: "code-plus", delivery: "started", workOrderId: "wo-1", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
      dispatchRow(
        "call-2",
        { agent: "doc-writer", task: "写文档", batch_title: "登录页改造" },
        { targetSessionId: "s2", agentName: "doc-writer", delivery: "started", workOrderId: "wo-2", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
      dispatchRow(
        "call-3",
        { agent: "code-reviewer", task: "写测试", batch_title: "登录页改造" },
        { targetSessionId: "s3", agentName: "code-reviewer", delivery: "started", workOrderId: "wo-3", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
    ]),
    receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "完成"),
    receiptTurn("receipt-2", BATCH, "wo-2", "doc-writer 的工单被中断"),
    receiptTurn("receipt-3", BATCH, "wo-3", "code-reviewer 的工单未完成"),
  ];

  const batch = selectWorkOrderBatches(units)[0]!;
  const byId = (key: string) => batch.orders.find((order) => order.key === key)!;
  assert.equal(byId("wo-1").status, "completed");
  assert.equal(byId("wo-2").status, "cancelled", "被中断 ≠ 干砸");
  assert.equal(byId("wo-3").status, "failed");
});
