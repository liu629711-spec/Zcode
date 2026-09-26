// ============================================================
// run 级停滞（`run.stalled`）的吸收与解除（workflowRuns 归约的一条规则）
// ============================================================
// 纯函数。观察的生产方是 bootstrap 的 driver：它的 RunStallClock 在「整条 run 连续
// 一段阈值没有一次**成功**的模型请求、且本段至少排定过一次重试」时发一条 `run-stalled`
// （每段恰好一次，下一次成功才重新上膛）。这里只管两件事：事件到达时把键置上（活 run），
// 以及运动/终态证据到达时把键摘掉——置真在 reducer 的 `run-stalled` case，摘键在这里。
//
// 摘键而不是置 false：协议只有「键在场 / 键缺席」两态（与 pendingQuestions 同一条惯例）。

import type { WorkflowRunState } from "./workflow-runs.js";

/**
 * 结束一段停滞的事件：**运动证据**。清单与 bootstrap 的 roster 事件索引
 * （dynamic-workflow-run-roster-events.ts 的 PROGRESS_EVENT_TYPES）同一条纪律——
 * 脚本或某个 ask 往前走了，停滞观察即告结束。
 *
 * `node-waiting` 刻意**不在**其中：等槽位、等退避正是停滞的常态形态，它不证明任何人在动。
 * `run-started`（新的一世出生）与 `run-settled`（终态没有还在等的读者）也摘键——
 * 停滞观察只属于活着的 run。
 */
const STALL_ENDING_EVENT_TYPES: ReadonlySet<string> = new Set([
  "node-queued",
  "node-dispatched",
  "node-executing",
  "node-settled",
  "node-progress",
  "phase-entered",
  "log",
  "report",
  "artifact-published",
  "usage-updated",
  "run-started",
  "run-settled",
]);

/** 摘掉 `stalled` 键（本来就缺席时原样返回，引用不变）。 */
function withoutStall(run: WorkflowRunState): WorkflowRunState {
  if (run.stalled !== true) return run;
  const { stalled: _ended, ...rest } = run;
  return rest;
}

/** 一条运动/终态事件到达时，先结掉上一段停滞，再进各事件的归约。 */
export function resolveStall(run: WorkflowRunState, eventType: string): WorkflowRunState {
  return run.stalled === true && STALL_ENDING_EVENT_TYPES.has(eventType)
    ? withoutStall(run)
    : run;
}
