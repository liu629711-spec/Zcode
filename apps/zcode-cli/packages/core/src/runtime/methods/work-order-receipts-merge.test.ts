// ============================================================
// 回执洪泛合并（audit C-P2，2026-10-05 老板拍板）的可运行检查：
//  1. buildMergedReceiptOriginMeta：单张=原样返回（与既有轮头逐字节同形）；
//     多张=顶层沿用首张 + receipts 逐张带对账身份；
//  2. 收集窗：notRunnableBefore 未到期的回执命令不被出队（含被普通命令
//     「路过」），到期后正常出队；
//  3. 出队顺序：窗内回执不阻塞其他高优先命令。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/runtime/methods/work-order-receipts-merge.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { AgentWorkOrderEnvelope, BackgroundResultOriginMeta } from "@zcode/contracts";
import {
  createRuntimeCommandQueue,
  type WorkOrderReceiptRuntimeCommand,
} from "../command-queue.js";
import { buildMergedReceiptOriginMeta } from "./work-order-receipts.js";

const BRANCH = 0;
const TRACE = {} as WorkOrderReceiptRuntimeCommand["traceContext"];

function envelope(workOrderId: string): AgentWorkOrderEnvelope {
  return {
    workOrderId,
    fromAgentName: "boss",
    fromSessionId: "sess-boss",
    task: `任务 ${workOrderId}`,
  };
}

function receiptMeta(workOrderId: string, status: "completed" | "failed" = "completed"): BackgroundResultOriginMeta {
  return {
    backgroundSource: "agent_work_order_receipt",
    workId: workOrderId,
    title: `${workOrderId} 交活`,
    receiptStatus: status,
  };
}

function makeCommand(
  workOrderId: string,
  overrides: Partial<WorkOrderReceiptRuntimeCommand> = {},
): WorkOrderReceiptRuntimeCommand {
  return {
    branchGeneration: BRANCH,
    createdAt: new Date(),
    envelope: envelope(workOrderId),
    id: `cmd-${workOrderId}` as WorkOrderReceiptRuntimeCommand["id"],
    mode: "work-order-receipt",
    mergedReceipts: [],
    originMeta: receiptMeta(workOrderId),
    outcome: { status: "completed", response: `答案 ${workOrderId}` },
    priority: "next",
    source: "agent_work_order_receipt",
    targetAgentName: "code-plus",
    targetSessionId: "sess-worker",
    text: `<work-order-receipt id="${workOrderId}">答案</work-order-receipt>`,
    traceContext: TRACE,
    workOrderId,
    ...overrides,
  };
}

test("合并元数据：单张回执原样返回自己的 originMeta（不挂数组，与既有轮头同形）", () => {
  const command = makeCommand("wo-1");
  assert.equal(buildMergedReceiptOriginMeta([command]), command.originMeta);
});

test("合并元数据：多张回执=顶层沿用首张 + receipts 逐张带身份与失败线索", () => {
  const first = makeCommand("wo-1");
  const failed = makeCommand("wo-2", {
    originMeta: {
      ...receiptMeta("wo-2", "failed"),
      task: "原任务原文",
      failureCode: "invalid_model_request",
      failureModelId: "m-x",
      failureReason: "boom",
      retried: true,
      agentId: "agent-2",
      batchId: "batch-1",
      batchTitle: "登录页改造",
    },
    outcome: { status: "failed", reason: "boom", retried: true },
  });
  const merged = buildMergedReceiptOriginMeta([first, failed]);
  assert.equal(merged.workId, "wo-1", "顶层沿用首张");
  assert.equal(merged.receiptStatus, "completed");
  assert.equal(merged.receipts?.length, 2);
  assert.deepEqual(
    merged.receipts?.map((item) => [item.workId, item.receiptStatus]),
    [
      ["wo-1", "completed"],
      ["wo-2", "failed"],
    ],
  );
  const second = merged.receipts?.[1];
  assert.equal(second?.task, "原任务原文");
  assert.equal(second?.failureCode, "invalid_model_request");
  assert.equal(second?.agentId, "agent-2");
  assert.equal(second?.batchId, "batch-1");
  assert.equal(second?.retried, true);
});

test("收集窗：未到期的回执命令不出队，其他命令照常先走", () => {
  const queue = createRuntimeCommandQueue();
  const held = makeCommand("wo-1", { notRunnableBefore: new Date(Date.now() + 60_000) });
  const plain = makeCommand("wo-2");
  queue.enqueue(held);
  queue.enqueue(plain);
  // 同优先级按先后；held 被窗挡住，plain 必须先出。
  assert.equal(queue.dequeueNextBatch()[0]?.workOrderId, "wo-2");
  assert.equal(queue.dequeueNextBatch()[0]?.workOrderId, undefined, "窗内的不出队");
  assert.equal(queue.hasPending(), true, "窗内命令仍在队列");
});

test("收集窗：到期后正常出队", () => {
  const queue = createRuntimeCommandQueue();
  const held = makeCommand("wo-1", { notRunnableBefore: new Date(Date.now() - 1) });
  queue.enqueue(held);
  assert.equal(queue.dequeueNextBatch()[0]?.workOrderId, "wo-1");
});
