// ============================================================
// AgentDispatch Port (D29/D2): 协议层实现的跨会话工单投递。
// ============================================================
// 落在 bootstrap 协议层（不是宿主进程）：context.sessions 会话注册表与
// sessionStore 都在这里，一个 CLI 进程只服务一个 workspace，派单因此天然
// 只能投同 workspace 的档案/会话；跨 workspace/跨机器目标在这个进程里根本
// 不可见，等同于被结构性拒绝。
//
// 嵌套上限=1 的端口级自检：工单唤醒轮的 inputId 带 workorder- 前缀
// （core turn-loop 据此进 denylist 终审），工单轮内再调 AgentDispatch 在这里
// 就被拒绝，不会触达目标会话。

import {
  SessionEventType,
  WORK_ORDER_INPUT_ID_PREFIX,
  type AgentDispatchPort,
  type AgentDispatchRequest,
  type AgentDispatchResult,
  type AgentWorkOrderEnvelope,
  type SessionEvent,
} from "@zcode/contracts";
import type { ModelSelection, ZCodeSessionPersona, ZCodeWorkspaceRef } from "@zcode/shared";
import {
  isSelfDispatch,
  resolveWorkOrderTarget,
  type AgentProfile,
  type WorkOrderReceiptOutcome,
} from "@zcode/core";

import type {
  ZCodeProtocolAgentServerContext,
  ZCodeProtocolSessionRecord,
} from "./server-types.js";

export interface ProtocolAgentDispatchPortDeps {
  /** 归属会话解析器（automation-port 同款惰性绑定）：本端口所服务的 session record。 */
  resolveOwnSession?: () => ZCodeProtocolSessionRecord | undefined;
  /**
   * 既有 createSession 链（createSessionRecordForV4 包装）；由 server-operations 注入。
   * persona 缺省 = 派生无 persona 的普通会话（对齐稿 §三 #2/#5，无工牌/档案/记忆）；
   * model 是普通会话的出生常驻模型（createSession 的 model 参数），员工新段不经过它。
   * memoryEnabled=false 只由普通工位传（干净工位不读工作区记忆笔记，对齐稿 §六）。
   */
  createPersonaSessionRecord: (input: {
    workspace: ZCodeWorkspaceRef;
    persona?: ZCodeSessionPersona;
    model?: ModelSelection;
    memoryEnabled?: boolean;
  }) => Promise<{ sessionId: string }>;
  /** 冷会话恢复（activateSessionForResume 包装）；由 server-operations 注入。 */
  activateSessionRecord: (sessionId: string) => Promise<ZCodeProtocolSessionRecord>;
}

/**
 * 工单拒绝 guard id（UI 按它分流成人话，不走错误文本匹配——网关约定：
 * 携带 reasonCode 的领域错误原样上行进 ack.reasonCode，error.message 进 ack.message）。
 */
export const AGENT_WORK_ORDER_GUARDS = {
  nested: "guard.agentWorkOrderNested",
  targetNotFound: "guard.agentWorkOrderTargetNotFound",
  targetAmbiguous: "guard.agentWorkOrderTargetAmbiguous",
  selfTarget: "guard.agentWorkOrderSelfTarget",
  /** 未点名派单缺 newSession=true（core 工具 schema refine 之外的第二道墙，端口级执法）。 */
  input: "guard.agentWorkOrderInput",
} as const;

function agentWorkOrderGuardError(reasonCode: string, message: string): Error {
  return Object.assign(new Error(message), { name: "AgentWorkOrderGuardError", reasonCode });
}

/**
 * persona 快照 × 目标档案对号（照 findLatestPersonaChatRow 判据，D26）：
 * 两边都有号只认号；任一侧无号才比名字。
 */
export function personaMatchesProfile(
  persona: ZCodeSessionPersona,
  profile: Pick<AgentProfile, "name" | "agentId">,
): boolean {
  if (persona.agentId !== undefined && profile.agentId !== undefined) {
    return persona.agentId === profile.agentId;
  }
  return persona.name === profile.name;
}

/** 工作区路径归一（与 UI selectProjectAgentsForWorkspace 同判据：斜杠/盘符大小写容错）。 */
export function normalizeWorkspacePath(value: string): string {
  return value
    .replace(/\\/g, "/")
    .replace(/^[A-Za-z]:/, (drive) => drive.toLowerCase())
    .replace(/\/+$/u, "");
}

