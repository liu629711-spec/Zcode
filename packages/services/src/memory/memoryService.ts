import {
  AGENT_MEMORY_INDEX_FILE_NAME,
  type AgentMemoryCatalog,
  type AgentMemoryFileSummary,
  type AgentMemoryTargetParams,
  type IMemoryService,
  type ProjectMemoryFileSummary,
  type ProjectMemoryWorkspaceSummary,
} from "./memory.js";
import { lstat, readdir, realpath, rm, unlink } from "node:fs/promises";
import { basename, isAbsolute, join, relative, sep } from "node:path";
import { resolveAgentMemoryRoot, atomicWritePrivateTextFile } from "@zcode/shared/node";
import { readProjectMemoryFileFromStableHandle } from "#src/memory/projectMemoryStableRead.js";
import { getZCodeDataRootDir } from "#src/paths.js";

const PROJECT_MEMORY_INDEX_FILE_NAME = "MEMORY.md";
const PROJECT_MEMORY_DIRECTORY_NAME = "memory";
const PROJECT_KEY_SUFFIX_PATTERN = /^(.*)-[a-f0-9]{16}$/i;
/** 面板单条记忆的读写上限（与只读预览同一档）：记忆文件不该长成书。 */
const AGENT_MEMORY_MAX_BYTES = 5 * 1024 * 1024;

function isNotFoundError(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}

function getProjectMemoriesRoot(): string {
  return join(getZCodeDataRootDir(), "cli", "memories", "projects");
}

function isValidPathSegment(value: string): boolean {
  return (
    value.length > 0 &&
    value !== "." &&
    value !== ".." &&
    basename(value) === value &&
    !value.includes("/") &&
    !value.includes("\\")
  );
}

function isProjectMemoryFileName(fileName: string): boolean {
  return (
    fileName === PROJECT_MEMORY_INDEX_FILE_NAME ||
    (fileName.endsWith(".md") && fileName !== PROJECT_MEMORY_INDEX_FILE_NAME)
  );
}

function resolveWorkspaceLabel(workspaceId: string): string {
  const slug = PROJECT_KEY_SUFFIX_PATTERN.exec(workspaceId)?.[1];
  return slug?.trim() || workspaceId;
}

async function isPlainDirectory(path: string): Promise<boolean> {
  try {
    const metadata = await lstat(path);
    return metadata.isDirectory() && !metadata.isSymbolicLink();
  } catch (error) {
    if (isNotFoundError(error)) {
      return false;
    }
    throw error;
  }
}

async function requirePlainDirectory(path: string): Promise<void> {
  const metadata = await lstat(path);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new Error(`Project Memory directory is not a regular directory: ${path}`);
  }
}

async function requireProjectMemoriesRoot(): Promise<string> {
  const projectsRoot = getProjectMemoriesRoot();
  // 只校验 workspace 子目录时，projectsRoot symlink 会让 list/read 跟随到本地数据目录外。
  await requirePlainDirectory(projectsRoot);
  return projectsRoot;
}

async function requireExactProjectMemoryFile(
  memoryRoot: string,
  fileName: string,
): Promise<string> {
  const memoryEntries = await readdir(memoryRoot, { withFileTypes: true });
  const fileEntry = memoryEntries.find((entry) => entry.name === fileName);
  const requestedFilePath = join(memoryRoot, fileName);
  if (!fileEntry) {
    // 文件确实不存在时继续透传原始 ENOENT；只有大小写别名能命中时才拒绝读取。
    await lstat(requestedFilePath);
    throw new Error(`Project Memory file name does not match exactly: ${fileName}`);
  }
  if (!fileEntry.isFile() || fileEntry.isSymbolicLink()) {
    throw new Error(`Project Memory file is not a regular file: ${fileName}`);
  }
  return requestedFilePath;
}

async function assertContainedProjectMemoryPath(
  projectsRoot: string,
  targetPath: string,
): Promise<void> {
  const projectsRootRealPath = await realpath(projectsRoot);
  const targetRealPath = await realpath(targetPath);
  const relativePath = relative(projectsRootRealPath, targetRealPath);
  if (relativePath === ".." || relativePath.startsWith(`..${sep}`) || isAbsolute(relativePath)) {
    throw new Error(`Project Memory path is outside the local profile: ${targetPath}`);
  }
}

