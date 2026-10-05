// ============================================================
// 团队调度器（团队看板批2 2026-10-05）：依赖持派释放 + 自动返修循环。
// ============================================================
// 三条确定性规则（tianshu/AgentCore 口径：确定性闸门先行，模型轮只做建议）：
//  1. 依赖持派释放：信封带 dependsOn 且有前置未 completed → 派单端口有意挂起
//     （台账行 payload.held 记未满足的批内符号名，不投目标、不接回执线）；前置
//     全部 completed 后在此释放投递（同 workOrderId，同自动重试一样不换身份）。
//     依赖 honored 与「自动流转」开关无关——那是模型声明的依赖，系统照办。
//  2. 自动返修：评审单（reviews=被审工单 id）verdict=不通过 且 团队开关开 →
//     派返修单回原员工（repairOf+repairRound 随信封，落回原工地卡）。
//  3. 复审：返修单 completed → 派复审单回原评审人（reviews=返修工单 id，落回
//     评审批次）——直到通过或 repairRound 到上限（TEAM_BOARD_MAX_REPAIR_ROUNDS）
//     停手，看板标 escalated，等人拍板。
// 纯判定函数（computeXxx）与执行器分离：判定可测，执行器只在 bootstrap 触发。
// 同会话串行（schedulerChains）仿 batchQcTriggerChains：并发回执的轮次计数
// 不竞态，在飞的返修单挡住重复触发。

import {
  WORK_ORDER_INPUT_ID_PREFIX,
  AGENT_WORK_ORDER_TASK_MAX_CHARS,
  type AgentWorkOrderEnvelope,
} from "@zcode/contracts";
import { TEAM_BOARD_MAX_REPAIR_ROUNDS } from "@zcode/shared/zcode-protocol-v4";
import { parseReviewVerdict } from "@zcode/core";
import type { ZCodeProtocolAgentServerContext, ZCodeProtocolSessionRecord } from "./server-types.js";
import { createProtocolAgentDispatchPort } from "./agent-dispatch-port.js";
import {
  scheduleWorkOrderReceiptRelay,
  type ReceiptRelayDeps,
} from "./agent-dispatch-receipts.js";
import { computeCompletedTaskKeys, type TeamBoardInputRow } from "./team-board.js";

/** 团队「自动流转」开关行的确定性 id（台账 upsert 幂等，重存即改状态）。 */
export function teamFlowRowId(sessionId: string, batchId: string): string {
  return `agentWorkOrderTeamFlow:${sessionId}:${batchId}`;
}

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function envelopeOf(row: TeamBoardInputRow): Record<string, unknown> | undefined {
  const envelope = row.payload.envelope;
  return typeof envelope === "object" && envelope !== null && !Array.isArray(envelope)
    ? (envelope as Record<string, unknown>)
    : undefined;
}


/** 一张待释放的持派工单：台账行快照 + 释放投递所需身份。 */
export interface ReadyHeldWorkOrder {
  rowId: string;
  rowPayload: Record<string, unknown>;
  workOrderId: string;
  envelope: AgentWorkOrderEnvelope;
  agentName: string;
  agentId?: string;
  targetSessionId?: string;
  modelSelection?: { providerId: string; modelId: string; options?: { reasoningLevel?: string } };
}

/** 挑出前置已全部满足的持派工单（纯函数；不满足的继续挂着）。 */
export function computeReadyHeldWorkOrders(
  rows: readonly TeamBoardInputRow[],
): ReadyHeldWorkOrder[] {
  const completedTaskKeys = computeCompletedTaskKeys(rows);
  const ready: ReadyHeldWorkOrder[] = [];
  for (const row of rows) {
    if (row.kind !== "agentWorkOrderDispatch") continue;
    if (row.payload.held === undefined) continue;
    const envelope = envelopeOf(row);
    if (!envelope) continue;
    const dependsOn = Array.isArray(envelope.dependsOn) ? envelope.dependsOn : [];
    const pendingKeys = dependsOn.filter(
      (key) => !(typeof key === "string" && completedTaskKeys.has(key)),
    );
    if (pendingKeys.length > 0) continue;
    const workOrderId = readString(row.payload.workOrderId);
    const agentName = readString(row.payload.agentName) ?? "";
    if (!workOrderId) continue;
    const modelSelection =
      typeof row.payload.modelSelection === "object" && row.payload.modelSelection !== null
        ? (row.payload.modelSelection as ReadyHeldWorkOrder["modelSelection"])
        : undefined;
    ready.push({
      rowId: row.id,
      rowPayload: row.payload as Record<string, unknown>,
      workOrderId,
      envelope: envelope as unknown as AgentWorkOrderEnvelope,
      agentName,
      ...(readString(row.payload.agentId) ? { agentId: readString(row.payload.agentId) } : {}),
      ...(readString(row.payload.targetSessionId)
        ? { targetSessionId: readString(row.payload.targetSessionId) }
        : {}),
      ...(modelSelection ? { modelSelection } : {}),
    });
  }
  return ready;
}

