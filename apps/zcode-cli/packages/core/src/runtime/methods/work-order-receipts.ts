// ============================================================
// 派单（D29/D3）回执投递：core 侧的窄公共入口与回执轮执行。
// bootstrap 协议端口在目标轮 TurnComplete/TurnError 后，经公共 AgentRuntime
// 接口把回执投回发起方会话；与 enqueueBackgroundTaskNotification 同型
// （排队 + 账本 admission），发起方忙时在回合边界独立成轮，绝不混进进行中回合。
// ============================================================

import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { TraceContext } from "../deps.js";
import type {
  AgentWorkOrderEnvelope,
  BackgroundReceiptOriginMeta,
  BackgroundResultOriginMeta,
} from "@zcode/contracts";
import { AGENT_WORK_ORDER_TASK_MAX_CHARS } from "@zcode/contracts";
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

/**
 * 回执收集窗（回执洪泛合并，audit C-P2，2026-10-05 老板拍板）：同批回执常在
 * 毫秒级先后到达，逐条独立成轮=逐条烧一轮模型。入队后先收窗 2 秒，窗内排队
 * 到位的兄弟回执在 run 时吸收进同一轮（轮头 originMeta.receipts 逐张带身份）。
 * 错峰到达的回执各自成轮——反馈实时性不为此让路。测试可改写本常量取 0。
 */
