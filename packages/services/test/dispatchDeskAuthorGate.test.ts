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

// 服务面作者通道：setTicketSpec 转发派活台仓库（作者通道=店主写验收标准，是票据进看板的唯一门票）。
// 空 criteria 由仓库抛错不吞（契约=UI 捕获给 toast）；缺票 null 传播（UI 回弹）；deliverables 不上 UI 入口。
test("author gate service surface: setTicketSpec forwards, rejects empty criteria, null on missing ticket", async () => {
  const dir = await mkdtemp(join(tmpdir(), "zcode-desk-author-gate-"));
  const dbPath = join(dir, "tasks-index.sqlite");
  const bootstrap = new DispatchDeskRepo(dbPath);
  await bootstrap.ensureReady();
  bootstrap.close();
  const raw = new DatabaseSync(dbPath);
  // 一张未进台票（无 acceptance_criteria，seed 同 dispatchDesk.test.ts）。
  raw
    .prepare(
      `INSERT INTO tasks (workspace_key, workspace_path, task_id, title, task_status, mode,
        created_at, updated_at)
      VALUES (?, ?, ?, ?, NULL, 'build', 1, 1)`,
    )
    .run("/w", "/w", "a-author", "a-author");
  const deskRepo = new DispatchDeskRepo(dbPath);
  await deskRepo.ensureReady();

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
    // 空 criteria：仓库抛错，服务面原样上抛，不得吞。
    await assert.rejects(
      service.setTicketSpec({ ...WS, taskId: "a-author", acceptanceCriteria: "   ", now: 10 }),
    );

    // 缺票：null 原样传播。
    assert.equal(
      await service.setTicketSpec({
        ...WS,
        taskId: "missing",
        acceptanceCriteria: "done",
        now: 11,
      }),
      null,
    );

    // 成功写入：criteria 首尾空白被仓库 trim；写完立即成为调度认领候选（进台）。
    const ticket = await service.setTicketSpec({
      ...WS,
      taskId: "a-author",
      acceptanceCriteria: "  done  ",
      now: 12,
    });
    assert.equal(ticket?.acceptanceCriteria, "done");
    const claimed = await deskRepo.claimDueTickets({ now: 13, limit: 5 });
    assert.deepEqual(
      claimed.map((t) => t.taskId),
      ["a-author"],
    );
  } finally {
    service.disposeAll();
    raw.close();
    await rm(dir, { recursive: true, force: true }).catch(() => undefined);
  }
});
