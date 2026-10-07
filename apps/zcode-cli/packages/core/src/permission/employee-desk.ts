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

// ============================================================
// 记忆柜读闸（拍板 2026-10-07）：员工读**别人**的本子改判 ask。本子里的复盘/
// 教训是员工间的私密账——看别人的要老板点头；自己的本子、工作区、柜外路径照旧
// 放行。与写闸同一纪律：纯函数、不碰文件系统、逐例可测。
// ============================================================

/** 读闸拦的读工具：单一 file_path（Read）/ path（Glob/Grep）参数可判。 */
const EMPLOYEE_CABINET_READ_GATED_TOOL_NAMES = new Set(["Read", "Glob", "Grep"]);

export interface EmployeeCabinetReadInput {
  toolName: string;
  /** 工具输入（尚未按具体工具 schema 解析）：Read 取 `file_path`，Glob/Grep 取 `path`。 */
  input: unknown;
  /** 员工自己的随身本子根（读自己的不拦）。 */
  personalNotebookRoot: string;
  /** 共享记忆柜根（所有员工本子的父目录）；缺席 = 无柜不拦（宁缺毋滥）。 */
  memoryCabinetRoot?: string;
  pathModule?: EmployeeDeskPathModule;
}

/**
 * 这次读取是否落在**别人的本子**里。相对路径一律不拦：本子根与柜根都是绝对路径
 * 注入，员工会话 cwd=工作区，相对路径到不了另一盘符下的柜（同盘下柜在用户主目录，
 * 工作区通常在项目目录——`..` 链能否穿越不赌，解析交给绝对路径判定）。
 */
export function isForeignMemoryCabinetRead(input: EmployeeCabinetReadInput): boolean {
  if (!EMPLOYEE_CABINET_READ_GATED_TOOL_NAMES.has(input.toolName)) return false;
  if (!input.memoryCabinetRoot) return false;
  if (!input.input || typeof input.input !== "object") return false;
  const record = input.input as Record<string, unknown>;
  const raw = record.file_path ?? record.path;
  if (typeof raw !== "string" || raw.trim().length === 0) return false;
  const path = input.pathModule ?? nodePath;
  if (!path.isAbsolute(raw)) return false;
  const resolved = path.resolve(raw);
  // Glob/Grep 的 path 可以就是柜根本身（搜**所有**本子）——目录相等也算在柜内。
  const inCabinet =
    resolved === input.memoryCabinetRoot || isInside(resolved, input.memoryCabinetRoot, path);
  if (!inCabinet) return false;
  if (isInside(resolved, input.personalNotebookRoot, path)) return false;
  return true;
}

/** contained = 非空、非绝对（跨盘符时 relative 给绝对路径）、不以 `..` 开头（穿越）。 */
function isInside(resolved: string, root: string, path: EmployeeDeskPathModule): boolean {
  const relativePath = path.relative(root, resolved);
  return (
    relativePath.length > 0 && !path.isAbsolute(relativePath) && !relativePath.startsWith("..")
  );
}

// ============================================================
// Bash 重定向逃逸检测（2026-10-04 对照 Open-ClaudeCode validateOutputRedirections
// 补齐，收掉员工桌子闸此前「只看 Write/Edit 的 file_path」的天花板）：
// 重定向目标按"创建文件"走与 Write/Edit 完全相同的两桌校验；解析不了的一律转人工
// （宁可多问）。全程不碰文件系统、不执行命令，纯字符串解析，逐例可测。
// ============================================================

/** 一条 Bash 命令里发现的「写不出桌/查不清」逃逸点。 */
export interface EmployeeDeskBashEscape {
  kind:
    | "redirect-outside" // 重定向目标解析后落在两桌之外
    | "redirect-opaque" // 目标带 $VAR/`cmd`/%VAR%/~，解析不了——宁可多问
    | "redirect-missing" // 重定向运算符后面没有目标（残缺命令）
    | "heredoc" // << / <<< 出现：正文可有任意字符，整条命令不可校验
    | "subshell" // 顶层括号（子 shell/命令替换/进程替换），目标归属不可信
    | "cd-with-redirect"; // cd 与重定向同条命令：相对目标归属随 cd 漂移
  detail: string;
}

export interface EmployeeDeskBashInput {
  command: string;
  /** 工作区根（员工的正式桌子）；空串 = 算不出桌子，不拦。 */
  workspaceRoot: string;
  /** 随身记事本根；缺席 = 只有工作区一张桌子。 */
  personalNotebookRoot?: string;
  pathModule?: EmployeeDeskPathModule;
}

interface BashToken {
  text: string;
  /** 引号内来的整段（里面的 > 不算运算符）。 */
  quoted: boolean;
  /** 重定向运算符本身：> >> >& &>。 */
  operator: boolean;
}

function isBashSpace(ch: string): boolean {
  return ch === " " || ch === "\t" || ch === "\n" || ch === "\r";
}

/**
 * 引号感知的 bash 分词：引号内一律是普通字符；顶层 > / >> / >& / &> 切成运算符
 * token（附着形式 `>file`、`2>file` 天然成立）；顶层括号与 heredoc 只记旗标——
 * 它们让整条命令的目标归属不可信，直接按"查不清"处理，不做半吊子解析。
 */
