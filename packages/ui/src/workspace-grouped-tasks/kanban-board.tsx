// 派活台看板（列模式）：五列布局的人工闸机容器。拖拽流转 + 卡上动作，唯一写路径是
// zcodeTaskService.recordReviewDecision / manualRequeue 两个人工通道方法；守卫返回
// null=竞态落空，必须提示+刷新对齐真相，绝不静默。拖拽数据面是列模式独立的
// dispatch-desk-* 类型（见 kanban-transitions.ts），列表模式的行重排住在
// WorkspaceGroupedTasksSection 自己的 DndContext 里，二选一挂载互不感知（零改动红线）。
import { useCallback, useRef, useState, type ReactNode } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  pointerWithin,
  useDroppable,
  useSensor,
  useSensors,
  type CollisionDetection,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { createPortal } from "react-dom";
import type { ZCodeTaskMeta } from "@zcode/shared";
import { TID_DISPATCH_DESK_BOARD, TID_DISPATCH_DESK_COLUMN } from "@zcode/shared";
import type { ZCodeGroupedTaskView } from "@zcode/services";
import { cn } from "@/components/lib/utils.js";
import type { CodeViewerSource } from "@/lib/codeViewer.js";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog.js";
import { Button } from "@/components/ui/button.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { logger } from "@/logger.js";
import { useBaseWorkspaceServices } from "@/hooks/useWorkspaceServices.js";
import { shouldHideGroupedTaskContent, useGroupedTaskView } from "@/hooks/useGroupedTaskView.js";
import type { WorkspaceTabState } from "@/store/tabStore.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import { DispatchDeskCard } from "@/workspace-grouped-tasks/kanban-card.js";
import {
  groupDispatchDeskColumns,
  type DispatchDeskColumn,
  type DispatchDeskColumnId,
} from "@/workspace-grouped-tasks/kanban-columns.js";
import {
  DISPATCH_DESK_COLUMN_DROPPABLE_TYPE,
  doesDispatchDeskDragRequireConfirm,
  isDispatchDeskDragLegal,
  type DispatchDeskActionRequest,
  type DispatchDeskCardDragData,
  type DispatchDeskColumnDropData,
} from "@/workspace-grouped-tasks/kanban-transitions.js";
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

