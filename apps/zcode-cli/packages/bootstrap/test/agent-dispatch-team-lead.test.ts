// ============================================================
// 组长制（2026-10-12）的可运行检查。
// 驱动真实端口/回执链（council-controls.test 与 agent-dispatch-receipts.test 同款
// 桩纪律），只盖三件事：
//  1. 端口守卫代次化：gen-1 × 组长的工单轮放行转派（信封 gen=2 + 父单号）；
//     非组长 / gen-2 轮内照旧 guard.agentWorkOrderNested 拒绝；
//  2. 回执挂起：父单轮 completed 而子单还有 admitted 的 → 老板零回执；
//     汇总轮（agent-work-order-consolidate:<父单>）落终态 → 老板收到带回答的 completed；
//  3. 汇总超时：宽限到点合成失败回执，不让老板的单永远"进行中"；
//  4. 子单销账钩子：gen-2 回执投递后回调父单 runtime 的汇总自查（对号父单 id）。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/agent-dispatch-team-lead.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { SessionEventType, WORK_ORDER_INPUT_ID_PREFIX } from "@zcode/contracts";
import { createProtocolAgentDispatchPort } from "../src/zcode-protocol/agent-dispatch-port.js";
import {
  scheduleWorkOrderConsolidationRelay,
  scheduleWorkOrderReceiptRelay,
} from "../src/zcode-protocol/agent-dispatch-receipts.js";

const tick = (ms = 0) => new Promise((resolve) => setTimeout(resolve, ms));
const LEAD_SESSION = "sess-lead";
const BOSS_SESSION = "sess-boss";

// ── 端口守卫代次化 ────────────────────────────────────────────────

interface PortHarnessOptions {
  activeInputId?: string;
  leadPersona?: boolean;
  admissionGeneration?: number;
}

async function runPortDispatch(options: PortHarnessOptions) {
  const admittedEnvelopes: Array<Record<string, unknown>> = [];
  const workerSinks = new Set<{ onSessionEvent: (event: never) => void }>();
  const workerRuntime = {
    // 回执链的落点：gen-2 子单终态事件由测试手动派发，借它把测试进程从看门狗里摘出来。
    subscribeEvents(sub: { onSessionEvent: (event: never) => void }) {
      const sink = { onSessionEvent: sub.onSessionEvent };
      workerSinks.add(sink);
      return () => {
        workerSinks.delete(sink);
      };
    },
    notify(type: SessionEventType, workOrderId: string) {
      const payload =
        type === SessionEventType.TurnComplete
          ? { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}${workOrderId}`, resultType: "success", response: "子活干完" }
          : { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}${workOrderId}`, error: { message: "boom" } };
      return Promise.all(
        [...workerSinks].map((sink) => sink.onSessionEvent({ type, payload } as never)),
      );
    },
    async enqueueAgentWorkOrderReceipt() {},
    async maybeEnqueueAgentWorkOrderConsolidation() {},
    async enqueueAgentWorkOrder(input: { envelope: Record<string, unknown>; inputId?: string }) {
      admittedEnvelopes.push(input.envelope);
      const workOrderId = (input.envelope as { workOrderId: string }).workOrderId;
      return {
        delivery: "started" as const,
        // 端口调 core 时不传 inputId（core 按 workorder-<id> 自派）；桩同源派生。
        inputId: input.inputId ?? `${WORK_ORDER_INPUT_ID_PREFIX}${workOrderId}`,
        workOrderId,
      };
    },
    async ensureSessionPersistedForExternalActivity() {},
  };
  const leadRuntime = {
    // 工单轮内（activeInputId 在场）= 组长被老板工单唤醒的现场。
    getActiveTurnInfo() {
      return options.activeInputId ? { inputId: options.activeInputId } : undefined;
    },
    getProjectAgentPersona() {
      return options.leadPersona ? { name: "lead", teamLead: true } : { name: "worker" };
    },
    getAgentProfiles() {
      return [
        {
          name: "worker-a",
          description: "d",
          systemPrompt: "p",
          source: "user" as const,
        },
      ];
    },
    getSessionModelSelection() {
      return undefined;
    },
  };
  const leadRecord = {
    app: { sessionId: LEAD_SESSION, runtime: leadRuntime },
    workspace: { workspacePath: "/w" },
    traceContext: undefined,
  };
  const sessionStore = {
    // 活跃工单的代次对账（组长制批走台账，不查内存）。
    async listSessionInputs() {
      return [
        {
          kind: "agentWorkOrder",
          status: "admitted",
          payload: {
            workOrderId: "wo-1",
            envelope: {
              workOrderId: "wo-1",
              ...(options.admissionGeneration
                ? { generation: options.admissionGeneration }
                : {}),
            },
          },
        },
      ];
    },
  };
  const sessions = new Map<string, unknown>();
  const context = {
    logger: undefined,
    deps: { sessionStore },
    sessions,
  };
  let activated = 0;
  const port = createProtocolAgentDispatchPort(context as never, {
    resolveOwnSession: () => leadRecord as never,
    async createPersonaSessionRecord() {
      const record = {
        app: { sessionId: "sess-worker-2", runtime: workerRuntime },
        traceContext: undefined,
      };
      sessions.set("sess-worker-2", record);
      activated += 1;
      return { sessionId: "sess-worker-2" };
    },
    async activateSessionRecord() {
      activated += 1;
      return sessions.get("sess-worker-2") as never;
    },
  });
  let deliveryError: { reasonCode?: string; message: string } | undefined;
  try {
    await port.dispatch({
      agent: "worker-a",
      task: "子任务正文",
      newSession: true,
      sourceSessionId: LEAD_SESSION,
    } as never);
  } catch (error) {
    deliveryError = error as { reasonCode?: string; message: string };
  }
  return { admittedEnvelopes, deliveryError, workerRuntime, activatedCount: activated };
}

