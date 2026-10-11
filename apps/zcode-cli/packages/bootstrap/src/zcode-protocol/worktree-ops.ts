// ============================================================
// 隔离仓 git 操作（任务隔离批 2026-10-11）
// ============================================================
// 隔离单 = 员工在主仓 `.git/zcode-worktrees/<workOrderId>` 下的独立 git worktree
// （"独立工作室"）里干活：主工作区文件一个不碰。交活（工单轮 completed 终态）后
// 收尾：提交全部改动 → 合并回主工作区 → 收走工作室；合不动（冲突/主区未保存改动
// 挡路）时工作室原样保留，路径随回执告知老板。
//
// 选址纪律：工作室放主仓 .git 内——git 永不把 .git 里的东西当工作区文件（git
// status 干净），且不在主工作区目录树里留可见杂物；findLatestPersonaSessionId
// 配套排除 `.git` 内的会话目录，隔离工位绝不参与工位复用。
// ponytail: 收尾只覆盖"回执链活着"的路径——进程在工单中途崩溃时 relay 随之消失，
// 工作室原样留在磁盘（改动都在，不丢数据），路径在派单台账 payload.envelope 里
// 可查；自动收尾的崩溃恢复（resume 清扫补收）留下一批。

import { execFile } from "node:child_process";
import { mkdir, cp } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import type { AgentWorkOrderWorktree } from "@zcode/contracts";

async function git(cwd: string, args: string[]): Promise<string> {
  return await new Promise((resolvePromise, reject) => {
    execFile(
      "git",
      args,
      { cwd, windowsHide: true, maxBuffer: 16 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error) {
          reject(
            new Error(
              `git ${args.join(" ")} failed: ${String(stderr || error.message).slice(0, 400)}`,
            ),
          );
          return;
        }
        resolvePromise(String(stdout));
      },
    );
  });
}

export interface TaskWorktreeCreation {
  /** 隔离仓根（git worktree 检出根）。 */
  worktreePath: string;
  /** 员工会话的工作区：主工作区在仓根时=worktreePath，在仓内子目录时=对应子目录。 */
  sessionWorkspacePath: string;
  branch: string;
  baseCommit: string;
}

/** 开一间隔离工作室：主仓 HEAD 起一根专用分支 + worktree。工作区不是 git 仓库即抛。 */
export async function createTaskWorktree(input: {
  workspacePath: string;
  workOrderId: string;
}): Promise<TaskWorktreeCreation> {
  // 主工作区可能是仓内子目录（rev-parse 回仓根）：员工工作区照搬相对位置，
  // 文件工具的相对路径语义与主工作区完全一致。
  let toplevel: string;
  try {
    toplevel = resolve((await git(input.workspacePath, ["rev-parse", "--show-toplevel"])).trim());
  } catch {
    throw new Error(
      "isolation=worktree requires the workspace to be inside a git repository, but this workspace is not one.",
    );
  }
  const branch = `zcode-task-${input.workOrderId}`;
  const worktreePath = join(toplevel, ".git", "zcode-worktrees", input.workOrderId);
  const baseCommit = (await git(toplevel, ["rev-parse", "HEAD"])).trim();
  await mkdir(dirname(worktreePath), { recursive: true });
  await git(toplevel, ["worktree", "add", "-b", branch, worktreePath, baseCommit]);
  const rel = relative(toplevel, resolve(input.workspacePath));
  const sessionWorkspacePath = rel === "" ? worktreePath : join(worktreePath, rel);
  return { worktreePath, sessionWorkspacePath, branch, baseCommit };
}

/**
 * 记忆种子（学习沉淀连续性）：project 作用域员工的随身柜住在 `<工作区>/.zcode/
 * agent-memory/<工号>`——隔离仓是主仓 HEAD 的检出，未提交的记忆不在里面。开仓时
 * 把主工作区的记忆目录整目录拷进隔离仓对应位置（best-effort，失败不挡派单）：
 * 员工读得到旧笔记；收尾合并时同内容文件不产生冲突。
 */
export async function seedTaskWorktreeMemory(input: {
  sourceWorkspacePath: string;
  targetWorkspacePath: string;
}): Promise<void> {
  const from = join(input.sourceWorkspacePath, ".zcode", "agent-memory");
  const to = join(input.targetWorkspacePath, ".zcode", "agent-memory");
  await cp(from, to, { recursive: true, force: true, errorOnExist: false }).catch(() => undefined);
}

export interface TaskWorktreeFinalizeResult {
  /** 随回执告知老板的一行大白话；没有可说的事项时缺席。 */
  note?: string;
}

/** 收尾一间已完成工单的隔离工作室：提交 → 合并 → 收走。合不动时保留现场只带话。 */
export async function finalizeTaskWorktree(
  worktree: AgentWorkOrderWorktree,
  options: { commitMessage: string },
): Promise<TaskWorktreeFinalizeResult> {
  const status = (await git(worktree.path, ["status", "--porcelain"])).trim();
  const tip = (await git(worktree.path, ["rev-parse", "HEAD"])).trim();
  const hasUncommitted = status.length > 0;
  // 员工有 git 工具：改完自己 commit 的活不会出现在 status 里，比对基准提交才不漏。
  const hasOwnCommits = tip !== worktree.baseCommit;
  if (!hasUncommitted && !hasOwnCommits) {
    const removed = await removeTaskWorktreeQuietly(worktree);
    return {
      note: removed
        ? "本单开在隔离工作室，没有改动任何文件，工作室已收走。"
        : "本单开在隔离工作室，没有改动任何文件；工作室收走失败，可手动删除。",
    };
  }
  if (hasUncommitted) {
    await git(worktree.path, ["add", "-A"]);
    await git(worktree.path, ["commit", "-m", options.commitMessage]);
  }
  try {
    await git(worktree.workspacePath, ["merge", "--no-ff", "--no-edit", worktree.branch]);
  } catch {
    return {
      note: `隔离工作室的改动没能自动合回主工作区（可能你有没保存的改动挡了路，或两边改到了同一处）。工作室原样保留在 ${worktree.path}（分支 ${worktree.branch}），处理完可以让员工重试合并，或自己手动合并。`,
    };
  }
  const removed = await removeTaskWorktreeQuietly(worktree);
  return {
    note: removed
      ? "本单在隔离工作室完成，改动已自动合并回主工作区。"
      : `本单在隔离工作室完成，改动已自动合并回主工作区；工作室目录没清干净，可手动删除：${worktree.path}`,
  };
}

/** 收工作室 + 删分支：尽力而为，失败不挡回执（内容已安全落地主仓或原样保留）。 */
async function removeTaskWorktreeQuietly(worktree: AgentWorkOrderWorktree): Promise<boolean> {
  try {
    await git(worktree.workspacePath, ["worktree", "remove", "--force", worktree.path]);
    await git(worktree.workspacePath, ["branch", "-D", worktree.branch]);
    return true;
  } catch {
    return false;
  }
}
