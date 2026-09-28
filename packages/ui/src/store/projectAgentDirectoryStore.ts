import { create } from "zustand";
import type { AgentSummary } from "@zcode/shared";

/**
 * 驻场智能体档案目录（D2 跨视图对号）：侧栏已取到的档案摘要（名字+颜色）下发给
 * 其它消费方——Header「···」菜单判员工行、时间线/置顶视图反推徽章与上色。
 * 只带名字与颜色两个字段，避免把 AgentSummary 整壳泄露给渲染层。
 */
export type ProjectAgentDirectoryEntry = Pick<AgentSummary, "name" | "color">;

const EMPTY_DIRECTORY: readonly ProjectAgentDirectoryEntry[] = [];

interface ProjectAgentDirectoryState {
  agentsByWorkspaceKey: Record<string, readonly ProjectAgentDirectoryEntry[]>;
  publishAgentsByWorkspaceKey: (
    agentsByWorkspaceKey: ReadonlyMap<string, readonly AgentSummary[]>,
  ) => void;
}

export const useProjectAgentDirectoryStore = create<ProjectAgentDirectoryState>()((set) => ({
  agentsByWorkspaceKey: {},
  publishAgentsByWorkspaceKey: (agentsByWorkspaceKey) => {
    const next: Record<string, readonly ProjectAgentDirectoryEntry[]> = {};
    for (const [workspaceKey, agents] of agentsByWorkspaceKey) {
      next[workspaceKey] = agents.map((agent) => ({
        name: agent.name,
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