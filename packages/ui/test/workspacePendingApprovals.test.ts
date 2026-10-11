import assert from "node:assert/strict";
import test from "node:test";
import type { ZCodeTaskMeta } from "@zcode/shared";
import { attachTaskListRowActivity } from "../src/v4/taskListRowActivity.js";
import { collectPendingApprovalItems } from "../src/WorkspaceSidebar/pendingApprovalsModel.js";

// 待你拍板（跨任务待批聚合 2026-10-12）聚合纯函数的可运行检查：
// 判定与任务行红点同源（活性计数 / 旧摘要队首单条），跨分组平铺保持新→旧序。

function task(overrides: Partial<ZCodeTaskMeta> = {}): ZCodeTaskMeta {
  return {
    taskId: "t",
    traceId: "trace",
    title: "会话",
    workspacePath: "/w",
    createdAt: 1,
    updatedAt: 2,
    mode: "build",
    ...overrides,
  } as ZCodeTaskMeta;
}

test("聚合：活性计数命中、旧摘要队首兜底、干净行不收", () => {
  const items = collectPendingApprovalItems([
    {
      workspacePath: "/w1",
      items: [
        // 活性 sidecar 计数（实时层）：2 件待批。
        attachTaskListRowActivity(task({ taskId: "a", title: "修登录" }), {
          phase: "running",
          lastActivityAt: 3,
          hasBackgroundWork: false,
          pendingInteractions: { permissionCount: 1, userInputCount: 1 },
        }),
        // 干净行：不收。
        attachTaskListRowActivity(task({ taskId: "b", title: "闲聊" }), {
          phase: "completedSuccess",
          lastActivityAt: 3,
          hasBackgroundWork: false,
        }),
        // 旧摘要只有队首单条（无计数 sidecar）：有 pendingInteraction 也算在等。
        task({ taskId: "c", title: "老会话", pendingInteraction: {
          interactionId: "i-1",
          kind: "permission",
        } }),
      ],
    },
    {
      workspacePath: "/w2",
      items: [
        attachTaskListRowActivity(task({ taskId: "d", title: "另一个项目的单" }), {
          phase: "running",
          lastActivityAt: 4,
          hasBackgroundWork: false,
          pendingInteractions: { permissionCount: 0, userInputCount: 3 },
        }),
      ],
    },
  ]);

  assert.deepEqual(
    items.map((item) => item.task.taskId),
    ["a", "c", "d"],
    "按分组内新→旧平铺，干净行不出现",
  );
  assert.equal(items[0]?.count, 2);
  assert.equal(items[1]?.count, 1, "旧摘要缺计数回落 1");
  assert.equal(items[2]?.count, 3);
  // 跳转凭据随行：workspacePath/identity 原样带去 onSelectTask。
  assert.equal(items[2]?.workspacePath, "/w2");
});

test("聚合：全干净时返回空数组（分组整体不渲染）", () => {
  const items = collectPendingApprovalItems([
    {
      workspacePath: "/w",
      items: [
        attachTaskListRowActivity(task(), {
          phase: "running",
          lastActivityAt: 3,
          hasBackgroundWork: false,
        }),
      ],
    },
  ]);
  assert.deepEqual(items, []);
});
