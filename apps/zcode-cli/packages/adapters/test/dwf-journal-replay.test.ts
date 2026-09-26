import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";
import {
  DwfRunReplayError,
  loadDwfRunReplay,
} from "../src/storage/session-store/repositories/dwf-journal-replay.js";

// ============================================================
// 回放投影的单测：折叠核经真实 sqlite 装载路径一起钉
// （空事件 / 多 attempt / 超大 run / 缺表 / 坏数据 / 未知 run）。
// 建表用最小列集：装载面只读这几列，列序与 migration 无关。
// ============================================================

function openDb(): DatabaseSync {
  const db = new DatabaseSync(":memory:");
  db.exec(`
    create table dwf_run (
      id text primary key,
      name text,
      status text not null,
      resumed_from text,
      time_created integer not null,
      time_updated integer not null
    );
    create table dwf_node (
      id integer primary key autoincrement,
      run_id text not null,
      site_id text not null,
      ordinal integer not null,
      status text not null,
      unique(run_id, site_id, ordinal)
    );
    create table dwf_event (
      id integer primary key autoincrement,
      run_id text not null,
      sequence integer not null,
      type text not null,
      payload_json text,
      unique(run_id, sequence)
    );
  `);
  return db;
}

function insertRun(
  db: DatabaseSync,
  runId: string,
  overrides: { name?: string; status?: string; resumedFrom?: string } = {},
): void {
  db.prepare(
    "insert into dwf_run (id, name, status, resumed_from, time_created, time_updated) values (?, ?, ?, ?, 1000, 2000)",
  ).run(
    runId,
    overrides.name ?? null,
    overrides.status ?? "completed",
    overrides.resumedFrom ?? null,
  );
}

function insertEvent(db: DatabaseSync, runId: string, sequence: number, type: string, payload: unknown): void {
  db.prepare(
    "insert into dwf_event (run_id, sequence, type, payload_json) values (?, ?, ?, ?)",
  ).run(runId, sequence, type, JSON.stringify(payload));
}

/** 一个 ask 实例的完整一生（queued → dispatched → [*mid*] → executing → settled）。 */
function lifecycle(
  db: DatabaseSync,
  runId: string,
  base: number,
  siteId: string,
  ordinal: number,
  extra: {
    outcome?: string;
    cached?: boolean;
    error?: unknown;
    /** 派发后、执行前插入的旁路事件（等待 / 修复等），按给定序落在轨迹上。 */
    mid?: Array<[string, Record<string, unknown>]>;
  } = {},
): number {
  const instance = { siteId, ordinal };
  const events: Array<[string, Record<string, unknown>]> = [
    ["node-queued", { instance, kind: "ask", phaseName: "评审", instructionsHead: "请评审这份材料" }],
    ["node-dispatched", { instance, kind: "ask", phaseName: "评审", instructionsHead: "请评审这份材料" }],
    ...(extra.mid ?? []),
    ["node-executing", { instance }],
    [
      "node-settled",
      {
        instance,
        outcome: extra.outcome ?? "ok",
        ...(extra.cached === undefined ? {} : { cached: extra.cached }),
        ...(extra.error === undefined ? {} : { error: extra.error }),
      },
    ],
  ];
  for (const [index, [type, payload]] of events.entries()) {
    insertEvent(db, runId, base + index, type, payload);
  }
  return base + events.length;
}

test("空事件 run：合法状态，返回空时间线而不是报错", () => {
  const db = openDb();
  insertRun(db, "run-empty");
  const replay = loadDwfRunReplay(db, "run-empty");
  assert.equal(replay.run.runId, "run-empty");
  assert.equal(replay.run.status, "completed");
  assert.equal(replay.timeline.eventCount, 0);
  assert.equal(replay.timeline.instances.length, 0);
  assert.equal(replay.timeline.attempts.length, 0);
  assert.equal(replay.timeline.phases.length, 0);
  assert.equal(replay.timeline.lastSequence, undefined);
});