function compareProjectMemoryFiles(
  left: ProjectMemoryFileSummary,
  right: ProjectMemoryFileSummary,
): number {
  if (left.kind !== right.kind) {
    return left.kind === "index" ? -1 : 1;
  }
  return left.name.localeCompare(right.name, "en");
}

// ============================================================================
// 智能体记事本（G3 记忆面板）：目录推导与 core persistent-memory 同源
// （@zcode/shared/node resolveAgentMemoryRoot），只收 .md 文件；安全防护照
// 上方 Project Memory 的先例——普通目录、精确文件名、拒符号链接、realpath 越界检查。
// ============================================================================

function isAgentMemoryFileName(fileName: string): boolean {
  return (
    fileName === AGENT_MEMORY_INDEX_FILE_NAME ||
    (fileName.endsWith(".md") && fileName !== AGENT_MEMORY_INDEX_FILE_NAME)
  );
}

function requireAgentMemoryFileName(fileName: string): string {
  if (!isValidPathSegment(fileName) || !isAgentMemoryFileName(fileName)) {
    throw new Error(`Invalid agent memory file name: ${fileName}`);
  }
  return fileName;
}

/**
 * 记事本根目录：project/local 落在 <workspacePath>/.zcode 下；user 档案跟着账号走。
 * ponytail: user 根取 services 数据目录（缺省 ~/.zcode，与 CLI storage.dir 缺省一致）；
 * CLI 显式改过 storage.dir 时这里会漂移——面板随 catalog.rootDir 说真话，
 * 收口路径 = 有服务面回传 runtime memory.storageRoot 后替换这一行。
 */
function resolveAgentMemoryDirectory(params: AgentMemoryTargetParams): string {
  if (params.scope === "user") {
    return resolveAgentMemoryRoot({
      agentName: params.agentName,
      ...(params.agentId ? { agentId: params.agentId } : {}),
      scope: "user",
      storageRoot: getZCodeDataRootDir(),
      workspaceRoot: "",
    });
  }
  const workspacePath = params.workspacePath?.trim();
  if (!workspacePath) {
    throw new Error(`Agent memory scope ${params.scope} requires a workspace path`);
  }
  return resolveAgentMemoryRoot({
    agentName: params.agentName,
    ...(params.agentId ? { agentId: params.agentId } : {}),
    scope: params.scope,
    storageRoot: "",
    workspaceRoot: workspacePath,
  });
}

async function requireExactAgentMemoryFile(
  memoryRoot: string,
  fileName: string,
): Promise<string> {
  const memoryEntries = await readdir(memoryRoot, { withFileTypes: true });
  const fileEntry = memoryEntries.find((entry) => entry.name === fileName);
  const requestedFilePath = join(memoryRoot, fileName);
  if (!fileEntry) {
    // 文件确实不存在时继续透传原始 ENOENT；只有大小写别名能命中时才拒绝读取。
    await lstat(requestedFilePath);
    throw new Error(`Agent memory file name does not match exactly: ${fileName}`);
  }
  if (!fileEntry.isFile() || fileEntry.isSymbolicLink()) {
    throw new Error(`Agent memory file is not a regular file: ${fileName}`);
  }
  return requestedFilePath;
}

async function listAgentMemoryCatalog(
  params: AgentMemoryTargetParams,
): Promise<AgentMemoryCatalog> {
  const rootDir = resolveAgentMemoryDirectory(params);
  let memoryEntries;
  try {
    // 只校验单个固定根，symlink 根目录直接拒绝（同 requirePlainDirectory 先例）。
    await requirePlainDirectory(rootDir);
    memoryEntries = await readdir(rootDir, { withFileTypes: true });
  } catch (error) {
    if (isNotFoundError(error)) {
      // 记事本还没建（智能体还没记过任何东西）= 空集，不是错误。
      return { rootDir, scope: params.scope, files: [] };
    }
    throw error;
  }

  const files: AgentMemoryFileSummary[] = [];
  for (const memoryEntry of memoryEntries) {
    if (
      !memoryEntry.isFile() ||
      memoryEntry.isSymbolicLink() ||
      !isAgentMemoryFileName(memoryEntry.name)
    ) {
      continue;
    }
    const filePath = join(rootDir, memoryEntry.name);
    let fileMetadata;
    try {
      fileMetadata = await lstat(filePath);
    } catch (error) {
      // readdir 后事实文件可能被并发删除；它不再属于本次快照。
      if (isNotFoundError(error)) {
        continue;
      }
      throw error;
    }
    if (!fileMetadata.isFile() || fileMetadata.isSymbolicLink()) {
      continue;
    }
    files.push({
      name: memoryEntry.name,
      path: filePath,
      kind: memoryEntry.name === AGENT_MEMORY_INDEX_FILE_NAME ? "index" : "item",
      size: fileMetadata.size,
      updatedAt: fileMetadata.mtimeMs,
    });
  }

  files.sort(compareProjectMemoryFiles);
  return { rootDir, scope: params.scope, files };
}

