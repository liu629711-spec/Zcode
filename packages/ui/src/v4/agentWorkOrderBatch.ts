import type { ConversationTurnRenderUnit } from "@/v4/conversationTurnRenderUnits.js";
import type {
  AgentWorkOrderReceiptMeta,
} from "@/v4/agentWorkOrderTurn.js";
import {
  resolveAgentWorkOrderBatchQcMeta,
  resolveAgentWorkOrderReceiptMetaItems,
} from "@/v4/agentWorkOrderTurn.js";
import { parseFailedReceiptAgentName, parseReceiptDelivererName } from "@/v4/workOrderForward.js";

// ============================================================
// 批次派单（工地卡）纯规则：把一轮列表里的散单/回执按 batchId 聚成批次模型。
//
// 证据只有两处（都不从文本反推，与工单卡同一纪律）：
// 1. 发起方会话里成功的 AgentDispatch 工具行——输入带 batch_title，结果回显
//    workOrderId/batchId/batchTitle/agentName/delivery（CLI 权威铸造）；
// 2. 回执轮头 originMeta——backgroundSource === agent_work_order_receipt 且带批次。
// 纯函数无 React：Timeline 处算好，ConversationTurnGroup 只认渲染信息。
// ============================================================

export type WorkOrderBatchOrderStatus =
  | "dispatched"
  | "queued"
  | "completed"
  | "cancelled"
  | "failed";

/**
 * 评审结论（圆桌卡徽章，2026-10-02）：从评审人回执**第一行格式行**提取——
 * 「评审结论：通过 / 不通过 / 有条件通过」（评审要求块的协议行，CLI 下发）。
 * 提取不出（旧数据/模型没守格式）一律 undefined：宁可不戴徽章，也不从
 * 自由文本里猜结论。
 */
export type ReviewVerdict = "pass" | "fail" | "conditional";

export function extractReviewVerdict(text: string | undefined): ReviewVerdict | undefined {
  if (!text) return undefined;
  const lines = text.split("\n").map((line) => line.trim()).filter(Boolean).slice(0, 3);
  for (const line of lines) {
    const match = /^评审结论\s*[：:]\s*(有条件通过|不通过|通过)\s*$/u.exec(line);
    if (match) {
      const verdict = match[1];
      if (verdict === "通过") return "pass";
      if (verdict === "不通过") return "fail";
      return "conditional";
    }
  }
  return undefined;
}

/** 批次里的一张工单：先由派单行给出台账（dispatched/queued），回执到了更新终态。 */
export interface WorkOrderBatchOrder {
  key: string;
  agentName: string;
  kind: "work-order" | "receipt";
  status: WorkOrderBatchOrderStatus;
  receiptTitle?: string;
  /** 展示用摘要（有界）。 */
  receiptText?: string;
  /** 行内转交用的成果原文全文（completed 回执才带）+ 来源轮 key（工地卡定位用）。 */
  receiptAnswer?: string;
  receiptUnitKey?: string;
  /**
   * 失败回执的结构化线索（2026-10-01 员工可靠性批，随回执证据进批次模型）：
   * 行内大白话 + 一键重派（task = 原工单任务原文）。
   */
  task?: string;
  failureCode?: string;
  failureModelId?: string;
  failureReason?: string;
  retried?: boolean;
  /** 交活方工号（重派按号找人）；随失败回执证据进批次模型。 */
  agentId?: string;
  /** 评审结论徽章（仅评审单的 completed 回执带；提取不出缺席——不从文本猜）。 */
  verdict?: ReviewVerdict;
}

