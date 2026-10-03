/**
 * 圆桌会 UI 的共用视觉原子：状态灯/徽章配色、员工芯片、行尾裁定徽章。
 * 摘要卡（CouncilMeetingCard）、裁决卡与过程流共用同一套词表——颜色只说结论，
 * 每位员工的固定角色色来自档案色板（与工地卡同一纪律）。
 */
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import {
  resolveSubagentColorFromName,
  SUBAGENT_COLOR_CLASS,
} from "@/lib/subagentColors.js";
import type {
  CouncilConfidence,
  CouncilMeetingViewStatus,
  CouncilSeatLensId,
  CouncilStance,
} from "@/v4/councilMeeting.js";

/** 状态 → 状态灯词表（StepRunStatus 四值），摘要卡与专区共用一份词表。 */
export const COUNCIL_STATUS_DOT_KEY: Record<
  CouncilMeetingViewStatus,
  keyof typeof STATUS_DOT
> = {
  running: "running",
  approved: "done",
  rejected: "failed",
  deadlocked: "running",
};

/** 状态徽章配色：进行中琥珀、通过绿、否决红、分歧品牌色（等老板拍板）。 */
export const COUNCIL_STATUS_BADGE_CLASS: Record<
  CouncilMeetingViewStatus,
  string
> = {
  running: "bg-warning/10 text-warning",
  approved: "bg-success/10 text-success",
  rejected: "bg-destructive/10 text-destructive",
  deadlocked: "bg-brand/10 text-brand",
};

export const COUNCIL_STANCE_BADGE_CLASS: Record<CouncilStance, string> = {
  approve: "bg-success/10 text-success",
  reject: "bg-destructive/10 text-destructive",
  abstain: "bg-muted text-foreground-subtle",
};

const CONFIDENCE_CLASS: Record<CouncilConfidence, string> = {
  high: "text-foreground",
  medium: "text-foreground-subtle",
  low: "text-foreground-subtlest",
};

/** 员工芯片：档案色板按名取色，无名退灰（与工地卡同一视觉词表）。 */
export function AgentChip({ name }: { name: string }) {
  return (
    <span
      className={cn(
        "flex h-5 max-w-full shrink-0 items-center rounded-[4px] px-1.5 leading-none",
        name
          ? SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(name)]
          : "bg-muted text-foreground-subtle",
      )}
    >
      <span className="max-w-28 truncate text-ui-xs font-medium">
        {name || "—"}
      </span>
    </span>
  );
}

export function lensMessageId(lens: CouncilSeatLensId): string {
  return `chat.council.lens.${lens}`;
}

/** 行尾裁定徽章：立场+信心+（一票）否决，散文永远换不来这枚徽章。 */
export function VerdictBadge({
  stance,
  confidence,
  veto,
}: {
  stance: CouncilStance;
  confidence: CouncilConfidence;
  veto: boolean;
}) {
  const { intl } = useZCodeIntl();
  return (
    <span className="inline-flex shrink-0 items-center gap-1">
      <span
        className={cn(
          "rounded-[4px] px-1.5 text-ui-2xs font-medium leading-4",
          COUNCIL_STANCE_BADGE_CLASS[stance],
        )}
      >
        {intl.formatMessage({ id: `chat.council.stance.${stance}` })}
      </span>
      <span className={cn("text-ui-2xs", CONFIDENCE_CLASS[confidence])}>
        {intl.formatMessage({ id: `chat.council.confidence.${confidence}` })}
      </span>
      {veto ? (
        <span className="rounded-[4px] bg-destructive/10 px-1.5 text-ui-2xs font-medium leading-4 text-destructive">
          {intl.formatMessage({ id: "chat.council.veto.badge" })}
        </span>
      ) : null}
    </span>
  );
}
