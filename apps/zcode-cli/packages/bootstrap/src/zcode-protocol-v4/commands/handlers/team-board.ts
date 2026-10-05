// 团队看板命令组：teamBoardState（团队看板批 2026-10-05）。
// 只读快照命令，与 dispatchAgentWorkOrder 同族（不经模型轮、不排队）；聚合全在
// 协议层 team-board.ts（台账行 + 会话注册表的工位在跑状态），这里只做接线。
// 注：非输入类命令（不排队、不带 baseRevision）。
import type {
  CommandEnvelope,
  CommandPayloadMap,
  CommandResult,
} from "@zcode/shared/zcode-protocol-v4";
import { requireRecord } from "../record-access.js";
import type { V4CommandCoreHost } from "../types.js";

async function teamAutoFlow(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["teamAutoFlow"];
  if (!host.teamAutoFlow) {
    throw new Error("v4 teamAutoFlow requires host.teamAutoFlow capability");
  }
  const record = requireRecord(host, envelope.sessionId);
  const result = await host.teamAutoFlow(record.app.sessionId, payload);
  return { type: "teamAutoFlow", batchId: result.batchId, enabled: result.enabled };
}

async function teamBoardState(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  void (envelope.payload as CommandPayloadMap["teamBoardState"]);
  if (!host.getTeamBoardState) {
    throw new Error("v4 teamBoardState requires host.getTeamBoardState capability");
  }
  const record = requireRecord(host, envelope.sessionId);
  const snapshot = await host.getTeamBoardState(record.app.sessionId);
  return { type: "teamBoardState", snapshot };
}

function requireTeamPlanPort(host: V4CommandCoreHost): NonNullable<V4CommandCoreHost["teamPlanPort"]> {
  if (!host.teamPlanPort) {
    throw new Error("v4 teamPlan commands require host.teamPlanPort capability");
  }
  return host.teamPlanPort;
}

async function teamPlanUpdate(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["teamPlanUpdate"];
  const record = requireRecord(host, envelope.sessionId);
  await requireTeamPlanPort(host).update({
    planId: payload.planId,
    tasks: payload.tasks,
    sourceSessionId: record.app.sessionId,
  });
  return { type: "teamPlanUpdate", planId: payload.planId };
}

async function teamPlanApprove(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["teamPlanApprove"];
  const record = requireRecord(host, envelope.sessionId);
  const result = await requireTeamPlanPort(host).approve({
    planId: payload.planId,
    sourceSessionId: record.app.sessionId,
  });
  return {
    type: "teamPlanApprove",
    planId: result.planId,
    ...(result.batchId ? { batchId: result.batchId } : {}),
    dispatched: result.dispatched,
    failed: result.failed,
  };
}

async function teamPlanDiscard(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["teamPlanDiscard"];
  const record = requireRecord(host, envelope.sessionId);
  await requireTeamPlanPort(host).discard({
    planId: payload.planId,
    sourceSessionId: record.app.sessionId,
  });
  return { type: "teamPlanDiscard", planId: payload.planId };
}

export const teamBoardHandlers = {
  teamBoardState,
  teamAutoFlow,
  teamPlanUpdate,
  teamPlanApprove,
  teamPlanDiscard,
};
