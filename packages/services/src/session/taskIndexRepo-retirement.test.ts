// ============================================================
// 工位退役反查（2026-10-02 拍板：派单只续用侧栏看得见的工位）的可运行检查：
// 任务索引的 tombstone / 归档行都能被 getTaskRetirement 识别，缺行 known=false。
// 运行：npx tsx --test packages/services/src/session/taskIndexRepo-retirement.test.ts
// ============================================================

import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import type { ZCodeTaskMeta } from "@zcode/shared";
import { TaskIndexRepo } from "./taskIndexRepo.js";

const WORKSPACE = "D:/retirement-test/ws";

function meta(taskId: string): ZCodeTaskMeta {
  return {
    taskId,
    traceId: `trace-${taskId}`,
    title: `task ${taskId}`,
    workspacePath: WORKSPACE,
    createdAt: 1,
    updatedAt: 1,
    mode: "build",
  };
}

test("getTaskRetirement：在册行按 deleted/archived 报退役，缺行 known=false，跨工作区不误伤", async () => {
  const dbPath = join(
    mkdtempSync(join(tmpdir(), `task-index-ret-${randomUUID()}`)),
    "tasks-index.sqlite",
  );
  const repo = new TaskIndexRepo(dbPath);
  await repo.seedTaskMetaIfMissing(meta("sess_active"));
  await repo.seedTaskMetaIfMissing(meta("sess_deleted"));
  await repo.seedTaskMetaIfMissing(meta("sess_archived"));
  await repo.updateTaskState({
    workspacePath: WORKSPACE,
    taskId: "sess_deleted",
    patch: { deleted: true },
  });
  await repo.updateTaskState({
    workspacePath: WORKSPACE,
    taskId: "sess_archived",
    patch: { archived: true },
  });

  assert.deepEqual(
    await repo.getTaskRetirement({ workspacePath: WORKSPACE, taskId: "sess_active" }),
    { retired: false, known: true },
  );
  assert.deepEqual(
    await repo.getTaskRetirement({ workspacePath: WORKSPACE, taskId: "sess_deleted" }),
    { retired: true, known: true },
  );
  assert.deepEqual(
    await repo.getTaskRetirement({ workspacePath: WORKSPACE, taskId: "sess_archived" }),
    { retired: true, known: true },
  );
  assert.deepEqual(
    await repo.getTaskRetirement({ workspacePath: WORKSPACE, taskId: "sess_missing" }),
    { retired: false, known: false },
  );
  // 同号任务在别的 workspace 没入册：不得误报退役。
  assert.deepEqual(
    await repo.getTaskRetirement({
      workspacePath: "D:/retirement-test/other-ws",
      taskId: "sess_active",
    }),
    { retired: false, known: false },
  );
  (repo as unknown as { close(): void }).close();
});
