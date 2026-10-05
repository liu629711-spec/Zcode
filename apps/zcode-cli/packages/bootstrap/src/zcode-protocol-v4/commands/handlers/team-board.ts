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

export const teamBoardHandlers = {
  teamBoardState,
};
