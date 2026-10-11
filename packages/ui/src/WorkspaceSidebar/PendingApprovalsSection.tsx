// 待你拍板（跨任务待批聚合 2026-10-12）的侧栏分组组件：聚合纯函数在
// ./pendingApprovalsModel.js（无 @/ 别名依赖，gate 测试直跑）；这里只负责渲染。
// 有单才出现、点击行走侧栏同一条 onSelectTask 跳转协议（跨 workspace 自动切 tab）。
import { useMemo } from "react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import { TaskInteractionBadge } from "@/TaskInteractionBadge.js";
import { collectPendingApprovalItems, type PendingApprovalItem, type PendingApprovalTaskGroup } from "./pendingApprovalsModel.js";

export type { PendingApprovalItem, PendingApprovalTaskGroup };
export { collectPendingApprovalItems };

export function WorkspacePendingApprovalsSection({
  items,
  onSelectTask,
}: {
  items: readonly PendingApprovalItem[];
  onSelectTask: (
    targetWorkspacePath: string,
    taskId: string,
    targetWorkspaceIdentity?: string,
  ) => void;
}) {
  const { intl } = useZCodeIntl();
  const totalCount = useMemo(
    () => items.reduce((sum, item) => sum + item.count, 0),
    [items],
  );
  if (items.length === 0) {
    return null;
  }
  const title = intl.formatMessage({ id: "workspaceSidebar.pendingApprovals.title" });
  const hint = intl.formatMessage({ id: "workspaceSidebar.pendingApprovals.hint" });
  return (
    <section
      aria-label={title}
      data-testid="workspace-pending-approvals"
      className="px-2 pb-1"
    >
      <div className="overflow-hidden rounded-xl border border-border bg-card">
        <div className="flex items-center justify-between gap-2 px-2.5 py-1.5" title={hint}>
          <h3 className="truncate text-ui-base font-medium text-foreground">{title}</h3>
          <span
            data-testid="workspace-pending-approvals-count"
            className="inline-flex h-5 shrink-0 items-center justify-center rounded-full bg-interaction-confirmation-surface px-2 text-ui-sm font-medium text-interaction-confirmation-foreground"
          >
            {totalCount}
          </span>
        </div>
        <ul className="max-h-56 overflow-y-auto pb-1">
          {items.map((item) => (
            <li
              key={
                buildTaskWorkspaceKey(item.workspacePath, item.workspaceIdentity) +
                ":" +
                item.task.taskId
              }
            >
              <button
                type="button"
                className="flex w-full min-w-0 items-center gap-2 px-2.5 py-1.5 text-left transition-colors hover:bg-hover"
                onClick={() =>
                  onSelectTask(item.workspacePath, item.task.taskId, item.workspaceIdentity)
                }
              >
                <span className="min-w-0 flex-1 truncate text-ui-base text-foreground">
                  {item.task.title}
                </span>
                <TaskInteractionBadge
                  interaction={item.task.pendingInteraction}
                  formatMessage={(id) => intl.formatMessage({ id })}
                />
                {item.count > 1 ? (
                  <span className="shrink-0 text-ui-xs text-foreground-subtlest">×{item.count}</span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
