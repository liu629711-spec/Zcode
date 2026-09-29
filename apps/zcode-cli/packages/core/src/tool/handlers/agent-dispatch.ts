// ============================================================
// AgentDispatch Tool Handler (D29/D1): 把任务转交给目标智能体的常驻会话
// ============================================================

import {
  AgentDispatchInputSchema,
  AgentDispatchOutputSchema,
  AgentDispatchInputJsonSchema,
  AgentDispatchOutputJsonSchema,
  CoreErrorType,
  createCoreError,
  type AgentDispatchInput,
  type AgentDispatchOutput,
  type ToolPermissionSpec,
} from "@zcode/contracts";
import type { ToolEntry, ToolExecutionContext, ToolHandler } from "../types.js";

const AGENT_DISPATCH_TOOL_TIMEOUT_MS = 30_000;
const AGENT_DISPATCH_MODEL_BYTES = 16_000;

/** 与 cron.ts 的 assertNotAutomationTurn 同款终审：provider denylist 只是可见性约束。 */
function assertNotWorkOrderTurn(context: ToolExecutionContext): void {
  if (!context.workOrderTurn) return;
  // 工单触发的轮次里再派单 = A→B→A 死循环。handler 以 executor 传入的本轮事实
  // 做最终拒绝，且不能调用端口（嵌套上限=1，端口层执法，不靠提示词）。
  throw createCoreError(CoreErrorType.PermissionDenied, `AgentDispatch is not allowed while running an agent work order.`, {
    context: {
      toolCallId: context.toolCallId,
      toolName: "AgentDispatch",
    },
    recoverable: false,
    retryable: false,
  });
}

function assertDispatchPort(
  context: ToolExecutionContext,
): asserts context is ToolExecutionContext & {
  agentDispatchPort: NonNullable<ToolExecutionContext["agentDispatchPort"]>;
} {
  if (context.agentDispatchPort) return;
  throw createCoreError(
    CoreErrorType.ConfigurationError,
    "AgentDispatchPort is not configured for AgentDispatch",
    {
      context: {
        toolCallId: context.toolCallId,
        toolName: "AgentDispatch",
      },
      recoverable: false,
    },
  );
}

const agentDispatchHandler: ToolHandler = async (input, context) => {
  assertNotWorkOrderTurn(context);
  const parsed = AgentDispatchInputSchema.parse(input) as AgentDispatchInput;
  assertDispatchPort(context);

  // 发起方署名由执行器注入（当前会话），模型不可伪造来源信封。
  const result = await context.agentDispatchPort.dispatch({
    agent: parsed.agent,
    task: parsed.task,
    ...(parsed.newSession === undefined ? {} : { newSession: parsed.newSession }),
    sourceSessionId: context.sessionId,
  });
  const deliveryNote =
    result.delivery === "queued"
      ? "The target session is busy; the work order is queued and will run at the next turn boundary."
      : "The work order has been delivered into the target session.";
  return {
    targetSessionId: result.targetSessionId,
    agentName: result.agentName,
    ...(result.agentId ? { agentId: result.agentId } : {}),
    delivery: result.delivery,
    createdSession: result.createdSession,
    message: `Work order accepted by ${result.agentName}. ${deliveryNote} This only confirms delivery; the answer will arrive later as a receipt in this session.`,
  } satisfies AgentDispatchOutput;
};

const agentDispatchPermission: ToolPermissionSpec = {
  permission: "agent.dispatch",
  reason: "AgentDispatch delivers a task into another agent's resident session in this workspace",
  riskLevel: "medium",
  sideEffectScope: "workspace",
  needsApproval: false,
  patternSources: ["toolName"],
  alwaysAllowPatternSources: ["toolName"],
  denyPriority: "beforeAsk",
};

const agentDispatchResultBudget = {
  maxInlineBytes: AGENT_DISPATCH_MODEL_BYTES,
  maxModelBytes: AGENT_DISPATCH_MODEL_BYTES,
  strategy: "truncate" as const,
  preview: {
    maxBytes: AGENT_DISPATCH_MODEL_BYTES,
    direction: "head" as const,
  },
};

const agentDispatchTimeout = {
  defaultMs: AGENT_DISPATCH_TOOL_TIMEOUT_MS,
  maxMs: AGENT_DISPATCH_TOOL_TIMEOUT_MS,
  allowCallOverride: false,
};

export const agentDispatchToolEntry: ToolEntry = {
  capability: "Hand a task to another agent's resident session in this workspace",
  metadata: {
    name: "AgentDispatch",
    // 三条划界写进 contract：与点将（Agent/Task = 临时工）、与 SendMessage、与普通回话。
    description:
      "Hand a complete task to another agent's resident persona session in the current workspace. The task runs in that agent's own session with its own identity, memory and permissions, leaves a permanent record there, and a receipt with the final answer is delivered back to this session later. This is different from the Agent tool (@-mentioned temporary helpers): temporary helpers run inside THIS conversation and leave no trace elsewhere. It is also different from SendMessage, which only relays a message to a running subagent. Use it when the user asks to hand work over to a specific agent of this workspace, not for quick in-conversation questions.",
    modelInstructions: [
      "Use only when the user explicitly asks to hand a task to another agent of this workspace (派单/交给/转交某智能体). Do not use it for quick questions that a temporary in-conversation helper can answer.",
      "Pass the agent's agentId when it is known; otherwise pass the agent's exact name. Ambiguous or unknown names are rejected - never guess an id.",
      "Write the task as complete standalone instructions; the target agent cannot see this conversation.",
      "Set newSession=true only when the user asks for a fresh session; by default the task goes to the agent's latest persona session.",
      "A successful call only means the work order was accepted (queued if the target is busy). The final answer arrives as a separate receipt; do not claim the task is done.",
      "Never call AgentDispatch from a turn that was itself started by an agent work order; nesting is not allowed.",
    ],
    readOnly: false,
    destructive: false,
    concurrentSafe: true,
    timeoutMs: AGENT_DISPATCH_TOOL_TIMEOUT_MS,
    maxOutputBytes: AGENT_DISPATCH_MODEL_BYTES,
    sideEffectScope: "workspace",
    riskLevel: "medium",
    needsApproval: false,
  },
  handler: agentDispatchHandler,
  inputSchema: AgentDispatchInputJsonSchema,
  outputSchema: AgentDispatchOutputJsonSchema,
  runtimeInputSchema: AgentDispatchInputSchema,
  runtimeOutputSchema: AgentDispatchOutputSchema,
  permission: agentDispatchPermission,
  resultBudget: agentDispatchResultBudget,
  timeout: agentDispatchTimeout,
  cancellation: {
    supported: true,
    cleanup: "none",
    userVisibleMessage: "AgentDispatch was cancelled before the work order was delivered",
  },
  trace: {
    required: true,
    propagateToAdapters: true,
    recordInput: "summary",
    recordOutput: "summary",
  },
};
