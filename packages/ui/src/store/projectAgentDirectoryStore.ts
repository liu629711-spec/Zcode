import { create } from "zustand";
import type { AgentSummary } from "@zcode/shared";
import type { PersonaBadgeAgentEntry } from "@/WorkspaceSidebar/projectAgentsModel.js";

/**
 * 驻场智能体档案目录（D2 跨视图对号）：侧栏已取到的档案摘要（名字+颜色+工号）下发给
 * 其它消费方——Header「···」菜单判员工行、时间线/置顶视图反推徽章与上色。
 * 只带对号要用的最小面（PersonaBadgeAgentEntry），避免把 AgentSummary 整壳泄露给渲染层；
 * 条目多带一个 scope：收编菜单（身份轴终局 §九④）要按档案作用域分流——只有项目档案
 * 给「升级为全局员工」，全局员工的行不给（已是全局，点了也是拒绝）。
 */
export type ProjectAgentDirectoryEntry = PersonaBadgeAgentEntry & {
  scope: AgentSummary["scope"];
};

const EMPTY_DIRECTORY: readonly ProjectAgentDirectoryEntry[] = [];

interface ProjectAgentDirectoryState {
  agentsByWorkspaceKey: Record<string, readonly ProjectAgentDirectoryEntry[]>;
  publishAgentsByWorkspaceKey: (
    agentsByWorkspaceKey: ReadonlyMap<string, readonly ProjectAgentDirectoryEntry[]>,
  ) => void;
}

export const useProjectAgentDirectoryStore = create<ProjectAgentDirectoryState>()((set) => ({
  agentsByWorkspaceKey: {},
  publishAgentsByWorkspaceKey: (agentsByWorkspaceKey) => {
    const next: Record<string, readonly ProjectAgentDirectoryEntry[]> = {};
    for (const [workspaceKey, agents] of agentsByWorkspaceKey) {
      next[workspaceKey] = agents.map((agent) => ({
        name: agent.name,
        description: agent.description,
        scope: agent.scope,
        ...(agent.agentId ? { agentId: agent.agentId } : {}),
        ...(agent.color ? { color: agent.color } : {}),
      }));
    }
    set({ agentsByWorkspaceKey: next });
  },
}));

/** 按工作区取档案目录（引用稳定：缺席复用同一空数组常量）。 */
export function selectProjectAgentDirectoryForWorkspace(
  state: ProjectAgentDirectoryState,
  workspaceKey: string,
): readonly ProjectAgentDirectoryEntry[] {
  return state.agentsByWorkspaceKey[workspaceKey] ?? EMPTY_DIRECTORY;
}