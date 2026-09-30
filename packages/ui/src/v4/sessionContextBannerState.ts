/**
 * 上下文换班横幅的纯状态机（照 sessionQuotaBannerState 的先例：
 * 纯函数 + 可测试，组件只消费结果）。
 *
 * 产品语义（对齐稿 docs/ZCode-员工工作流与上下文换班-可行性分析.md §2.2/§2.4）：
 * - 用量占窗口的百分比达到阈值（缺省 40%，用户可设，0=关闭）就提示换班；
 * - 阈值必须远低于自动压缩线（约七八成），否则横幅永远轮不到出场；
 * - 「忽略」后的再次提醒节奏归 sessionContextBannerDismissalStore（按会话记忆），
 *   本文件只管"用量 × 阈值"这一层。
 */

export interface SessionContextBannerInput {
  usedTokens: number;
  maxTokens: number;
  /** 用户设置的阈值百分比；0 = 关闭；undefined = 产品默认 40。 */
  thresholdPercent: number | undefined;
}

export interface SessionContextBannerState {
  visible: boolean;
  /** 0-100 的整数用量百分比；数据不足时为 null（不可见）。 */
  percent: number | null;
  usedTokens: number;
  maxTokens: number;
}

/** 再次提醒的步进：用量比上次忽略时又涨了这么多百分点就重新提示。 */
export const SESSION_CONTEXT_BANNER_REMIND_STEP_PERCENT = 5;

/** 产品默认阈值（D4 拍板：缺省 40%，用户可在设置页调整）。 */
export const DEFAULT_CONTEXT_HANDOVER_THRESHOLD_PERCENT = 40;

export function resolveContextHandoverThresholdPercent(
  setting: number | undefined,
): number {
  if (setting === undefined || !Number.isFinite(setting)) {
    return DEFAULT_CONTEXT_HANDOVER_THRESHOLD_PERCENT;
  }
  if (setting <= 0) {
    // 0 与负数都按「关闭」处理；设置页不提供负值，这里兜底防御。
    return 0;
  }
  return Math.min(100, Math.max(1, Math.round(setting)));
}

export function buildSessionContextBannerState(
  input: SessionContextBannerInput,
): SessionContextBannerState {
  const percent = resolveContextUsagePercent(input.usedTokens, input.maxTokens);
  if (percent === null) {
    return {
      visible: false,
      percent: null,
      usedTokens: input.usedTokens,
      maxTokens: input.maxTokens,
    };
  }
  const threshold = resolveContextHandoverThresholdPercent(input.thresholdPercent);
  const visible = threshold > 0 && percent >= threshold;
  return { visible, percent, usedTokens: input.usedTokens, maxTokens: input.maxTokens };
}

/** 用量百分比（0-100 整数）；任一数据非正数（usage_update 的瞬时 used=0 假值）返回 null。 */
export function resolveContextUsagePercent(
  usedTokens: number,
  maxTokens: number,
): number | null {
  if (!Number.isFinite(usedTokens) || usedTokens <= 0) {
    return null;
  }
  if (!Number.isFinite(maxTokens) || maxTokens <= 0) {
    return null;
  }
  return Math.min(100, Math.max(1, Math.round((usedTokens / maxTokens) * 100)));
}
