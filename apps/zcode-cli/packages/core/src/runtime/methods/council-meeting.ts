// ============================================================
// 圆桌会（真会议）编排状态机：召集、回执驱动的轮次推进、收票重询、
// 主席合成轮与控场（暂停/插话）。批改线：批次质检（work-order-batch-qc.ts）
// 的闸门台账行 + 串行链 + 独立成轮模式在这里逐条镜像，但收票用内核
// parseCouncilVerdictLine/countCouncilVotes/resolveCouncilDecision 代码算——
// 散文不算票，主席不代笔。
// ============================================================
// 全链路：CouncilConvene 工具（用户批准后）→ conveneCouncilMeeting（本文件，
// 会议行先落账再逐席经 AgentDispatchPort 派回员工会话）→ 席位回执由 bootstrap
// deliverWorkOrderReceipt 投回 → 并列触发 maybeAdvanceCouncilRound（本文件）：
// 本轮全部席位回执到齐 → 解析裁定行；缺行/畸形对该席补交一次（信封指名只补
// 裁定行）；仍缺按弃权如实标注 → resolveCouncilDecision：首轮全票同立场早退，
// 否则第 2 轮（匿名质询材料 + 抗从众条款）→ 终轮过半判定 → 主席合成轮出决议卡。
// 暂停冻结下一轮派单与推进（进行中的席位发言自然跑完）；插话进下一轮信封。

import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { Logger, SessionId, SessionStorePort, TraceContext } from "../deps.js";
import type {
  AgentDispatchPort,
  AgentDispatchResult,
  BackgroundResultOriginMeta,
  CouncilMeetingKind,
  CouncilMeetingRound,
  CouncilMeetingSeat,
  CouncilMeetingState,
  CouncilSeatLensId,
} from "@zcode/contracts";
import { uuidv7 } from "@zcode/shared";
import {
  createRuntimeCommandId,
  type CouncilModerationRuntimeCommand,
} from "../command-queue.js";
import { runtimeInputMetadata } from "../../agent/runtime-input-presentation.js";
import {
  buildCouncilSeatPlan,
  countCouncilVotes,
  parseCouncilVerdictLine,
  resolveCouncilDecision,
  type CouncilOutcome,
  type CouncilTally,
  type CouncilVerdict,
} from "../../subagent/council.js";
import { WORK_ORDER_RESTRICTED_TOOL_NAMES } from "../../subagent/work-order.js";
import {
  anonymizeCouncilStatements,
  buildCouncilMeetingTitle,
  buildCouncilModerationTaskText,
  buildCouncilRequeryTaskText,
  buildCouncilSeatTaskText,
  councilMeetingLedgerId,
  councilRoundLedgerId,
  councilSeatRequeryLedgerId,
  type CouncilMeetingFailedDispatch,
  type CouncilMeetingRecordPayload,
  type CouncilSeatStatement,
  type CouncilVoteRecord,
} from "../../subagent/council-meeting.js";
import type { WorkOrderReceiptOutcome } from "../../subagent/work-order.js";
import type { AgentRuntimeInternal } from "../internal.js";
import { isStaleBranchRuntimeCommand } from "./runtime-command-generation.js";
import { beginForegroundExecution, finishForegroundExecution } from "./runtime-command-queue.js";

// ── 串行链：同场会议的全部编排动作互斥（防双收票/双主席轮）──────────

/**
 * 同批并发交活时最后两张回执的触发可能交错（batchQc 同款竞态）：同场会议的
 * 推进/暂停/插话按 `${sessionId}:${councilId}` 串行。Map 只在本进程内记账；
 * 跨进程/重启由台账行的确定性 id 兜住（闸门行在册即跳过）。
 */
const councilMeetingChains = new Map<string, Promise<unknown>>();

async function runInCouncilChain<T>(
  sessionId: string,
  councilId: string,
  work: () => Promise<T>,
): Promise<T> {
  const chainKey = `${sessionId}:${councilId}`;
  const previous = councilMeetingChains.get(chainKey) ?? Promise.resolve();
  const chain = previous.then(work, work);
  councilMeetingChains.set(chainKey, chain);
  try {
    return await chain;
  } finally {
    if (councilMeetingChains.get(chainKey) === chain) {
      councilMeetingChains.delete(chainKey);
    }
  }
}

// ── 台账行防御性读取（payload 按 unknown 收窄，畸形行不猜）──────────

type LedgerRowLike = {
  id: string;
  kind: string;
  status: string;
  admittedSequence: number;
  payload: { text: string; [key: string]: unknown };
};

function readEnvelope(row: LedgerRowLike): Record<string, unknown> | undefined {
  const envelope = row.payload?.envelope;
  if (typeof envelope !== "object" || envelope === null || Array.isArray(envelope)) return undefined;
  return envelope as Record<string, unknown>;
}

function readMeetingPayload(row: LedgerRowLike | undefined): CouncilMeetingRecordPayload | undefined {
  if (!row) return undefined;
  const payload = row.payload;
  if (typeof payload !== "object" || payload === null) return undefined;
  const council = (payload as Record<string, unknown>).council;
  if (typeof council !== "object" || council === null || Array.isArray(council)) return undefined;
  const record = council as Record<string, unknown>;
  if (
    typeof record.councilId !== "string" ||
    typeof record.status !== "string" ||
    !Array.isArray(record.seats)
  ) {
    return undefined;
  }
  return payload as unknown as CouncilMeetingRecordPayload;
}

