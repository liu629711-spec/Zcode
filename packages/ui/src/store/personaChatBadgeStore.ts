import { create } from "zustand";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import type { PersonaChatBadge } from "@/WorkspaceSidebar/projectAgentsModel.js";

const EMPTY_WORKSPACE_BADGES: ReadonlyMap<string, PersonaChatBadge> = new Map();

/**
 * 驻场智能体会话的徽章登记（D2：任务列表行小徽章）。
 *
 * ponytail: 登记来源是"打开驻场智能体会话"的入口（此刻 UI 手里确实有档案），作用是
 * 会话刚建、列表行尚未带 persona 标题前缀时的即时徽章。冷重启后登记会丢，但侧栏
 * 用 applyDerivedPersonaChatBadges 按标题前缀（「智能体名 · …」随 tasks-index 持久）
 * 反推补上；meta_json 的 agentPersona 落盘（第二落点）落地前，手动改过标题的行
 * 反推不到——已知上限。登记键是 taskId=sessionId，全局唯一，删会话后残留登记无副作用。
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