function tokenizeBashCommand(command: string): {
  tokens: BashToken[];
  sawTopLevelParen: boolean;
  sawHeredoc: boolean;
} {
  const tokens: BashToken[] = [];
  let current = "";
  let currentQuoted = false;
  let inSingle = false;
  let inDouble = false;
  let escaped = false;
  let sawTopLevelParen = false;
  let sawHeredoc = false;

  const flush = () => {
    if (current.length > 0) {
      tokens.push({ text: current, quoted: currentQuoted, operator: false });
    }
    current = "";
    currentQuoted = false;
  };

  for (let i = 0; i < command.length; i += 1) {
    const ch = command[i]!;
    if (escaped) {
      current += ch;
      escaped = false;
      continue;
    }
    if (ch === "\\" && !inSingle && !inDouble) {
      escaped = true;
      continue;
    }
    if (inSingle) {
      if (ch === "'") inSingle = false;
      else current += ch;
      continue;
    }
    if (inDouble) {
      if (ch === '"') inDouble = false;
      else current += ch;
      continue;
    }
    if (ch === "'") {
      flush();
      inSingle = true;
      currentQuoted = true;
      continue;
    }
    if (ch === '"') {
      flush();
      inDouble = true;
      currentQuoted = true;
      continue;
    }
    if (ch === "(" || ch === ")") {
      flush();
      sawTopLevelParen = true;
      continue;
    }
    if (ch === "<") {
      flush();
      if (command[i + 1] === "<") {
        sawHeredoc = true;
        i += 1;
        continue;
      }
      continue; // 单个 < 是输入重定向（读），不属于写闸管辖
    }
    if (ch === ">") {
      flush();
      let op = ">";
      while (command[i + 1] === ">") {
        op += ">";
        i += 1;
      }
      if (command[i + 1] === "&") {
        op += "&";
        i += 1;
      }
      tokens.push({ text: op, quoted: false, operator: true });
      continue;
    }
    if (ch === "&") {
      flush();
      if (command[i + 1] === ">") {
        tokens.push({ text: "&>", quoted: false, operator: true });
        i += 1;
        continue;
      }
      if (command[i + 1] === "&") i += 1; // && 命令分隔符
      continue; // 裸 & 后台符
    }
    if (ch === ";" || ch === "|") {
      flush();
      continue;
    }
    if (isBashSpace(ch)) {
      flush();
      continue;
    }
    current += ch;
  }
  flush();
  return { tokens, sawTopLevelParen, sawHeredoc };
}

/**
 * 这条 Bash 命令里有没有"写不出员工两桌/查不清"的点。工作区缺席（算不出桌子）
 * 一律返回空——宁缺毋滥，与 Write/Edit 判定同口径。
 * 已知天花板（ponytail，升级路径=参数级命令分析）：tee/cp/mv/dd/sed -i 这类
 * **参数即写目标**的命令仍不拦——本检测只管重定向与结构逃逸，与 Open-ClaudeCode
 * validateOutputRedirections 同一射程。
 */
export function findEmployeeDeskBashEscapes(input: EmployeeDeskBashInput): EmployeeDeskBashEscape[] {
  const escapes: EmployeeDeskBashEscape[] = [];
  if (typeof input.workspaceRoot !== "string" || input.workspaceRoot.length === 0) return escapes;
  const command = input.command;
  if (typeof command !== "string" || command.length === 0) return escapes;
  const path = input.pathModule ?? nodePath;
  const { tokens, sawTopLevelParen, sawHeredoc } = tokenizeBashCommand(command);
  if (sawHeredoc) {
    escapes.push({ kind: "heredoc", detail: "heredoc 正文不可校验" });
  }
  if (sawTopLevelParen) {
    escapes.push({ kind: "subshell", detail: "顶层括号（子 shell/命令替换/进程替换）" });
  }

  const hasCd = tokens.some((token) => !token.operator && !token.quoted && token.text === "cd");
  let sawRedirect = false;
  const pushedKinds = new Set<string>();
  const pushOnce = (escape: EmployeeDeskBashEscape) => {
    escapes.push(escape);
    pushedKinds.add(escape.kind);
  };

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index]!;
    if (!token.operator) continue;
    sawRedirect = true;
    const op = token.text;
    const target = tokens[index + 1];
    if (!target || target.operator) {
      pushOnce({ kind: "redirect-missing", detail: `${op} 后没有目标` });
      continue;
    }
    // fd 复制不是文件：>&2、>& 1、>&1（ attached 数字是描述符，非路径）。
    if (op === ">&" && /^\d+$/.test(target.text)) continue;
    if (target.text.startsWith("&") && /^\d+$/.test(target.text.slice(1))) continue;
    if (target.text === "/dev/null") continue; // 丢弃槽，Open-ClaudeCode 同款豁免
    if (/[$`%~]/.test(target.text)) {
      pushOnce({
        kind: "redirect-opaque",
        detail: `${op} ${target.text}：目标带变量/展开，解析不了`,
      });
      continue;
    }
    if (/^[A-Za-z]:[^\\/]/.test(target.text)) {
      // 盘符相对路径（bash 吃掉反斜杠后的 D:xxx）：归属取决于该盘的当前目录，解析不可信。
      pushOnce({
        kind: "redirect-opaque",
        detail: `${op} ${target.text}：盘符相对路径，归属不可信`,
      });
      continue;
    }
    const resolved = path.resolve(input.workspaceRoot, target.text);
    const insideWorkspace = isInside(resolved, input.workspaceRoot, path);
    const insideNotebook =
      input.personalNotebookRoot !== undefined &&
      isInside(resolved, input.personalNotebookRoot, path);
    if (!insideWorkspace && !insideNotebook) {
      pushOnce({
        kind: "redirect-outside",
        detail: `${op} ${target.text}：落在两桌之外`,
      });
    }
  }

  if (hasCd && sawRedirect) {
    pushOnce({
      kind: "cd-with-redirect",
      detail: "cd 与重定向同条命令，相对目标归属随 cd 漂移",
    });
  }
  return escapes;
}