/** 团队开关：任一相关批次开着即算开（返修链跨工地/评审两个批次）。 */
export function isTeamAutoFlowEnabled(
  rows: readonly TeamBoardInputRow[],
  batchIds: readonly (string | undefined)[],
): boolean {
  const wanted = new Set(batchIds.filter((id): id is string => typeof id === "string" && !!id));
  return rows.some(
    (row) =>
      row.kind === "agentWorkOrderTeamFlow" &&
      row.payload.enabled === true &&
      typeof row.payload.batchId === "string" &&
      wanted.has(row.payload.batchId),
  );
}

export type AutoRepairAction =
  | {
      kind: "repair";
      agent: string;
      agentId?: string;
      task: string;
      batchId?: string;
      batchTitle?: string;
      reviewsWorkOrderId: string;
      repairRound: number;
    }
  | { kind: "stop"; repairRound: number };

/** 返修工单正文：原任务原文 + 评审意见（评审回执正文有界截断，总长守工单上限）。 */
export function buildRepairTaskText(
  originalTask: string,
  reviewResponse: string,
  repairRound: number,
): string {
  const budget = AGENT_WORK_ORDER_TASK_MAX_CHARS;
  const findingsBudget = Math.max(1200, budget - originalTask.length - 400);
  const findings =
    reviewResponse.length > findingsBudget
      ? `${reviewResponse.slice(0, findingsBudget - 1)}…`
      : reviewResponse;
  return [
    `返修（第 ${repairRound} 轮）：你此前交付的成果未通过评审，请按下方意见修复后重新交付。`,
    "── 原任务 ──",
    originalTask,
    "── 评审意见 ──",
    findings,
  ].join("\n");
}

/**
 * 返修判定（纯函数）：评审回执 verdict=不通过 且 开关开 且 被审工单可寻址 且
 * 没有在飞的返修单 → 派返修；repairRound 已到上限 → stop（escalated）。
 */
export function computeAutoRepairAction(
  rows: readonly TeamBoardInputRow[],
  input: {
    reviewsWorkOrderId: string | undefined;
    reviewResponse: string | undefined;
    reviewBatchId: string | undefined;
  },
): AutoRepairAction | undefined {
  if (parseReviewVerdict(input.reviewResponse) !== "fail") return undefined;
  const reviewsId = input.reviewsWorkOrderId;
  if (!reviewsId) return undefined;
  const targetRow = rows.find(
    (row) => row.kind === "agentWorkOrderDispatch" && row.payload.workOrderId === reviewsId,
  );
  if (!targetRow) return undefined;
  const targetEnvelope = envelopeOf(targetRow);
  const originalTask = readString(targetEnvelope?.task);
  const agentName = readString(targetRow.payload.agentName);
  if (!originalTask || !agentName) return undefined;
  const originalBatchId = readString(targetEnvelope?.batchId);
  if (!isTeamAutoFlowEnabled(rows, [input.reviewBatchId, originalBatchId])) return undefined;
  // 在飞的返修单挡住重复触发（同张被审单同时只允许一个返修在途）。
  if (
    rows.some(
      (row) =>
        row.kind === "agentWorkOrderDispatch" &&
        row.status === "admitted" &&
        envelopeOf(row)?.repairOf === reviewsId,
    )
  ) {
    return undefined;
  }
  const repairRound =
    rows.filter(
      (row) =>
        row.kind === "agentWorkOrderDispatch" && envelopeOf(row)?.repairOf === reviewsId,
    ).length + 1;
  if (repairRound > TEAM_BOARD_MAX_REPAIR_ROUNDS) {
    return { kind: "stop", repairRound: repairRound - 1 };
  }
  const agentId = readString(targetRow.payload.agentId);
  return {
    kind: "repair",
    agent: agentId ?? agentName,
    ...(agentId ? { agentId } : {}),
    task: buildRepairTaskText(originalTask, input.reviewResponse ?? "", repairRound),
    ...(originalBatchId ? { batchId: originalBatchId } : {}),
    ...(readString(targetEnvelope?.batchTitle)
      ? { batchTitle: readString(targetEnvelope?.batchTitle) }
      : {}),
    reviewsWorkOrderId: reviewsId,
    repairRound,
  };
}

