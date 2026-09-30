// ============================================================
// 派单回执（D29/D3）：目标轮完成钩子与回执投递（bootstrap 侧）。
// ============================================================
// 从 agent-dispatch-port 拆出（max-lines 纪律）：dispatch() 之后的另一半——
// 订阅目标轮终态、失败自动重试一次（2026-10-01 员工可靠性批）、把回执投回
// 发起方会话。对 deps 的依赖刻意收窄成 activateSessionRecord 一项
// （发起方不在场时冷恢复），与端口其余依赖零耦合，不形成循环引用。
// 完整链路：dispatch() → enqueueAgentWorkOrder（工单轮）→ 本模块的完成钩子
// → deliverWorkOrderReceipt → 发起方 enqueueAgentWorkOrderReceipt（回执轮）。

import {
  SessionEventType,
  WORK_ORDER_INPUT_ID_PREFIX,
  type AgentWorkOrderEnvelope,
  type SessionEvent,
} from "@zcode/contracts";
import type { ModelSelection } from "@zcode/shared";
import type { WorkOrderReceiptOutcome } from "@zcode/core";

import type {
  ZCodeProtocolAgentServerContext,
  ZCodeProtocolSessionRecord,
} from "./server-types.js";

/** 回执链对端口依赖的窄视图：只需要「发起方不在场时冷恢复」这一件事。 */
interface ReceiptRelayDeps {
  activateSessionRecord: (sessionId: string) => Promise<ZCodeProtocolSessionRecord>;
}

/**
 * 目标会话事件 → 回执终态。按工单唤醒轮的 `workorder-` inputId 对号
 * （TurnComplete/TurnError 载荷带 inputId），其余事件一律忽略。
 * cancelled 轮不假报完成；TurnError/异常终态如实带原因（Codex 错误指引口径）。
 * 载荷按 unknown record 防御性收窄（server-operations 同风格），畸形载荷不投递。
 */
export function receiptOutcomeFromSessionEvent(
  event: SessionEvent,
  workOrderInputId: string,
): WorkOrderReceiptOutcome | undefined {
  const payload = event.payload;
  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) return undefined;
  const record = payload as Record<string, unknown>;
  if (record.inputId !== workOrderInputId) return undefined;
  if (event.type === SessionEventType.TurnComplete) {
    if (record.resultType === "success" && typeof record.response === "string") {
      return { status: "completed", response: record.response };
    }
    if (record.resultType === "cancelled") return { status: "cancelled" };
    return {
      status: "failed",
      reason: `the work order turn ended with resultType ${String(record.resultType)}`,
    };
  }
  if (event.type === SessionEventType.TurnError) {
    const error = record.error;
    if (typeof error !== "object" || error === null || Array.isArray(error)) {
      return { status: "failed", reason: "unknown turn error" };
    }
    const errorRecord = error as Record<string, unknown>;
    const reason =
      typeof errorRecord.message === "string" && errorRecord.message.trim()
        ? errorRecord.message
        : "unknown turn error";
    // 结构化失败线索（2026-10-01 员工可靠性批）：TurnError 载荷的 error 是
    // projectExecutionErrorPayload 的投影——message 已是真根因，code/attribution
    // 让 UI 讲大白话（如 invalid_model_request + 被拒模型 id），不回退到文本反推。
    // 上界与消费侧 shared schema 对齐（failureCode max 128 / modelId max 160）：
    // 超界会让回执轮头行 parse 失败被整行丢掉，截断诚实而不是赌上游。
    const attribution =
      typeof errorRecord.attribution === "object" && errorRecord.attribution !== null
        ? (errorRecord.attribution as Record<string, unknown>)
        : undefined;
    const failureCodeRaw =
      typeof errorRecord.code === "string" && errorRecord.code.trim()
        ? errorRecord.code.trim()
        : undefined;
    const failureModelIdRaw =
      typeof attribution?.modelId === "string" && attribution.modelId.trim()
        ? attribution.modelId.trim()
        : undefined;
    return {
      status: "failed",
      reason,
      ...(failureCodeRaw ? { failureCode: failureCodeRaw.slice(0, 128) } : {}),
      ...(failureModelIdRaw ? { failureModelId: failureModelIdRaw.slice(0, 160) } : {}),
    };
  }
  return undefined;
}

