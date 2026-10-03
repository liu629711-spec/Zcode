/**
 * 圆桌会评审专区的 ③ 过程流（默认折叠）：席位发言时间线——每位员工固定角色
 * 色条+姓名+类型徽章（陈述=首轮/质询=二轮）+正文+行尾裁定徽章+轮次标记；
 * 同人同轮短发言已按纯函数规则合并（裁定取最后一条）。
 * 数据只来自 CouncilMeetingModel（纯函数聚合 originMeta.council*），这里只管画。
 */
import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import {
  resolveSubagentColorFromName,
  SUBAGENT_COLOR_CLASS,
} from "@/lib/subagentColors.js";
import type {
  CouncilFlowEntry,
  CouncilMeetingModel,
} from "@/v4/councilMeeting.js";
import { AgentChip, VerdictBadge } from "@/v4/councilMeetingVisuals.js";

function FlowEntryRow({ entry }: { entry: CouncilFlowEntry }) {
  const { intl } = useZCodeIntl();
  const colorClass = entry.agentName
    ? SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(entry.agentName)]
    : "bg-muted";
  return (
    <div className="flex min-w-0 gap-2.5" data-testid="council-flow-entry">
      {/* 角色色条：每位员工固定角色色（档案色板按名取色）。 */}
      <span
        aria-hidden="true"
        className={cn(
          "mt-0.5 w-0.5 shrink-0 rounded-full self-stretch",
          colorClass,
        )}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-1.5">
          <AgentChip name={entry.agentName} />
          <span className="flex h-4 shrink-0 items-center rounded-[4px] bg-muted px-1.5 leading-none text-ui-2xs text-foreground-subtle">
            {intl.formatMessage({ id: `chat.council.flow.type.${entry.type}` })}
          </span>
          <span className="min-w-0 flex-1" />
          {entry.verdict ? (
            <VerdictBadge
              stance={entry.verdict.stance}
              confidence={entry.verdict.confidence}
              veto={entry.verdict.veto}
            />
          ) : null}
        </div>
        {entry.status === "completed" ? (
          <p className="min-w-0 whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtle">
            {entry.text || "—"}
          </p>
        ) : (
          <p className="min-w-0 text-ui-xs leading-4 text-foreground-subtlest">
            {intl.formatMessage({
              id:
                entry.status === "cancelled"
                  ? "chat.council.receipt.cancelled"
                  : "chat.council.receipt.failed",
            })}
          </p>
        )}
      </div>
    </div>
  );
}

export function CouncilProcessFlow({
  meeting,
}: {
  meeting: CouncilMeetingModel;
}) {
  const { intl } = useZCodeIntl();
  const [expanded, setExpanded] = useState(false);
  const sectionId = "council-process-flow-content";
  let lastRound = 0;
  return (
    <section
      aria-label={intl.formatMessage({ id: "chat.council.section.flow" })}
      className="rounded-xl border border-card-border bg-card"
      data-testid="council-zone-flow"
    >
      <button
        type="button"
        onClick={() => setExpanded((current) => !current)}
        aria-expanded={expanded}
        aria-controls={sectionId}
        data-testid="council-zone-flow-toggle"
        className="flex w-full min-w-0 cursor-pointer items-center gap-2 rounded-xl px-3.5 py-2.5 text-left font-[inherit] transition-colors hover:bg-surface-hover/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
      >
        <span className="text-ui-base font-medium text-foreground-subtle">
          {intl.formatMessage({ id: "chat.council.section.flow" })}
        </span>
        <span className="font-mono text-ui-xs tabular-nums text-foreground-subtlest">
          {meeting.flow.length}
        </span>
        <span className="min-w-0 flex-1" />
        <span
          aria-hidden="true"
          className="flex size-6 shrink-0 items-center justify-center text-foreground-subtle"
        >
          {expanded ? (
            <ChevronDown className="size-3.5" />
          ) : (
            <ChevronRight className="size-3.5" />
          )}
        </span>
      </button>
      {expanded ? (
        <div
          id={sectionId}
          className="flex flex-col gap-3 border-t border-card-border/60 px-3.5 py-3"
        >
          {meeting.flow.length === 0 ? (
            <p className="text-ui-xs text-foreground-subtlest">
              {intl.formatMessage({ id: "chat.council.flow.empty" })}
            </p>
          ) : (
            meeting.flow.map((entry) => {
              const roundMarker =
                entry.round !== lastRound ? (
                  <div
                    key={`round-${entry.round}`}
                    className="flex items-center gap-2 text-ui-2xs font-medium text-foreground-subtlest"
                  >
                    <span className="rounded-full border border-card-border px-2 py-0.5">
                      {intl.formatMessage(
                        { id: "chat.council.flow.roundMarker" },
                        {
                          round: String(entry.round),
                          type: intl.formatMessage({
                            id: `chat.council.flow.type.${entry.round === 1 ? "statement" : "cross"}`,
                          }),
                        },
                      )}
                    </span>
                    <span
                      aria-hidden="true"
                      className="h-px min-w-0 flex-1 bg-card-border/60"
                    />
                  </div>
                ) : null;
              lastRound = entry.round;
              return (
                <div key={entry.key} className="flex flex-col gap-2">
                  {roundMarker}
                  <FlowEntryRow entry={entry} />
                </div>
              );
            })
          )}
        </div>
      ) : null}
    </section>
  );
}
