// ============================================================
// 派单频控/成本熔断（audit A 盲区 2026-10-05）的可运行检查。
// 驱动真实 assertDispatchBudget（端口 dispatch() 内的同一条守卫）：
//  1. 在飞（admitted 派单行）≥ 16 → guard.agentWorkOrderRate；
//  2. 10 分钟频窗内派单 ≥ 20 → guard.agentWorkOrderRate；
//  3. 全局近窗花费 ≥ 1 亿 token → guard.agentWorkOrderCost；
//  4. 台账/用量查询炸了 → fail-open（不拦派单）；
//  5. 干净台账 → 放行。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/dispatch-budget.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { assertDispatchBudget, AGENT_WORK_ORDER_GUARDS } from "../src/zcode-protocol/agent-dispatch-port.js";

interface FakeRow {
  kind: string;
  status: string;
  time: { created: number; updated: number };
}

function makeStore(input: {
  rows?: FakeRow[];
  totalTokens?: number;
  listSessionInputsError?: Error;
  queryAppUsageError?: Error;
}) {
  return {
    async listSessionInputs() {
      if (input.listSessionInputsError) throw input.listSessionInputsError;
      return input.rows ?? [];
    },
    async queryAppUsage() {
      if (input.queryAppUsageError) throw input.queryAppUsageError;
      return { totals: { totalTokens: input.totalTokens ?? 0 } };
    },
  } as never;
}

const CONTEXT = { logger: undefined } as never;
const SESSION = "sess-boss" as never;

function dispatchRow(status: string, created: number): FakeRow {
  return { kind: "agentWorkOrderDispatch", status, time: { created, updated: created } };
}

test("频控：在飞派单行到上限 → guard.agentWorkOrderRate", async () => {
  const now = Date.now();
  const rows = Array.from({ length: 16 }, (_, i) => dispatchRow("admitted", now - i * 1000));
  await assert.rejects(
    assertDispatchBudget(CONTEXT, makeStore({ rows }), SESSION as unknown as string),
    (error: Error & { reasonCode?: string }) => error.reasonCode === AGENT_WORK_ORDER_GUARDS.rate,
  );
});

test("频控：10 分钟频窗内派满 → guard.agentWorkOrderRate（在飞少但派得勤也拦）", async () => {
  const now = Date.now();
  // 15 张已收口（discarded）但在窗内 + 1 张在飞：在飞未超，频窗超。
  const rows = [
    ...Array.from({ length: 19 }, (_, i) => dispatchRow("discarded", now - i * 1000)),
    dispatchRow("admitted", now - 30_000),
  ];
  await assert.rejects(
    assertDispatchBudget(CONTEXT, makeStore({ rows }), SESSION as unknown as string),
    (error: Error & { reasonCode?: string }) => error.reasonCode === AGENT_WORK_ORDER_GUARDS.rate,
  );
});

test("频窗外的旧派单行不计入频窗", async () => {
  const now = Date.now();
  const rows = Array.from({ length: 25 }, (_, i) =>
    dispatchRow("discarded", now - (11 * 60_000 + i * 1000)),
  );
  await assertDispatchBudget(CONTEXT, makeStore({ rows }), SESSION as unknown as string);
});

test("成本熔断：全局近窗花费超线 → guard.agentWorkOrderCost", async () => {
  await assert.rejects(
    assertDispatchBudget(
      CONTEXT,
      makeStore({ totalTokens: 100_000_000 }),
      SESSION as unknown as string,
    ),
    (error: Error & { reasonCode?: string }) => error.reasonCode === AGENT_WORK_ORDER_GUARDS.cost,
  );
});

test("台账/用量查询炸了 → fail-open 放行", async () => {
  await assertDispatchBudget(
    CONTEXT,
    makeStore({
      listSessionInputsError: new Error("db boom"),
      queryAppUsageError: new Error("db boom"),
    }),
    SESSION as unknown as string,
  );
});

test("干净台账 → 放行", async () => {
  await assertDispatchBudget(
    CONTEXT,
    makeStore({
      rows: [dispatchRow("admitted", Date.now()), dispatchRow("discarded", Date.now() - 1)],
      totalTokens: 1234,
    }),
    SESSION as unknown as string,
  );
});
