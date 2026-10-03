/**
 * 圆桌会（真会议）一行摘要卡：一场会议一行收敛——种类词+状态徽章+票数速览，
 * 不展开长文；点整行进入圆桌画布两态视图（CouncilStageLayer，2026-10-03 刀2：
 * 会话⇄圆桌，原评审专区弹层由画布右缘卷宗抽屉承接）。
 *
 * 数据来自 selectCouncilMeetings（纯函数，按 originMeta.councilId 聚席位回执轮），
 * 这里只管画。与工地卡同一条纪律：挂在与会议相关的最新证据轮上（活边），
 * 状态只认证据——票没交齐一律「进行中」，绝不假报终局。
 */
import { UsersRound } from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import {
  COUNCIL_STATUS_BADGE_CLASS,
  COUNCIL_STATUS_DOT_KEY,
} from "@/v4/councilMeetingVisuals.js";
import { useCouncilStageStore } from "@/store/councilStageStore.js";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";

export function CouncilMeetingCard({
  meeting,
}: {
  meeting: CouncilMeetingModel;
}) {
  const { intl } = useZCodeIntl();
  const openCouncilStage = useCouncilStageStore(
    (state) => state.openCouncilStage,
  );
  const statusId = `chat.council.status.${meeting.status}`;
  const votes = meeting.tally;
  return (
    <button
      type="button"
      onClick={() => openCouncilStage(meeting.councilId)}
      aria-label={intl.formatMessage({ id: "chat.council.card.openAria" })}
      data-testid="council-meeting-card"
      className="flex w-full min-w-0 cursor-pointer items-center gap-2 rounded-xl border border-card-border bg-card px-3.5 py-2.5 text-left font-[inherit] shadow-xs transition-colors hover:bg-surface-hover/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
    >
      <UsersRound
        className="size-4 shrink-0 text-foreground-subtle"
        aria-hidden="true"
      />
      <span className="shrink-0 text-ui-base font-medium text-foreground-subtle">
        {intl.formatMessage({ id: "chat.council.title" })}
        {meeting.kind ? (
          <span className="text-ui-base font-normal text-foreground-subtlest">
            {" · "}
            {intl.formatMessage({ id: `chat.council.kind.${meeting.kind}` })}
          </span>
        ) : null}
      </span>
      <span className="min-w-0 flex-1" />
      <span
        className={cn(
          "shrink-0 rounded-[4px] px-1.5 py-0.5 text-ui-2xs font-medium leading-4",
          COUNCIL_STATUS_BADGE_CLASS[meeting.status],
        )}
      >
        {intl.formatMessage({ id: statusId })}
      </span>
      <span className="flex shrink-0 items-center gap-1.5">
        <span
          aria-hidden="true"
          className={cn(
            "size-1.5 rounded-full",
            STATUS_DOT[COUNCIL_STATUS_DOT_KEY[meeting.status]],
          )}
        />
        <span className="font-mono text-ui-xs tabular-nums text-foreground-subtlest">
          {intl.formatMessage(
            { id: "chat.council.card.votesSummary" },
            {
              approve: String(votes.approve),
              reject: String(votes.reject),
              abstain: String(votes.abstain),
            },
          )}
        </span>
      </span>
    </button>
  );
}
