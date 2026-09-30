import { useEffect, useRef } from "react";
import { AlertTriangleIcon, ArrowRightLeftIcon, XIcon } from "lucide-react";
import { Button } from "@/components/ui/button.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import type { SessionContextBannerState } from "@/v4/sessionContextBannerState.js";

/**
 * 上下文换班横幅：用量达到阈值时提示「换个新会话继续」。
 * 形状照 ConversationQuotaBanner（会话流顶部一条，不挡对话），动作两个：
 * 主按钮=换班（生成交接单 → 开接班会话），次按钮=本次忽略。
 */
export function ConversationContextBanner({
  state,
  pending,
  onHandover,
  onDismiss,
  onShown,
}: {
  state: SessionContextBannerState;
  /** 换班流程进行中（交接单正在生成/落盘）：主按钮转 pending，防重复触发。 */
  pending: boolean;
  onHandover: () => void;
  onDismiss: () => void;
  onShown?: () => void;
}) {
  const { intl } = useZCodeIntl();
  const bannerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!onShown || !state.visible || !bannerRef.current) return;
    let intersecting = false;
    const report = () => {
      if (intersecting && document.visibilityState === "visible") onShown();
    };
    const observer = new IntersectionObserver(([entry]) => {
      intersecting = entry?.isIntersecting ?? false;
      report();
    });
    observer.observe(bannerRef.current);
    document.addEventListener("visibilitychange", report);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", report);
    };
  }, [onShown, state.visible]);
  if (!state.visible || state.percent === null) return null;

  return (
    <div
      ref={bannerRef}
      className="mb-3 w-full px-4 max-md:px-2"
      data-testid="v4-session-context-banner"
    >
      <div className="flex w-full flex-wrap items-center gap-2 rounded-xl border border-warning/30 bg-warning/10 px-3 py-2">
        <div className="flex min-w-0 flex-1 items-center gap-2 text-ui-base text-foreground">
          <AlertTriangleIcon className="size-4 shrink-0 text-warning" />
          <div className="min-w-0 break-words">
            {intl.formatMessage(
              { id: "chat.contextBanner.message" },
              {
                percent: state.percent,
                used: new Intl.NumberFormat().format(Math.max(0, state.usedTokens)),
                total: new Intl.NumberFormat().format(Math.max(0, state.maxTokens)),
              },
            )}
          </div>
        </div>
        <Button
          type="button"
          size="sm"
          disabled={pending}
          className="h-auto shrink-0 gap-1.5 rounded-full"
          onClick={onHandover}
          data-testid="v4-session-context-banner-handover"
        >
          <ArrowRightLeftIcon className="size-3.5" />
          {intl.formatMessage({ id: "chat.contextBanner.action.handover" })}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          onClick={onDismiss}
          aria-label={intl.formatMessage({ id: "common.close" })}
          className="shrink-0 rounded-full text-foreground-subtle hover:text-foreground"
        >
          <XIcon className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}
