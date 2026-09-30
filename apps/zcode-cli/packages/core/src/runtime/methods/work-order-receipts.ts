// ============================================================
// 派单（D29/D3）回执投递：core 侧的窄公共入口与回执轮执行。
// bootstrap 协议端口在目标轮 TurnComplete/TurnError 后，经公共 AgentRuntime
// 接口把回执投回发起方会话；与 enqueueBackgroundTaskNotification 同型
// （排队 + 账本 admission），发起方忙时在回合边界独立成轮，绝不混进进行中回合。
// ============================================================

import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { TraceContext } from "../deps.js";
import type { AgentWorkOrderEnvelope, BackgroundResultOriginMeta } from "@zcode/contracts";
import { uuidv7 } from "@zcode/shared";
import {
  createRuntimeCommandId,
  type WorkOrderReceiptRuntimeCommand,
} from "../command-queue.js";
import { runtimeInputMetadata } from "../../agent/runtime-input-presentation.js";
import {
  buildWorkOrderReceiptEnvelopeText,
  buildWorkOrderReceiptTitle,
  type WorkOrderReceiptOutcome,
} from "../../subagent/work-order.js";
import type { AgentRuntimeInternal } from "../internal.js";
import { isStaleBranchRuntimeCommand } from "./runtime-command-generation.js";
import { beginForegroundExecution, finishForegroundExecution } from "./runtime-command-queue.js";

export interface EnqueueAgentWorkOrderReceiptInput {
  /** 回执指回的工单身份（与工单信封的 workOrderId 一致，供 UI/日志对账）。 */
  workOrderId: string;
  /** 交活方档案名（回执标题与信封署名）。 */
  agentName: string;
  agentId?: string;
  /** 交活方 persona 会话（信封 from-session）。 */
  targetSessionId: string;
  /** 原工单信封（谁派的单/派给谁）——回执要指回发起时的工单。 */
  envelope: AgentWorkOrderEnvelope;
  /** 目标轮终态（completed 带最终答案本体；cancelled/failed 如实带原因）。 */
  outcome: WorkOrderReceiptOutcome;
  traceContext?: TraceContext;
}

/**
 * 公共入口（AgentRuntime 面上）：把一条派单回执投进发起方会话。
 * 发起方忙 → 回执在 runtime 命令队列排队，回合边界独立成轮（绝不打断进行中回合）；
 * 崩溃后残余 admission 照既有收口语义在 resume 时落 discarded——回执是 best-effort
 * 通知，权威事实永远在目标会话的留档里。
 */
export function enqueueAgentWorkOrderReceipt(
  this: AgentRuntimeInternal,
  input: EnqueueAgentWorkOrderReceiptInput,
): void {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  if (this.shuttingDown) {
    // teardown 期间不能再启动模型轮次；回执如实丢弃（目标会话仍有完整留档）。
    this.logger?.info("Dropped agent work order receipt during session shutdown", {
      ...traceContextToLogContext(traceContext),
      event: "agent_work_order_receipt.shutdown_dropped",
      module: "core.runtime",
      sessionId: this.sessionId,
      workOrderId: input.workOrderId,
    });
    return;
  }
  const text = buildWorkOrderReceiptEnvelopeText({
    workOrderId: input.workOrderId,
    agentName: input.agentName,
    targetSessionId: input.targetSessionId,
    outcome: input.outcome,
  });
  const originMeta: BackgroundResultOriginMeta = {
    backgroundSource: "agent_work_order_receipt",
    workId: input.workOrderId,
    title: buildWorkOrderReceiptTitle(input.agentName, input.outcome.status),
    // 批次（工地卡）随回执轮头下发：发起方 UI 据它把同批回执归进一张工地卡。
    ...(input.envelope.batchId ? { batchId: input.envelope.batchId } : {}),
    ...(input.envelope.batchTitle ? { batchTitle: input.envelope.batchTitle } : {}),
  };
  const command: WorkOrderReceiptRuntimeCommand = {
    branchGeneration: this.branchGeneration,
    createdAt: new Date(),
    envelope: input.envelope,
    id: createRuntimeCommandId(),
    mode: "work-order-receipt",
    originMeta,
    outcome: input.outcome,
    priority: "next",
    source: "agent_work_order_receipt",
    targetSessionId: input.targetSessionId,
    targetAgentId: input.agentId,
    targetAgentName: input.agentName,
    text,
    traceContext,
    workOrderId: input.workOrderId,
  };
  this.enqueueRuntimeCommand(command);
  // 账本 admission（durable 痕迹）：runtime 命令队列是纯内存的，崩溃后由 resume 统一收口。
  const admission = this.sessionStore?.saveSessionInput?.({
    id: String(command.id),
    sessionID: this.sessionId,
    kind: "agentWorkOrderReceipt",
    delivery: "queue",
    payload: {
      text,
      workOrderId: input.workOrderId,
      outcome: input.outcome,
      envelope: input.envelope,
    },
  });
  if (admission) {
    void this.trackResidencyBlockingWork(admission).catch((error) => {
      this.logger?.warn("Failed to admit agent work order receipt to ledger", {
        ...traceContextToLogContext(traceContext),
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "session_input.admit_failed",
        module: "core.runtime",
        status: "failed",
        workOrderId: input.workOrderId,
      });
    });
  }
  this.logger?.info("Agent work order receipt enqueued", {
    ...traceContextToLogContext(traceContext),
    event: "agent_work_order_receipt.enqueued",
    module: "core.runtime",
    outcomeStatus: input.outcome.status,
    sessionId: this.sessionId,
    targetSessionId: input.targetSessionId,
    workOrderId: input.workOrderId,
  });
}

