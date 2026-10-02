// ============================================================
// Employee desk - 「这个写入目标在员工的桌子范围内吗」的纯判定
// ============================================================
//
// 与 workflow-draft-path 同一款纪律：PermissionService/权限流只需要一个布尔值，
// 判定本身必须能脱离执行器被逐例测到（含 Windows 盘符/大小写语义），全程只看
// 路径字符串、不碰文件系统——权限判定是纯函数，同一份输入在任何机器上同答案。
//
// 2026-10-02 真机实证：弱模型员工把 PRD 产出写进老板家目录的绝对路径（完全访问
// 模式下没有任何闸）。双层记忆拍板后员工的桌子=当前工作区 + 随身记事本；本判定
// 找出「往桌外写」的 Write/Edit，权限流据此把 yolo 的放行改判为 ask（见
// permission-flow 的 applyEmployeeDeskPermission）。

import nodePath from "node:path";

/**
 * 判定所需的 `node:path` 子集（测试注入 win32/posix 用），与 workflow-draft-path 同款。
 */
interface EmployeeDeskPathModule {
  isAbsolute: (path: string) => boolean;
  relative: (from: string, to: string) => string;
  resolve: (...paths: string[]) => string;
}

export interface EmployeeDeskInput {
  /** 工具输入里的写入目标，相对路径按工作区根解析，绝对路径原样保留。 */
  filePath: string;
  /** 工作区根（员工的正式桌子）；空串 = 算不出桌子，不拦（避免误伤，见下）。 */
  workspaceRoot: string;
  /** 随身记事本根；缺席 = 只有工作区一张桌子。 */
  personalNotebookRoot?: string;
  pathModule?: EmployeeDeskPathModule;
}

/** 员工桌子闸拦的写文件工具：与 workflow-draft 白名单同一对，单一 file_path 可判。 */
const EMPLOYEE_DESK_GATED_TOOL_NAMES = new Set(["Edit", "Write"]);

export interface EmployeeDeskWriteInput {
  toolName: string;
  /** 工具输入（尚未按具体工具 schema 解析），只从中取 `file_path`。 */
  input: unknown;
  workspaceRoot?: string;
  personalNotebookRoot?: string;
  pathModule?: EmployeeDeskPathModule;
}

/**
 * 这次写入是否落在员工的桌子**外**（= 权限流要把它改判成 ask）。
 * 工作区缺席时不拦：桌子算不出来就不能断言"写到了桌外"，宁缺毋滥。
 */
export function isOutsideEmployeeDesksWrite(input: EmployeeDeskWriteInput): boolean {
  if (!EMPLOYEE_DESK_GATED_TOOL_NAMES.has(input.toolName)) return false;
  if (typeof input.workspaceRoot !== "string" || input.workspaceRoot.length === 0) return false;
  if (!input.input || typeof input.input !== "object") return false;
  const filePath = (input.input as Record<string, unknown>).file_path;
  if (typeof filePath !== "string" || filePath.length === 0) return false;
  return isOutsideDesks({
    filePath,
    workspaceRoot: input.workspaceRoot,
    ...(input.personalNotebookRoot ? { personalNotebookRoot: input.personalNotebookRoot } : {}),
    ...(input.pathModule ? { pathModule: input.pathModule } : {}),
  });
}

/** 目标是否在「工作区 ∪ 随身记事本」之外。 */
function isOutsideDesks(input: EmployeeDeskInput): boolean {
  const path = input.pathModule ?? nodePath;
  const resolved = path.resolve(input.workspaceRoot, input.filePath);
  if (isInside(resolved, input.workspaceRoot, path)) return false;
  if (input.personalNotebookRoot && isInside(resolved, input.personalNotebookRoot, path)) {
    return false;
  }
  return true;
}

/** contained = 非空、非绝对（跨盘符时 relative 给绝对路径）、不以 `..` 开头（穿越）。 */
function isInside(resolved: string, root: string, path: EmployeeDeskPathModule): boolean {
  const relativePath = path.relative(root, resolved);
  return (
    relativePath.length > 0 && !path.isAbsolute(relativePath) && !relativePath.startsWith("..")
  );
}
