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

/** 派单回执轮头里与工单对账的元数据（originMeta.workId ≡ workOrderId；批次同源于信封）。 */
export interface AgentWorkOrderReceiptMeta {
  workOrderId: string;
  title: string;
  /** 原工单批次（工地卡聚合键）；缺席 = 散单（旧 CLI / 单发）。 */
  batchId?: string;
  batchTitle?: string;
}

/**
 * 回执轮的工单对账元数据（发起方会话）。回执的 envelope 落在消息 metadata（model-only，
 * 不进 rows），UI 唯一可见的权威来源是轮头 originMeta——CLI 已把批次写在上面。
 * 来源不符或缺 workId 一律 undefined（调用方退回普通渲染，不从标题反推批次）。
 */
export function resolveAgentWorkOrderReceiptMeta(
  header: TurnHeaderRow | undefined,
): AgentWorkOrderReceiptMeta | undefined {
  if (header?.origin !== "backgroundResult") return undefined;
  const originMeta = header.originMeta;
  if (originMeta?.backgroundSource !== AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE) return undefined;
  if (!originMeta.workId.trim()) return undefined;
  return {
    workOrderId: originMeta.workId,
    title: originMeta.title,
    ...(originMeta.batchId ? { batchId: originMeta.batchId } : {}),
    ...(originMeta.batchTitle ? { batchTitle: originMeta.batchTitle } : {}),
  };
}
