import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import test from "node:test";
import { computeDispatchRetryAt, DispatchDeskRepo } from "../src/session/dispatchDeskRepo.js";

const require = createRequire(import.meta.url);
const { DatabaseSync } = require("node:sqlite") as typeof import("node:sqlite");

const WS = { workspacePath: "/w" };
const STALE_MS = 10 * 60_000;

interface Ctx {
  dbPath: string;
  raw: DatabaseSync;
  cleanup: () => Promise<void>;
}

async function setup(): Promise<Ctx> {
  const dir = await mkdtemp(join(tmpdir(), "zcode-dispatch-desk-"));
  const dbPath = join(dir, "tasks-index.sqlite");
  const bootstrap = new DispatchDeskRepo(dbPath);
  await bootstrap.ensureReady();
  bootstrap.close();
  const raw = new DatabaseSync(dbPath);
  return {
    dbPath,
    raw,
    cleanup: async () => {
      raw.close();
      // Windows：GC 未及回收的 StatementSync 句柄可能短暂锁住 WAL 文件；
      // 残留目录交给系统临时目录清理，不为测试便利引入全局 GC。
      await rm(dir, { recursive: true, force: true }).catch(() => undefined);
    },
  };
}

function seedTicket(
  raw: DatabaseSync,
  params: {
    workspacePath: string;
    taskId: string;
    title?: string;
    taskStatus?: "running" | "completed" | "error";
    acceptanceCriteria?: string | null;
    createdAt?: number;
  },
): void {
  raw
    .prepare(
      `INSERT INTO tasks (workspace_key, workspace_path, task_id, title, task_status, mode,
        created_at, updated_at, acceptance_criteria)
      VALUES (?, ?, ?, ?, ?, 'build', ?, ?, ?)`,
    )
    .run(
      params.workspacePath,
      params.workspacePath,
      params.taskId,
      params.title ?? params.taskId,
      params.taskStatus ?? null,
      params.createdAt ?? 1000,
      params.createdAt ?? 1000,
      params.acceptanceCriteria ?? null,
    );
}

function rawTicket(
  raw: DatabaseSync,
  taskId: string,
): {
  dispatch_state: string;
  review_state: string | null;
  claimed_at: number | null;
  dispatch_attempts: number;
  retry_at: number | null;
} {
  return raw
    .prepare(
      `SELECT dispatch_state, review_state, claimed_at, dispatch_attempts, retry_at
      FROM tasks WHERE task_id = ?`,
    )
    .get(taskId) as never;
}

test("migration 0004 adds dispatch desk bypass columns with CHECK and safe defaults", async () => {
  const ctx = await setup();
  try {
    const columns = ctx.raw.prepare(`PRAGMA table_info(tasks)`).all() as Array<{
      name: string;
      type: string;
      notnull: number;
      dflt_value: string | null;
    }>;
    const byName = new Map(columns.map((c) => [c.name, c]));
    assert.equal(byName.get("review_state")?.type, "TEXT");
    assert.equal(byName.get("review_state")?.notnull, 0);
    assert.equal(byName.get("dispatch_state")?.notnull, 1);
    assert.equal(byName.get("dispatch_state")?.dflt_value, "'idle'");
    assert.equal(byName.get("dispatch_attempts")?.notnull, 1);
    assert.equal(byName.get("acceptance_criteria")?.type, "TEXT");
    assert.equal(byName.get("deliverables")?.type, "TEXT");

    // CHECK 兜底：越界值在 SQL 层被拒。
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "t-check" });
    assert.throws(() =>
      ctx.raw.exec(`UPDATE tasks SET review_state='approved!' WHERE task_id='t-check'`),
    );
    assert.throws(() =>
      ctx.raw.exec(`UPDATE tasks SET dispatch_state='queued' WHERE task_id='t-check'`),
    );

    // 账本登记 0004；二次初始化幂等。
    assert.ok(ctx.raw.prepare(`SELECT 1 FROM tasks_schema_migration WHERE id='0004_dispatch_desk'`).get());
    const second = new DispatchDeskRepo(ctx.dbPath);
    await second.ensureReady();
    second.close();
    const executed = ctx.raw
      .prepare(`SELECT count(*) AS n FROM tasks_schema_migration`)
      .get() as { n: number };
    assert.equal(executed.n, 4);
  } finally {
    await ctx.cleanup();
  }
});

