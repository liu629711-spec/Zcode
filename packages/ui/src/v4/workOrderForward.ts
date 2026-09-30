/**
 * 派单回执卡的「转交」纯规则（2026-09-30 接力协作第一刀）。
 *
 * 转交 = 拿着回执里的成果原文，作为一张新工单派给下一位员工（施工交活 → 转质检评审、
 * PRD 定稿 → 转施工开工）。老板当调度员，链路复用既有的 AgentDispatch 派单管线；
 * 目标会话看不见发起方的回执，所以成果原文必须全文进任务。
 */

/**
 * 从回执标题里解出交活方的名字。标题由 CLI 权威铸造（buildWorkOrderReceiptTitle）：
 * completed =「<名字> 交活」，cancelled/failed =「<名字> 的工单被中断/未完成」。
 * 只有 completed（「交活」结尾）才返回名字——失败/中断的回执没有成果可转交。
 */
export function parseReceiptDelivererName(title: string): string | undefined {
  const trimmed = title.trim();
  const suffix = " 交活";
  if (!trimmed.endsWith(suffix) || trimmed.length <= suffix.length) {
    return undefined;
  }
  const name = trimmed.slice(0, -suffix.length).trim();
  return name.length > 0 ? name : undefined;
}

/** 转交目标 = 名册去掉交活方本人（转回给他等于把活弹回原地）。 */
export function selectForwardTargets<T extends { name: string }>(
  directory: readonly T[],
  delivererName: string | undefined,
): T[] {
  const excluded = delivererName?.trim().toLowerCase();
  return directory.filter((agent) => agent.name.trim().toLowerCase() !== excluded);
}

/**
 * 失败/中断回执标题里的员工名（2026-10-01 员工可靠性批）：failed =「<名字> 的工单
 * 未完成」，cancelled =「<名字> 的工单被中断」（buildWorkOrderReceiptTitle 权威铸造）。
 * 一键重派要靠它找到原员工；注意无名工位的标题种子是任务正文，解出的「名字」不一定
 * 在名册里——调用方必须拿名册对号后再放行重派按钮。
 */
export function parseFailedReceiptAgentName(title: string): string | undefined {
  const trimmed = title.trim();
  for (const suffix of [" 的工单未完成", " 的工单被中断"] as const) {
    if (trimmed.endsWith(suffix) && trimmed.length > suffix.length) {
      const name = trimmed.slice(0, -suffix.length).trim();
      if (name.length > 0) return name;
    }
  }
  return undefined;
}

/**
 * 转交任务的正文：成果原文全文进任务（目标会话看不见发起方的回执），老板附言
 * 可选；没有附言时明确「拿不准就问老板」，堵住弱模型自由发挥的口子。
 */
export function buildWorkOrderForwardTask(input: {
  delivererName: string;
  answer: string;
  note: string;
}): string {
  const answer = input.answer.trim();
  const note = input.note.trim();
  return [
    `老板转交：以下是同事 ${input.delivererName} 交来的工作成果，请在此基础上继续。`,
    "",
    "―― 成果原文 ――",
    answer.length > 0 ? answer : "（本轮没有留下文字成果）",
    "――――――",
    "",
    note.length > 0
      ? `老板的后续要求：${note}`
      : "老板没有写附加要求：先通读成果，拿不准下一步就问老板，不要自行发挥。",
  ].join("\n");
}
