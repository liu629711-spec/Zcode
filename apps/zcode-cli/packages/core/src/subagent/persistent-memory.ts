import { join } from "node:path";

import type { FileSystemPort, Logger, TraceContext } from "@zcode/contracts";
// 记事本目录推导的唯一实现（G3 面板同源）：core 注入与 services 面板都按它落盘/读盘。
import { resolveAgentMemoryRoot } from "@zcode/shared/node";

import type { AgentRuntimeConfig, MemoryRuntimeConfig } from "../runtime/types.js";
import type { AgentMemoryScope, AgentProfile } from "./profile.js";
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
    // 号在场按号落位（D26）：与 services 记忆面板同一判据，改名不换目录。
    ...(input.profile.agentId ? { agentId: input.profile.agentId } : {}),
    scope: input.profile.memory,
    storageRoot: input.memory.storageRoot,
    workspaceRoot: input.workspaceRoot,
  });
  // 读路径不得现造目录（D25）：目录由首次真实写入（Write createParents）落盘。
  // 曾在读取时 ensure 建目录——改过名的旧会话按快照旧名恢复，会把空目录造在
  // 旧 key 下，随后档案再改名时 services 的搬家因「目标已存在」跳过，记忆
  // 被留在旧目录里，面板看上去就是「记忆没了」。

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
 * 驻场智能体「主会话」的记忆注入：子代理派生在 methods/subagent.ts 调
 * loadPersistentAgentMemory；主会话没有对应入口，这里按同一读法在会话 context
 * 初始化时读一次 MEMORY.md 拼进 persona system prompt。G7 口径统一：
 * - scope 按 persona 携带的 memoryScope 读（不再写死 project），user 作用域走
 *   storageRoot 分支；
 * - 与子代理派生、工具面回填（projectPersistentAgentMemoryTools）共用同一个记忆
 *   总开关门控——关了就不注入也不再提示写记忆；真实语义是「不再注入不再提示」，
 *   不是目录只读（主会话仍持 Write/Edit 核心工具）。
 */
export async function loadProjectAgentMemoryPrompt(input: {
  fileSystemPort: FileSystemPort | undefined;
  agentName: string;
  /** 员工工号（D26）：persona 快照携带，在场即按号定位记事本。 */
  agentId?: string;
  memory: MemoryRuntimeConfig | undefined;
  memoryScope: AgentMemoryScope;
  logger?: Logger;
  traceContext?: TraceContext;
  workspaceRoot: string;
}): Promise<string | undefined> {
  if (!input.fileSystemPort) return undefined;
  if (!isPersistentAgentMemoryEnabled(input.memory)) return undefined;

  const rootDir = resolveAgentMemoryRoot({
    agentName: input.agentName,
    ...(input.agentId ? { agentId: input.agentId } : {}),
    scope: input.memoryScope,
    storageRoot: input.memory.storageRoot,
    workspaceRoot: input.workspaceRoot,
  });
  // 读路径不得现造目录（D25），理由同 loadPersistentAgentMemory：旧名会话恢复
  // 在这里造出的空目录，会挡住后续档案改名的记事本搬家。

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
    scope: input.memoryScope,
  });
}
