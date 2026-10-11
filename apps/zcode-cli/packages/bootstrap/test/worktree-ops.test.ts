// ============================================================
// 隔离仓 git 操作（任务隔离批）的可运行检查。
// 真实 git + 临时仓库驱动 worktree-ops.ts：
//  1. createTaskWorktree 在主仓 .git/zcode-worktrees/ 下开独立工作室，
//     员工在里面写文件不碰主工作区；
//  2. finalizeTaskWorktree 把改动提交并合并回主工作区、收走工作室；
//  3. 主工作区有未提交改动挡路 → 合并拒绝，工作室原样保留，note 带路径；
//  4. 没有改动的隔离单 → 不产生合并，工作室直接收走；
//  5. 员工自己 commit 过（分支尖端离开基准）→ 只看 status 会漏，照样合并。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/worktree-ops.test.ts
// ============================================================

import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  createTaskWorktree,
  finalizeTaskWorktree,
  seedTaskWorktreeMemory,
} from "../src/zcode-protocol/worktree-ops.js";

function git(cwd: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile("git", args, { cwd, windowsHide: true }, (error, stdout, stderr) => {
      if (error) {
        reject(new Error(`git ${args.join(" ")}: ${String(stderr || error.message)}`));
        return;
      }
      resolve(String(stdout));
    });
  });
}

async function makeMainRepo(): Promise<{ root: string; cleanup: () => Promise<void> }> {
  const root = await mkdtemp(join(tmpdir(), "zcode-worktree-ops-"));
  await git(root, ["init", "-b", "main"]);
  await git(root, ["config", "user.name", "ZCode Test"]);
  await git(root, ["config", "user.email", "zcode-test@example.com"]);
  await writeFile(join(root, "README.md"), "# main\n", "utf-8");
  await git(root, ["add", "-A"]);
  await git(root, ["commit", "-m", "init"]);
  return {
    root,
    cleanup: async () => {
      await rm(root, { recursive: true, force: true }).catch(() => undefined);
    },
  };
}

test("隔离单在独立工作室干活不碰主工作区；交活自动合并回来并收走工作室", async () => {
  const main = await makeMainRepo();
  try {
    const creation = await createTaskWorktree({ workspacePath: main.root, workOrderId: "wt-ok" });
    assert.ok(creation.worktreePath.includes(join(".git", "zcode-worktrees", "wt-ok")));
    assert.equal(creation.branch, "zcode-task-wt-ok");

    // 员工在工作室里干活：新文件 + 改主工作区同名的另一个文件不存在的路径。
    await writeFile(join(creation.sessionWorkspacePath, "feature.ts"), "export const x = 1;\n", "utf-8");
    // 主工作区文件原封不动。
    assert.equal(await readFile(join(main.root, "README.md"), "utf-8"), "# main\n");

    const finalize = await finalizeTaskWorktree(
      {
        path: creation.worktreePath,
        branch: creation.branch,
        workspacePath: main.root,
        baseCommit: creation.baseCommit,
      },
      { commitMessage: "zcode-task(wt-ok): feature" },
    );
    assert.match(finalize.note ?? "", /已自动合并/);
    // 合并落地：主工作区拿到员工文件（Windows autocrlf 检出会带 \r\n，归一化比对）；
    // 工作室与分支已收走。
    assert.equal(
      (await readFile(join(main.root, "feature.ts"), "utf-8")).replace(/\r\n/g, "\n"),
      "export const x = 1;\n",
    );
    await assert.rejects(stat(creation.worktreePath));
    const branches = await git(main.root, ["branch", "--list", creation.branch]);
    assert.equal(branches.trim(), "");
  } finally {
    await main.cleanup();
  }
});

