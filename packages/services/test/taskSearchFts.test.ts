import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import test from "node:test";
import { TaskIndexRepo } from "../src/session/taskIndexRepo.js";
import {
  adoptTaskSearchFts,
  buildCjkBigramText,
  buildTaskFtsMatch,
} from "../src/session/tasksDatabase/task-search-fts5.js";

// 全文搜索（migration 0005）：tasks.searchable_text 的 FTS5 倒排。
// 中文按重叠二字词索引/短语查询；纯拉丁与含单字汉字回落 LIKE。

const require = createRequire(import.meta.url);
const { DatabaseSync } = require("node:sqlite") as typeof import("node:sqlite");

const WS = { workspacePath: "/w" };

interface Ctx {
  repo: TaskIndexRepo;
  raw: DatabaseSync;
  cleanup: () => Promise<void>;
}

async function setup(): Promise<Ctx> {
  const dir = await mkdtemp(join(tmpdir(), "zcode-task-search-fts-"));
  const repo = new TaskIndexRepo(join(dir, "tasks-index.sqlite"));
  await repo.ensureReady();
  const raw = new DatabaseSync(join(dir, "tasks-index.sqlite"));
  return {
    repo,
    raw,
    cleanup: async () => {
      raw.close();
      repo.close();
      await rm(dir, { recursive: true, force: true }).catch(() => undefined);
    },
  };
}

async function seedTask(
  repo: TaskIndexRepo,
  taskId: string,
  title: string,
  searchableText: string,
): Promise<void> {
  await repo.syncTaskMeta({
    meta: {
      taskId,
      traceId: `trace-${taskId}`,
      title,
      workspacePath: WS.workspacePath,
      createdAt: 1000,
      updatedAt: 2000,
      mode: "build",
    },
    searchableText,
  });
}

async function searchIds(repo: TaskIndexRepo, search: string): Promise<string[]> {
  const result = await repo.queryTaskList({
    kind: "active",
    workspaceScopes: [WS],
    search,
  });
  return result.items.map((item) => item.taskId);
}

test("buildCjkBigramText 展开重叠二字词，buildTaskFtsMatch 编译短语并按规则回落", () => {
  assert.equal(buildCjkBigramText("登录报错"), "登录 录报 报错");
  assert.equal(buildCjkBigramText("a登录b"), "登录");
  assert.equal(buildCjkBigramText("abc"), "");
  assert.equal(buildCjkBigramText(""), "");

  // 中文 ≥2 字 → 短语；纯拉丁 → null（保 LIKE 中缀）；含单字汉字 → null（回落）。
  assert.equal(buildTaskFtsMatch("登录 报错"), `"登录" AND "报错"`);
  assert.equal(buildTaskFtsMatch("页面报错"), `"页面 面报 报错"`);
  assert.equal(buildTaskFtsMatch("fix 登录"), `"登录" AND "fix"*`);
  assert.equal(buildTaskFtsMatch("error"), null);
  assert.equal(buildTaskFtsMatch("周报 做"), null);
  assert.equal(buildTaskFtsMatch("   "), null);
});

test("中文多词 AND：两个词都出现的任务才命中", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "task-a", "票 A", "登录页面报错了，请检查配置");
    await seedTask(ctx.repo, "task-b", "票 B", "数据库连接超时，请重试");

    assert.deepEqual(await searchIds(ctx.repo, "登录 报错"), ["task-a"]);
    assert.deepEqual(await searchIds(ctx.repo, "数据库 超时"), ["task-b"]);
    // 单词（二字）也走 FTS。
    assert.deepEqual(await searchIds(ctx.repo, "超时"), ["task-b"]);
    assert.deepEqual(await searchIds(ctx.repo, "报错"), ["task-a"]);
  } finally {
    await ctx.cleanup();
  }
});

test("二字词是子串语义：周报 命中长句中间，混写英文随迁", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "task-a", "票 A", "帮我写个周报总结");
    await seedTask(ctx.repo, "task-b", "票 B", "fix the 登录 bug now");
    await seedTask(ctx.repo, "task-c", "票 C", "纯英文正文 no cjk here");

    assert.deepEqual(await searchIds(ctx.repo, "周报"), ["task-a"]);
    assert.deepEqual(await searchIds(ctx.repo, "写个周报"), ["task-a"]);
    // 混合词：fix 与 登录 都命中才算。
    assert.deepEqual(await searchIds(ctx.repo, "fix 登录"), ["task-b"]);
    assert.deepEqual(await searchIds(ctx.repo, "登录 bug"), ["task-b"]);
    assert.deepEqual(await searchIds(ctx.repo, "fix 周报"), []);
  } finally {
    await ctx.cleanup();
  }
});

test("单字汉字与纯拉丁回落 LIKE：保住词中缀命中", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "task-a", "票 A", "登录页面报错了");
    await seedTask(ctx.repo, "task-b", "票 B", "a TypeError was thrown");

    // 单字 → LIKE 子串。
    assert.deepEqual(await searchIds(ctx.repo, "错"), ["task-a"]);
    // 纯拉丁 → LIKE 词中缀（"error" 命中 "TypeError"）。
    assert.deepEqual(await searchIds(ctx.repo, "error"), ["task-b"]);
    assert.deepEqual(await searchIds(ctx.repo, "ype"), ["task-b"]);
  } finally {
    await ctx.cleanup();
  }
});

test("标题进索引；更新换正文后旧词失效；删除后不可搜", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "task-a", "修复登录", "");
    assert.deepEqual(await searchIds(ctx.repo, "修复"), ["task-a"]);

    // 更新正文（传 searchableText）→ FTS 行重建。
    await seedTask(ctx.repo, "task-a", "票 A", "新的正文内容");
    assert.deepEqual(await searchIds(ctx.repo, "修复"), []);
    assert.deepEqual(await searchIds(ctx.repo, "新的正文"), ["task-a"]);

    // 不传 searchableText（保留现有值）→ 依旧可搜。
    await ctx.repo.syncTaskMeta({
      meta: {
        taskId: "task-a",
        traceId: "trace-task-a",
        title: "票 A",
        workspacePath: WS.workspacePath,
        createdAt: 1000,
        updatedAt: 3000,
        mode: "build",
      },
    });
    assert.deepEqual(await searchIds(ctx.repo, "新的正文"), ["task-a"]);

    // 删除 → FTS 行摘除。
    await ctx.repo.syncTaskMeta({
      meta: {
        taskId: "task-a",
        traceId: "trace-task-a",
        title: "票 A",
        workspacePath: WS.workspacePath,
        createdAt: 1000,
        updatedAt: 4000,
        mode: "build",
      },
      deleted: true,
    });
    assert.deepEqual(await searchIds(ctx.repo, "新的正文"), []);
  } finally {
    await ctx.cleanup();
  }
});

test("migration 0005 回填：清空 FTS 表后重跑 adoptTaskSearchFts 能恢复命中", async () => {
  const ctx = await setup();
  try {
    await seedTask(ctx.repo, "task-a", "票 A", "登录页面报错了");
    assert.deepEqual(await searchIds(ctx.repo, "登录"), ["task-a"]);

    ctx.raw.exec("DELETE FROM task_search_fts");
    assert.deepEqual(await searchIds(ctx.repo, "登录"), []);

    adoptTaskSearchFts(ctx.raw);
    assert.deepEqual(await searchIds(ctx.repo, "登录"), ["task-a"]);
  } finally {
    await ctx.cleanup();
  }
});
