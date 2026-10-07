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

/** 批次质检轮（发起方，纪律协议批）的来源取值：同上，CLI 权威给出。 */
export const AGENT_WORK_ORDER_BATCH_QC_BACKGROUND_SOURCE = "agent_work_order_batch_qc";

/** 批次质检轮头里与批次对账的元数据（originMeta.workId ≡ batchId；结论走本轮正文）。 */
export interface AgentWorkOrderBatchQcMeta {
  batchId: string;
  title: string;
  batchTitle?: string;
  /** 合议口味（评审会批）：originMeta.qcKind === "review"，UI 据此换合议词表。 */
  review?: boolean;
}

/**
 * 批次质检轮的对账元数据（发起方会话）。来源不符或缺 workId 一律 undefined
 * （调用方退回普通渲染，不从标题反推批次）。
 */
export function resolveAgentWorkOrderBatchQcMeta(
  header: TurnHeaderRow | undefined,
): AgentWorkOrderBatchQcMeta | undefined {
  if (header?.origin !== "backgroundResult") return undefined;
  const originMeta = header.originMeta;
  if (originMeta?.backgroundSource !== AGENT_WORK_ORDER_BATCH_QC_BACKGROUND_SOURCE) {
    return undefined;
  }
  if (!originMeta.workId.trim()) return undefined;
  return {
    batchId: originMeta.workId,
    title: originMeta.title,
    ...(originMeta.batchTitle ? { batchTitle: originMeta.batchTitle } : {}),
    ...(originMeta.qcKind === "review" ? { review: true } : {}),
  };
}

/** 派单回执轮头里与工单对账的元数据（originMeta.workId ≡ workOrderId；批次同源于信封）。 */
export interface AgentWorkOrderReceiptMeta {
  workOrderId: string;
  title: string;
  /**
   * 回执终态（地基清理 2026-10-04，CLI 结构化下发）：三态灯的权威来源。
   * 缺席（旧 CLI / 旧轮头）= 调用方回退认中文标题。
   */
  receiptStatus?: "completed" | "failed" | "cancelled";
  /** 原工单批次（工地卡聚合键）；缺席 = 散单（旧 CLI / 单发）。 */
  batchId?: string;
  batchTitle?: string;
  /** 评审单标记（随信封下发）：一键重派透传 review 的权威数据源。 */
  review?: true;
  /**
   * 失败回执的结构化线索（2026-10-01 员工可靠性批，CLI 只在 failed 时下发）：
   * 大白话映射 + 一键重派的权威数据源，UI 不从回执文本反推。
   */
  task?: string;
  failureCode?: string;
  failureModelId?: string;
  failureReason?: string;
  retried?: boolean;
  /** 交活方工号（一键重派按号找人，改名不误派；audit 2026-10-01）。 */
  agentId?: string;
  /** 交活方档案名（标题显示时本地化的结构化数据源，2026-10-07）。 */
  agentName?: string;
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
    ...(originMeta.receiptStatus ? { receiptStatus: originMeta.receiptStatus } : {}),
    ...(originMeta.batchId ? { batchId: originMeta.batchId } : {}),
    ...(originMeta.batchTitle ? { batchTitle: originMeta.batchTitle } : {}),
    ...(originMeta.review ? { review: true as const } : {}),
    ...(originMeta.task ? { task: originMeta.task } : {}),
    ...(originMeta.failureCode ? { failureCode: originMeta.failureCode } : {}),
    ...(originMeta.failureModelId ? { failureModelId: originMeta.failureModelId } : {}),
    ...(originMeta.failureReason ? { failureReason: originMeta.failureReason } : {}),
    ...(originMeta.retried ? { retried: true } : {}),
    ...(originMeta.agentId ? { agentId: originMeta.agentId } : {}),
    ...(originMeta.agentName ? { agentName: originMeta.agentName } : {}),
  };
}

/**
 * 回执轮的逐张对账元数据（回执洪泛合并 2026-10-05）：合并轮读轮头
 * `originMeta.receipts`（CLI 逐张下发，receiptStatus 必带），缺数组的单张轮/
 * 旧轮头回退单张解析。来源不符 = 空数组（调用方退回普通渲染，不从文本反推）。
 */
export function resolveAgentWorkOrderReceiptMetaItems(
  header: TurnHeaderRow | undefined,
): AgentWorkOrderReceiptMeta[] {
  if (header?.origin !== "backgroundResult") return [];
  const originMeta = header.originMeta;
  if (originMeta?.backgroundSource !== AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE) return [];
  const items = originMeta.receipts;
  if (Array.isArray(items) && items.length > 0) {
    return items.map((item) => ({
      workOrderId: item.workId,
      title: item.title,
      receiptStatus: item.receiptStatus,
      ...(item.batchId ? { batchId: item.batchId } : {}),
      ...(item.batchTitle ? { batchTitle: item.batchTitle } : {}),
      ...(item.review ? { review: true as const } : {}),
      ...(item.task ? { task: item.task } : {}),
      ...(item.failureCode ? { failureCode: item.failureCode } : {}),
      ...(item.failureModelId ? { failureModelId: item.failureModelId } : {}),
      ...(item.failureReason ? { failureReason: item.failureReason } : {}),
      ...(item.retried ? { retried: true } : {}),
      ...(item.agentId ? { agentId: item.agentId } : {}),
      ...(item.agentName ? { agentName: item.agentName } : {}),
    }));
  }
  const single = resolveAgentWorkOrderReceiptMeta(header);
  return single ? [single] : [];
}

