// 派单命令组：dispatchAgentWorkOrder（D29/D5）。
// @ 菜单「派单」直达宿主能力：目标解析/最新 persona 会话定位/冷会话恢复/回执接线
// 全在协议派单端口（zcode-protocol/agent-dispatch-port.ts）里，core 的 AgentDispatch
// 工具与本命令共用同一条投递链——UI 不往 composer 塞文本（塞了就变点将语义）。
// 注：非输入类命令（不排队、不带 baseRevision），与 startSavedWorkflow 同类。
import type {
  CommandEnvelope,
  CommandPayloadMap,
  CommandResult,
} from "@zcode/shared/zcode-protocol-v4";
import { requireRecord } from "../record-access.js";
import type { V4CommandCoreHost } from "../types.js";

async function dispatchAgentWorkOrder(
  host: V4CommandCoreHost,
  envelope: CommandEnvelope,
): Promise<CommandResult | undefined> {
  const payload = envelope.payload as CommandPayloadMap["dispatchAgentWorkOrder"];
  // 发起方必须是真实会话：端口按 envelope.sessionId 惰性绑定归属 record，
  // 并把它的 persona（若有）签进工单信封、把它的会话 id 写成回执归还地址。
  const record = requireRecord(host, envelope.sessionId);
  if (!host.dispatchAgentWorkOrder) {
    throw new Error("v4 dispatchAgentWorkOrder requires host.dispatchAgentWorkOrder capability");
  }
  const result = await host.dispatchAgentWorkOrder(record.app.sessionId, payload);
  return {
    type: "dispatchAgentWorkOrder",
    targetSessionId: result.targetSessionId,
    agentName: result.agentName,
    delivery: result.delivery,
    createdSession: result.createdSession,
  };
}

export const agentWorkOrderHandlers = {
  dispatchAgentWorkOrder,
};
