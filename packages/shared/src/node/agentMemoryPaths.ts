import { join, resolve } from "node:path";
import { normalizeAgentId } from "../subagents-types.js";

/**
 * 智能体记事本（agent memory）目录推导的唯一实现。
 *
 * core 的记忆注入（apps/zcode-cli/packages/core/src/subagent/persistent-memory.ts）
 * 与 services 的记忆面板服务（packages/services/src/memory/memoryService.ts）都按
 * 这份推导落盘/读盘——两边各自手写一份必然漂移（漂移=面板显示空白而智能体在别处写）。
 * UI 侧的展示用提示（packages/ui buildAgentMemoryDirectoryHint）是纯字符串拼接，
 * 不引 node 模块，由 packages/ui/test 里的双向测试钉住与本实现的漂移。
 */

export type AgentMemoryPathScope = "user" | "project" | "local";

/** 记忆 key 只保留档案名里的安全字符；清完为空时用 unknown 兜底。 */
export function sanitizeAgentMemoryKey(agentName: string): string {
  const key = agentName.replace(/[^a-zA-Z0-9_-]/g, "-");
  return key === "" ? "unknown" : key;
}

/**
 * 记事本目录名（D26）：**有号按号，无号才按名字**。
 * 号是 uuid，目录名加 `a-` 段：既与"老档案按名字落的目录"物理区分（搬家时一眼
 * 认得出哪种是号目录），也隔开极端巧合——档案名字符集 [a-zA-Z0-9-]{3,50} 恰好
 * 容得下一个 uuid 字串。
 * 为什么值得动目录：名字目录的整类事故都是"改名后记忆找不到"（真机连出三次），
 * 号目录则改名永不移动。代价是目录名不再是人话——面板与删除提示都以员工名为标题、
 * 路径只作次要信息。
 */
export function resolveAgentMemoryKey(input: { agentName: string; agentId?: string }): string {
  const agentId = normalizeAgentId(input.agentId);
  return agentId ? `a-${agentId}` : sanitizeAgentMemoryKey(input.agentName);
}

/**
 * 记事本根目录：user 跟人走（<storageRoot>/agent-memory/<key>/），
 * project 跟项目走（<ws>/.zcode/agent-memory/<key>/），
 * local 跟机器走（<ws>/.zcode/agent-memory-local/<key>/）。
 */
export function resolveAgentMemoryRoot(input: {
  agentName: string;
  /** 员工工号（D26）：在场即按号落位，名字只作无号时的回落。 */
  agentId?: string;
  scope: AgentMemoryPathScope;
  storageRoot: string;
  workspaceRoot: string;
}): string {
  const agentKey = resolveAgentMemoryKey({
    agentName: input.agentName,
    ...(input.agentId ? { agentId: input.agentId } : {}),
  });
  if (input.scope === "user") {
    return join(input.storageRoot, "agent-memory", agentKey);
  }
  const workspace = resolve(input.workspaceRoot);
  return input.scope === "project"
    ? join(workspace, ".zcode", "agent-memory", agentKey)
    : join(workspace, ".zcode", "agent-memory-local", agentKey);
}

export interface AgentMemoryDirectoryRenamePlan {
  fromDir: string;
  toDir: string;
}

/**
 * 档案改名/上号时的记事本目录迁移（G6 + D26）：返回从旧目录到新目录的搬家对。
 * 触发条件是**记忆 key 变化**，不是档案名或文件路径变化：
 * - 老档案（无号）改名 → key 随名字变 → 搬家（G6 原语义）；
 * - 无号档案这次写入补上了号 → 从名字目录搬到号目录（一次性搬家，此后永不移动）；
 * - 有号档案改名 → key 不变 → 返回 undefined，目录原地不动（断链事故的正解）。
 * key 用 sanitize 后的记忆键比对，不用文件路径：档案文件名存盘时小写化
 * （`<name>.md`）而记忆 key 保大小写，『Alpha』改『alpha』文件路径不变、key 变了。
 * 只做同 scope 同根的搬家；scope 变化（跨根迁移）是另一档语义，返回 undefined 不动。
 * 落点无法定位（user 档缺 userMemoryRoot、workspace/local 档缺 workspacePath）时同样
 * 返回 undefined——调用侧没有明确落点就不动目录。
 */
export function planAgentMemoryDirectoryRename(input: {
  previousAgentName: string;
  nextAgentName: string;
  previousAgentId?: string;
  nextAgentId?: string;
  memoryScope: AgentMemoryPathScope;
  userMemoryRoot?: string;
  workspacePath?: string;
}): AgentMemoryDirectoryRenamePlan | undefined {
  const previous = {
    agentName: input.previousAgentName,
    ...(input.previousAgentId ? { agentId: input.previousAgentId } : {}),
  };
  const next = {
    agentName: input.nextAgentName,
    ...(input.nextAgentId ? { agentId: input.nextAgentId } : {}),
  };
  if (resolveAgentMemoryKey(previous) === resolveAgentMemoryKey(next)) {
    return undefined;
  }
  if (input.memoryScope === "user") {
    const userMemoryRoot = input.userMemoryRoot?.trim();
    if (!userMemoryRoot) return undefined;
    return {
      fromDir: resolveAgentMemoryRoot({
        ...previous,
        scope: "user",
        storageRoot: userMemoryRoot,
        workspaceRoot: "",
      }),
      toDir: resolveAgentMemoryRoot({
        ...next,
        scope: "user",
        storageRoot: userMemoryRoot,
        workspaceRoot: "",
      }),
    };
  }
  const workspacePath = input.workspacePath?.trim();
  if (!workspacePath) return undefined;
  return {
    fromDir: resolveAgentMemoryRoot({
      ...previous,
      scope: input.memoryScope,
      storageRoot: "",
      workspaceRoot: workspacePath,
    }),
    toDir: resolveAgentMemoryRoot({
      ...next,
      scope: input.memoryScope,
      storageRoot: "",
      workspaceRoot: workspacePath,
    }),
  };
}
