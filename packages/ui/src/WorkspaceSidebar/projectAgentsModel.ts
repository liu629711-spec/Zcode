import type { AgentSummary, SubAgentConfig, ZCodeSessionPersona } from "@zcode/shared";

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
// agent.path = `<workspacePath>/.zcode/agents/<name>.md` 全路径，一次服务解析可列任意本地工作区，
// 所以按路径前缀把 agent 归到具体工作区（斜杠/盘符大小写容错的简单前缀匹配即可，不引 path 库）。
export function selectProjectAgentsForWorkspace<T extends Pick<AgentSummary, "scope" | "path">>(
  agents: readonly T[],
  workspacePath: string,
): T[] {
  const normalize = (value: string): string =>
    value
      .replace(/\\/g, "/")
      .replace(/^[A-Za-z]:/, (drive) => drive.toLowerCase())
      .replace(/\/+$/, "");
  const prefix = `${normalize(workspacePath)}/.zcode/agents/`;
  return agents.filter(
    (agent) => agent.scope === "workspace" && normalize(agent.path).startsWith(prefix),
  );
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

/**
 * createSession.persona 载荷：随会话创建一次性进入 runtime config。
 * 即 shared 的 ZCodeSessionPersona（单一来源，防两处形状漂移）：
 * G2 起除 name/systemPrompt/memoryScope 外还携带档案的模型/工具/颜色，
 * 让驻场会话与档案同一副面孔；全部可选，缺席 = 跟随会话缺省。
 */
export type ProjectAgentPersona = ZCodeSessionPersona;

export function toProjectAgentPersona(agent: AgentSummary): ProjectAgentPersona {
  return {
    name: agent.name,
    systemPrompt: agent.systemPrompt,
    ...(agent.memory ? { memoryScope: agent.memory } : {}),
    ...(agent.modelSelection ? { modelSelection: agent.modelSelection } : {}),
    // 空数组与缺席同义 = 继承全部工具（与子代理派遣的 allowedTools 语义一致）。
    ...(agent.tools?.length ? { tools: [...agent.tools] } : {}),
    ...(agent.disallowedTools?.length ? { disallowedTools: [...agent.disallowedTools] } : {}),
    ...(agent.color ? { color: agent.color } : {}),
  };
}
