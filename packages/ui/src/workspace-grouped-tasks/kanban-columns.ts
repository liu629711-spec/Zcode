// 派活台看板列投影：纯函数，不依赖 React。
// 谓词逐字对表 packages/services/src/session/dispatchDeskRepo.ts 的 claimDueTickets
// 候选 WHERE（:355-388）与守卫——UI 不发明状态机，看板与调度器对同一张票的判断必须一致。
import type { ZCodeTaskMeta } from "@zcode/shared";

/** 重试封顶：对齐 dispatchDeskRepo.DESK_DISPATCH_MAX_ATTEMPTS（常量独立一份勿互相引用；
    该模块引 node:sqlite，不能进渲染层）。达限票停在 failed_to_dispatch 等人工接手。 */
export const DISPATCH_DESK_MAX_ATTEMPTS = 5;

/** 看板五列 id（无"打回"列：changes_requested 是 claim 候选，归待派列+返工徽标）。 */
export type DispatchDeskColumnId =
  | "idle"
  | "dispatching"
  | "pendingReview"
  | "failed"
  | "approved";

/** 列固定展示顺序；空列不消失（是片C 的投放目标）。 */
export const DISPATCH_DESK_COLUMN_ORDER: readonly DispatchDeskColumnId[] = [
  "idle",
  "dispatching",
  "pendingReview",
  "failed",
  "approved",
];

/** 投影输入只收原始字段（不绑 ZCodeTaskMeta 整型），测试表驱动直喂。 */
export interface DispatchDeskTicketFields {
  /** 准入谓词：acceptanceCriteria 非空（未进台的票不进看板，否则看板说谎）。 */
  hasSpec: boolean;
  taskStatus?: ZCodeTaskMeta["status"];
  dispatchState?: ZCodeTaskMeta["dispatchState"];
  reviewState?: ZCodeTaskMeta["reviewState"];
  dispatchAttempts?: number;
  retryAt?: number;
}

export interface DispatchDeskTicketProjection {
  column: DispatchDeskColumnId;
  /** 待派列里的 changes_requested 返工徽标（它就是 claim 候选）。 */
  rework: boolean;
  /** 失败封顶（attempts >= 封顶）：红标"需人工"，不再显示自动重试倒计时。 */
  needsHuman: boolean;
  /** blocked 面：done 未评审（completed + reviewState NULL），claim WHERE 不认它。 */
  blockedDone: boolean;
  /** 退避中距下次重派的分钟数（向上取整、钳非负）；仅退避中非 null。 */
  retryMinutes: number | null;
}

/**
 * 逐票投影。判定顺序即状态机事实：
 * 1) 评审态优先——pending_review/approved 由 submitForReview 写入时 dispatch_state 已回 idle，
 *    先看评审态对在途窗口也稳；
 * 2) completed + reviewState NULL 的 done 未评审票落卡住列（claim WHERE 第三个子句不认它，
 *    任何 dispatch_state 下都不会被自动重派，唯一解锁是人工打回）；
 * 3) failed_to_dispatch 按封顶分退避/需人工两态；
 * 4) claimed/dispatched 在途；
 * 5) 其余（idle；老票缺省 undefined 同 idle）归待派。
 */
export function projectDispatchDeskTicket(
  fields: DispatchDeskTicketFields,
  now: number,
): DispatchDeskTicketProjection | null {
  if (!fields.hasSpec) {
    return null;
  }

  const attempts = fields.dispatchAttempts ?? 0;
  if (fields.reviewState === "pending_review") {
    return { column: "pendingReview", rework: false, needsHuman: false, blockedDone: false, retryMinutes: null };
  }
  if (fields.reviewState === "approved") {
    return { column: "approved", rework: false, needsHuman: false, blockedDone: false, retryMinutes: null };
  }
  if (fields.taskStatus === "completed" && fields.reviewState === undefined) {
    return { column: "failed", rework: false, needsHuman: false, blockedDone: true, retryMinutes: null };
  }
  if (fields.dispatchState === "failed_to_dispatch") {
    if (attempts >= DISPATCH_DESK_MAX_ATTEMPTS) {
      return { column: "failed", rework: false, needsHuman: true, blockedDone: false, retryMinutes: null };
    }
    const retryMinutes =
      fields.retryAt === undefined ? 0 : Math.max(0, Math.ceil((fields.retryAt - now) / 60_000));
    return { column: "failed", rework: false, needsHuman: false, blockedDone: false, retryMinutes };
  }
  if (fields.dispatchState === "claimed" || fields.dispatchState === "dispatched") {
    return { column: "dispatching", rework: false, needsHuman: false, blockedDone: false, retryMinutes: null };
  }
  return {
    column: "idle",
    rework: fields.reviewState === "changes_requested",
    needsHuman: false,
    blockedDone: false,
    retryMinutes: null,
  };
}

/** ZCodeTaskMeta → 原始字段适配（与 taskIndexRepo 旁路列投影同口径：NULL 列 → undefined）。 */
export function readDispatchDeskTicketFields(task: ZCodeTaskMeta): DispatchDeskTicketFields {
  return {
    hasSpec: typeof task.acceptanceCriteria === "string" && task.acceptanceCriteria.length > 0,
    taskStatus: task.status,
    dispatchState: task.dispatchState,
    reviewState: task.reviewState,
    dispatchAttempts: task.dispatchAttempts,
    retryAt: task.retryAt,
  };
}

export interface DispatchDeskColumn {
  id: DispatchDeskColumnId;
  tickets: Array<{ task: ZCodeTaskMeta; projection: DispatchDeskTicketProjection }>;
}

/** 全量分组：恒返回五列（含空列），列内保持输入顺序。 */
export function groupDispatchDeskColumns(
  tasks: readonly ZCodeTaskMeta[],
  now: number,
): DispatchDeskColumn[] {
  const columns: DispatchDeskColumn[] = DISPATCH_DESK_COLUMN_ORDER.map((id) => ({ id, tickets: [] }));
  const byId = new Map(columns.map((column) => [column.id, column]));
  for (const task of tasks) {
    const projection = projectDispatchDeskTicket(readDispatchDeskTicketFields(task), now);
    if (!projection) {
      continue;
    }
    byId.get(projection.column)?.tickets.push({ task, projection });
  }
  return columns;
}
