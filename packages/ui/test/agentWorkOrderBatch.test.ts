import assert from "node:assert/strict";
import test from "node:test";
import type { TurnHeaderRow } from "@zcode/shared/zcode-protocol-v4";
import type { ConversationTurnRenderUnit } from "../src/v4/conversationTurnRenderUnits.js";
import {
  extractReviewVerdict,
  formatBatchDuration,
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
  receiptStatus?: "completed" | "failed" | "cancelled",
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
        ...(receiptStatus ? { receiptStatus } : {}),
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
        state: "failed",
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
  const order = batches[0]!.orders.find((candidate) => candidate.key === "wo-1")!;
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

test("selectWorkOrderBatches：结构化 receiptStatus 是三态权威，标题解析只兜底（地基清理）", () => {
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
    ]),
    // 字段说 failed、标题说「交活」：字段赢——UI 不再从中文标题反推终态。
    receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "结果不可信", "failed"),
    // 旧轮头没有字段：回退标题解析，completed 照旧。
    receiptTurn("receipt-2", BATCH, "wo-2", "doc-writer 交活", "旧数据"),
  ];

  const batch = selectWorkOrderBatches(units)[0]!;
  const byId = (key: string) => batch.orders.find((order) => order.key === key)!;
  assert.equal(byId("wo-1").status, "failed", "receiptStatus 权威于标题");
  assert.equal(byId("wo-1").receiptAnswer, undefined, "failed 回执不挂成果全文");
  assert.equal(byId("wo-2").status, "completed", "旧轮头回退标题解析");
  assert.ok(byId("wo-2").receiptAnswer, "回退口径下 completed 照旧挂全文");
});

// ============================================================
// 批次质检（纪律协议批）：质检轮证据进批次模型。
// ============================================================

function qcTurn(
  key: string,
  batch: { batchId: string; batchTitle?: string },
  state: "failed" | "running" | "completedSuccess" | "completedInterrupted" = "completedSuccess",
  options?: { review?: boolean },
): ConversationTurnRenderUnit {
  return unit(key, {
    header: turnHeaderRow({
      turnId: key,
      origin: "backgroundResult",
      state,
      originMeta: {
        backgroundSource: "agent_work_order_batch_qc",
        workId: batch.batchId,
        // 标题与口味同源同铸（buildBatchQcTitle）：合议批的轮头标题就是「合议 · X」。
        title: `${options?.review ? "合议" : "质检"} · ${batch.batchTitle}`,
        batchId: batch.batchId,
        ...(batch.batchTitle ? { batchTitle: batch.batchTitle } : {}),
        ...(options?.review ? { qcKind: "review" as const } : {}),
      },
    }),
  });
}

test("selectWorkOrderBatches：质检轮把工地卡跟到质检卡旁，完成态画质检完成灯", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "code-plus", task: "拆组件", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle }, { workOrderId: "wo-1", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "code-plus", delivery: "started" }),
      dispatchRow("call-2", { agent: "code-plus", task: "补样式", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle }, { workOrderId: "wo-2", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "code-plus", delivery: "started" }),
    ]),
    receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "好了"),
    receiptTurn("receipt-2", BATCH, "wo-2", "code-plus 交活", "也好了"),
    qcTurn("qc-turn", BATCH),
  ];
  const batches = selectWorkOrderBatches(units);
  assert.equal(batches.length, 1);
  assert.equal(batches[0]?.hostUnitKey, "qc-turn");
  assert.deepEqual(batches[0]?.qc, { unitKey: "qc-turn", state: "done" });
});

test("selectWorkOrderBatches：在跑的质检轮 → qc.state running（状态灯讲「正在质检」）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    qcTurn("qc-turn", BATCH, "running"),
  ];
  const batches = selectWorkOrderBatches(units);
  assert.deepEqual(batches[0]?.qc, { unitKey: "qc-turn", state: "running" });
});

test("selectWorkOrderBatches：派单行 input.review=true → batch.review=true（圆桌构图的依据，结构化非文本反推）", () => {
  const reviewTurn = dispatchTurn("review-dispatch", [
    dispatchRow("call-1", { agent: "code-reviewer", task: "评审登录页", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle, review: true }, { workOrderId: "wo-1", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "code-reviewer", delivery: "started" }),
    dispatchRow("call-2", { agent: "frontend-design", task: "评审登录页", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle, review: true }, { workOrderId: "wo-2", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "frontend-design", delivery: "started" }),
  ]);
  const reviewed = selectWorkOrderBatches([reviewTurn]);
  assert.equal(reviewed.length, 1);
  assert.equal(reviewed[0]?.review, true);

  const buildTurn = dispatchTurn("build-dispatch", [
    dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle }, { workOrderId: "wo-1", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "a", delivery: "started" }),
    dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle }, { workOrderId: "wo-2", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "b", delivery: "started" }),
  ]);
  const built = selectWorkOrderBatches([buildTurn]);
  assert.equal(built[0]?.review, undefined);
});

