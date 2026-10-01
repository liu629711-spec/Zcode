// ============================================================
// 批次质检触发（纪律协议批）的可运行检查。驱动**真实** maybeStartBatchQc：
// 假台账复刻 session_inputs 的行形状（agentWorkOrderDispatch / 闸门行），
// 老板侧假 runtime 记录质检开轮并**模拟 core 落闸门行**（触发侧串行链 +
// 台账行合起来才构成「每批只质检一次」）。验证：
//  1. 同批派单行全部收口 → 质检恰好开一次，清单按工单号定序、批次标题随行；
//  2. 闸门行已在（含 discarded）→ 不开（重派回执到货不再二检）；
//  3. 同批还有一张 admitted → 不开（批次没收口）；
//  4. 信封无批次 → 直通不开；
//  5. 并发两次触发（同批两张回执同时销账）→ 串行链 + 闸门只放一次。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/agent-dispatch-batch-qc.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { maybeStartBatchQc } from "../src/zcode-protocol/agent-dispatch-receipts.js";

const BOSS_SESSION = "sess-boss";
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

function dispatchRow(workOrderId: string, status: "admitted" | "discarded"): LedgerRow {
  return {
    id: `agentWorkOrderDispatch:${workOrderId}`,
    sessionID: BOSS_SESSION,
    kind: "agentWorkOrderDispatch",
    delivery: "queue",
    payload: {
      text: "carrier",
      workOrderId,
      envelope: {
        workOrderId,
        fromAgentName: `员工-${workOrderId}`,
        fromSessionId: "sess-worker",
        task: `任务${workOrderId}`,
        batchId: BATCH_ID,
        batchTitle: "登录页整改",
      },
    },
    admittedSequence: 1,
    status,
    time: { created: 0, updated: 0 },
  };
}

function gateRow(): LedgerRow {
  return {
    id: `agentWorkOrderBatchQc:${BATCH_ID}`,
    sessionID: BOSS_SESSION,
    kind: "agentWorkOrderBatchQc",
    delivery: "queue",
    payload: { text: "gate" },
    admittedSequence: 9,
    status: "admitted",
    time: { created: 0, updated: 0 },
  };
}

function harness(rows: LedgerRow[]) {
  const store = {
    rows,
    async listSessionInputs() {
      return this.rows;
    },
  };
  const qcCalls: Array<{ batchId: string; batchTitle?: string; orders: unknown[] }> = [];
  const boss = {
    // 模拟 core 的闸门落行：质检一旦开轮，闸门行立刻可见（并发触发的锚）。
    enqueueAgentWorkOrderBatchQc(input: { batchId: string; batchTitle?: string; orders: unknown[] }) {
      qcCalls.push(input);
      store.rows.push(gateRow());
    },
  };
  const context = {
    logger: undefined,
    deps: { sessionStore: store },
    sessions: new Map([
      [BOSS_SESSION, { app: { runtime: boss, sessionId: BOSS_SESSION }, traceContext: undefined }],
    ]),
  };
  const deps = { activateSessionRecord: async () => ({}) as never };
  const trigger = (envelope: { batchId?: string }) =>
    maybeStartBatchQc(context as never, deps, {
      fromSessionId: BOSS_SESSION,
      envelope: { batchId: BATCH_ID, ...envelope } as never,
    });
  return { qcCalls, trigger };
}

test("同批全部收口：质检开一次，清单按工单号定序、批次标题随行", async () => {
  const { qcCalls, trigger } = harness([
    dispatchRow("wo-2", "discarded"),
    dispatchRow("wo-1", "discarded"),
    { ...gateRow(), id: "别的批次闸门" },
  ]);
  await trigger({ batchId: BATCH_ID });
  assert.equal(qcCalls.length, 1);
  assert.equal(qcCalls[0]?.batchId, BATCH_ID);
  assert.equal(qcCalls[0]?.batchTitle, "登录页整改");
  assert.deepEqual(
    qcCalls[0]?.orders.map((order) => (order as { workOrderId: string }).workOrderId),
    ["wo-1", "wo-2"],
  );
});

test("闸门行已在（discarded 也算）：重派回执到货不再二检", async () => {
  const { qcCalls, trigger } = harness([
    dispatchRow("wo-1", "discarded"),
    { ...gateRow(), status: "discarded" },
  ]);
  await trigger({});
  assert.equal(qcCalls.length, 0);
});

test("同批还有 admitted 行：批次没收口，不开质检", async () => {
  const { qcCalls, trigger } = harness([
    dispatchRow("wo-1", "discarded"),
    dispatchRow("wo-2", "admitted"),
  ]);
  await trigger({});
  assert.equal(qcCalls.length, 0);
});

test("信封无批次：直通不开", async () => {
  const { qcCalls, trigger } = harness([dispatchRow("wo-1", "discarded")]);
  await trigger({ batchId: undefined });
  assert.equal(qcCalls.length, 0);
});

test("并发两次触发（同批两单同时销账）：串行链 + 闸门只放一次", async () => {
  const { qcCalls, trigger } = harness([
    dispatchRow("wo-1", "discarded"),
    dispatchRow("wo-2", "discarded"),
  ]);
  await Promise.all([trigger({}), trigger({})]);
  assert.equal(qcCalls.length, 1);
});

test("台账里没有本批派单行：不开（防孤儿批次）", async () => {
  const { qcCalls, trigger } = harness([]);
  await trigger({});
  assert.equal(qcCalls.length, 0);
});