export const RECEIPT_COALESCE_MS = 2_000;

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
export async function enqueueAgentWorkOrderReceipt(
  this: AgentRuntimeInternal,
  input: EnqueueAgentWorkOrderReceiptInput,
): Promise<void> {
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
    // 回执终态结构化（地基清理 2026-10-04）：UI 三态灯的权威来源，不再认中文
    // 标题；三种终态恒带（不是 failed 专属）。
    receiptStatus: input.outcome.status,
    // 批次（工地卡）随回执轮头下发：发起方 UI 据它把同批回执归进一张工地卡。
    ...(input.envelope.batchId ? { batchId: input.envelope.batchId } : {}),
    ...(input.envelope.batchTitle ? { batchTitle: input.envelope.batchTitle } : {}),
    // 评审单标记随轮头下发（2026-10-07）：失败卡一键重派据此透传 review，
    // 重派收口才不会把合议口味退化成普通质检（评审会批交接项）。
    ...(input.envelope.review === true ? { review: true as const } : {}),
    // 圆桌会（真会议）随轮头下发：席位回执轮带 councilId/类型/轮次/阶段 + 座次与
    // 攻角——UI 据它把同场会议的发言聚成圆桌卡并按座次入席（batchId 同纪律）。
    ...(input.envelope.councilId
      ? {
          councilId: input.envelope.councilId,
          ...(input.envelope.councilKind ? { councilKind: input.envelope.councilKind } : {}),
          ...(input.envelope.councilRound ? { councilRound: input.envelope.councilRound } : {}),
          councilPhase: "deliberation" as const,
          ...(input.envelope.councilSeatIndex !== undefined &&
          input.envelope.councilSeatLens !== undefined
            ? {
                councilSeat: {
                  index: input.envelope.councilSeatIndex,
                  lens: input.envelope.councilSeatLens,
                },
              }
            : {}),
        }
      : {}),
    // 失败回执的结构化线索（2026-10-01 员工可靠性批）：原工单任务原文（一键重派
    // 的权威数据源，有界同 agentWorkOrderMeta）+ 根因分类码/被拒模型/已重试——
    // UI 讲大白话的依据，不从回执文本反推。completed/cancelled 不带。
    ...(input.outcome.status === "failed"
      ? {
          task:
            input.envelope.task.length > AGENT_WORK_ORDER_TASK_MAX_CHARS
              ? `${input.envelope.task.slice(0, AGENT_WORK_ORDER_TASK_MAX_CHARS - 1)}…`
              : input.envelope.task,
          ...(input.outcome.failureCode ? { failureCode: input.outcome.failureCode } : {}),
          ...(input.outcome.failureModelId
            ? { failureModelId: input.outcome.failureModelId }
            : {}),
          ...(input.outcome.reason ? { failureReason: input.outcome.reason } : {}),
          ...(input.outcome.retried ? { retried: true } : {}),
          // 一键重派按工号找人（员工改名后旧名会派错人）：失败回执带上交活方工号。
          ...(input.agentId ? { agentId: input.agentId } : {}),
        }
      : {}),
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
    // 收集窗（回执洪泛合并）：窗内不出队，兄弟回执由 run 侧吸收进同一轮。
    notRunnableBefore: new Date(Date.now() + RECEIPT_COALESCE_MS),
    mergedReceipts: [],
  };
  this.enqueueRuntimeCommand(command);
  // 账本 admission（durable 痕迹）：runtime 命令队列是纯内存的，崩溃后由 resume 统一收口。
  // 必须 await 到底且失败可见（2026-10-03 真机实证）：原 fire-and-forget 的落账失败
  // 无声无息——重启后命令 id 撞历史主键，回执行整整一周没进账，圆桌会收票作为
  // 第一个依赖回执台账的消费者被闷死。
  try {
    const admissionResult = await this.sessionStore?.saveSessionInput?.({
    id: String(command.id),
    sessionID: this.sessionId,
    kind: "agentWorkOrderReceipt",
    delivery: "queue",
    payload: {
      text,
      workOrderId: input.workOrderId,
      outcome: input.outcome,
      envelope: input.envelope,
      // 重投所需的最小身份（audit 2026-10-01 对账批）：崩溃后 resume 清扫重投
      // 回执时，从 payload 就能重建回执轮，不用反解析信封文本。agentId 也在场
      // （评委C P2-1）：重投的失败卡按工号重派，改名顶名不误派。
      agentName: input.agentName,
      ...(input.agentId ? { agentId: input.agentId } : {}),
      targetSessionId: input.targetSessionId,
    },
  });
    const admission = admissionResult;
    if (admission) {
      void this.trackResidencyBlockingWork(admission).catch((error) => {
        this.logger?.warn("Failed to track receipt residency", {
          ...traceContextToLogContext(traceContext),
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.residency_track_failed",
          module: "core.runtime",
          workOrderId: input.workOrderId,
        });
      });
    }
  } catch (error) {
    // 落账失败必须响亮：圆桌会收票按台账凑票，回执行缺席=会议卡死（真机教训）。
    this.logger?.warn("Failed to admit agent work order receipt to ledger", {
      ...traceContextToLogContext(traceContext),
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "session_input.admit_failed",
      module: "core.runtime",
      status: "failed",
      workOrderId: input.workOrderId,
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
 * 合并回执轮的轮头元数据（纯函数，测试可对账）：单张回执原样返回自己的
 * originMeta（与既有单张轮逐字节同形）；多张时顶层沿用首张、`receipts` 逐张
 * 带对账身份——身份不混批，只是同轮多张（回执洪泛合并 2026-10-05）。
 */
export function buildMergedReceiptOriginMeta(
  commands: readonly {
    originMeta: BackgroundResultOriginMeta;
    outcome: { status: "completed" | "failed" | "cancelled" };
  }[],
): BackgroundResultOriginMeta {
  const first = commands[0];
  if (!first) throw new Error("cannot build receipt origin meta from an empty batch");
  if (commands.length === 1) return first.originMeta;
  return {
    ...first.originMeta,
    receipts: commands.map((command) => {
      const meta = command.originMeta;
      const item: BackgroundReceiptOriginMeta = {
        workId: meta.workId,
        title: meta.title,
        receiptStatus: meta.receiptStatus ?? command.outcome.status,
        ...(meta.batchId ? { batchId: meta.batchId } : {}),
        ...(meta.batchTitle ? { batchTitle: meta.batchTitle } : {}),
        ...(meta.review ? { review: true } : {}),
        ...(meta.task ? { task: meta.task } : {}),
        ...(meta.failureCode ? { failureCode: meta.failureCode } : {}),
        ...(meta.failureModelId ? { failureModelId: meta.failureModelId } : {}),
        ...(meta.failureReason ? { failureReason: meta.failureReason } : {}),
        ...(meta.retried ? { retried: true } : {}),
        ...(meta.agentId ? { agentId: meta.agentId } : {}),
      };
      return item;
    }),
  };
}

/**
 * 回执轮：落库 synthetic notice（provider 可见、UI 不画气泡；轮头卡走
 * backgroundResult 链）→ 以回执身份独立成轮。回执轮是发起方自己的普通轮，
 * 不带 workorder- 前缀身份（发起方在回执轮里仍可派新单，嵌套上限只限工单轮）。
 * 收集窗内同到的兄弟回执（非圆桌席位单）在此吸收进同一轮：一份 notice、一次
 * 模型轮、逐张台账行 promote 对齐同一条消息——省轮次不丢身份。
 */
export async function runWorkOrderReceiptCommand(
  this: AgentRuntimeInternal,
  command: WorkOrderReceiptRuntimeCommand,
): Promise<void> {
  if (isStaleBranchRuntimeCommand(this, command)) return;
  // 圆桌会席位回执静默落账（2026-10-03 老板验收反馈）：不逐席吵醒主会话——
  // 老板要的是最终整合结果（主席合议轮一个交付），不是每席交一回就同步一遍。
  // 席位证据仍落库（notice 带 originMeta.council*），评审专区照常聚合；权威
  // 推进在 maybeAdvanceCouncilRound（回执投递侧已带即时 outcome）。
  if (command.envelope.councilId) {
    const messageID = createMessageId();
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
        this.logger?.warn("Failed to mark council seat receipt promoted", {
          ...traceContextToLogContext(command.traceContext),
          commandId: command.id,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.promote_mark_failed",
          module: "core.runtime",
          workOrderId: command.workOrderId,
        });
      });
    this.logger?.info("Council seat receipt recorded silently (no host turn)", {
      ...traceContextToLogContext(command.traceContext),
      event: "agent_work_order_receipt.council_silent",
      module: "core.runtime",
      outcomeStatus: command.outcome.status,
      sessionId: this.sessionId,
      workOrderId: command.workOrderId,
    });
    return;
  }
  // 吸收收集窗内排队到位的兄弟回执（回执洪泛合并）：只收非圆桌席位单（席位
  // 回执走上面的静默路径）；陈旧分支件就地丢弃（台账由 resume 清扫收口）。
  for (const sibling of this.runtimeCommandQueue.snapshot()) {
    if (sibling.mode !== "work-order-receipt" || sibling === command) continue;
    if (sibling.envelope.councilId) continue;
    if (isStaleBranchRuntimeCommand(this, sibling)) {
      this.runtimeCommandQueue.removeById(sibling.id);
      continue;
    }
    if (this.runtimeCommandQueue.removeById(sibling.id)) {
      command.mergedReceipts.push(sibling);
    }
  }
  const receiptCommands = [command, ...command.mergedReceipts];
  const mergedOriginMeta = buildMergedReceiptOriginMeta(receiptCommands);
  const combinedText = receiptCommands.map((receipt) => receipt.text).join("\n\n");
  const foregroundExecution = beginForegroundExecution.call(this, command);
  try {
    const messageID = createMessageId();
    // 内存历史与持久化同一份原文（逐张 <work-order-receipt> 信封连排）；执行期
    // 投影再补「非用户权威」框架（provider-entry-origins 的 receipt 分支）。
    this.messageHistory.addUser(combinedText, runtimeInputMetadata("agent_work_order_receipt"));
    await this.persistSyntheticUserNoticeForSession({
      messageID,
      metadata: {
        envelope: command.envelope,
        inputPresentation: "agent_work_order_receipt",
        originMeta: mergedOriginMeta,
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
      text: combinedText,
      traceContext: command.traceContext,
      visibility: "model-only",
    });
    // 逐张台账行 promote 对齐同一条消息：质检闸门/圆桌收票按台账行对账，
    // 吸收进同一轮不改变行数——只是轮次少了（audit C-P2 的省法）。
    for (const receipt of receiptCommands) {
      await this.sessionStore
        ?.markSessionInputPromoted?.({
          id: String(receipt.id),
          sessionID: this.sessionId,
          promotedMessageID: messageID,
        })
        .catch((error) => {
          this.logger?.warn("Failed to mark agent work order receipt promoted", {
            ...traceContextToLogContext(command.traceContext),
            commandId: String(receipt.id),
            errorMessage: error instanceof Error ? error.message : String(error),
            event: "session_input.promote_mark_failed",
            module: "core.runtime",
            status: "failed",
            workOrderId: receipt.workOrderId,
          });
        });
    }
    this.logger?.info("Agent work order receipt turn started", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      event: "agent_work_order_receipt.turn_started",
      mergedReceiptCount: command.mergedReceipts.length,
      messageId: messageID,
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
    await this.executeTurnCommand(combinedText, undefined, {
      abortSignal: foregroundExecution.controller.signal,
      // 回执轮不使用 workorder- 前缀（那是工单轮 denylist 的身份信号）。
      inputId: uuidv7(),
      inputPresentation: "agent_work_order_receipt",
      inputSource: "agent_work_order_receipt",
      inputVisibility: "model-only",
      originMeta: mergedOriginMeta,
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
      mergedReceiptCount: command.mergedReceipts.length,
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
  } finally {
    finishForegroundExecution.call(this, foregroundExecution);
  }
}
