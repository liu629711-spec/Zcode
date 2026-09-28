import { create } from "zustand";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import type { PersonaChatBadge } from "@/WorkspaceSidebar/projectAgentsModel.js";

const EMPTY_WORKSPACE_BADGES: ReadonlyMap<string, PersonaChatBadge> = new Map();
// localStorage 落盘：改名/删除补链写下的登记必须跨重启存活（标题反推对改名行失效，
// 登记是它们唯一的工牌来源）。键值结构尽量扁：workspaceKey → [taskId, badge] 列表。
const STORAGE_KEY = "zcode.personaChatBadges.v1";

type PersistedBadgeMap = Record<string, [string, PersonaChatBadge][]>;

function readPersistedBadges(): Record<string, ReadonlyMap<string, PersonaChatBadge>> {
  if (typeof window === "undefined") {
    return {};
  }
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return {};
    }
    const parsed = JSON.parse(raw) as PersistedBadgeMap;
    const restored: Record<string, ReadonlyMap<string, PersonaChatBadge>> = {};
    for (const [workspaceKey, entries] of Object.entries(parsed)) {
      if (!Array.isArray(entries)) continue;
      const map = new Map<string, PersonaChatBadge>();
      for (const [taskId, badge] of entries) {
        if (typeof taskId === "string" && taskId && badge && typeof badge.name === "string") {
          map.set(taskId, badge);
        }
      }
      if (map.size > 0) {
        restored[workspaceKey] = map;
      }
    }
    return restored;
  } catch {
    return {};
  }
}

function persistBadges(badgeByWorkspaceKey: Record<string, ReadonlyMap<string, PersonaChatBadge>>): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    const serializable: PersistedBadgeMap = {};
    for (const [workspaceKey, map] of Object.entries(badgeByWorkspaceKey)) {
      if (map.size > 0) {
        serializable[workspaceKey] = [...map.entries()];
      }
    }
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(serializable));
  } catch {
    // 落盘失败只损失跨重启工牌，不影响本次运行；不弹错打扰用户。
  }
}

/**
 * 驻场智能体会话的徽章登记（D2：任务列表行小徽章）。
 *
 * ponytail: 登记来源是"打开驻场智能体会话"的入口（此刻 UI 手里确实有档案），作用是
 * 会话刚建、列表行尚未带 persona 标题前缀时的即时徽章；登记已落 localStorage，改名
 * 补链（relabel）写下的登记跨重启也活着。未改过名的行冷重启由标题前缀反推
 * （applyDerivedPersonaChatBadges）兜底；meta_json 的 agentPersona 落盘（第二落点）
 * 落地前，手动改过标题的行靠登记存活。登记键是 taskId=sessionId，全局唯一。
 */
interface PersonaChatBadgeState {
  badgeByWorkspaceKey: Record<string, ReadonlyMap<string, PersonaChatBadge>>;
  registerPersonaChatBadge: (params: {
    workspacePath: string;
    workspaceIdentity?: string;
    taskId: string;
    badge: PersonaChatBadge;
  }) => void;
  /**
   * 档案改名补链：该工作区登记里这个员工的工牌整体换成 toBadge。
   * 带 fromAgentId 就按号找人（D26：号不随改名变，登记里名字陈旧也照样命中），
   * 没号的老登记回落到 fromName。标题前缀行的补挂由调用方 register 逐行写。
   */
  relabelPersonaChatBadges: (params: {
    workspacePath: string;
    workspaceIdentity?: string;
    fromName: string;
    fromAgentId?: string;
    toBadge: PersonaChatBadge;
  }) => void;
  /** 档案删除摘牌：name 或号命中的工牌摘掉（与标题反推在档案消失后的行为一致）。 */
  removePersonaChatBadgesByNames: (params: {
    workspacePath: string;
    workspaceIdentity?: string;
    names: readonly string[];
    agentIds?: readonly string[];
  }) => void;
}

