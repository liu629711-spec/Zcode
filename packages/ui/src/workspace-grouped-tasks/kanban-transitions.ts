// 派活台人工闸机：转移合法性与确认门控的纯函数表（不依赖 React）。
// 本表是 UI 侧"给不给入口"的镜像，权威守卫在服务端两个通道方法——
// packages/services/src/session/dispatchDeskRepo.ts 的 recordReviewDecision/manualRequeue
// UPDATE WHERE；守卫拒绝返回 null=竞态落空，UI 必须提示+刷新，绝不静默。
import type { ZCodeTaskMeta } from "@zcode/shared";
import type {
  DispatchDeskColumnId,
  DispatchDeskTicketProjection,
} from "@/workspace-grouped-tasks/kanban-columns.js";

/** 人工动作种别：approve/requestChanges 走 recordReviewDecision，requeue 走 manualRequeue。 */
export type DispatchDeskActionKind = "approve" | "requestChanges" | "requeue";

/**
 * 人工可写转移表：合法拖拽只有两条——待审→已通过（批准，经确认框）、
 * 卡住→待派（人工接手重派，落地等价 manualRequeue）。其余一切转移非法：
 * 目标列 droppable 置 disabled（isOver 不点亮），落地沉默拒绝。
 */
export function isDispatchDeskDragLegal(
  from: DispatchDeskColumnId,
  to: DispatchDeskColumnId,
): boolean {
  return (from === "pendingReview" && to === "approved") || (from === "failed" && to === "idle");
}

/** 批准即终态不可逆：待审→已通过必须经确认框才发起，其余合法转移免确认直达。 */
export function doesDispatchDeskDragRequireConfirm(
  from: DispatchDeskColumnId,
  to: DispatchDeskColumnId,
): boolean {
  return from === "pendingReview" && to === "approved";
}

/**
 * 卡上动作（不拖，按钮直达）：待审卡给"打回返工"；卡住列退避/封顶票给"人工接手重派"；
 * blockedDone（done 未评审）只有"打回返工"——这是 changes_requested 的特批路径，
 * 不给批准入口（守卫本就拒绝，UI 不给入口）。
 */
export function getDispatchDeskCardActions(
  projection: DispatchDeskTicketProjection,
): DispatchDeskActionKind[] {
  if (projection.column === "pendingReview") {
    return ["requestChanges"];
  }
  if (projection.column === "failed") {
    return projection.blockedDone ? ["requestChanges"] : ["requeue"];
  }
  return [];
}

/**
 * 拖拽 handle 准入：approved 终态不可拖（不可逆，无去向）；blockedDone 无合法去向
 * （唯一出路是卡上按钮）；无合法去向的待派/在途卡同样不给 handle。
 */
export function canDispatchDeskTicketDrag(projection: DispatchDeskTicketProjection): boolean {
  return (
    projection.column === "pendingReview" ||
    (projection.column === "failed" && !projection.blockedDone)
  );
}

// 列模式独立的 dnd 数据面：与列表模式行重排（grouped-task/group 类型）互不识别。
// 两套 DndContext 随 board/list 二选一挂载，类型互异是第二条防线，保证列表模式零改动。
export const DISPATCH_DESK_COLUMN_DROPPABLE_TYPE = "dispatch-desk-column";

/** 列 droppable 数据：onDragEnd 以 type 判别落点，杜绝列表模式行重排数据串台。 */
export interface DispatchDeskColumnDropData {
  type: typeof DISPATCH_DESK_COLUMN_DROPPABLE_TYPE;
  columnId: DispatchDeskColumnId;
}

/** 卡片拖拽数据：columnId 定起点列，task 供服务调用，projection 供 overlay 原样重绘。 */
export interface DispatchDeskCardDragData {
  type: "dispatch-desk-card";
  columnId: DispatchDeskColumnId;
  task: ZCodeTaskMeta;
  projection: DispatchDeskTicketProjection;
}

/** 一次人工动作请求（确认框与按钮共用）。 */
export interface DispatchDeskActionRequest {
  kind: DispatchDeskActionKind;
  task: ZCodeTaskMeta;
}