test("extractReviewVerdict：只认第一行格式行，自由文本一律不猜", () => {
  assert.equal(extractReviewVerdict(["评审结论：通过", "理由……"].join(String.fromCharCode(10))), "pass");
  assert.equal(extractReviewVerdict(["评审结论：不通过", "…"].join(String.fromCharCode(10))), "fail");
  assert.equal(extractReviewVerdict(["评审结论：有条件通过", "…"].join(String.fromCharCode(10))), "conditional");
  assert.equal(extractReviewVerdict("评审结论:通过"), "pass");
  // 格式行不在前三行内 / 不是三选一 / 缺席：宁可不戴徽章也不猜。
  // 格式行在前三行内仍认（弱模型可能先寒暄一句）；拖到第四行外就不认了。
  assert.equal(extractReviewVerdict(["好的。", "评审结论：通过"].join(String.fromCharCode(10))), "pass");
  assert.equal(extractReviewVerdict(["一", "二", "三", "评审结论：通过"].join(String.fromCharCode(10))), undefined);
  assert.equal(extractReviewVerdict("评审结论：差不多通过"), undefined);
  assert.equal(extractReviewVerdict(undefined), undefined);
});

test("selectWorkOrderBatches：评审会批的合议轮 → qc.review=true（合议词表的依据，不从标题反推）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "code-reviewer", task: "评审登录页", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle }, { workOrderId: "wo-1", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "code-reviewer", delivery: "started" }),
      dispatchRow("call-2", { agent: "frontend-design", task: "评审登录页", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle }, { workOrderId: "wo-2", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "frontend-design", delivery: "started" }),
    ]),
    qcTurn("council-turn", BATCH, "completedSuccess", { review: true }),
  ];
  const batches = selectWorkOrderBatches(units);
  assert.equal(batches.length, 1);
  assert.deepEqual(batches[0]?.qc, { unitKey: "council-turn", state: "done", review: true });
});

test("selectWorkOrderBatches：质检轮失败/被中断 → qc.state failed，绝不假报「完成」", () => {
  for (const state of ["failed", "completedInterrupted"] as const) {
    const units = [
      dispatchTurn("dispatch-turn", [
        dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
        dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
      ]),
      qcTurn("qc-turn", BATCH, state),
    ];
    const batches = selectWorkOrderBatches(units);
    assert.deepEqual(batches[0]?.qc, { unitKey: "qc-turn", state: "failed" }, state);
  }
});

test("selectWorkOrderBatches：全绿免检推导——回执全 completed 且无质检轮头 → qc.state skipped（2026-10-05）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    receiptTurn("receipt-1", BATCH, "wo-1", "a 交活", "干完了", "completed"),
    receiptTurn("receipt-2", BATCH, "wo-2", "b 交活", "也干完了", "completed"),
  ];
  const batch = selectWorkOrderBatches(units)[0]!;
  assert.deepEqual(batch.qc, { unitKey: "receipt-2", state: "skipped" });
  assert.equal(batch.hostUnitKey, "receipt-2", "免检批的工地卡贴在最新回执上");
});

test("selectWorkOrderBatches：有回执未 completed（在跑/失败/中断）不推免检", () => {
  const base = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    receiptTurn("receipt-1", BATCH, "wo-1", "a 交活", "干完了", "completed"),
  ];
  assert.equal(selectWorkOrderBatches(base)[0]?.qc, undefined, "缺回执（还在跑）不推");
  assert.equal(selectWorkOrderBatches([...base, receiptTurn("receipt-2", BATCH, "wo-2", "b 的工单未完成", "", "failed")])[0]?.qc, undefined, "有失败不推");
  assert.equal(selectWorkOrderBatches([...base, receiptTurn("receipt-2", BATCH, "wo-2", "b 的工单被中断", "", "cancelled")])[0]?.qc, undefined, "有中断不推");
});