/** 复审判定（纯函数）：返修单 completed → 找原评审人派复审单（落回评审批次）。 */
export function computeReReviewAction(
  rows: readonly TeamBoardInputRow[],
  input: { repairWorkOrderId: string | undefined },
): { agent: string; agentId?: string; task: string; batchId?: string; batchTitle?: string; reviews: string } | undefined {
  if (!input.repairWorkOrderId) return undefined;
  const repairRow = rows.find(
    (row) =>
      row.kind === "agentWorkOrderDispatch" && row.payload.workOrderId === input.repairWorkOrderId,
  );
  const repairEnvelope = repairRow ? envelopeOf(repairRow) : undefined;
  const originalId = readString(repairEnvelope?.repairOf);
  const repairRound = repairEnvelope?.repairRound;
  if (!originalId || typeof repairRound !== "number") return undefined;
  const reviewerRow = rows.find(
    (row) =>
      row.kind === "agentWorkOrderDispatch" &&
      envelopeOf(row)?.reviews === originalId &&
      envelopeOf(row)?.review === true,
  );
  if (!reviewerRow) return undefined;
  const agentName = readString(reviewerRow.payload.agentName);
  if (!agentName) return undefined;
  const reviewerEnvelope = envelopeOf(reviewerRow);
  const agentId = readString(reviewerRow.payload.agentId);
  return {
    agent: agentId ?? agentName,
    ...(agentId ? { agentId } : {}),
    task: [
      `第 ${repairRound} 轮复审：工单 ${input.repairWorkOrderId} 是针对你此前评审不通过的工单 ${originalId} 的返修成果，请重新评审它。`,
      "正文第一行仍按「评审结论：通过 / 不通过 / 有条件通过」格式给出结论，随后列主要意见。",
    ].join("\n"),
    ...(readString(reviewerEnvelope?.batchId)
      ? { batchId: readString(reviewerEnvelope?.batchId) }
      : {}),
    ...(readString(reviewerEnvelope?.batchTitle)
      ? { batchTitle: readString(reviewerEnvelope?.batchTitle) }
      : {}),
    reviews: input.repairWorkOrderId,
  };
}

/** 同会话串行（并发回执的轮次计数不竞态），仿 batchQcTriggerChains。 */
const schedulerChains = new Map<string, Promise<void>>();

function enqueueOnChain(
  sessionId: string,
  run: () => Promise<void>,
): Promise<void> {
  const previous = schedulerChains.get(sessionId) ?? Promise.resolve();
  const chain = previous.then(run, run);
  schedulerChains.set(sessionId, chain);
  void chain.finally(() => {
    if (schedulerChains.get(sessionId) === chain) schedulerChains.delete(sessionId);
  });
  return chain;
}

/**
 * 回执销账后的调度窗（deliverWorkOrderReceipt 触发）：释放就绪持派单 → 返修判定
 * → 复审判定。任何一步失败只留痕不冒泡——调度是增强，回执本身已安全落地。
 */
