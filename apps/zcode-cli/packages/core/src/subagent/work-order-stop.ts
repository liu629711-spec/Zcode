// ============================================================
// 工单停机守卫（2026-10-04，Hermes kanban_stop.py 同款）：弱模型员工收单后
// 光说不练（只回确认/寒暄、零工具调用）是反复真机事故。信封的三段纪律是
// 提示词层；本模块是机制层——轮收尾时代码判定"这算不算交付"，不算就补射
// 一次提醒（hermes 的 nudge），再犯直接判轮失败走既有失败链（回执失败卡 +
// 发起方自动重试一棒，重试再犯同样在此收口，有界）。
// 参照：hermes-agent/agent/kanban_stop.py（"A plain-text reply is NOT a
// terminal state"，2 次机会后 protocol_violation）。
// 评审单豁免：评审的产出就是意见本身，只评审不动手是纪律不是病。
// ============================================================

/** 补射提醒（进模型的 system-reminder 附件；给足逃生口——纯文本任务重申即可）。 */
export const WORK_ORDER_NUDGE_TEXT =
  "停机检查：你上一条回复只是确认/寒暄，没有动手也没有交付。" +
  "工单即任务——现在立即执行任务本身，完成后直接给出成果。" +
  "如果你的上一条已经是完整成果，请以「交付：」开头把完整成果重新给出；" +
  "除此之外不要回复任何确认、表态或说明。";

/** 两轮都光说不练时抛出的轮失败原因（原样进发起方的回执失败卡，说人话）。 */
export const WORK_ORDER_NO_ACTION_ERROR =
  "工单未执行：提醒后员工仍未动手（两轮都只有确认/寒暄，没有实际动作或交付）。可打回重派。";

/**
 * 寒暄词表（2026-10-02 两连修沉淀的观测集：两个本地弱模型的首轮真实回样）。
 * 判定口径：去空白/标点/符号后 ≤30 字且含任一寒暄词 → 算寒暄；空回复 → 算寒暄。
 * 30 字上限防误伤：真正的纯文本成果很少这么短还带确认词。
 */
const ACK_LEXICON = [
  "在的",
  "收到",
  "好的",
  "好嘞",
  "嗯",
  "哦",
  "ok",
  "okay",
  "了解",
  "明白",
  "就位",
  "准备好了",
  "随时",
  "开工",
  "开始吧",
  "这就去",
  "马上来",
] as const;

/** 寒暄判定的字符上界（归一化后）。 */
const ACK_MAX_CHARS = 30;

export function isAckOnlyWorkOrderReply(modelResponse: string): boolean {
  const normalized = modelResponse
    .toLowerCase()
    .replace(/[\s\p{P}\p{S}]/gu, "");
  if (normalized.length === 0) return true;
  if (normalized.length > ACK_MAX_CHARS) return false;
  return ACK_LEXICON.some((word) => normalized.includes(word));
}

export type WorkOrderStopVerdict = "deliver" | "nudge" | "fail";

/**
 * 工单轮收尾裁决：deliver=放行走正常回执；nudge=补射一次提醒后同轮续跑；
 * fail=两轮都光说不练，抛错走失败链。动过手（有任何工具调用）一律算交付；
 * 评审单豁免。
 */
export function assessWorkOrderStop(input: {
  modelResponse: string;
  toolCallCount: number;
  review?: boolean;
  alreadyNudged?: boolean;
}): WorkOrderStopVerdict {
  if (input.review) return "deliver";
  if (input.toolCallCount > 0) return "deliver";
  if (!isAckOnlyWorkOrderReply(input.modelResponse)) return "deliver";
  return input.alreadyNudged ? "fail" : "nudge";
}
