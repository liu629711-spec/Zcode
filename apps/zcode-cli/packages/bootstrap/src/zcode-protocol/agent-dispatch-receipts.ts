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
  type SessionId,
} from "@zcode/contracts";
import type { ModelSelection } from "@zcode/shared";
import type { BatchQcOrder, WorkOrderReceiptOutcome } from "@zcode/core";

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
/** 回执看门狗默认时长：工单合法长跑可超 30 分钟，触发时会先查活跃轮再决定。 */
const WATCHDOG_MS_DEFAULT = 30 * 60_000;

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
    /** 回执看门狗时长；缺席用 30 分钟默认（测试注入小值）。 */
    watchdogMs?: number;
  },
): () => void {
  let unsubscribe: () => void;
  let watchdog: ReturnType<typeof setTimeout> | undefined;
  const clearWatchdog = () => {
    if (watchdog !== undefined) {
      clearTimeout(watchdog);
      watchdog = undefined;
    }
  };
  const handleTerminalOutcome = (outcome: WorkOrderReceiptOutcome) => {
    clearWatchdog();
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
  };
  // 回执看门狗（audit 2026-10-01 对账批）：目标 record 关闭/卡死时订阅随 record
  // 消失，回执永不到达，老板的卡永远停在「已派单」。到点先看本单是否还在目标
  // 会话里活跃（长任务合法超过 30 分钟）：还在跑就重新武装；确认死单才合成
  // 超时失败回执，走与 TurnError 相同的终态处理。
  const armWatchdog = () => {
    watchdog = setTimeout(() => {
      const runtime = input.targetRecord.app.runtime;
      const stillRunning =
        runtime.getActiveTurnInfo()?.inputId === input.inputId || runtime.hasActiveOrQueuedTurnWork();
      if (stillRunning) {
        armWatchdog();
        return;
      }
      context.logger?.warn("Agent work order receipt timed out; synthesizing failure", {
        event: "agent_work_order_receipt.watchdog_fired",
        fromSessionId: input.envelope.fromSessionId,
        module: "bootstrap.zcode_protocol",
        targetSessionId: input.targetRecord.app.sessionId,
        workOrderId: input.envelope.workOrderId,
      });
      // fire 先退订（终审评委A P1-2）：fire 与事件终态竞态时，重试腿的终态会同时
      // 命中本订阅与 leg2 新订阅造成双投；退订后本订阅不再收事件。
      unsubscribe();
      handleTerminalOutcome({
        status: "failed",
        // 大白话直出（终审评委B）：这行会进老板的失败卡（generic 句式带原原因）。
        reason: "工单等回执超时：目标会话可能已被关闭或中断，这单没有留下结果。",
      });
    }, input.watchdogMs ?? WATCHDOG_MS_DEFAULT);
  };
  unsubscribe = input.targetRecord.app.runtime.subscribeEvents({
    onSessionEvent: (event: SessionEvent) => {
      const outcome = receiptOutcomeFromSessionEvent(event, input.inputId);
      if (!outcome) return;
      unsubscribe();
      handleTerminalOutcome(outcome);
    },
  });
  armWatchdog();
  return () => {
    clearWatchdog();
    unsubscribe();
  };
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
    watchdogMs?: number;
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
    ...(input.watchdogMs === undefined ? {} : { watchdogMs: input.watchdogMs }),
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
  await deliverWorkOrderReceiptInner(context, deps, input);
  // 销账（终审评委A/B 共同 P1）：回执已到达发起方，派单台账行收口——
  // 不收口的话，之后任何一次 resume 都会为早已完成的工单合成假失败。
  // status 枚举没有"已兑现"，用 discarded + 专用 reason 表示收口。
  try {
    await context.deps.sessionStore?.settleSessionInput?.({
      id: `agentWorkOrderDispatch:${input.envelope.workOrderId}`,
      sessionID: input.envelope.fromSessionId as SessionId,
      status: "discarded",
      reason: "receipt_delivered",
    });
  } catch {
    // 收口失败 ≠ 回执失败；清扫侧按"有回执行即不合成"兜底。
  }
  // 批次质检（纪律协议批）：本单销账后批次可能刚好全部收口——触发侧自查
  // （闸门行 + 同批派单行全收口），不满足就是空转一次查询。质检失败不影响回执。
  if (input.envelope.batchId) {
    try {
      await maybeStartBatchQc(context, deps, {
        fromSessionId: input.envelope.fromSessionId,
        envelope: input.envelope,
      });
    } catch (error) {
      context.logger?.warn("Failed to start batch QC after receipt delivery", {
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "agent_work_order_batch_qc.trigger_failed",
        fromSessionId: input.envelope.fromSessionId,
        module: "bootstrap.zcode_protocol",
        workOrderId: input.envelope.workOrderId,
      });
    }
  }
}

