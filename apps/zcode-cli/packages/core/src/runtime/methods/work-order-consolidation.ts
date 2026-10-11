// ============================================================
// 组长汇总轮（组长制批 2026-10-12）：子单收齐后在组长会话自动开的汇总轮。
// ============================================================
// gen-1 工单轮转派了子单（envelope.parentWorkOrderId 对上号的 gen-2 派单行）时，
// 父单回执在 bootstrap 侧挂起不投；每张子单回执销账后 bootstrap 回调本方法自查，
// 子单全部收口且未汇总过 → 铸汇总命令。汇总轮的最终回答就是父单的最终回执
// （bootstrap watcher 盯 `agent-work-order-consolidate:<父单id>` 的终态）。
// 幂等闸 = 台账确定性行 `agentWorkOrderConsolidation:<父单id>`，存在即汇总过。

import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { TraceContext } from "../deps.js";
import { WORK_ORDER_CONSOLIDATE_INPUT_ID_PREFIX } from "@zcode/contracts";
import {
  createRuntimeCommandId,
  type WorkOrderConsolidationRuntimeCommand,
} from "../command-queue.js";
import { runtimeInputMetadata } from "../../agent/runtime-input-presentation.js";
import {
  buildWorkOrderConsolidationEnvelopeText,
  WORK_ORDER_RESTRICTED_TOOL_NAMES,
  type WorkOrderConsolidationItem,
} from "../../subagent/work-order.js";
import { latestReceiptOutcomeByWorkOrderId } from "./work-order-batch-qc.js";
import { beginForegroundExecution, finishForegroundExecution } from "./runtime-command-queue.js";
import { isStaleBranchRuntimeCommand } from "./runtime-command-generation.js";
import type { AgentRuntimeInternal } from "../internal.js";

const CONSOLIDATION_GATE_PREFIX = "agentWorkOrderConsolidation:";

function envelopeField(
  payload: unknown,
): { parentWorkOrderId?: string; task?: string; workOrderId?: string } | undefined {
  if (typeof payload !== "object" || payload === null) return undefined;
  const envelope = (payload as { envelope?: unknown }).envelope;
  if (typeof envelope !== "object" || envelope === null) return undefined;
  const record = envelope as Record<string, unknown>;
  return {
    ...(typeof record.parentWorkOrderId === "string"
      ? { parentWorkOrderId: record.parentWorkOrderId }
      : {}),
    ...(typeof record.task === "string" ? { task: record.task } : {}),
    ...(typeof record.workOrderId === "string" ? { workOrderId: record.workOrderId } : {}),
  };
}

/**
 * 公共触发（AgentRuntime 面上）：本会话里 parentWorkOrderId 对上号的子单若已全部
 * 收口且未汇总过，开一轮汇总；否则空转。幂等安全的自查入口——bootstrap 在每张
 * 子单回执销账后回调；闸门行保证同一张父单只汇总一次。
 */
