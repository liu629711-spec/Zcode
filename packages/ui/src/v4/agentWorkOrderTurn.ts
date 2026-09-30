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
  /**
   * 失败回执的结构化线索（2026-10-01 员工可靠性批，CLI 只在 failed 时下发）：
   * 大白话映射 + 一键重派的权威数据源，UI 不从回执文本反推。
   */
  task?: string;
  failureCode?: string;
  failureModelId?: string;
  failureReason?: string;
  retried?: boolean;
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
    ...(originMeta.task ? { task: originMeta.task } : {}),
    ...(originMeta.failureCode ? { failureCode: originMeta.failureCode } : {}),
    ...(originMeta.failureModelId ? { failureModelId: originMeta.failureModelId } : {}),
    ...(originMeta.failureReason ? { failureReason: originMeta.failureReason } : {}),
    ...(originMeta.retried ? { retried: true } : {}),
  };
}

/**
 * 失败回执的大白话呈现（2026-10-01 员工可靠性批）。结构化线索全缺席（旧 CLI）一律
 * undefined——宁可不说，也不拿空话占位；已知分类码翻译成人话，其余给通用句式带原根因。
 */
export interface ReceiptFailurePresentation {
  /** 主短语的 i18n key（chat.receipt.failure.*）。 */
  messageId: string;
  /** 主短语的插值（modelId / reason）。 */
  values?: Record<string, string>;
  /** 原始根因（主短语已含 reason 时缺席，避免重复）。 */
  detail?: string;
  /** 已自动重试过一次仍失败（UI 在主短语前加一行说明）。 */
  retried: boolean;
}

export function describeReceiptFailure(
  meta: Pick<
    AgentWorkOrderReceiptMeta,
    "failureCode" | "failureModelId" | "failureReason" | "retried"
  >,
): ReceiptFailurePresentation | undefined {
  if (!meta.failureCode && !meta.failureReason && !meta.retried) return undefined;
  const retried = meta.retried === true;
  const modelId = meta.failureModelId?.trim();
  if (meta.failureCode === "invalid_model_request") {
    return {
      messageId: modelId
        ? "chat.receipt.failure.invalidModelRequest.named"
        : "chat.receipt.failure.invalidModelRequest",
      ...(modelId ? { values: { modelId } } : {}),
      ...(meta.failureReason ? { detail: meta.failureReason } : {}),
      retried,
    };
  }
  return {
    messageId:
      meta.failureReason
        ? "chat.receipt.failure.generic"
        : // retried-only / 畸形半截 originMeta：ICU 插值缺参会原样印 {reason}，
          // 没有根因就给没有插值的句式（评审 A 的 P3）。
          "chat.receipt.failure.genericNoReason",
    ...(meta.failureReason ? { values: { reason: meta.failureReason } } : {}),
    retried,
  };
}
