// 待你拍板（跨任务待批聚合 2026-10-12）的聚合纯函数层：刻意不带任何 @/ 别名
// 依赖（ui gate 测试在 node 直跑编译产物，别名解析不了），判定与任务行红点同源。
import type { ZCodeTaskMeta } from "@zcode/shared";
import { getTaskListAttention } from "../v4/taskListRowActivity.js";

export interface PendingApprovalItem {
  task: ZCodeTaskMeta;
  workspacePath: string;
  workspaceIdentity?: string;
  /** 该会话挂着的待批总数（权限确认 + 用户提问）；旧摘要缺计数时回落 1。 */
  count: number;
}

export interface PendingApprovalTaskGroup {
  workspacePath: string;
  workspaceIdentity?: string;
  items: readonly ZCodeTaskMeta[];
}

/**
 * 聚合纯函数：跨分组收集"在等老板"的任务行，按分组内既有顺序（新→旧）平铺。
 * 判定与 TaskListItem 行内红点同源（pendingInteraction 或活性计数 >0），不另立口径。
 */
export function collectPendingApprovalItems(
  groups: readonly PendingApprovalTaskGroup[],
): PendingApprovalItem[] {
  const items: PendingApprovalItem[] = [];
  for (const group of groups) {
    for (const task of group.items) {
      const attention = getTaskListAttention(task);
      if (attention) {
        items.push({
          task,
          workspacePath: group.workspacePath,
          ...(group.workspaceIdentity ? { workspaceIdentity: group.workspaceIdentity } : {}),
          count: attention.count,
        });
        continue;
      }
      // 旧摘要只有队首单条、没有计数 sidecar：有 pendingInteraction 也算在等。
      if (task.pendingInteraction) {
        items.push({
          task,
          workspacePath: group.workspacePath,
          ...(group.workspaceIdentity ? { workspaceIdentity: group.workspaceIdentity } : {}),
          count: 1,
        });
      }
    }
  }
  return items;
}
