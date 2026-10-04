// ============================================================
// 派单回执重试链的可运行检查（2026-10-01 员工可靠性批，评审 A 的 P1）。
// 驱动**真实** scheduleWorkOrderReceiptRelay（不是镜像副本）：员工侧假 runtime
// 复刻 core 的事件分发时序（notifyEventSinks 的 for..of Set 迭代 + 零 await 的
// enqueueAgentWorkOrder），老板侧假 runtime 只记录收到的回执终态。验证：
//  1. 第一棒失败分发期间零投递（重试订阅推迟到宏任务，不会被同一次迭代访问到）；
//  2. 两棒都失败 → 恰好一条回执、盖 retried 章、由第二棒触发；
//  3. 重试成功 → 只投 completed、不盖章；
//  4. 重试 enqueue 抛错（会话关闭）→ 原失败如实送达、无章；
//  5. 看门狗：目标会话死掉 → 合成超时失败回执（重派入口不丢）；本单还在跑 →
//     重新武装不误杀长任务。
// 每条测试结尾必须退订：默认看门狗 30 分钟，不退订会把测试进程吊住。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/agent-dispatch-receipts.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { SessionEventType } from "@zcode/contracts";
import {
  receiptOutcomeFromSessionEvent,
  scheduleWorkOrderReceiptRelay,
} from "../src/zcode-protocol/agent-dispatch-receipts.js";

const WORK_ORDER_INPUT_ID_PREFIX = "workorder-";
const INPUT_ID = `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`;
const BOSS_SESSION = "sess-boss";
const WORKER_SESSION = "sess-worker";

interface RecordedReceipt {
  status: string;
  retried: boolean;
  failureCode?: string;
}

/** 员工侧：Set 迭代分发 + 零 await 的 enqueue（微任务签名与 core 同形，这是竞态的关键）。 */
function makeWorkerRuntime() {
  const sinks = new Set<{ onSessionEvent: (event: never) => void }>();
  let failEnqueue = false;
  let busy = false;
  return {
    setFailEnqueue: (value: boolean) => {
      failEnqueue = value;
    },
    setBusy: (value: boolean) => {
      busy = value;
    },
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
    async enqueueAgentWorkOrder() {
      if (failEnqueue) {
        throw new Error("Cannot accept an agent work order during session shutdown");
      }
      return { delivery: "started" as const, inputId: INPUT_ID, workOrderId: "wo-1" };
    },
    // 看门狗探测面：busy = 本单还在目标会话里活跃（长任务合法超时）。
    getActiveTurnInfo() {
      return busy ? { inputId: INPUT_ID } : undefined;
    },
    hasActiveOrQueuedTurnWork() {
      return busy;
    },
  };
}

/** 老板侧：只记录回执（deliverWorkOrderReceipt 的落点）。 */
function makeBossRuntime() {
  const receipts: RecordedReceipt[] = [];
  return {
    receipts,
    enqueueAgentWorkOrderReceipt(input: {
      outcome: { status: string; retried?: boolean; failureCode?: string };
    }) {
      receipts.push({
        status: input.outcome.status,
        retried: input.outcome.retried === true,
        ...(input.outcome.failureCode ? { failureCode: input.outcome.failureCode } : {}),
      });
    },
  };
}

function harness(options?: { watchdogMs?: number }) {
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
    envelope: { workOrderId: "wo-1", fromSessionId: BOSS_SESSION } as never,
    agentName: "worker",
    ...(options?.watchdogMs === undefined ? {} : { watchdogMs: options.watchdogMs }),
  });
  return { worker, boss, unsubscribe };
}

function turnError(reason: string) {
  return {
    type: SessionEventType.TurnError,
    payload: { inputId: INPUT_ID, error: { message: reason, code: "invalid_model_request" } },
  };
}

