import assert from "node:assert/strict";
import test from "node:test";
import type { ZCodeTaskMeta } from "@zcode/shared";
import {
  DISPATCH_DESK_MAX_ATTEMPTS,
  groupDispatchDeskColumns,
  projectDispatchDeskTicket,
  readDispatchDeskTicketFields,
  type DispatchDeskTicketFields,
} from "../src/workspace-grouped-tasks/kanban-columns.js";

// ============================================================
// 派活台看板列投影的可运行检查（纯函数表驱动，不碰 React）
// ============================================================
// 钉的是谓词与调度器的逐字对表：准入（acceptance_criteria 非空）、
// claim WHERE 的候选分解（评审态优先 / done 未评审 blocked / 退避与封顶 / 在途 / 待派）。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/dispatchDeskKanbanProjection.test.ts

const NOW = 1_700_000_000_000;

function baseFields(overrides: Partial<DispatchDeskTicketFields>): DispatchDeskTicketFields {
  return { hasSpec: true, ...overrides };
}

function project(
  overrides: Partial<DispatchDeskTicketFields>,
  now: number = NOW,
): NonNullable<ReturnType<typeof projectDispatchDeskTicket>> {
  const projection = projectDispatchDeskTicket(baseFields(overrides), now);
  assert.ok(projection, "hasSpec=true 的票必须进看板");
  return projection;
}

test("准入谓词：无 spec（未进台）的票不进看板，无论其它字段处于什么状态", () => {
  const noSpecVariants: Partial<DispatchDeskTicketFields>[] = [
    {},
    { dispatchState: "idle" },
    { dispatchState: "dispatched", taskStatus: "running" },
    { dispatchState: "failed_to_dispatch", dispatchAttempts: 9 },
    { reviewState: "pending_review" },
  ];
  for (const variant of noSpecVariants) {
    assert.equal(
      projectDispatchDeskTicket({ hasSpec: false, ...variant }, NOW),
      null,
      `无 spec 不进看板：${JSON.stringify(variant)}`,
    );
  }
});

test("待派列：idle / 老票缺省，changes_requested 归待派并带返工徽标", () => {
  assert.deepEqual(project({ dispatchState: "idle", reviewState: undefined }), {
    column: "idle",
    rework: false,
    needsHuman: false,
    blockedDone: false,
    retryMinutes: null,
  });
  // 老票：dispatchState 缺省同 idle（列默认值）。
  assert.equal(project({}).column, "idle");
  // changes_requested 是 claim 候选 → 待派列 + 返工徽标（无独立打回列）。
  const rework = project({ dispatchState: "idle", reviewState: "changes_requested" });
  assert.equal(rework.column, "idle");
  assert.equal(rework.rework, true);
  // done 未评审但被人工打回的票（changes_requested）同样是 claim 候选 → 待派 + 返工。
  const doneRework = project({
    taskStatus: "completed",
    dispatchState: "idle",
    reviewState: "changes_requested",
  });
  assert.equal(doneRework.column, "idle");
  assert.equal(doneRework.rework, true);
});

test("干活中：claimed 与 dispatched 都算在途；带 changes_requested 的在途返工也归进行中", () => {
  for (const dispatchState of ["claimed", "dispatched"] as const) {
    assert.equal(project({ dispatchState }).column, "dispatching");
  }
  assert.equal(
    project({ dispatchState: "dispatched", reviewState: "changes_requested" }).column,
    "dispatching",
  );
});

test("待审列：pending_review 优先于派发态（submitForReview 在途窗口也稳）", () => {
  for (const dispatchState of ["idle", "claimed", "dispatched"] as const) {
    assert.equal(
      project({ dispatchState, reviewState: "pending_review" }).column,
      "pendingReview",
    );
  }
});

test("已通过列：approved 是终态，压过任何派发态", () => {
  for (const dispatchState of ["idle", "claimed", "dispatched", "failed_to_dispatch"] as const) {
    assert.equal(project({ dispatchState, reviewState: "approved" }).column, "approved");
  }
});

test("卡住列·blocked 面：done 未评审（completed + reviewState NULL）任何派发态下都落卡住", () => {
  // claim WHERE 的第三个子句不认这类票，dispatchState 是什么都派不出去 → 看板必须落卡住列。
  for (const dispatchState of ["idle", "claimed", "dispatched"] as const) {
    const blocked = project({ taskStatus: "completed", dispatchState });
    assert.equal(blocked.column, "failed");
    assert.equal(blocked.blockedDone, true);
    assert.equal(blocked.retryMinutes, null);
  }
  // 承重分支顺序：completed+NULL 先于 failed_to_dispatch 判定——顺序颠倒会给这张
  // 永远不会被 claim 的票画上不存在的自动重试倒计时。
  const blockedWhileFailed = project({
    taskStatus: "completed",
    dispatchState: "failed_to_dispatch",
    dispatchAttempts: 2,
  });
  assert.equal(blockedWhileFailed.column, "failed");
  assert.equal(blockedWhileFailed.blockedDone, true);
  assert.equal(blockedWhileFailed.needsHuman, false);
  assert.equal(blockedWhileFailed.retryMinutes, null);
});

