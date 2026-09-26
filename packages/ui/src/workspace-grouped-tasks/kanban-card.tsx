// 派活台看板卡片：纯呈现，零写路径零拖拽（拖放是片C，成卡入口是片D）。
// 状态徽标/图钉只复用本仓既有词汇：确认系带色（TaskInteractionBadge）、转圈（task-row）、
// 完成注意力 bg-success/14（task-row）、状态点词汇（run-status-presentation）。
import { LoaderIcon, XCircle } from "lucide-react";
import type { ZCodeTaskMeta } from "@zcode/shared";
import { TID_DISPATCH_DESK_CARD } from "@zcode/shared";
import { cn } from "@/components/lib/utils.js";
import { Badge } from "@/components/ui/badge.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { formatTaskRelativeTime } from "@/lib/taskListItemPresentation.js";
import { getPathLeaf } from "@/lib/path.js";
import {
  DISPATCH_DESK_MAX_ATTEMPTS,
  type DispatchDeskTicketProjection,
} from "@/workspace-grouped-tasks/kanban-columns.js";

/** 待审确认系带：TaskInteractionBadge.tsx 的 badgeClassName 词汇（同色不同控件，语义同源）。 */
const REVIEW_CONFIRMATION_BADGE_CLASS =
  "h-5 rounded-full border border-transparent bg-interaction-confirmation-surface px-2 text-ui-sm font-medium text-interaction-confirmation-foreground";
/** 完成注意力徽标：task-row.tsx 完成态同款。 */
const SUCCESS_ATTENTION_BADGE_CLASS =
  "h-5 border-transparent bg-success/14 px-2 text-ui-base font-medium text-success dark:bg-success/18";
/** 需人工红标：完成注意力的 destructive 变体（红色只属于出错/待人工，不用于等待）。 */
const NEEDS_HUMAN_BADGE_CLASS =
  "h-5 border-transparent bg-destructive/14 px-2 text-ui-base font-medium text-destructive dark:bg-destructive/18";
/** 返工徽标：注意力系 warning（打回是待办，不是故障）。 */
const REWORK_BADGE_CLASS =
  "h-5 border-transparent bg-warning/14 px-2 text-ui-base font-medium text-warning dark:bg-warning/18";

export function DispatchDeskCard(props: {
  task: ZCodeTaskMeta;
  projection: DispatchDeskTicketProjection;
}) {
  const { task, projection } = props;
  const { intl } = useZCodeIntl();
  const backoff = projection.column === "failed" && !projection.needsHuman && !projection.blockedDone;

  return (
    <div
      data-testid={TID_DISPATCH_DESK_CARD}
      className="rounded-lg border border-border bg-background p-2"
    >
      <div className="flex min-w-0 items-center gap-1.5 text-ui-sm text-foreground-subtlest">
        <span className="truncate">{getPathLeaf(task.workspacePath) || task.workspacePath}</span>
        <span className="truncate">{task.taskId}</span>
        <span className="ml-auto flex shrink-0 items-center gap-1">
          {projection.rework ? (
            <Badge className={REWORK_BADGE_CLASS}>
              {intl.formatMessage({ id: "dispatchDesk.column.changesRequested" })}
            </Badge>
          ) : null}
          {projection.column === "pendingReview" ? (
            <Badge className={REVIEW_CONFIRMATION_BADGE_CLASS}>
              {intl.formatMessage({ id: "dispatchDesk.column.pendingReview" })}
            </Badge>
          ) : null}
          {projection.column === "approved" ? (
            <Badge className={SUCCESS_ATTENTION_BADGE_CLASS}>
              {intl.formatMessage({ id: "dispatchDesk.column.approved" })}
            </Badge>
          ) : null}
          {projection.needsHuman || projection.blockedDone ? (
            <Badge className={NEEDS_HUMAN_BADGE_CLASS}>
              {intl.formatMessage({ id: "dispatchDesk.needsHuman" })}
            </Badge>
          ) : null}
          {projection.column === "dispatching" ? (
            <LoaderIcon className="size-3.5 shrink-0 animate-spin text-foreground-subtle motion-reduce:animate-none" />
          ) : null}
        </span>
      </div>
      <div className="mt-1 line-clamp-2 text-ui-base font-medium text-foreground">{task.title}</div>
      {task.acceptanceCriteria ? (
        <div className="mt-1 line-clamp-1 text-ui-sm text-foreground-subtlest">
          {task.acceptanceCriteria}
        </div>
      ) : null}
      {projection.column === "failed" && task.lastDispatchError ? (
        <div
          className="mt-1 flex min-w-0 items-center gap-1 text-ui-sm text-destructive"
          title={task.lastDispatchError}
        >
          <XCircle className="size-3.5 shrink-0" aria-hidden="true" />
          <span className="truncate">{task.lastDispatchError}</span>
        </div>
      ) : null}
      <div
        className={cn(
          "mt-1.5 flex items-center justify-between gap-2 text-ui-sm text-foreground-subtle",
          backoff && "tabular-nums",
        )}
      >
        <span>
          {formatTaskRelativeTime(task.updatedAt || task.createdAt, intl)}
        </span>
        {backoff ? (
          <span>
            {intl.formatMessage(
              { id: "dispatchDesk.retryCountdown" },
              { minutes: String(projection.retryMinutes ?? 0) },
            )}
            {"·"}
            {intl.formatMessage(
              { id: "dispatchDesk.attempts" },
              {
                count: String(task.dispatchAttempts ?? 0),
                max: String(DISPATCH_DESK_MAX_ATTEMPTS),
              },
            )}
          </span>
        ) : null}
      </div>
    </div>
  );
}
