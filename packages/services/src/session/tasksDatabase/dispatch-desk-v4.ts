// 冻结派活台旁路列 SQL：不能改已发布 migration，新列只经本 migration 追加。
// 列语义与状态机见 dispatchDeskRepo.ts；不改 task_status 三值枚举（running/completed/error）。
// 旁路列不进 schema-v1 的 TASK_INDEX_SCHEMA（那会改变 0001 已应用 checksum），
// 与 searchable_text/cron_automation_id 等历史列同款处理：只经 ALTER TABLE 追加。
// review_state 可空：NULL=未进入评审流；CHECK 在 ADD COLUMN 下已验证可用（node:sqlite / SQLite 3.4x）。
export const DISPATCH_DESK_MIGRATION_SQL = `
      ALTER TABLE tasks ADD COLUMN review_state TEXT
        CHECK (review_state IS NULL OR review_state IN ('pending_review','approved','changes_requested'));
      ALTER TABLE tasks ADD COLUMN acceptance_criteria TEXT;
      ALTER TABLE tasks ADD COLUMN deliverables TEXT;
      ALTER TABLE tasks ADD COLUMN dispatch_state TEXT NOT NULL DEFAULT 'idle'
        CHECK (dispatch_state IN ('idle','claimed','dispatched','failed_to_dispatch'));
      ALTER TABLE tasks ADD COLUMN claimed_at INTEGER;
      ALTER TABLE tasks ADD COLUMN dispatch_attempts INTEGER NOT NULL DEFAULT 0;
      ALTER TABLE tasks ADD COLUMN retry_at INTEGER;
      ALTER TABLE tasks ADD COLUMN last_dispatch_error TEXT;

      CREATE INDEX IF NOT EXISTS idx_tasks_dispatch_due
      ON tasks (dispatch_state, workspace_key)
      WHERE deleted = 0;
    `;
