import { join, resolve } from "node:path";

import type { FileSystemPort, Logger, TraceContext } from "@zcode/contracts";

import { ensureMemoryDirectoryExists } from "../memory/directory.js";
import type { AgentRuntimeConfig, MemoryRuntimeConfig } from "../runtime/types.js";
import type { AgentProfile, AgentMemoryScope } from "./profile.js";
import { buildPersistentAgentMemoryPrompt } from "./persistent-memory-prompt.js";

const PERSISTENT_MEMORY_TOOLS = ["Write", "Edit"] as const;

function sanitizePersistentAgentMemoryKey(agentName: string): string {
  const key = agentName.replace(/[^a-zA-Z0-9_-]/g, "-");
  return key === "" ? "unknown" : key;
}

function resolvePersistentAgentMemoryRoot(input: {
  agentName: string;
  scope: AgentMemoryScope;
  storageRoot: string;
  workspaceRoot: string;
}): string {
  const agentKey = sanitizePersistentAgentMemoryKey(input.agentName);
  if (input.scope === "user") {
    return join(input.storageRoot, "agent-memory", agentKey);
  }
  const workspace = resolve(input.workspaceRoot);
  return input.scope === "project"
    ? join(workspace, ".zcode", "agent-memory", agentKey)
    : join(workspace, ".zcode", "agent-memory-local", agentKey);
}

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

export function projectPersistentAgentMemoryTools(config: AgentRuntimeConfig): AgentRuntimeConfig {
  const memory = config.memory;
  const profiles = config.subagents?.profiles;
  if (!profiles || !isPersistentAgentMemoryEnabled(memory)) return config;

  const projected = profiles.map(withPersistentAgentMemoryTools);
  if (projected.every((profile, index) => profile === profiles[index])) return config;
  return {
    ...config,
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

  const rootDir = resolvePersistentAgentMemoryRoot({
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
  const rootDir = resolvePersistentAgentMemoryRoot({
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