// 列优先碰撞：指针在列体内直接命中，落在列间缝隙回退最近列心。本 DndContext 只注册
// 列 droppable，卡片不参与碰撞——落点永远是"列"，不是卡缝。
const dispatchDeskCollisionDetection: CollisionDetection = (args) => {
  const pointerCollisions = pointerWithin(args);
  return pointerCollisions.length > 0 ? pointerCollisions : closestCenter(args);
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

/** 批准确认框：批准即终态不可逆，确认后才允许发起 recordReviewDecision。 */
function DispatchDeskApproveConfirmDialog(props: {
  request: DispatchDeskActionRequest | null;
  onCancel: () => void;
  onConfirm: (request: DispatchDeskActionRequest) => void;
}) {
  const { intl } = useZCodeIntl();
  return (
    <AlertDialog
      open={props.request !== null}
      onOpenChange={(open) => {
        if (!open) {
          props.onCancel();
        }
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {intl.formatMessage({ id: "dispatchDesk.confirm.approve.title" })}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {intl.formatMessage({ id: "dispatchDesk.confirm.approve.body" })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel type="button" size="sm">
            {intl.formatMessage({ id: "dispatchDesk.confirm.approve.cancel" })}
          </AlertDialogCancel>
          <AlertDialogAction
            type="button"
            size="sm"
            onClick={() => {
              // Radix 默认收起弹窗即所需语义；进行中锁与失败提示由 performAction 负责。
              if (props.request) {
                props.onConfirm(props.request);
              }
            }}
          >
            {intl.formatMessage({ id: "dispatchDesk.confirm.approve.confirm" })}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function DispatchDeskColumnView(props: {
  column: DispatchDeskColumn;
  children: ReactNode;
  /** 当前拖拽的非法目标列禁用 droppable：不参与碰撞，isOver 恒 false，悬停无任何反馈。 */
  droppableDisabled: boolean;
}) {
  const { intl } = useZCodeIntl();
  const { isOver, setNodeRef } = useDroppable({
    id: `dispatch-desk-column:${props.column.id}`,
    data: {
      type: DISPATCH_DESK_COLUMN_DROPPABLE_TYPE,
      columnId: props.column.id,
    },
    disabled: props.droppableDisabled,
  });
  return (
    <section
      ref={setNodeRef}
      data-testid={TID_DISPATCH_DESK_COLUMN}
      data-column={props.column.id}
      aria-label={intl.formatMessage({ id: `dispatchDesk.column.${props.column.id}` })}
      // 悬停高亮替换底色而不是叠加：两个 bg-* 同层时由样式表顺序裁决，不可靠。
      className={cn(
        "flex min-w-[272px] flex-1 flex-col rounded-xl p-2",
        isOver ? "bg-selected" : COLUMN_BACKGROUND[props.column.id],
      )}
    >
      <div className="flex h-8 min-w-0 items-center gap-1.5">
        <span
          aria-hidden="true"
          className={`size-2 shrink-0 rounded-full ${COLUMN_DOT[props.column.id]}`}
        />
        <span className="truncate text-ui-base font-medium text-foreground">
          {intl.formatMessage({ id: `dispatchDesk.column.${props.column.id}` })}
        </span>
        <span className={TASK_GROUP_COUNT_BADGE_CLASS}>{props.column.tickets.length}</span>
      </div>
      <div className="mt-1 flex flex-col gap-2">{props.children}</div>
    </section>
  );
}

export function DispatchDeskKanbanBoard(props: {
  workspaceTabs: WorkspaceTabState[];
  /** 成卡行预览入口透传（拖拽 overlay 克隆不传）：可选回调，缺席即卡片无预览按钮。 */
  onOpenCodeViewer?: (source: CodeViewerSource) => void;
}) {
  const { workspaceTabs, onOpenCodeViewer } = props;
  const { intl } = useZCodeIntl();
  const services = useBaseWorkspaceServices();
  const { view, loading, initialized, loadError, refresh } = useGroupedTaskView({
    workspaceTabs,
  });
  // 门禁只拦首屏（旧数据优于空白）；画过一次后不再回隐藏态，同 WorkspaceGroupedTasksSection 闩锁惯例。
  const hasPaintedBoardRef = useRef(false);
  const [activeDrag, setActiveDrag] = useState<DispatchDeskCardDragData | null>(null);
  // 动作进行中锁：按票粒度锁卡（禁二次点击/拖拽），不同卡的并发动作互不阻塞。
  const [pendingActionKeys, setPendingActionKeys] = useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );
  // 批准确认门控：拖拽松手只打开确认框，确认后才发起服务调用。
  const [approvalRequest, setApprovalRequest] = useState<DispatchDeskActionRequest | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 6,
      },
    }),
  );

  const performAction = useCallback(
    (request: DispatchDeskActionRequest) => {
      const key = taskKey(request.task);
      setPendingActionKeys((previous) => new Set(previous).add(key));
      const ref = {
        workspacePath: request.task.workspacePath,
        workspaceIdentity: request.task.workspaceIdentity,
        taskId: request.task.taskId,
      };
      void (async () => {
        try {
          const now = Date.now();
          const result =
            request.kind === "requeue"
              ? await services.zcodeTaskService.manualRequeue({ ...ref, now })
              : await services.zcodeTaskService.recordReviewDecision({
                  ...ref,
                  decision: request.kind === "approve" ? "approved" : "changes_requested",
                  now,
                });
          if (result === null) {
            // 竞态落空（守卫 changes=0）：绝不静默——提示后刷新；卡片因数据未动自然回弹原列。
            toast(intl.formatMessage({ id: "dispatchDesk.ticketStateChanged" }), {
              variant: "info",
            });
          }
        } catch (error) {
          logger.error("[DispatchDeskKanbanBoard] 人工动作调用失败", error);
          toast(intl.formatMessage({ id: "dispatchDesk.moveFailed" }), { variant: "warning" });
        } finally {
          setPendingActionKeys((previous) => {
            if (!previous.has(key)) {
              return previous;
            }
            const next = new Set(previous);
            next.delete(key);
            return next;
          });
          void refresh();
        }
      })();
    },
    [intl, refresh, services.zcodeTaskService],
  );

  const handleDragStart = useCallback((event: DragStartEvent) => {
    const data = event.active.data.current as DispatchDeskCardDragData | undefined;
    if (data?.type !== "dispatch-desk-card") {
      return;
    }
    setActiveDrag(data);
  }, []);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      setActiveDrag(null);
      const active = event.active.data.current as DispatchDeskCardDragData | undefined;
      const over = event.over?.data.current as DispatchDeskColumnDropData | undefined;
      if (
        active?.type !== "dispatch-desk-card" ||
        over?.type !== DISPATCH_DESK_COLUMN_DROPPABLE_TYPE
      ) {
        // 非法/无处可放：不写任何状态，overlay 依 dropAnimation 吸回原位（沉默拒绝）。
        return;
      }
      if (!isDispatchDeskDragLegal(active.columnId, over.columnId)) {
        return;
      }
      if (doesDispatchDeskDragRequireConfirm(active.columnId, over.columnId)) {
        setApprovalRequest({ kind: "approve", task: active.task });
        return;
      }
      performAction({ kind: "requeue", task: active.task });
    },
    [performAction],
  );

  const handleDragCancel = useCallback(() => {
    setActiveDrag(null);
  }, []);

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

  const dragOverlayNode = activeDrag ? (
    <DragOverlay dropAnimation={{ duration: 150, easing: "cubic-bezier(0.2, 0, 0, 1)" }}>
      {/* 列宽自适应（min 272px、flex-1 均分），卡宽随主区宽度浮动约 256~360px；
          overlay 取中位近似 w-80（320px），避免引入测量逻辑，宽度误差有界可接受。 */}
      <div className="w-80">
        <DispatchDeskCard task={activeDrag.task} projection={activeDrag.projection} dragOverlay />
      </div>
    </DragOverlay>
  ) : null;

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={dispatchDeskCollisionDetection}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={handleDragCancel}
    >
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
            <DispatchDeskColumnView
              key={column.id}
              column={column}
              droppableDisabled={
                activeDrag !== null && !isDispatchDeskDragLegal(activeDrag.columnId, column.id)
              }
            >
              {column.tickets.length === 0 ? (
                <div className="py-4 text-center text-ui-sm text-foreground-subtlest">
                  {intl.formatMessage({ id: "dispatchDesk.emptyColumn" })}
                </div>
              ) : (
                column.tickets.map(({ task, projection }) => (
                  <DispatchDeskCard
                    key={taskKey(task)}
                    task={task}
                    projection={projection}
                    locked={pendingActionKeys.has(taskKey(task))}
                    onAction={(kind) => {
                      performAction({ kind, task });
                    }}
                    onOpenCodeViewer={onOpenCodeViewer}
                  />
                ))
              )}
            </DispatchDeskColumnView>
          ))
        )}
      </div>
      <DispatchDeskApproveConfirmDialog
        request={approvalRequest}
        onCancel={() => {
          setApprovalRequest(null);
        }}
        onConfirm={(request) => {
          setApprovalRequest(null);
          performAction(request);
        }}
      />
      {/* overlay 是鼠标浮层，挂到 body，避免被看板横向滚动容器的裁剪上下文影响。 */}
      {typeof document === "undefined"
        ? dragOverlayNode
        : createPortal(dragOverlayNode, document.body)}
    </DndContext>
  );
}
