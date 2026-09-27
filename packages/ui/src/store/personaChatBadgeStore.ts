import { create } from "zustand";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import type { PersonaChatBadge } from "@/WorkspaceSidebar/projectAgentsModel.js";

const EMPTY_WORKSPACE_BADGES: ReadonlyMap<string, PersonaChatBadge> = new Map();

/**
 * 驻场智能体会话的徽章登记（D2：任务列表行小徽章）。
 *
 * ponytail: 登记来源只有"打开驻场智能体会话"的入口（此刻 UI 手里确实有档案），所以
 * 冷重启后徽章会消失——持久化 producer（tasks-index meta_json 的 agentPersona 标记，
 * 身份片已定的第二落点）落地前这是已知上限；登记键是 taskId=sessionId，全局唯一，
 * 删会话后残留的登记无副作用，不做清理。
 */
interface PersonaChatBadgeState {
  badgeByWorkspaceKey: Record<string, ReadonlyMap<string, PersonaChatBadge>>;
  registerPersonaChatBadge: (params: {
    workspacePath: string;
    workspaceIdentity?: string;
    taskId: string;
    badge: PersonaChatBadge;
  }) => void;
}

export const usePersonaChatBadgeStore = create<PersonaChatBadgeState>()((set) => ({
  badgeByWorkspaceKey: {},
  registerPersonaChatBadge: ({ workspacePath, workspaceIdentity, taskId, badge }) => {
    const workspaceKey = buildTaskWorkspaceKey(workspacePath, workspaceIdentity);
    set((state) => {
      const existing = state.badgeByWorkspaceKey[workspaceKey];
      if (existing?.get(taskId) === badge) {
        return state;
      }
      const next = new Map(existing);
      next.set(taskId, badge);
      return {
        badgeByWorkspaceKey: { ...state.badgeByWorkspaceKey, [workspaceKey]: next },
      };
    });
  },
}));

/** 行级订阅选择器：返回该工作区的徽章登记（引用稳定，空表为常量，不触发多余重渲染）。 */
export function selectPersonaChatBadgesForWorkspace(
  state: PersonaChatBadgeState,
  workspaceKey: string,
): ReadonlyMap<string, PersonaChatBadge> {
  return state.badgeByWorkspaceKey[workspaceKey] ?? EMPTY_WORKSPACE_BADGES;
}