test("主工作区有未保存改动挡路：合并拒绝，工作室原样保留并带路径", async () => {
  const main = await makeMainRepo();
  try {
    const creation = await createTaskWorktree({ workspacePath: main.root, workOrderId: "wt-block" });
    // 员工改 README.md；老板在主工作区也没保存地改了同一个文件。
    await writeFile(join(creation.sessionWorkspacePath, "README.md"), "# employee edit\n", "utf-8");
    await writeFile(join(main.root, "README.md"), "# boss dirty edit\n", "utf-8");

    const finalize = await finalizeTaskWorktree(
      {
        path: creation.worktreePath,
        branch: creation.branch,
        workspacePath: main.root,
        baseCommit: creation.baseCommit,
      },
      { commitMessage: "zcode-task(wt-block): edit readme" },
    );
    assert.match(finalize.note ?? "", /没能自动合回/);
    assert.match(finalize.note ?? "", /原样保留/);
    // 老板的未保存改动原样、员工的活原样（都在工作室里）。（\r\n 归一化：合并
    // 尝试时 git 会摸一遍文件换行，内容不动。）
    assert.equal(
      (await readFile(join(main.root, "README.md"), "utf-8")).replace(/\r\n/g, "\n"),
      "# boss dirty edit\n",
    );
    assert.equal(
      await readFile(join(creation.worktreePath, "README.md"), "utf-8"),
      "# employee edit\n",
    );
  } finally {
    await main.cleanup();
  }
});

test("没有改动的隔离单：不合并、工作室直接收走", async () => {
  const main = await makeMainRepo();
  try {
    const creation = await createTaskWorktree({ workspacePath: main.root, workOrderId: "wt-empty" });
    const finalize = await finalizeTaskWorktree(
      {
        path: creation.worktreePath,
        branch: creation.branch,
        workspacePath: main.root,
        baseCommit: creation.baseCommit,
      },
      { commitMessage: "zcode-task(wt-empty): nothing" },
    );
    assert.match(finalize.note ?? "", /没有改动任何文件/);
    await assert.rejects(stat(creation.worktreePath));
  } finally {
    await main.cleanup();
  }
});

test("员工自己 commit 过（分支离开基准提交）：status 干净也照样合并", async () => {
  const main = await makeMainRepo();
  try {
    const creation = await createTaskWorktree({ workspacePath: main.root, workOrderId: "wt-commit" });
    await writeFile(join(creation.sessionWorkspacePath, "own.ts"), "export const y = 2;\n", "utf-8");
    await git(creation.sessionWorkspacePath, ["add", "-A"]);
    await git(creation.sessionWorkspacePath, ["commit", "-m", "employee's own commit"]);

    const finalize = await finalizeTaskWorktree(
      {
        path: creation.worktreePath,
        branch: creation.branch,
        workspacePath: main.root,
        baseCommit: creation.baseCommit,
      },
      { commitMessage: "zcode-task(wt-commit): own" },
    );
    assert.match(finalize.note ?? "", /已自动合并/);
    assert.equal(
      (await readFile(join(main.root, "own.ts"), "utf-8")).replace(/\r\n/g, "\n"),
      "export const y = 2;\n",
    );
  } finally {
    await main.cleanup();
  }
});

test("工作区不是 git 仓库：开仓即拒，不建半截工作室", async () => {
  const plain = await mkdtemp(join(tmpdir(), "zcode-worktree-plain-"));
  try {
    await assert.rejects(
      createTaskWorktree({ workspacePath: plain, workOrderId: "wt-nogit" }),
      /git repository/,
    );
  } finally {
    await rm(plain, { recursive: true, force: true }).catch(() => undefined);
  }
});

test("记忆种子：主工作区的随身柜整目录拷进隔离仓", async () => {
  const main = await makeMainRepo();
  try {
    const notebook = join(main.root, ".zcode", "agent-memory", "a-1111");
    await mkdir(notebook, { recursive: true });
    await writeFile(join(notebook, "notes.md"), "老笔记\n", "utf-8");
    const creation = await createTaskWorktree({ workspacePath: main.root, workOrderId: "wt-mem" });
    await seedTaskWorktreeMemory({
      sourceWorkspacePath: main.root,
      targetWorkspacePath: creation.sessionWorkspacePath,
    });
    assert.equal(
      await readFile(join(creation.sessionWorkspacePath, ".zcode", "agent-memory", "a-1111", "notes.md"), "utf-8"),
      "老笔记\n",
    );
  } finally {
    await main.cleanup();
  }
});
