import assert from "node:assert/strict";
import test from "node:test";
import {
  DISPATCH_DESK_COLUMN_ORDER,
  projectDispatchDeskTicket,
  type DispatchDeskColumnId,
  type DispatchDeskTicketFields,
} from "../src/workspace-grouped-tasks/kanban-columns.js";
import {
  canDispatchDeskTicketDrag,
  doesDispatchDeskDragRequireConfirm,
  getDispatchDeskCardActions,
  isDispatchDeskDragLegal,
} from "../src/workspace-grouped-tasks/kanban-transitions.js";

// ============================================================
// 派活台人工闸机的可运行检查：转移合法性与确认门控（纯函数表驱动，不碰 React）
// ============================================================
// 钉的是封闭转移表：两条合法拖拽链（待审→已通过、卡住→待派），其余 from×to 全矩阵非法；
// 批准必须经确认门控；approved 无 handle，blockedDone 无 handle（唯一出路是卡上按钮）。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/dispatchDeskKanbanTransitions.test.ts

const NOW = 1_700_000_000_000;

/** 封闭合法表（与规约逐字对表，测试侧独立于实现维护）。 */
const LEGAL_DRAGS: ReadonlyArray<readonly [DispatchDeskColumnId, DispatchDeskColumnId]> = [
  ["pendingReview", "approved"],
  ["failed", "idle"],
];

function projectionOf(overrides: Partial<DispatchDeskTicketFields>) {
  const projection = projectDispatchDeskTicket({ hasSpec: true, ...overrides }, NOW);
  assert.ok(projection, "表驱动输入必须产出投影");
  return projection;
}

test("转移合法性：from×to 全矩阵只有两条合法链，其余全非法", () => {
  for (const from of DISPATCH_DESK_COLUMN_ORDER) {
    for (const to of DISPATCH_DESK_COLUMN_ORDER) {
      const expected = LEGAL_DRAGS.some(
        ([legalFrom, legalTo]) => legalFrom === from && legalTo === to,
      );
      assert.equal(
        isDispatchDeskDragLegal(from, to),
        expected,
        `${from} → ${to} 应为 ${expected ? "合法" : "非法"}`,
      );
    }
  }
});

test("确认门控：待审→已通过必须经确认才发起，其余转移免确认直达", () => {
  for (const from of DISPATCH_DESK_COLUMN_ORDER) {
    for (const to of DISPATCH_DESK_COLUMN_ORDER) {
      const expected = from === "pendingReview" && to === "approved";
      assert.equal(
        doesDispatchDeskDragRequireConfirm(from, to),
        expected,
        `${from} → ${to} 确认门控应为 ${expected}`,
      );
    }
  }
});

test("拖拽 handle 准入：approved 终态无 handle；blockedDone 无 handle；退避/封顶/待审可拖", () => {
  assert.equal(
    canDispatchDeskTicketDrag(projectionOf({ reviewState: "approved" })),
    false,
    "approved 终态不可拖（不可逆，无去向）",
  );
  assert.equal(
    canDispatchDeskTicketDrag(projectionOf({ taskStatus: "completed", dispatchState: "idle" })),
    false,
    "blockedDone（done 未评审）无合法去向，不给 handle（唯一出路是卡上打回按钮）",
  );
  assert.equal(
    canDispatchDeskTicketDrag(projectionOf({ dispatchState: "idle" })),
    false,
    "待派卡无合法去向，不给 handle",
  );
  assert.equal(
    canDispatchDeskTicketDrag(projectionOf({ dispatchState: "dispatched" })),
    false,
    "在途卡无合法去向，不给 handle",
  );
  assert.equal(
    canDispatchDeskTicketDrag(projectionOf({ reviewState: "pending_review" })),
    true,
    "待审卡可拖向已通过列（经确认）",
  );
  assert.equal(
    canDispatchDeskTicketDrag(
      projectionOf({
        dispatchState: "failed_to_dispatch",
        dispatchAttempts: 2,
        retryAt: NOW + 60_000,
      }),
    ),
    true,
    "退避中的卡住票可拖向待派列（等价 manualRequeue）",
  );
  assert.equal(
    canDispatchDeskTicketDrag(
      projectionOf({ dispatchState: "failed_to_dispatch", dispatchAttempts: 9 }),
    ),
    true,
    "封顶卡住票可拖向待派列",
  );
});

test("卡上动作：待审=打回；退避/封顶=人工接手重派；blockedDone 只有打回（不给批准入口）", () => {
  assert.deepEqual(getDispatchDeskCardActions(projectionOf({ reviewState: "pending_review" })), [
    "requestChanges",
  ]);
  assert.deepEqual(
    getDispatchDeskCardActions(
      projectionOf({
        dispatchState: "failed_to_dispatch",
        dispatchAttempts: 2,
        retryAt: NOW + 60_000,
      }),
    ),
    ["requeue"],
  );
  assert.deepEqual(
    getDispatchDeskCardActions(
      projectionOf({ dispatchState: "failed_to_dispatch", dispatchAttempts: 9 }),
    ),
    ["requeue"],
  );
  assert.deepEqual(
    getDispatchDeskCardActions(projectionOf({ taskStatus: "completed", dispatchState: "idle" })),
    ["requestChanges"],
    "blockedDone 只有打回返工；批准按钮不给（守卫本就拒绝，UI 不给入口）",
  );
  for (const fields of [
    { dispatchState: "idle" },
    { dispatchState: "idle", reviewState: "changes_requested" },
    { dispatchState: "dispatched" },
    { reviewState: "approved" },
  ] as const) {
    assert.deepEqual(
      getDispatchDeskCardActions(projectionOf(fields)),
      [],
      `${JSON.stringify(fields)} 无卡上动作`,
    );
  }
});