async function readAgentMemoryFileEntry(
  params: AgentMemoryTargetParams & { fileName: string },
): Promise<{ content: string; updatedAt: number }> {
  const fileName = requireAgentMemoryFileName(params.fileName);
  const rootDir = resolveAgentMemoryDirectory(params);
  await requirePlainDirectory(rootDir);
  const filePath = await requireExactAgentMemoryFile(rootDir, fileName);
  return readProjectMemoryFileFromStableHandle({
    fileName,
    filePath,
    validatePath: async () => {
      await requirePlainDirectory(rootDir);
      await requireExactAgentMemoryFile(rootDir, fileName);
      await assertContainedProjectMemoryPath(rootDir, filePath);
    },
  });
}

async function writeAgentMemoryFileEntry(
  params: AgentMemoryTargetParams & { fileName: string; content: string },
): Promise<{ updatedAt: number }> {
  const fileName = requireAgentMemoryFileName(params.fileName);
  if (Buffer.byteLength(params.content, "utf-8") > AGENT_MEMORY_MAX_BYTES) {
    throw new Error(`Agent memory file exceeds the 5 MiB limit: ${fileName}`);
  }
  const rootDir = resolveAgentMemoryDirectory(params);
  await requirePlainDirectory(rootDir);
  const filePath = await requireExactAgentMemoryFile(rootDir, fileName);
  await assertContainedProjectMemoryPath(rootDir, filePath);
  // 临时文件 + rename 原子替换：写一半崩了也不会把记忆截断成空文件。
  await atomicWritePrivateTextFile(filePath, params.content);
  const finalStat = await lstat(filePath);
  return { updatedAt: finalStat.mtimeMs };
}

async function deleteAgentMemoryFileEntry(
  params: AgentMemoryTargetParams & { fileName: string },
): Promise<void> {
  const fileName = requireAgentMemoryFileName(params.fileName);
  const rootDir = resolveAgentMemoryDirectory(params);
  await requirePlainDirectory(rootDir);
  const filePath = await requireExactAgentMemoryFile(rootDir, fileName);
  await assertContainedProjectMemoryPath(rootDir, filePath);
  await unlink(filePath);
}

async function clearAgentMemoryFilesEntry(params: AgentMemoryTargetParams): Promise<void> {
  const rootDir = resolveAgentMemoryDirectory(params);
  try {
    // 清空 = 删整个 key 目录：只按面板文件清单（.md）清会留下员工写进来的
    // 非 .md 杂页。symlink 假目录在这里直接拒绝（同 requirePlainDirectory 先例）。
    await requirePlainDirectory(rootDir);
  } catch (error) {
    // 本来就还没记过任何东西：清空按幂等成功处理。
    if (isNotFoundError(error)) {
      return;
    }
    throw error;
  }
  // rm 对根路径本身不跟随 symlink；配合上面的普通目录校验，越界删除无从谈起。
  // maxRetries 兜 Windows 上与员工并发写相撞的 EPERM/ENOTEMPTY 瞬时失败（评审 A P2）。
  await rm(rootDir, { recursive: true, force: true, maxRetries: 3 });
}

