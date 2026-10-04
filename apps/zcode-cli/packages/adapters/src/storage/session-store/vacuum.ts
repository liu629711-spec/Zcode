import type { DatabaseSync } from "node:sqlite";

// 每周一次后台 VACUUM（地基清理 2026-10-04）：删任务清内容后，SQLite 释放的页
// 只进 freelist，数据库文件不会缩小。这里在启动后空闲时点补一次全库压缩。
// 两个闸门都满足才动手：
//  - 距上次压缩 ≥ 7 天（local_setting 哨兵，scope='local' 空会话全局一份）；
//  - freelist ≥ 64MB（可回收空间不够就不值得全库重写）。
// VACUUM 需要库级独占：多进程共享同一 db 时可能 busy 失败——不致命，下周自然重试。
// 哨兵只在真正压缩成功后推进，跳过不算过期。

const VACUUM_MIN_INTERVAL_MS = 7 * 24 * 60 * 60_000;
const VACUUM_MIN_FREELIST_BYTES = 64 * 1024 * 1024;
/** 启动后延迟再压，避开冷启动的写高峰与迁移窗口。 */
const VACUUM_STARTUP_DELAY_MS = 30_000;

const VACUUM_SETTING_NAMESPACE = "session_store";
const VACUUM_SETTING_KEY = "last_vacuum_at";

export function scheduleWeeklyVacuum(
  db: DatabaseSync,
  onError?: (error: unknown) => void,
): void {
  const timer = setTimeout(() => {
    try {
      vacuumIfDue(db);
    } catch (error) {
      onError?.(error);
    }
  }, VACUUM_STARTUP_DELAY_MS);
  // 不拖住进程退出：压缩永远不值得阻塞 shutdown。
  timer.unref?.();
}

function vacuumIfDue(db: DatabaseSync): void {
  const last = readVacuumSentinel(db);
  if (last !== undefined && Date.now() - last < VACUUM_MIN_INTERVAL_MS) return;
  const pageSize = Number(
    (db.prepare("pragma page_size").get() as { page_size?: unknown } | undefined)?.page_size ?? 0,
  );
  const freelistPages = Number(
    (db.prepare("pragma freelist_count").get() as { freelist_count?: unknown } | undefined)
      ?.freelist_count ?? 0,
  );
  if (pageSize <= 0 || freelistPages * pageSize < VACUUM_MIN_FREELIST_BYTES) return;
  db.exec("vacuum");
  db.exec("pragma wal_checkpoint(truncate)");
  writeVacuumSentinel(db, Date.now());
}

function readVacuumSentinel(db: DatabaseSync): number | undefined {
  const row = db
    .prepare(
      "select value from local_setting where scope = 'local' and scope_id = '' and namespace = ? and key = ?",
    )
    .get(VACUUM_SETTING_NAMESPACE, VACUUM_SETTING_KEY) as { value?: unknown } | undefined;
  const parsed = Number(row?.value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
}

function writeVacuumSentinel(db: DatabaseSync, time: number): void {
  db.prepare(
    `
      insert into local_setting (
        scope, scope_id, namespace, key, value, schema_version, time_created, time_updated
      ) values ('local', '', ?, ?, ?, 1, ?, ?)
      on conflict(scope, scope_id, namespace, key) do update set
        value = excluded.value,
        time_updated = excluded.time_updated
    `,
  ).run(VACUUM_SETTING_NAMESPACE, VACUUM_SETTING_KEY, String(time), time, time);
}
