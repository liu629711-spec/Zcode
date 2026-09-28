import { ServiceChannels, type AgentMemoryScope } from "@zcode/shared";
import { createServiceDescriptor } from "../descriptors.js";

export const PROJECT_MEMORY_PREVIEW_LIMIT_EXCEEDED_ERROR_CODE =
  "PROJECT_MEMORY_PREVIEW_LIMIT_EXCEEDED";
export const PROJECT_MEMORY_FILE_CHANGED_ERROR_CODE = "PROJECT_MEMORY_FILE_CHANGED";

/** 智能体记事本的索引文件名（与 core persistent-memory 注入的 MEMORY.md 同一约定）。 */
export const AGENT_MEMORY_INDEX_FILE_NAME = "MEMORY.md";

export interface ProjectMemoryFileSummary {
  name: string;
  /** 已由 MemoryService 校验并限制在本地 Project Memory 根目录内的实际路径。 */
  path: string;
  kind: "index" | "item";
  size: number;
  updatedAt: number;
}

export interface ProjectMemoryWorkspaceSummary {
  id: string;
  label: string;
  updatedAt: number;
  files: ProjectMemoryFileSummary[];
}

export interface IMemoryService {
  /** 列出当前本地 profile 中可查看的 Project Memory。 */
  listProjectMemories(): Promise<ProjectMemoryWorkspaceSummary[]>;

  /** 原样读取一个 Project Memory Markdown 文件。 */
  readProjectMemoryFile(params: {
    workspaceId: string;
    fileName: string;
  }): Promise<{ content: string; updatedAt: number }>;

  /**
   * 列出驻场智能体记事本（G3 记忆面板）：<ws>/.zcode/agent-memory(-local)/<key>/ 或
   * user 档案的用户数据目录。只回 .md 文件快照；目录不存在 = 还没记过，回空集不报错。
   */
  listAgentMemoryFiles(params: AgentMemoryTargetParams): Promise<AgentMemoryCatalog>;

  /** 原样读取一条记忆（含 frontmatter 的 markdown 全文）。 */
  readAgentMemoryFile(
    params: AgentMemoryTargetParams & { fileName: string },
  ): Promise<{ content: string; updatedAt: number }>;

  /** 整文件覆盖写回一条已有记忆（面板只编辑列表里出现过的文件，不在这创建新文件）。 */
  writeAgentMemoryFile(
    params: AgentMemoryTargetParams & { fileName: string; content: string },
  ): Promise<{ updatedAt: number }>;

  /** 删除单条记忆文件（不可恢复；索引 MEMORY.md 被删后智能体下个会话会自己重建）。 */
  deleteAgentMemoryFile(params: AgentMemoryTargetParams & { fileName: string }): Promise<void>;
}

/** 记忆面板的目标定位：档案名 + 记忆范围 + 工作区（user 档案不需要工作区）。 */
export interface AgentMemoryTargetParams {
  agentName: string;
  /**
   * 员工工号（D26）：有号就按号定位记事本，与 core 的注入路径同一判据
   * （shared/node resolveAgentMemoryKey）。缺席 = 无号老档案，回落档案名。
   * 两边必须同源传，否则面板显示空白而智能体在另一个目录里写（面板/注入漂移）。
   */
  agentId?: string;
  scope: AgentMemoryScope;
  /** project/local 记事本在 <workspacePath>/.zcode 下，必传；user 记事本可省略。 */
  workspacePath?: string;
}

export interface AgentMemoryFileSummary {
  name: string;
  /** 已由 MemoryService 校验并限制在记事本根目录内的实际路径。 */
  path: string;
  kind: "index" | "item";
  size: number;
  updatedAt: number;
}

export interface AgentMemoryCatalog {
  /** 面板实际读取的记事本根目录——展示给用户看真话，user 档案的记事本不在项目里。 */
  rootDir: string;
  scope: AgentMemoryScope;
  files: AgentMemoryFileSummary[];
}

export const IMemoryService = createServiceDescriptor<IMemoryService>(ServiceChannels.Memory);
