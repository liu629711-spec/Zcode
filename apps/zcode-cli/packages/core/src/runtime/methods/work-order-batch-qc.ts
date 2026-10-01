// ============================================================
// 批次质检（纪律协议批）：批次全部收口后发起方会话的自动验货轮。
// 触发（maybeEnqueueAgentWorkOrderBatchQc）与开轮（enqueueAgentWorkOrderBatchQc）
// 都在 core：活回执投递（bootstrap deliverWorkOrderReceipt）与 resume 清扫
// （steering 的重投/合成回执）两条路都调同一个入口——清扫路径不经过 bootstrap，
// 漏掉它崩溃过的批次就永远不开质检（评审 A1/B4）。
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
  WORK_ORDER_RESTRICTED_TOOL_NAMES,
  type BatchQcOrder,
} from "../../subagent/work-order.js";
import type { AgentRuntimeInternal } from "../internal.js";
import { isStaleBranchRuntimeCommand } from "./runtime-command-generation.js";
import { beginForegroundExecution, finishForegroundExecution } from "./runtime-command-queue.js";

/**
 * 闸门台账行的确定性 id（会话命名空间）：`agentWorkOrderBatchQc:<sessionId>:<batchId>`。
 * batchId 是模型回传的裸串（跨会话可能撞车，评审 A3），闸门行又是 session_input 的
 * 全局主键——不带会话命名空间，两个老板会话复用同一 batchId 时第二个永远查不到自己的
 * 闸门行，「每批只质检一次」被击穿。触发查询与落行两侧都用本函数，杜绝拼法漂移。
 */
export function batchQcLedgerId(sessionId: string, batchId: string): string {
  return `agentWorkOrderBatchQc:${sessionId}:${batchId}`;
}

export interface EnqueueAgentWorkOrderBatchQcInput {
  batchId: string;
  batchTitle?: string;
  /**
   * 合议口味（评审会批 2026-10-02）：本批工单全是评审单 → 收口轮是合议轮
   * （合议要求随信封、AgentDispatch 进 denylist、originMeta.qcKind="review"），
   * 缺席 = 批次质检轮。
   */
  review?: boolean;
  orders: readonly BatchQcOrder[];
  traceContext?: TraceContext;
}

/**
 * 公共入口（AgentRuntime 面上）：为一批已收口的工单开一轮自动质检。
 * 闸门（每批只此一次）由确定性台账行承担，且**落闸先于开轮、落闸失败就放弃开轮**
 * （评审 A4/R1）：落闸 await 到底，写不进账本就不开轮——宁缺毋滥，绝不出现
 * 「质检已在跑而闸门缺席」的二开窗口；写成功路径下 node:sqlite 同步写保证
 * 命令出队前闸门行必已可见。
 * 崩溃窗口（闸门已落、轮未跑）质检丢失不补跑：与回执同一档 best-effort，权威
 * 事实在各目标会话的留档里。
 */
