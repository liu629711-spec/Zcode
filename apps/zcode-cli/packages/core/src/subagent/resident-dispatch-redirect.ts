// ============================================================
// 驻场智能体派遣改判（2026-09-29 真机教训）：模型选临时工，产品语义要转交。
// ============================================================
// 真机证据：工具清单里有 AgentDispatch、新说明书也在请求里（dump 逐字比对过），
// GLM 系模型仍直接调 Agent(type = 驻场档案名)——软引导不可靠。点将的先例是
// 在端口层执法（createAgentCallPinnedPort），这里同样：派遣端口是唯一入口，
// 模型选什么都没用，命中判据就把这次调用改判为工单投递。
//
// 判据（互补于点将，不重叠）：
// - 本轮 @ 点将名单非空 → 放行（用户明确要在本会话搭把手，点将语义优先）；
// - 目标 subagent_type 命中本工作区项目作用域档案（.zcode/agents/*.md）
//   且本轮没有 @ → 改判为 AgentDispatch。
// 其余一切（Explore/general-purpose/插件档案…）逐字原样转发，行为不变。
//
// 兜底：dispatch 被拒（工单轮嵌套上限的端口自检抛错等）→ 回落到原派遣，
// 行为与从前一致——改判只增不减能力。

import type { AgentDispatchPort, AgentOutput, SubagentPort } from "@zcode/contracts";
import type { AgentProfile } from "./profile.js";

export type ResidentRedirectDecision =
  | { kind: "pass" }
  | { kind: "redirect"; profile: AgentProfile };

/** 纯判据（可测）：是否把这次派遣改判为工单投递。 */
export function resolveResidentDispatchRedirect(
  agentType: string,
  pinnedNames: readonly string[],
  profiles: readonly AgentProfile[],
): ResidentRedirectDecision {
  if (pinnedNames.length > 0) return { kind: "pass" };
  const candidate = agentType.trim().toLowerCase();
  if (candidate.length === 0) return { kind: "pass" };
  const match = profiles.find(
    (profile) => profile.source === "project" && profile.name.trim().toLowerCase() === candidate,
  );
  return match ? { kind: "redirect", profile: match } : { kind: "pass" };
}

export interface ResidentDispatchRedirectDeps {
  getPinnedNames: () => readonly string[];
  getResidentProfiles: () => readonly AgentProfile[];
  getDispatchPort: () => AgentDispatchPort | undefined;
  getSourceSessionId: () => string;
}

export function createResidentDispatchRedirectPort(
  port: SubagentPort | undefined,
  deps: ResidentDispatchRedirectDeps,
): SubagentPort | undefined {
  if (!port) return undefined;

  const redirect = async (
    request: { agentType: string; prompt: string; description: string },
    fallback: () => Promise<AgentOutput>,
  ): Promise<AgentOutput> => {
    const decision = resolveResidentDispatchRedirect(
      request.agentType,
      deps.getPinnedNames(),
      deps.getResidentProfiles(),
    );
    if (decision.kind === "pass") return fallback();
    const dispatchPort = deps.getDispatchPort();
    if (!dispatchPort) return fallback();
    const startedAt = Date.now();
    try {
      const result = await dispatchPort.dispatch({
        agent: decision.profile.agentId ?? decision.profile.name,
        task: request.prompt,
        sourceSessionId: deps.getSourceSessionId(),
      });
      // 给模型的诚实口径：受理 ≠ 完成，回执稍后到——模型据此如实转述给用户。
      const text = [
        `The task was handed to the resident agent ${result.agentName} instead of a temporary helper.`,
        result.delivery === "queued"
          ? "That agent is busy right now; the work order is queued and will run at its next turn boundary."
          : "It is now running in that agent's own session, with its own memory and permissions.",
        "The final answer will arrive later as a receipt in THIS conversation. Do not wait for it and do not claim the work is done.",
      ].join(" ");
      return {
        status: "completed",
        agentId: result.agentId ?? `work-order:${result.targetSessionId}`,
        agentType: request.agentType,
        description: request.description,
        prompt: request.prompt,
        content: [{ type: "text" as const, text }],
        totalToolUseCount: 0,
        totalDurationMs: Date.now() - startedAt,
        workOrderRedirect: true,
      };
    } catch {
      // 端口拒绝（工单轮嵌套上限等）→ 回落原派遣，不改行为。
      return fallback();
    }
  };

  // 只包 launch/run：Agent 工具走 launch（内部对后台请求转调内层 start，不重入本包装），
  // 所以这两条已覆盖全部模型可达路径；直呼 start 的调用方在本仓不存在。
  return {
    ...port,
    async launch(request, options) {
      return redirect(request, () => port.launch(request, options));
    },
    async run(request, options) {
      return redirect(request, () => port.run(request, options));
    },
  };
}
