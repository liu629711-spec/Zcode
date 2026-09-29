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
import { resolveModelReference } from "./model-reference.js";

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

  // 指定模型（2026-09-29 用户需求）：在发起方目录里解析注册表拼写，解析失败如实
  // 报错并附可选清单（recoverable——模型可重试修正，不是死路）。
  let modelSelection: {
    providerId: string;
    modelId: string;
    options?: { reasoningLevel?: string };
  } | undefined;
  if (parsed.model !== undefined) {
    if (!context.modelCatalogPort) {
      throw createCoreError(
        CoreErrorType.ConfigurationError,
        "Model catalog is not configured for AgentDispatch",
        {
          context: {
            toolCallId: context.toolCallId,
            toolName: "AgentDispatch",
          },
          recoverable: false,
        },
      );
    }
    const resolution = resolveModelReference(parsed.model, context.modelCatalogPort.listModels());
    if (!resolution.ok) {
      throw createCoreError(CoreErrorType.ToolExecutionFailed, resolution.message, {
        context: {
          toolCallId: context.toolCallId,
          toolName: "AgentDispatch",
        },
        recoverable: true,
      });
    }
    modelSelection = resolution.selection;
  }

  // 发起方署名由执行器注入（当前会话），模型不可伪造来源信封。
  const result = await context.agentDispatchPort.dispatch({
    agent: parsed.agent,
    task: parsed.task,
    ...(parsed.title === undefined ? {} : { title: parsed.title }),
    ...(parsed.newSession === undefined ? {} : { newSession: parsed.newSession }),
    ...(modelSelection === undefined ? {} : { modelSelection }),
    sourceSessionId: context.sessionId,
  });
  const deliveryNote =
    result.delivery === "queued"
      ? "The target session is busy; the work order is queued and will run at the next turn boundary."
      : "The work order has been delivered into the target session.";
  const acceptance = `Work order accepted by ${result.agentName}. ${deliveryNote} This only confirms delivery; the answer will arrive later as a receipt in this session.`;
  return {
    targetSessionId: result.targetSessionId,
    agentName: result.agentName,
    ...(result.agentId ? { agentId: result.agentId } : {}),
    delivery: result.delivery,
    createdSession: result.createdSession,
    ...(result.model ? { model: result.model } : {}),
    message: result.model ? `${acceptance} Running on model ${result.model}.` : acceptance,
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
    // 选择的默认取向（2026-09-29 真机教训）：用户点名让某个智能体做任何事（哪怕一句你好）
    // 都必须走本工具——临时工豁免会把"让 X 给我发个你好"这类主场景吞给 Agent 工具。
    description:
      "Hand a task to another agent of this workspace: the task is delivered into that agent's own resident session, runs there with that agent's identity, memory and permissions, and leaves a permanent record in that session; a receipt carrying the final answer is delivered back to this conversation afterwards. This is the right tool whenever the user asks another agent BY NAME to do something or to send something - e.g. 'have UI-plus send me a greeting', '让 UI-plus 给我发个你好', 'ask code-plus to fix the login page', 'open a new session for code-plus and have it ...' - no matter how small the task is: the user expects it to land in that agent's own session, not this one. Do not confuse it with the Agent tool's in-conversation helpers: a helper runs inside THIS conversation, leaves nothing in the named agent's session, and that agent never learns about the task. Only keep a named agent's task inside this conversation when the user explicitly asks to handle it right here without involving that agent's session.",
    modelInstructions: [
      "Default to this tool whenever the user's message names another agent of this workspace as the doer - 让/叫/请/交给/转交/给 <name> 发…做…, have <name> ..., ask <name> to ..., open a new session for <name> - INCLUDING greetings and other one-line asks. The named agent must receive the task in its own session; do NOT answer with an in-conversation subagent for these.",
      "Handle the request inside this conversation only when the user explicitly wants it handled here without involving the other agent's session, or is asking about the agents rather than tasking them.",
      "Pass the agent's agentId when it is known; otherwise pass the agent's exact name. Ambiguous or unknown names are rejected - never guess an id.",
      "Write the task as complete standalone instructions; the target agent cannot see this conversation.",
      "Set newSession=true only when the user asks for a fresh session; by default the task goes to the agent's latest persona session. When a new session is requested, also pass a short human title (title) in the user's language - it becomes the session's name in the list.",
      "If the user asks for a new session WITHOUT naming which agent should work in it, ask first instead of picking an agent from earlier conversation context - a session opened for the wrong agent is a wrong delivery.",
      "When the user names a model for the task ('用 deepseek 回复我', 'use deepseek-v4.1'), pass it as model; the override applies to that task only. Changing an agent's default model is done via its profile, not this tool.",
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
