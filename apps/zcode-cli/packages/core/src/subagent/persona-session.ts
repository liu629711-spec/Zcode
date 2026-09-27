// 驻场智能体主会话（persona session）的纯拼装规则。
// 注入通道是 runtime 既有 config.systemPrompt（builder customSystemPrompt）；
// 这里只收口「多段拼接」与「标题记账」两个纯规则，I/O 见 persistent-memory.ts。

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
