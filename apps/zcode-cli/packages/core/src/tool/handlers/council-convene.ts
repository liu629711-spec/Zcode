// ============================================================
// CouncilConvene Tool Handler（真会议 2026-10-03）：主智能体提议开圆桌会
// ============================================================
// 一次调用带议题/类型/名册；批准后由代码逐席经派单端口直派（座次由内核
// buildCouncilSeatPlan 铸，模型不传 lens/round）。轮次推进、收票、重询、主席
// 合成轮全在 runtime/methods/council-meeting.ts——工具面只盖「提议开会」这一次
// 批准（会议是一次批准而非 N 次派单批准），且绝不自动开会：needsApproval+
// alwaysAsk 双保险，yolo 也不放行。

import {
  CoreErrorType,
  COUNCIL_CONVENE_TOOL_NAME,
  CouncilConveneInputJsonSchema,
  CouncilConveneInputSchema,
  CouncilConveneOutputJsonSchema,
  CouncilConveneOutputSchema,
  createCoreError,
  type CouncilConveneInput,
  type CouncilConveneOutput,
  type ToolPermissionSpec,
} from "@zcode/contracts";
import { uuidv7 } from "@zcode/shared";
import type { ToolEntry, ToolExecutionContext, ToolHandler } from "../types.js";
import { conveneCouncilMeeting } from "../../runtime/methods/council-meeting.js";

const COUNCIL_TOOL_TIMEOUT_MS = 60_000;
const COUNCIL_MODEL_BYTES = 16_000;

/** 与 AgentDispatch 的 assertNotWorkOrderTurn 同款终审：席位单不能再从工单轮里发。 */
function assertNotWorkOrderTurn(context: ToolExecutionContext): void {
  if (!context.workOrderTurn) return;
  // 工单触发的轮次里再召集会议 = 嵌套派单（每个席位单都是跨会话工单）。
  throw createCoreError(
    CoreErrorType.PermissionDenied,
    "CouncilConvene is not allowed in this turn.",
    {
      context: {
        toolCallId: context.toolCallId,
        toolName: COUNCIL_CONVENE_TOOL_NAME,
      },
      recoverable: false,
      retryable: false,
    },
  );
}

function assertDispatchPort(
  context: ToolExecutionContext,
): asserts context is ToolExecutionContext & {
  agentDispatchPort: NonNullable<ToolExecutionContext["agentDispatchPort"]>;
} {
  if (context.agentDispatchPort) return;
  throw createCoreError(
    CoreErrorType.ConfigurationError,
    "AgentDispatchPort is not configured for CouncilConvene",
    {
      context: {
        toolCallId: context.toolCallId,
        toolName: COUNCIL_CONVENE_TOOL_NAME,
      },
      recoverable: false,
    },
  );
}

const councilConveneHandler: ToolHandler = async (input, context) => {
  assertNotWorkOrderTurn(context);
  const parsed = CouncilConveneInputSchema.parse(input) as CouncilConveneInput;
  assertDispatchPort(context);
  // 席位派单/回执订阅/模型口径（跟随发起会话当前模型）全在派单端口里；本 handler
  // 只做「批准后的召集」这一件事：会议行落账 + 逐席直派。
  const result = await conveneCouncilMeeting(
    {
      sessionId: context.sessionId,
      sessionStore: context.sessionStore,
      agentDispatchPort: context.agentDispatchPort,
      traceContext: context.traceContext,
    },
    {
      councilId: uuidv7(),
      kind: parsed.kind,
      topic: parsed.topic,
      seats: parsed.seats,
      ...(parsed.materials === undefined ? {} : { materials: parsed.materials }),
      ...(parsed.title === undefined ? {} : { title: parsed.title }),
    },
  );
  if (result.status === "cancelled") {
    // 全席派单失败：会议如实取消并带逐席原因——召集模型修正名册后重新提议。
    const reasons = result.failedSeats
      .map((entry) => `${entry.agentName}: ${entry.reason}`)
      .join("; ");
    throw createCoreError(
      CoreErrorType.ToolExecutionFailed,
      `Council was cancelled: every seat failed to dispatch. ${reasons}`,
      {
        context: {
          toolCallId: context.toolCallId,
          toolName: COUNCIL_CONVENE_TOOL_NAME,
        },
        recoverable: true,
      },
    );
  }
  const failedNote =
    result.failedSeats.length > 0
      ? ` Seats that could not be dispatched are marked absent and counted as abstentions: ${result.failedSeats
          .map((entry) => entry.agentName)
          .join(", ")}.`
      : "";
  const message = `Council convened (id ${result.councilId}): ${result.dispatchedSeats} seat(s) dispatched round 1; each seat deliberates independently and a verdict is expected as receipts arrive. Do not write the seats' conclusions yourself - the system tallies their verdict lines and a chair turn will produce the final card.${failedNote}`;
  return {
    councilId: result.councilId,
    kind: parsed.kind,
    topic: parsed.topic,
    round: 1,
    status: result.status,
    seats: result.seats.map((seat) => ({
      index: seat.index,
      agentName: seat.agentName,
      lens: seat.lens,
    })),
    dispatchedSeats: result.dispatchedSeats,
    ...(result.failedSeats.length > 0 ? { failedSeats: result.failedSeats } : {}),
    message,
  } satisfies CouncilConveneOutput;
};

/**
 * 批准弹窗的文案（用户面）：token 警告必须写明——会议是 N 场独立模型轮 × 至多
 * 两轮，消耗大量 token；批准才开，绝不自动开会。
 */