/**
 * 完成钩子：订阅目标 record 的会话事件，目标轮落终态后把回执投回发起方会话
 * （发起方 record 不在场则先 activateSessionForResume 恢复——落库等重开）。
 * 失败不判死（2026-10-01 员工可靠性批，真机事故 9-30 工单三连炸只能手动重派）：
 * 首个 failed 终态先自动重试一次——原信封原模型原 workOrderId 原样再投，第二棒
 * （retried）落终态才投回执，措辞带「已重试仍失败」。cancelled/completed 照旧直投。
 * 目标 record 被关闭/进程退出时钩子随 record 消失，回执与工单同样属于「死会话里
 * 不可恢复」的 best-effort 通知（发起方持有受理凭据）。
 *
 * 重试的订阅时点纪律（评审 A 的 P1）：重试整体推迟到**下一个宏任务**——本回调
 * 正跑在 notifyEventSinks 的 Set 迭代中途，此刻挂新订阅会被同一次迭代访问到，
 * 收到正在分发的那条 TurnError（两次 attempt 的 inputId 确定性相同），回执提前
 * 盖章、重试结果被丢。分发循环是微任务链，必然在任何宏任务前跑完，所以推迟到
 * `setTimeout(0)` 后的订阅/enqueue 都在干净时点（queueMicrotask 不行，仍排在
 * 迭代恢复之前）。之后第一棒再没有在飞事件，同 inputId 的终态只可能来自重试轮。
 */
export function scheduleWorkOrderReceiptRelay(
  context: ZCodeProtocolAgentServerContext,
  deps: ReceiptRelayDeps,
  input: {
    targetRecord: ZCodeProtocolSessionRecord;
    inputId: string;
    envelope: AgentWorkOrderEnvelope;
    agentName: string;
    agentId?: string;
    /** 本单生效的 per-order 模型覆盖：enqueue 时带什么，重试就原样带什么。 */
    modelSelection?: ModelSelection;
    /** true = 本订阅是自动重试的第二棒：失败不再重试，如实投递。 */
    retried?: boolean;
  },
): () => void {
  const unsubscribe = input.targetRecord.app.runtime.subscribeEvents({
    onSessionEvent: (event: SessionEvent) => {
      const outcome = receiptOutcomeFromSessionEvent(event, input.inputId);
      if (!outcome) return;
      unsubscribe();
      if (outcome.status === "failed" && !input.retried) {
        // 重试推迟到下一个宏任务（本函数头注：Set 迭代中途挂订阅会收到正在
        // 分发的这条终态事件）。catch 兜 retryWorkOrderOnce 的投递失败。
        setTimeout(() => {
          void retryWorkOrderOnce(context, deps, input, outcome).catch((error) => {
            context.logger?.warn("Failed to schedule agent work order retry", {
              errorMessage: error instanceof Error ? error.message : String(error),
              event: "agent_work_order_receipt.retry_schedule_failed",
              fromSessionId: input.envelope.fromSessionId,
              module: "bootstrap.zcode_protocol",
              targetSessionId: input.targetRecord.app.sessionId,
              workOrderId: input.envelope.workOrderId,
            });
          });
        }, 0);
        return;
      }
      const finalOutcome =
        outcome.status === "failed" && input.retried ? { ...outcome, retried: true } : outcome;
      void deliverWorkOrderReceipt(context, deps, {
        envelope: input.envelope,
        agentName: input.agentName,
        ...(input.agentId ? { agentId: input.agentId } : {}),
        targetSessionId: input.targetRecord.app.sessionId,
        outcome: finalOutcome,
      }).catch((error) => {
        context.logger?.warn("Failed to deliver agent work order receipt", {
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "agent_work_order_receipt.delivery_failed",
          fromSessionId: input.envelope.fromSessionId,
          module: "bootstrap.zcode_protocol",
          outcomeStatus: finalOutcome.status,
          targetSessionId: input.targetRecord.app.sessionId,
          workOrderId: input.envelope.workOrderId,
        });
      });
    },
  });
  return unsubscribe;
}