export async function maybeEnqueueAgentWorkOrderConsolidation(
  this: AgentRuntimeInternal,
  input: { workOrderId: string; traceContext?: TraceContext },
): Promise<void> {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  const sessionStore = this.sessionStore;
  if (!sessionStore?.listSessionInputs || !sessionStore.saveSessionInput) return;
  const rows = await sessionStore.listSessionInputs({ sessionID: this.sessionId });
  const gateId = `${CONSOLIDATION_GATE_PREFIX}${input.workOrderId}`;
  if (rows.some((row) => row.id === gateId)) return;

  // 子单清单：本会话台账里 parentWorkOrderId 对上号的派单行。没有 = 这不是挂起的父单。
  const subRows = rows.filter(
    (row) =>
      row.kind === "agentWorkOrderDispatch" &&
      envelopeField(row.payload)?.parentWorkOrderId === input.workOrderId,
  );
  if (subRows.length === 0) return;
  // 还有没收口的子单（admitted = 回执未销账）就再等等：下一次子单销账会再触发。
  if (subRows.some((row) => row.status === "admitted")) return;

  // 每张子单的回执终态（同 parent 的回执行取最新）。收口的单没有回执行是异常
  // （崩溃清扫应合成失败回执），如实按「回执缺失」入清单，不让汇总轮猜。
  const receipts = latestReceiptOutcomeByWorkOrderId(rows);
  // failed 单的 reason 从回执行 outcome.reason 补（latestReceiptOutcome 的窄视里没有）。
  const reasonByWorkOrderId = new Map<string, string>();
  for (const row of rows) {
    if (row.kind !== "agentWorkOrderReceipt") continue;
    const payload = row.payload as
      | { workOrderId?: unknown; outcome?: { reason?: unknown } }
      | undefined;
    const workOrderId = typeof payload?.workOrderId === "string" ? payload.workOrderId : "";
    const reason = payload?.outcome?.reason;
    if (workOrderId && typeof reason === "string" && reason.trim()) {
      reasonByWorkOrderId.set(workOrderId, reason);
    }
  }
  const items: WorkOrderConsolidationItem[] = subRows.map((row) => {
    const payload = row.payload as
      | { workOrderId?: unknown; agentName?: unknown }
      | undefined;
    const envelope = envelopeField(row.payload);
    const workOrderId =
      typeof payload?.workOrderId === "string"
        ? payload.workOrderId
        : (envelope?.workOrderId ?? "");
    const agentName = typeof payload?.agentName === "string" ? payload.agentName : "";
    const outcome = workOrderId ? receipts.get(workOrderId) : undefined;
    const status = outcome?.status;
    const normalizedStatus =
      status === "completed" || status === "cancelled" || status === "failed"
        ? status
        : "failed";
    return {
      workOrderId,
      agentName,
      task: envelope?.task ?? "",
      status: normalizedStatus,
      ...(normalizedStatus === "completed" && typeof outcome?.response === "string"
        ? { response: outcome.response }
        : {}),
      ...(normalizedStatus !== "completed"
        ? {
            reason:
              outcome === undefined
                ? "回执缺失（工单已收口但没有留下回执，可能是异常收口）"
                : reasonByWorkOrderId.get(workOrderId) ?? "未完成（原因未留痕）",
          }
        : {}),
    };
  });

  const command: WorkOrderConsolidationRuntimeCommand = {
    branchGeneration: this.branchGeneration,
    createdAt: new Date(),
    id: createRuntimeCommandId(),
    inputId: `${WORK_ORDER_CONSOLIDATE_INPUT_ID_PREFIX}${input.workOrderId}`,
    mode: "work-order-consolidation",
    priority: "next",
    source: "agent_work_order_batch_qc",
    text: buildWorkOrderConsolidationEnvelopeText({ workOrderId: input.workOrderId, items }),
    traceContext,
    workOrderId: input.workOrderId,
  };
  this.enqueueRuntimeCommand(command);
  // 闸门行（幂等凭据）：写进台账才铸轮——失败留在下一次子单销账重触发时自愈。
  await sessionStore.saveSessionInput({
    id: gateId,
    sessionID: this.sessionId,
    kind: "agentWorkOrderConsolidation",
    delivery: "queue",
    payload: {
      text: command.text,
      workOrderId: input.workOrderId,
    },
  });
  this.logger?.info("Agent work order consolidation enqueued", {
    ...traceContextToLogContext(traceContext),
    event: "agent_work_order.consolidation_enqueued",
    module: "core.runtime",
    sessionId: this.sessionId,
    subOrderCount: items.length,
    workOrderId: input.workOrderId,
  });
}

/**
 * 组长汇总轮：以汇总身份独立成轮（确定性 inputId，bootstrap watcher 靠它把本轮
 * 的最终回答当父单回执回传）。presentation/source 用 batch-qc 家族标签（闭集不动）；
 * 禁派单——汇总就是收口，要改派由老板重派。
 */
export async function runWorkOrderConsolidationCommand(
  this: AgentRuntimeInternal,
  command: WorkOrderConsolidationRuntimeCommand,
): Promise<void> {
  if (isStaleBranchRuntimeCommand(this, command)) return;
  const foregroundExecution = beginForegroundExecution.call(this, command);
  try {
    const messageID = createMessageId();
    this.messageHistory.addUser(command.text, runtimeInputMetadata("agent_work_order_batch_qc"));
    await this.persistSyntheticUserNoticeForSession({
      messageID,
      metadata: {
        inputPresentation: "agent_work_order_batch_qc",
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
        id: `${CONSOLIDATION_GATE_PREFIX}${command.workOrderId}`,
        sessionID: this.sessionId,
        promotedMessageID: messageID,
      })
      .catch((error) => {
        this.logger?.warn("Failed to mark agent work order consolidation promoted", {
          ...traceContextToLogContext(command.traceContext),
          commandId: command.id,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.promote_mark_failed",
          module: "core.runtime",
          workOrderId: command.workOrderId,
        });
      });
    this.logger?.info("Agent work order consolidation turn started", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      event: "agent_work_order.consolidation_turn_started",
      inputId: command.inputId,
      messageId: messageID,
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
    await this.executeTurnCommand(command.text, undefined, {
      abortSignal: foregroundExecution.controller.signal,
      inputId: command.inputId,
      inputPresentation: "agent_work_order_batch_qc",
      inputSource: "agent_work_order_batch_qc",
      inputVisibility: "model-only",
      recordedInputMessageId: messageID,
      skipInputRecord: true,
      skipUserPromptSubmitHooks: true,
      traceContext: command.traceContext,
      // 汇总轮禁派单：这是收口动作，要改派由老板重派。
      toolDisallowlist: [...WORK_ORDER_RESTRICTED_TOOL_NAMES],
    });
  } catch (error) {
    this.logger?.warn("Agent work order consolidation turn failed", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agent_work_order.consolidation_turn_failed",
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
  } finally {
    finishForegroundExecution.call(this, foregroundExecution);
  }
}
