import type { PermissionDecisionResult } from "../../permission/service.js";
import { isOutsideEmployeeDesksWrite } from "../../permission/employee-desk.js";

interface EmployeeDeskPermissionInput {
  decision: PermissionDecisionResult;
  executionInput: unknown;
  toolName: string;
  workingDirectory: string;
  workspaceRoot: string;
  /**
   * 员工桌子（驻场员工会话才带）：personalNotebookRoot 是随身记事本根。**缺席 =
   * 非驻场会话（老板），闸整体不生效**——判空信号与判定参数同源，避免"没带参数"
   * 被误读成"没带本子"。
   */
  desk?: { personalNotebookRoot: string };
}

/**
 * 员工桌子闸（2026-10-02 真机实证后立）：驻场员工（persona 会话）的 Write/Edit
 * 落在「工作区 ∪ 随身记事本」之外时，把**放行改判为 ask**——包括 yolo 模式的
 * mode.yolo 直通与 session allow（真机事故同款教训：确认窗被逐次放行等于递梯子，
 * 但搬货出店这件事老板必须当场知情）。deny / ask / alwaysAsk 的既有决定一律尊重，
 * 不二次改判；memory.file.markdown 的自动放行也压不过本闸（它在调整链上游，
 * 且记忆柜本身就在桌子内）。
 *
 * 已知天花板（ponytail）：只看 Write/Edit 的 file_path；Bash 重定向写文件不走
 * 本闸（与 workflow 草稿免确认同一级天花板）——真机再见到绕道就给 Bash 加同款。
 */
export function applyEmployeeDeskPermission(
  input: EmployeeDeskPermissionInput,
): PermissionDecisionResult {
  const decision = input.decision;
  if (!input.desk) return decision;
  if (decision.decision !== "allow") return decision;
  if (decision.ruleId === "mode.plan.nonReadOnly") return decision;
  if (
    !isOutsideEmployeeDesksWrite({
      toolName: input.toolName,
      input: input.executionInput,
      workspaceRoot: input.workspaceRoot,
      personalNotebookRoot: input.desk.personalNotebookRoot,
    })
  ) {
    return decision;
  }
  return {
    ...decision,
    allowed: false,
    decision: "ask",
    escalated: true,
    reason:
      "Employee wrote outside its desks (the workspace and the personal notebook) - the boss must approve",
    ruleId: "guard.employeeDesk",
  };
}
