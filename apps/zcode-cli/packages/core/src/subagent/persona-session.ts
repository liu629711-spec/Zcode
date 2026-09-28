// 驻场智能体主会话（persona session）的纯拼装规则。
// 注入通道是 runtime 既有 config.systemPrompt（builder customSystemPrompt）；
// 这里只收口「多段拼接」「标题记账」「persona 快照」「persona → runtime config 映射」
// 四个纯规则，I/O 见 persistent-memory.ts。

import { normalizeAgentId } from "@zcode/shared";
import type {
  AgentColor,
  AgentMemoryScope,
  ModelSelection,
  ZCodeSessionPersona,
} from "@zcode/shared";

/**
 * persona system prompt 拼装：跳过空段，`\n\n` 连接——与子代理派生
 * （runtime/methods/subagent.ts 的 agentPrompt 组合）同一约定，block 左边界交给 builder。
 */
export function joinPersonaSystemPrompt(
  parts: readonly (string | undefined)[],
): string | undefined {
  const joined = parts
    .filter((part): part is string => typeof part === "string" && part.length > 0)
    .join("\n\n");
  return joined.length > 0 ? joined : undefined;
}

/** 会话标题记账：驻场智能体会话以「智能体名 · 首条输入」落库，侧栏据此认出归属。 */
export function buildProjectAgentSessionTitle(personaName: string, baseTitle: string): string {
  return `${personaName} · ${baseTitle}`;
}

/**
 * persona 落盘快照（G1 冷重启保身份）：runtime config → 会话记录的结构化标记。
 * name/systemPrompt 任一缺席即返回 undefined——宁可整份不落盘（恢复后按普通会话
 * 处理），也不落半截快照让回灌端拼出残缺身份。可选字段缺席时快照不带该键；
 * 模型不属于快照——resume 只回身份与工具面（G2），不回灌模型（由 session selection
 * entry 恢复）。工具面写的是档案原始值：Write/Edit 追加在 runtime 装配处现算，
 * 记忆总开关关开后恢复不会残留记忆写入工具。
 */
export function buildProjectAgentPersonaSnapshot(
  persona:
    | {
        name: string;
        agentId?: string;
        memory?: AgentMemoryScope;
        color?: AgentColor;
        tools?: readonly string[];
        disallowedTools?: readonly string[];
      }
    | undefined,
  systemPrompt: string | undefined,
): ZCodeSessionPersona | undefined {
  if (!persona?.name || !systemPrompt) return undefined;
  // 号先过形状判据再落盘：片三起号决定记事本目录，半截/乱码号落进快照等于把记忆
  // 寄到一个没人认识的路径上，宁可当无号（回落到名字对号）。
  const agentId = normalizeAgentId(persona.agentId);
  return {
    name: persona.name,
    ...(agentId ? { agentId } : {}),
    systemPrompt,
    ...(persona.memory ? { memoryScope: persona.memory } : {}),
    ...(persona.color ? { color: persona.color } : {}),
    ...(persona.tools?.length ? { tools: [...persona.tools] } : {}),
    ...(persona.disallowedTools?.length
      ? { disallowedTools: [...persona.disallowedTools] }
      : {}),
  };
}

/** persona 载荷映射出的 runtime config 切片（bootstrap createRecord 装配用）。 */
export interface PersonaRuntimeConfigFragment {
  /** 档案模型：仅 create 路径落下；resume/fork 回灌不映射（模型只由 entry 恢复）。 */
  modelSelection?: ModelSelection;
  toolAllowlist?: readonly string[];
  toolDisallowlist?: readonly string[];
  systemPrompt: string;
  projectAgentPersona: {
    name: string;
    /** 员工工号（D26）：随快照落盘，resume/fork 回灌同源。 */
    agentId?: string;
    memory?: AgentMemoryScope;
    color?: AgentColor;
    tools?: readonly string[];
    disallowedTools?: readonly string[];
  };
}

/**
 * persona 载荷 → runtime config 切片（G2 对话入口全档案生效，create 与 resume/fork
 * 回灌共用；bootstrap createRecord 保证显式 params（model/toolAllowlist/toolDenylist）
 * 始终优先，本函数只提供缺省值）。
 * - modelSelection：仅 create 路径落下（includeModelSelection），resume/fork 传 false——
 *   『模型只由 entry 恢复』裁决不破。
 * - tools/disallowedTools：档案工具面落到既有 toolAllowlist/toolDisallowlist 会话安全
 *   边界通道；空数组与缺席同义 = 继承全部工具（与子代理派遣的 allowedTools 语义一致）。
 *   原始档案值同时随 projectAgentPersona 走到 persona 快照落盘（resume/fork 回灌数据源）；
 *   记忆写入端（Write/Edit）追加在 core runtime 装配处统一现算
 *   （persistent-memory.projectPersistentAgentMemoryTools），不在此混入。
 * - color：无运行时语义，随 projectAgentPersona 走到 persona 快照落盘（侧栏徽章取色）。
 */
export function mapPersonaToRuntimeConfig(
  persona: ZCodeSessionPersona,
  options: { includeModelSelection: boolean },
): PersonaRuntimeConfigFragment {
  const agentId = normalizeAgentId(persona.agentId);
  return {
    ...(options.includeModelSelection && persona.modelSelection
      ? { modelSelection: persona.modelSelection }
      : {}),
    ...(persona.tools?.length ? { toolAllowlist: [...persona.tools] } : {}),
    ...(persona.disallowedTools?.length
      ? { toolDisallowlist: [...persona.disallowedTools] }
      : {}),
    systemPrompt: persona.systemPrompt,
    projectAgentPersona: {
      name: persona.name,
      ...(agentId ? { agentId } : {}),
      ...(persona.memoryScope ? { memory: persona.memoryScope } : {}),
      ...(persona.color ? { color: persona.color } : {}),
      ...(persona.tools?.length ? { tools: [...persona.tools] } : {}),
      ...(persona.disallowedTools?.length
        ? { disallowedTools: [...persona.disallowedTools] }
        : {}),
    },
  };
}
