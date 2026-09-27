import type { AgentSummary, SubAgentConfig } from "@zcode/shared";

export interface ProjectAgentDraft {
  name: string;
  description: string;
  systemPrompt: string;
}

export type ProjectAgentDraftError =
  | "nameLength"
  | "nameCharacters"
  | "descriptionRequired"
  | "promptRequired";

// 校验规则与 settings/SubagentsSection 的 agent 表单一致，报错文案复用其 locale 键。
export function validateProjectAgentDraft(draft: ProjectAgentDraft): ProjectAgentDraftError[] {
  const errors: ProjectAgentDraftError[] = [];
  const name = draft.name.trim();
  if (name.length < 3 || name.length > 50) {
    errors.push("nameLength");
  } else if (!/^[a-zA-Z0-9-]+$/u.test(name)) {
    errors.push("nameCharacters");
  }
  if (!draft.description.trim()) {
    errors.push("descriptionRequired");
  }
  if (!draft.systemPrompt.trim()) {
    errors.push("promptRequired");
  }
  return errors;
}

// 侧栏只展示驻场（workspace 作用域）智能体；顺序沿用 service 已按名称排好的列表。
export function selectProjectAgents<T extends Pick<AgentSummary, "scope">>(
  agents: readonly T[],
): T[] {
  return agents.filter((agent) => agent.scope === "workspace");
}

export function toProjectAgentCreateConfig(draft: ProjectAgentDraft): SubAgentConfig {
  // memory 固定 project：驻场智能体只服务本项目（AgentMemoryScope 由 services 侧定义）。
  return {
    name: draft.name.trim(),
    description: draft.description.trim(),
    systemPrompt: draft.systemPrompt.trim(),
    memory: "project",
  };
}
