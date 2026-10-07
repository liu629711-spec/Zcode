// ============================================================
// 员工桌子闸的可运行检查：纯判定（employee-desk.ts）+ 权限流调整器
// （employee-desk-permission.ts）。全程注入 path.win32 钉死 Windows 语义
// （盘符/反斜杠/大小写），不碰文件系统。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/permission/employee-desk.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { win32 } from "node:path";

import {
  isForeignMemoryCabinetRead,
  isOutsideEmployeeDesksWrite,
} from "./employee-desk.js";
import { applyEmployeeDeskPermission } from "../tool/executor/employee-desk-permission.js";
import type { PermissionDecisionResult } from "./service.js";

const WORKSPACE = "D:\\Zcodetest";
const PERSONAL = "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-1";
const WIN = win32;

function outside(toolName: string, filePath: string, personalNotebookRoot?: string): boolean {
  return isOutsideEmployeeDesksWrite({
    toolName,
    input: { file_path: filePath },
    workspaceRoot: WORKSPACE,
    ...(personalNotebookRoot ? { personalNotebookRoot } : {}),
    pathModule: WIN,
  });
}

function allowDecision(ruleId = "mode.yolo"): PermissionDecisionResult {
  return {
    allowed: true,
    decision: "allow",
    escalated: false,
    mode: "yolo",
    ruleId,
    riskLevel: "medium",
  };
}

function adjust(
  filePath: string,
  decision: PermissionDecisionResult,
  personalNotebookRoot?: string,
): PermissionDecisionResult {
  return applyEmployeeDeskPermission({
    decision,
    executionInput: { file_path: filePath },
    toolName: "Write",
    workingDirectory: WORKSPACE,
    workspaceRoot: WORKSPACE,
    ...(personalNotebookRoot ? { desk: { personalNotebookRoot } } : {}),
  });
}

test("纯判定：工作区内的相对/绝对写入不算桌外", () => {
  assert.equal(outside("Write", "src\\app.ts"), false);
  assert.equal(outside("Write", "D:\\Zcodetest\\docs\\PRD.md"), false);
  // 工作区同名前缀目录不算桌内（D:\Zcodetest-other 不是桌子）。
  assert.equal(outside("Write", "D:\\Zcodetest-other\\x.md"), true);
});

test("纯判定：家目录绝对路径（真机事故原样）与相对路径逃逸都算桌外", () => {
  assert.equal(outside("Write", "C:\\Users\\22830\\PRD\\带校验的登录表单组件\\PRD-v1.0.md"), true);
  assert.equal(outside("Write", "..\\..\\escape.md"), true);
  // 跨盘符：relative 给绝对路径，必须判桌外。
  assert.equal(outside("Write", "E:\\elsewhere\\x.md"), true);
});

test("纯判定：随身本子内的写入不算桌外；其余 Write/Edit 之外的工具不归本闸管", () => {
  assert.equal(outside("Write", "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-1\\MEMORY.md", PERSONAL), false);
  assert.equal(outside("Edit", "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-1\\note.md", PERSONAL), false);
  assert.equal(outside("Write", "C:\\Users\\22830\\PRD\\x.md", PERSONAL), true);
  assert.equal(outside("Bash", "C:\\Users\\22830\\PRD\\x.md"), false, "Bash 不在闸内（天花板已注明）");
});

test("纯判定：工作区缺席（算不出桌子）一律不拦", () => {
  assert.equal(
    isOutsideEmployeeDesksWrite({
      toolName: "Write",
      input: { file_path: "C:\\Users\\22830\\PRD\\x.md" },
      workspaceRoot: "",
      pathModule: WIN,
    }),
    false,
  );
});

test("调整器：yolo 放行的桌外写入被改判 ask（压完全访问），桌内放行原样", () => {
  const outside = adjust("C:\\Users\\22830\\PRD\\PRD-v1.0.md", allowDecision(), PERSONAL);
  assert.equal(outside.decision, "ask");
  assert.equal(outside.allowed, false);
  assert.equal(outside.ruleId, "guard.employeeDesk");
  assert.equal(outside.escalated, true);

  const inside = adjust("src\\app.ts", allowDecision(), PERSONAL);
  assert.equal(inside.decision, "allow");
  assert.equal(inside.ruleId, "mode.yolo");
});

test("调整器：随身本子的记忆写入不被误拦（双层记忆的正当落点）", () => {
  const memoryWrite = adjust("C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-1\\MEMORY.md", allowDecision(), PERSONAL);
  assert.equal(memoryWrite.decision, "allow");
});

test("调整器：deny / ask 既有决定一律尊重，绝不二次改判", () => {
  const denied: PermissionDecisionResult = {
    ...allowDecision(),
    allowed: false,
    decision: "deny",
    ruleId: "tool.policyDenied",
  };
  assert.equal(adjust("C:\\Users\\22830\\PRD\\x.md", denied, PERSONAL).ruleId, "tool.policyDenied");

  const asked: PermissionDecisionResult = {
    ...allowDecision(),
    allowed: false,
    decision: "ask",
    ruleId: "rule.project.ask",
  };
  assert.equal(adjust("C:\\Users\\22830\\PRD\\x.md", asked, PERSONAL).ruleId, "rule.project.ask");
});

