import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import test from "node:test";
import { createZCodeTaskServiceAdapter } from "../src/zcode-agent/zcodeTaskServiceAdapter.js";
import { DispatchDeskRepo } from "../src/session/dispatchDeskRepo.js";
import { TaskIndexRepo } from "../src/session/taskIndexRepo.js";

const require = createRequire(import.meta.url);
const { DatabaseSync } = require("node:sqlite") as typeof import("node:sqlite");

const WS = { workspacePath: "/w" };

// 服务面人工通道：recordReviewDecision/manualRequeue 转发派活台仓库，
// null 原样传播（UI 靠 null 回弹）；调度器/代理四方法不上服务面由描述符 grep 门禁把守。
test("manual gate service surface: review decision forwards with guard, manualRequeue resets failed tickets", async () => {
  const dir = await mkdtemp(join(tmpdir(), "zcode-desk-manual-gate-"));
  const dbPath = join(dir, "tasks-index.sqlite");
  const bootstrap = new DispatchDeskRepo(dbPath);
  await bootstrap.ensureReady();
  bootstrap.close();
  const raw = new DatabaseSync(dbPath);
  // 两张进台票（seed 同 dispatchDesk.test.ts）：a-review 走到 pending_review，b-failed 派发失败。
  for (const taskId of ["a-review", "b-failed"]) {
    raw
      .prepare(
        `INSERT INTO tasks (workspace_key, workspace_path, task_id, title, task_status, mode,
          created_at, updated_at, acceptance_criteria)
        VALUES (?, ?, ?, ?, NULL, 'build', 1, 1, 'done')`,
      )
      .run("/w", "/w", taskId, taskId);
  }
  const deskRepo = new DispatchDeskRepo(dbPath);
  await deskRepo.ensureReady();
  const [a] = await deskRepo.claimDueTickets({ now: 10, limit: 1 });
  assert.equal(a?.taskId, "a-review");
  await deskRepo.markDispatched({ ...WS, taskId: "a-review" }, { claimedAt: a.claimedAt!, now: 11 });
  await deskRepo.submitForReview({ ...WS, taskId: "a-review" }, { now: 12 });
  const [b] = await deskRepo.claimDueTickets({ now: 13, limit: 1 });
  assert.equal(b?.taskId, "b-failed");
  await deskRepo.markDispatchFailed(
    { ...WS, taskId: "b-failed" },
    { claimedAt: b.claimedAt!, now: 14, error: "boom" },
  );

  type Options = Parameters<typeof createZCodeTaskServiceAdapter>[0];
  const disposable = () => ({ dispose() {} });
  const service = createZCodeTaskServiceAdapter({
    dispatchDeskRepo: deskRepo,
    taskIndexRepo: new TaskIndexRepo(join(dir, "tasks.sqlite")),
    zcodeAgentService: { disposeAll() {} } as unknown as Options["zcodeAgentService"],
    taskIndexSyncer: {
      onSessionTerminalEvent: disposable,
      onSessionReadyEvent: disposable,
      disposeAll() {},
    } as unknown as Options["taskIndexSyncer"],
  });
  try {
    // 转发 + 守卫：pending_review→approved 成功；approved 终态再裁返回 null。
    const approved = await service.recordReviewDecision({
      ...WS,
      taskId: "a-review",
      decision: "approved",
      now: 100,
    });
    assert.equal(approved?.reviewState, "approved");
    assert.equal(
      await service.recordReviewDecision({
        ...WS,
        taskId: "a-review",
        decision: "changes_requested",
        now: 101,
      }),
      null,
    );

    // null 传播：非 failed 票与不存在的票原样返回 null。
    assert.equal(await service.manualRequeue({ ...WS, taskId: "a-review", now: 102 }), null);
    assert.equal(await service.manualRequeue({ ...WS, taskId: "missing", now: 102 }), null);

    // 端到端：failed 票经服务面重派成功，字段归位并立即恢复可认领。
    const requeued = await service.manualRequeue({ ...WS, taskId: "b-failed", now: 200 });
    assert.equal(requeued?.dispatchState, "idle");
    assert.equal(requeued?.dispatchAttempts, 0);
    assert.equal(requeued?.retryAt, null);
    assert.equal(requeued?.lastDispatchError, null);
    const reClaimed = await deskRepo.claimDueTickets({ now: 300, limit: 5 });
    assert.deepEqual(reClaimed.map((t) => t.taskId), ["b-failed"]);
  } finally {
    service.disposeAll();
    raw.close();
    await rm(dir, { recursive: true, force: true }).catch(() => undefined);
  }
});