export async function enqueueAgentWorkOrderBatchQc(
  this: AgentRuntimeInternal,
  input: EnqueueAgentWorkOrderBatchQcInput,
): Promise<void> {
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
    ...(input.review === true ? { review: true } : {}),
    orders: input.orders,
  });
  const originMeta: BackgroundResultOriginMeta = {
    backgroundSource: "agent_work_order_batch_qc",
    workId: input.batchId,
    title: buildBatchQcTitle(input.batchTitle, { review: input.review === true }),
    batchId: input.batchId,
    ...(input.batchTitle === undefined ? {} : { batchTitle: input.batchTitle }),
    // 合议口味随轮头下发：UI 据此换合议词表（不从标题文本反推）。
    ...(input.review === true ? { qcKind: "review" as const } : {}),
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
    ...(input.review === true ? { review: true } : {}),
    priority: "next",
    source: "agent_work_order_batch_qc",
    text,
    traceContext,
  };
  // 闸门台账行（deterministic id，upsert 幂等）**先于** enqueueRuntimeCommand 且
  // await 到底（见函数头注）。resume 清扫会把残余 admitted 行收口为 discarded，
  // 不影响触发侧的「行存在即跳过」判定；run 阶段 promote 的就是这一行（id 对齐）。
  try {
    const admission = await this.sessionStore?.saveSessionInput?.({
      id: batchQcLedgerId(this.sessionId, input.batchId),
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
    this.enqueueRuntimeCommand(command);
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
  } catch (error) {
    // 落闸失败 = 闸门缺席，开轮就是裸奔（评审 R1）：放弃这轮质检，如实留痕。
    // 批次数据无损（回执都已送达），老板下次重派或手动验收不受影响。
    this.logger?.warn("Failed to save batch QC gate row; skipping inspection turn", {
      ...traceContextToLogContext(traceContext),
      batchId: input.batchId,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agent_work_order_batch_qc.gate_save_failed",
      module: "core.runtime",
      status: "failed",
    });
    return;
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
 * 同批并发交活时，最后两张回执的触发可能交错：都先查闸门再落行会双双放行。
 * 触发按 `${sessionId}:${batchId}` 串行——前一个跑完（含落闸）才轮到下一个，
 * 后者必见闸门行而跳过。Map 只在本进程内记账；落闸是同步写，跨进程/重启由
 * 台账行兜住（活投递与 resume 清扫同进程内也各自串行于本 Map）。
 */
const batchQcTriggerChains = new Map<string, Promise<void>>();

export interface MaybeEnqueueAgentWorkOrderBatchQcInput {
  batchId: string;
  traceContext?: TraceContext;
}

/**
 * 公共触发（AgentRuntime 面上）：本会话里 batchId 批次若已全部收口且未质检过，
 * 开一轮质检；否则空转。幂等安全的自查入口——活回执投递与 resume 清扫共用。
 */
export async function maybeEnqueueAgentWorkOrderBatchQc(
  this: AgentRuntimeInternal,
  input: MaybeEnqueueAgentWorkOrderBatchQcInput,
): Promise<void> {
  const chainKey = `${this.sessionId}:${input.batchId}`;
  const previous = batchQcTriggerChains.get(chainKey) ?? Promise.resolve();
  const chain = previous.then(() => startBatchQcIfComplete.call(this, input));
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
  this: AgentRuntimeInternal,
  input: MaybeEnqueueAgentWorkOrderBatchQcInput,
): Promise<void> {
  const sessionStore = this.sessionStore;
  if (!sessionStore?.listSessionInputs) return;
  const rows = await sessionStore.listSessionInputs({ sessionID: this.sessionId });
  // 闸门：本批的质检台账行已在（任意状态）→ 这批已经质检过（或已排队），绝不二开。
  if (rows.some((record) => record.id === batchQcLedgerId(this.sessionId, input.batchId))) {
    return;
  }
  // 收口判定：同批派单行一张不缺、且全部不在 admitted（回执已销账或 resume 已收口）。
  const batchRows = rows.filter(
    (record) =>
      record.kind === "agentWorkOrderDispatch" &&
      (record.payload as { envelope?: { batchId?: unknown } } | undefined)?.envelope !==
        undefined &&
      (record.payload as { envelope?: { batchId?: unknown } }).envelope?.batchId ===
        input.batchId,
  );
  if (batchRows.length === 0) return;
  if (batchRows.some((record) => record.status === "admitted")) return;
  const orders = batchQcOrdersFromDispatchRows(batchRows, input.batchId);
  if (orders.length === 0) return;
  const batchTitle = readBatchTitleFromDispatchRows(batchRows);
  // 口味判定（评审会批）：同批派单行的信封**全是** review=true → 合议轮；混批或
  // 普通批都走质检轮（旧批次无 review 字段 → false，口径不变）。混批在此收敛为
  // 质检——派单侧的 schema refine 已把「评审单必须挂 batch_title」钉死，这里只是
  // 对畸形台账的兜底裁决，不为它再加一道有竞态的端口守卫。
  const review = batchRows.every(
    (record) =>
      (record.payload as { envelope?: { review?: unknown } } | undefined)?.envelope !== undefined &&
      (record.payload as { envelope?: { review?: unknown } }).envelope?.review === true,
  );
  await this.enqueueAgentWorkOrderBatchQc({
    batchId: input.batchId,
    ...(batchTitle === undefined ? {} : { batchTitle }),
    ...(review ? { review: true } : {}),
    orders,
    ...(input.traceContext === undefined ? {} : { traceContext: input.traceContext }),
  });
  this.logger?.info("Batch QC triggered: all work orders settled", {
    batchId: input.batchId,
    event: "agent_work_order_batch_qc.triggered",
    module: "core.runtime",
    orderCount: orders.length,
    sessionId: this.sessionId,
  });
}

/**
 * 派单行 payload → 质检清单（防御性收窄；任务为空/信封畸形的行跳过，不猜测）。
 * 员工名读 payload **顶层**的 agentName（员工真名）——信封里的 fromAgentName 是
 * 发起方署名，读它打回必派错人（评审 B1）；工号同源，改名不误派。
 */
function batchQcOrdersFromDispatchRows(
  rows: readonly { payload: { text: string; [key: string]: unknown } }[],
  batchId: string,
): BatchQcOrder[] {
  const orders: BatchQcOrder[] = [];
  for (const record of rows) {
    const payload = record.payload;
    const envelope = payload?.envelope;
    if (typeof envelope !== "object" || envelope === null || Array.isArray(envelope)) continue;
    const envelopeRecord = envelope as Record<string, unknown>;
    if (envelopeRecord.batchId !== batchId) continue;
    const workOrderId =
      typeof envelopeRecord.workOrderId === "string" ? envelopeRecord.workOrderId : "";
    const task = typeof envelopeRecord.task === "string" ? envelopeRecord.task.trim() : "";
    const agentName = typeof payload.agentName === "string" ? payload.agentName.trim() : "";
    const agentId = typeof payload.agentId === "string" ? payload.agentId.trim() : "";
    if (!workOrderId || !task) continue;
    orders.push({
      workOrderId,
      agentName,
      ...(agentId ? { agentId } : {}),
      task,
    });
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
    // 落库账本行就是闸门行（id 同 batchQcLedgerId）——promote 对准它，别用
    // command.id（那行不存在，promote 会对着空气开枪，评审 A2/B3）。
    await this.sessionStore
      ?.markSessionInputPromoted?.({
        id: batchQcLedgerId(this.sessionId, command.batchId),
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
      // 合议轮禁派单（评审会批）：合议的产出是意见不是工单，机制层掐死
      // 「评审→重派→再评审」的循环，决定权留在用户手里。质检轮仍不带 denylist
      // ——打回重派靠发起方自己的 AgentDispatch。
      ...(command.review === true
        ? { toolDisallowlist: [...WORK_ORDER_RESTRICTED_TOOL_NAMES] }
        : {}),
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
