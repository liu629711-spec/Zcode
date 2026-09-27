import { join } from "node:path";

import type { FileSystemPort, Logger, TraceContext } from "@zcode/contracts";
// 记事本目录推导的唯一实现（G3 面板同源）：core 注入与 services 面板都按它落盘/读盘。
import { resolveAgentMemoryRoot } from "@zcode/shared/node";

import { ensureMemoryDirectoryExists } from "../memory/directory.js";
import type { AgentRuntimeConfig, MemoryRuntimeConfig } from "../runtime/types.js";
import type { AgentProfile } from "./profile.js";
import { buildPersistentAgentMemoryPrompt } from "./persistent-memory-prompt.js";

const PERSISTENT_MEMORY_TOOLS = ["Write", "Edit"] as const;

function isPersistentAgentMemoryEnabled(
  memory: MemoryRuntimeConfig | undefined,
): memory is MemoryRuntimeConfig & { storageRoot: string } {
  return memory?.enabled === true && memory.use !== false && Boolean(memory.storageRoot);
}

function withPersistentAgentMemoryTools(profile: AgentProfile): AgentProfile {
  if (!profile.memory || profile.tools === undefined) return profile;

  const tools = [...profile.tools];
  for (const tool of PERSISTENT_MEMORY_TOOLS) {
    if (!tools.includes(tool)) tools.push(tool);
  }
  return tools.length === profile.tools.length ? profile : { ...profile, tools };
}

/**
 * 驻场主会话的工具面回填（G2/D8）：persona 会话把档案 tools 白名单落到会话级
 * toolAllowlist 时，记忆写入端（Write/Edit）必须随身份在场——否则档案没勾
 * Write/Edit 的驻场智能体一开对话就永远写不了记忆（D6 写入端被工具面砍断）。
 * 语义与 withPersistentAgentMemoryTools（子代理派遣路径）一致：追加不覆盖；
 * 白名单缺席（继承全部工具）不动——Write/Edit 本来就在。调用侧保证只在
 * persona 带记忆 scope 且记忆总开关开启时进入。
 */
function withPersistentAgentMemorySessionToolAllowlist(
  toolAllowlist: readonly string[],
): readonly string[] {
  const tools = [...toolAllowlist];
  for (const tool of PERSISTENT_MEMORY_TOOLS) {
    if (!tools.includes(tool)) tools.push(tool);
  }
  return tools;
}

export function projectPersistentAgentMemoryTools(config: AgentRuntimeConfig): AgentRuntimeConfig {
  const memory = config.memory;
  const memoryEnabled = isPersistentAgentMemoryEnabled(memory);
  const profiles = config.subagents?.profiles;
  // 主会话（persona）侧：档案工具白名单在场且记忆总开关开启时，补齐记忆写入工具。
  // 普通（无 persona）会话与无白名单的 persona 会话都不进这个分支，行为不变。
  const sessionAllowlist =
    memoryEnabled && config.projectAgentPersona?.memory && config.toolAllowlist
      ? withPersistentAgentMemorySessionToolAllowlist(config.toolAllowlist)
      : config.toolAllowlist;
  const allowlistChanged = sessionAllowlist !== config.toolAllowlist;
  if (!profiles || !memoryEnabled) {
    return allowlistChanged ? { ...config, toolAllowlist: sessionAllowlist } : config;
  }

  const projected = profiles.map(withPersistentAgentMemoryTools);
  if (projected.every((profile, index) => profile === profiles[index])) {
    return allowlistChanged ? { ...config, toolAllowlist: sessionAllowlist } : config;
  }
  return {
    ...config,
    ...(allowlistChanged ? { toolAllowlist: sessionAllowlist } : {}),
    subagents: {
      ...config.subagents,
      profiles: projected,
    },
  };
}

interface PersistentAgentMemory {
  rootDir: string;
  prompt: string;
}

export async function loadPersistentAgentMemory(input: {
  fileSystemPort: FileSystemPort | undefined;
  memory: MemoryRuntimeConfig | undefined;
  logger?: Logger;
  profile: AgentProfile;
  traceContext?: TraceContext;
  workspaceRoot: string;
}): Promise<PersistentAgentMemory | undefined> {
  if (!input.profile.memory || !input.fileSystemPort) return undefined;
  if (!isPersistentAgentMemoryEnabled(input.memory)) return undefined;

  const rootDir = resolveAgentMemoryRoot({
    agentName: input.profile.name,
    scope: input.profile.memory,
    storageRoot: input.memory.storageRoot,
    workspaceRoot: input.workspaceRoot,
  });
  await ensureMemoryDirectoryExists(
    input.fileSystemPort,
    rootDir,
    input.traceContext,
    input.logger,
  );

  let indexContent = "";
  try {
    indexContent = (await input.fileSystemPort.readTextFile({ path: join(rootDir, "MEMORY.md") }))
      .content;
  } catch {
    // 不存在或不可读的 MEMORY.md 都使用基线的 empty index 文案。
  }

  return {
    rootDir,
    prompt: buildPersistentAgentMemoryPrompt({
      indexContent,
      rootDir,
      scope: input.profile.memory,
    }),
  };
}

/**
 * 驻场智能体「主会话」的项目记忆注入（新接的点）：子代理派生在 methods/subagent.ts
 * 调 loadPersistentAgentMemory；主会话没有对应入口，这里按同一读法只支持 project scope
 * （驻场智能体固定 project，见 projectAgentsModel.toProjectAgentCreateConfig），
 * 在会话 context 初始化时读一次 MEMORY.md 拼进 persona system prompt。
 */
export async function loadProjectAgentMemoryPrompt(input: {
  fileSystemPort: FileSystemPort | undefined;
  agentName: string;
  logger?: Logger;
  traceContext?: TraceContext;
  workspaceRoot: string;
}): Promise<string | undefined> {
  if (!input.fileSystemPort) return undefined;
  const rootDir = resolveAgentMemoryRoot({
    agentName: input.agentName,
    scope: "project",
    // 主会话读记忆不做 enabled 门控（与子代理不同：persona 会话由 Host 显式创建，
    // 记忆目录就是它的持久状态）；storageRoot 仅 user scope 用得到，这里传占位值。
    storageRoot: "",
    workspaceRoot: input.workspaceRoot,
  });
  await ensureMemoryDirectoryExists(
    input.fileSystemPort,
    rootDir,
    input.traceContext,
    input.logger,
  );

  let indexContent = "";
  try {
    indexContent = (await input.fileSystemPort.readTextFile({ path: join(rootDir, "MEMORY.md") }))
      .content;
  } catch {
    // 不存在或不可读的 MEMORY.md 都使用基线的 empty index 文案。
  }

  return buildPersistentAgentMemoryPrompt({
    indexContent,
    rootDir,
    scope: "project",
  });
}