export async function runTeamSchedulerAfterReceipt(
  context: ZCodeProtocolAgentServerContext,
  deps: ReceiptRelayDeps,
  initiatorRecord: ZCodeProtocolSessionRecord,
  input: { envelope: AgentWorkOrderEnvelope; outcome: { status: string; response?: string } },
): Promise<void> {
  const store = context.deps.sessionStore;
  const listInputs = store?.listSessionInputs?.bind(store);
  if (!listInputs) return;
  await enqueueOnChain(initiatorRecord.app.sessionId, async () => {
    const rows = (await listInputs({
      sessionID: initiatorRecord.app.sessionId as never,
    })) as unknown as TeamBoardInputRow[];

    // 1) 释放就绪的持派工单（依赖 honored，与开关无关）。
    for (const held of computeReadyHeldWorkOrders(rows)) {
      try {
        if (!held.targetSessionId) continue;
        const targetRecord =
          context.sessions.get(held.targetSessionId) ??
          (await deps.activateSessionRecord(held.targetSessionId));
        await targetRecord.app.runtime.enqueueAgentWorkOrder({
          envelope: held.envelope,
          traceContext: initiatorRecord.traceContext,
          ...(held.modelSelection ? { modelSelection: held.modelSelection } : {}),
        });
        scheduleWorkOrderReceiptRelay(context, deps, {
          targetRecord,
          inputId: `${WORK_ORDER_INPUT_ID_PREFIX}${held.envelope.workOrderId}`,
          envelope: held.envelope,
          agentName: held.agentName,
          ...(held.agentId ? { agentId: held.agentId } : {}),
          ...(held.modelSelection ? { modelSelection: held.modelSelection } : {}),
        });
        const { held: _held, ...payloadWithoutHeld } = held.rowPayload;
        await store?.saveSessionInput?.({
          id: held.rowId,
          sessionID: initiatorRecord.app.sessionId as never,
          kind: "agentWorkOrderDispatch",
          delivery: "queue",
          payload: payloadWithoutHeld as { text: string; [key: string]: unknown },
        });
        context.logger?.info("Held team work order released after dependencies completed", {
          event: "team_scheduler.held_released",
          module: "bootstrap.zcode_protocol",
          sessionId: initiatorRecord.app.sessionId,
          workOrderId: held.workOrderId,
        });
      } catch (error) {
        context.logger?.warn("Failed to release held team work order", {
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "team_scheduler.release_failed",
          module: "bootstrap.zcode_protocol",
          workOrderId: held.workOrderId,
        });
      }
    }

    // 2) 评审不通过 → 自动返修（开关开才有）。
    // 3) 返修完成 → 复审。两者都经派单端口（频控三道闸照常兜底）。
    const port = createProtocolAgentDispatchPort(context, {
      resolveOwnSession: () => initiatorRecord,
      createPersonaSessionRecord:
        deps.createPersonaSessionRecord ??
        (async () => {
          throw new Error("team scheduler requires createPersonaSessionRecord capability");
        }),
      activateSessionRecord: deps.activateSessionRecord,
    });
    try {
      if (input.envelope.review === true && input.outcome.status === "completed") {
        const action = computeAutoRepairAction(rows, {
          reviewsWorkOrderId: input.envelope.reviews,
          reviewResponse: input.outcome.response,
          reviewBatchId: input.envelope.batchId,
        });
        if (action?.kind === "repair") {
          await port.dispatch({
            agent: action.agent,
            task: action.task,
            ...(action.batchId ? { batchId: action.batchId } : {}),
            ...(action.batchTitle ? { batchTitle: action.batchTitle } : {}),
            repairOf: action.reviewsWorkOrderId,
            repairRound: action.repairRound,
            sourceSessionId: initiatorRecord.app.sessionId,
          });
          context.logger?.info("Auto repair dispatched after failed review verdict", {
            event: "team_scheduler.auto_repair",
            module: "bootstrap.zcode_protocol",
            repairRound: action.repairRound,
            reviewsWorkOrderId: action.reviewsWorkOrderId,
            sessionId: initiatorRecord.app.sessionId,
          });
        } else if (action?.kind === "stop") {
          context.logger?.info("Auto repair loop hit the round cap; escalating to user", {
            event: "team_scheduler.auto_repair_escalated",
            module: "bootstrap.zcode_protocol",
            repairRound: action.repairRound,
            reviewsWorkOrderId: input.envelope.reviews,
            sessionId: initiatorRecord.app.sessionId,
          });
        }
      }
      if (input.envelope.repairOf !== undefined && input.outcome.status === "completed") {
        const reReview = computeReReviewAction(rows, {
          repairWorkOrderId: input.envelope.workOrderId,
        });
        if (reReview) {
          await port.dispatch({
            agent: reReview.agent,
            task: reReview.task,
            ...(reReview.batchId ? { batchId: reReview.batchId } : {}),
            ...(reReview.batchTitle ? { batchTitle: reReview.batchTitle } : {}),
            review: true,
            reviews: reReview.reviews,
            sourceSessionId: initiatorRecord.app.sessionId,
          });
          context.logger?.info("Re-review dispatched after repair completion", {
            event: "team_scheduler.re_review",
            module: "bootstrap.zcode_protocol",
            repairWorkOrderId: input.envelope.workOrderId,
            sessionId: initiatorRecord.app.sessionId,
          });
        }
      }
    } catch (error) {
      context.logger?.warn("Team scheduler dispatch failed", {
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "team_scheduler.dispatch_failed",
        module: "bootstrap.zcode_protocol",
        sessionId: initiatorRecord.app.sessionId,
      });
    }
  });
}
