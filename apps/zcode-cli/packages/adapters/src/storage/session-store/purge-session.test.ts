// ============================================================
// 删任务清聊天内容（地基清理 2026-10-04）的可运行检查。
// 驱动**真实** sqlite + 真实迁移 schema + 真实 purgeSession 仓储函数。验证：
//  1. purge 后 session 行与 message/part/session_entry/todo/session_input/
//     session_target(_next)/model_usage/turn_usage/tool_usage 全部清空——
//     连接未开 foreign_keys，schema 里的 cascade 不会自动生效，显式逐表删
//     必须与 schema 声明一一对应，漏一张表就是永久孤儿行；
//  2. set-null 语义：workflow_run.parent_session_id、workflow_activity.
//     child_session_id、session_task_link.parent_session_id 指针放开，行本身留下；
//  3. 级联边界：别的会话（sess-keep）的行一根毫毛都不能少；
//  4. 会话不存在 → 返回 false（幂等）。
// 运行：npx tsx --test apps/zcode-cli/packages/adapters/src/storage/session-store/purge-session.test.ts
// ============================================================

import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync, type SQLInputValue } from "node:sqlite";
import { test } from "node:test";
import { runSqliteSessionMigrations } from "./migration-runner.js";
import { purgeSession } from "./repositories/sessions.js";

const PURGED = "sess-purge";
const KEPT = "sess-keep";
const NOW = 1_700_000_000_000;

