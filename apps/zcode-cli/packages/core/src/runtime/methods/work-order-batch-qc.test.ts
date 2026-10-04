// ============================================================
// 批次质检触发（纪律协议批）的可运行检查：驱动**真实**
// maybeEnqueueAgentWorkOrderBatchQc + enqueueAgentWorkOrderBatchQc（互调用真实
// 实现，只有 sessionStore/enqueueRuntimeCommand 是假件）。台账行用**真实 payload
// 形状**：员工名在 payload 顶层 agentName、信封 fromAgentName 是发起方署名
// （评审 B1 的钉子——夹具写反会让 bug 焊死在绿色里）。验证：
//  1. 同批全部收口 → 开一次，清单按工单号定序、员工名取顶层字段、工号随行；
//  2. 闸门行已在（任意状态）→ 不开；同批还有 admitted → 不开；孤儿批次 → 不开；
//  3. 并发两次触发（同批两单同时销账）→ 串行链 + 闸门只放一次；
//  4. 落闸（save）先于开轮（enqueueRuntimeCommand）——评审 A4 的真不变量；
//  5. 闸门行 id 带会话命名空间——评审 A3；
//  6. 评审会批（2026-10-02）：全评审单 → 合议口味、混批/旧批 → 质检、合议轮
//     选项带 toolDisallowlist（禁派单是机制不是提示词空话）。触发入口与 resume
//     清扫（steering 的 discardPersistedPendingSteerInputs）是同一个
//     maybeEnqueueAgentWorkOrderBatchQc，这里即覆盖清扫路径的口味推导。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/runtime/methods/work-order-batch-qc.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { WorkOrderBatchQcRuntimeCommand } from "../command-queue.js";
import {
  batchQcContentKey,
  batchQcGateId,
  batchQcLedgerId,
  enqueueAgentWorkOrderBatchQc,
  maybeEnqueueAgentWorkOrderBatchQc,
  runWorkOrderBatchQcCommand,
} from "./work-order-batch-qc.js";

const SESSION = "sess-boss";
const BATCH_ID = "batch-1";

interface LedgerRow {
  id: string;
  sessionID: string;
  kind: string;
  delivery: string;
  payload: { text: string; [key: string]: unknown };
  admittedSequence: number;
  status: "admitted" | "discarded";
  time: { created: number; updated: number };
}

/** 与 agent-dispatch-port.ts 落账的真实 payload 形状一致（员工名在顶层、信封里是发起方署名）。 */
function dispatchRow(
  workOrderId: string,
  status: "admitted" | "discarded",
  options?: { review?: boolean; batchId?: string },
): LedgerRow {
  return {
    id: `agentWorkOrderDispatch:${workOrderId}`,
    sessionID: SESSION,
    kind: "agentWorkOrderDispatch",
    delivery: "queue",
    payload: {
      text: `任务${workOrderId}`,
      workOrderId,
      agentName: `员工-${workOrderId}`,
      agentId: `agent-${workOrderId}`,
      targetSessionId: "sess-worker",
      envelope: {
        workOrderId,
        fromAgentName: "老板",
        fromSessionId: SESSION,
        task: `任务${workOrderId}`,
        batchId: options?.batchId ?? BATCH_ID,
        batchTitle: "登录页整改",
        ...(options?.review ? { review: true } : {}),
      },
    },
    admittedSequence: 1,
    status,
    time: { created: 0, updated: 0 },
  };
}

function makeRuntime(rows: LedgerRow[]) {
  const sequence: string[] = [];
  const qcCommands: WorkOrderBatchQcRuntimeCommand[] = [];
  const store = {
    rows,
    async listSessionInputs() {
      return this.rows;
    },
    saveSessionInput(input: { id: string; sessionID: string; kind: string; delivery: string; payload: object }) {
      sequence.push(`save:${input.id}`);
      this.rows.push(input as unknown as LedgerRow);
      return { id: input.id };
    },
  };
  const runtime = {
    sessionId: SESSION,
    shuttingDown: false,
    branchGeneration: 0,
    rootTraceContext: { traceId: "trace" },
    logger: undefined,
    sessionStore: store,
    store,
    trackResidencyBlockingWork: async () => ({}),
    enqueueRuntimeCommand(command: WorkOrderBatchQcRuntimeCommand) {
      sequence.push(`enqueue:${command.batchId}`);
      qcCommands.push(command);
    },
    qcCommands,
    sequence,
  };
  (runtime as unknown as Record<string, unknown>).enqueueAgentWorkOrderBatchQc = (
    input: Parameters<typeof enqueueAgentWorkOrderBatchQc>[0],
  ) => enqueueAgentWorkOrderBatchQc.call(runtime as never, input);
  return runtime as unknown as TestRuntime;
}

