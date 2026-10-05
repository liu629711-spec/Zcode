// ============================================================
// Team Message Port（团队看板批3）：队内直达消息的协议层实现。
// ============================================================
// 同事串工位：成员 A 给同队的 B 发一句话，B 的工位被唤醒一次消息轮（不是工
// 单——不建派单台账行、不接回执线、不计频控在飞）。纪律执法：
//  - 找队：发起轮的工单身份 → 全局台账（dispatch 行 → 消息行兜底，消息轮里
//    回消息也找得到队）→ 批次 + 队长会话；找不到 = 不在任何队里，拒绝。
//  - 队内校验：收件人必须是同批次派过单的员工（台账派单行对账）。
//  - 防冒名：from = 发起会话的 persona（系统铸造），模型不可伪造。
//  - 必留痕：队长会话台账落 agentWorkOrderTeamMessage 行，看板通讯区可见。

import {
  AGENT_WORK_ORDER_TASK_MAX_CHARS,
  type AgentWorkOrderEnvelope,
  type SessionId,
  type TeamMessagePort,
  type TeamMessageRequest,
  type TeamMessageResult,
} from "@zcode/contracts";
import { resolveWorkOrderTarget } from "@zcode/core";
import type { ZCodeProtocolAgentServerContext } from "./server-types.js";
import type { ProtocolAgentDispatchPortDeps } from "./agent-dispatch-port.js";
import { findLatestPersonaSessionId, isReuseSessionRetired } from "./agent-dispatch-port.js";
import type { TeamBoardInputRow } from "./team-board.js";

/** 消息留痕行的确定性 id（= 消息工单的 workOrderId，消息轮里回消息据此找队）。 */
export function teamMessageRowId(messageWorkOrderId: string): string {
  return `agentWorkOrderTeamMessage:${messageWorkOrderId}`;
}

/** 消息轮的 carrier 前缀：成员模型看到的第一行，说明这不是派单而是同事的一句话。 */
export function buildTeamMessageText(fromName: string, message: string): string {
  const body = message.length > AGENT_WORK_ORDER_TASK_MAX_CHARS - 200
    ? `${message.slice(0, AGENT_WORK_ORDER_TASK_MAX_CHARS - 201)}…`
    : message;
  return `【队内消息】来自 ${fromName}（不是新工单，无需交活回执；需要回话就用 TeamMessage 回给对方）：\n\n${body}`;
}

/** 队内校验（纯函数）：收件人是否本批次派过单的员工（工号优先，其次精确名）。 */
export function isTargetInTeam(
  rows: readonly TeamBoardInputRow[],
  batchId: string,
  target: { name: string; agentId?: string },
): boolean {
  return rows.some((row) => {
    if (row.kind !== "agentWorkOrderDispatch") return false;
    const envelope = row.payload.envelope;
    const rowBatchId =
      typeof envelope === "object" && envelope !== null
        ? (envelope as { batchId?: unknown }).batchId
        : undefined;
    if (rowBatchId !== batchId) return false;
    if (target.agentId !== undefined) return row.payload.agentId === target.agentId;
    return row.payload.agentName === target.name;
  });
}

function teamMessageGuardError(reasonCode: string, message: string): Error {
  return Object.assign(new Error(message), { name: "AgentWorkOrderGuardError", reasonCode });
}

export const TEAM_MESSAGE_GUARDS = {
  notInTeam: "guard.teamMessageNotInTeam",
  targetNotFound: "guard.teamMessageTargetNotFound",
  noDesk: "guard.teamMessageNoDesk",
} as const;

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