const councilConvenePermission: ToolPermissionSpec = {
  permission: "agent.councilConvene",
  reason:
    "圆桌会将召集 N 位员工各独立发言（至多两轮），每人一轮模型调用，消耗大量 token；批准后系统才开始派单。",
  riskLevel: "medium",
  sideEffectScope: "workspace",
  needsApproval: true,
  // 大动作：一场会议就是十几次模型轮，任何宽松模式都不该无人值守地开。
  alwaysAsk: true,
  patternSources: ["toolName"],
  denyPriority: "beforeAsk",
};

const councilConveneResultBudget = {
  maxInlineBytes: COUNCIL_MODEL_BYTES,
  maxModelBytes: COUNCIL_MODEL_BYTES,
  strategy: "truncate" as const,
  preview: {
    maxBytes: COUNCIL_MODEL_BYTES,
    direction: "head" as const,
  },
};

const councilConveneTimeout = {
  defaultMs: COUNCIL_TOOL_TIMEOUT_MS,
  maxMs: COUNCIL_TOOL_TIMEOUT_MS,
  allowCallOverride: false,
};

export const councilConveneToolEntry: ToolEntry = {
  capability:
    "Convene a roundtable council: several employee agents deliberate as opposing-angle seats and the system tallies a binding verdict",
  metadata: {
    name: COUNCIL_CONVENE_TOOL_NAME,
    // 触发文案随工具描述下发（无 system prompt 改动，评审会触发文案同一先例）。
    // token 警告写明：N 位员工各独立发言（至多两轮），消耗大量 token。
    description:
      "Convene a roundtable council (圆桌会) - a REAL meeting where N employee agents each speak INDEPENDENTLY in their own sessions as opposing-angle seats (impact / requirement intent / edge cases / cost / minimal-change), cross-question each other for at most 2 rounds, and the system tallies their structured verdict lines into a binding decision (approved / rejected / deadlocked) plus a chair's summary card. WARNING: 圆桌会将召集 N 位员工各独立发言（至多两轮），消耗大量 token - one call spawns N independent model turns (plus a second round unless round 1 is unanimous), so never call it proactively or for small asks. ONLY call it when the user explicitly asks for a council/roundtable meeting or a formal multi-agent review/acceptance (e.g. 开个圆桌会 / 让大家开个会评审这个方案 / 召集验收会); the user must approve the convening dialog before any seat is dispatched - if the user declines, do not retry. Pass the user's motion verbatim as topic, the exact agent names to seat (never the agent who did the work being reviewed), and kind=plan for pre-work proposal review or kind=acceptance for post-completion acceptance. While the meeting runs, receipts arrive on their own; the chair turn produces the decision card automatically - do not summarize the meeting yourself and do not dispatch seat orders with AgentDispatch.",
    // 派发规则（对齐稿 §三 同族）：圆桌会是「真会议」，与 AgentDispatch 的散单/
    // 批次派单划界——会议编排（座次/轮次/收票/重询）全是代码驱动，模型只提议。
    modelInstructions: [
      `Trigger ONLY on an explicit user request to convene a meeting (圆桌会/圆桌会议/开会评一评/大家把关/验收会/"have the team review this in a council"). A roundtable is EXPENSIVE: N seats each speak independently (up to 2 rounds), consuming a large amount of tokens - for quick opinions use AgentDispatch instead; for one reviewer use a single dispatch.`,
      "Before calling, confirm the seat list with the user if their request does not name the participants; propose agents that fit the motion and NEVER seat the agent who did the work being reviewed. Seats are workspace agents by exact name - unknown or ambiguous names fail per-seat.",
      "Pass topic in the user's language (the seats deliberate on exactly this text); pass kind=plan when reviewing a proposal BEFORE work starts and kind=acceptance when accepting finished work; attach bounded materials (paths, requirement quotes) when they help the seats.",
      "The tool call asks the user for approval first (the dialog states the token cost). If approved, the system dispatches every seat, advances rounds by itself, and a chair turn posts the final decision card; you only relay that it convened and later summarize the card in one or two sentences. NEVER impersonate seats, NEVER pre-write their conclusions, and NEVER dispatch seat orders yourself via AgentDispatch.",
      "The user may pause the meeting or interject remarks (主持人插话) while it runs; interjections are injected into the next round's envelopes. Do not call CouncilConvene again for a meeting already running - wait for receipts and the chair card.",
    ],
    readOnly: false,
    destructive: false,
    concurrentSafe: false,
    timeoutMs: COUNCIL_TOOL_TIMEOUT_MS,
    maxOutputBytes: COUNCIL_MODEL_BYTES,
    sideEffectScope: "workspace",
    riskLevel: "medium",
    needsApproval: true,
  },
  handler: councilConveneHandler,
  inputSchema: CouncilConveneInputJsonSchema,
  outputSchema: CouncilConveneOutputJsonSchema,
  runtimeInputSchema: CouncilConveneInputSchema,
  runtimeOutputSchema: CouncilConveneOutputSchema,
  permission: councilConvenePermission,
  resultBudget: councilConveneResultBudget,
  timeout: councilConveneTimeout,
  cancellation: {
    supported: true,
    cleanup: "none",
    userVisibleMessage: "CouncilConvene was cancelled before the meeting was convened",
  },
  trace: {
    required: true,
    propagateToAdapters: true,
    recordInput: "summary",
    recordOutput: "summary",
  },
};
