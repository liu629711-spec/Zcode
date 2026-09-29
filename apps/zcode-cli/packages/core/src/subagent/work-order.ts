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