export interface WorkOrderBatchModel {
  batchId: string;
  /** 批次人类短标题（模型给 batch_title）；缺席时卡片退回通用标题。 */
  title?: string;
  orders: WorkOrderBatchOrder[];
  /** 批次墙钟（AgentCore 状态条同款）：各站轮头 startedAt 最小值 / endedAt 最大值。 */
  startedAtMs?: number;
  endedAtMs?: number;
  /**
   * 评审会批（2026-10-02）：派单行的 review=true 工具入参或合议轮头在场即成立——
   * 卡片据此换圆桌构图；普通施工批保持工地竖轨。缺席 = 工地。
   */
  review?: boolean;
  /**
   * 批次质检（纪律协议批）：质检轮的证据在场才有。质检结论正文在质检轮自己的
   * 「质检 · 标题」卡里渲染，工地卡只画状态灯（进行中/已完成/未完成——轮失败或
   * 被中断时绝不假报「完成」，评审 C1），不重复贴结论。
   *
   * skipped（全绿免检，2026-10-05）是 UI 的**确定性推导态**，不是轮头下发：
   * 免检批根本不开质检轮、没有任何轮头可读。推导判据=本批回执全部 completed
   * （结构化 receiptStatus）+ 质检轮头缺席 + 非评审批——与 core 证据门禁的
   * 跳过条件在稳态下一一对应（门禁只有零工具调用摸鱼/取消/失败才开轮，那些
   * 批次要么有非 completed 回执、要么质检轮头会到场）。误报窗口只剩质检轮
   * 头未及的亚秒瞬间与崩溃窗，与回执同为 best-effort 档。
   */
  qc?: {
    /** 质检轮 unit（工地卡 host 跟着它走到质检卡旁边）；skipped 时 = 批次 host 轮。 */
    unitKey: string;
    /** 质检轮头状态三态（轮头闭集坍缩：completedSuccess→done，running→running，其余→failed）；skipped=全绿免检推导态。 */
    state: "running" | "done" | "failed" | "skipped";
    /** 合议口味（评审会批）：CLI 的 originMeta.qcKind="review"，UI 据此换合议词表。 */
    review?: boolean;
  };
  /**
   * 批次卡挂载轮 = 批次证据**最后**出现的轮（最新回执/质检轮）：工地卡是一块活状态板，
   * 永远贴在批次活动的前沿，而不是钉在派单轮上被后续内容越顶越远。
   */
  hostUnitKey: string;
}

/** ConversationTurnGroup 的挂载/抑制信息：批次卡挂在 host，member 的散卡被压掉。 */
export interface WorkOrderBatchRenderInfo {
  batch: WorkOrderBatchModel;
  isHost: boolean;
  isMember: boolean;
}

/** 回执摘要在工地卡上的字符上界（整篇正文仍在回执轮内照常渲染，这里只截展示行）。 */
const WORK_ORDER_BATCH_SNIPPET_MAX_CHARS = 240;

function parseRecord(value: unknown): Record<string, unknown> | undefined {
  let candidate = value;
  if (typeof candidate === "string") {
    try {
      candidate = JSON.parse(candidate) as unknown;
    } catch {
      return undefined;
    }
  }
  return typeof candidate === "object" && candidate !== null && !Array.isArray(candidate)
    ? (candidate as Record<string, unknown>)
    : undefined;
}

function firstRecord(candidates: readonly unknown[]): Record<string, unknown> | undefined {
  for (const candidate of candidates) {
    const record = parseRecord(candidate);
    if (record) return record;
  }
  return undefined;
}

function readString(record: Record<string, unknown> | undefined, key: string): string | undefined {
  const value = record?.[key];
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : undefined;
}

function readBooleanTrue(record: Record<string, unknown> | undefined, key: string): boolean {
  return record?.[key] === true;
}

/** 回执标题里的交活方名字：completed 直接解；失败/中断按 CLI 后缀解，解不出留空。 */
function receiptAgentNameFromTitle(title: string): string {
  return parseReceiptDelivererName(title) ?? parseFailedReceiptAgentName(title) ?? "";
}

/**
 * 回执终态三分（地基清理 2026-10-04）：CLI 结构化的 receiptStatus 是权威；
 * 旧轮头没有它才回退认中文标题（标题由 CLI 权威铸造，仍是可靠回退）。
 * 交活=completed；被中断=cancelled（老板自己叫停的，不算干砸）；其余=failed。
 */
function resolveReceiptStatus(
  receipt: Pick<AgentWorkOrderReceiptMeta, "receiptStatus" | "title">,
): WorkOrderBatchOrderStatus {
  if (receipt.receiptStatus) return receipt.receiptStatus;
  return parseReceiptDelivererName(receipt.title)
    ? "completed"
    : receipt.title.includes("被中断")
      ? "cancelled"
      : "failed";
}

/** 是否交活终态（completed）：优先结构化字段，旧轮头回退标题解析。 */
function isReceiptCompleted(receipt: Pick<AgentWorkOrderReceiptMeta, "receiptStatus" | "title">): boolean {
  return resolveReceiptStatus(receipt) === "completed";
}

function receiptFullText(unit: ConversationTurnRenderUnit): string | undefined {
  const text = unit.latestAssistantTextRow?.text ?? unit.assistantTextRows.at(-1)?.text;
  return text?.trim() || undefined;
}