/**
 * 回执轮：落库 synthetic notice（provider 可见、UI 不画气泡；轮头卡走
 * backgroundResult 链）→ 以回执身份独立成轮。回执轮是发起方自己的普通轮，
 * 不带 workorder- 前缀身份（发起方在回执轮里仍可派新单，嵌套上限只限工单轮）。
 */
export async function runWorkOrderReceiptCommand(
  this: AgentRuntimeInternal,
  command: WorkOrderReceiptRuntimeCommand,
): Promise<void> {
  if (isStaleBranchRuntimeCommand(this, command)) return;
  const foregroundExecution = beginForegroundExecution.call(this, command);
  try {
    const messageID = createMessageId();
    // 内存历史与持久化同一份原文（<work-order-receipt> 信封）；执行期投影再补
    // 「非用户权威」框架（provider-entry-origins 的 agent_work_order_receipt 分支）。
    this.messageHistory.addUser(command.text, runtimeInputMetadata("agent_work_order_receipt"));
    await this.persistSyntheticUserNoticeForSession({
      messageID,
      metadata: {
        envelope: command.envelope,
        inputPresentation: "agent_work_order_receipt",
        originMeta: command.originMeta,
        outcome: {
          status: command.outcome.status,
          ...(command.outcome.reason ? { reason: command.outcome.reason } : {}),
        },
        visibility: "model-only",
        ...(command.targetAgentId ? { targetAgentId: command.targetAgentId } : {}),
        targetAgentName: command.targetAgentName,
        targetSessionId: command.targetSessionId,
      },
      sessionId: this.sessionId,
      source: "agent_work_order_receipt",
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
        this.logger?.warn("Failed to mark agent work order receipt promoted", {
          ...traceContextToLogContext(command.traceContext),
          commandId: command.id,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.promote_mark_failed",
          module: "core.runtime",
          status: "failed",
          workOrderId: command.workOrderId,
        });
      });
    this.logger?.info("Agent work order receipt turn started", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      event: "agent_work_order_receipt.turn_started",
      messageId: messageID,
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
    await this.executeTurnCommand(command.text, undefined, {
      abortSignal: foregroundExecution.controller.signal,
      // 回执轮不使用 workorder- 前缀（那是工单轮 denylist 的身份信号）。
      inputId: uuidv7(),
      inputPresentation: "agent_work_order_receipt",
      inputSource: "agent_work_order_receipt",
      inputVisibility: "model-only",
      originMeta: command.originMeta,
      backgroundSource: "agent_work_order_receipt",
      recordedInputMessageId: messageID,
      skipInputRecord: true,
      skipUserPromptSubmitHooks: true,
      traceContext: command.traceContext,
    });
  } catch (error) {
    this.logger?.warn("Agent work order receipt turn failed", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agent_work_order_receipt.turn_failed",
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
  } finally {
    finishForegroundExecution.call(this, foregroundExecution);
  }
}
