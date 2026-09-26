// 派活台看板（列模式）：六列布局的只读看板容器，纯呈现——零拖拽零写路径
// （拖放是片C，评审动作是片D）。数据走 useGroupedTaskView 既有链路，与列表模式二选一挂载。
import { useRef } from "react";
import type { ZCodeTaskMeta } from "@zcode/shared";
import { TID_DISPATCH_DESK_BOARD, TID_DISPATCH_DESK_COLUMN } from "@zcode/shared";
import type { ZCodeGroupedTaskView } from "@zcode/services";
import { Button } from "@/components/ui/button.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { shouldHideGroupedTaskContent, useGroupedTaskView } from "@/hooks/useGroupedTaskView.js";
import type { WorkspaceTabState } from "@/store/tabStore.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import { DispatchDeskCard } from "@/workspace-grouped-tasks/kanban-card.js";
import {
  groupDispatchDeskColumns,
  type DispatchDeskColumnId,
} from "@/workspace-grouped-tasks/kanban-columns.js";
import { taskKey } from "@/workspace-grouped-tasks/ids.js";
import { TASK_GROUP_COUNT_BADGE_CLASS } from "@/workspace-grouped-tasks/types.js";

/** 列头状态点：复用 run-status STATUS_DOT 词汇（等待=空环不红、在途=动感、失败=红、通过=绿）。 */
const COLUMN_DOT: Record<DispatchDeskColumnId, string> = {
  idle: STATUS_DOT.pending,
  dispatching: STATUS_DOT.running,
  pendingReview: STATUS_DOT.pending,
  failed: STATUS_DOT.failed,
  approved: STATUS_DOT.done,
};

const COLUMN_BACKGROUND: Record<DispatchDeskColumnId, string> = {
  idle: "bg-surface/50",
  dispatching: "bg-surface/50",
  pendingReview: "bg-warning/5",
  failed: "bg-destructive/5",
  approved: "bg-success/5",
};

function collectBoardTasks(view: ZCodeGroupedTaskView): ZCodeTaskMeta[] {
  const tasks: ZCodeTaskMeta[] = [];
  for (const node of view.nodes) {
    if (node.type === "group") {
      tasks.push(...node.tasks);
      continue;
    }
    tasks.push(node.task);
  }
  return tasks;
}

export function DispatchDeskKanbanBoard(props: { workspaceTabs: WorkspaceTabState[] }) {
  const { workspaceTabs } = props;
  const { intl } = useZCodeIntl();
  const { view, loading, initialized, loadError, refresh } = useGroupedTaskView({
    workspaceTabs,
  });
  // 门禁只拦首屏（旧数据优于空白）；画过一次后不再回隐藏态，同 WorkspaceGroupedTasksSection 闩锁惯例。
  const hasPaintedBoardRef = useRef(false);
  const tasks = collectBoardTasks(view);
  const columns = groupDispatchDeskColumns(tasks, Date.now());
  const totalTickets = columns.reduce((sum, column) => sum + column.tickets.length, 0);
  if (totalTickets > 0) {
    hasPaintedBoardRef.current = true;
  }
  if (
    shouldHideGroupedTaskContent({
      initialized,
      loading,
      hasNodes: totalTickets > 0,
      hasPaintedOnce: hasPaintedBoardRef.current,
    })
  ) {
    return null;
  }

  return (
    <div
      data-testid={TID_DISPATCH_DESK_BOARD}
      aria-label={intl.formatMessage({ id: "dispatchDesk.title" })}
      className="flex gap-2 overflow-x-auto pb-2"
    >
      {loadError && totalTickets === 0 ? (
        <div className="flex w-full items-center justify-center py-6">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={() => {
              void refresh();
            }}
          >
            {intl.formatMessage({ id: "dispatchDesk.loadFailedRetry" })}
          </Button>
        </div>
      ) : totalTickets === 0 ? (
        <div className="w-full px-3 py-2 text-ui-base text-foreground-subtle">
          {intl.formatMessage({ id: "dispatchDesk.empty" })}
        </div>
      ) : (
        columns.map((column) => (
          <section
            key={column.id}
            data-testid={TID_DISPATCH_DESK_COLUMN}
            data-column={column.id}
            aria-label={intl.formatMessage({ id: `dispatchDesk.column.${column.id}` })}
            className={`flex w-[272px] shrink-0 flex-col rounded-xl p-2 ${COLUMN_BACKGROUND[column.id]}`}
          >
            <div className="flex h-8 min-w-0 items-center gap-1.5">
              <span
                aria-hidden="true"
                className={`size-2 shrink-0 rounded-full ${COLUMN_DOT[column.id]}`}
              />
              <span className="truncate text-ui-base font-medium text-foreground">
                {intl.formatMessage({ id: `dispatchDesk.column.${column.id}` })}
              </span>
              <span className={TASK_GROUP_COUNT_BADGE_CLASS}>{column.tickets.length}</span>
            </div>
            <div className="mt-1 flex flex-col gap-2">
              {column.tickets.length === 0 ? (
                <div className="py-4 text-center text-ui-sm text-foreground-subtlest">
                  {intl.formatMessage({ id: "dispatchDesk.emptyColumn" })}
                </div>
              ) : (
                column.tickets.map(({ task, projection }) => (
                  <DispatchDeskCard key={taskKey(task)} task={task} projection={projection} />
                ))
              )}
            </div>
          </section>
        ))
      )}
    </div>
  );
}