function receiptSnippet(unit: ConversationTurnRenderUnit): string | undefined {
  const trimmed = receiptFullText(unit);
  if (!trimmed) return undefined;
  return trimmed.length > WORK_ORDER_BATCH_SNIPPET_MAX_CHARS
    ? `${trimmed.slice(0, WORK_ORDER_BATCH_SNIPPET_MAX_CHARS - 1)}…`
    : trimmed;
}

interface BatchDraft {
  title?: string;
  orders: Map<string, WorkOrderBatchOrder>;
  review?: boolean;
  qc?: { unitKey: string; state: "running" | "done" | "failed" | "skipped"; review?: boolean };
  memberUnitKeys: Set<string>;
  hostUnitKey: string;
  /** 批次墙钟（AgentCore 状态条同款）：各站轮头 startedAt 的最小值 / endedAt 的最大值。 */
  startedAtMs?: number;
  endedAtMs?: number;
}

/** 站头自带权威工时（startedAt/endedAt），批次卡只做 min/max 聚合，冷热同形。 */
function trackBatchUnitTime(draft: BatchDraft, unit: ConversationTurnRenderUnit): void {
  const startedAt = unit.header?.startedAt;
  if (typeof startedAt === "number") {
    draft.startedAtMs =
      draft.startedAtMs === undefined ? startedAt : Math.min(draft.startedAtMs, startedAt);
  }
  const endedAt = unit.header?.endedAt;
  if (typeof endedAt === "number") {
    draft.endedAtMs =
      draft.endedAtMs === undefined ? endedAt : Math.max(draft.endedAtMs, endedAt);
  }
}

function mergeOrder(draft: BatchDraft, order: WorkOrderBatchOrder, unitKey: string): void {
  draft.memberUnitKeys.add(unitKey);
  const existing = draft.orders.get(order.key);
  if (!existing) {
    draft.orders.set(order.key, order);
    return;
  }
  // 回执是终态证据：状态/回执字段以它为准；派单行只在缺署名时补——
  // 反过来（派单行覆盖回执）会把已交的活打回"在跑"。
  draft.orders.set(order.key, {
    ...existing,
    ...order,
    ...(existing.kind === "work-order" && order.kind === "receipt" && !order.agentName
      ? { agentName: existing.agentName }
      : {}),
  });
}

/** 批次最少要 2 张工单才聚合（单张照旧画散卡，聚合没有收益）。 */
const WORK_ORDER_BATCH_MIN_ORDERS = 2;

