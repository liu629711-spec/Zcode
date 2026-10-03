/**
 * 圆桌画布的共用视觉段（对照 councilMeetingVisuals 的原子层）：桌面发言气泡、
 * 收口裁决石、底栏控场条。与画布布局（CouncilTableStage.tsx）分离。
 * 数据只来自 CouncilMeetingModel 与 councilStageModel 纯函数，这里只管画。
 */
import { Bell, CirclePause, SendHorizontal } from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip.js";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";
import {
  AgentChip,
  COUNCIL_STATUS_BADGE_CLASS,
} from "@/v4/councilMeetingVisuals.js";
import {
  councilStoneConclusionLine,
  isCouncilMeetingClosed,
  type CouncilStageBubble,
} from "@/v4/councilStageModel.js";

export function CouncilStageBubbleView({
  bubble,
  variant,
  onLocate,
}: {
  bubble: CouncilStageBubble;
  variant: "current" | "previous";
  onLocate: (entryKey: string) => void;
}) {
  const { intl } = useZCodeIntl();
  return (
    <button
      type="button"
      onClick={() => onLocate(bubble.entryKey)}
      aria-label={intl.formatMessage(
        { id: "chat.council.stage.bubble.openAria" },
        { name: bubble.agentName || "—" },
      )}
      data-testid={`council-stage-bubble-${variant}`}
      className={cn(
        "w-full cursor-pointer rounded-xl border bg-card px-3 py-2 text-left font-[inherit] shadow-xs transition-colors hover:bg-surface-hover/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30",
        variant === "current" ? "border-brand/30" : "border-card-border/60",
      )}
    >
      <span className="flex min-w-0 items-center gap-1.5">
        <AgentChip name={bubble.agentName} />
        <span className="shrink-0 text-ui-2xs text-foreground-subtlest">
          {intl.formatMessage({
            id:
              variant === "current"
                ? "chat.council.stage.bubble.latest"
                : "chat.council.stage.bubble.previous",
          })}
        </span>
      </span>
      <span
        className={cn(
          "mt-1 block whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtle",
          variant === "current" ? "line-clamp-4" : "line-clamp-2",
        )}
      >
        {bubble.text}
      </span>
    </button>
  );
}

/** 裁决石（收口态桌心）：结论一行+票型；只读——控场在底栏按禁用态呈现。 */
export function CouncilVerdictStone({ meeting }: { meeting: CouncilMeetingModel }) {
  const { intl } = useZCodeIntl();
  const line = councilStoneConclusionLine(meeting);
  const votes = meeting.tally;
  return (
    <div
      role="status"
      aria-label={intl.formatMessage({ id: "chat.council.section.conclusion" })}
      data-testid="council-stage-stone"
      className="w-full rounded-2xl border border-brand/25 bg-card px-4 py-3 text-center shadow-sm"
    >
      <div className="text-ui-2xs font-medium text-foreground-subtlest">
        {intl.formatMessage({ id: "chat.council.section.conclusion" })}
      </div>
      <p className="mt-1 line-clamp-3 whitespace-pre-wrap break-words text-ui-sm leading-5 text-foreground">
        {line ?? intl.formatMessage({ id: `chat.council.status.${meeting.status}` })}
      </p>
      <div className="mt-2 flex flex-wrap items-center justify-center gap-2 text-ui-2xs text-foreground-subtlest">
        <span className="rounded-full border border-card-border px-2 py-0.5">
          {intl.formatMessage(
            { id: "chat.council.roundPill" },
            { round: String(meeting.round) },
          )}
        </span>
        <span className="flex items-center gap-2 font-mono tabular-nums">
          <span className="flex items-center gap-1">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-success" />
            {votes.approve}
          </span>
          <span className="flex items-center gap-1">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-destructive" />
            {votes.reject}
          </span>
          <span className="flex items-center gap-1">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-foreground/25" />
            {votes.abstain}
          </span>
        </span>
        {votes.vetoes > 0 ? (
          <span className="rounded-[4px] bg-destructive/10 px-1.5 py-0.5 font-medium text-destructive">
            {intl.formatMessage(
              { id: "chat.council.veto.note" },
              { count: String(votes.vetoes) },
            )}
          </span>
        ) : null}
      </div>
    </div>
  );
}

/**
 * 底栏控场条：暂停/催一句/插话。控场能力在 v4 命令面（shared
 * zcode-protocol-v4 command）里没有对应命令：全部控件明确禁用 + tooltip 说明，
 * 绝不渲染点了没反应的假按钮（刀1 专区同口径）。命令接入后按宿主注入的能力
 * 收敛禁用态，收口态保持只读。右侧=票数速览+收口徽章。
 */
export function CouncilStageControlBar({ meeting }: { meeting: CouncilMeetingModel }) {
  const { intl } = useZCodeIntl();
  const unavailable = intl.formatMessage({
    id: "chat.council.controls.unavailable",
  });
  const votes = meeting.tally;
  const closed = isCouncilMeetingClosed(meeting);
  return (
    <TooltipProvider>
      <section
        aria-label={intl.formatMessage({ id: "chat.council.section.controls" })}
        className="flex flex-wrap items-center gap-2 border-t border-card-border/60 px-3.5 py-2.5"
        data-testid="council-stage-controls"
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
                <Bell aria-hidden="true" className="size-3.5" />
                {intl.formatMessage({ id: "chat.council.stage.urge" })}
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
        <span className="min-w-0 flex-1" />
        {closed ? (
          <span
            className={cn(
              "shrink-0 rounded-[4px] px-1.5 py-0.5 text-ui-2xs font-medium leading-4",
              COUNCIL_STATUS_BADGE_CLASS[meeting.status],
            )}
          >
            {intl.formatMessage({ id: `chat.council.status.${meeting.status}` })}
          </span>
        ) : null}
        <span className="flex shrink-0 items-center gap-2 text-ui-2xs text-foreground-subtlest">
          <span className="flex items-center gap-1">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-success" />
            {intl.formatMessage({ id: "chat.council.votes.approve" })}
            <span className="font-mono tabular-nums">{votes.approve}</span>
          </span>
          <span className="flex items-center gap-1">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-destructive" />
            {intl.formatMessage({ id: "chat.council.votes.reject" })}
            <span className="font-mono tabular-nums">{votes.reject}</span>
          </span>
          <span className="flex items-center gap-1">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-foreground/25" />
            {intl.formatMessage({ id: "chat.council.votes.abstain" })}
            <span className="font-mono tabular-nums">{votes.abstain}</span>
          </span>
        </span>
      </section>
    </TooltipProvider>
  );
}
