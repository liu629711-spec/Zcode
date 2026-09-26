import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import test from "node:test";
import { TaskIndexRepo } from "../src/session/taskIndexRepo.js";

// 片A 数据投影：tasks 表派活台旁路列（migration 0004，写侧 DispatchDeskRepo）
// 必须经 TaskIndexRow → rowToMeta 进入 UI 任务列表数据链；本文件只测读侧投影。
const require = createRequire(import.meta.url);
const { DatabaseSync } = require("node:sqlite") as typeof import("node:sqlite");

const WS = { workspacePath: "/w" };

interface Ctx {
  repo: TaskIndexRepo;
  raw: DatabaseSync;
  cleanup: () => Promise<void>;
}

async function setup(): Promise<Ctx> {
  const dir = await mkdtemp(join(tmpdir(), "zcode-task-index-projection-"));
  const repo = new TaskIndexRepo(join(dir, "tasks-index.sqlite"));
  await repo.ensureReady();
  const raw = new DatabaseSync(join(dir, "tasks-index.sqlite"));
  return {
    repo,
    raw,
    cleanup: async () => {
      raw.close();
      repo.close();
      // Windows：GC 未及回收的 StatementSync 句柄可能短暂锁住 WAL 文件；
      // 残留目录交给系统临时目录清理，不为测试便利引入全局 GC。
      await rm(dir, { recursive: true, force: true }).catch(() => undefined);
    },
  };
}

async function seedTask(repo: TaskIndexRepo, taskId: string): Promise<void> {
  await repo.syncTaskMeta({
    meta: {
      taskId,
      traceId: `trace-${taskId}`,
      title: `票 ${taskId}`,
      workspacePath: WS.workspacePath,
      createdAt: 1000,
      updatedAt: 2000,
      mode: "build",
    },
  });
}

test("dispatch desk bypass columns project through getTaskRow and list queries", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "desk-1");
    // 模拟 DispatchDeskRepo 状态机写侧：一张派发失败、待人工评审的票。
    ctx.raw
      .prepare(
        `UPDATE tasks SET dispatch_state='failed_to_dispatch', review_state='pending_review',
        acceptance_criteria='WHEN a THEN b', deliverables=?, claimed_at=1234,
        dispatch_attempts=2, retry_at=3456, last_dispatch_error='host unavailable'
        WHERE task_id='desk-1'`,
      )
      .run(JSON.stringify([{ path: "src/a.ts", bytes: 3 }]));

    // getTaskRow 路径（getTaskMeta）。
    const single = await ctx.repo.getTaskMeta({ ...WS, taskId: "desk-1" });
    assert.ok(single);
    assert.equal(single.dispatchState, "failed_to_dispatch");
    assert.equal(single.reviewState, "pending_review");
    assert.equal(single.acceptanceCriteria, "WHEN a THEN b");
    assert.deepEqual(single.deliverables, [{ path: "src/a.ts", bytes: 3 }]);
    assert.equal(single.claimedAt, 1234);
    assert.equal(single.dispatchAttempts, 2);
    assert.equal(single.retryAt, 3456);
    assert.equal(single.lastDispatchError, "host unavailable");

    // 列表主查询路径（queryTaskList）。
    const listed = await ctx.repo.queryTaskList({
      kind: "active",
      workspaceScopes: [WS],
      sortBy: "updated",
    });
    const item = listed.items.find((t) => t.taskId === "desk-1");
    assert.ok(item, "列表应包含带派活状态的票");
    assert.equal(item.dispatchState, "failed_to_dispatch");
    assert.equal(item.reviewState, "pending_review");
    assert.equal(item.acceptanceCriteria, "WHEN a THEN b");
    assert.deepEqual(item.deliverables, [{ path: "src/a.ts", bytes: 3 }]);
    assert.equal(item.claimedAt, 1234);
    assert.equal(item.dispatchAttempts, 2);
    assert.equal(item.retryAt, 3456);
    assert.equal(item.lastDispatchError, "host unavailable");
  } finally {
    await ctx.cleanup();
  }
});

test("legacy tickets without dispatch activity read safe defaults from bypass columns", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "plain-old");
    const meta = await ctx.repo.getTaskMeta({ ...WS, taskId: "plain-old" });
    assert.ok(meta);
    // 老票：dispatch_state/dispatch_attempts 走 NOT NULL 默认值，其余列 NULL。
    assert.equal(meta.dispatchState, "idle");
    assert.equal(meta.dispatchAttempts, 0);
    assert.equal(meta.reviewState, undefined);
    assert.equal(meta.acceptanceCriteria, undefined);
    assert.equal(meta.deliverables, undefined);
    assert.equal(meta.claimedAt, undefined);
    assert.equal(meta.retryAt, undefined);
    assert.equal(meta.lastDispatchError, undefined);
  } finally {
    await ctx.cleanup();
  }
});

test("corrupt deliverables column reads as undefined instead of throwing", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "bad-json");
    ctx.raw
      .prepare(`UPDATE tasks SET deliverables='{oops' WHERE task_id='bad-json'`)
      .run();
    const meta = await ctx.repo.getTaskMeta({ ...WS, taskId: "bad-json" });
    assert.ok(meta);
    assert.equal(meta.deliverables, undefined);
    // 坏 JSON 不影响同行的其余旁路列投影。
    assert.equal(meta.dispatchState, "idle");
    assert.equal(meta.acceptanceCriteria, undefined);
  } finally {
    await ctx.cleanup();
  }
});

test("invalid meta_json falls back to row projection without throwing, bypass columns still projected", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "broken-meta");
    ctx.raw
      .prepare(
        `UPDATE tasks SET meta_json='{broken', dispatch_state='dispatched', claimed_at=99
        WHERE task_id='broken-meta'`,
      )
      .run();
    const meta = await ctx.repo.getTaskMeta({ ...WS, taskId: "broken-meta" });
    assert.ok(meta, "meta_json 损坏时回退行投影，不能抛");
    assert.equal(meta.taskId, "broken-meta");
    assert.equal(meta.title, "票 broken-meta");
    assert.equal(meta.dispatchState, "dispatched");
    assert.equal(meta.claimedAt, 99);
    assert.equal(meta.reviewState, undefined);
    assert.equal(meta.dispatchAttempts, 0);
  } finally {
    await ctx.cleanup();
  }
});
