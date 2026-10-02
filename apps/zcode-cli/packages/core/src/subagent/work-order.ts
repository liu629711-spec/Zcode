// ============================================================
// 派单（D29）纯规则：目标解析、工单信封拼装、工单轮身份判定。
// ============================================================

import type { AgentProfile } from "./profile.js";
import type { ModelSelection } from "@zcode/shared";
import { WORK_ORDER_INPUT_ID_PREFIX, type AgentWorkOrderEnvelope } from "@zcode/contracts";

// 前缀单一来源在 contracts（turn-loop state 与 bootstrap 端口自检共用同一词表）。
export { WORK_ORDER_INPUT_ID_PREFIX };

// 目标解析判据（与点将的宽容匹配刻意不同——工单永不落错人）：
// - uuid 形状 → 按工号（agentId）在全量现役档案里唯一匹配；
// - 其余 → 精确名匹配，限驻场身份档（项目档 + 用户级档，身份轴终局 §九③——
//   全局员工在每个工作区都可按名字接单；插件档 source "plugin" 不是身份，不参与）；
// - 命中 0 个或多个一律拒绝（Hermes 教训：歧义名字拒绝解析）。
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export type WorkOrderTargetResolution =
  | { kind: "resolved"; profile: AgentProfile }
  | { kind: "not_found"; agent: string }
  | { kind: "ambiguous"; agent: string; matchedNames: string[] };

export function isAgentIdLike(candidate: string): boolean {
  return UUID_PATTERN.test(candidate.trim());
}

export function resolveWorkOrderTarget(
  agent: string,
  profiles: readonly AgentProfile[],
): WorkOrderTargetResolution {
  const candidate = agent.trim();
  if (candidate.length === 0) {
    return { kind: "not_found", agent };
  }
  const matches = isAgentIdLike(candidate)
    ? profiles.filter((profile) => profile.agentId?.toLowerCase() === candidate.toLowerCase())
    : profiles.filter(
        (profile) =>
          // 身份档 = project + user + 内置班底（2026-09-30 内置虚拟化）：三者都有
          // 工牌、会话与记忆，按名字都可命中。真机事故：漏掉 built-in 让派单方
          // 拿到「没有注册 prd-engineer」，转头抓了个临时工冒名顶替。
          // 插件档不是身份（升级会覆写），维持排除。
          (profile.source === "project" ||
            profile.source === "user" ||
            profile.source === "built-in") &&
          profile.name.trim().toLowerCase() === candidate.toLowerCase(),
      );
  if (matches.length === 1) {
    return { kind: "resolved", profile: matches[0]! };
  }
  if (matches.length > 1) {
    return { kind: "ambiguous", agent: candidate, matchedNames: matches.map((m) => m.name) };
  }
  return { kind: "not_found", agent: candidate };
}

/**
 * 自派拒绝（对齐稿 §三规则 6：员工点名自己 → 拒绝）：给自己派工单只会复制
 * 自己的对话，端口在构造信封前用这道纯判据拦下。判据与 personaMatchesProfile
 * 同纪律：两边都有工号只认号；任一侧无号才比名字。发起方缺席（用户会话、
 * 无 persona 快照）不算自派。
 */
export function isSelfDispatch(
  sourcePersona: { name: string; agentId?: string } | undefined,
  profile: { name: string; agentId?: string },
): boolean {
  if (!sourcePersona) return false;
  if (sourcePersona.agentId !== undefined && profile.agentId !== undefined) {
    return sourcePersona.agentId === profile.agentId;
  }
  return sourcePersona.name === profile.name;
}

// ── 员工默认模型护栏（2026-10-01）：派单当场校验，别等跑起来才炸 ─────
// 真机事故 2026-09-30：员工配的模型已下线，工单轮三连炸 "Provider rejected the
// model request"（invalid_model_request 在模型层被判不可重试），工单直接死。

