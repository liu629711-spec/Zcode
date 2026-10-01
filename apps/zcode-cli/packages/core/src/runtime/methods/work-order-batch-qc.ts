// ============================================================
// 批次质检（纪律协议批）：批次全部收口后发起方会话的自动验货轮。
// bootstrap 触发侧（agent-dispatch-receipts）在最后一张回执销账后经公共
// AgentRuntime 接口开轮；本文件只负责排队、闸门台账行、落库 synthetic notice
// 与按质检身份成轮。与回执轮同型（排队 + 独立成轮，绝不打断进行中回合）。
// ============================================================

import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { TraceContext } from "../deps.js";
import type { BackgroundResultOriginMeta } from "@zcode/contracts";
import { uuidv7 } from "@zcode/shared";
import {
  createRuntimeCommandId,
  type WorkOrderBatchQcRuntimeCommand,
} from "../command-queue.js";
import { runtimeInputMetadata } from "../../agent/runtime-input-presentation.js";
import {
  buildBatchQcEnvelopeText,
  buildBatchQcTitle,
  type BatchQcOrder,
} from "../../subagent/work-order.js";
import type { AgentRuntimeInternal } from "../internal.js";
import { isStaleBranchRuntimeCommand } from "./runtime-command-generation.js";
import { beginForegroundExecution, finishForegroundExecution } from "./runtime-command-queue.js";

/** 质检闸门台账行的确定性 id 前缀：触发侧靠「行存在（任意状态）」实现每批只质检一次。 */
export const BATCH_QC_LEDGER_ID_PREFIX = "agentWorkOrderBatchQc:";

export function batchQcLedgerId(batchId: string): string {
  return `${BATCH_QC_LEDGER_ID_PREFIX}${batchId}`;
}

export interface EnqueueAgentWorkOrderBatchQcInput {
  batchId: string;
  batchTitle?: string;
  orders: readonly BatchQcOrder[];
  traceContext?: TraceContext;
}

/**
 * 公共入口（AgentRuntime 面上）：为一批已收口的工单开一轮自动质检。
 * 闸门（每批只此一次）由确定性台账行承担：本函数落 `agentWorkOrderBatchQc:<batchId>`，
 * 触发侧先查行（任意状态）再触发——进程内触发侧串行 + 行持久化，并发与跨重启都堵住。
 * 崩溃窗口（闸门已落、轮未跑）质检丢失不补跑：与回执同一档 best-effort，权威事实
 * 在各目标会话的留档里。
 */
export function enqueueAgentWorkOrderBatchQc(
  this: AgentRuntimeInternal,
  input: EnqueueAgentWorkOrderBatchQcInput,
): void {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  if (this.shuttingDown) {
    // teardown 期间不能再启动模型轮次；质检如实丢弃（各单回执仍已送达）。
    this.logger?.info("Dropped agent work order batch QC during session shutdown", {
      ...traceContextToLogContext(traceContext),
      batchId: input.batchId,
      event: "agent_work_order_batch_qc.shutdown_dropped",
      module: "core.runtime",
      sessionId: this.sessionId,
    });
    return;
  }
  const text = buildBatchQcEnvelopeText({
    batchId: input.batchId,
    ...(input.batchTitle === undefined ? {} : { batchTitle: input.batchTitle }),
    orders: input.orders,
  });
  const originMeta: BackgroundResultOriginMeta = {
    backgroundSource: "agent_work_order_batch_qc",
    workId: input.batchId,
    title: buildBatchQcTitle(input.batchTitle),
    batchId: input.batchId,
    ...(input.batchTitle === undefined ? {} : { batchTitle: input.batchTitle }),
  };
  const command: WorkOrderBatchQcRuntimeCommand = {
    branchGeneration: this.branchGeneration,
    createdAt: new Date(),
    id: createRuntimeCommandId(),
    mode: "work-order-batch-qc",
    originMeta,
    orders: input.orders,
    batchId: input.batchId,
    ...(input.batchTitle === undefined ? {} : { batchTitle: input.batchTitle }),
    priority: "next",
    source: "agent_work_order_batch_qc",
    text,
    traceContext,
  };
  this.enqueueRuntimeCommand(command);
  // 闸门台账行（durable 痕迹）：确定性 id，upsert 幂等。resume 清扫会把残余
  // admitted 行收口为 discarded，不影响触发侧的「行存在即跳过」判定。
  const admission = this.sessionStore?.saveSessionInput?.({
    id: batchQcLedgerId(input.batchId),
    sessionID: this.sessionId,
    kind: "agentWorkOrderBatchQc",
    delivery: "queue",
    payload: {
      text,
      batchId: input.batchId,
      ...(input.batchTitle === undefined ? {} : { batchTitle: input.batchTitle }),
      orders: [...input.orders],
    },
  });
  if (admission) {
    void this.trackResidencyBlockingWork(admission).catch((error) => {
      this.logger?.warn("Failed to admit agent work order batch QC to ledger", {
        ...traceContextToLogContext(traceContext),
        batchId: input.batchId,
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "session_input.admit_failed",
        module: "core.runtime",
        status: "failed",
      });
    });
  }
  this.logger?.info("Agent work order batch QC enqueued", {
    ...traceContextToLogContext(traceContext),
    batchId: input.batchId,
    event: "agent_work_order_batch_qc.enqueued",
    module: "core.runtime",
    orderCount: input.orders.length,
    sessionId: this.sessionId,
  });
}