export const usePersonaChatBadgeStore = create<PersonaChatBadgeState>()((set) => {
  const withWorkspaceMap = (
    state: PersonaChatBadgeState,
    params: { workspacePath: string; workspaceIdentity?: string },
    mutate: (map: Map<string, PersonaChatBadge>) => boolean,
  ): Partial<PersonaChatBadgeState> | null => {
    const workspaceKey = buildTaskWorkspaceKey(params.workspacePath, params.workspaceIdentity);
    const existing = state.badgeByWorkspaceKey[workspaceKey];
    const next = new Map(existing);
    if (!mutate(next)) {
      return null;
    }
    const badgeByWorkspaceKey = { ...state.badgeByWorkspaceKey, [workspaceKey]: next };
    persistBadges(badgeByWorkspaceKey);
    return { badgeByWorkspaceKey };
  };

  return {
    badgeByWorkspaceKey: readPersistedBadges(),
    registerPersonaChatBadge: ({ workspacePath, workspaceIdentity, taskId, badge }) => {
      set((state) => {
        const existing = state.badgeByWorkspaceKey[
          buildTaskWorkspaceKey(workspacePath, workspaceIdentity)
        ]?.get(taskId);
        if (existing === badge) {
          return state;
        }
        const patch = withWorkspaceMap(state, { workspacePath, workspaceIdentity }, (map) => {
          if (map.get(taskId) === badge) {
            return false;
          }
          map.set(taskId, badge);
          return true;
        });
        return patch ?? state;
      });
    },
    relabelPersonaChatBadges: ({
      workspacePath,
      workspaceIdentity,
      fromName,
      fromAgentId,
      toBadge,
    }) => {
      set((state) => {
        const patch = withWorkspaceMap(state, { workspacePath, workspaceIdentity }, (map) => {
          let changed = false;
          for (const [taskId, badge] of map) {
            // 两边都有号就只认号（D26）——名字并集会把"恰好叫旧名的另一个员工"换错牌；
            // 任一侧没号（号之前的老登记）才回落到旧名比对。
            const hit =
              fromAgentId !== undefined && badge.agentId !== undefined
                ? badge.agentId === fromAgentId
                : badge.name === fromName;
            if (hit && badge !== toBadge) {
              map.set(taskId, toBadge);
              changed = true;
            }
          }
          return changed;
        });
        return patch ?? state;
      });
    },
    removePersonaChatBadgesByNames: ({ workspacePath, workspaceIdentity, names, agentIds }) => {
      const nameSet = new Set(names);
      const agentIdSet = new Set(agentIds ?? []);
      set((state) => {
        const patch = withWorkspaceMap(state, { workspacePath, workspaceIdentity }, (map) => {
          let changed = false;
          for (const [taskId, badge] of map) {
            if (nameSet.has(badge.name) || (badge.agentId && agentIdSet.has(badge.agentId))) {
              map.delete(taskId);
              changed = true;
            }
          }
          return changed;
        });
        return patch ?? state;
      });
    },
  };
});

/** 行级订阅选择器：返回该工作区的徽章登记（引用稳定，空表为常量，不触发多余重渲染）。 */
export function selectPersonaChatBadgesForWorkspace(
  state: PersonaChatBadgeState,
  workspaceKey: string,
): ReadonlyMap<string, PersonaChatBadge> {
  return state.badgeByWorkspaceKey[workspaceKey] ?? EMPTY_WORKSPACE_BADGES;
}

/** 测试/补链用：读取当前登记的只读快照（不经 hook）。 */
export function getPersonaChatBadgesSnapshot(): PersonaChatBadgeState["badgeByWorkspaceKey"] {
  return usePersonaChatBadgeStore.getState().badgeByWorkspaceKey;
}

/** 测试用：清空登记与落盘（不清理真实用户数据的路径）。 */
export function resetPersonaChatBadgeStoreForTest(): void {
  if (typeof window !== "undefined") {
    window.localStorage.removeItem(STORAGE_KEY);
  }
  usePersonaChatBadgeStore.setState({ badgeByWorkspaceKey: {} });
}
