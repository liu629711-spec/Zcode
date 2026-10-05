// ============================================================
// 团队看板纯规则（团队看板批 2026-10-05）的可运行检查：
//  1. DAG 分层：依赖经 taskKey 解析，depth = 1 + max(前置)；环不炸；
//  2. teamHasDependencies 决定画 DAG 还是平铺；
//  3. 进度聚合只认 completed。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/teamBoardModel.test.ts
// ============================================================

import assert from "node:assert/strict";
import test from "node:test";
import {
  computeDagDepths,
  teamHasDependencies,
  teamProgress,
} from "../src/app-shell/teamBoardModel.js";

test("DAG 分层：依赖经 taskKey 解析，链式任务逐层加深", () => {
  const depths = computeDagDepths([
    { workOrderId: "wo-1", taskKey: "req", dependsOn: undefined },
    { workOrderId: "wo-2", taskKey: "impl", dependsOn: ["req"] },
    { workOrderId: "wo-3", taskKey: "review", dependsOn: ["impl"] },
  ]);
  assert.equal(depths.get("wo-1"), 0);
  assert.equal(depths.get("wo-2"), 1);
  assert.equal(depths.get("wo-3"), 2);
});

test("DAG 分层：环与悬空 key 稳定收敛，不炸（环无真解，同输入恒同输出）", () => {
  const orders = [
    { workOrderId: "wo-1", taskKey: "a", dependsOn: ["b"] },
    { workOrderId: "wo-2", taskKey: "b", dependsOn: ["a"] },
    { workOrderId: "wo-3", taskKey: "c", dependsOn: ["不存在的key"] },
  ];
  const depths = computeDagDepths(orders);
  for (const order of orders) {
    const depth = depths.get(order.workOrderId);
    assert.equal(typeof depth, "number");
    assert.ok(depth >= 0 && depth < orders.length);
  }
  const again = computeDagDepths(orders);
  assert.deepEqual([...depths.entries()], [...again.entries()]);
});

test("平铺判定与进度聚合", () => {
  assert.equal(
    teamHasDependencies([
      { workOrderId: "wo-1", dependsOn: undefined },
      { workOrderId: "wo-2", dependsOn: [] },
    ]),
    false,
  );
  assert.equal(teamHasDependencies([{ workOrderId: "wo-1", dependsOn: ["x"] }]), true);
  assert.deepEqual(
    teamProgress({
      orders: [
        { workOrderId: "1", status: "completed" },
        { workOrderId: "2", status: "failed" },
        { workOrderId: "3", status: "in_flight" },
      ],
    } as never),
    { completed: 1, total: 3 },
  );
});
