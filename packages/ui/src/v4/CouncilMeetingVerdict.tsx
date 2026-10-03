/**
 * 圆桌会评审专区的 ② 裁决卡：共识区/分歧区/还没交票/盲区区/建议下一步。
 * 分歧每条都带立场+信心+一句话——永不折叠、可回原话（点开看回执正文全文），
 * 绝不显示成「已解决」：分歧是多席位最值钱的信号，不许被折叠或均值洗掉。
 * 数据只来自 CouncilMeetingModel（纯函数聚合 originMeta.council*），这里只管画。
 */
import { useState } from "react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import type {
  CouncilBlindSpot,
  CouncilMeetingModel,
  CouncilPendingSeat,
  CouncilStanceEntry,
} from "@/v4/councilMeeting.js";
import {
  AgentChip,
  lensMessageId,
  VerdictBadge,
} from "@/v4/councilMeetingVisuals.js";

/** 一条立场：立场+信心+一句话；「看原话」展开回执正文全文。 */
function StanceEntryRow({ entry }: { entry: CouncilStanceEntry }) {
  const { intl } = useZCodeIntl();
  const [showOriginal, setShowOriginal] = useState(false);
  const detailId = `council-original-${entry.unitKey}-${entry.seatIndex}`;
  return (
    <div
      className="flex min-w-0 flex-col gap-1"
      data-testid="council-stance-entry"
    >
      <div className="flex min-w-0 flex-wrap items-center gap-1.5">
        <AgentChip name={entry.agentName} />
        <span className="shrink-0 text-ui-2xs text-foreground-subtlest">
          {intl.formatMessage({ id: lensMessageId(entry.lens) })}
        </span>
        <VerdictBadge
          stance={entry.stance}
          confidence={entry.confidence}
          veto={entry.veto}
        />
      </div>
      <div className="flex min-w-0 flex-wrap items-start gap-x-2 gap-y-0.5">
        <p className="min-w-0 flex-1 whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtle">
          {entry.snippet || "—"}
        </p>
        <button
          type="button"
          onClick={() => setShowOriginal((current) => !current)}
          aria-expanded={showOriginal}
          aria-controls={detailId}
          className="shrink-0 cursor-pointer rounded-[4px] text-ui-2xs text-foreground-subtle underline-offset-2 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
        >
          {intl.formatMessage({
            id: showOriginal
              ? "chat.council.entry.original.hide"
              : "chat.council.entry.original.show",
          })}
        </button>
      </div>
      {showOriginal ? (
        <div
          id={detailId}
          className="rounded-lg bg-surface-hover/40 px-2.5 py-2 text-ui-xs leading-4 whitespace-pre-wrap break-words text-foreground-subtle"
        >
          {entry.statement || "—"}
        </div>
      ) : null}
    </div>
  );
}

function VerdictGroup({
  title,
  emptyMessage,
  entries,
  testId,
}: {
  title: string;
  emptyMessage: string;
  entries: readonly CouncilStanceEntry[];
  testId: string;
}) {
  return (
    <div data-testid={testId}>
      <div className="text-ui-2xs font-medium text-foreground-subtlest">
        {title}
      </div>
      <div className="mt-1.5 flex flex-col gap-2.5">
        {entries.length > 0 ? (
          entries.map((entry) => (
            <StanceEntryRow
              key={`${entry.unitKey}:${entry.seatIndex}`}
              entry={entry}
            />
          ))
        ) : (
          <p className="text-ui-xs text-foreground-subtlest">{emptyMessage}</p>
        )}
      </div>
    </div>
  );
}

function PendingSeats({ pending }: { pending: readonly CouncilPendingSeat[] }) {
  const { intl } = useZCodeIntl();
  if (pending.length === 0) return null;
  return (
    <div data-testid="council-pending-seats">
      <div className="text-ui-2xs font-medium text-foreground-subtlest">
        {intl.formatMessage({ id: "chat.council.pending" })}
      </div>
      <ul className="mt-1.5 flex flex-col gap-1.5">
        {pending.map((seat) => (
          <li
            key={seat.seatIndex}
            className="flex min-w-0 flex-wrap items-center gap-1.5"
          >
            <AgentChip name={seat.agentName} />
            <span className="shrink-0 text-ui-2xs text-foreground-subtlest">
              {intl.formatMessage({ id: lensMessageId(seat.lens) })}
            </span>
            <span className="text-ui-2xs text-foreground-subtle">
              {intl.formatMessage({
                id: `chat.council.pending.${seat.reason}`,
              })}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function BlindSpots({
  blindSpots,
}: {
  blindSpots: readonly CouncilBlindSpot[];
}) {
  const { intl } = useZCodeIntl();
  return (
    <div data-testid="council-blind-spots">
      <div className="text-ui-2xs font-medium text-foreground-subtlest">
        {intl.formatMessage({ id: "chat.council.blindSpot" })}
      </div>
      <div className="mt-1.5 flex flex-col gap-1.5">
        {blindSpots.length > 0 ? (
          blindSpots.map((spot) => (
            <div
              key={spot.lens}
              className="flex min-w-0 flex-wrap items-center gap-1.5"
            >
              <span className="flex h-5 shrink-0 items-center rounded-[4px] bg-warning/10 px-1.5 leading-none text-ui-xs font-medium text-warning">
                {intl.formatMessage({ id: lensMessageId(spot.lens) })}
              </span>
              <span className="min-w-0 flex-1 text-ui-2xs text-foreground-subtlest">
                {intl.formatMessage({ id: "chat.council.blindSpot.reason" })}
              </span>
            </div>
          ))
        ) : (
          <p className="text-ui-xs text-foreground-subtlest">
            {intl.formatMessage({ id: "chat.council.blindSpot.empty" })}
          </p>
        )}
      </div>
    </div>
  );
}

export function CouncilVerdictCard({
  meeting,
}: {
  meeting: CouncilMeetingModel;
}) {
  const { intl } = useZCodeIntl();
  return (
    <section
      aria-label={intl.formatMessage({ id: "chat.council.section.verdict" })}
      className="rounded-xl border border-card-border bg-card px-3.5 py-3"
      data-testid="council-zone-verdict"
    >
      <div className="flex flex-col gap-3.5">
        <VerdictGroup
          title={intl.formatMessage({ id: "chat.council.consensus" })}
          emptyMessage={intl.formatMessage({
            id: "chat.council.consensus.empty",
          })}
          entries={meeting.consensus}
          testId="council-consensus"
        />
        {/* 分歧区永不折叠：分歧是多席位最值钱的信号，不许被折叠或显示成已解决。 */}
        <VerdictGroup
          title={intl.formatMessage({ id: "chat.council.disagreement" })}
          emptyMessage={intl.formatMessage({
            id: "chat.council.disagreement.empty",
          })}
          entries={meeting.disagreements}
          testId="council-disagreements"
        />
        <PendingSeats pending={meeting.pending} />
        <BlindSpots blindSpots={meeting.blindSpots} />
        <div
          className="border-t border-card-border/60 pt-3"
          data-testid="council-next-step"
        >
          <div className="text-ui-2xs font-medium text-foreground-subtlest">
            {intl.formatMessage({ id: "chat.council.nextStep" })}
          </div>
          <p className="mt-1 text-ui-xs leading-4 text-foreground-subtle">
            {intl.formatMessage({
              id: `chat.council.nextStep.${meeting.status}`,
            })}
          </p>
        </div>
      </div>
    </section>
  );
}
