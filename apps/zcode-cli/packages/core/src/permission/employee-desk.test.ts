// ============================================================
// 员工桌子闸的可运行检查：纯判定（employee-desk.ts）+ 权限流调整器
// （employee-desk-permission.ts）。全程注入 path.win32 钉死 Windows 语义
// （盘符/反斜杠/大小写），不碰文件系统。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/permission/employee-desk.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { win32 } from "node:path";

import { isOutsideEmployeeDesksWrite } from "./employee-desk.js";
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
