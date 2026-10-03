/**
 * 圆桌会评审专区段组件（四段式拆出的两段，2026-10-03 圆桌会刀2 起共用）：
 * ① 局势条：状态灯 + 轮次 pill + 票数条（同意/打回/弃权比例，语义色）；
 *   专区弹层与圆桌画布顶部局势带共用这一段。
 * 主席结论段：收口结果卡口径的结论摘要正文。
 * （原专区弹层与控场条已由圆桌画布两态视图承接：刀2 把「打开会议」收敛为
 *   进入圆桌画布，抽屉复用本文件的段组件与裁决卡/过程流，弹层壳删除。）
 * 数据只来自 CouncilMeetingModel（纯函数聚合 originMeta.council*），这里只管画。
 */
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";
import {
  COUNCIL_STATUS_BADGE_CLASS,
  COUNCIL_STATUS_DOT_KEY,
} from "@/v4/councilMeetingVisuals.js";

// ── ① 局势条 ────────────────────────────────────────────────────────

export function CouncilStatusStrip({ meeting }: { meeting: CouncilMeetingModel }) {
  const { intl } = useZCodeIntl();
  const votes = meeting.tally;
  const total = votes.approve + votes.reject + votes.abstain;
  const segments = [
    { key: "approve" as const, count: votes.approve, className: "bg-success" },
    {
      key: "reject" as const,
      count: votes.reject,
      className: "bg-destructive",
    },
    {
      key: "abstain" as const,
      count: votes.abstain,
      className: "bg-foreground/25",
    },
  ];
  return (
    <section
      aria-label={intl.formatMessage({ id: "chat.council.section.status" })}
      className="rounded-xl border border-card-border bg-card/50 px-3.5 py-3"
      data-testid="council-zone-status"
    >
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span
          aria-hidden="true"
          className={cn(
            "size-1.5 rounded-full",
            STATUS_DOT[COUNCIL_STATUS_DOT_KEY[meeting.status]],
          )}
        />
        <span
          className={cn(
            "rounded-[4px] px-1.5 py-0.5 text-ui-2xs font-medium leading-4",
            COUNCIL_STATUS_BADGE_CLASS[meeting.status],
          )}
        >
          {intl.formatMessage({ id: `chat.council.status.${meeting.status}` })}
        </span>
        <span className="rounded-full border border-card-border px-2 py-0.5 text-ui-2xs text-foreground-subtle">
          {intl.formatMessage(
            { id: "chat.council.roundPill" },
            { round: String(meeting.round) },
          )}
        </span>
        <span className="min-w-0 flex-1" />
        {votes.vetoes > 0 ? (
          <span className="rounded-[4px] bg-destructive/10 px-1.5 py-0.5 text-ui-2xs font-medium text-destructive">
            {intl.formatMessage(
              { id: "chat.council.veto.note" },
              { count: String(votes.vetoes) },
            )}
          </span>
        ) : null}
      </div>
      <div className="mt-2.5 flex items-center gap-3">
        {/* 票数条：三段比例。无票时整条是空轨，绝不用假比例占位。 */}
        <div
          role="img"
          aria-label={
            total > 0
              ? intl.formatMessage(
                  { id: "chat.council.votes.barAria" },
                  {
                    approve: String(votes.approve),
                    reject: String(votes.reject),
                    abstain: String(votes.abstain),
                    total: String(total),
                  },
                )
              : intl.formatMessage({ id: "chat.council.votes.none" })
          }
          className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-muted"
        >
          {total > 0 ? (
            <div className="flex h-full">
              {segments.map((segment) =>
                segment.count > 0 ? (
                  <span
                    key={segment.key}
                    aria-hidden="true"
                    className={cn("h-full", segment.className)}
                    style={{ width: `${(segment.count / total) * 100}%` }}
                  />
                ) : null,
              )}
            </div>
          ) : null}
        </div>
        <span className="flex shrink-0 items-center gap-2 text-ui-2xs text-foreground-subtlest">
          {segments.map((segment) => (
            <span key={segment.key} className="flex items-center gap-1">
              <span
                aria-hidden="true"
                className={cn("size-1.5 rounded-full", segment.className)}
              />
              {intl.formatMessage({ id: `chat.council.votes.${segment.key}` })}
              <span className="font-mono tabular-nums">{segment.count}</span>
            </span>
          ))}
        </span>
      </div>
    </section>
  );
}

// ── 主席结论（收口结果卡口径：结论摘要；白名单收敛后时间线不再内联渲染）──

export function CouncilModeratorSummary({ meeting }: { meeting: CouncilMeetingModel }) {
  const { intl } = useZCodeIntl();
  const text = meeting.moderatorSummary?.text;
  if (!text) return null;
  return (
    <section
      aria-label={intl.formatMessage({ id: "chat.council.section.conclusion" })}
      className="rounded-xl border border-card-border bg-card px-3.5 py-3"
      data-testid="council-zone-conclusion"
    >
      <div className="text-ui-2xs font-medium text-foreground-subtlest">
        {intl.formatMessage({ id: "chat.council.section.conclusion" })}
      </div>
      <p className="mt-1.5 min-w-0 whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtle">
        {text}
      </p>
    </section>
  );
}
