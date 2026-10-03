/**
 * 圆桌会评审专区（四段式弹层）：一行摘要卡点开后的会议全貌。
 * ① 局面条：状态灯 + 轮次 pill + 票数条（同意/打回/弃权比例，语义色）——本文件；
 * ② 裁决卡：共识/分歧/盲区/建议下一步——见 CouncilMeetingVerdict.tsx；
 * ③ 过程流（默认折叠）：席位发言时间线——见 CouncilMeetingFlow.tsx；
 * ④ 控场条：暂停/恢复+插话（可@点名）——本文件。v4 命令面（shared
 *    zcode-protocol-v4）还没有圆桌会控场命令——按钮按红线给明确禁用态+tooltip
 *    说明，不做假按钮；命令接入后按宿主注入的能力收敛禁用态。
 * 数据只来自 CouncilMeetingModel（纯函数聚合 originMeta.council*），这里只管画。
 */
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog.js";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip.js";
import { CirclePause, CirclePlay, SendHorizontal } from "lucide-react";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";
import {
  COUNCIL_STATUS_BADGE_CLASS,
  COUNCIL_STATUS_DOT_KEY,
} from "@/v4/councilMeetingVisuals.js";
import { CouncilVerdictCard } from "@/v4/CouncilMeetingVerdict.js";
import { CouncilProcessFlow } from "@/v4/CouncilMeetingFlow.js";

// ── ① 局面条 ────────────────────────────────────────────────────────

function CouncilStatusStrip({ meeting }: { meeting: CouncilMeetingModel }) {
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

// ── ④ 控场条 ────────────────────────────────────────────────────────

/**
 * 控场能力（暂停/恢复/插话）在 v4 命令面（shared zcode-protocol-v4 command）里
 * 还没有对应命令：全部控件明确禁用 + tooltip 说明，绝不渲染点了没反应的假按钮。
 * 命令接入后：按钮 onClick 走宿主注入的命令回调，禁用态按能力缺席收敛。
 */
function CouncilControlBar() {
  const { intl } = useZCodeIntl();
  const unavailable = intl.formatMessage({
    id: "chat.council.controls.unavailable",
  });
  return (
    <TooltipProvider>
      <section
        aria-label={intl.formatMessage({ id: "chat.council.section.controls" })}
        className="flex flex-wrap items-center gap-2 border-t border-card-border/60 px-3.5 py-3"
        data-testid="council-zone-controls"
      >
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex">
              <Button type="button" variant="outline" size="sm" disabled>
                <CirclePause aria-hidden="true" className="size-3.5" />
                {intl.formatMessage({ id: "chat.council.controls.pause" })}
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent>{unavailable}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex">
              <Button type="button" variant="outline" size="sm" disabled>
                <CirclePlay aria-hidden="true" className="size-3.5" />
                {intl.formatMessage({ id: "chat.council.controls.resume" })}
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent>{unavailable}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex min-w-0 flex-1 basis-48">
              <Input
                type="text"
                disabled
                aria-label={intl.formatMessage({
                  id: "chat.council.controls.interjectAria",
                })}
                placeholder={intl.formatMessage({
                  id: "chat.council.controls.interjectPlaceholder",
                })}
                className="min-w-0 flex-1"
              />
            </span>
          </TooltipTrigger>
          <TooltipContent>{unavailable}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex">
              <Button type="button" variant="secondary" size="sm" disabled>
                <SendHorizontal aria-hidden="true" className="size-3.5" />
                {intl.formatMessage({ id: "chat.council.controls.send" })}
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent>{unavailable}</TooltipContent>
        </Tooltip>
      </section>
    </TooltipProvider>
  );
}

// ── 专区弹层 ─────────────────────────────────────────────────────────

export function CouncilMeetingZone({
  meeting,
  open,
  onOpenChange,
}: {
  meeting: CouncilMeetingModel;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { intl } = useZCodeIntl();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex max-w-2xl flex-col gap-0 p-0"
        data-testid="council-meeting-zone"
      >
        <DialogHeader className="gap-1.5 border-b border-card-border/60 px-4 py-3.5 pr-12 text-left">
          <DialogTitle className="flex items-center gap-2">
            {intl.formatMessage({ id: "chat.council.title" })}
            {meeting.kind ? (
              <span className="text-ui-sm font-normal text-foreground-subtlest">
                {intl.formatMessage({
                  id: `chat.council.kind.${meeting.kind}`,
                })}
              </span>
            ) : null}
          </DialogTitle>
          <DialogDescription>
            {intl.formatMessage({ id: "chat.council.zone.subtitle" })}
          </DialogDescription>
        </DialogHeader>
        <div className="flex max-h-[60vh] flex-col gap-3 overflow-y-auto px-4 py-3.5">
          <CouncilStatusStrip meeting={meeting} />
          <CouncilVerdictCard meeting={meeting} />
          <CouncilProcessFlow meeting={meeting} />
        </div>
        <CouncilControlBar />
      </DialogContent>
    </Dialog>
  );
}