function profileToPersona(profile: AgentProfile): ZCodeSessionPersona {
  // 与 UI toProjectAgentPersona 同一映射纪律：可选字段缺席 = 跟随会话缺省；
  // memoryScope 缺省 project（档案缺 memory 字段时同约定）。
  return {
    name: profile.name,
    ...(profile.agentId ? { agentId: profile.agentId } : {}),
    systemPrompt: profile.systemPrompt,
    memoryScope: profile.memory ?? "project",
    ...(profile.modelSelection ? { modelSelection: profile.modelSelection } : {}),
    ...(profile.tools?.length ? { tools: [...profile.tools] } : {}),
    ...(profile.disallowedTools?.length ? { disallowedTools: [...profile.disallowedTools] } : {}),
    ...(profile.color ? { color: profile.color } : {}),
  };
}

/**
 * 点名派单的目标解析终审：not_found/ambiguous 在这里带 guard 词表抛出，
 * resolved 直接回档案（调用方因此拿到非空类型，无需断言）。
 */
function requireWorkOrderTarget(agent: string, profiles: readonly AgentProfile[]): AgentProfile {
  const resolution = resolveWorkOrderTarget(agent, profiles);
  if (resolution.kind === "not_found") {
    // 报错带全名单（照 selfTarget 先例）：派单方看不见名册时，这一行是它唯一
    // 能拿到正确名字的地方——拿到后重试，而不是抓个临时工冒名。
    const availableNames = profiles.map((profile) => profile.name).join(", ") || "none";
    throw agentWorkOrderGuardError(
      AGENT_WORK_ORDER_GUARDS.targetNotFound,
      `No agent profile matches "${resolution.agent}" here. Available agents: ${availableNames}. Retry with one of these exact names, or ask the user who should do the task.`,
    );
  }
  if (resolution.kind === "ambiguous") {
    throw agentWorkOrderGuardError(
      AGENT_WORK_ORDER_GUARDS.targetAmbiguous,
      `Agent "${resolution.agent}" matches multiple profiles (${resolution.matchedNames.join(", ")}). Dispatch by the agent's unique agentId instead.`,
    );
  }
  return resolution.profile;
}

/**
 * 目标最新 persona 会话：listSessions 内存过滤（persona 快照 × 档案对号 +
 * 同 workspace），按 time.updated 取最新。persona 快照随会话落盘
 * （session-store codecs select *），服务端此前没有「按 agentId 找最新 persona
 * 会话」的现成服务，这里在 bootstrap 侧补一个。
 */
export async function findLatestPersonaSessionId(
  store: NonNullable<ZCodeProtocolAgentServerContext["deps"]["sessionStore"]>,
  workspacePath: string,
  profile: Pick<AgentProfile, "name" | "agentId">,
): Promise<string | undefined> {
  const sessions = await store.listSessions();
  const normalizedWorkspace = normalizeWorkspacePath(workspacePath);
  let latest: { sessionId: string; updatedAt: number } | undefined;
  for (const session of sessions) {
    if (!session.persona) continue;
    if (!personaMatchesProfile(session.persona, profile)) continue;
    // workspace 护栏：目录前缀判据（同 UI），别的 workspace 的同名档案会话不参与。
    if (!normalizeWorkspacePath(session.directory).startsWith(normalizedWorkspace)) continue;
    const updatedAt = session.time.updated;
    if (!latest || updatedAt > latest.updatedAt) {
      latest = { sessionId: session.id, updatedAt };
    }
  }
  return latest?.sessionId;
}