type TestRuntime = Record<string, unknown> & {
  qcCommands: WorkOrderBatchQcRuntimeCommand[];
  sequence: string[];
  store: { rows: LedgerRow[] };
};

const trigger = (runtime: TestRuntime, batchId = BATCH_ID) =>
  maybeEnqueueAgentWorkOrderBatchQc.call(runtime as never, { batchId });

/** 回执台账行（与 work-order-receipts.ts 落账的 payload 形状一致）。 */
function receiptRow(
  workOrderId: string,
  outcome: { status: string; toolCallCount?: number },
  sequence = 10,
): LedgerRow {
  return {
    id: `receipt-${workOrderId}-${sequence}`,
    sessionID: SESSION,
    kind: "agentWorkOrderReceipt",
    delivery: "queue",
    payload: {
      text: `回执${workOrderId}`,
      workOrderId,
      outcome,
    },
    admittedSequence: sequence,
    status: "discarded",
    time: { created: 0, updated: 0 },
  };
}

test("证据门禁：全部回执 completed 且动过手 → 免检，不排队质检轮、落 skipped 闸门行", async () => {
  const runtime = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    dispatchRow("wo-2", "discarded"),
    receiptRow("wo-1", { status: "completed", toolCallCount: 3 }, 11),
    receiptRow("wo-2", { status: "completed", toolCallCount: 1 }, 12),
  ]);
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 0, "全绿批次不该开质检轮");
  const skipRow = runtime.store.rows.find(
    (row: LedgerRow) => row.kind === "agentWorkOrderBatchQc",
  ) as LedgerRow | undefined;
  assert.ok(skipRow, "免检也要落内容锚定闸门行（这批永不重开）");
  assert.equal((skipRow.payload as { skipped?: boolean }).skipped, true);
});

test("证据门禁：一张单零工具调用（可能摸鱼）→ 照旧开质检轮", async () => {
  const runtime = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    dispatchRow("wo-2", "discarded"),
    receiptRow("wo-1", { status: "completed", toolCallCount: 5 }, 11),
    receiptRow("wo-2", { status: "completed", toolCallCount: 0 }, 12),
  ]);
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 1);
});

test("证据门禁：取消/失败的回执不是绿 → 照旧开质检轮", async () => {
  const cancelled = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    receiptRow("wo-1", { status: "cancelled", toolCallCount: 2 }, 11),
  ]);
  await trigger(cancelled);
  assert.equal(cancelled.qcCommands.length, 1);

  const failed = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    receiptRow("wo-1", { status: "failed", toolCallCount: 9 }, 11),
  ]);
  await trigger(failed);
  assert.equal(failed.qcCommands.length, 1);
});

test("证据门禁：回执行缺席（旧批次）fail-open 照旧开轮；评审批全绿也不免检", async () => {
  const legacy = makeRuntime([dispatchRow("wo-1", "discarded")]);
  await trigger(legacy);
  assert.equal(legacy.qcCommands.length, 1);

  const review = makeRuntime([
    dispatchRow("wo-1", "discarded", { review: true }),
    dispatchRow("wo-2", "discarded", { review: true }),
    receiptRow("wo-1", { status: "completed", toolCallCount: 4 }, 11),
    receiptRow("wo-2", { status: "completed", toolCallCount: 4 }, 12),
  ]);
  await trigger(review);
  assert.equal(review.qcCommands.length, 1, "合议轮是裁决流程本身，永不免检");
});

test("latestReceiptOutcomeByWorkOrderId：同单多张回执取 admittedSequence 最新", async () => {
  const runtime = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    receiptRow("wo-1", { status: "completed", toolCallCount: 0 }, 11),
    receiptRow("wo-1", { status: "completed", toolCallCount: 6 }, 12),
  ]);
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 0, "最新回执是绿的就算绿");
});

test("同批全部收口：开一次，员工名取台账顶层字段、工号随行、按工单号定序", async () => {
  const runtime = makeRuntime([dispatchRow("wo-2", "discarded"), dispatchRow("wo-1", "discarded")]);
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 1);
  const command = runtime.qcCommands[0]!;
  assert.equal(command.batchId, BATCH_ID);
  assert.equal(command.batchTitle, "登录页整改");
  assert.deepEqual(
    command.orders.map((order) => order.workOrderId),
    ["wo-1", "wo-2"],
  );
  // B1 钉子：员工名是台账顶层的 agentName（员工），不是信封里的发起方署名。
  assert.equal(command.orders[0]?.agentName, "员工-wo-1");
  assert.equal(command.orders[0]?.agentId, "agent-wo-1");
  assert.match(command.text, /agent-id="agent-wo-1"/);
});

