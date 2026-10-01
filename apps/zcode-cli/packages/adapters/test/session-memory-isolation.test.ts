// 派单隔离工位标记（memory_isolation 列）的落盘回读检查（audit 2026-10-01）。
// 运行：npx tsx --test apps/zcode-cli/packages/adapters/test/session-memory-isolation.test.ts

import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";
import { runSqliteSessionMigrations } from "../src/storage/session-store/migration-runner.js";
import { createSession, getSession, updateSession } from "../src/storage/session-store/repositories/sessions.js";

function openMigratedDb(): DatabaseSync {
  const db = new DatabaseSync(":memory:");
  runSqliteSessionMigrations(db, ":memory:");
  return db;
}

const BASE = {
  projectID: "proj" as const,
  slug: "worker-session",
  directory: "D:/ws",
  title: "worker",
  version: "test",
};

test("createSession 落 memoryIsolated，getSession 读回 true", () => {
  const db = openMigratedDb();
  createSession(db, { ...BASE, id: "s1" as never, memoryIsolated: true });
  assert.equal(getSession(db, "s1" as never)?.memoryIsolated, true);
});

test("缺省会话不带标记（undefined），隔离不误伤普通会话", () => {
  const db = openMigratedDb();
  createSession(db, { ...BASE, id: "s2" as never });
  assert.equal(getSession(db, "s2" as never)?.memoryIsolated, undefined);
});

test("updateSession 只置位不清除：显式 true 生效，其余写通道不动标记", async () => {
  const db = openMigratedDb();
  createSession(db, { ...BASE, id: "s3" as never });
  await updateSession(db, { id: "s3" as never, title: "renamed", memoryIsolated: true });
  assert.equal(getSession(db, "s3" as never)?.memoryIsolated, true);
  // 别的 update 调用（如改标题）不携带标记 → 不清除。
  await updateSession(db, { id: "s3" as never, title: "renamed-again" });
  assert.equal(getSession(db, "s3" as never)?.memoryIsolated, true);
});

test("createSession 重放（upsert）不漂移既有标记（首写为准，同 persona 纪律）", () => {
  const db = openMigratedDb();
  createSession(db, { ...BASE, id: "s4" as never, memoryIsolated: true });
  createSession(db, { ...BASE, id: "s4" as never, title: "replayed" });
  assert.equal(getSession(db, "s4" as never)?.memoryIsolated, true);
});