export function createProtocolAgentDispatchPort(
  context: ZCodeProtocolAgentServerContext,
  deps: ProtocolAgentDispatchPortDeps,
): AgentDispatchPort {
  return {
    async dispatch(input: AgentDispatchRequest): Promise<AgentDispatchResult> {
      const ownRecord = deps.resolveOwnSession?.();
      if (!ownRecord) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.nested,
          "AgentDispatch is not bound to a session record",
        );
      }
      // 端口自检：工单唤醒轮内再派单 → 直接拒绝（照 automation-port 自检先例）。
      const activeInputId = ownRecord.app.runtime.getActiveTurnInfo()?.inputId;
      if (activeInputId?.trim().startsWith(WORK_ORDER_INPUT_ID_PREFIX)) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.nested,
          "Cannot dispatch a work order while running an agent work order turn.",
        );
      }

      const profiles = ownRecord.app.runtime.getAgentProfiles();
      // 未点名派单（对齐稿 §三 #2/#5）：不带 agent 只允许 newSession=true —— 派生
      // 无 persona 普通会话；否则无落点。core 工具 schema refine 之外的端口级第二道墙。
      if (input.agent === undefined && input.newSession !== true) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.input,
          "Dispatching without an agent requires newSession=true: name a target agent to deliver into its resident session, or set newSession=true to spawn an unbadged ordinary session.",
        );
      }
      const namedProfile =
        input.agent === undefined ? undefined : requireWorkOrderTarget(input.agent, profiles);

      const sourcePersona = ownRecord.app.runtime.getProjectAgentPersona();
      // 自派拒绝（对齐稿 §三规则 6）：员工给自己派工单只会复制自己的对话，
      // 构造信封前端口级硬墙。错误信息教会模型改道：换人/直接干。
      // 仅点名员工的派单有「自派」可言；无 persona 普通会话没有目标档案。
      if (namedProfile && isSelfDispatch(sourcePersona, namedProfile)) {
        const otherNames = profiles
          .filter((candidate) => candidate !== namedProfile)
          .map((candidate) => candidate.name);
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.selfTarget,
          `You cannot dispatch a work order to yourself ("${namedProfile.name}") — that would just copy your own conversation. Dispatch to another agent instead (available: ${
            otherNames.join(", ") || "none - ask the user"
          }), or do the task directly in this conversation.`,
        );
      }
      const envelope: AgentWorkOrderEnvelope = {
        workOrderId: crypto.randomUUID(),
        ...(sourcePersona?.agentId ? { fromAgentId: sourcePersona.agentId } : {}),
        fromAgentName: sourcePersona?.name ?? "",
        fromSessionId: ownRecord.app.sessionId,
        task: input.task,
      };

      // 人类短标题（模型派单时给的 title；缺席回落任务正文）：工位落行首输入、
      // 无名工位的回执署名都用它——回执卡不再退化成「智能体 交活」（评审低危⑤）。
      const workerTitleSeed = input.title?.trim() || input.task;

      let targetSessionId: string | undefined;
      let createdSession = false;
      if (namedProfile && input.newSession !== true) {
        targetSessionId = context.deps.sessionStore
          ? await findLatestPersonaSessionId(
              context.deps.sessionStore,
              ownRecord.workspace.workspacePath,
              namedProfile,
            )
          : undefined;
      }
      if (!targetSessionId) {
        // 既有 createSession persona 链开新段；工作区用发起方自己的 workspace ref
        // （一个 CLI 进程一个 workspace，跨 workspace 目标在这里就不存在）。
        // 本单指定模型（D32）覆盖档案默认：员工新段从出生就带着它（persona
        // modelSelection 走 create 路径落会话常驻选择），后续轮不回退。
        // 可见性（真机反馈 2026-09-30）：用户点名“新建一个会话”就是要看得见的新
        // 会话——不盖 workOrderOnly（隐藏机制保留备用）；派单堆积的真正解法是
        // 复用最近会话（不带 newSession 的派单永远不新增行）+ 守住 CLI 后门。
        // 未点名（对齐稿 §三 #2/#5）：不带 persona 开普通会话——无工牌/档案/记忆，
        // 不占任何员工身份；指定模型经 createSession 的 model 参数成为出生常驻模型；
        // memoryEnabled=false 是工位隔离（真机事故：worker 读到老板记忆笔记，
        // 把"发你好"干成了"今天想干点啥"）——干净工位不读老板的记忆。
        const created = await deps.createPersonaSessionRecord(
          namedProfile
            ? {
                workspace: ownRecord.workspace,
                persona: {
                  ...profileToPersona(namedProfile),
                  ...(input.modelSelection === undefined
                    ? {}
                    : { modelSelection: input.modelSelection }),
                },
              }
            : {
                workspace: ownRecord.workspace,
                ...(input.modelSelection === undefined
                  ? {}
                  : { model: input.modelSelection }),
                memoryEnabled: false,
              },
        );
        targetSessionId = created.sessionId;
        createdSession = true;
      }

      // 冷会话先恢复再提交（照 v4-bridge 冷恢复先例）；对已在场 record 幂等返回。
      const targetRecord =
        context.sessions.get(targetSessionId) ??
        (await deps.activateSessionRecord(targetSessionId));
      // 真机事故修复（2026-09-29）：新建的 persona 会话此前不落行，draft 态不进会话区
      // （侧栏看不见“新开的会话”），且冷恢复按 id 查库落空。工单是外部活动，落行语义
      // 照既有的 ensureSessionPersistedForExternalActivity——首输入用人类短标题
      // （模型派单时给的 title；缺席回落任务正文），标题因此是「智能体名 · 短标题」。
      // 投递前后都幂等（sessionPersisted 置位即返回）。
      await targetRecord.app.runtime.ensureSessionPersistedForExternalActivity(
        workerTitleSeed,
        { traceContext: targetRecord.traceContext },
      );
      const admission = await targetRecord.app.runtime.enqueueAgentWorkOrder({
        envelope,
        traceContext: targetRecord.traceContext,
        ...(input.modelSelection === undefined
          ? {}
          : { modelSelection: input.modelSelection }),
      });
      // 完成钩子（D29/D3）：目标轮 TurnComplete/TurnError（按 workorder- inputId 对号）
      // 后把携带最终答案的回执投回发起方会话。投递成功 ≠ 已处理；本轮终态见回执。
      scheduleWorkOrderReceiptRelay(context, deps, {
        targetRecord,
        inputId: admission.inputId,
        envelope,
        // 无名工位的回执署名用会话标题（"发个你好 交活"），不再退化成「智能体 交活」。
        agentName: namedProfile?.name ?? workerTitleSeed,
        ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
      });
      context.logger?.info("Agent work order dispatched", {
        createdSession,
        delivery: admission.delivery,
        event: "agent_dispatch.port.dispatched",
        fromSessionId: envelope.fromSessionId,
        module: "bootstrap.zcode_protocol",
        targetSessionId,
        unbadged: namedProfile === undefined,
        workOrderId: envelope.workOrderId,
      });
      return {
        targetSessionId,
        agentName: namedProfile?.name ?? "",
        ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
        delivery: admission.delivery,
        createdSession,
        ...(input.modelSelection
          ? { model: input.modelSelection.modelId }
          : {}),
      };
    },
  };
}

