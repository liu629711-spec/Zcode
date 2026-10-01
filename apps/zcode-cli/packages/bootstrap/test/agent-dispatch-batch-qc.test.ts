// ============================================================
// 批次质检触发（纪律协议批）的端到端可运行检查：驱动**真实**
// scheduleWorkOrderReceiptRelay + 真实 deliverWorkOrderReceipt 链路——员工侧假
// runtime 复刻 core 的事件分发时序，老板侧假 runtime 记录收到的回执与质检触发
// （触发判定本体在 core runtime 上，由 work-order-batch-qc.test.ts 钉住；这里钉
// 「回执投递后触发入口被走到、且带对批次」这条接线）。验证：
//  1. 带批次的回执落终态 → 老板侧质检触发入口被调用、batchId 正确；
//  2. 散单（无批次）→ 不触发。
// 每条测试结尾必须退订：默认看门狗 30 分钟，不退订会把测试进程吊住。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/agent-dispatch-batch-qc.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { SessionEventType } from "@zcode/contracts";
import { scheduleWorkOrderReceiptRelay } from "../src/zcode-protocol/agent-dispatch-receipts.js";

const WORK_ORDER_INPUT_ID_PREFIX = "workorder-";
const INPUT_ID = `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`;
const BOSS_SESSION = "sess-boss";
const WORKER_SESSION = "sess-worker";
const BATCH_ID = "batch-1";

function makeWorkerRuntime() {
  const sinks = new Set<{ onSessionEvent: (event: never) => void }>();
  return {
    async notifyEventSinks(event: { type: SessionEventType; payload: Record<string, unknown> }) {
      for (const sink of sinks) {
        await sink.onSessionEvent(event as never);
      }
    },
    subscribeEvents(input: { onSessionEvent: (event: never) => void }) {
      const sink = { onSessionEvent: input.onSessionEvent };
      sinks.add(sink);
      return () => {
        sinks.delete(sink);
      };
    },
    getActiveTurnInfo() {
      return undefined;
    },
    hasActiveOrQueuedTurnWork() {
      return false;
    },
  };
}

function makeBossRuntime() {
  const receipts: string[] = [];
  const qcTriggers: string[] = [];
  return {
    receipts,
    qcTriggers,
    enqueueAgentWorkOrderReceipt() {
      receipts.push("receipt");
    },
    maybeEnqueueAgentWorkOrderBatchQc(input: { batchId: string }) {
      qcTriggers.push(input.batchId);
      return Promise.resolve();
    },
  };
}

function harness(input: { envelopeBatchId?: string }) {
  const worker = makeWorkerRuntime();
  const boss = makeBossRuntime();
  const context = {
    logger: undefined,
    sessions: new Map([
      [BOSS_SESSION, { app: { runtime: boss, sessionId: BOSS_SESSION }, traceContext: undefined }],
    ]),
  };
  const deps = { activateSessionRecord: async () => ({}) as never };
  const unsubscribe = scheduleWorkOrderReceiptRelay(context as never, deps, {
    targetRecord: {
      app: { runtime: worker, sessionId: WORKER_SESSION },
      traceContext: undefined,
    } as never,
    inputId: INPUT_ID,
    envelope: {
      workOrderId: "wo-1",
      fromSessionId: BOSS_SESSION,
      ...(input.envelopeBatchId === undefined ? {} : { batchId: input.envelopeBatchId }),
    } as never,
    agentName: "worker",
    watchdogMs: 5,
  });
  return { worker, boss, unsubscribe };
}

test("带批次的回执落终态：质检触发入口被走到、batchId 正确（receipt 照常投递）", async () => {
  const { worker, boss, unsubscribe } = harness({ envelopeBatchId: BATCH_ID });
  await worker.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: INPUT_ID, resultType: "success", response: "done" },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(boss.receipts, ["receipt"]);
  assert.deepEqual(boss.qcTriggers, [BATCH_ID]);
  unsubscribe();
});

test("散单（信封无批次）：不触发质检", async () => {
  const { worker, boss, unsubscribe } = harness({});
  await worker.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: INPUT_ID, resultType: "success", response: "done" },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(boss.receipts, ["receipt"]);
  assert.deepEqual(boss.qcTriggers, []);
  unsubscribe();
});