test("卡住列·退避中：attempts < 封顶显示倒计时；retryAt 缺失或已过期视为 0 分钟", () => {
  const backoff = project({
    dispatchState: "failed_to_dispatch",
    dispatchAttempts: 2,
    retryAt: NOW + 5 * 60_000,
  });
  assert.equal(backoff.column, "failed");
  assert.equal(backoff.needsHuman, false);
  assert.equal(backoff.blockedDone, false);
  assert.equal(backoff.retryMinutes, 5);

  // 向上取整：29 秒也算 1 分钟。
  assert.equal(
    project({ dispatchState: "failed_to_dispatch", dispatchAttempts: 1, retryAt: NOW + 29_000 })
      .retryMinutes,
    1,
  );
  assert.equal(
    project({ dispatchState: "failed_to_dispatch", dispatchAttempts: 1, retryAt: NOW - 60_000 })
      .retryMinutes,
    0,
  );
  assert.equal(
    project({ dispatchState: "failed_to_dispatch", dispatchAttempts: 1, retryAt: undefined })
      .retryMinutes,
    0,
  );
});

test("卡住列·封顶：attempts >= 5 红标需人工，不再显示自动重试倒计时", () => {
  for (const attempts of [DISPATCH_DESK_MAX_ATTEMPTS, DISPATCH_DESK_MAX_ATTEMPTS + 3]) {
    const capped = project({
      dispatchState: "failed_to_dispatch",
      dispatchAttempts: attempts,
      retryAt: NOW + 60_000,
    });
    assert.equal(capped.column, "failed");
    assert.equal(capped.needsHuman, true);
    assert.equal(capped.retryMinutes, null);
  }
  // 封顶线之下不算需人工。
  assert.equal(
    project({ dispatchState: "failed_to_dispatch", dispatchAttempts: DISPATCH_DESK_MAX_ATTEMPTS - 1 })
      .needsHuman,
    false,
  );
});

test("适配器：ZCodeTaskMeta → 原始字段，hasSpec 与 acceptance_criteria IS NOT NULL 同口径", () => {
  const task = {
    taskId: "t1",
    workspacePath: "/w",
    acceptanceCriteria: "构建通过",
    status: "completed",
    dispatchState: "failed_to_dispatch",
    reviewState: "changes_requested",
    dispatchAttempts: 3,
    retryAt: 123,
  } as unknown as ZCodeTaskMeta;
  assert.deepEqual(readDispatchDeskTicketFields(task), {
    hasSpec: true,
    taskStatus: "completed",
    dispatchState: "failed_to_dispatch",
    reviewState: "changes_requested",
    dispatchAttempts: 3,
    retryAt: 123,
  });
  // NULL 列 → undefined → 未进台。
  assert.equal(readDispatchDeskTicketFields({ taskId: "t2", workspacePath: "/w" } as ZCodeTaskMeta).hasSpec, false);
});

test("分组：恒返回五列（空列不消失），无 spec 票被排除，票按谓词归列", () => {
  const mkTask = (patch: Record<string, unknown>): ZCodeTaskMeta =>
    ({ taskId: `t-${Math.random()}`, workspacePath: "/w", ...patch }) as ZCodeTaskMeta;
  const tasks: ZCodeTaskMeta[] = [
    mkTask({ acceptanceCriteria: "a" }), // idle
    mkTask({ acceptanceCriteria: "a", dispatchState: "dispatched" }), // dispatching
    mkTask({ acceptanceCriteria: "a", reviewState: "pending_review" }), // pendingReview
    mkTask({
      acceptanceCriteria: "a",
      dispatchState: "failed_to_dispatch",
      dispatchAttempts: 9,
    }), // failed (needsHuman)
    mkTask({ acceptanceCriteria: "a", reviewState: "approved" }), // approved
    mkTask({ taskId: "t-no-spec" }), // 无 spec：不进任何列
  ];
  const columns = groupDispatchDeskColumns(tasks, NOW);
  assert.deepEqual(
    columns.map((column) => column.id),
    ["idle", "dispatching", "pendingReview", "failed", "approved"],
  );
  assert.deepEqual(
    columns.map((column) => column.tickets.length),
    [1, 1, 1, 1, 1],
  );
  assert.equal(columns[0]?.tickets[0]?.projection.rework, false);
  assert.equal(columns[3]?.tickets[0]?.projection.needsHuman, true);
  // 全空输入仍返回五列空列。
  const empty = groupDispatchDeskColumns([], NOW);
  assert.equal(empty.length, 5);
  assert.ok(empty.every((column) => column.tickets.length === 0));
});