// ── 派单回执（D29/D3）：目标轮完成钩子与回执投递 ─────────────────────

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
    const reason =
      typeof error === "object" && error !== null && typeof (error as Record<string, unknown>).message === "string"
        ? ((error as Record<string, unknown>).message as string)
        : "unknown turn error";
    return { status: "failed", reason };
  }
  return undefined;
}

/**
 * 完成钩子：订阅目标 record 的会话事件，目标轮落终态后把回执投回发起方会话
 * （发起方 record 不在场则先 activateSessionForResume 恢复——落库等重开）。
 * 首个终态事件即退订；目标 record 被关闭/进程退出时钩子随 record 消失，
 * 回执与工单同样属于「死会话里不可恢复」的 best-effort 通知（发起方持有受理凭据）。
 */
function scheduleWorkOrderReceiptRelay(
  context: ZCodeProtocolAgentServerContext,
  deps: ProtocolAgentDispatchPortDeps,
  input: {
    targetRecord: ZCodeProtocolSessionRecord;
    inputId: string;
    envelope: AgentWorkOrderEnvelope;
    agentName: string;
    agentId?: string;
  },
): void {
  const unsubscribe = input.targetRecord.app.runtime.subscribeEvents({
    onSessionEvent: (event: SessionEvent) => {
      const outcome = receiptOutcomeFromSessionEvent(event, input.inputId);
      if (!outcome) return;
      unsubscribe();
      void deliverWorkOrderReceipt(context, deps, {
        envelope: input.envelope,
        agentName: input.agentName,
        ...(input.agentId ? { agentId: input.agentId } : {}),
        targetSessionId: input.targetRecord.app.sessionId,
        outcome,
      }).catch((error) => {
        context.logger?.warn("Failed to deliver agent work order receipt", {
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "agent_work_order_receipt.delivery_failed",
          fromSessionId: input.envelope.fromSessionId,
          module: "bootstrap.zcode_protocol",
          outcomeStatus: outcome.status,
          targetSessionId: input.targetRecord.app.sessionId,
          workOrderId: input.envelope.workOrderId,
        });
      });
    },
  });
}

async function deliverWorkOrderReceipt(
  context: ZCodeProtocolAgentServerContext,
  deps: ProtocolAgentDispatchPortDeps,
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
