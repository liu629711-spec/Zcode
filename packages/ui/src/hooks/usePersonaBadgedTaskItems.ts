import { useMemo } from "react";
import {
  resolvePersonaChatBadgeForTask,
  type PersonaChatBadgeCarrier,
} from "@/WorkspaceSidebar/projectAgentsModel.js";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import { usePersonaChatBadgeStore } from "@/store/personaChatBadgeStore.js";
import { useProjectAgentDirectoryStore } from "@/store/projectAgentDirectoryStore.js";

/**
 * 任务行的员工徽章合并（D2 跨视图版）：三路对号（行内 → 持久登记 → 标题反推）
 * 与侧栏项目视图完全同优先级——时间线/置顶等视图从这统一吃结果，
 * 不再出现"项目视图认得出员工、切个视图就隐身"的分裂。
 * 未命中行保持原引用，避免打穿行级 memo。
 */
export function usePersonaBadgedTaskItems<
  T extends { taskId: string; title: string; workspacePath: string; workspaceIdentity?: string },
>(items: readonly T[]): (T & PersonaChatBadgeCarrier)[] {
  const badgeByWorkspaceKey = usePersonaChatBadgeStore((state) => state.badgeByWorkspaceKey);
  const agentsByWorkspaceKey = useProjectAgentDirectoryStore((state) => state.agentsByWorkspaceKey);
  return useMemo(
    () =>
      items.map((item): T & PersonaChatBadgeCarrier => {
        if ((item as T & PersonaChatBadgeCarrier).agentPersona) {
          return item;
        }
        const workspaceKey = buildTaskWorkspaceKey(item.workspacePath, item.workspaceIdentity);
        const badge = resolvePersonaChatBadgeForTask(
          item,
          badgeByWorkspaceKey[workspaceKey],
          agentsByWorkspaceKey[workspaceKey],
        );
        return badge ? { ...item, agentPersona: badge } : item;
      }),
    [items, badgeByWorkspaceKey, agentsByWorkspaceKey],
  );
}