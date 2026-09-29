import assert from "node:assert/strict";
import test from "node:test";
import type { TurnHeaderRow } from "@zcode/shared/zcode-protocol-v4";
import {
  AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE,
  resolveAgentWorkOrderMeta,
} from "../src/v4/agentWorkOrderTurn.js";

// ============================================================
// D29/D5 工单轮在 UI 的认法：只认 turnHeader.origin === "agentWorkOrder" 上的
// agentWorkOrder 元数据（与 workflowLaunch 同一纪律），绝不从轮头/正文文本反推。
//
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/agentWorkOrderTurn.test.ts
// ============================================================

function turnHeaderRow(overrides: Partial<TurnHeaderRow>): TurnHeaderRow {
  return {
    rowId: 1,
    turnId: "turn-1",
    createdAt: 0,
    createdAtSeq: 0,
    kind: "turnHeader",
    origin: "userInput",
    state: "running",
    startedAt: 0,
    ...overrides,
  } as TurnHeaderRow;
}

const META = {
  workOrderId: "wo-1",
  fromAgentName: "code-plus",
  fromSessionId: "session-1",
  task: "把登录页的无障碍问题修一遍",
};

test("resolveAgentWorkOrderMeta：工单轮带元数据时原样透出", () => {
  const header = turnHeaderRow({ origin: "agentWorkOrder", agentWorkOrder: META });
  assert.deepEqual(resolveAgentWorkOrderMeta(header), META);
});

test("resolveAgentWorkOrderMeta：元数据缺席（旧 CLI/畸形帧）退回 undefined，不猜", () => {
  const header = turnHeaderRow({ origin: "agentWorkOrder" });
  assert.equal(resolveAgentWorkOrderMeta(header), undefined);
});

test("resolveAgentWorkOrderMeta：其他 origin（含 backgroundResult）不误判成工单轮", () => {
  assert.equal(resolveAgentWorkOrderMeta(turnHeaderRow({ origin: "userInput" })), undefined);
  assert.equal(resolveAgentWorkOrderMeta(turnHeaderRow({ origin: "backgroundResult" })), undefined);
  assert.equal(resolveAgentWorkOrderMeta(undefined), undefined);
});

test("回执背景来源取值与 CLI 权威词表对齐（漂移即发起方收据头消失）", () => {
  assert.equal(AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE, "agent_work_order_receipt");
});
