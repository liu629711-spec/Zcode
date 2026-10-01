// ============================================================
// auth_failed 隔离的可运行检查（2026-10-01 拍板：改自动化、解除阻塞）。
// 钉死的规矩：**炸一个、不连坐**——一位员工的会话鉴权失败（两棒都失败走完
// 自动重试链），老板拿到的是诚实的失败回执（带结构化根因 + retried 章），
// 而另一位员工在同一老板会话里的工单照常交付成功回执，队列不被堵死。
// 驱动**真实** scheduleWorkOrderReceiptRelay（与 agent-dispatch-receipts.test.ts
// 同一套假件纪律）。每条测试结尾必须退订：默认看门狗 30 分钟，不退订吊死进程。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/auth-failure-isolation.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { SessionEventType } from "@zcode/contracts";
import { scheduleWorkOrderReceiptRelay } from "../src/zcode-protocol/agent-dispatch-receipts.js";

const BOSS_SESSION = "sess-boss";
const WORKER_A = "sess-worker-a";
const WORKER_B = "sess-worker-b";

/** 员工侧：Set 迭代分发 + 零 await 的 enqueue（微任务签名与 core 同形）。 */
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
    async enqueueAgentWorkOrder() {
      return { delivery: "started" as const, inputId: "irrelevant", workOrderId: "wo" };
    },
    getActiveTurnInfo() {
      return undefined;
    },
    hasActiveOrQueuedTurnWork() {
      return false;
    },
  };
}

interface RecordedReceipt {
  workOrderId: string;
  status: string;
  retried: boolean;
  failureCode?: string;
}

function makeBossRuntime() {
  const receipts: RecordedReceipt[] = [];
  return {
    receipts,
    enqueueAgentWorkOrderReceipt(input: {
      workOrderId: string;
      outcome: { status: string; retried?: boolean; failureCode?: string };
    }) {
      receipts.push({
        workOrderId: input.workOrderId,
        status: input.outcome.status,
        retried: input.outcome.retried === true,
        ...(input.outcome.failureCode ? { failureCode: input.outcome.failureCode } : {}),
      });
    },
  };
}

function harness() {
  const boss = makeBossRuntime();
  const workerA = makeWorkerRuntime();
  const workerB = makeWorkerRuntime();
  const context = {
    logger: undefined,
    sessions: new Map([
      [BOSS_SESSION, { app: { runtime: boss, sessionId: BOSS_SESSION }, traceContext: undefined }],
    ]),
  };
  const deps = { activateSessionRecord: async () => ({}) as never };
  const inputId = (workOrderId: string) => `workorder-${workOrderId}`;
  const unsubscribeA = scheduleWorkOrderReceiptRelay(context as never, deps, {
    targetRecord: {
      app: { runtime: workerA, sessionId: WORKER_A },
      traceContext: undefined,
    } as never,
    inputId: inputId("wo-a"),
    envelope: { workOrderId: "wo-a", fromSessionId: BOSS_SESSION } as never,
    agentName: "worker-a",
    watchdogMs: 5,
  });
  const unsubscribeB = scheduleWorkOrderReceiptRelay(context as never, deps, {
    targetRecord: {
      app: { runtime: workerB, sessionId: WORKER_B },
      traceContext: undefined,
    } as never,
    inputId: inputId("wo-b"),
    envelope: { workOrderId: "wo-b", fromSessionId: BOSS_SESSION } as never,
    agentName: "worker-b",
    watchdogMs: 5,
  });
  const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
  return { boss, workerA, workerB, unsubscribeA, unsubscribeB, tick, inputId };
}

test("鉴权失败两棒走完：老板拿到诚实失败回执（根因+retried 章），另一位员工照常交活", async () => {
  const { boss, workerA, workerB, unsubscribeA, unsubscribeB, tick, inputId } = harness();
  // 员工 A 第一棒鉴权失败。
  await workerA.notifyEventSinks({
    type: SessionEventType.TurnError,
    payload: {
      inputId: inputId("wo-a"),
      error: { message: "Provider rejected the request: invalid api key", code: "auth_failed" },
    },
  });
  await tick(); // 自动重试推迟到宏任务
  // 员工 B 在 A 重试期间完成自己的活——A 的失败不连坐 B。
  await workerB.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: inputId("wo-b"), resultType: "success", response: "B 干完了" },
  });
  // 员工 A 第二棒（自动重试）仍然鉴权失败 → 诚实终态。
  await workerA.notifyEventSinks({
    type: SessionEventType.TurnError,
    payload: {
      inputId: inputId("wo-a"),
      error: { message: "Provider rejected the request: invalid api key", code: "auth_failed" },
    },
  });
  await tick();
  await tick();

  const receiptA = boss.receipts.find((receipt) => receipt.workOrderId === "wo-a");
  const receiptB = boss.receipts.find((receipt) => receipt.workOrderId === "wo-b");
  assert.ok(receiptA, "A 的失败回执必须到达老板");
  assert.equal(receiptA?.status, "failed");
  assert.equal(receiptA?.retried, true, "两棒都失败要盖 retried 章");
  assert.equal(receiptA?.failureCode, "auth_failed", "结构化根因让失败卡讲大白话");
  assert.ok(receiptB, "B 的成功回执必须照常到达（不连坐）");
  assert.equal(receiptB?.status, "completed");
  unsubscribeA();
  unsubscribeB();
});

test("鉴权失败第一棒就恢复：只投 completed、不盖章、不惊动别的员工", async () => {
  const { boss, workerA, workerB, unsubscribeA, unsubscribeB, tick, inputId } = harness();
  await workerA.notifyEventSinks({
    type: SessionEventType.TurnError,
    payload: {
      inputId: inputId("wo-a"),
      error: { message: "transient auth blip", code: "auth_failed" },
    },
  });
  await tick();
  await workerA.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: inputId("wo-a"), resultType: "success", response: "重试后成了" },
  });
  await workerB.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: inputId("wo-b"), resultType: "success", response: "B 也成了" },
  });
  await tick();
  assert.deepEqual(
    boss.receipts.map((receipt) => [receipt.workOrderId, receipt.status]),
    [
      ["wo-a", "completed"],
      ["wo-b", "completed"],
    ],
  );
  unsubscribeA();
  unsubscribeB();
});
