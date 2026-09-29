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
  WORK_ORDER_INPUT_ID_PREFIX,
  type AgentDispatchPort,
  type AgentDispatchRequest,
  type AgentDispatchResult,
  type AgentWorkOrderEnvelope,
} from "@zcode/contracts";
import type { ZCodeSessionPersona, ZCodeWorkspaceRef } from "@zcode/shared";
import { resolveWorkOrderTarget, type AgentProfile } from "@zcode/core";

import type {
  ZCodeProtocolAgentServerContext,
  ZCodeProtocolSessionRecord,
} from "./server-types.js";

export interface ProtocolAgentDispatchPortDeps {
  /** 归属会话解析器（automation-port 同款惰性绑定）：本端口所服务的 session record。 */
  resolveOwnSession?: () => ZCodeProtocolSessionRecord | undefined;
  /** 既有 createSession persona 链（createSessionRecordForV4 包装）；由 server-operations 注入。 */
  createPersonaSessionRecord: (input: {
    workspace: ZCodeWorkspaceRef;
    persona: ZCodeSessionPersona;
  }) => Promise<{ sessionId: string }>;
  /** 冷会话恢复（activateSessionForResume 包装）；由 server-operations 注入。 */
  activateSessionRecord: (sessionId: string) => Promise<ZCodeProtocolSessionRecord>;
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
        throw new Error("AgentDispatch is not bound to a session record");
      }
      // 端口自检：工单唤醒轮内再派单 → 直接拒绝（照 automation-port 自检先例）。
      const activeInputId = ownRecord.app.runtime.getActiveTurnInfo()?.inputId;
      if (activeInputId?.trim().startsWith(WORK_ORDER_INPUT_ID_PREFIX)) {
        throw new Error("Cannot dispatch a work order while running an agent work order turn.");
      }

      const profiles = ownRecord.app.runtime.getAgentProfiles();
      const resolution = resolveWorkOrderTarget(input.agent, profiles);
      if (resolution.kind === "not_found") {
        throw new Error(
          `No agent profile matches "${resolution.agent}" in this workspace. Ask the user to check the agent name or id.`,
        );
      }
      if (resolution.kind === "ambiguous") {
        throw new Error(
          `Agent "${resolution.agent}" matches multiple profiles (${resolution.matchedNames.join(", ")}). Dispatch by the agent's unique agentId instead.`,
        );
      }
      const profile = resolution.profile;

      const sourcePersona = ownRecord.app.runtime.getProjectAgentPersona();
      const envelope: AgentWorkOrderEnvelope = {
        workOrderId: crypto.randomUUID(),
        ...(sourcePersona?.agentId ? { fromAgentId: sourcePersona.agentId } : {}),
        fromAgentName: sourcePersona?.name ?? "",
        fromSessionId: ownRecord.app.sessionId,
        task: input.task,
      };

      let targetSessionId: string | undefined;
      let createdSession = false;
      if (input.newSession !== true) {
        targetSessionId = context.deps.sessionStore
          ? await findLatestPersonaSessionId(
              context.deps.sessionStore,
              ownRecord.workspace.workspacePath,
              profile,
            )
          : undefined;
      }
      if (!targetSessionId) {
        // 既有 createSession persona 链开新段；工作区用发起方自己的 workspace ref
        // （一个 CLI 进程一个 workspace，跨 workspace 目标在这里就不存在）。
        const created = await deps.createPersonaSessionRecord({
          workspace: ownRecord.workspace,
          persona: profileToPersona(profile),
        });
        targetSessionId = created.sessionId;
        createdSession = true;
      }

      // 冷会话先恢复再提交（照 v4-bridge 冷恢复先例）；对已在场 record 幂等返回。
      const targetRecord =
        context.sessions.get(targetSessionId) ??
        (await deps.activateSessionRecord(targetSessionId));
      const admission = await targetRecord.app.runtime.enqueueAgentWorkOrder({
        envelope,
        traceContext: targetRecord.traceContext,
      });
      context.logger?.info("Agent work order dispatched", {
        createdSession,
        delivery: admission.delivery,
        event: "agent_dispatch.port.dispatched",
        fromSessionId: envelope.fromSessionId,
        module: "bootstrap.zcode_protocol",
        targetSessionId,
        workOrderId: envelope.workOrderId,
      });
      return {
        targetSessionId,
        agentName: profile.name,
        ...(profile.agentId ? { agentId: profile.agentId } : {}),
        delivery: admission.delivery,
        createdSession,
      };
    },
  };
}