test("闸门行已在（discarded 也算）：不开；同批还有 admitted：不开；孤儿批次：不开", async () => {
  const gated = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    {
      id: batchQcLedgerId(SESSION, BATCH_ID),
      sessionID: SESSION,
      kind: "agentWorkOrderBatchQc",
      delivery: "queue",
      payload: { text: "gate" },
      admittedSequence: 9,
      status: "discarded",
      time: { created: 0, updated: 0 },
    },
  ]);
  await trigger(gated);
  assert.equal(gated.qcCommands.length, 0);

  const inFlight = makeRuntime([dispatchRow("wo-1", "discarded"), dispatchRow("wo-2", "admitted")]);
  await trigger(inFlight);
  assert.equal(inFlight.qcCommands.length, 0);

  const orphan = makeRuntime([]);
  await trigger(orphan);
  assert.equal(orphan.qcCommands.length, 0);
});

test("并发两次触发（同批两单同时销账）：串行链 + 闸门只放一次", async () => {
  const runtime = makeRuntime([dispatchRow("wo-1", "discarded"), dispatchRow("wo-2", "discarded")]);
  await Promise.all([trigger(runtime), trigger(runtime)]);
  assert.equal(runtime.qcCommands.length, 1);
});

test("落闸先于开轮（A4 真不变量）：save 一定排在 enqueueRuntimeCommand 之前", async () => {
  const runtime = makeRuntime([dispatchRow("wo-1", "discarded")]);
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 1);
  const gateIndex = runtime.sequence.findIndex((entry) =>
    entry.startsWith(`save:${batchQcGateId(SESSION, batchQcContentKey([{ workOrderId: "wo-1" }]))}`),
  );
  const enqueueIndex = runtime.sequence.findIndex((entry) => entry.startsWith("enqueue:"));
  assert.ok(gateIndex !== -1, "闸门行应已落库（内容锚定键）");
  assert.ok(enqueueIndex !== -1);
  assert.ok(gateIndex < enqueueIndex, "落闸必须先于开轮");
});

test("闸门行 id 带会话命名空间（A3）且锚定工单集合：跨会话/换 batchId 都不共享闸门", async () => {
  const runtime = makeRuntime([dispatchRow("wo-1", "discarded")]);
  await trigger(runtime);
  assert.ok(
    runtime.sequence.some(
      (entry) =>
        entry.startsWith(
          `save:${batchQcGateId(SESSION, batchQcContentKey([{ workOrderId: "wo-1" }]))}`,
        ),
    ),
    "落闸写内容锚定键",
  );
});

test("换 batchId 不重开质检（内容锚定，2026-10-04 地基清理）：同组工单换号=同一批", async () => {
  // 模型把同一组工单（wo-1/wo-2）报成新 batchId "batch-1b"：内容键相同 → 闸门命中。
  const sameOrdersNewId = makeRuntime([
    dispatchRow("wo-1", "discarded", { batchId: "batch-1b" }),
    dispatchRow("wo-2", "discarded", { batchId: "batch-1b" }),
    {
      id: batchQcGateId(SESSION, batchQcContentKey([{ workOrderId: "wo-1" }, { workOrderId: "wo-2" }])),
      sessionID: SESSION,
      kind: "agentWorkOrderBatchQc",
      delivery: "queue",
      payload: { text: "gate" },
      admittedSequence: 9,
      status: "discarded",
      time: { created: 0, updated: 0 },
    },
  ]);
  await trigger(sameOrdersNewId, "batch-1b");
  assert.equal(sameOrdersNewId.qcCommands.length, 0, "换号绕闸被内容锚定拦下");

  // 历史批次闸门行在旧 batchId 键下：兜底检查同样拦截，不因换键式而重开。
  const legacyGate = makeRuntime([
    dispatchRow("wo-1", "discarded"),
    {
      id: batchQcLedgerId(SESSION, BATCH_ID),
      sessionID: SESSION,
      kind: "agentWorkOrderBatchQc",
      delivery: "queue",
      payload: { text: "legacy gate" },
      admittedSequence: 9,
      status: "discarded",
      time: { created: 0, updated: 0 },
    },
  ]);
  await trigger(legacyGate);
  assert.equal(legacyGate.qcCommands.length, 0, "旧键闸门行兜底拦截");

  // 真正的新一批（不同工单集合）：新键不命中 → 照常质检。
  const freshBatch = makeRuntime([
    dispatchRow("wo-9", "discarded", { batchId: "batch-2" }),
    dispatchRow("wo-10", "discarded", { batchId: "batch-2" }),
  ]);
  await trigger(freshBatch, "batch-2");
  assert.equal(freshBatch.qcCommands.length, 1, "新批次照常开质检");
});

