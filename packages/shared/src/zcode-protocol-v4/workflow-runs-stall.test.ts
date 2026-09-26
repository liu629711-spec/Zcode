// ============================================================
// run 级停滞（workflow-runs-stall.ts）的可运行检查（shared 包的第一个测试先例）
// ============================================================
// resolveStall 是纯函数：构造最小 WorkflowRunState 直接喂——只要过得了它的类型即可，
// 不追求 zod 全 schema（必填字段见 workflowRunSchema：runId/status/usage/actors/
// nodes/lastEventSequence）。置键那半边（run-stalled case 的活 run 守卫）在
// workflow-runs-reducer.ts 里，走 reduceWorkflowRunsState 全路径驱动。
//
// 本文件参与 tsc -b（shared 的 tsconfig 未排除 *.test.ts），import 一律 .js 后缀。
//
// 运行：npx tsx --test packages/shared/src/zcode-protocol-v4/workflow-runs-stall.test.ts

import assert from "node:assert/strict";
import { test } from "node:test";
import { reduceWorkflowRunsState } from "./workflow-runs-reducer.js";
import { resolveStall, STALL_ENDING_EVENT_TYPES } from "./workflow-runs-stall.js";
import type { WorkflowRunState, WorkflowRunsState } from "./workflow-runs.js";

/** 最小合法 run（类型上过得去 resolveStall / WorkflowRunsState 即可）。 */
function minimalRun(status: WorkflowRunState["status"], stalled = false): WorkflowRunState {
  return {
    runId: "r1",
    status,
    usage: { spentTokens: 0, nodesUsed: 0 },
    actors: [],
    nodes: [],
    lastEventSequence: 0,
    ...(stalled ? { stalled: true as const } : {}),
  };
}

test("摘键白名单逐成员：stalled run 收到任一成员即被摘键（含新加的 run-launched）", () => {
  for (const eventType of STALL_ENDING_EVENT_TYPES) {
    const next = resolveStall(minimalRun("running", true), eventType);
    assert.ok(!("stalled" in next), `${eventType} 到达必须摘掉 stalled 键`);
  }
});

test("run-stalled 只对活 run（pending/running）置键；终态 run 原样返回（无 delta）", () => {
  for (const status of ["pending", "running"] as const) {
    const prior: WorkflowRunsState = { revision: 0, runs: [minimalRun(status)] };
    const next = reduceWorkflowRunsState(prior, { runId: "r1", eventType: "run-stalled" });
    assert.equal(next?.runs[0]?.stalled, true, `status=${status} 必须置上 stalled`);
  }
  for (const status of ["completed", "errored", "stopped"] as const) {
    const prior: WorkflowRunsState = { revision: 0, runs: [minimalRun(status)] };
    const next = reduceWorkflowRunsState(prior, { runId: "r1", eventType: "run-stalled" });
    assert.equal(next, null, `终态 status=${status} 必须被忽略（归约返回 null）`);
  }
});

test("非摘键事件对无 stalled 键的 run 原样返回（引用不变）", () => {
  for (const eventType of ["node-waiting", "concurrency-changed", "run-stalled"]) {
    const base = minimalRun("running");
    assert.equal(resolveStall(base, eventType), base, `${eventType} 不得产生新引用`);
  }
  // 负例才是「node-waiting 刻意不在白名单」（等槽位/退避不是运动证据）的真正钉子：
  // 带 stalled 键的 run 收到它必须原样通过——它若进了白名单，键被摘即返回新引用，此处即红。
  // （上面的循环跑在无 stalled 键的 run 上，resolveStall 在 stalled!==true 处短路，
  // 区分不了成员与否，守不住这个回归方向。）
  const stalled = minimalRun("running", true);
  assert.equal(resolveStall(stalled, "node-waiting"), stalled);
});

test("重复投递同一条 run-stalled 幂等：第二次无 delta，键仍在", () => {
  const first = reduceWorkflowRunsState(undefined, { runId: "r1", eventType: "run-stalled", sequence: 1 });
  assert.equal(first?.runs[0]?.stalled, true);
  const second = reduceWorkflowRunsState(first, { runId: "r1", eventType: "run-stalled", sequence: 1 });
  assert.equal(second, null, "同一条事件重放不得产生 delta");
  assert.equal(first?.runs[0]?.stalled, true, "重放之后键仍在");
  // 纯函数半边：run-stalled 自己不是摘键成员，已置键的 run 原样通过。
  const stalledRun = minimalRun("running", true);
  assert.equal(resolveStall(stalledRun, "run-stalled"), stalledRun);
});

test("序纪律回归：run-stalled 置键 → run-launched 到达 → 键被摘", () => {
  const stalled = reduceWorkflowRunsState(undefined, { runId: "r1", eventType: "run-stalled", sequence: 1 });
  assert.equal(stalled?.runs[0]?.stalled, true);
  const launched = reduceWorkflowRunsState(stalled, {
    runId: "r1",
    eventType: "run-launched",
    sequence: 2,
    payload: { phaseNames: ["alpha"] },
  });
  assert.ok(launched, "run-launched 必须产生 delta（摘键本身就是变化）");
  assert.ok(!("stalled" in launched.runs[0]!), "run-launched 必须摘掉 stalled 键");
  assert.deepEqual(launched.runs[0]!.phaseNames, ["alpha"]);
});
