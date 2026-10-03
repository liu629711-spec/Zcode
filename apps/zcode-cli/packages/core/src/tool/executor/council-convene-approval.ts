import { COUNCIL_CONVENE_TOOL_NAME } from "@zcode/contracts";
import type { PermissionDecisionResult } from "../../permission/service.js";

/**
 * 圆桌会召集批准闸（2026-10-03 老板验收反馈）：召集必须老板当场点头。一场真会议
 * 是 N 席 × 至多两轮的跨会话模型调用，token 消耗量大；且「要不要开会」是老板的
 * 权力，不是模型的。工具元数据虽标了 needsApproval/alwaysAsk，但桌面执行链不消费
 * 它（真机实证：yolo 下一次调用就直接开了会）——所以在改判链上与员工桌子闸同款
 * 硬升 ask；deny / ask 的既有决定一律尊重，不二次改判。
 */
export function applyCouncilConveneApproval(input: {
  decision: PermissionDecisionResult;
  toolName: string;
}): PermissionDecisionResult {
  if (input.toolName !== COUNCIL_CONVENE_TOOL_NAME) return input.decision;
  if (input.decision.decision !== "allow") return input.decision;
  return {
    ...input.decision,
    allowed: false,
    decision: "ask",
    escalated: true,
    reason:
      "Convening a roundtable council will dispatch every seat for up to two rounds of model calls and cost many tokens - the boss must approve",
    ruleId: "guard.councilConvene",
  };
}