test("migration upgrade path: rows written before 0004 gain safe defaults, data preserved", async () => {
  const ctx = await setup();
  try {
    // 模拟旧版本库：已有任务行（旧世界没有这些旁路列，也就没有 criteria）+ 回滚 0004。
    seedTicket(ctx.raw, {
      workspacePath: "/w",
      taskId: "legacy-completed",
      title: "旧任务",
      taskStatus: "completed",
    });
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "legacy-fresh" });
    ctx.raw.exec("DROP INDEX IF EXISTS idx_tasks_dispatch_due");
    for (const column of [
      "review_state",
      "acceptance_criteria",
      "deliverables",
      "dispatch_state",
      "claimed_at",
      "dispatch_attempts",
      "retry_at",
      "last_dispatch_error",
    ]) {
      ctx.raw.exec(`ALTER TABLE tasks DROP COLUMN ${column}`);
    }
    ctx.raw.prepare(`DELETE FROM tasks_schema_migration WHERE id='0004_dispatch_desk'`).run();

    const repo = new DispatchDeskRepo(ctx.dbPath);
    await repo.ensureReady();
    const legacy = await repo.getTicket({ ...WS, taskId: "legacy-completed" });
    assert.ok(legacy);
    assert.equal(legacy.dispatchState, "idle");
    assert.equal(legacy.reviewState, null);
    assert.equal(legacy.dispatchAttempts, 0);
    assert.equal(legacy.title, "旧任务");
    const fresh = await repo.getTicket({ ...WS, taskId: "legacy-fresh" });
    assert.equal(fresh?.acceptanceCriteria, null);
    assert.equal(fresh?.dispatchState, "idle");
    repo.close();
  } finally {
    await ctx.cleanup();
  }
});

test("two clients racing the same ticket: exactly one claim wins", async () => {
  const ctx = await setup();
  try {
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "race-1", acceptanceCriteria: "done" });
    const clientA = new DispatchDeskRepo(ctx.dbPath);
    const clientB = new DispatchDeskRepo(ctx.dbPath);
    await clientA.ensureReady();
    await clientB.ensureReady();

    const [a, b] = await Promise.all([
      clientA.claimDueTickets({ now: 5000, limit: 1 }),
      clientB.claimDueTickets({ now: 5000, limit: 1 }),
    ]);
    assert.equal(a.length + b.length, 1, "同票两次 claim 只成功一次");
    const winner = a.length === 1 ? a : b;
    assert.equal(winner[0].dispatchState, "claimed");
    assert.equal(rawTicket(ctx.raw, "race-1").dispatch_state, "claimed");

    // 落败方立刻再抢同一张票也拿不到（guard+changes 幂等）。
    const again = await (a.length === 1 ? clientB : clientA).claimDueTickets({
      now: 5001,
      limit: 1,
    });
    assert.deepEqual(again, []);
    clientA.close();
    clientB.close();
  } finally {
    await ctx.cleanup();
  }
});

test("cross-connection guard: uncommitted claim transaction blocks the other client (SQLITE_BUSY), no dirty claim", async () => {
  const ctx = await setup();
  try {
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "lock-1", acceptanceCriteria: "done" });
    // 竞争方先完成初始化，避免其迁移事务先撞上持锁方。
    const contender = new DispatchDeskRepo(ctx.dbPath, 25);
    await contender.ensureReady();
    // 持锁方：模拟一个未提交的写事务占用 tasks-index。
    ctx.raw.exec("BEGIN IMMEDIATE");

    await assert.rejects(
      contender.claimDueTickets({ now: 5000, limit: 1 }),
      (error: Error & { errcode?: number }) =>
        error.errcode === 5 || /locked|busy/i.test(error.message),
    );
    // 持锁方提交后，竞争方正常认领，且没有任何一方产生双认领。
    ctx.raw.exec("COMMIT");
    const claims = await contender.claimDueTickets({ now: 5001, limit: 1 });
    assert.equal(claims.length, 1);
    assert.equal(claims[0].taskId, "lock-1");
    contender.close();
  } finally {
    await ctx.cleanup();
  }
});