/** 模型护栏的裁决：工单实际要跑的模型 + 它从哪来（端口据此决定 per-order 覆盖与日志）。 */
export interface DispatchModelResolution {
  /** 模型终值；undefined = 连回落目标都没有，交给会话缺省机制。 */
  modelSelection: ModelSelection | undefined;
  /**
   * 终值来源：requested=本单指定（D32）且有效；fallback=跟随发起会话当前模型
   * （老板正用它说话，必然可用——2026-10-02 拍板后这就是"没指定"的默认值）。
   */
  source: "requested" | "fallback";
  /** source=fallback 时的原因：unset=本单没指定（正常默认）；unavailable=指定的模型不在注册表里（下线/没钱/拼错）。 */
  reason?: "unset" | "unavailable";
  /** true = 指定的模型没过校验又没有回落目标——只能带着坏候选上场（端口据此告警）。 */
  unavailable?: boolean;
}

/**
 * 派单模型裁决（2026-10-02 老板拍板：**默认跟随发起会话当前模型**）：
 * 候选只有本单指定（requested）；缺席 → 主会话当前模型（"我选了哪个，大家都用
 * 哪个"）；指定了但不在注册表里（下线/没钱/拼错）→ 同样回落主会话当前模型并
 * 留痕，绝不带着坏候选上场——除非连回落目标都没有（unavailable，端口告警）。
 * 员工档案自配的模型**不再自动候选**（前口径"档案默认在没指定时当候选"废止：
 * 各员工档案押不同供应商，有的一分钱不剩，默认分散不合理）。要跑某个特定模型，
 * 派单时明说（工具 model 参数 / 图纸 subagent_model）。
 */
export function resolveDispatchModelSelection(input: {
  /** 本单指定的模型（工单参数 model_selection）；派单方明确点名的那一个。 */
  requested?: ModelSelection;
  /** 回落目标：主会话当前模型（既是"没指定"的默认值，也是"指定失效"的回落值）。 */
  fallback?: ModelSelection;
  /** 模型可用性判据；缺席 = 没法校验。 */
  isModelAvailable?: (selection: ModelSelection) => boolean;
}): DispatchModelResolution {
  const { requested, fallback, isModelAvailable } = input;
  if (requested === undefined) {
    return fallback
      ? { modelSelection: fallback, source: "fallback", reason: "unset" }
      : { modelSelection: undefined, source: "fallback", reason: "unset" };
  }
  if (!isModelAvailable || isModelAvailable(requested)) {
    return { modelSelection: requested, source: "requested" };
  }
  return fallback
    ? { modelSelection: fallback, source: "fallback", reason: "unavailable" }
    : { modelSelection: requested, source: "requested", unavailable: true };
}

/**
 * 信封同族标签的中和：`<work-order…>` 与 `<work-order-receipt…>` 开合全算。
 * 两个方向（工单正文、回执正文）用同一条规则——只中和本族标签是不对称的：
 * 员工最终回复里夹带的完整伪造 `<work-order>` 信封会原样进发起方上下文
 * （审计 2026-10-01 P1-2）。`\b` 在 "r"+"-" 处成界，前缀彼此互染，索性一族全收。
 */
const ENVELOPE_TAG_NEUTRALIZATION = /<\/?work-order(?:-receipt)?\b/gi;

/**
 * 批次质检信封（纪律协议批）的同族中和：员工答案会进发起方上下文，夹带的伪造
 * `<batch-qc>` 信封会在质检轮里被当成真的验货输入（注入面与 work-order 家族同型）。
 */
const BATCH_QC_TAG_NEUTRALIZATION = /<\/?batch-qc\b/gi;

function neutralizeEnvelopeTags(text: string): string {
  return text.replace(ENVELOPE_TAG_NEUTRALIZATION, (tag) => tag.replace("<", "&lt;"));
}

function neutralizeBatchQcTags(text: string): string {
  return text.replace(BATCH_QC_TAG_NEUTRALIZATION, (tag) => tag.replace("<", "&lt;"));
}

/**
 * 信封属性转义：from-agent 是档案名（仓库 .md 档案可带任意字符，解析端不做
 * 字符集校验），原样插进 HTML 属性会被用来提前闭合/伪造信封头（审计 P2-2）。
 * @点将解析端字符集严格、档案解析端宽松——两个口径在消费端靠转义对齐。
 */