/** 组合器共用的最小 intl 视图（值替换走 {placeholder} 槽）。 */
interface TitleIntl {
  formatMessage: (
    desc: { id: string },
    values?: Record<string, string | number>,
  ) => string;
}

/**
 * 回执轮头标题的显示时本地化（2026-10-07）：结构化字段（receiptStatus + agentName）
 * 齐全才组合（中文输出与 CLI 铸造原文逐字一致，en 换词表）；旧轮头缺结构化字段
 * 一律 undefined，调用方退回原文——不从标题文本反推。
 */
export function composeAgentWorkOrderReceiptHeadTitle(
  intl: TitleIntl,
  meta: { receiptStatus?: "completed" | "failed" | "cancelled"; agentName?: string; title: string },
): string | undefined {
  const name = meta.agentName?.trim();
  if (!name || !meta.receiptStatus) return undefined;
  const key =
    meta.receiptStatus === "completed"
      ? "chat.receipt.headTitle.completed"
      : meta.receiptStatus === "cancelled"
        ? "chat.receipt.headTitle.cancelled"
        : "chat.receipt.headTitle.failed";
  return intl.formatMessage({ id: key }, { name });
}

/**
 * 批次质检/合议轮头标题的显示时本地化：qcKind 结构化分流口味，batchTitle 有则带
 * 参数组合、无则走整词键（与 CLI 铸造的缺省词逐字对齐）。
 */
export function composeAgentWorkOrderBatchQcHeadTitle(
  intl: TitleIntl,
  meta: { qcKind?: "review"; batchTitle?: string },
): string {
  const review = meta.qcKind === "review";
  const batchTitle = meta.batchTitle?.trim();
  if (batchTitle) {
    return intl.formatMessage(
      { id: review ? "chat.workOrderBatch.qcTitleReview" : "chat.workOrderBatch.qcTitle" },
      { title: batchTitle },
    );
  }
  return intl.formatMessage({
    id: review ? "chat.workOrderBatch.qcTitleReviewFallback" : "chat.workOrderBatch.qcTitleFallback",
  });
}

/**
 * 圆桌会主席轮头标题的显示时本地化：议题只随标题文本下发（CLI 铸造原文是唯一
 * 载体），这里按铸造格式剥离已知中文前缀换词表——前缀对不上（旧数据/异形）退回
 * 原文。与 workOrderForward 的 legacy 标题反解同一档：显示层回退，不作权威依据。
 */
export function composeCouncilModerationHeadTitle(
  intl: TitleIntl,
  meta: { councilKind?: "plan" | "acceptance"; title: string },
): string | undefined {
  const raw = meta.title;
  const topic = raw.startsWith("圆桌会 · ") ? raw.slice("圆桌会 · ".length).trim() : "";
  if (topic) {
    return intl.formatMessage({ id: "chat.council.moderationTitle" }, { title: topic });
  }
  if (raw === "圆桌会验收" || raw === "圆桌会评审") {
    return intl.formatMessage({
      id: raw === "圆桌会验收" ? "chat.council.moderationFallbackAcceptance" : "chat.council.moderationFallbackPlan",
    });
  }
  return undefined;
}

/**
 * 后台结果轮头标题的显示时本地化总入口：只接派单家族（回执/质检合议/圆桌会主席），
 * bash/subagent/workflow 一律 undefined（标题本就是权威原文）。返回 undefined =
 * 调用方显示铸造原文。
 */
export function composeAgentWorkOrderHeadTitle(
  intl: TitleIntl,
  originMeta: TurnHeaderRow["originMeta"],
): string | undefined {
  if (!originMeta) return undefined;
  if (originMeta.backgroundSource === AGENT_WORK_ORDER_RECEIPT_BACKGROUND_SOURCE) {
    return composeAgentWorkOrderReceiptHeadTitle(intl, originMeta);
  }
  if (originMeta.backgroundSource === AGENT_WORK_ORDER_BATCH_QC_BACKGROUND_SOURCE) {
    if (originMeta.councilId && originMeta.councilPhase === "moderation") {
      return composeCouncilModerationHeadTitle(intl, {
        ...(originMeta.councilKind ? { councilKind: originMeta.councilKind } : {}),
        title: originMeta.title,
      });
    }
    return composeAgentWorkOrderBatchQcHeadTitle(intl, originMeta);
  }
  return undefined;
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
