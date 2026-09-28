import type { AgentProfile } from "./profile.js";

// 用户在任意会话里 @ 点名员工（D24「@点将」）的纯规则：解析正文里的 `agent://<名字>`
// 引用、与现役档案取交集、决定派遣落到谁头上。
// 契约照 plugin 引用的既有纪律（core/src/plugin-reference/references.ts）：
// - 只接受 Markdown 链接 destination 形如 `agent://<name>`，协议名大小写敏感（仅小写）；
// - 身份只来自 destination，label 永不参与解析（防"label 骗过统计"）；
// - 名字段限档案名字符集 [A-Za-z0-9-]{3,50}，其余一律拒绝，不做宽容匹配。

const AGENT_REFERENCE_SCHEME = "agent://";
const MARKDOWN_LINK_PATTERN =
  /\[(?:\\.|[^\\\]])*\]\((?:<((?:\\.|[^>])*?)>|((?:\\.|[^)\s])*))\)/g;
const AGENT_NAME_PATTERN = /^[A-Za-z0-9-]{3,50}$/;

/** 单轮点名上限：再多就是误贴或对抗样本，fail closed 截断并如实记账。 */
export const MAX_AGENT_CALLS_PER_TURN = 4;

export interface ExtractAgentReferencesResult {
  /** 按正文首次出现顺序、按名字去重后的点名名单。 */
  names: string[];
  /** 超过单轮上限被丢弃的次数。 */
  truncatedCount: number;
  /** 意图是 agent:// 但形状非法的次数。 */
  invalidCount: number;
}

export function isValidAgentReferenceName(candidate: string): boolean {
  return AGENT_NAME_PATTERN.test(candidate);
}

export function extractAgentReferences(text: string): ExtractAgentReferencesResult {
  const names: string[] = [];
  let truncatedCount = 0;
  let invalidCount = 0;
  for (const match of text.matchAll(MARKDOWN_LINK_PATTERN)) {
    const destination = (match[1] ?? match[2] ?? "").trim();
    if (!destination.toLowerCase().startsWith(AGENT_REFERENCE_SCHEME)) {
      continue;
    }
    // 协议名大小写敏感：`Agent://` 是"意图命中但形状非法"，计入 invalidCount 而不是收进名单。
    if (!destination.startsWith(AGENT_REFERENCE_SCHEME)) {
      invalidCount += 1;
      continue;
    }
    const name = destination.slice(AGENT_REFERENCE_SCHEME.length);
    if (!isValidAgentReferenceName(name)) {
      invalidCount += 1;
      continue;
    }
    if (names.includes(name)) {
      continue;
    }
    if (names.length >= MAX_AGENT_CALLS_PER_TURN) {
      truncatedCount += 1;
      continue;
    }
    names.push(name);
  }
  return { names, truncatedCount, invalidCount };
}

/**
 * 点名名单 × 现役档案：大小写不敏感对号（档案文件名落盘即小写化，
 * 派遣侧的归一化匹配也是大小写不敏感），命不中的如实带出，由提示语告诉用户。
 */
export function resolveAgentCallTargets(
  names: readonly string[],
  profiles: readonly AgentProfile[],
): { resolved: AgentProfile[]; unresolved: string[] } {
  const resolved: AgentProfile[] = [];
  const unresolved: string[] = [];
  for (const name of names) {
    const lowered = name.toLowerCase();
    const profile = profiles.find((candidate) => candidate.name.trim().toLowerCase() === lowered);
    if (profile) {
      resolved.push(profile);
    } else {
      unresolved.push(name);
    }
  }
  return { resolved, unresolved };
}

/**
 * 派遣归属裁决（点将的真正落点）：点名名单非空时，派遣只能在名单内。
 * - 模型给的 subagent_type 在名单里 → 尊重它的选择（名单内多人时有意义）；
 * - 不在名单里 → 用名单第一位。用户点名了就派给他，这不是建议、是命令。
 *   （模型省略 subagent_type 时，Agent 工具已按 general-purpose 填好，同样走这条：
 *   general-purpose 不在名单里 → 改派点名的人。）
 * - 名单为空 → 原样返回，普通会话行为字节级不变。
 */
export function resolvePinnedAgentType(
  requested: string,
  pinnedNames: readonly string[],
): string {
  if (pinnedNames.length === 0) {
    return requested;
  }
  const loweredRequested = requested.trim().toLowerCase();
  const matched = pinnedNames.find((name) => name.trim().toLowerCase() === loweredRequested);
  return matched ?? pinnedNames[0];
}

/**
 * 给模型的点将告示（进 attachment + 原文落库，与 plugin 引用同一持久化纪律：
 * 只写内存 attachment 会让 cold resume 丢掉它，历史序列错位还会砸穿前缀缓存）。
 */
export function buildAgentCallReminderBody(input: {
  resolved: readonly { name: string; description: string }[];
  unresolved: readonly string[];
}): string | undefined {
  if (input.resolved.length === 0 && input.unresolved.length === 0) {
    return undefined;
  }
  const lines = ["[点将] 用户在这条消息里点名了项目员工，派活只能派给名单里的人。"];
  if (input.resolved.length > 0) {
    lines.push(
      "点名名单（Agent 工具的 subagent_type 请从中选一个；省略则系统按第一位派）：",
      ...input.resolved.map((agent) => `- ${agent.name}：${agent.description}`),
    );
  }
  if (input.unresolved.length > 0) {
    lines.push(
      `没找到对应档案，已忽略：${input.unresolved.join("、")}（要用先在项目里建这个员工）`,
    );
  }
  return lines.join("\n");
}
