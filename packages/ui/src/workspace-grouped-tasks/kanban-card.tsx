// 派活台看板卡片：拖拽 handle（列模式独立数据面，见 kanban-transitions.ts）+ 卡上动作按钮。
// 动作只经 onAction 上抛，写路径收敛在看板容器（唯一人工通道 = 两个服务方法）。
// 状态徽标/图钉只复用本仓既有词汇：确认系带色（TaskInteractionBadge）、转圈（task-row）、
// 完成注意力 bg-success/14（task-row）、状态点词汇（run-status-presentation）。
import { LoaderIcon, XCircle } from "lucide-react";
import { useDraggable } from "@dnd-kit/core";
import type { ZCodeTaskMeta } from "@zcode/shared";
import { TID_DISPATCH_DESK_CARD } from "@zcode/shared";
import { cn } from "@/components/lib/utils.js";
import { Badge } from "@/components/ui/badge.js";
import { Button } from "@/components/ui/button.js";
import type { CodeViewerSource, FileCodeViewerSource } from "@/lib/codeViewer.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { formatTaskRelativeTime } from "@/lib/taskListItemPresentation.js";
import { getPathLeaf } from "@/lib/path.js";
import {
  DISPATCH_DESK_MAX_ATTEMPTS,
  type DispatchDeskTicketProjection,
} from "@/workspace-grouped-tasks/kanban-columns.js";
import {
  canDispatchDeskTicketDrag,
  getDispatchDeskCardActions,
  type DispatchDeskActionKind,
} from "@/workspace-grouped-tasks/kanban-transitions.js";
import { DispatchDeskCardDeliverables } from "@/workspace-grouped-tasks/kanban-deliverables.js";
import { taskKey } from "@/workspace-grouped-tasks/ids.js";

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

/** 卡上动作文案：approve 无按钮（批准只有拖拽一条路，经确认框），键保留给语义对表。 */
const ACTION_LABEL_ID: Record<DispatchDeskActionKind, string> = {
  approve: "dispatchDesk.review.approve",
  requestChanges: "dispatchDesk.review.requestChanges",
  requeue: "dispatchDesk.action.requeue",
};

export function DispatchDeskCard(props: {
  task: ZCodeTaskMeta;
  projection: DispatchDeskTicketProjection;
  /** 拖拽 overlay 克隆：纯展示——无 handle、无动作按钮、禁指针。 */
  dragOverlay?: boolean;
  /** 动作进行中锁卡：禁二次点击与拖拽（服务在途，重复发起只会收到 null）。 */
  locked?: boolean;
  onAction?: (kind: DispatchDeskActionKind) => void;
  /** 成卡行预览入口：缺席时成卡行不渲染打开按钮（拖拽 overlay 克隆天然缺席）。 */
  onOpenCodeViewer?: (source: CodeViewerSource) => void;
}) {
  const { task, projection, dragOverlay = false, locked = false, onAction, onOpenCodeViewer } = props;
  const { intl } = useZCodeIntl();
  // 成卡项的 source 补上任务 workspace 作用域，PreviewPane 才能用正确 host 读取文件。
  const openDeliverable = onOpenCodeViewer
    ? (source: FileCodeViewerSource) =>
        onOpenCodeViewer({
          ...source,
          workspacePath: task.workspacePath,
          ...(task.workspaceIdentity ? { workspaceIdentity: task.workspaceIdentity } : {}),
        })
    : undefined;
  const backoff =
    projection.column === "failed" && !projection.needsHuman && !projection.blockedDone;
  const actions = dragOverlay ? [] : getDispatchDeskCardActions(projection);
  const draggable = useDraggable({
    id: `dispatch-desk-card:${taskKey(task)}`,
    disabled: dragOverlay || locked || !canDispatchDeskTicketDrag(projection),
    data: { type: "dispatch-desk-card", columnId: projection.column, task, projection },
  });
  // handle 只在真的可拖时挂载：approved/blockedDone/无去向卡不留任何拖拽入口。
  const dragHandleProps =
    dragOverlay || locked || !canDispatchDeskTicketDrag(projection)
      ? {}
      : { ...draggable.attributes, ...draggable.listeners };

  return (
    <div
      ref={dragOverlay ? undefined : draggable.setNodeRef}
      {...dragHandleProps}
      data-testid={TID_DISPATCH_DESK_CARD}
      className={cn(
        "rounded-lg border border-border bg-background p-2",
        dragOverlay
          ? // 拖起 overlay：旋转 1deg + 高不透明 + 投影；禁指针防止落点被 overlay 自身截走。
            "pointer-events-none cursor-grabbing rotate-1 opacity-90 shadow-lg"
          : draggable.isDragging && "opacity-30",
      )}
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
      {/* 第 5 行 成卡/交付行：deliverables 缺席/不可辨时整行缺席（无则缺席）。 */}
      <DispatchDeskCardDeliverables
        deliverables={task.deliverables}
        onOpenCodeViewer={openDeliverable}
      />
      <div
        className={cn(
          "mt-1.5 flex items-center justify-between gap-2 text-ui-sm text-foreground-subtle",
          backoff && "tabular-nums",
        )}
      >
        <span>{formatTaskRelativeTime(task.updatedAt || task.createdAt, intl)}</span>
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
      {actions.length > 0 ? (
        <div className="mt-1.5 flex items-center gap-1">
          {actions.map((kind) => (
            <Button
              key={kind}
              type="button"
              variant="ghost"
              size="sm"
              disabled={locked}
              // 卡根是拖拽 handle：按钮按下必须断开 sensor 事件链，点击/拖拽互不劫持。
              onPointerDown={(event) => event.stopPropagation()}
              onMouseDown={(event) => event.stopPropagation()}
              onClick={(event) => {
                event.stopPropagation();
                if (!locked) {
                  onAction?.(kind);
                }
              }}
            >
              {intl.formatMessage({ id: ACTION_LABEL_ID[kind] })}
            </Button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