test("落闸失败：放弃开轮（fail-closed，评审 R1）", async () => {
  const runtime = makeRuntime([dispatchRow("wo-1", "discarded")]);
  (runtime as unknown as {
    sessionStore: { saveSessionInput: () => Promise<never> };
  }).sessionStore.saveSessionInput = async () => {
    throw new Error("db down");
  };
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 0, "闸门写不进就不能开轮");
});

test("评审会批：同批全是评审单 → 合议口味（review 命令 + qcKind + 合议正文）", async () => {
  const runtime = makeRuntime([
    dispatchRow("wo-1", "discarded", { review: true }),
    dispatchRow("wo-2", "discarded", { review: true }),
  ]);
  await trigger(runtime);
  assert.equal(runtime.qcCommands.length, 1);
  const command = runtime.qcCommands[0]!;
  assert.equal(command.review, true);
  assert.equal(command.originMeta.qcKind, "review");
  assert.equal(command.originMeta.title, "合议 · 登录页整改");
  assert.match(command.text, /合议要求/);
  assert.ok(!command.text.includes("打回"), "合议没有打回权");
});

test("混批（评审单 + 施工单）收敛为质检口味：旧批次与畸形台账口径不变", async () => {
  const mixed = makeRuntime([
    dispatchRow("wo-1", "discarded", { review: true }),
    dispatchRow("wo-2", "discarded"),
  ]);
  await trigger(mixed);
  assert.equal(mixed.qcCommands.length, 1);
  assert.equal(mixed.qcCommands[0]!.review, undefined);
  assert.equal(mixed.qcCommands[0]!.originMeta.qcKind, undefined);
  assert.match(mixed.qcCommands[0]!.text, /质检要求/);
  // 混批兜底成质检时，评审单在清单里带 review="true" 属性，质检诊断只核对不计数。
  assert.match(mixed.qcCommands[0]!.text, /<order id="wo-1" agent="员工-wo-1" agent-id="agent-wo-1" review="true">/);
  assert.match(mixed.qcCommands[0]!.text, /同样只核对意见是否与任务对应/);

  const legacy = makeRuntime([dispatchRow("wo-1", "discarded")]);
  await trigger(legacy);
  assert.equal(legacy.qcCommands[0]!.review, undefined);
  assert.match(legacy.qcCommands[0]!.text, /质检要求/);
});

test("诊断轮禁派单是机制不是空话：质检与合议轮的轮选项都带 toolDisallowlist", async () => {
  // 复用 startBatchQcIfComplete（活回执投递与 resume 清扫共用同一入口）的产出，
  // 直接驱动真实 runWorkOrderBatchQcCommand，捕获 executeTurnCommand 收到的选项。
  const captured: Record<string, unknown>[] = [];
  const makeRunRuntime = (command: WorkOrderBatchQcRuntimeCommand) =>
    ({
      sessionId: SESSION,
      shuttingDown: false,
      branchGeneration: 0,
      rootTraceContext: { traceId: "trace" },
      activeForegroundExecution: undefined,
      messageHistory: { addUser: () => undefined },
      persistSyntheticUserNoticeForSession: async () => undefined,
      sessionStore: undefined,
      executeTurnCommand: async (_text: string, _x: unknown, options: Record<string, unknown>) => {
        captured.push(options);
      },
    }) as never;
  const buildCommand = (review: boolean): WorkOrderBatchQcRuntimeCommand =>
    ({
      branchGeneration: 0,
      createdAt: new Date(),
      id: "cmd-1",
      mode: "work-order-batch-qc",
      originMeta: {
        backgroundSource: "agent_work_order_batch_qc",
        workId: BATCH_ID,
        title: review ? "合议 · 登录页整改" : "质检 · 登录页整改",
        batchId: BATCH_ID,
      },
      orders: [{ workOrderId: "wo-1", agentName: "a", task: "t" }],
      batchId: BATCH_ID,
      ...(review ? { review: true } : {}),
      priority: "next",
      source: "agent_work_order_batch_qc",
      text: "envelope",
      traceContext: { traceId: "trace" },
    }) as unknown as WorkOrderBatchQcRuntimeCommand;
  await runWorkOrderBatchQcCommand.call(makeRunRuntime(buildCommand(true)), buildCommand(true));
  await runWorkOrderBatchQcCommand.call(makeRunRuntime(buildCommand(false)), buildCommand(false));
  assert.equal(captured.length, 2);
  // advisory 改造（2026-10-04）：质检与合议一律禁派单——诊断建议的采纳权在老板。
  assert.deepEqual(captured[0]!.toolDisallowlist, ["AgentDispatch"]);
  assert.deepEqual(captured[1]!.toolDisallowlist, ["AgentDispatch"]);
});
