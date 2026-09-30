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
   * 终值来源：requested=本单指定（D32）且有效；ambient=环境默认（新会话=档案默认、
   * 复用会话=会话常驻）且有效——复用会话据此**不传** per-order 覆盖，让会话自己跑；
   * fallback=回落主会话当前模型（老板正用它说话，必然可用）。
   */
  source: "requested" | "ambient" | "fallback";
  /** source=fallback 时的原因：unset=谁都没配模型；unavailable=配的模型不在注册表里（下线/已删/拼错）。 */
  reason?: "unset" | "unavailable";
  /** true = 候选模型没过校验又没有回落目标——只能带着坏候选上场（端口据此告警）。 */
  unavailable?: boolean;
}

/**
 * 派单模型裁决：候选 = 本单指定（requested，缺席才轮到环境默认 ambient），
 * 过了可用性判据就放行；候选不在注册表里（已下线/已删/拼错）**直接**回落主会话
 * 当前模型（fallback）——不再试环境默认（老板拍板口径：「配了已下线模型→回落主
 * 会话当前模型」，环境默认是「没指定时」的候选来源，不是失效后的下一级）。
 * 判据缺席时只做「没配 → 回落」，不越权拦人。
 */
export function resolveDispatchModelSelection(input: {
  /** 本单指定的模型（工单参数 model_selection）；派单方明确点名的那一个。 */
  requested?: ModelSelection;
  /** 环境默认：新会话=档案默认；复用会话=会话常驻。requested 缺席时才生效。 */
  ambient?: ModelSelection;
  /** 回落目标：主会话当前模型。 */
  fallback?: ModelSelection;
  /** 模型可用性判据；缺席 = 没法校验。 */
  isModelAvailable?: (selection: ModelSelection) => boolean;
}): DispatchModelResolution {
  const { requested, ambient, fallback, isModelAvailable } = input;
  const candidate = requested ?? ambient;
  const source = requested ? ("requested" as const) : ("ambient" as const);
  if (!candidate) {
    return fallback
      ? { modelSelection: fallback, source: "fallback", reason: "unset" }
      : { modelSelection: undefined, source: "fallback", reason: "unset" };
  }
  if (!isModelAvailable || isModelAvailable(candidate)) {
    return { modelSelection: candidate, source };
  }
  return fallback
    ? { modelSelection: fallback, source: "fallback", reason: "unavailable" }
    : { modelSelection: candidate, source, unavailable: true };
}

/**
 * 工单正文（目标会话的模型输入 carrier）。自带结构化信封标签（谁派的单），
 * 与 task-notification 同一家族：不走 <system-reminder> 包装，正文自证来源。
 * 任务正文里出现的同形关闭标签一律中和，防伪造信封边界。
 */
export function buildWorkOrderEnvelopeText(envelope: AgentWorkOrderEnvelope): string {
  const task = envelope.task.replace(/<\/?work-order\b/gi, (tag) => tag.replace("<", "&lt;"));
  const fromLabel = envelope.fromAgentName.trim() || "user";
  return [
    `<work-order id="${envelope.workOrderId}" from-agent="${fromLabel}" from-session="${envelope.fromSessionId}">`,
    task,
    "</work-order>",
    // 语言随工单（真机 2026-09-30）：弱模型拿着英文系统提示，中文工单也回英文，
    // 还自由发挥成自我介绍。信封后缀一行硬要求：语言跟工单走，没交代的事不做。
    // 放在标签外——事件面把标签内原文当「任务正文」透出，里面不能掺指令。
    "回复要求：使用与上面工单正文相同的语言；工单未交代的事项不要自行发挥。",
  ].join("\n");
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
      ? (input.outcome.response ?? "").replace(/<\/?work-order-receipt\b/gi, (tag) => tag.replace("<", "&lt;"))
      : status === "cancelled"
        ? "The target agent's turn was interrupted before it could produce a final answer. No answer was delivered; you may re-dispatch the task or report the interruption to the user."
        : `The target agent's turn failed before producing a final answer${
            input.outcome.reason ? `: ${input.outcome.reason}` : "."
          }${
            // 已自动重试过一次仍失败（2026-10-01 员工可靠性批）：明说，别让派单方
            // 再用同样的方式盲试——要么换路子，要么如实报告用户。
            input.outcome.retried
              ? " An automatic retry was already attempted and failed with the same error; do not blindly re-dispatch the same way."
              : " You may re-dispatch the task or report the failure to the user."
          }`;
  const agentLabel = input.agentName.trim() || "agent";
  return [
    `<work-order-receipt id="${input.workOrderId}" from-agent="${agentLabel}" from-session="${input.targetSessionId}" status="${status}">`,
    body,
    "</work-order-receipt>",
  ].join("\n");
}
