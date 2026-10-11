// 圆桌会控场命令组：councilPause / councilInterject（人工控场批）。
// 直呼 runtime 方法（同 file-rewind / session-mgmt 模式）：串行链、暂停闸门、
// 幂等与终态忽略都在 core（runInCouncilChain），这里只做定位与接线。
// 非输入类命令（不排队、不带 baseRevision，同 teamBoardState 族）；
// 会议未开/已终态时 core 如实忽略，ACK 照常返回。
import type {
  CommandEnvelope,
  CommandPayloadMap,
  CommandResult,
} from "@zcode/shared/zcode-protocol-v4";
import { requireRecord } from "../record-access.js";
import type { V4CommandCoreHost } from "../types.js";

async function councilPause(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["councilPause"];
  const record = requireRecord(host, envelope.sessionId);
  await record.app.runtime.setCouncilMeetingPaused({
    councilId: payload.councilId,
    paused: payload.paused,
    traceContext: record.traceContext,
  });
  return { type: "councilPause", paused: payload.paused };
}

async function councilInterject(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["councilInterject"];
  const record = requireRecord(host, envelope.sessionId);
  const targetedSeats = payload.targetSeatIndexes?.length ?? 0;
  await record.app.runtime.addCouncilInterjection({
    councilId: payload.councilId,
    text: payload.text,
    ...(payload.targetSeatIndexes ? { targetSeatIndexes: payload.targetSeatIndexes } : {}),
    traceContext: record.traceContext,
  });
  return { type: "councilInterject", targetedSeats };
}

export const councilControlHandlers = {
  councilPause,
  councilInterject,
} as const;