export function selectWorkOrderBatches(
  units: readonly ConversationTurnRenderUnit[],
): WorkOrderBatchModel[] {
  const drafts = new Map<string, BatchDraft>();
  const ensureDraft = (batchId: string, unit: ConversationTurnRenderUnit): BatchDraft => {
    let draft = drafts.get(batchId);
    if (!draft) {
      draft = { orders: new Map(), memberUnitKeys: new Set(), hostUnitKey: unit.key };
      drafts.set(batchId, draft);
      trackBatchUnitTime(draft, unit);
      return draft;
    }
    // host 跟着最新证据走：回执每到一轮，工地卡就搬一轮——状态板贴活边。
    draft.hostUnitKey = unit.key;
    trackBatchUnitTime(draft, unit);
    return draft;
  };

  for (const unit of units) {
    // 证据零：质检轮（纪律协议批）。批次收口后的自动验货：工地卡跟着走到质检卡
    // 旁边并画状态灯；**不 continue**——质检轮里的打回重派 AgentDispatch 工具行
    // 是新订单证据，落到下面的工单行扫描里照常聚合。
    const qc = resolveAgentWorkOrderBatchQcMeta(unit.header);
    if (qc?.batchId) {
      const draft = ensureDraft(qc.batchId, unit);
      draft.title ??= qc.batchTitle;
      // 合议轮头本身就是评审会证据（合议只对评审会批开）。
      if (qc.review) draft.review = true;
      draft.qc = {
        unitKey: unit.key,
        state:
          unit.header?.state === "running"
            ? "running"
            : unit.header?.state === "completedSuccess"
              ? "done"
              : // failed / completedInterrupted / 畸形缺席：没验成就是没验成，不假报完成。
                "failed",
        ...(qc.review ? { review: true } : {}),
      };
    }

    // 证据一：回执轮头（发起方会话）。终态权威，先到先记账也行——合并规则让回执赢。
    // 合并回执轮（洪泛合并 2026-10-05）逐张记账：轮头 originMeta.receipts 数组
    // 每张是一条独立证据；缺数组（单张/旧轮头）回退单张解析。合并轮的正文是
    // 一整段模型回复，逐张归属不可分——摘要与成果全文只挂单张轮，合并张不挂。
    const receiptItems = resolveAgentWorkOrderReceiptMetaItems(unit.header);
    const isMergedReceiptUnit = receiptItems.length > 1;
    for (const receipt of receiptItems) {
      if (!receipt.batchId) continue;
      const draft = ensureDraft(receipt.batchId, unit);
      draft.title ??= receipt.batchTitle;
      mergeOrder(
        draft,
        {
          key: receipt.workOrderId,
          agentName: receiptAgentNameFromTitle(receipt.title),
          kind: "receipt",
          // 终态三分（地基清理 2026-10-04）：结构化 receiptStatus 权威，旧轮头
          // 回退标题解析。cancelled 曾被画成红色"未完成"，误导老板（audit D P2-4）。
          status: resolveReceiptStatus(receipt),
          receiptTitle: receipt.title,
          ...(!isMergedReceiptUnit && receiptSnippet(unit)
            ? { receiptText: receiptSnippet(unit) }
            : {}),
          // 全文只在 completed（转交语义=成果已交付）时挂：失败/中断没有可转交的成果。
          ...(!isMergedReceiptUnit && isReceiptCompleted(receipt) && receiptFullText(unit)
            ? { receiptAnswer: receiptFullText(unit), receiptUnitKey: unit.key }
            : {}),
          // 失败线索与原任务（一键重派）只在 failed 回执上在场（CLI 权威下发）。
          ...(receipt.task ? { task: receipt.task } : {}),
          ...(receipt.failureCode ? { failureCode: receipt.failureCode } : {}),
          ...(receipt.failureModelId ? { failureModelId: receipt.failureModelId } : {}),
          ...(receipt.failureReason ? { failureReason: receipt.failureReason } : {}),
          ...(receipt.retried ? { retried: true } : {}),
          ...(receipt.agentId ? { agentId: receipt.agentId } : {}),
          // 评审结论徽章（圆桌卡）：只认第一行格式行，提取不出不带。
          ...(!isMergedReceiptUnit && isReceiptCompleted(receipt)
            ? (() => {
                const verdict = extractReviewVerdict(receiptFullText(unit));
                return verdict ? { verdict } : {};
              })()
            : {}),
        },
        unit.key,
      );
    }
    if (receiptItems.length > 0) {
      continue;
    }

    // 证据二：本轮成功的 AgentDispatch 工具行（同轮可并发多张，逐行收）。
    for (const row of unit.assistantWorkRows) {
      if (row.kind !== "toolCall" || row.status !== "success") continue;
      if (row.toolName.toLowerCase().replace(/[^a-z0-9]/gu, "") !== "agentdispatch") continue;
      const input = firstRecord([row.input, row.inputText]);
      const output = firstRecord([row.output?.text]);
      const batchId = readString(output, "batchId") ?? readString(input, "batch_id");
      const title = readString(output, "batchTitle") ?? readString(input, "batch_title");
      if (!batchId) continue;
      const draft = ensureDraft(batchId, unit);
      draft.title ??= title;
      // 评审会证据：派单工具入参带 review=true（CLI 权威铸造，结构化非文本反推）。
      if (readBooleanTrue(input, "review") || readBooleanTrue(output, "review")) {
        draft.review = true;
      }
      const workOrderId = readString(output, "workOrderId");
      mergeOrder(
        draft,
        {
          key: workOrderId ?? `tc:${row.toolCallId}`,
          agentName: readString(output, "agentName") ?? readString(input, "agent") ?? "",
          kind: "work-order",
          status: readString(output, "delivery") === "queued" ? "queued" : "dispatched",
        },
        unit.key,
      );
    }
  }

  const models: WorkOrderBatchModel[] = [];
  for (const [batchId, draft] of drafts) {
    if (draft.orders.size < WORK_ORDER_BATCH_MIN_ORDERS) continue;
    const orders = [...draft.orders.values()];
    // 全绿免检推导（2026-10-05）：没有质检轮头、非评审批、回执全部 completed 时，
    // 批次画「免检」态而不是没有质检灯（字段注释里有推导纪律与误报窗口的边界）。
    const qc =
      draft.qc ??
      (!draft.review &&
      orders.length > 0 &&
      orders.every((order) => order.status === "completed")
        ? { unitKey: draft.hostUnitKey, state: "skipped" as const }
        : undefined);
    models.push({
      batchId,
      ...(draft.title ? { title: draft.title } : {}),
      orders,
      ...(draft.review ? { review: true } : {}),
      ...(qc ? { qc } : {}),
      ...(draft.startedAtMs !== undefined ? { startedAtMs: draft.startedAtMs } : {}),
      ...(draft.endedAtMs !== undefined ? { endedAtMs: draft.endedAtMs } : {}),
      hostUnitKey: draft.hostUnitKey,
    });
  }
  return models;
}

