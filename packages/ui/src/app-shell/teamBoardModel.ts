// 团队看板纯规则（团队看板批 2026-10-05）：DAG 分层与进度聚合。
// 快照 schema（@zcode/shared/zcode-protocol-v4 team-board.ts）是唯一数据契约；
// 这里不含 React，面板只认算好的几何与数字——与工地卡纯函数同一纪律。

import type { TeamBoardOrder, TeamBoardTeam } from "@zcode/shared/zcode-protocol-v4";

/**
 * DAG 分层深度：depth(单) = 1 + max(前置 depth)，无前置 = 0。
 * 依赖边经 taskKey 解析（前置=同批中 taskKey 匹配的工单）；环与悬空 key 按
 * 叶子处理（画得出图，不炸）——批2 的调度器才负责拒坏依赖，看板只负责画。
 */
export function computeDagDepths(
  orders: readonly Pick<TeamBoardOrder, "workOrderId" | "taskKey" | "dependsOn">[],
): Map<string, number> {
  const byId = new Map(orders.map((order) => [order.workOrderId, order]));
  const byKey = new Map<string, string>();
  for (const order of orders) {
    if (order.taskKey && !byKey.has(order.taskKey)) byKey.set(order.taskKey, order.workOrderId);
  }
  const depths = new Map<string, number>();
  const visiting = new Set<string>();
  const depth = (workOrderId: string): number => {
    const memoed = depths.get(workOrderId);
    if (memoed !== undefined) return memoed;
    if (visiting.has(workOrderId)) return 0;
    visiting.add(workOrderId);
    const order = byId.get(workOrderId);
    let value = 0;
    for (const key of order?.dependsOn ?? []) {
      const depId = byKey.get(key);
      if (depId !== undefined && depId !== workOrderId) {
        value = Math.max(value, depth(depId) + 1);
      }
    }
    visiting.delete(workOrderId);
    depths.set(workOrderId, value);
    return value;
  };
  for (const order of orders) depth(order.workOrderId);
  return depths;
}

/** 本团队是否有任何显式依赖（决定画 DAG 还是平铺清单）。 */
export function teamHasDependencies(
  orders: readonly Pick<TeamBoardOrder, "dependsOn">[],
): boolean {
  return orders.some((order) => (order.dependsOn?.length ?? 0) > 0);
}

/** 团队进度：completed 终态计数（cancelled/failed 不算完成也不算重来——老板看得见行）。 */
export function teamProgress(team: Pick<TeamBoardTeam, "orders">): { completed: number; total: number } {
  return {
    completed: team.orders.filter((order) => order.status === "completed").length,
    total: team.orders.length,
  };
}