function escapeEnvelopeAttribute(value: string): string {
  return value.replace(/[&<>"]/gu, (ch) => `&#${ch.charCodeAt(0)};`);
}

/**
 * 工单正文（目标会话的模型输入 carrier）。自带结构化信封标签（谁派的单），
 * 与 task-notification 同一家族：不走 <system-reminder> 包装，正文自证来源。
 * 任务正文里出现的同形关闭标签一律中和，防伪造信封边界。
 */
export function buildWorkOrderEnvelopeText(envelope: AgentWorkOrderEnvelope): string {
  const task = neutralizeEnvelopeTags(envelope.task);
  const fromLabel = escapeEnvelopeAttribute(envelope.fromAgentName.trim() || "user");
  const lines = [
    `<work-order id="${envelope.workOrderId}" from-agent="${fromLabel}" from-session="${envelope.fromSessionId}">`,
    task,
    "</work-order>",
  ];
  // 评审单（评审会批 2026-10-02）：评审要求跟在标签外——事件面把标签内原文当
  // 「任务正文」透出，里面不能掺指令（与下面的语言硬要求同一纪律）。
  if (envelope.review === true) {
    lines.push(
      "评审要求（本单是评审单：你是评审人，不是施工人）：",
      "1. 只评审，不动手：不要修改文件、不要动手修复发现的问题。",
      "2. 结论先行：第一行只写格式行「评审结论：通过」或「评审结论：不通过」或「评审结论：有条件通过」，三选一，系统按这行归档；另起一段再写理由。",
      "3. 理由分条讲事实（指出具体文件、行为或复现步骤），不写空评。",
      "4. 必须给一条「最不放心的地方」：哪怕结论是通过，也要说出你最担心的一点。",
    );
  }
  lines.push(
    // 桌子铁规矩（2026-10-02 真机实证：弱模型把 PRD 产出写进老板家目录的绝对路径）
    // ——产出一律写进当前工作区；记忆另有专属柜子（双层记忆说明书里给路径）。
    "产出要求：所有产出文件一律写进你当前的工作区目录内（用相对路径）；用户主目录与任何工作区外的绝对路径不是你的桌子，不要往那里写文件。",
    // 语言随工单（真机 2026-09-30）：弱模型拿着英文系统提示，中文工单也回英文，
    // 还自由发挥成自我介绍。信封后缀一行硬要求：语言跟工单走，没交代的事不做。
    // 放在标签外——事件面把标签内原文当「任务正文」透出，里面不能掺指令。
    "回复要求：使用与上面工单正文相同的语言；工单未交代的事项不要自行发挥。",
  );
  return lines.join("\n");
}

/** 工单唤醒轮禁用的工具（嵌套上限=1 的 denylist 哨兵）。 */
export const WORK_ORDER_RESTRICTED_TOOL_NAMES = ["AgentDispatch"] as const;

export function isWorkOrderInputId(value: string | undefined): boolean {
  return typeof value === "string" && value.trim().startsWith(WORK_ORDER_INPUT_ID_PREFIX);
}

// ── 派单回执（D29/D3）纯规则 ────────────────────────────────────────

/** 目标轮的终态口径：completed 带最终答案本体；cancelled/failed 如实带原因。 */
export interface WorkOrderReceiptOutcome {
  status: "completed" | "cancelled" | "failed";
  /** status=completed 的最终答案本体（D3：回执携带最终答案）。 */
  response?: string;
  /** status=failed 的原因（照 Codex 错误指引口径：如实告知，指引下一步）。 */
  reason?: string;
  /**
   * status=failed 的结构化线索（2026-10-01 员工可靠性批）：随回执轮头 originMeta
   * 下发，UI 据此讲大白话/给一键重派，不从回执文本反推。
   */
  /** 根因分类码（TurnError 载荷 error.code，如 invalid_model_request）。 */
  failureCode?: string;
  /** 被拒的模型 id（error.attribution.modelId——模型被供应商拒收时讲人话用）。 */
  failureModelId?: string;
  /** 已自动重试过一次仍失败：信封与回执卡措辞据此升级，不谎报「重试后失败」。 */
  retried?: boolean;
}

export type WorkOrderReceiptStatus = WorkOrderReceiptOutcome["status"];

/**
 * 回执轮头卡标题（后台结果轮头链按 originMeta.title 原样透传，标题由 CLI 权威给出）。
 * 按终态区分措辞：失败/中断的回执绝不标题写「交活」——不假报完成。
 */
export function buildWorkOrderReceiptTitle(agentName: string, status: WorkOrderReceiptStatus): string {
  const name = agentName.trim() || "智能体";
  if (status === "completed") return `${name} 交活`;
  if (status === "cancelled") return `${name} 的工单被中断`;
  return `${name} 的工单未完成`;
}

/**
 * 回执正文（发起方会话的模型输入 carrier）。自带结构化信封标签（谁交的活、
 * 指回哪张工单），与 <work-order> 同一家族；答案正文里出现的同形关闭标签
 * 一律中和，防对方输出伪造信封边界（Claude Code 来源信封铁律）。
 */
export function buildWorkOrderReceiptEnvelopeText(input: {
  workOrderId: string;
  agentName: string;
  targetSessionId: string;
  outcome: WorkOrderReceiptOutcome;
}): string {
  const status = input.outcome.status;
  const body =
    status === "completed"
      ? neutralizeEnvelopeTags(neutralizeBatchQcTags(input.outcome.response ?? ""))
      : status === "cancelled"
        ? "The target agent's turn was interrupted before it could produce a final answer. No answer was delivered; you may re-dispatch the task or report the interruption to the user."
        : `The target agent's turn failed before producing a final answer${
            input.outcome.reason
              ? `: ${neutralizeEnvelopeTags(neutralizeBatchQcTags(input.outcome.reason))}`
              : "."
          }${
            // 已自动重试过一次仍失败（2026-10-01 员工可靠性批）：明说，别让派单方
            // 再用同样的方式盲试——要么换路子，要么如实报告用户。
            input.outcome.retried
              ? " An automatic retry was already attempted and failed with the same error; do not blindly re-dispatch the same way."
              : " You may re-dispatch the task or report the failure to the user."
          }`;
  const agentLabel = escapeEnvelopeAttribute(input.agentName.trim() || "agent");
  return [
    `<work-order-receipt id="${input.workOrderId}" from-agent="${agentLabel}" from-session="${input.targetSessionId}" status="${status}">`,
    body,
    "</work-order-receipt>",
  ].join("\n");
}

// ── 批次质检（纪律协议批）纯规则 ────────────────────────────────────

/** 质检清单里的一张工单：来自发起方派单台账的同批行（触发侧已过滤）。 */
export interface BatchQcOrder {
  workOrderId: string;
  /**
   * 员工名（台账行 payload 顶层的 agentName）。信封里的 fromAgentName 是**发起方**
   * 署名（驻场时是老板档案名、用户直派时是空串）——读它必然派错人（评审 B1）。
   */
  agentName: string;
  /** 员工工号（打回重派按号点名，改名不误派；台账行在场时随行）。 */
  agentId?: string;
  task: string;
  /** 评审单标记（评审会批）：混批兜底成质检时，质检要求据此豁免评审单不打回。 */
  review?: boolean;
}

/**
 * 批次收口自动轮头卡标题（后台结果轮头链按 originMeta.title 原样透传，标题由 CLI
 * 权威给出）。质检是发起方自己的动作，标题不署任何员工名；评审会批（review）
 * 同一轮链换合议词表——「合议 · 批名」。
 */
export function buildBatchQcTitle(batchTitle?: string, options?: { review?: boolean }): string {
  const title = batchTitle?.trim();
  if (options?.review === true) return title ? `合议 · ${title}` : "评审合议";
  return title ? `质检 · ${title}` : "批次质检";
}

/** 质检信封里的单条任务上界：与 agentWorkOrderMeta 的落库上界同口径（超界截断诚实）。 */
const BATCH_QC_TASK_MAX_CHARS = 8_192;

/**
 * 批次收口自动轮正文（发起方会话的模型输入 carrier）。自带 <batch-qc> 信封（批次
 * 身份 + 逐单清单）；要求块按口味二选一：普通批 = 质检要求（可打回重派），评审会批
 * （review）= 合议要求（只出大白话结论，禁派单，决定权交还用户）。回执答案不进信封
 * ——它们本就在发起方历史里，按 receipt 信封的 workOrderId 对号。
 * ponytail: 清单来自老板自己派的单（同批派单台账行），不设单数上限；单条任务
 * 有界。真出现超大批次（几十单 × 8192 字）再谈分页。
 */
export function buildBatchQcEnvelopeText(input: {
  batchId: string;
  batchTitle?: string;
  orders: readonly BatchQcOrder[];
  /** 评审会批：本批工单全是评审单，收口轮是合议而不是质检。 */
  review?: boolean;
}): string {
  const title = neutralizeEnvelopeTags(neutralizeBatchQcTags(input.batchTitle?.trim() ?? ""));
  const orderLines = input.orders.map((order) => {
    const task = neutralizeEnvelopeTags(neutralizeBatchQcTags(order.task)).trim();
    const clipped =
      task.length > BATCH_QC_TASK_MAX_CHARS
        ? `${task.slice(0, BATCH_QC_TASK_MAX_CHARS - 1)}…`
        : task;
    const agentLabel = escapeEnvelopeAttribute(order.agentName.trim() || "agent");
    return `<order id="${order.workOrderId}" agent="${agentLabel}"${
      order.agentId ? ` agent-id="${escapeEnvelopeAttribute(order.agentId)}"` : ""
    }${order.review === true ? ` review="true"` : ""}>${clipped}</order>`;
  });
  const requirements = input.review
    ? [
        "合议要求（自动合议，不是用户发言）：",
        "1. 本批是评审会：上列每张工单都是评审单，各评审人的意见就在本会话历史里（<work-order-receipt> 信封，信封 id 与上列 order 的 id 一一对应）。先逐单找到意见。",
        "2. 你只合议、不动手：不要改文件、不要重派工单（AgentDispatch 对你禁用）、不要替用户做决定——结论交给用户裁决。",
        "3. 结论分四段：①总体结论（全体通过 / 有分歧 / 全体不通过；「有条件通过」计入通过一类，但结论里必须写清条件）②每位评审一行：名字 + 结论 + 一句话立场 ③分歧点：谁与谁不一致、差在哪（没有就写「无」）④你的建议：一句话，供用户拍板。",
        "4. 有评审缺席（找不到回执 / 失败 / 被中断）就如实标注缺席，其余人照常合议；评审意见与工作区实际产物对不上号的，如实指出。",
        "5. 全程用本会话用户的语言，大白话，不贴大段原文。",
      ]
    : [
        "质检要求（自动验货，不是用户发言）：",
        "1. 上列每张工单的回执都在本会话历史里（<work-order-receipt> 信封，信封 id 与上列 order 的 id 一一对应）。先逐单找到回执。",
        "2. 逐单验货：对照工单任务核对回执答案；凡能落到工作区的（代码/文件改动），以实际文件为准复核，不轻信员工的自我报告。",
        "3. 回执缺失、答案与任务对不上号、或答案自相矛盾的，如实标注「无法核实」，不要猜、不要补。",
        "4. 被中断（cancelled）的单是用户自己叫停的，不参与打回，结论里如实说明即可；评审单（review=\"true\"，输出的是评审意见）同样不参与打回，只核对意见是否与任务对应。",
        "5. 不合格的单（打回）：用 AgentDispatch 工具重派给同一位员工——agent 参数优先用清单里的 agent-id（工号，员工改名也能找到人），没有工号才用 agent 名；batch_id 与 batch_title 必须原样沿用本批次，任务正文 = 原任务全文 + 换行 + 「【质检打回】」+ 具体不合格原因与修改要求。整个批次最多打回这一轮，重派的单不再自动质检。",
        "6. 没有问题就什么都不重派；拿不准的不要打回，写进结论里留给用户判断。",
        "7. 最后用本会话用户的语言给一段大白话验收结论：每单一行（通过 / 已打回重派 / 无法核实 + 一句原因），最后一句总评。不要贴大段代码或长篇复述。",
      ];
  return [
    // batchId 是模型回传的任意字符串（AgentDispatch 工具 batch_id，端口只 trim）——
    // 与 title 同样过属性转义，员工答案里种 batch_id 也伪造不了信封结构（评审 B2）。
    `<batch-qc id="${escapeEnvelopeAttribute(input.batchId)}"${title ? ` title="${escapeEnvelopeAttribute(title)}"` : ""}>`,
    ...orderLines,
    "</batch-qc>",
    "",
    ...requirements,
  ].join("\n");
}
