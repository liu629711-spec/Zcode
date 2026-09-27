import { join, resolve } from "node:path";

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
 * 记事本根目录：user 跟人走（<storageRoot>/agent-memory/<key>/），
 * project 跟项目走（<ws>/.zcode/agent-memory/<key>/），
 * local 跟机器走（<ws>/.zcode/agent-memory-local/<key>/）。
 */
export function resolveAgentMemoryRoot(input: {
  agentName: string;
  scope: AgentMemoryPathScope;
  storageRoot: string;
  workspaceRoot: string;
}): string {
  const agentKey = sanitizeAgentMemoryKey(input.agentName);
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
 * 档案改名迁移（G6）：改名后记事本目录跟着走，返回从旧目录到新目录的搬家对。
 * 触发条件是 sanitize 后的记忆 key 变化，不是档案文件路径变化——档案文件名存盘时
 * 小写化（`<name>.md`）而记忆 key 保大小写，『Alpha』改『alpha』文件路径不变、
 * key 变了，照文件路径触发会漏掉纯大小写改名。
 * 只做同 scope 同根的目录搬家；scope 变化（跨根迁移）是另一档语义，返回 undefined 不动。
 * 落点无法定位（user 档缺 userMemoryRoot、workspace/local 档缺 workspacePath）时同样
 * 返回 undefined——调用侧没有明确落点就不动目录。
 */
export function planAgentMemoryDirectoryRename(input: {
  previousAgentName: string;
  nextAgentName: string;
  memoryScope: AgentMemoryPathScope;
  userMemoryRoot?: string;
  workspacePath?: string;
}): AgentMemoryDirectoryRenamePlan | undefined {
  const previousKey = sanitizeAgentMemoryKey(input.previousAgentName);
  const nextKey = sanitizeAgentMemoryKey(input.nextAgentName);
  if (previousKey === nextKey) return undefined;
  if (input.memoryScope === "user") {
    const userMemoryRoot = input.userMemoryRoot?.trim();
    if (!userMemoryRoot) return undefined;
    return {
      fromDir: resolveAgentMemoryRoot({
        agentName: previousKey,
        scope: "user",
        storageRoot: userMemoryRoot,
        workspaceRoot: "",
      }),
      toDir: resolveAgentMemoryRoot({
        agentName: nextKey,
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
      agentName: previousKey,
      scope: input.memoryScope,
      storageRoot: "",
      workspaceRoot: workspacePath,
    }),
    toDir: resolveAgentMemoryRoot({
      agentName: nextKey,
      scope: input.memoryScope,
      storageRoot: "",
      workspaceRoot: workspacePath,
    }),
  };
}