test("claim honors limit, workspace filter, and per-workspace single-flight", async () => {
  const ctx = await setup();
  try {
    for (const taskId of ["w1-a", "w1-b", "w1-c"]) {
      seedTicket(ctx.raw, { workspacePath: "/w1", taskId, acceptanceCriteria: "done" });
    }
    seedTicket(ctx.raw, { workspacePath: "/w2", taskId: "w2-a", acceptanceCriteria: "done" });
    const repo = new DispatchDeskRepo(ctx.dbPath);
    await repo.ensureReady();

    // 每 tick 认领至多 N 张（limit=2，按 created_at/task_id FIFO）。
    const first = await repo.claimDueTickets({ now: 1000, limit: 2 });
    assert.deepEqual(
      first.map((t) => t.taskId).sort(),
      ["w1-a", "w1-b"],
    );
    // 单飞：w1 已有 claimed 在途，w1 剩余票整队跳过；w2 不受影响。
    const second = await repo.claimDueTickets({ now: 2000, limit: 10 });
    assert.deepEqual(second.map((t) => t.taskId), ["w2-a"]);
    // workspace 过滤：只看 w1 → 仍被单飞挡住。
    const filtered = await repo.claimDueTickets({
      now: 3000,
      limit: 10,
      workspacePath: "/w1",
    });
    assert.deepEqual(filtered, []);
    await assert.rejects(repo.claimDueTickets({ now: 1, limit: 0 }), /limit/);
    repo.close();
  } finally {
    await ctx.cleanup();
  }
});

test("dispatch handoff uses claim token; backoff and zombie reclaim work", async () => {
  const ctx = await setup();
  try {
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "handoff", acceptanceCriteria: "done" });
    const repo = new DispatchDeskRepo(ctx.dbPath);
    await repo.ensureReady();
    const [claimed] = await repo.claimDueTickets({ now: 1000, limit: 1 });

    // 凭据不符（他人/旧一轮认领）不能转移状态。
    assert.equal(
      await repo.markDispatched({ ...WS, taskId: "handoff" }, { claimedAt: 999, now: 1100 }),
      false,
    );

    // 派发失败 → 退避：累计尝试、retry_at 到期前不可认领、到期后重新可派。
    assert.equal(
      await repo.markDispatchFailed(
        { ...WS, taskId: "handoff" },
        { claimedAt: claimed.claimedAt!, now: 1200, error: "host unavailable" },
      ),
      true,
    );
    const failed = await repo.getTicket({ ...WS, taskId: "handoff" });
    assert.equal(failed?.dispatchState, "failed_to_dispatch");
    assert.equal(failed?.dispatchAttempts, 1);
    assert.equal(failed?.retryAt, computeDispatchRetryAt(1200, 1));
    assert.equal(failed?.lastDispatchError, "host unavailable");
    assert.deepEqual(await repo.claimDueTickets({ now: 1200 + 29_999, limit: 5 }), []);
    const retried = await repo.claimDueTickets({ now: 1200 + 30_001, limit: 5 });
    assert.deepEqual(retried.map((t) => t.taskId), ["handoff"]);

    // 重派后凭据匹配才能 claimed→dispatched。
    assert.equal(
      await repo.markDispatched(
        { ...WS, taskId: "handoff" },
        { claimedAt: retried[0].claimedAt!, now: 31002 },
      ),
      true,
    );
    const dispatched = await repo.getTicket({ ...WS, taskId: "handoff" });
    assert.equal(dispatched?.dispatchState, "dispatched");
    assert.equal(dispatched?.claimedAt, retried[0].claimedAt);

    // 僵尸回收：claimed 超时未结算 → 回收并当场重新认领。
    ctx.raw
      .prepare(`UPDATE tasks SET dispatch_state='claimed', claimed_at=? WHERE task_id='handoff'`)
      .run(99);
    const zombies = await repo.claimDueTickets({ now: 100 + STALE_MS, limit: 5 });
    assert.deepEqual(zombies.map((t) => t.taskId), ["handoff"]);
    const after = rawTicket(ctx.raw, "handoff");
    assert.equal(after.dispatch_state, "claimed");
    assert.equal(after.claimed_at, 100 + STALE_MS);
    repo.close();
  } finally {
    await ctx.cleanup();
  }
});