export function createProtocolTeamMessagePort(
  context: ZCodeProtocolAgentServerContext,
  deps: Pick<ProtocolAgentDispatchPortDeps, "activateSessionRecord" | "createPersonaSessionRecord">,
): TeamMessagePort {
  return {
    async send(input: TeamMessageRequest): Promise<TeamMessageResult> {
      const store = context.deps.sessionStore;
      if (!store) {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.notInTeam,
          "Team roster is unavailable in this deployment; TeamMessage cannot verify team membership.",
        );
      }
      const ownRecord =
        context.sessions.get(input.sourceSessionId) ??
        (await deps.activateSessionRecord(input.sourceSessionId));
      // from 身份系统铸造（防冒名）：发起会话的 persona；无名会话发不了队内消息。
      const fromPersona = ownRecord.app.runtime.getProjectAgentPersona();
      const fromName = fromPersona?.name?.trim() ?? "";
      if (!fromName) {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.notInTeam,
          "TeamMessage requires a named agent identity; this session has no agent persona.",
        );
      }

      // 找队：发起轮工单身份 → 全局台账（派单行 → 消息行兜底）→ 批次 + 队长会话。
      const store_any = store as {
        getSessionInputById?: (id: string) => Promise<{
          id: string;
          sessionID: string;
          status: string;
          payload: Record<string, unknown>;
        } | null>;
      };
      let batchId: string | undefined;
      let captainSessionId: string | undefined;
      if (input.workOrderId) {
        for (const rowId of [
          `agentWorkOrderDispatch:${input.workOrderId}`,
          teamMessageRowId(input.workOrderId),
        ]) {
          try {
            const row = await store_any.getSessionInputById?.(rowId);
            const payloadEnvelope = row?.payload?.envelope as { batchId?: unknown } | undefined;
            batchId =
              readString(payloadEnvelope?.batchId) ?? readString(row?.payload?.batchId) ?? batchId;
            captainSessionId = row?.sessionID ?? captainSessionId;
            if (batchId && captainSessionId) break;
          } catch {
            // 查不到就试下一个键；都查不到按不在队里处理。
          }
        }
      }
      if (!batchId || !captainSessionId) {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.notInTeam,
          "You are not part of any team batch (no work order identity found for this turn), so there is nobody to message. Use AgentDispatch to hand work to someone instead.",
        );
      }

      // 目标解析（与派单同一纪律：工号优先、歧义拒绝、报错带全名单）。
      const profiles = ownRecord.app.runtime.getAgentProfiles();
      const resolution = resolveWorkOrderTarget(input.to, profiles);
      if (resolution.kind === "not_found") {
        const availableNames = profiles.map((profile) => profile.name).join(", ") || "none";
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.targetNotFound,
          `No agent profile matches "${resolution.agent}" here. Available agents: ${availableNames}.`,
        );
      }
      if (resolution.kind === "ambiguous") {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.targetNotFound,
          `Agent "${resolution.agent}" matches multiple profiles (${resolution.matchedNames.join(", ")}). Message by the agent's unique agentId instead.`,
        );
      }
      const target = resolution.profile;

      // 队内校验：收件人必须本批次派过单。
      if (!store.listSessionInputs) {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.notInTeam,
          "Team roster is unavailable in this deployment; TeamMessage cannot verify team membership.",
        );
      }
      const captainRows = (await store.listSessionInputs({
        sessionID: captainSessionId as SessionId,
      })) as unknown as TeamBoardInputRow[];
      if (
        !isTargetInTeam(captainRows, batchId, {
          name: target.name,
          ...(target.agentId ? { agentId: target.agentId } : {}),
        })
      ) {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.notInTeam,
          `"${target.name}" is not a member of your team batch - messages stay inside the team. The team roster (agents dispatched into this batch) is what you can message; for cross-team asks, report to the user instead.`,
        );
      }

      // 工位：目标员工最新 persona 会话（与派单同一解析）；没收工的员工没有工位。
      let targetSessionId = await findLatestPersonaSessionId(
        store,
        ownRecord.workspace.workspacePath,
        target,
      );
      if (targetSessionId && (await isReuseSessionRetired(context, targetSessionId))) {
        targetSessionId = undefined;
      }
      if (!targetSessionId) {
        throw teamMessageGuardError(
          TEAM_MESSAGE_GUARDS.noDesk,
          `"${target.name}" has no desk session to wake yet (nobody dispatched work to them in this batch). Hand them work with AgentDispatch first - the dispatch creates their desk.`,
        );
      }
      const targetRecord =
        context.sessions.get(targetSessionId) ??
        (await deps.activateSessionRecord(targetSessionId));

      // 投递：直接 enqueue（无派单台账行、无回执线——一句话的唤醒不是工单）。
      const messageWorkOrderId = crypto.randomUUID();
      const envelope: AgentWorkOrderEnvelope = {
        workOrderId: messageWorkOrderId,
        ...(fromPersona?.agentId ? { fromAgentId: fromPersona.agentId } : {}),
        fromAgentName: fromName,
        fromSessionId: input.sourceSessionId,
        task: buildTeamMessageText(fromName, input.message),
        batchId,
      };
      await targetRecord.app.runtime.enqueueAgentWorkOrder({
        envelope,
        traceContext: targetRecord.traceContext,
      });

      // 必留痕：队长台账落通讯行（看板通讯区的唯一数据源）。
      await store?.saveSessionInput?.({
        id: teamMessageRowId(messageWorkOrderId),
        sessionID: captainSessionId as SessionId,
        kind: "agentWorkOrderTeamMessage",
        delivery: "queue",
        payload: {
          text: input.message,
          batchId,
          from: fromName,
          to: target.name,
        },
      });
      context.logger?.info("Team message delivered", {
        event: "team_message.delivered",
        module: "bootstrap.zcode_protocol",
        batchId,
        from: fromName,
        to: target.name,
        sessionId: input.sourceSessionId,
        targetSessionId,
      });
      return {
        targetSessionId,
        toName: target.name,
        batchId,
        delivery: "sent",
      };
    },
  };
}
