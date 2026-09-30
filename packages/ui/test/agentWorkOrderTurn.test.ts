import assert from "node:assert/strict";
import test from "node:test";
import type { TurnHeaderRow } from "@zcode/shared/zcode-protocol-v4";
import {
  AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE,
  describeReceiptFailure,
  resolveAgentWorkOrderMeta,
  resolveAgentWorkOrderReceiptMeta,
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

// ── 失败回执结构化线索（2026-10-01 员工可靠性批）─────────────────────

const FAILED_META = {
  backgroundSource: "agent_work_order_receipt" as const,
  workId: "wo-fail-1",
  title: "code-builder 的工单未完成",
  task: "把登录页的无障碍问题修一遍",
  failureCode: "invalid_model_request",
  failureModelId: "glm-5.3-flash-local",
  failureReason: "Provider rejected the model request.",
  retried: true,
};

test("resolveAgentWorkOrderReceiptMeta：失败线索原样透出", () => {
  const header = turnHeaderRow({
    origin: "backgroundResult",
    originMeta: FAILED_META,
  });
  const meta = resolveAgentWorkOrderReceiptMeta(header);
  assert.ok(meta);
  assert.equal(meta.task, FAILED_META.task);
  assert.equal(meta.failureCode, "invalid_model_request");
  assert.equal(meta.failureModelId, "glm-5.3-flash-local");
  assert.equal(meta.retried, true);
});

test("describeReceiptFailure：invalid_model_request 讲人话并点名模型，原根因进详情", () => {
  const presentation = describeReceiptFailure(FAILED_META);
  assert.ok(presentation);
  assert.equal(presentation.messageId, "chat.receipt.failure.invalidModelRequest.named");
  assert.deepEqual(presentation.values, { modelId: "glm-5.3-flash-local" });
  assert.equal(presentation.detail, "Provider rejected the model request.");
  assert.equal(presentation.retried, true);
});

test("describeReceiptFailure：没带模型名时走无名短语；未知分类码走通用句式带原根因", () => {
  const unnamed = describeReceiptFailure({
    failureCode: "invalid_model_request",
    failureReason: "Provider rejected the model request.",
  });
  assert.equal(unnamed?.messageId, "chat.receipt.failure.invalidModelRequest");
  assert.equal(unnamed?.values, undefined);
  const generic = describeReceiptFailure({
    failureCode: "UNKNOWN_ERROR",
    failureReason: "provider timeout",
  });
  assert.equal(generic?.messageId, "chat.receipt.failure.generic");
  assert.deepEqual(generic?.values, { reason: "provider timeout" });
  assert.equal(generic?.detail, undefined);
});

test("describeReceiptFailure：结构化线索全缺席（旧 CLI）返回 undefined，不拿空话占位", () => {
  assert.equal(describeReceiptFailure({}), undefined);
});

test("describeReceiptFailure：只有 retried 章（无根因）走无插值句式，不印 {reason}", () => {
  const presentation = describeReceiptFailure({ retried: true });
  assert.equal(presentation?.messageId, "chat.receipt.failure.genericNoReason");
  assert.equal(presentation?.values, undefined);
});