export function createMemoryService(): IMemoryService {
  async function listProjectMemories(): Promise<ProjectMemoryWorkspaceSummary[]> {
    let projectsRoot: string;
    let projectEntries;
    try {
      projectsRoot = await requireProjectMemoriesRoot();
      projectEntries = await readdir(projectsRoot, { withFileTypes: true });
    } catch (error) {
      if (isNotFoundError(error)) {
        return [];
      }
      throw error;
    }

    const workspaces: ProjectMemoryWorkspaceSummary[] = [];
    for (const projectEntry of projectEntries) {
      if (!projectEntry.isDirectory() || projectEntry.isSymbolicLink()) {
        continue;
      }

      const workspaceId = projectEntry.name;
      const workspaceRoot = join(projectsRoot, workspaceId);
      const memoryRoot = join(workspaceRoot, PROJECT_MEMORY_DIRECTORY_NAME);
      if (!(await isPlainDirectory(workspaceRoot)) || !(await isPlainDirectory(memoryRoot))) {
        continue;
      }

      let memoryEntries;
      try {
        memoryEntries = await readdir(memoryRoot, { withFileTypes: true });
      } catch (error) {
        // 目录检查后 Memory Agent 仍可能删除目录；catalog 快照只跳过已消失的 workspace。
        if (isNotFoundError(error)) {
          continue;
        }
        throw error;
      }
      const files: ProjectMemoryFileSummary[] = [];
      for (const memoryEntry of memoryEntries) {
        if (
          !memoryEntry.isFile() ||
          memoryEntry.isSymbolicLink() ||
          !isProjectMemoryFileName(memoryEntry.name)
        ) {
          continue;
        }

        const filePath = join(memoryRoot, memoryEntry.name);
        let fileMetadata;
        try {
          fileMetadata = await lstat(filePath);
        } catch (error) {
          // readdir 后事实文件可能被并发删除；它不再属于本次只读快照。
          if (isNotFoundError(error)) {
            continue;
          }
          throw error;
        }
        if (!fileMetadata.isFile() || fileMetadata.isSymbolicLink()) {
          continue;
        }
        files.push({
          name: memoryEntry.name,
          path: filePath,
          kind: memoryEntry.name === PROJECT_MEMORY_INDEX_FILE_NAME ? "index" : "item",
          size: fileMetadata.size,
          updatedAt: fileMetadata.mtimeMs,
        });
      }

      if (files.length === 0) {
        continue;
      }

      files.sort(compareProjectMemoryFiles);
      workspaces.push({
        id: workspaceId,
        label: resolveWorkspaceLabel(workspaceId),
        updatedAt: Math.max(...files.map((file) => file.updatedAt)),
        files,
      });
    }

    workspaces.sort(
      (left, right) => right.updatedAt - left.updatedAt || left.id.localeCompare(right.id, "en"),
    );
    return workspaces;
  }

  async function readProjectMemoryFile(params: {
    workspaceId: string;
    fileName: string;
  }): Promise<{ content: string; updatedAt: number }> {
    if (
      !isValidPathSegment(params.workspaceId) ||
      !isValidPathSegment(params.fileName) ||
      !isProjectMemoryFileName(params.fileName)
    ) {
      throw new Error("Invalid Project Memory path");
    }

    const projectsRoot = await requireProjectMemoriesRoot();
    const workspaceRoot = join(projectsRoot, params.workspaceId);
    const memoryRoot = join(workspaceRoot, PROJECT_MEMORY_DIRECTORY_NAME);
    await requirePlainDirectory(workspaceRoot);
    await requirePlainDirectory(memoryRoot);

    // 大小写不敏感文件系统会让请求名称命中不同大小写的磁盘文件，绕过 catalog 白名单。
    const filePath = await requireExactProjectMemoryFile(memoryRoot, params.fileName);
    return readProjectMemoryFileFromStableHandle({
      fileName: params.fileName,
      filePath,
      validatePath: async () => {
        await requireProjectMemoriesRoot();
        await requirePlainDirectory(workspaceRoot);
        await requirePlainDirectory(memoryRoot);
        await requireExactProjectMemoryFile(memoryRoot, params.fileName);
        await assertContainedProjectMemoryPath(projectsRoot, filePath);
      },
    });
  }

  return {
    listProjectMemories,
    readProjectMemoryFile,
    listAgentMemoryFiles: listAgentMemoryCatalog,
    readAgentMemoryFile: readAgentMemoryFileEntry,
    writeAgentMemoryFile: writeAgentMemoryFileEntry,
    deleteAgentMemoryFile: deleteAgentMemoryFileEntry,
    clearAgentMemoryFiles: clearAgentMemoryFilesEntry,
  };
}