function turnCompleteSuccess() {
  return {
    type: SessionEventType.TurnComplete,
    payload: { inputId: INPUT_ID, resultType: "success", response: "done" },
  };
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

test("第一棒失败分发期间零投递（重试订阅推迟到宏任务，P1 竞态不复发）", async () => {
  const { worker, boss, unsubscribe } = harness({ watchdogMs: 5 });
  await worker.notifyEventSinks(turnError("boom attempt-1"));
  assert.equal(boss.receipts.length, 0);
  await tick();
  assert.equal(boss.receipts.length, 0, "重试订阅+enqueue 落定后也不该有回执");
  unsubscribe();
});

test("两棒都失败：恰好一条回执、盖 retried 章、带结构化失败线索", async () => {
  const { worker, boss, unsubscribe } = harness({ watchdogMs: 5 });
  await worker.notifyEventSinks(turnError("boom attempt-1"));
  await tick();
  await worker.notifyEventSinks(turnError("boom attempt-2"));
  await tick();
  assert.deepEqual(boss.receipts, [
    { status: "failed", retried: true, failureCode: "invalid_model_request" },
  ]);
  unsubscribe();
});

test("重试成功：只投 completed、不盖 retried 章", async () => {
  const { worker, boss, unsubscribe } = harness({ watchdogMs: 5 });
  await worker.notifyEventSinks(turnError("boom"));
  await tick();
  await worker.notifyEventSinks(turnCompleteSuccess());
  await tick();
  assert.deepEqual(boss.receipts, [{ status: "completed", retried: false }]);
  unsubscribe();
});

test("重试 enqueue 抛错（会话关闭）：原失败如实送达、无 retried 章", async () => {
  const { worker, boss, unsubscribe } = harness({ watchdogMs: 5 });
  worker.setFailEnqueue(true);
  await worker.notifyEventSinks(turnError("boom"));
  await tick();
  assert.deepEqual(boss.receipts, [
    { status: "failed", retried: false, failureCode: "invalid_model_request" },
  ]);
  unsubscribe();
});

test("看门狗：目标会话死掉后合成超时失败回执（重派入口不丢）", async () => {
  const { boss, unsubscribe } = harness({ watchdogMs: 10 });
  await new Promise((resolve) => setTimeout(resolve, 120));
  assert.equal(boss.receipts.length, 1);
  assert.equal(boss.receipts[0]!.status, "failed");
  unsubscribe();
});

test("看门狗：本单还在跑就重新武装，不误杀长任务", async () => {
  const { worker, boss, unsubscribe } = harness({ watchdogMs: 10 });
  worker.setBusy(true);
  await new Promise((resolve) => setTimeout(resolve, 120));
  assert.equal(boss.receipts.length, 0, "长任务在跑，看门狗不得开火");
  worker.setBusy(false);
  await new Promise((resolve) => setTimeout(resolve, 120));
  assert.equal(boss.receipts.length, 1, "确认死单后才合成");
  unsubscribe();
});

// ============================================================
// 证据门禁（地基清理·质检免检 2026-10-05）：TurnComplete 载荷的 toolCallCount
// 随 outcome 回传，批次质检闸门判绿用（completed 且 >0 = 真动过手）。
// ============================================================
test("receiptOutcomeFromSessionEvent：toolCallCount 随 success 终态回传，失败终态不带", () => {
  const success = receiptOutcomeFromSessionEvent(
    {
      type: SessionEventType.TurnComplete,
      payload: { inputId: INPUT_ID, resultType: "success", response: "干完了", toolCallCount: 4.7 },
    } as never,
    INPUT_ID,
  );
  assert.equal(success?.status, "completed");
  assert.equal(success?.toolCallCount, 4, "应向下取整为非负整数");

  const noCount = receiptOutcomeFromSessionEvent(
    {
      type: SessionEventType.TurnComplete,
      payload: { inputId: INPUT_ID, resultType: "success", response: "干完了" },
    } as never,
    INPUT_ID,
  );
  assert.equal(noCount?.toolCallCount, undefined, "载荷缺席时不编造");

  const failed = receiptOutcomeFromSessionEvent(
    {
      type: SessionEventType.TurnError,
      payload: { inputId: INPUT_ID, error: { message: "炸了" } },
    } as never,
    INPUT_ID,
  );
  assert.equal(failed?.status, "failed");
  assert.equal(failed?.toolCallCount, undefined, "失败终态不带证据项");
});