test("review gate: agent path can only submit for review, never approve", async () => {
  const ctx = await setup();
  try {
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "gate-1", acceptanceCriteria: "done" });
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "gate-2", acceptanceCriteria: "done" });
    const repo = new DispatchDeskRepo(ctx.dbPath);
    await repo.ensureReady();
    const [ticket] = await repo.claimDueTickets({ now: 1000, limit: 1 });
    assert.equal(ticket.taskId, "gate-1");
    await repo.markDispatched(
      { ...WS, taskId: ticket.taskId },
      { claimedAt: ticket.claimedAt!, now: 1100 },
    );

    // 代理交活：pending_review + deliverables 持久化 + 释放单飞槽。
    const submitted = await repo.submitForReview({ ...WS, taskId: ticket.taskId }, {
      now: 1200,
      deliverables: [{ path: "src/a.ts", bytes: 3 }],
    });
    assert.equal(submitted?.reviewState, "pending_review");
    assert.equal(submitted?.dispatchState, "idle");
    assert.deepEqual(submitted?.deliverables, [{ path: "src/a.ts", bytes: 3 }]);

    // 代理路径没有决策参数；重复交活被守卫拒绝且不改状态。
    assert.equal(
      await repo.submitForReview({ ...WS, taskId: ticket.taskId }, { now: 1300 }),
      null,
    );
    assert.equal(rawTicket(ctx.raw, ticket.taskId).review_state, "pending_review");
    // spec 编辑不触碰评审态——代理可调用的全部入口走一遍也写不出 approved。
    await repo.setTicketSpec(
      { ...WS, taskId: ticket.taskId },
      { acceptanceCriteria: "改过的标准", now: 1350 },
    );
    assert.equal(rawTicket(ctx.raw, ticket.taskId).review_state, "pending_review");

    // pending_review 的票不可再派；另一张空闲票正常认领。
    const next = await repo.claimDueTickets({ now: 1400, limit: 10 });
    assert.deepEqual(next.map((t) => t.taskId), ["gate-2"]);
    await repo.markDispatched(
      { ...WS, taskId: next[0].taskId },
      { claimedAt: next[0].claimedAt!, now: 1450 },
    );
    await repo.submitForReview({ ...WS, taskId: next[0].taskId }, { now: 1500 });

    // 人工 approve：唯一 approved 入口；approve 后终态，不可再裁决。
    const approved = await repo.recordReviewDecision(
      { ...WS, taskId: ticket.taskId },
      { decision: "approved", now: 1600 },
    );
    assert.equal(approved?.reviewState, "approved");
    assert.equal(
      await repo.recordReviewDecision(
        { ...WS, taskId: ticket.taskId },
        { decision: "changes_requested", now: 1650 },
      ),
      null,
    );
    assert.equal(rawTicket(ctx.raw, ticket.taskId).review_state, "approved");

    // 人工打回 → 返工重新可派 → 再交活，循环闭环。
    const rejected = await repo.recordReviewDecision(
      { ...WS, taskId: next[0].taskId },
      { decision: "changes_requested", now: 1900 },
    );
    assert.equal(rejected?.reviewState, "changes_requested");
    const reclaimed = await repo.claimDueTickets({ now: 2000, limit: 10 });
    assert.deepEqual(reclaimed.map((t) => t.taskId), ["gate-2"]);
    repo.close();
  } finally {
    await ctx.cleanup();
  }
});