test("selectWorkOrderBatches：评审批永不推免检（合议轮是裁决流程本身）", () => {
  const units = [
    unit("dispatch-turn", {
      assistantWorkRows: [
        dispatchRow("call-1", { agent: "a", task: "t", review: true, batch_title: BATCH.batchTitle }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started", review: true }),
        dispatchRow("call-2", { agent: "b", task: "t", review: true, batch_title: BATCH.batchTitle }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started", review: true }),
      ],
    }),
    receiptTurn("receipt-1", BATCH, "wo-1", "a 交活", "通过", "completed"),
    receiptTurn("receipt-2", BATCH, "wo-2", "b 交活", "通过", "completed"),
  ];
  const batch = selectWorkOrderBatches(units)[0]!;
  assert.equal(batch.review, true);
  assert.equal(batch.qc, undefined, "评审批缺席合议轮头时也不画免检灯");
});

// ============================================================
// 回执洪泛合并（2026-10-05）：合并轮头 originMeta.receipts 逐张记账。
// ============================================================

function mergedReceiptTurn(
  key: string,
  batch: { batchId: string; batchTitle?: string },
  items: { workId: string; title: string; receiptStatus: "completed" | "failed" | "cancelled" }[],
): ConversationTurnRenderUnit {
  return unit(key, {
    header: turnHeaderRow({
      turnId: key,
      origin: "backgroundResult",
      state: "completedSuccess",
      originMeta: {
        backgroundSource: "agent_work_order_receipt",
        workId: items[0]!.workId,
        title: items[0]!.title,
        receiptStatus: items[0]!.receiptStatus,
        batchId: batch.batchId,
        ...(batch.batchTitle ? { batchTitle: batch.batchTitle } : {}),
        receipts: items.map((item) => ({
          workId: item.workId,
          title: item.title,
          receiptStatus: item.receiptStatus,
          batchId: batch.batchId,
          ...(batch.batchTitle ? { batchTitle: batch.batchTitle } : {}),
        })),
      },
    }),
    assistantTextRows: [
      { kind: "assistantText" as const, rowId: 2, turnId: key, createdAt: 0, createdAtSeq: 0, text: "两单合并答复", state: "complete" as const },
    ],
    latestAssistantTextRow: {
      kind: "assistantText" as const,
      rowId: 2,
      turnId: key,
      createdAt: 0,
      createdAtSeq: 0,
      text: "两单合并答复",
      state: "complete" as const,
    },
  });
}

test("selectWorkOrderBatches：合并回执轮逐张进工地卡，合并张不挂摘要/成果全文（归属不可分）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    mergedReceiptTurn("receipt-merged", BATCH, [
      { workId: "wo-1", title: "a 交活", receiptStatus: "completed" },
      { workId: "wo-2", title: "b 交活", receiptStatus: "completed" },
    ]),
  ];
  const batch = selectWorkOrderBatches(units)[0]!;
  assert.deepEqual(
    batch.orders.map((order) => [order.key, order.status]),
    [
      ["wo-1", "completed"],
      ["wo-2", "completed"],
    ],
  );
  assert.equal(batch.hostUnitKey, "receipt-merged", "工地卡贴到合并轮");
  assert.equal(batch.orders[0]?.receiptAnswer, undefined, "合并张不挂成果全文（逐张归属不可分）");
  assert.equal(batch.orders[0]?.receiptText, undefined, "合并张不挂摘要");
});

test("selectWorkOrderBatches：合并轮全绿同样推免检（同批同时收口是最常见形态）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    mergedReceiptTurn("receipt-merged", BATCH, [
      { workId: "wo-1", title: "a 交活", receiptStatus: "completed" },
      { workId: "wo-2", title: "b 交活", receiptStatus: "completed" },
    ]),
  ];
  const batch = selectWorkOrderBatches(units)[0]!;
  assert.deepEqual(batch.qc, { unitKey: "receipt-merged", state: "skipped" });
});

test("selectWorkOrderBatches：质检轮里的打回重派工单行照常聚进工地卡", () => {
  const rework = dispatchRow(
    "call-qc-1",
    { agent: "code-plus", task: "拆组件\n【质检打回】样式不对", batch_id: BATCH.batchId, batch_title: BATCH.batchTitle },
    { workOrderId: "wo-3", batchId: BATCH.batchId, batchTitle: BATCH.batchTitle, agentName: "code-plus", delivery: "queued" },
  );
  // 质检轮自己的轮里带打回重派的工具行（同一 unit）。
  const qcWithRework = unit("qc-turn", {
    header: qcTurn("qc-turn", BATCH).header,
    assistantWorkRows: [rework],
  });
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    qcWithRework,
  ];
  const batches = selectWorkOrderBatches(units);
  assert.equal(batches.length, 1);
  const keys = batches[0]?.orders.map((order) => order.key);
  assert.ok(keys?.includes("wo-3"), "打回重派单应进批次");
  assert.equal(batches[0]?.hostUnitKey, "qc-turn");
});

