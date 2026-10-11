import type { TaskNotificationPayload } from "@zcode/shared";
import type {
  ConversationSnapshot,
  PendingInteraction,
  SessionPhase,
  SessionSummary,
  UserInputRequestPayload,
} from "@zcode/shared/zcode-protocol-v4";
import type { IntlInstance } from "@/i18n/index.js";

type FormatMessage = IntlInstance["formatMessage"];

function terminalStatusForPhase(
  phase: SessionPhase,
): Extract<TaskNotificationPayload["status"], "completed" | "failed"> | null {
  if (phase === "completedSuccess" || phase === "completedInterrupted") {
    return "completed";
  }
  if (phase === "error") {
    return "failed";
  }
  return null;
}

function taskTitleBody(
  title: string,
  fallbackMessageId: string,
  formatMessage: FormatMessage,
): string {
  const trimmedTitle = title.trim();
  if (trimmedTitle) {
    return formatMessage({ id: "notification.taskWithTitle" }, { title: trimmedTitle });
  }
  return formatMessage({ id: fallbackMessageId });
}

function terminalPayloadForSession(
  session: SessionSummary,
  status: Extract<TaskNotificationPayload["status"], "completed" | "failed">,
  formatMessage: FormatMessage,
): TaskNotificationPayload {
  if (status === "completed") {
    return {
      taskId: session.sessionId,
      status,
      title: formatMessage({ id: "notification.completed" }),
      body: taskTitleBody(session.title, "notification.completedBody", formatMessage),
    };
  }

  return {
    taskId: session.sessionId,
    status,
    title: formatMessage({ id: "notification.error" }),
    body: taskTitleBody(session.title, "notification.error", formatMessage),
  };
}

export function collectTerminalTaskNotificationPayloads(params: {
  previousBySessionId: ReadonlyMap<string, SessionSummary>;
  sessions: readonly SessionSummary[];
  formatMessage: FormatMessage;
}): TaskNotificationPayload[] {
  const payloads: TaskNotificationPayload[] = [];
  for (const session of params.sessions) {
    const status = terminalStatusForPhase(session.phase);
    if (!status) continue;

    const previous = params.previousBySessionId.get(session.sessionId);
    if (!previous) continue;

    const previousStatus = terminalStatusForPhase(previous.phase);
    if (previousStatus === status) continue;

    payloads.push(terminalPayloadForSession(session, status, params.formatMessage));
  }
  return payloads;
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isPlanApprovalInteraction(payload: UserInputRequestPayload): boolean {
  if (payload.toolName === "ExitPlanMode") {
    return true;
  }
  if (!isPlainRecord(payload.schema)) {
    return false;
  }
  return (
    payload.schema.interaction === "plan_approval" || payload.schema.toolName === "ExitPlanMode"
  );
}

function pendingInteractionPayload(params: {
  snapshot: ConversationSnapshot;
  interaction: PendingInteraction;
  formatMessage: FormatMessage;
}): TaskNotificationPayload | null {
  const { snapshot, interaction, formatMessage } = params;
  const requestId = interaction.interactionId;
  const taskId = snapshot.sessionId;
  const taskTitle = snapshot.meta.title;

  // workspace Hook review 的唯一入口是 Settings；不得把它伪装成
  // permission/elicitation 系统通知。
  if (interaction.payload.kind === "workspaceHookReview") {
    return null;
  }

  if (interaction.payload.kind === "permission") {
    return {
      taskId,
      status: "permission_request",
      requestId,
      title: formatMessage({ id: "notification.permissionRequired" }),
      body:
        interaction.payload.summary.trim() ||
        taskTitleBody(taskTitle, "notification.permissionRequired", formatMessage),
    };
  }

  if (isPlanApprovalInteraction(interaction.payload)) {
    return {
      taskId,
      status: "elicitation_request",
      requestId,
      title: formatMessage({ id: "notification.planApprovalRequired" }),
      body: formatMessage({ id: "notification.planApprovalBody" }),
    };
  }

  return {
    taskId,
    status: "elicitation_request",
    requestId,
    title: formatMessage({ id: "notification.inputRequired" }),
    body:
      interaction.payload.prompt.trim() ||
      taskTitleBody(taskTitle, "notification.inputRequired", formatMessage),
  };
}

export function collectPendingInteractionNotificationPayloads(params: {
  snapshot: ConversationSnapshot;
  seenRequestIds: ReadonlySet<string>;
  formatMessage: FormatMessage;
}): TaskNotificationPayload[] {
  return params.snapshot.pendingInteractions
    .filter((interaction) => !params.seenRequestIds.has(interaction.interactionId))
    .flatMap((interaction) => {
      const payload = pendingInteractionPayload({
        snapshot: params.snapshot,
        interaction,
        formatMessage: params.formatMessage,
      });
      return payload ? [payload] : [];
    });
}

/**
 * 跨会话提问/待批准通知（通知中枢 2026-10-12）：sessions-index 摘要只带队首一条
 * 的轻量身份（kind/toolName，不带问题正文），正文用会话标题拼。与快照版
 * （collectPendingInteractionNotificationPayloads，正文有真实 prompt）并存：
 * 快照版只在"你正看着的那个会话"挂载，摘要版覆盖其余所有会话——同一单两边都
 * 发时 requestId 相同，主进程 3 秒去重窗兜住重复。
 */
export function collectSessionInteractionNotificationPayloads(params: {
  summaries: readonly SessionSummary[];
  seenInteractionIds: ReadonlySet<string>;
  formatMessage: FormatMessage;
}): TaskNotificationPayload[] {
  const payloads: TaskNotificationPayload[] = [];
  for (const session of params.summaries) {
    const interaction = session.pendingInteraction;
    if (!interaction || params.seenInteractionIds.has(interaction.interactionId)) {
      continue;
    }
    if (interaction.kind === "permission") {
      payloads.push({
        taskId: session.sessionId,
        status: "permission_request",
        requestId: interaction.interactionId,
        title: params.formatMessage({ id: "notification.permissionRequired" }),
        body: taskTitleBody(session.title, "notification.permissionRequired", params.formatMessage),
      });
      continue;
    }
    // userInput：ExitPlanMode 计划批准有专属词表；其余按"等你输入"。
    const isPlanApproval = interaction.toolName === "ExitPlanMode";
    payloads.push({
      taskId: session.sessionId,
      status: "elicitation_request",
      requestId: interaction.interactionId,
      title: params.formatMessage({
        id: isPlanApproval ? "notification.planApprovalRequired" : "notification.inputRequired",
      }),
      body: isPlanApproval
        ? params.formatMessage({ id: "notification.planApprovalBody" })
        : taskTitleBody(session.title, "notification.inputRequired", params.formatMessage),
    });
  }
  return payloads;
}