function openDb(): { db: DatabaseSync; cleanup: () => void } {
  const dir = mkdtempSync(join(tmpdir(), "purge-session-"));
  const db = new DatabaseSync(join(dir, "db.sqlite"));
  runSqliteSessionMigrations(db, join(dir, "db.sqlite"));
  return { db, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

function seed(db: DatabaseSync): void {
  for (const sessionId of [PURGED, KEPT]) {
    db
      .prepare(
        "insert into session (id, project_id, slug, directory, title, version, time_created, time_updated) values (?, 'p', 's', '/tmp/x', 't', 'test', ?, ?)",
      )
      .run(sessionId, NOW, NOW);
    db
      .prepare(
        "insert into message (id, session_id, time_created, time_updated, data) values (?, ?, ?, ?, '{}')",
      )
      .run(`m-${sessionId}`, sessionId, NOW, NOW);
    db
      .prepare(
        "insert into part (id, message_id, session_id, time_created, time_updated, data) values (?, ?, ?, ?, ?, '{}')",
      )
      .run(`p-${sessionId}`, `m-${sessionId}`, sessionId, NOW, NOW);
    db
      .prepare(
        "insert into session_entry (id, session_id, type, time_created, time_updated, data) values (?, ?, 'command', ?, ?, '{}')",
      )
      .run(`e-${sessionId}`, sessionId, NOW, NOW);
    db
      .prepare(
        "insert into todo (session_id, content, status, priority, position, time_created, time_updated) values (?, 'c', 'pending', 'high', 0, ?, ?)",
      )
      .run(sessionId, NOW, NOW);
    db
      .prepare(
        "insert into session_target (session_id, target_id, objective, status, time_created, time_updated) values (?, 'tg', 'o', 'active', ?, ?)",
      )
      .run(sessionId, NOW, NOW);
    db
      .prepare(
        "insert into model_usage (id, logical_request_id, session_id, query_source, provider_id, model_id, status, started_at) values (?, 'lr', ?, 'user', 'prov', 'model', 'completed', ?)",
      )
      .run(`mu-${sessionId}`, sessionId, NOW);
    db
      .prepare(
        "insert into turn_usage (session_id, turn_id, status, started_at) values (?, ?, 'completed', ?)",
      )
      .run(sessionId, `turn-${sessionId}`, NOW);
    db
      .prepare(
        "insert into tool_usage (id, session_id, tool_call_id, tool_name, status, started_at) values (?, ?, 'tc', 'Bash', 'completed', ?)",
      )
      .run(`tu-${sessionId}`, sessionId, NOW);
    db
      .prepare(
        "insert into session_input (id, session_id, kind, delivery, payload, admitted_sequence, status, time_created, time_updated) values (?, ?, 'agentWorkOrderDispatch', 'queue', '{}', 1, 'discarded', ?, ?)",
      )
      .run(`si-${sessionId}`, sessionId, NOW, NOW);
  }
  // set-null 引用 + 级联边界：sess-keep 作为 child 的 link 必须活着（只放开 parent 指针）。
  db
    .prepare(
      "insert into workflow_run (id, name, parent_session_id, cwd, script_hash, status, time_created, time_updated) values ('run-1', 'r', ?, '/tmp', 'h', 'completed', ?, ?)",
    )
    .run(PURGED, NOW, NOW);
  db
    .prepare(
      "insert into workflow_activity (id, run_id, call_index, call_path, type, input_hash, child_session_id, status, time_created, time_updated) values ('act-1', 'run-1', 0, '0', 'agent', 'h0', ?, 'completed', ?, ?)",
    )
    .run(PURGED, NOW, NOW);
  db
    .prepare(
      "insert into session_task_link (id, parent_session_id, child_session_id, role, depth, path, status, time_created, time_updated) values ('link-keep', ?, ?, 'worker', 0, 'a', 'active', 1, 1)",
    )
    .run(PURGED, KEPT);
  db
    .prepare(
      "insert into session_task_link (id, parent_session_id, child_session_id, role, depth, path, status, time_created, time_updated) values ('link-die', NULL, ?, 'worker', 0, 'b', 'active', 1, 1)",
    )
    .run(PURGED);
}

function scalar(db: DatabaseSync, sql: string, ...args: SQLInputValue[]): number {
  return Number((db.prepare(sql).get(...args) as { n?: unknown } | undefined)?.n ?? 0);
}

test("purgeSession：内容行全清、set-null 放指针、别的会话毫发无损", () => {
  const { db, cleanup } = openDb();
  try {
    seed(db);
    assert.equal(purgeSession(db, PURGED as never), true);

    for (const table of [
      "message",
      "part",
      "session_entry",
      "todo",
      "session_target",
      "model_usage",
      "turn_usage",
      "tool_usage",
      "session_input",
    ]) {
      assert.equal(
        scalar(db, `select count(*) as n from ${table} where session_id = ?`, PURGED),
        0,
        `${table} 应清空`,
      );
    }
    assert.equal(scalar(db, "select count(*) as n from session where id = ?", PURGED), 0);
    // set-null：行留着，指针放开。
    assert.equal(
      scalar(db, "select count(*) as n from workflow_run where id = 'run-1' and parent_session_id is null"),
      1,
    );
    assert.equal(
      scalar(db, "select count(*) as n from workflow_activity where id = 'act-1' and child_session_id is null"),
      1,
    );
    assert.equal(
      scalar(db, "select count(*) as n from session_task_link where id = 'link-keep' and parent_session_id is null"),
      1,
    );
    // 级联边界：以被删会话为 child 的 link 行清掉；以它为 parent 的 link 留下。
    assert.equal(scalar(db, "select count(*) as n from session_task_link where id = 'link-die'"), 0);

    // 对照组：没被删的会话一根毫毛不少（session 行、message、part、台账行各 1）。
    assert.equal(scalar(db, "select count(*) as n from session where id = ?", KEPT), 1);
    assert.equal(scalar(db, "select count(*) as n from message where session_id = ?", KEPT), 1);
    assert.equal(scalar(db, "select count(*) as n from part where session_id = ?", KEPT), 1);
    assert.equal(scalar(db, "select count(*) as n from session_input where session_id = ?", KEPT), 1);
  } finally {
    db.close();
    cleanup();
  }
});

test("purgeSession：会话不存在返回 false（幂等，UI 重复删除不炸）", () => {
  const { db, cleanup } = openDb();
  try {
    assert.equal(purgeSession(db, "sess-missing" as never), false);
  } finally {
    db.close();
    cleanup();
  }
});