/**
 * 自动重试（只此一次）：原信封原模型原 workOrderId 再投同一目标会话；投不进去就把原失败如实上报。
 * 调用方（宏任务推迟后）保证本函数不在事件分发迭代中跑：先订阅后 enqueue 都在
 * 干净时点，第一棒没有在飞事件，同 inputId 的终态只可能来自重试轮。
 */
async function retryWorkOrderOnce(
  context: ZCodeProtocolAgentServerContext,
  deps: ReceiptRelayDeps,
  input: {
    targetRecord: ZCodeProtocolSessionRecord;
    envelope: AgentWorkOrderEnvelope;
    agentName: string;
    agentId?: string;
    modelSelection?: ModelSelection;
  },
  failedOutcome: WorkOrderReceiptOutcome,
): Promise<void> {
  // 与 core enqueueAgentWorkOrder 的派生规则一致（workorder-<workOrderId>）：
  // 重试沿用原 workOrderId，inputId 因此确定，无需等 admission 再订阅。
  const retryInputId = `${WORK_ORDER_INPUT_ID_PREFIX}${input.envelope.workOrderId}`;
  const unsubscribe = scheduleWorkOrderReceiptRelay(context, deps, {
    targetRecord: input.targetRecord,
    inputId: retryInputId,
    envelope: input.envelope,
    agentName: input.agentName,
    ...(input.agentId ? { agentId: input.agentId } : {}),
    modelSelection: input.modelSelection,
    retried: true,
  });
  try {
    const admission = await input.targetRecord.app.runtime.enqueueAgentWorkOrder({
      envelope: input.envelope,
      traceContext: input.targetRecord.traceContext,
      ...(input.modelSelection === undefined ? {} : { modelSelection: input.modelSelection }),
    });
    context.logger?.info("Agent work order failed once; auto-retry dispatched", {
      event: "agent_work_order.auto_retry",
      fromSessionId: input.envelope.fromSessionId,
      module: "bootstrap.zcode_protocol",
      targetSessionId: input.targetRecord.app.sessionId,
      workOrderId: input.envelope.workOrderId,
      delivery: admission.delivery,
    });
  } catch (error) {
    unsubscribe();
    context.logger?.warn("Agent work order retry could not be enqueued; delivering original failure", {
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agent_work_order.retry_enqueue_failed",
      fromSessionId: input.envelope.fromSessionId,
      module: "bootstrap.zcode_protocol",
      targetSessionId: input.targetRecord.app.sessionId,
      workOrderId: input.envelope.workOrderId,
    });
    await deliverWorkOrderReceipt(context, deps, {
      envelope: input.envelope,
      agentName: input.agentName,
      ...(input.agentId ? { agentId: input.agentId } : {}),
      targetSessionId: input.targetRecord.app.sessionId,
      outcome: failedOutcome,
    });
  }
}

async function deliverWorkOrderReceipt(
  context: ZCodeProtocolAgentServerContext,
  deps: ReceiptRelayDeps,
  input: {
    envelope: AgentWorkOrderEnvelope;
    agentName: string;
    agentId?: string;
    targetSessionId: string;
    outcome: WorkOrderReceiptOutcome;
  },
): Promise<void> {
  const initiatorRecord =
    context.sessions.get(input.envelope.fromSessionId) ??
    (await deps.activateSessionRecord(input.envelope.fromSessionId));
  initiatorRecord.app.runtime.enqueueAgentWorkOrderReceipt({
    workOrderId: input.envelope.workOrderId,
    agentName: input.agentName,
    ...(input.agentId ? { agentId: input.agentId } : {}),
    targetSessionId: input.targetSessionId,
    envelope: input.envelope,
    outcome: input.outcome,
    traceContext: initiatorRecord.traceContext,
  });
  context.logger?.info("Agent work order receipt delivered", {
    event: "agent_work_order_receipt.delivered",
    fromSessionId: input.envelope.fromSessionId,
    module: "bootstrap.zcode_protocol",
    outcomeStatus: input.outcome.status,
    targetSessionId: input.targetSessionId,
    workOrderId: input.envelope.workOrderId,
  });
}