test("端口守卫代次化：gen-1 × 组长的工单轮放行转派，子单信封 gen=2 且带父单号", async () => {
  const { admittedEnvelopes, deliveryError, workerRuntime } = await runPortDispatch({
    activeInputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`,
    leadPersona: true,
    admissionGeneration: 1,
  });
  assert.equal(deliveryError, undefined);
  assert.equal(admittedEnvelopes.length, 1);
  assert.equal(admittedEnvelopes[0]?.generation, 2);
  assert.equal(admittedEnvelopes[0]?.parentWorkOrderId, "wo-1");
  // 收尾：给子单回执链喂一条终态（用真实铸出的 workOrderId 对号），
  // 把看门狗从测试进程里摘掉。
  const subWorkOrderId = String(admittedEnvelopes[0]?.workOrderId ?? "");
  await workerRuntime.notify(SessionEventType.TurnComplete, subWorkOrderId);
  await tick(5);
});

test("端口守卫代次化：非组长在工单轮内派单照旧拒绝", async () => {
  const { admittedEnvelopes, deliveryError } = await runPortDispatch({
    activeInputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`,
    leadPersona: false,
    admissionGeneration: 1,
  });
  assert.equal(admittedEnvelopes.length, 0);
  assert.equal(deliveryError?.reasonCode, "guard.agentWorkOrderNested");
});

test("端口守卫代次化：组长自己在 gen-2 轮内再派也被拒（嵌套上限=2）", async () => {
  const { admittedEnvelopes, deliveryError } = await runPortDispatch({
    activeInputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`,
    leadPersona: true,
    admissionGeneration: 2,
  });
  assert.equal(admittedEnvelopes.length, 0);
  assert.equal(deliveryError?.reasonCode, "guard.agentWorkOrderNested");
});

// ── 回执挂起 + 汇总轮收口 ─────────────────────────────────────────

function makeLeadRecordHarness(options: {
  subRows: Array<{ status: string }>;
  watchdogMs?: number;
}) {
  const receipts: Array<{ status: string; response?: string; reason?: string }> = [];
  const consolidationCalls: string[] = [];
  const sinks = new Set<{ onSessionEvent: (event: never) => void }>();
  const leadRuntime = {
    isWorkOrderDebriefExpected() {
      return false;
    },
    async maybeEnqueueAgentWorkOrderConsolidation(input: { workOrderId: string }) {
      consolidationCalls.push(input.workOrderId);
    },
    subscribeEvents(sub: { onSessionEvent: (event: never) => void }) {
      const sink = { onSessionEvent: sub.onSessionEvent };
      sinks.add(sink);
      return () => {
        sinks.delete(sink);
      };
    },
    async notifyEventSinks(event: { type: SessionEventType; payload: Record<string, unknown> }) {
      for (const sink of sinks) {
        await sink.onSessionEvent(event as never);
      }
    },
    async enqueueAgentWorkOrderReceipt(input: {
      outcome: { status: string; response?: string; reason?: string };
    }) {
      receipts.push({
        status: input.outcome.status,
        ...(input.outcome.response ? { response: input.outcome.response } : {}),
        ...(input.outcome.reason ? { reason: input.outcome.reason } : {}),
      });
    },
  };
  const leadRecord = {
    app: { sessionId: LEAD_SESSION, runtime: leadRuntime },
    traceContext: undefined,
  };
  const context = {
    logger: undefined,
    deps: {
      sessionStore: {
        async listSessionInputs() {
          return options.subRows.map((row) => ({
            kind: "agentWorkOrderDispatch",
            status: row.status,
            payload: {
              envelope: { parentWorkOrderId: "wo-1" },
            },
          }));
        },
      },
    },
    sessions: new Map([[LEAD_SESSION, leadRecord]]),
  };
  const deps = { activateSessionRecord: async () => leadRecord as never };
  const unsubscribe = scheduleWorkOrderReceiptRelay(context as never, deps, {
    targetRecord: leadRecord as never,
    inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`,
    envelope: { workOrderId: "wo-1", fromSessionId: BOSS_SESSION, generation: 1 } as never,
    agentName: "lead",
    ...(options.watchdogMs === undefined ? {} : { watchdogMs: options.watchdogMs }),
  });
  return { receipts, consolidationCalls, leadRuntime, unsubscribe };
}

test("回执挂起：父单轮 completed 而子单还有 admitted 的 → 老板零回执", async () => {
  const { receipts, consolidationCalls, leadRuntime, unsubscribe } = makeLeadRecordHarness({
    subRows: [{ status: "admitted" }],
    watchdogMs: 5,
  });
  await leadRuntime.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`, resultType: "success", response: "已分派" },
  });
  await tick(5);
  assert.equal(receipts.length, 0, "子单没收口，父单回执必须挂起");
  assert.equal(consolidationCalls.length, 0);
  unsubscribe();
});

test("汇总轮收口：agent-work-order-consolidate 终态把汇总回答当父单回执投给老板", async () => {
  const { receipts, consolidationCalls, leadRuntime, unsubscribe } = makeLeadRecordHarness({
    subRows: [{ status: "admitted" }],
    watchdogMs: 5,
  });
  await leadRuntime.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`, resultType: "success", response: "已分派" },
  });
  await tick(5);
  await leadRuntime.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: {
      inputId: "agent-work-order-consolidate:wo-1",
      resultType: "success",
      response: "汇总：两位组员都交了活",
    },
  });
  await tick(5);
  assert.equal(consolidationCalls.length, 0, "watcher 路径不走 runtime 自查");
  assert.deepEqual(receipts, [{ status: "completed", response: "汇总：两位组员都交了活" }]);
  unsubscribe();
});