/**
 * 质检轮：落库 synthetic notice（provider 可见、UI 不画气泡；轮头卡走
 * backgroundResult 链）→ 以质检身份独立成轮。不带 workorder- 前缀身份、
 * 不带 denylist——打回重派靠发起方自己的 AgentDispatch，嵌套上限只限工单轮。
 */
export async function runWorkOrderBatchQcCommand(
  this: AgentRuntimeInternal,
  command: WorkOrderBatchQcRuntimeCommand,
): Promise<void> {
  if (isStaleBranchRuntimeCommand(this, command)) return;
  const foregroundExecution = beginForegroundExecution.call(this, command);
  try {
    const messageID = createMessageId();
    // 内存历史与持久化同一份原文（<batch-qc> 信封 + 验货要求）；执行期投影再补
    // 「非用户权威」框架（provider-entry-origins 的 agent_work_order_batch_qc 分支）。
    this.messageHistory.addUser(command.text, runtimeInputMetadata("agent_work_order_batch_qc"));
    await this.persistSyntheticUserNoticeForSession({
      messageID,
      metadata: {
        batchId: command.batchId,
        ...(command.batchTitle === undefined ? {} : { batchTitle: command.batchTitle }),
        inputPresentation: "agent_work_order_batch_qc",
        originMeta: command.originMeta,
        visibility: "model-only",
      },
      sessionId: this.sessionId,
      source: "agent_work_order_batch_qc",
      text: command.text,
      traceContext: command.traceContext,
      visibility: "model-only",
    });
    await this.sessionStore
      ?.markSessionInputPromoted?.({
        id: String(command.id),
        sessionID: this.sessionId,
        promotedMessageID: messageID,
      })
      .catch((error) => {
        this.logger?.warn("Failed to mark agent work order batch QC promoted", {
          ...traceContextToLogContext(command.traceContext),
          batchId: command.batchId,
          commandId: command.id,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.promote_mark_failed",
          module: "core.runtime",
          status: "failed",
        });
      });
    this.logger?.info("Agent work order batch QC turn started", {
      ...traceContextToLogContext(command.traceContext),
      batchId: command.batchId,
      commandId: command.id,
      event: "agent_work_order_batch_qc.turn_started",
      messageId: messageID,
      module: "core.runtime",
    });
    await this.executeTurnCommand(command.text, undefined, {
      abortSignal: foregroundExecution.controller.signal,
      // 质检轮不使用 workorder- 前缀（那是工单轮 denylist 的身份信号）。
      inputId: uuidv7(),
      inputPresentation: "agent_work_order_batch_qc",
      inputSource: "agent_work_order_batch_qc",
      inputVisibility: "model-only",
      originMeta: command.originMeta,
      backgroundSource: "agent_work_order_batch_qc",
      recordedInputMessageId: messageID,
      skipInputRecord: true,
      skipUserPromptSubmitHooks: true,
      traceContext: command.traceContext,
    });
  } catch (error) {
    this.logger?.warn("Agent work order batch QC turn failed", {
      ...traceContextToLogContext(command.traceContext),
      batchId: command.batchId,
      commandId: command.id,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agent_work_order_batch_qc.turn_failed",
      module: "core.runtime",
    });
  } finally {
    finishForegroundExecution.call(this, foregroundExecution);
  }
}