test("done stays human: completed-without-submission tickets are never auto redispatched; human can force-reject", async () => {
  const ctx = await setup();
  try {
    seedTicket(ctx.raw, {
      workspacePath: "/w",
      taskId: "stuck-completed",
      taskStatus: "completed",
      acceptanceCriteria: "done",
    });
    seedTicket(ctx.raw, {
      workspacePath: "/w",
      taskId: "crashed-error",
      taskStatus: "error",
      acceptanceCriteria: "done",
    });
    ctx.raw
      .prepare(
        `UPDATE tasks SET dispatch_state='dispatched', claimed_at=500
        WHERE task_id IN ('stuck-completed','crashed-error')`,
      )
      .run();
    const repo = new DispatchDeskRepo(ctx.dbPath);
    await repo.ensureReady();

    // sweeper 认领前的回收：error 崩溃自动转退避重派；completed 只释放单飞槽。
    const next = await repo.claimDueTickets({ now: STALE_MS + 1, limit: 10 });
    const crashed = await repo.getTicket({ ...WS, taskId: "crashed-error" });
    assert.equal(crashed?.dispatchState, "failed_to_dispatch");
    assert.equal(crashed?.dispatchAttempts, 1);
    assert.ok((crashed?.retryAt ?? 0) > STALE_MS + 1, "error 崩溃走基础退避");
    const stuck = await repo.getTicket({ ...WS, taskId: "stuck-completed" });
    assert.equal(stuck?.dispatchState, "idle");
    assert.equal(stuck?.reviewState, null);
    assert.deepEqual(next.map((t) => t.taskId), []);

    // completed 未交活：后续 tick 也不自动重派（done stays human）。
    assert.deepEqual(await repo.claimDueTickets({ now: STALE_MS + 20_000, limit: 10 }), []);
    // 人工强制打回是它唯一的解锁路径；approve 不允许从 NULL 直接写出。
    assert.equal(
      await repo.recordReviewDecision(
        { ...WS, taskId: "stuck-completed" },
        { decision: "approved", now: STALE_MS + 20_001 },
      ),
      null,
    );
    await repo.recordReviewDecision(
      { ...WS, taskId: "stuck-completed" },
      { decision: "changes_requested", now: STALE_MS + 20_002 },
    );
    const rework = await repo.claimDueTickets({ now: STALE_MS + 20_003, limit: 10 });
    assert.deepEqual(rework.map((t) => t.taskId), ["stuck-completed"]);
    repo.close();
  } finally {
    await ctx.cleanup();
  }
});

test("setTicketSpec: non-empty criteria required, deliverables COALESCE keeps existing", async () => {
  const ctx = await setup();
  try {
    seedTicket(ctx.raw, { workspacePath: "/w", taskId: "spec-1" });
    const repo = new DispatchDeskRepo(ctx.dbPath);
    await repo.ensureReady();
    await assert.rejects(
      repo.setTicketSpec({ ...WS, taskId: "spec-1" }, { acceptanceCriteria: "  ", now: 1 }),
      /acceptance_criteria/,
    );
    assert.equal(
      await repo.setTicketSpec({ ...WS, taskId: "missing" }, { acceptanceCriteria: "x", now: 1 }),
      null,
    );
    const withSpec = await repo.setTicketSpec({ ...WS, taskId: "spec-1" }, {
      acceptanceCriteria: "WHEN a THEN b",
      deliverables: { kind: "patch" },
      now: 2,
    });
    assert.equal(withSpec?.acceptanceCriteria, "WHEN a THEN b");
    assert.deepEqual(withSpec?.deliverables, { kind: "patch" });
    // 更新 spec 不带 deliverables → 保留旧值。
    const kept = await repo.setTicketSpec({ ...WS, taskId: "spec-1" }, {
      acceptanceCriteria: "WHEN a THEN c",
      now: 3,
    });
    assert.deepEqual(kept?.deliverables, { kind: "patch" });
    repo.close();
  } finally {
    await ctx.cleanup();
  }
});