/**
 * 批次墙钟的展示格式（AgentCore 状态条同款等宽短格式）：一分钟内只显秒，跨分显
 * m/ss，跨小时显 h/mm。数字贴在进度旁自会说话，不占词条。
 */
export function formatBatchDuration(durationMs: number): string {
  if (!Number.isFinite(durationMs) || durationMs < 0) return "—";
  const totalSeconds = Math.floor(durationMs / 1000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);
  const twoDigits = (value: number) => (value < 10 ? `0${value}` : `${value}`);
  if (hours > 0) return `${hours}h${twoDigits(minutes)}m`;
  if (totalMinutes > 0) return `${minutes}m${twoDigits(seconds)}s`;
  return `${seconds}s`;
}

/** 每轮的挂载/抑制信息（Timeline 算一次，ConversationTurnGroup 查表）。 */
export function selectWorkOrderBatchRenderInfo(
  units: readonly ConversationTurnRenderUnit[],
): Map<string, WorkOrderBatchRenderInfo[]> {
  const infoByUnitKey = new Map<string, WorkOrderBatchRenderInfo[]>();
  for (const batch of selectWorkOrderBatches(units)) {
    for (const unitKey of collectMemberUnitKeys(units, batch)) {
      const infos = infoByUnitKey.get(unitKey) ?? [];
      infos.push({
        batch,
        isHost: unitKey === batch.hostUnitKey,
        isMember: true,
      });
      infoByUnitKey.set(unitKey, infos);
    }
  }
  return infoByUnitKey;
}

function collectMemberUnitKeys(
  units: readonly ConversationTurnRenderUnit[],
  batch: WorkOrderBatchModel,
): Set<string> {
  // member = 该轮为批次贡献过证据。重扫一遍比在 draft 里多存一份更省心：
  // 批次总数小（同会话同批的轮寥寥），O(batches × units) 只在批次存在时发生。
  const memberUnitKeys = new Set<string>();
  const orderKeys = new Set(batch.orders.map((order) => order.key));
  for (const unit of units) {
    // 质检轮是批次证据轮（工地卡跟到它旁边挂载）——必须进 member 集，否则
    // 挂载表里没有这个 key、工地卡会消失。member 的压制只作用于回执散卡
    // （isAgentWorkOrderReceiptResult 把门），质检轮自己的「质检」标题卡照常渲染。
    if (resolveAgentWorkOrderBatchQcMeta(unit.header)?.batchId === batch.batchId) {
      memberUnitKeys.add(unit.key);
      continue;
    }
    const receiptItems = resolveAgentWorkOrderReceiptMetaItems(unit.header);
    if (
      receiptItems.some(
        (receipt) => receipt.batchId === batch.batchId && orderKeys.has(receipt.workOrderId),
      )
    ) {
      memberUnitKeys.add(unit.key);
      continue;
    }
    for (const row of unit.assistantWorkRows) {
      if (row.kind !== "toolCall" || row.status !== "success") continue;
      if (row.toolName.toLowerCase().replace(/[^a-z0-9]/gu, "") !== "agentdispatch") continue;
      const input = firstRecord([row.input, row.inputText]);
      const output = firstRecord([row.output?.text]);
      const workOrderId = readString(output, "workOrderId");
      const key = workOrderId ?? `tc:${row.toolCallId}`;
      if (
        (readString(output, "batchId") ?? readString(input, "batch_id")) === batch.batchId &&
        orderKeys.has(key)
      ) {
        memberUnitKeys.add(unit.key);
        break;
      }
    }
  }
  return memberUnitKeys;
}