function readReceiptOutcome(row: LedgerRowLike): WorkOrderReceiptOutcome | undefined {
  const outcome = row.payload?.outcome;
  if (typeof outcome !== "object" || outcome === null || Array.isArray(outcome)) return undefined;
  const record = outcome as Record<string, unknown>;
  if (record.status !== "completed" && record.status !== "cancelled" && record.status !== "failed") {
    return undefined;
  }
  return outcome as WorkOrderReceiptOutcome;
}

/** 席位派单行的信封必须带全圆桌会五参，缺任一即不认（口径与端口铸造侧对齐）。 */
function envelopeSeatsRound(
  envelope: Record<string, unknown> | undefined,
  councilId: string,
  round: CouncilMeetingRound,
): number | undefined {
  if (!envelope) return undefined;
  if (envelope.councilId !== councilId) return undefined;
  if (envelope.councilRound !== round) return undefined;
  const seatIndex = envelope.councilSeatIndex;
  return typeof seatIndex === "number" && Number.isInteger(seatIndex) && seatIndex >= 0
    ? seatIndex
    : undefined;
}

async function saveMeetingRow(
  store: SessionStorePort,
  sessionId: SessionId,
  payload: CouncilMeetingRecordPayload,
  text: string,
): Promise<void> {
  await store.saveSessionInput?.({
    id: councilMeetingLedgerId(sessionId, payload.council.councilId),
    sessionID: sessionId,
    kind: "councilMeeting",
    delivery: "queue",
    payload: { text, ...payload },
  });
}

// ── 召集（CouncilConvene 工具批准后的一次性入口）────────────────────

export interface CouncilConveneDeps {
  sessionId: SessionId;
  sessionStore?: SessionStorePort;
  agentDispatchPort?: AgentDispatchPort;
  logger?: Logger;
  traceContext?: TraceContext;
}

export interface ConveneCouncilMeetingInput {
  councilId: string;
  kind: CouncilMeetingKind;
  /** 议题原话（老板要审的方案/成果，随信封下发）。 */
  topic: string;
  /** 建议席位（员工名/工号，按提议顺序；座次由内核 buildCouncilSeatPlan 铸）。 */
  seats: readonly string[];
  materials?: string;
  title?: string;
}

export interface ConveneCouncilMeetingResult {
  councilId: string;
  status: "running" | "cancelled";
  seats: readonly CouncilMeetingSeat[];
  dispatchedSeats: number;
  failedSeats: { agentName: string; reason: string }[];
}

/** 经端口派一个席位单（代码直派后门：目标解析/台账/回执订阅全在端口里）。 */
async function dispatchCouncilSeat(
  port: AgentDispatchPort,
  input: {
    agentName: string;
    task: string;
    councilId: string;
    kind: CouncilMeetingKind;
    round: CouncilMeetingRound;
    seatIndex: number;
    lens: CouncilSeatLensId;
    sourceSessionId: string;
  },
): Promise<AgentDispatchResult> {
  return await port.dispatch({
    agent: input.agentName,
    task: input.task,
    councilId: input.councilId,
    councilKind: input.kind,
    councilRound: input.round,
    councilSeatIndex: input.seatIndex,
    councilSeatLens: input.lens,
    sourceSessionId: input.sourceSessionId,
  });
}

/**
 * 召集一场圆桌会：会议行先落账（收票推进的对账锚点，先于任何派单——快的席位
 * 回执可能先到），再按座次顺序逐席经端口派单。个别席位派单失败按缺席弃权如实
 * 标注、其余席位照常开会（合议先例：缺席如实标注）；全军覆没 → 会议取消并抛错
 * （错误信息带逐席原因，召集模型修正名册后重新提议）。
 */
