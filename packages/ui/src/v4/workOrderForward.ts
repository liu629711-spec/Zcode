/**
 * 派单回执的标题解析纯规则：从 CLI 权威铸造的回执标题里解员工名，
 * 供工地卡（agentWorkOrderBatch）与一键重派（ReceiptFailureNotice）共用。
 * 注意「转交」已于 2026-10-02 移除：成果接力归主流程（主心骨派单）与 @ 直派，
 * 这里不再有拼任务/选目标的转交专属规则。
 */

/**
 * 从回执标题里解出交活方的名字。标题由 CLI 权威铸造（buildWorkOrderReceiptTitle）：
 * completed =「<名字> 交活」，cancelled/failed =「<名字> 的工单被中断/未完成」。
 * 只有 completed（「交活」结尾）才返回名字——失败/中断的回执没有成果。
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