test("调整器：随身本子根缺席（非驻场员工/老板会话）闸整体不生效", () => {
  const result = adjust("C:\\Users\\22830\\PRD\\x.md", allowDecision());
  assert.equal(result.decision, "allow");
  assert.equal(result.ruleId, "mode.yolo");
});

// ── Bash 重定向逃逸检测（2026-10-04 对照 Open-ClaudeCode validateOutputRedirections）──

import {
  findEmployeeDeskBashEscapes,
} from "./employee-desk.js";

function bashEscapes(command: string, personalNotebookRoot?: string) {
  return findEmployeeDeskBashEscapes({
    command,
    workspaceRoot: WORKSPACE,
    ...(personalNotebookRoot ? { personalNotebookRoot } : {}),
    pathModule: WIN,
  });
}

function kinds(command: string, personalNotebookRoot?: string): string[] {
  return bashEscapes(command, personalNotebookRoot).map((escape) => escape.kind);
}

test("Bash 检测：桌内重定向放行，/dev/null 与 fd 复制豁免", () => {
  assert.deepEqual(kinds("echo hi > src\\out.txt"), []);
  assert.deepEqual(kinds("echo hi >> logs\\app.log"), []);
  assert.deepEqual(kinds("build 2> err.log"), []);
  assert.deepEqual(kinds("build > /dev/null 2>&1"), []);
  // 随身本子内的写入不算桌外（路径用正斜杠——反斜杠会被 bash 吃掉变盘符相对路径）。
  assert.deepEqual(kinds("echo hi > C:/Users/22830/.zcode/agent-memory/a-uuid-1/n.md", PERSONAL), []);
  // 引号里的 > 不是运算符；目标带引号照常解析。
  assert.deepEqual(kinds('echo "a > b" > src\\out.txt'), []);
  assert.deepEqual(kinds('echo hi > "src\\my file.txt"'), []);
  // 纯读、无重定向、cd 但无重定向：不归写闸管。
  assert.deepEqual(kinds("cat notes.md"), []);
  assert.deepEqual(kinds("cd src && ls"), []);
});

test("Bash 检测：桌外重定向（绝对/相对逃逸/追加/合并流）全部现形", () => {
  // 桌外路径用正斜杠写（Git Bash 的真实写法；反斜杠会被 bash 当转义符吃掉）。
  assert.deepEqual(kinds("echo x > C:/Users/22830/PRD/x.md"), ["redirect-outside"]);
  assert.deepEqual(kinds("echo x >> ../../escape.md"), ["redirect-outside"]);
  assert.deepEqual(kinds("build &> C:/Users/22830/all.log"), ["redirect-outside"]);
  assert.deepEqual(kinds("echo x > D:/Zcodetest-other/x.md"), ["redirect-outside"]);
  // 多条重定向命中一条即报；错误流单独落桌外也现形。
  assert.deepEqual(kinds("build > out.log 2> C:/Users/22830/err.log"), ["redirect-outside"]);
});

test("Bash 检测：查不清的四类——变量目标/盘符相对/heredoc/顶层括号——一律转人工", () => {
  assert.deepEqual(kinds("echo x > $HOME\\x.md"), ["redirect-opaque"]);
  assert.deepEqual(kinds("echo x > ~\\x.md"), ["redirect-opaque"]);
  // 反斜杠被 bash 吃掉后 D:\somewhere\x.md 变成盘符相对路径 D:somewherex.md——归属不可信。
  assert.deepEqual(kinds("echo x > D:\\somewhere\\x.md"), ["redirect-opaque"]);
  assert.equal(kinds("cat <<EOF\nhello > world\nEOF").includes("heredoc"), true);
  // `> >(tee …)`：第一看重定向的"目标"是进程替换本身——查不清 + 结构逃逸双报。
  assert.deepEqual(kinds("echo x > >(tee out.txt)"), ["subshell", "redirect-missing"]);
  assert.deepEqual(kinds("grep pat $(find .) > out.txt"), ["subshell"]);
});

test("Bash 检测：cd 与重定向同条命令 → 相对目标归属不可信", () => {
  assert.deepEqual(kinds("cd src && echo x > f.txt"), ["cd-with-redirect"]);
  // cd 走了人、重定向又落在桌外：两类逃逸同时现形。
  const both = kinds("cd src && echo x > C:/Users/22830/steal.md");
  assert.equal(both.includes("cd-with-redirect"), true);
  assert.equal(both.includes("redirect-outside"), true);
});

test("Bash 检测：运算符后缺目标按查不清处理；工作区缺席不拦", () => {
  assert.deepEqual(kinds("echo x >"), ["redirect-missing"]);
  assert.deepEqual(
    findEmployeeDeskBashEscapes({
      command: "echo x > C:\\Users\\22830\\PRD\\x.md",
      workspaceRoot: "",
      pathModule: WIN,
    }),
    [],
  );
});

