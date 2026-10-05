// ============================================================
// TeamMessage Tool Handler（团队看板批3）：队内直达消息——同事串工位。
// ============================================================
// 纪律在端口层执法（bootstrap team-message.ts）：只发队内（台账派单行对账）、
// 防冒名（from 按发起会话 persona 铸造）、必留痕（队长台账 + 看板通讯区）。
// handler 只做输入解析与端口转接；发起轮的工单身份（context.workOrderId）是
// 找队的钥匙——工单轮/消息轮里发消息才有，普通轮缺席 → 端口如实拒绝。

import {
  TeamMessageInputSchema,
  TeamMessageOutputSchema,
  TeamMessageInputJsonSchema,
  TeamMessageOutputJsonSchema,
  CoreErrorType,
  createCoreError,
  type TeamMessageInput,
  type TeamMessageOutput,
  type ToolPermissionSpec,
} from "@zcode/contracts";
import type { ToolEntry, ToolExecutionContext, ToolHandler } from "../types.js";

const TEAM_MESSAGE_TOOL_TIMEOUT_MS = 15_000;
const TEAM_MESSAGE_MODEL_BYTES = 4_000;

function assertTeamMessagePort(context: ToolExecutionContext): asserts context is ToolExecutionContext & {
  teamMessagePort: NonNullable<ToolExecutionContext["teamMessagePort"]>;
} {
  if (context.teamMessagePort) return;
  throw createCoreError(
    CoreErrorType.ConfigurationError,
    "TeamMessagePort is not configured for TeamMessage",
    {
      context: {
        toolCallId: context.toolCallId,
        toolName: "TeamMessage",
      },
      recoverable: false,
    },
  );
}

const teamMessageHandler: ToolHandler = async (input, context) => {
  const parsed = TeamMessageInputSchema.parse(input) as TeamMessageInput;
  assertTeamMessagePort(context);
  const result = await context.teamMessagePort.send({
    to: parsed.to,
    message: parsed.message,
    sourceSessionId: context.sessionId,
    // 发起轮的工单身份：工单轮/消息轮里才有；端口据它找回队与队长会话。
    ...(context.workOrderId ? { workOrderId: context.workOrderId } : {}),
  });
  return {
    targetSessionId: result.targetSessionId,
    toName: result.toName,
    ...(result.batchId ? { batchId: result.batchId } : {}),
    delivery: result.delivery,
    message: `Message delivered to ${result.toName}. It wakes their session as a short message turn - not a work order, so no receipt comes back; they will see it in their session and on the team board's comms log.`,
  } satisfies TeamMessageOutput;
};

const teamMessagePermission: ToolPermissionSpec = {
  permission: "agent.teamMessage",
  reason: "TeamMessage delivers a short in-team message to a teammate's resident session",
  riskLevel: "low",
  sideEffectScope: "workspace",
  needsApproval: false,
  patternSources: ["toolName"],
  alwaysAllowPatternSources: ["toolName"],
  denyPriority: "beforeAsk",
};

const teamMessageResultBudget = {
  maxInlineBytes: TEAM_MESSAGE_MODEL_BYTES,
  maxModelBytes: TEAM_MESSAGE_MODEL_BYTES,
  strategy: "truncate" as const,
  preview: {
    maxBytes: TEAM_MESSAGE_MODEL_BYTES,
    direction: "head" as const,
  },
};

const teamMessageTimeout = {
  defaultMs: TEAM_MESSAGE_TOOL_TIMEOUT_MS,
  maxMs: TEAM_MESSAGE_TOOL_TIMEOUT_MS,
  allowCallOverride: false,
};

export const teamMessageToolEntry: ToolEntry = {
  capability: "Send a short in-team message to a teammate of your current team",
  metadata: {
    name: "TeamMessage",
    description:
      "Send a short direct message to a teammate in YOUR current team (the batch you were dispatched into): it wakes the teammate's session as a brief message turn, like tapping a coworker's desk to ask a quick question - no captain relay, no receipt. Use it for coordination one-liners (field names, interface changes, 'ready yet?'); use AgentDispatch instead when handing over actual work, and SendMessage only for background helpers launched in this conversation. Messages are logged on the team board's comms log where the user can read them; you cannot forge the sender identity.",
    modelInstructions: [
      "Use TeamMessage for quick coordination with a teammate INSIDE your team when a full work order would be overkill (a question about an interface they own, a heads-up that you started, asking for their status). It must be a teammate dispatched into the same batch as your own work order; messaging anyone else is rejected with the team roster.",
      "A TeamMessage is a one-way ping: it wakes the teammate but no receipt comes back. If you need their output as a deliverable, dispatch a work order with AgentDispatch instead; if they reply with facts you need, they will message you back or you will see it on your next shared artifact.",
      "NEVER use TeamMessage to loop chit-chat with a teammate (ask -> reply -> ask ...): each message wakes their session and costs a turn. Ask one concrete question per message.",
    ],
    readOnly: false,
    destructive: false,
    concurrentSafe: true,
    timeoutMs: TEAM_MESSAGE_TOOL_TIMEOUT_MS,
    maxOutputBytes: TEAM_MESSAGE_MODEL_BYTES,
    sideEffectScope: "workspace",
    riskLevel: "low",
    needsApproval: false,
  },
  handler: teamMessageHandler,
  inputSchema: TeamMessageInputJsonSchema,
  outputSchema: TeamMessageOutputJsonSchema,
  runtimeInputSchema: TeamMessageInputSchema,
  runtimeOutputSchema: TeamMessageOutputSchema,
  permission: teamMessagePermission,
  resultBudget: teamMessageResultBudget,
  timeout: teamMessageTimeout,
  cancellation: {
    supported: true,
    cleanup: "none",
    userVisibleMessage: "TeamMessage was cancelled before the message was delivered",
  },
  trace: {
    required: true,
    propagateToAdapters: true,
    recordInput: "summary",
    recordOutput: "summary",
  },
};