async function deliverWorkOrderReceiptInner(
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

// ── 批次质检（纪律协议批）：批次全部收口后自动开一轮验货 ──────────────

/**
 * 闸门台账行的确定性 id。与 core enqueueAgentWorkOrderBatchQc 的派生规则一致
 * （`agentWorkOrderBatchQc:<batchId>`）：触发侧查行在先、core 落行在后，两侧
 * 必须同一拼法，否则闸门失效、每张回执都会重开质检轮。
 */
function batchQcLedgerId(batchId: string): string {
  return `agentWorkOrderBatchQc:${batchId}`;
}

/**
 * 同批发单并发交活时，最后两张回执的投递可能交错：都先查闸门再落行会双双放行。
 * 触发按 `${fromSessionId}:${batchId}` 串行——前一个触发跑完（含落行）才轮到
 * 下一个，后者必见闸门行而跳过。Map 只在本进程内记账，跨重启由台账行兜住。
 */
const batchQcTriggerChains = new Map<string, Promise<void>>();

/**
 * 批次收口触发：同批派单台账行全部收口（无一 admitted）且闸门行不存在（任意
 * 状态——resume 清扫会把残余收口为 discarded，行仍在）时，为发起方开一轮质检。
 * 「每批只质检一次」是硬边界：打回重派的单回执到达时闸门已在，不会二次质检。
 * 崩溃窗口（闸门行已落、质检轮未跑）不补跑：与回执同档 best-effort。
 */
export async function maybeStartBatchQc(
  context: ZCodeProtocolAgentServerContext,
  deps: ReceiptRelayDeps,
  input: {
    fromSessionId: string;
    envelope: AgentWorkOrderEnvelope;
  },
): Promise<void> {
  const batchId = input.envelope.batchId;
  if (!batchId) return;
  const chainKey = `${input.fromSessionId}:${batchId}`;
  const previous = batchQcTriggerChains.get(chainKey) ?? Promise.resolve();
  const chain = previous.then(() =>
    startBatchQcIfComplete(context, deps, { fromSessionId: input.fromSessionId, batchId }),
  );
  batchQcTriggerChains.set(chainKey, chain);
  try {
    await chain;
  } finally {
    if (batchQcTriggerChains.get(chainKey) === chain) {
      batchQcTriggerChains.delete(chainKey);
    }
  }
}

async function startBatchQcIfComplete(
  context: ZCodeProtocolAgentServerContext,
  deps: ReceiptRelayDeps,
  input: { fromSessionId: string; batchId: string },
): Promise<void> {
  const sessionStore = context.deps.sessionStore;
  if (!sessionStore?.listSessionInputs) return;
  const rows = await sessionStore.listSessionInputs({
    sessionID: input.fromSessionId as SessionId,
  });
  // 闸门：本批的质检台账行已在（任意状态）→ 这批已经质检过（或已排队），绝不二开。
  if (rows.some((record) => record.id === batchQcLedgerId(input.batchId))) return;
  // 收口判定：同批派单行一张不缺、且全部不在 admitted（回执已销账或 resume 已收口）。
  const batchRows = rows.filter(
    (record) =>
      record.kind === "agentWorkOrderDispatch" &&
      (record.payload as Record<string, unknown> | undefined)?.envelope !== undefined &&
      (record.payload as { envelope?: { batchId?: unknown } }).envelope?.batchId === input.batchId,
  );
  if (batchRows.length === 0) return;
  if (batchRows.some((record) => record.status === "admitted")) return;
  const orders = batchQcOrdersFromDispatchRows(batchRows, input.batchId);
  if (orders.length === 0) return;
  const initiatorRecord =
    context.sessions.get(input.fromSessionId) ?? (await deps.activateSessionRecord(input.fromSessionId));
  const batchTitle = readBatchTitleFromDispatchRows(batchRows);
  initiatorRecord.app.runtime.enqueueAgentWorkOrderBatchQc({
    batchId: input.batchId,
    ...(batchTitle === undefined ? {} : { batchTitle }),
    orders,
    traceContext: initiatorRecord.traceContext,
  });
  context.logger?.info("Batch QC triggered: all work orders settled", {
    batchId: input.batchId,
    event: "agent_work_order_batch_qc.triggered",
    fromSessionId: input.fromSessionId,
    module: "bootstrap.zcode_protocol",
    orderCount: orders.length,
  });
}

/** 派单行 payload → 质检清单（防御性收窄；任务为空/信封畸形的行跳过，不猜测）。 */
function batchQcOrdersFromDispatchRows(
  rows: readonly { payload: { text: string; [key: string]: unknown } }[],
  batchId: string,
): BatchQcOrder[] {
  const orders: BatchQcOrder[] = [];
  for (const record of rows) {
    const envelope = record.payload?.envelope;
    if (typeof envelope !== "object" || envelope === null || Array.isArray(envelope)) continue;
    const source = envelope as Record<string, unknown>;
    if (source.batchId !== batchId) continue;
    const workOrderId = typeof source.workOrderId === "string" ? source.workOrderId : "";
    const task = typeof source.task === "string" ? source.task.trim() : "";
    const agentName = typeof source.fromAgentName === "string" ? source.fromAgentName.trim() : "";
    if (!workOrderId || !task) continue;
    orders.push({ workOrderId, agentName, task });
  }
  // 质检清单与提示词按工单号定序：同样的批次铸出同样的输入（测试可对账）。
  orders.sort((a, b) => (a.workOrderId < b.workOrderId ? -1 : a.workOrderId > b.workOrderId ? 1 : 0));
  return orders;
}

/** 批次人类标题取自同批任一派单行的信封（铸造侧同批同题）；取不到就缺省。 */
function readBatchTitleFromDispatchRows(
  rows: readonly { payload: { text: string; [key: string]: unknown } }[],
): string | undefined {
  for (const record of rows) {
    const envelope = record.payload?.envelope;
    if (typeof envelope !== "object" || envelope === null || Array.isArray(envelope)) continue;
    const title = (envelope as { batchTitle?: unknown }).batchTitle;
    if (typeof title === "string" && title.trim()) return title.trim();
  }
  return undefined;
}
