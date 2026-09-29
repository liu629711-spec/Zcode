// ============================================================
// 派单（D29）纯规则：目标解析、工单信封拼装、工单轮身份判定。
// ============================================================

import type { AgentProfile } from "./profile.js";
import { WORK_ORDER_INPUT_ID_PREFIX, type AgentWorkOrderEnvelope } from "@zcode/contracts";

// 前缀单一来源在 contracts（turn-loop state 与 bootstrap 端口自检共用同一词表）。
export { WORK_ORDER_INPUT_ID_PREFIX };

// 目标解析判据（与点将的宽容匹配刻意不同——工单永不落错人）：
// - uuid 形状 → 按工号（agentId）在全量现役档案里唯一匹配；
// - 其余 → 精确名匹配，且仅限项目作用域档案（source === "project"）；
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
          profile.source === "project" && profile.name.trim().toLowerCase() === candidate.toLowerCase(),
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
          } You may re-dispatch the task or report the failure to the user.`;
  const agentLabel = input.agentName.trim() || "agent";
  return [
    `<work-order-receipt id="${input.workOrderId}" from-agent="${agentLabel}" from-session="${input.targetSessionId}" status="${status}">`,
    body,
    "</work-order-receipt>",
  ].join("\n");
}
