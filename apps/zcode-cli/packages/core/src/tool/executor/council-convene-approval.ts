import { COUNCIL_CONVENE_TOOL_NAME } from "@zcode/contracts";
import type { PermissionDecisionResult } from "../../permission/service.js";

/**
 * 圆桌会召集批准闸（2026-10-03 老板验收反馈）：召集必须老板当场点头。一场真会议
 * 是 N 席 × 至多两轮的跨会话模型调用，token 消耗量大；且「要不要开会」是老板的
 * 权力，不是模型的。工具元数据虽标了 needsApproval/alwaysAsk，但桌面执行链不消费
 * 它（真机实证：yolo 下一次调用就直接开了会）——所以在改判链上与员工桌子闸同款
 * 硬升 ask；deny / ask 的既有决定一律尊重，不二次改判。
 */
const COUNCIL_CONVENE_REASON =
  "召集圆桌会将派出全部席位、各自独立发言至多两轮，消耗大量 token——需要老板当场批准";

export function applyCouncilConveneApproval(input: {
  decision: PermissionDecisionResult;
  toolName: string;
}): PermissionDecisionResult {
  if (input.toolName !== COUNCIL_CONVENE_TOOL_NAME) return input.decision;
  // deny 一律尊重；ask 只换文案不换决定——工具自带的 alwaysAsk 会先给出英文通用
  // 理由（真机实证 2026-10-03：弹窗显示 "always requires explicit approval"，
  // token 警告被顶掉），老板要在批准框上直接看到代价。
  if (input.decision.decision === "deny") return input.decision;
  if (input.decision.ruleId === "guard.councilConvene") return input.decision;
  return {
    ...input.decision,
    allowed: false,
    decision: "ask",
    escalated: true,
    reason: COUNCIL_CONVENE_REASON,
    ruleId: "guard.councilConvene",
  };
}
