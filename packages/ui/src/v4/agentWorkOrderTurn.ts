import type { AgentWorkOrderMeta, TurnHeaderRow } from "@zcode/shared/zcode-protocol-v4";

/**
 * 派单工单轮在转写里的规则（D29/D5）。
 *
 * 目标会话的工单唤醒轮没有可见用户行（carrier 是 model-only synthetic notice），
 * turnHeader（origin === "agentWorkOrder"）上的 `agentWorkOrder` 元数据是工单卡
 * （来源标注 + 任务正文，缩进/边框防冒充）的唯一数据源——与启动轮的 workflowLaunch
 * 元数据同一纪律：活投影权威，冷恢复写在同一份（冷热同形）。元数据缺席（旧 CLI /
 * 畸形帧）即退回普通渲染，UI 不从轮头文本反推工单内容。
 */
export function resolveAgentWorkOrderMeta(
  header: TurnHeaderRow | undefined,
): AgentWorkOrderMeta | undefined {
  if (header?.origin !== "agentWorkOrder") return undefined;
  return header.agentWorkOrder;
}

/** 回执轮（发起方）的来源取值：CLI 在 originMeta.backgroundSource 上权威给出。 */
export const AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE = "agent_work_order_receipt";