test("多 attempt run：按 ordinal 分组，轨迹/出生事实/错误摘要逐条可读", () => {
  const db = openDb();
  insertRun(db, "run-multi", { name: "评审大队", resumedFrom: "run-prev" });
  let sequence = 0;
  // site A：两次执行（attempt 0 失败、attempt 1 成功，成功那次的轨迹里带等待与修复）；
  // site B：一次成功。
  sequence = lifecycle(db, "run-multi", sequence, "review", 0, {
    outcome: "failed",
    error: { name: "ProviderError", message: "模型请求超时" },
  });
  sequence = lifecycle(db, "run-multi", sequence, "review", 1, {
    mid: [
      ["node-waiting", { instance: { siteId: "review", ordinal: 1 }, cause: "slot", reason: "并发已满" }],
      ["node-repairing", { instance: { siteId: "review", ordinal: 1 }, attempt: 2 }],
      ["report", { instance: { siteId: "review", ordinal: 1 }, item: { findings: 3 } }],
    ],
  });
  sequence = lifecycle(db, "run-multi", sequence, "fix", 0);

  // 旁路事实：phase 刻度、缓存命中。
  insertEvent(db, "run-multi", sequence++, "phase-entered", { name: "评审", ordinal: 0 });
  insertEvent(db, "run-multi", sequence++, "phase-entered", { name: "修复", ordinal: 0 });
  // 缓存命中：settle 是出生事件，带 phaseName。
  insertEvent(db, "run-multi", sequence++, "node-settled", {
    instance: { siteId: "cached", ordinal: 0 },
    outcome: "ok",
    cached: true,
    phaseName: "评审",
  });

  const replay = loadDwfRunReplay(db, "run-multi");
  assert.equal(replay.run.name, "评审大队");
  assert.equal(replay.run.resumedFrom, "run-prev");
  assert.equal(replay.timeline.eventCount, sequence);

  // 分组：attempt 0 有两个站点，attempt 1 一个。
  assert.deepEqual(
    replay.timeline.attempts.map((group) => group.attempt),
    [0, 1],
  );
  assert.deepEqual(
    replay.timeline.attempts[0]!.instances.map((instance) => instance.siteId),
    // 缓存命中的 cached@0 也是 attempt 0 的一员（settle 即出生）。
    ["review", "fix", "cached"],
  );
  assert.deepEqual(
    replay.timeline.attempts[1]!.instances.map((instance) => instance.siteId),
    ["review"],
  );

  const failed = replay.timeline.attempts[0]!.instances[0]!;
  assert.equal(failed.outcome, "failed");
  assert.equal(failed.error, "模型请求超时");
  assert.equal(failed.label, "评审");
  assert.equal(failed.summary, "请评审这份材料");
  assert.deepEqual(
    failed.steps.map((step) => step.state),
    ["queued", "dispatched", "executing", "failed"],
  );

  const succeeded = replay.timeline.attempts[1]!.instances[0]!;
  // waiting 与 repairing 记在同一条轨迹上，等待原因与修复轮次可读。
  assert.deepEqual(
    succeeded.steps.map((step) => step.state),
    ["queued", "dispatched", "waiting", "repairing", "report", "executing", "completed"],
  );
  assert.match(succeeded.steps.find((step) => step.state === "waiting")!.detail!, /并发已满/);
  assert.match(succeeded.steps.find((step) => step.state === "repairing")!.detail!, /repair #2/);

  const cached = replay.timeline.instances.find((instance) => instance.siteId === "cached")!;
  assert.equal(cached.cached, true);
  assert.equal(cached.label, "评审");
  assert.deepEqual(
    cached.steps.map((step) => step.state),
    ["completed"],
  );

  assert.deepEqual(
    replay.timeline.phases.map((phase) => phase.name),
    ["评审", "修复"],
  );

  // node-progress 与 run 级事件不进轨迹（进度流水不是回放的职责）。
  const before = replay.timeline.instances.reduce((sum, instance) => sum + instance.steps.length, 0);
  insertEvent(db, "run-multi", sequence++, "node-progress", {
    instance: { siteId: "review", ordinal: 1 },
    lastTool: "Read",
  });
  insertEvent(db, "run-multi", sequence++, "usage-updated", { spentTokens: 42 });
  insertEvent(db, "run-multi", sequence++, "run-settled", { status: "completed" });
  const after = loadDwfRunReplay(db, "run-multi");
  assert.equal(
    after.timeline.instances.reduce((sum, instance) => sum + instance.steps.length, 0),
    before,
  );
  assert.equal(after.timeline.eventCount, sequence);
});

test("超大 run：几千事件一次折叠，分组与计数不漂移", () => {
  const db = openDb();
  insertRun(db, "run-big");
  const SITES = 4;
  const ROUNDS = 500; // 4 站点 × 500 轮 × 每轮 4 条生命周期 ≈ 8000 事件
  let sequence = 0;
  for (let round = 0; round < ROUNDS; round += 1) {
    for (let site = 0; site < SITES; site += 1) {
      sequence = lifecycle(db, "run-big", sequence, `site-${site}`, round);
    }
  }
  const replay = loadDwfRunReplay(db, "run-big");
  assert.equal(replay.timeline.eventCount, sequence);
  assert.equal(replay.timeline.instances.length, SITES * ROUNDS);
  assert.equal(replay.timeline.attempts.length, ROUNDS);
  // 每组都是 SITES 个实例，且组内出场序稳定（首提序 = 事件轨顺序）。
  for (const group of replay.timeline.attempts) {
    assert.equal(group.instances.length, SITES);
    assert.deepEqual(
      group.instances.map((instance) => instance.siteId),
      [`site-0`, `site-1`, `site-2`, `site-3`],
    );
    for (const instance of group.instances) {
      assert.equal(instance.steps.length, 4);
      assert.equal(instance.outcome, "ok");
    }
  }
  assert.equal(replay.timeline.lastSequence, sequence - 1);
});

test("缺表：校验器点名缺失的表", () => {
  const db = openDb();
  insertRun(db, "run-x");
  db.exec("drop table dwf_node");
  try {
    loadDwfRunReplay(db, "run-x");
    assert.fail("应当抛出 DwfRunReplayError");
  } catch (error) {
    assert.ok(error instanceof DwfRunReplayError);
    assert.equal(error.table, "dwf_node");
    assert.match(error.message, /dwf_node/);
  }

  // 三表全缺（全新空库 = migration 一条没跑）：点名全部。
  const bare = new DatabaseSync(":memory:");
  try {
    loadDwfRunReplay(bare, "run-x");
    assert.fail("应当抛出 DwfRunReplayError");
  } catch (error) {
    assert.ok(error instanceof DwfRunReplayError);
    assert.match(error.message, /dwf_run/);
    assert.match(error.message, /dwf_node/);
    assert.match(error.message, /dwf_event/);
  }
});

test("未知 run：校验器点名 dwf_run", () => {
  const db = openDb();
  try {
    loadDwfRunReplay(db, "no-such-run");
    assert.fail("应当抛出 DwfRunReplayError");
  } catch (error) {
    assert.ok(error instanceof DwfRunReplayError);
    assert.equal(error.table, "dwf_run");
    assert.match(error.message, /no-such-run/);
  }
});

test("坏数据：payload_json 解不开 / 不是对象，报到 sequence", () => {
  const db = openDb();
  insertRun(db, "run-bad");
  db.prepare(
    "insert into dwf_event (run_id, sequence, type, payload_json) values (?, ?, ?, ?)",
  ).run("run-bad", 0, "node-queued", "{not json");
  db.prepare(
    "insert into dwf_event (run_id, sequence, type, payload_json) values (?, ?, ?, ?)",
  ).run("run-bad", 1, "node-queued", "42");

  try {
    loadDwfRunReplay(db, "run-bad");
    assert.fail("应当抛出 DwfRunReplayError");
  } catch (error) {
    assert.ok(error instanceof DwfRunReplayError);
    assert.equal(error.table, "dwf_event");
    assert.match(error.message, /sequence 0/);
  }

  const db2 = openDb();
  insertRun(db2, "run-bad2");
  db2.prepare(
    "insert into dwf_event (run_id, sequence, type, payload_json) values (?, ?, ?, ?)",
  ).run("run-bad2", 1, "node-queued", "42");
  try {
    loadDwfRunReplay(db2, "run-bad2");
    assert.fail("应当抛出 DwfRunReplayError");
  } catch (error) {
    assert.ok(error instanceof DwfRunReplayError);
    assert.equal(error.table, "dwf_event");
    assert.match(error.message, /sequence 1/);
  }
});