export async function conveneCouncilMeeting(
  deps: CouncilConveneDeps,
  input: ConveneCouncilMeetingInput,
): Promise<ConveneCouncilMeetingResult> {
  const seatPlan = buildCouncilSeatPlan(input.seats);
  if (seatPlan.length === 0) {
    throw new Error("CouncilConvene requires at least one non-empty seat name");
  }
  const store = deps.sessionStore;
  if (!store?.saveSessionInput) {
    throw new Error("Session store is not configured; cannot convene a council meeting");
  }
  return await runInCouncilChain(deps.sessionId, input.councilId, async () => {
    const seats: CouncilMeetingSeat[] = seatPlan.map((seat) => ({
      index: seat.index,
      agentName: seat.agentName,
      lens: seat.lens.id,
    }));
    const council: CouncilMeetingState = {
      councilId: input.councilId,
      kind: input.kind,
      status: "running",
      round: 1,
      phase: "seating",
      seats,
      motion: input.topic,
      convenedBySessionId: deps.sessionId,
    };
    const payload: CouncilMeetingRecordPayload = {
      council,
      ...(input.title?.trim() ? { title: input.title.trim() } : {}),
      ...(input.materials?.trim() ? { materials: input.materials.trim() } : {}),
    };
    // 闸门会议行先落（await 到底）：落不进账本就不派单——无锚点的席位单回执
    // 到了也推进不了，钱花了会开不成（与 batchQc 落闸先于开轮同一纪律）。
    try {
      await saveMeetingRow(store, deps.sessionId, payload, input.topic);
    } catch (error) {
      throw new Error(
        `Failed to save the council meeting record; meeting not convened: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
    const port = deps.agentDispatchPort;
    const failed: { agentName: string; seatIndex: number; reason: string }[] = [];
    for (const seat of seatPlan) {
      if (!port) {
        failed.push({
          agentName: seat.agentName,
          seatIndex: seat.index,
          reason: "AgentDispatchPort is not configured",
        });
        continue;
      }
      try {
        await dispatchCouncilSeat(port, {
          agentName: seat.agentName,
          task: buildCouncilSeatTaskText({
            kind: input.kind,
            round: 1,
            seatIndex: seat.index,
            lens: seat.lens.id,
            seatCount: seatPlan.length,
            motion: input.topic,
            ...(input.materials?.trim() ? { materials: input.materials.trim() } : {}),
          }),
          councilId: input.councilId,
          kind: input.kind,
          round: 1,
          seatIndex: seat.index,
          lens: seat.lens.id,
          sourceSessionId: deps.sessionId,
        });
      } catch (error) {
        const reason = error instanceof Error ? error.message : String(error);
        failed.push({ agentName: seat.agentName, seatIndex: seat.index, reason });
        deps.logger?.warn("Council seat dispatch failed; seat marked absent", {
          ...(deps.traceContext ? traceContextToLogContext(deps.traceContext) : {}),
          councilId: input.councilId,
          event: "council_meeting.seat_dispatch_failed",
          module: "core.runtime",
          seatIndex: seat.index,
          sessionId: deps.sessionId,
        });
      }
    }
    if (failed.length > 0) {
      payload.failedDispatches = failed.map((entry) => ({
        round: 1,
        seatIndex: entry.seatIndex,
        ...(entry.reason ? { error: entry.reason } : {}),
      }));
    }
    if (failed.length === seatPlan.length) {
      // 全军覆没：如实取消，让召集方拿到失败原因修正名册（不空转一场哑会）。
      payload.council = { ...council, status: "cancelled" };
    }
    try {
      await saveMeetingRow(store, deps.sessionId, payload, input.topic);
    } catch {
      // 派单已投出，会议行的缺席标注落不进也只影响标注精度；回执推进不受影响。
    }
    return {
      councilId: input.councilId,
      status: payload.council.status === "cancelled" ? "cancelled" : "running",
      seats,
      dispatchedSeats: seatPlan.length - failed.length,
      failedSeats: failed.map((entry) => ({ agentName: entry.agentName, reason: entry.reason })),
    };
  });
}

// ── 回执驱动的轮次推进 ──────────────────────────────────────────────

export interface MaybeAdvanceCouncilRoundInput {
  councilId: string;
  /** 缺席 = 会议当前轮。 */
  round?: CouncilMeetingRound;
  /** 触发本次推进的回执工单 id（本单 outcome 可能尚未落账，作为台账的即时回退）。 */
  workOrderId?: string;
  outcome?: WorkOrderReceiptOutcome;
  traceContext?: TraceContext;
}

/**
 * 公共触发（AgentRuntime 面上；bootstrap 回执投递与解除暂停共用）：本轮全部
 * 席位回执到齐 → 收票推进。幂等安全的自查入口——判定不满足就是空转一次查询。
 */
export async function maybeAdvanceCouncilRound(
  this: AgentRuntimeInternal,
  input: MaybeAdvanceCouncilRoundInput,
): Promise<void> {
  await runInCouncilChain(this.sessionId, input.councilId, () =>
    advanceCouncilRoundLocked.call(this, input),
  );
}

async function advanceCouncilRoundLocked(
  this: AgentRuntimeInternal,
  input: MaybeAdvanceCouncilRoundInput,
): Promise<void> {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  if (this.shuttingDown) {
    this.logger?.info("Dropped council round advance during session shutdown", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      event: "council_meeting.advance.shutdown_dropped",
      module: "core.runtime",
      sessionId: this.sessionId,
    });
    return;
  }
  const store = this.sessionStore;
  if (!store?.listSessionInputs || !store?.saveSessionInput) return;
  const rows: LedgerRowLike[] = await store.listSessionInputs({ sessionID: this.sessionId });
  const meetingRow = rows.find((row) => row.id === councilMeetingLedgerId(this.sessionId, input.councilId));
  const meetingPayload = readMeetingPayload(meetingRow);
  if (!meetingPayload) {
    this.logger?.warn("Council advance found no meeting record; ignoring trigger", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      event: "council_meeting.advance.no_meeting",
      module: "core.runtime",
      sessionId: this.sessionId,
    });
    return;
  }
  const meeting = meetingPayload.council;
  if (meeting.status !== "running") return;
  if (meetingPayload.paused) {
    this.logger?.info("Council meeting is paused; advance frozen", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      event: "council_meeting.advance.paused",
      module: "core.runtime",
      sessionId: this.sessionId,
    });
    return;
  }
  const round = input.round ?? meeting.round;
  // 闸门：本轮收票决议行已在（任意状态）→ 这轮已经收过票，绝不二算。
  if (rows.some((row) => row.id === councilRoundLedgerId(this.sessionId, input.councilId, round))) {
    return;
  }
  const seats = [...meeting.seats].sort((a, b) => a.index - b.index);
  if (seats.length === 0) return;

  const dispatchRows = rows
    .filter((row) => {
      if (row.kind !== "agentWorkOrderDispatch") return false;
      return envelopeSeatsRound(readEnvelope(row), input.councilId, round) !== undefined;
    })
    // 原始单/补交单按入账序区分：补交永远晚于原始单派出。
    .sort((a, b) => a.admittedSequence - b.admittedSequence);
  // 席位单还有未交活的（派单行仍 admitted）→ 本轮没收齐，等下一张回执再推进。
  if (dispatchRows.some((row) => row.status === "admitted")) return;

  const receipts = rows
    .filter((row) => row.kind === "agentWorkOrderReceipt")
    .sort((a, b) => a.admittedSequence - b.admittedSequence);
  const outcomeByWorkOrderId = new Map<string, WorkOrderReceiptOutcome>();
  for (const row of receipts) {
    const workOrderId = row.payload?.workOrderId;
    const outcome = readReceiptOutcome(row);
    if (typeof workOrderId === "string" && outcome) {
      // 同工单多行（resume 重投合成）取最后一条：最新终态权威。
      outcomeByWorkOrderId.set(workOrderId, outcome);
    }
  }
  const requeryRows = new Map<number, LedgerRowLike>();
  for (const row of rows) {
    if (row.kind !== "councilSeatRequery") continue;
    const payload = row.payload as Record<string, unknown>;
    if (payload.councilId !== input.councilId || payload.round !== round) continue;
    const seatIndex = payload.seatIndex;
    if (typeof seatIndex === "number") requeryRows.set(seatIndex, row);
  }
  const failedSeats = new Set(
    (meetingPayload.failedDispatches ?? [])
      .filter((failure) => failure.round === round)
      .map((failure) => failure.seatIndex),
  );

  interface SeatVerdict {
    vote: CouncilVoteRecord;
    statement?: CouncilSeatStatement;
  }
  const seatVerdicts: SeatVerdict[] = [];
  let pendingRequery = false;
  for (const seat of seats) {
    const seatRows = dispatchRows.filter(
      (row) => envelopeSeatsRound(readEnvelope(row), input.councilId, round) === seat.index,
    );
    const requeryRow = requeryRows.get(seat.index);
    const requeryPayload = requeryRow
      ? (requeryRow.payload as Record<string, unknown>)
      : undefined;
    // 补交已派出（在飞/已完成）的证据 = 闸行 **或** 本席多出来的派单行：派单行
    // 由端口在派单返回前落账（bootstrap agent-dispatch-port），闸行 best-effort
    // 写失败时派单行仍是「补交已派出」的在账凭据——据它不再重发，杜绝「无闸门
    // 的补交无限重发模型调用」。
    const requeryDispatchRow = seatRows.length > 1 ? seatRows[seatRows.length - 1] : undefined;
    const requeryWorkOrderId =
      typeof requeryPayload?.requeryWorkOrderId === "string"
        ? requeryPayload.requeryWorkOrderId
        : requeryDispatchRow &&
            typeof readEnvelope(requeryDispatchRow)?.workOrderId === "string"
          ? (readEnvelope(requeryDispatchRow)!.workOrderId as string)
          : undefined;
    // 原始席位单 = 本席本轮最早的派单行（补交单永远晚于原始单入账——先派单后
    // 落闸的时序再也冒充不了原始单，主席材料不会混进裸裁定行）。
    const originalRow = seatRows[0];
    const originalWorkOrderId =
      originalRow && typeof readEnvelope(originalRow)?.workOrderId === "string"
        ? (readEnvelope(originalRow)!.workOrderId as string)
        : undefined;

    const outcomeOf = (workOrderId: string | undefined): WorkOrderReceiptOutcome | undefined => {
      if (!workOrderId) return undefined;
      if (input.workOrderId === workOrderId && input.outcome) return input.outcome;
      return outcomeByWorkOrderId.get(workOrderId);
    };

    if (!originalRow || !originalWorkOrderId) {
      if (failedSeats.has(seat.index)) {
        // 派单失败的缺席席：如实按弃权计（合议先例——缺席标注，不空等）。
        seatVerdicts.push({
          vote: {
            index: seat.index,
            agentName: seat.agentName,
            lens: seat.lens,
            stance: "abstain",
            veto: false,
            source: "dispatch_failed",
          },
        });
        continue;
      }
      // 台账既无派单行又无缺席标注：本轮没收齐（不应发生），不推进不猜测。
      return;
    }
    const originalOutcome = outcomeOf(originalWorkOrderId);
    if (!originalOutcome) return;
    if (originalOutcome.status !== "completed") {
      // 交活失败/被中断：bootstrap 已自动重试过一次，无正文可评——缺席弃权。
      seatVerdicts.push({
        vote: {
          index: seat.index,
          agentName: seat.agentName,
          lens: seat.lens,
          stance: "abstain",
          veto: false,
          source: "no_response",
          workOrderId: originalWorkOrderId,
        },
      });
      continue;
    }
    const originalVerdict = parseCouncilVerdictLine(originalOutcome.response ?? "");
    if (originalVerdict) {
      seatVerdicts.push({
        vote: {
          index: seat.index,
          agentName: seat.agentName,
          lens: seat.lens,
          stance: originalVerdict.stance,
          confidence: originalVerdict.confidence,
          veto: originalVerdict.veto,
          source: "verdict_line",
          workOrderId: originalWorkOrderId,
        },
        statement: { index: seat.index, statement: originalOutcome.response ?? "" },
      });
      continue;
    }
    // 缺行/畸形 → 对该席重询一次（信封指名只补裁定行）；补交已派出（闸行或
    // 补交派单行在账）即不再重询。
    if (!requeryRow && requeryDispatchRow === undefined) {
      const requeryId = councilSeatRequeryLedgerId(this.sessionId, input.councilId, round, seat.index);
      let requeryPayload: Record<string, unknown>;
      if (!this.agentDispatchPort) {
        requeryPayload = {
          councilId: input.councilId,
          round,
          seatIndex: seat.index,
          originalWorkOrderId,
          dispatchFailed: true,
          error: "AgentDispatchPort is not configured",
        };
      } else {
        try {
          const result = await dispatchCouncilSeat(this.agentDispatchPort, {
            agentName: seat.agentName,
            task: buildCouncilRequeryTaskText({ round, seatIndex: seat.index }),
            councilId: input.councilId,
            kind: meeting.kind,
            round,
            seatIndex: seat.index,
            lens: seat.lens,
            sourceSessionId: this.sessionId,
          });
          requeryPayload = {
            councilId: input.councilId,
            round,
            seatIndex: seat.index,
            originalWorkOrderId,
            requeryWorkOrderId: result.workOrderId,
          };
        } catch (error) {
          requeryPayload = {
            councilId: input.councilId,
            round,
            seatIndex: seat.index,
            originalWorkOrderId,
            dispatchFailed: true,
            error: error instanceof Error ? error.message : String(error),
          };
        }
      }
      // 补交闸行 best-effort 落账：写不进就放弃本轮推进——补交派单行已在账，
      // 回执一到照常收票；两处都写不进才会重发（代价是一次快速失败的派单重试）。
      try {
        await store.saveSessionInput({
          id: requeryId,
          sessionID: this.sessionId,
          kind: "councilSeatRequery",
          delivery: "queue",
          payload: { text: `council requery ${round} seat ${seat.index}`, ...requeryPayload },
        });
      } catch (error) {
        this.logger?.warn("Failed to save council requery gate row; aborting advance", {
          ...traceContextToLogContext(traceContext),
          councilId: input.councilId,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "council_meeting.requery_gate_save_failed",
          module: "core.runtime",
          status: "failed",
        });
        return;
      }
      pendingRequery = true;
      continue;
    }
    if (requeryPayload?.dispatchFailed === true || !requeryWorkOrderId) {
      seatVerdicts.push({
        vote: {
          index: seat.index,
          agentName: seat.agentName,
          lens: seat.lens,
          stance: "abstain",
          veto: false,
          source: "missing_verdict",
          workOrderId: originalWorkOrderId,
        },
        statement: { index: seat.index, statement: originalOutcome.response ?? "" },
      });
      continue;
    }
    const requeryOutcome = outcomeOf(requeryWorkOrderId);
    if (!requeryOutcome) return; // 补交单还没交活，本轮不收票。
    const requeryVerdict =
      requeryOutcome.status === "completed"
        ? parseCouncilVerdictLine(requeryOutcome.response ?? "")
        : undefined;
    if (requeryVerdict) {
      seatVerdicts.push({
        vote: {
          index: seat.index,
          agentName: seat.agentName,
          lens: seat.lens,
          stance: requeryVerdict.stance,
          confidence: requeryVerdict.confidence,
          veto: requeryVerdict.veto,
          source: "requery_verdict_line",
          workOrderId: requeryWorkOrderId,
        },
        statement: { index: seat.index, statement: originalOutcome.response ?? "" },
      });
    } else {
      // 重询一次仍缺：按弃权计并在台账如实标注（发言保留为材料，票不算）。
      seatVerdicts.push({
        vote: {
          index: seat.index,
          agentName: seat.agentName,
          lens: seat.lens,
          stance: "abstain",
          veto: false,
          source: "missing_verdict",
          workOrderId: originalWorkOrderId,
        },
        statement: { index: seat.index, statement: originalOutcome.response ?? "" },
      });
    }
  }
  if (pendingRequery) return; // 本轮有补交在飞，等补交回执再收票。

  // 全席到齐 → 代码算票（散文永远不计票）。
  const votes: CouncilVoteRecord[] = seatVerdicts.map((entry) => entry.vote);
  const verdicts: CouncilVerdict[] = votes.map((vote) => ({
    stance: vote.stance,
    confidence: vote.confidence ?? "low",
    veto: vote.veto,
  }));
  const tally = countCouncilVotes(verdicts);
  const decision = resolveCouncilDecision(tally, round);
  const statements = seatVerdicts
    .map((entry) => entry.statement)
    .filter((statement): statement is CouncilSeatStatement => statement !== undefined);

  // 决议落库（本轮收票闸门行 = 票数明细 + 各席裁定；确定性 id，upsert 幂等）。
  // 先落闸再派单/开主席轮：写不进就不推进（宁缺毋滥，batchQc 同纪律）。
  try {
    await store.saveSessionInput({
      id: councilRoundLedgerId(this.sessionId, input.councilId, round),
      sessionID: this.sessionId,
      kind: "councilRound",
      delivery: "queue",
      payload: {
        text: `council ${input.councilId} round ${round}: ${decision.outcome}`,
        councilId: input.councilId,
        round,
        outcome: decision.outcome,
        tally,
        votes,
      },
    });
  } catch (error) {
    this.logger?.warn("Failed to save council round gate row; aborting advance", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "council_meeting.round_gate_save_failed",
      module: "core.runtime",
      status: "failed",
    });
    return;
  }

  if (decision.outcome === "deliberate" && decision.nextRound === 2) {
    if (votes.every((vote) => vote.source === "dispatch_failed")) {
      // 全席连派单都没成功：再派一轮必然重演，直接按未决收口（不空转一轮哑会）。
      await finalizeCouncilMeeting.call(this, {
        councilId: input.councilId,
        meetingPayload,
        round,
        outcome: "deadlocked",
        tally,
        votes,
        statements,
        traceContext,
      });
      return;
    }
    await dispatchCouncilRound2.call(this, {
      councilId: input.councilId,
      meetingPayload,
      round1Votes: votes,
      round1Statements: statements,
      traceContext,
    });
    return;
  }
  await finalizeCouncilMeeting.call(this, {
    councilId: input.councilId,
    meetingPayload,
    round,
    outcome: decision.outcome,
    tally,
    votes,
    statements,
    traceContext,
  });
}

/** 第 2 轮（交叉质询）：匿名材料 + 抗从众条款 + 插话点名，逐席经端口直派。 */
async function dispatchCouncilRound2(
  this: AgentRuntimeInternal,
  input: {
    councilId: string;
    meetingPayload: CouncilMeetingRecordPayload;
    round1Votes: readonly CouncilVoteRecord[];
    round1Statements: readonly CouncilSeatStatement[];
    traceContext: TraceContext;
  },
): Promise<void> {
  const meeting = input.meetingPayload.council;
  const seats = [...meeting.seats].sort((a, b) => a.index - b.index);
  const peerStatements = anonymizeCouncilStatements(input.round1Statements);
  const interjections = input.meetingPayload.interjections ?? [];
  const nextFailed: CouncilMeetingFailedDispatch[] = [];
  if (!this.agentDispatchPort) {
    // 无端口就不该有圆桌会（召集门在工具注册层）；如实按未决收口。
    this.logger?.warn("Council round 2 skipped: no dispatch port", {
      ...traceContextToLogContext(input.traceContext),
      councilId: input.councilId,
      event: "council_meeting.round2.no_port",
      module: "core.runtime",
      sessionId: this.sessionId,
    });
    return;
  }
  for (const seat of seats) {
    const visibleInterjections = interjections
      .filter(
        (item) =>
          !item.targetSeatIndexes || item.targetSeatIndexes.length === 0
            ? true
            : item.targetSeatIndexes.includes(seat.index),
      )
      .map((item) => ({
        text: item.text,
        targeted: Boolean(item.targetSeatIndexes && item.targetSeatIndexes.length > 0),
      }));
    try {
      await dispatchCouncilSeat(this.agentDispatchPort, {
        agentName: seat.agentName,
        task: buildCouncilSeatTaskText({
          kind: meeting.kind,
          round: 2,
          seatIndex: seat.index,
          lens: seat.lens,
          seatCount: seats.length,
          motion: meeting.motion ?? "",
          ...(input.meetingPayload.materials ? { materials: input.meetingPayload.materials } : {}),
          ...(peerStatements.length > 0 ? { peerStatements } : {}),
          ...(visibleInterjections.length > 0 ? { interjections: visibleInterjections } : {}),
        }),
        councilId: input.councilId,
        kind: meeting.kind,
        round: 2,
        seatIndex: seat.index,
        lens: seat.lens,
        sourceSessionId: this.sessionId,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      nextFailed.push({ round: 2, seatIndex: seat.index, error: message });
      this.logger?.warn("Council round-2 seat dispatch failed; seat marked absent", {
        ...traceContextToLogContext(input.traceContext),
        councilId: input.councilId,
        errorMessage: message,
        event: "council_meeting.round2_seat_dispatch_failed",
        module: "core.runtime",
        seatIndex: seat.index,
        sessionId: this.sessionId,
      });
    }
  }
  if (nextFailed.length === seats.length) {
    // 全席派单失败：质询轮开不成，按未决收口（票仍是第 1 轮的票）。
    await finalizeCouncilMeeting.call(this, {
      councilId: input.councilId,
      meetingPayload: input.meetingPayload,
      round: 1,
      outcome: "deadlocked",
      tally: countCouncilVotes(
        input.round1Votes.map((vote) => ({
          stance: vote.stance,
          confidence: vote.confidence ?? "low",
          veto: vote.veto,
        })),
      ),
      votes: [...input.round1Votes],
      statements: [...input.round1Statements],
      traceContext: input.traceContext,
    });
    return;
  }
  const nextPayload: CouncilMeetingRecordPayload = {
    ...input.meetingPayload,
    council: { ...meeting, round: 2, phase: "seating" },
    ...(nextFailed.length > 0
      ? {
          failedDispatches: [
            ...(input.meetingPayload.failedDispatches ?? []),
            ...nextFailed,
          ],
        }
      : {}),
  };
  try {
    await saveMeetingRow(
      this.sessionStore as SessionStorePort,
      this.sessionId,
      nextPayload,
      nextPayload.council.motion ?? "",
    );
  } catch (error) {
    this.logger?.warn("Failed to persist council meeting round advance", {
      ...traceContextToLogContext(input.traceContext),
      councilId: input.councilId,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "council_meeting.meeting_row_save_failed",
      module: "core.runtime",
      status: "failed",
    });
  }
  this.logger?.info("Council round 2 dispatched: cross-questioning seats", {
    ...traceContextToLogContext(input.traceContext),
    councilId: input.councilId,
    event: "council_meeting.round2_dispatched",
    module: "core.runtime",
    sessionId: this.sessionId,
    seatCount: seats.length,
  });
}

/** 终局：会议状态落终态 + 主席合成轮（一次，仿 batchQc 闸门+独立成轮模式）。 */
async function finalizeCouncilMeeting(
  this: AgentRuntimeInternal,
  input: {
    councilId: string;
    meetingPayload: CouncilMeetingRecordPayload;
    round: CouncilMeetingRound;
    outcome: CouncilOutcome;
    tally: CouncilTally;
    votes: readonly CouncilVoteRecord[];
    statements: readonly CouncilSeatStatement[];
    traceContext: TraceContext;
  },
): Promise<void> {
  const meeting = input.meetingPayload.council;
  const terminalStatus =
    input.outcome === "approved" || input.outcome === "rejected" || input.outcome === "deadlocked"
      ? input.outcome
      : "deadlocked";
  const nextPayload: CouncilMeetingRecordPayload = {
    ...input.meetingPayload,
    council: { ...meeting, status: terminalStatus, phase: "moderation" },
  };
  try {
    await saveMeetingRow(
      this.sessionStore as SessionStorePort,
      this.sessionId,
      nextPayload,
      nextPayload.council.motion ?? "",
    );
  } catch (error) {
    this.logger?.warn("Failed to persist council meeting terminal status", {
      ...traceContextToLogContext(input.traceContext),
      councilId: input.councilId,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "council_meeting.meeting_row_save_failed",
      module: "core.runtime",
      status: "failed",
    });
  }
  await enqueueCouncilModerationTurn.call(this, {
    councilId: input.councilId,
    kind: meeting.kind,
    round: input.round,
    motion: meeting.motion ?? "",
    ...(nextPayload.title ? { title: nextPayload.title } : {}),
    outcome: input.outcome,
    tally: input.tally,
    votes: input.votes,
    statements: input.statements,
    traceContext: input.traceContext,
  });
}

// ── 主席合成轮（每场会议一次）───────────────────────────────────────

interface EnqueueCouncilModerationInput {
  councilId: string;
  kind: CouncilMeetingKind;
  round: CouncilMeetingRound;
  motion: string;
  title?: string;
  outcome: CouncilOutcome;
  tally: CouncilTally;
  votes: readonly CouncilVoteRecord[];
  statements: readonly CouncilSeatStatement[];
  traceContext: TraceContext;
}

async function enqueueCouncilModerationTurn(
  this: AgentRuntimeInternal,
  input: EnqueueCouncilModerationInput,
): Promise<void> {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  if (this.shuttingDown) {
    this.logger?.info("Dropped council moderation turn during session shutdown", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      event: "council_moderation.shutdown_dropped",
      module: "core.runtime",
      sessionId: this.sessionId,
    });
    return;
  }
  const text = buildCouncilModerationTaskText({
    kind: input.kind,
    motion: input.motion,
    round: input.round,
    outcome: input.outcome,
    tally: input.tally,
    votes: input.votes,
    statements: input.statements,
  });
  const originMeta: BackgroundResultOriginMeta = {
    // 主席轮与质检/合议轮同一轮头链（backgroundSource 闭集不动，接线图既定）：
    // UI 靠 councilId/councilPhase 分流圆桌卡，不从 source 反推。
    backgroundSource: "agent_work_order_batch_qc",
    workId: input.councilId,
    title: buildCouncilMeetingTitle(input.kind, input.title),
    councilId: input.councilId,
    councilKind: input.kind,
    councilRound: input.round,
    councilPhase: "moderation",
    // 内核绑定决议随轮头权威下发（contracts CouncilMeetingOutcome）：缺席弃权
    // 收口的会议 UI 复算永远等不齐票，卡片终态以此为准，复算只补中间过程票面。
    ...(input.outcome === "approved" || input.outcome === "rejected" || input.outcome === "deadlocked"
      ? { councilOutcome: input.outcome }
      : {}),
  };
  const command: CouncilModerationRuntimeCommand = {
    branchGeneration: this.branchGeneration,
    createdAt: new Date(),
    id: createRuntimeCommandId(),
    mode: "council-moderation",
    source: "agent_work_order_batch_qc",
    councilId: input.councilId,
    kind: input.kind,
    round: input.round,
    motion: input.motion,
    ...(input.title ? { title: input.title } : {}),
    originMeta,
    priority: "next",
    text,
    traceContext,
  };
  this.enqueueRuntimeCommand(command);
  const admission = this.sessionStore?.saveSessionInput?.({
    id: String(command.id),
    sessionID: this.sessionId,
    kind: "councilModeration",
    delivery: "queue",
    payload: {
      text,
      councilId: input.councilId,
      round: input.round,
      outcome: input.outcome,
      tally: input.tally,
      votes: [...input.votes],
    },
  });
  if (admission) {
    void this.trackResidencyBlockingWork(admission).catch((error) => {
      this.logger?.warn("Failed to admit council moderation turn to ledger", {
        ...traceContextToLogContext(traceContext),
        councilId: input.councilId,
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "session_input.admit_failed",
        module: "core.runtime",
        status: "failed",
      });
    });
  }
  this.logger?.info("Council moderation turn enqueued", {
    ...traceContextToLogContext(traceContext),
    councilId: input.councilId,
    event: "council_moderation.enqueued",
    module: "core.runtime",
    outcome: input.outcome,
    sessionId: this.sessionId,
  });
}

/**
 * 主席合成轮：落库 synthetic notice（provider 可见、UI 不画气泡；轮头卡走
 * backgroundResult 链）→ 以会议身份独立成轮。主席轮禁派单（产出是决议卡不是
 * 工单，toolDisallowlist 掐死循环——与合议轮同一执法）。
 */
export async function runCouncilModerationCommand(
  this: AgentRuntimeInternal,
  command: CouncilModerationRuntimeCommand,
): Promise<void> {
  if (isStaleBranchRuntimeCommand(this, command)) return;
  const foregroundExecution = beginForegroundExecution.call(this, command);
  try {
    const messageID = createMessageId();
    this.messageHistory.addUser(command.text, runtimeInputMetadata("agent_work_order_batch_qc"));
    await this.persistSyntheticUserNoticeForSession({
      messageID,
      metadata: {
        councilId: command.councilId,
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
        this.logger?.warn("Failed to mark council moderation promoted", {
          ...traceContextToLogContext(command.traceContext),
          commandId: command.id,
          councilId: command.councilId,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.promote_mark_failed",
          module: "core.runtime",
          status: "failed",
        });
      });
    this.logger?.info("Council moderation turn started", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      councilId: command.councilId,
      event: "council_moderation.turn_started",
      messageId: messageID,
      module: "core.runtime",
    });
    await this.executeTurnCommand(command.text, undefined, {
      abortSignal: foregroundExecution.controller.signal,
      inputId: uuidv7(),
      inputPresentation: "agent_work_order_batch_qc",
      inputSource: "agent_work_order_batch_qc",
      inputVisibility: "model-only",
      originMeta: command.originMeta,
      backgroundSource: "agent_work_order_batch_qc",
      recordedInputMessageId: messageID,
      skipInputRecord: true,
      skipUserPromptSubmitHooks: true,
      // 主席禁派单：圆桌会的产出是决议卡，决定权在老板（机制层掐死循环）。
      toolDisallowlist: [...WORK_ORDER_RESTRICTED_TOOL_NAMES],
      traceContext: command.traceContext,
    });
  } catch (error) {
    this.logger?.warn("Council moderation turn failed", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      councilId: command.councilId,
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "council_moderation.turn_failed",
      module: "core.runtime",
    });
  } finally {
    finishForegroundExecution.call(this, foregroundExecution);
  }
}

// ── 控场最小集：暂停 / 插话 ─────────────────────────────────────────

export interface SetCouncilMeetingPausedInput {
  councilId: string;
  paused: boolean;
  traceContext?: TraceContext;
}

/**
 * 暂停/恢复：冻结下一轮派单与推进（进行中的席位发言自然跑完——已投出去的
 * 席位单照常回执）；恢复时立即自查推进一次（冻结期间到齐的回执此刻收票）。
 */
export async function setCouncilMeetingPaused(
  this: AgentRuntimeInternal,
  input: SetCouncilMeetingPausedInput,
): Promise<void> {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  await runInCouncilChain(this.sessionId, input.councilId, async () => {
    const store = this.sessionStore;
    if (!store?.listSessionInputs || !store?.saveSessionInput) return;
    const rows: LedgerRowLike[] = await store.listSessionInputs({ sessionID: this.sessionId });
    const meetingRow = rows.find(
      (row) => row.id === councilMeetingLedgerId(this.sessionId, input.councilId),
    );
    const payload = readMeetingPayload(meetingRow);
    if (!payload || !meetingRow) {
      this.logger?.warn("Council pause control found no meeting record", {
        ...traceContextToLogContext(traceContext),
        councilId: input.councilId,
        event: "council_meeting.pause.no_meeting",
        module: "core.runtime",
        sessionId: this.sessionId,
      });
      return;
    }
    if (payload.council.status !== "running") return;
    const next: CouncilMeetingRecordPayload = { ...payload };
    if (input.paused) {
      next.paused = true;
    } else {
      delete next.paused;
    }
    try {
      await saveMeetingRow(store, this.sessionId, next, next.council.motion ?? "");
    } catch (error) {
      this.logger?.warn("Failed to persist council pause flag", {
        ...traceContextToLogContext(traceContext),
        councilId: input.councilId,
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "council_meeting.pause_save_failed",
        module: "core.runtime",
        status: "failed",
      });
      return;
    }
    this.logger?.info("Council meeting pause flag updated", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      event: "council_meeting.pause_updated",
      module: "core.runtime",
      paused: input.paused,
      sessionId: this.sessionId,
    });
  });
  if (!input.paused) {
    await maybeAdvanceCouncilRound.call(this, { councilId: input.councilId, traceContext });
  }
}

export interface AddCouncilInterjectionInput {
  councilId: string;
  text: string;
  /** 点名座位（0 起）；缺席 = 全体席位可见。 */
  targetSeatIndexes?: readonly number[];
  traceContext?: TraceContext;
}

/**
 * 插话：作为材料注入**下一轮**席位信封（未点名全体可见；点名只进被点席位）。
 * 会议未开/已终态时忽略（插话只对还在进行的会议有意义）。
 */
export async function addCouncilInterjection(
  this: AgentRuntimeInternal,
  input: AddCouncilInterjectionInput,
): Promise<void> {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  const text = input.text.trim();
  if (!text) return;
  await runInCouncilChain(this.sessionId, input.councilId, async () => {
    const store = this.sessionStore;
    if (!store?.listSessionInputs || !store?.saveSessionInput) return;
    const rows: LedgerRowLike[] = await store.listSessionInputs({ sessionID: this.sessionId });
    const meetingRow = rows.find(
      (row) => row.id === councilMeetingLedgerId(this.sessionId, input.councilId),
    );
    const payload = readMeetingPayload(meetingRow);
    if (!payload || !meetingRow) {
      this.logger?.warn("Council interjection found no meeting record", {
        ...traceContextToLogContext(traceContext),
        councilId: input.councilId,
        event: "council_meeting.interjection.no_meeting",
        module: "core.runtime",
        sessionId: this.sessionId,
      });
      return;
    }
    if (payload.council.status !== "running") return;
    const targets = (input.targetSeatIndexes ?? []).filter(
      (index) => Number.isInteger(index) && index >= 0,
    );
    const next: CouncilMeetingRecordPayload = {
      ...payload,
      interjections: [
        ...(payload.interjections ?? []),
        {
          id: uuidv7(),
          text,
          ...(targets.length > 0 ? { targetSeatIndexes: targets } : {}),
        },
      ],
    };
    try {
      await saveMeetingRow(store, this.sessionId, next, next.council.motion ?? "");
    } catch (error) {
      this.logger?.warn("Failed to persist council interjection", {
        ...traceContextToLogContext(traceContext),
        councilId: input.councilId,
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "council_meeting.interjection_save_failed",
        module: "core.runtime",
        status: "failed",
      });
      return;
    }
    this.logger?.info("Council interjection recorded", {
      ...traceContextToLogContext(traceContext),
      councilId: input.councilId,
      event: "council_meeting.interjection_recorded",
      module: "core.runtime",
      sessionId: this.sessionId,
      targetedSeats: targets.length,
    });
  });
}
