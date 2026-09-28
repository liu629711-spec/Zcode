import type { SubagentPort } from "@zcode/contracts";
import { resolvePinnedAgentType } from "./agent-call.js";

/**
 * 点将的落点（D24）：把本轮被点名的员工套在子代理端口外面，派遣归属在这里定。
 *
 * 为什么不改 Agent 工具入参或工具上下文：那条链要穿过 executor/batch/call-runner
 * 六七个装配点，把"本轮点将"当成工具参数传播——状态所有者错位（点将是会话轮的事实，
 * 不是工具的调用参数）。端口是派遣的唯一入口，包一层就覆盖 launch/run/start 三条路，
 * 也覆盖 SendMessage 之外的所有派遣方式。
 *
 * 未点将（名单为空）时逐字原样转发，普通会话行为不变。
 */
export function createAgentCallPinnedPort(
  port: SubagentPort | undefined,
  getPinnedNames: () => readonly string[],
): SubagentPort | undefined {
  if (!port) return undefined;
  const pin = (agentType: string): string => resolvePinnedAgentType(agentType, getPinnedNames());
  /** 名单为空或未改判 → 原对象直接转发，保持引用与行为字节级不变。 */
  const retype = <T extends { agentType: string }>(request: T): T => {
    const agentType = pin(request.agentType);
    return agentType === request.agentType ? request : { ...request, agentType };
  };

  return {
    ...port,
    async launch(request, options) {
      return port.launch(retype(request), options);
    },
    async run(request, options) {
      return port.run(retype(request), options);
    },
    ...(port.start
      ? {
          start: async (
            request: Parameters<NonNullable<SubagentPort["start"]>>[0],
            options?: Parameters<NonNullable<SubagentPort["start"]>>[1],
          ) => port.start!(retype(request), options),
        }
      : {}),
  };
}