function adjustBash(command: string, decision: PermissionDecisionResult, personalNotebookRoot?: string) {
  return applyEmployeeDeskPermission({
    decision,
    executionInput: { command },
    toolName: "Bash",
    workingDirectory: WORKSPACE,
    workspaceRoot: WORKSPACE,
    ...(personalNotebookRoot ? { desk: { personalNotebookRoot } } : {}),
  });
}

test("调整器：Bash 桌外重定向把 yolo 放行改判 ask（与 Write/Edit 同款护栏）", () => {
  const escaped = adjustBash("echo x > C:/Users/22830/PRD/x.md", allowDecision(), PERSONAL);
  assert.equal(escaped.decision, "ask");
  assert.equal(escaped.allowed, false);
  assert.equal(escaped.ruleId, "guard.employeeDesk");
  assert.equal(escaped.escalated, true);

  const inside = adjustBash("echo x > src\\out.txt", allowDecision(), PERSONAL);
  assert.equal(inside.decision, "allow");
  assert.equal(inside.ruleId, "mode.yolo");

  // 查不清（变量目标）同样转人工。
  const opaque = adjustBash("echo x > $HOME\\x.md", allowDecision(), PERSONAL);
  assert.equal(opaque.decision, "ask");

  // 老板会话（无本子）不闸；既有 deny/ask 尊重。
  assert.equal(
    adjustBash("echo x > C:/Users/22830/PRD/x.md", allowDecision()).decision,
    "allow",
  );
  const asked: PermissionDecisionResult = {
    ...allowDecision(),
    allowed: false,
    decision: "ask",
    ruleId: "rule.project.ask",
  };
  assert.equal(
    adjustBash("echo x > C:/Users/22830/PRD/x.md", asked, PERSONAL).ruleId,
    "rule.project.ask",
  );
});

// ============================================================
// 记忆柜读闸（拍板 2026-10-07）：读别人的本子改判 ask；自己的本子/工作区/柜外照旧。
// ============================================================

const CABINET = "C:\\Users\\22830\\.zcode\\agent-memory";

function foreignRead(
  toolName: string,
  rawPath: string,
  overrides?: { memoryCabinetRoot?: string },
): boolean {
  return isForeignMemoryCabinetRead({
    toolName,
    input: toolName === "Read" ? { file_path: rawPath } : { path: rawPath },
    personalNotebookRoot: PERSONAL,
    ...(overrides?.memoryCabinetRoot === undefined
      ? { memoryCabinetRoot: CABINET }
      : { memoryCabinetRoot: overrides.memoryCabinetRoot }),
    pathModule: WIN,
  });
}

test("读闸纯判定：别人的本子算串门，自己的本子/工作区/柜外/相对路径都不拦", () => {
  // 别人的本子（柜内、自己本子外）
  assert.equal(foreignRead("Read", "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-2\\MEMORY.md"), true);
  assert.equal(foreignRead("Glob", "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-2\\skills"), true);
  assert.equal(foreignRead("Grep", "C:\\Users\\22830\\.zcode\\agent-memory"), true);
  // 自己的本子
  assert.equal(foreignRead("Read", PERSONAL + "\\MEMORY.md"), false);
  // 工作区与柜外
  assert.equal(foreignRead("Read", WORKSPACE + "\\src\\a.ts"), false);
  assert.equal(foreignRead("Read", "C:\\Users\\22830\\PRD\\x.md"), false);
  // 相对路径不判（cwd=工作区语义，到不了另一盘符下的柜）
  assert.equal(foreignRead("Read", "..\\other\\MEMORY.md"), false);
  // 柜根缺席不拦（宁缺毋滥）
  assert.equal(
    foreignRead("Read", "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-2\\MEMORY.md", {
      memoryCabinetRoot: "",
    }),
    false,
  );
  // Write 不归读闸管（写闸另有判定）
  assert.equal(
    isForeignMemoryCabinetRead({
      toolName: "Write",
      input: { file_path: "C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-2\\x.md" },
      personalNotebookRoot: PERSONAL,
      memoryCabinetRoot: CABINET,
      pathModule: WIN,
    }),
    false,
  );
});

test("读闸调整器：yolo 放行的串门读被改判 ask；自己的本子放行原样", () => {
  const adjustRead = (filePath: string, deskMemoryCabinetRoot?: string) =>
    applyEmployeeDeskPermission({
      decision: allowDecision(),
      executionInput: { file_path: filePath },
      toolName: "Read",
      workingDirectory: WORKSPACE,
      workspaceRoot: WORKSPACE,
      desk: {
        personalNotebookRoot: PERSONAL,
        ...(deskMemoryCabinetRoot ? { memoryCabinetRoot: deskMemoryCabinetRoot } : {}),
      },
    });

  const foreign = adjustRead("C:\\Users\\22830\\.zcode\\agent-memory\\a-uuid-2\\MEMORY.md", CABINET);
  assert.equal(foreign.decision, "ask");
  assert.equal(foreign.ruleId, "guard.employeeDesk");

  const own = adjustRead(PERSONAL + "\\skills\\a.md", CABINET);
  assert.equal(own.decision, "allow");
});