test("汇总超时：宽限到点合成失败回执，不让老板的单永远进行中", async () => {
  const { receipts, leadRuntime, unsubscribe } = makeLeadRecordHarness({
    subRows: [{ status: "admitted" }],
    watchdogMs: 15,
  });
  await leadRuntime.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`, resultType: "success", response: "已分派" },
  });
  await new Promise((resolve) => setTimeout(resolve, 80));
  assert.equal(receipts.length, 1);
  assert.equal(receipts[0]?.status, "failed");
  assert.match(receipts[0]?.reason ?? "", /汇总/);
  unsubscribe();
});

test("没有子单的普通工单：completed 照旧直投，不挂起", async () => {
  const { receipts, leadRuntime, unsubscribe } = makeLeadRecordHarness({
    subRows: [],
    watchdogMs: 5,
  });
  await leadRuntime.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-1`, resultType: "success", response: "干完了" },
  });
  await tick(5);
  assert.deepEqual(receipts, [{ status: "completed", response: "干完了" }]);
  unsubscribe();
});

test("子单销账钩子：gen-2 回执投递后按父单号回调组长的汇总自查", async () => {
  // 直接驱动汇总 relay 的上游：scheduleWorkOrderReceiptRelay 投一张带 parentWorkOrderId
  // 的 gen-2 回执（targetRecord = 组长会话），断言 consolidation 自查被按父单号调用。
  const consolidationCalls: string[] = [];
  const sinks = new Set<{ onSessionEvent: (event: never) => void }>();
  const leadRuntime = {
    async maybeEnqueueAgentWorkOrderConsolidation(input: { workOrderId: string }) {
      consolidationCalls.push(input.workOrderId);
    },
    subscribeEvents(sub: { onSessionEvent: (event: never) => void }) {
      const sink = { onSessionEvent: sub.onSessionEvent };
      sinks.add(sink);
      return () => {
        sinks.delete(sink);
      };
    },
    async notifyEventSinks(event: { type: SessionEventType; payload: Record<string, unknown> }) {
      for (const sink of sinks) {
        await sink.onSessionEvent(event as never);
      }
    },
    async enqueueAgentWorkOrderReceipt() {},
  };
  const leadRecord = {
    app: { sessionId: LEAD_SESSION, runtime: leadRuntime },
    traceContext: undefined,
  };
  const context = {
    logger: undefined,
    deps: {},
    sessions: new Map([[LEAD_SESSION, leadRecord]]),
  };
  const deps = { activateSessionRecord: async () => leadRecord as never };
  const unsubscribe = scheduleWorkOrderReceiptRelay(context as never, deps, {
    targetRecord: leadRecord as never,
    inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-sub`,
    envelope: {
      workOrderId: "wo-sub",
      fromSessionId: LEAD_SESSION,
      generation: 2,
      parentWorkOrderId: "wo-1",
    } as never,
    agentName: "worker-a",
    watchdogMs: 5,
  });
  await leadRuntime.notifyEventSinks({
    type: SessionEventType.TurnComplete,
    payload: { inputId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-sub`, resultType: "success", response: "子活干完" },
  });
  await tick(5);
  assert.deepEqual(consolidationCalls, ["wo-1"]);
  unsubscribe();
});