test("selectWorkOrderBatchRenderInfo：质检轮是 host 也是 member（工地卡挂载靠 member 集；member 压制不波及质检卡）", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow("call-1", { agent: "a", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-1", batchId: BATCH.batchId, agentName: "a", delivery: "started" }),
      dispatchRow("call-2", { agent: "b", task: "t", batch_id: BATCH.batchId }, { workOrderId: "wo-2", batchId: BATCH.batchId, agentName: "b", delivery: "started" }),
    ]),
    qcTurn("qc-turn", BATCH),
  ];
  const info = selectWorkOrderBatchRenderInfo(units);
  const qcInfo = info.get("qc-turn")?.find((entry) => entry.batch.batchId === BATCH.batchId);
  assert.equal(qcInfo?.isHost, true);
  assert.equal(qcInfo?.isMember, true);
});

// ── 批次墙钟（2026-10-04，AgentCore 状态条同款）：站头权威工时只做 min/max 聚合 ──

function withHeaderTime(
  unit: ConversationTurnRenderUnit,
  startedAt: number,
  endedAt: number,
): ConversationTurnRenderUnit {
  return {
    ...unit,
    header: unit.header
      ? { ...unit.header, startedAt, endedAt }
      : turnHeaderRow({ turnId: unit.turnId, origin: "backgroundResult", state: "completedSuccess", startedAt, endedAt }),
  };
}

test("批次墙钟：startedAt 取最小、endedAt 取最大；收口批次带时间，缺席不编", () => {
  const units = [
    dispatchTurn("dispatch-turn", [
      dispatchRow(
        "call-1",
        { agent: "code-plus", task: "A", batch_title: "登录页改造" },
        { agentName: "code-plus", delivery: "started", workOrderId: "wo-1", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
      dispatchRow(
        "call-2",
        { agent: "doc-writer", task: "B", batch_title: "登录页改造" },
        { agentName: "doc-writer", delivery: "started", workOrderId: "wo-2", batchId: "batch-1", batchTitle: "登录页改造" },
      ),
    ]),
    withHeaderTime(receiptTurn("receipt-1", BATCH, "wo-1", "code-plus 交活", "完成"), 60_000, 125_000),
    withHeaderTime(receiptTurn("receipt-2", BATCH, "wo-2", "doc-writer 交活", "完成"), 61_000, 185_000),
  ];
  const batches = selectWorkOrderBatches(units);
  assert.equal(batches.length, 1);
  assert.equal(batches[0]!.startedAtMs, 60_000, "最早站头开局");
  assert.equal(batches[0]!.endedAtMs, 185_000, "最晚站头收工");

    // 没有任何带时间站头的批次（只有派单行，无回执/质检轮头）：时间字段缺席。
    const bare = selectWorkOrderBatches([
      dispatchTurn("dispatch-turn", [
        dispatchRow(
          "call-1",
          { agent: "code-plus", task: "A", batch_title: "B2" },
          { agentName: "code-plus", delivery: "started", workOrderId: "wo-9", batchId: "batch-2", batchTitle: "B2" },
        ),
        dispatchRow(
          "call-2",
          { agent: "doc-writer", task: "B", batch_title: "B2" },
          { agentName: "doc-writer", delivery: "started", workOrderId: "wo-10", batchId: "batch-2", batchTitle: "B2" },
        ),
      ]),
    ]);
    assert.equal(bare.length, 1);
    assert.equal(bare[0]!.startedAtMs, undefined);
    assert.equal(bare[0]!.endedAtMs, undefined);
  });

test("formatBatchDuration：秒/分/时三段格式与残缺兜底", () => {
  assert.equal(formatBatchDuration(42_000), "42s");
  assert.equal(formatBatchDuration(192_000), "3m12s");
  assert.equal(formatBatchDuration(3_840_000), "1h04m");
  assert.equal(formatBatchDuration(-1), "—");
  assert.equal(formatBatchDuration(Number.NaN), "—");
});
