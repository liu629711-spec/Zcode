/**
 * 侧栏「会议室」分组（2026-10-03 圆桌会刀1）：当前 workspace 的圆桌会目录，
 * 进行中/已收口两小节（收口含流会 deadlocked）。条目=议题+状态徽章+更新时间；
 * 点击=打开召集方会话并自动展开该会议的评审专区（councilFocusStore 握手，
 * 会议卡对号后开 Zone）。数据走 useWorkspaceCouncilMeetings（council/list 只读
 * +低频轮询）。三态齐全：加载/错误（带重试）/空（暂无会议）。
 */
import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, UsersRound } from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import {
  COUNCIL_STATUS_BADGE_CLASS,
} from "@/v4/councilMeetingVisuals.js";
import {
  buildCouncilMeetingDirectory,
  councilMeetingEntrySubject,
  type CouncilMeetingDirectoryEntry,
  type CouncilMeetingSummaryLike,
} from "@/v4/councilMeetingDirectory.js";
import { formatRelativeToNow } from "@/settings/automationFormat.js";
import {
  useWorkspaceCouncilMeetings,
} from "@/hooks/useWorkspaceCouncilMeetings.js";
import { useCouncilFocusStore } from "@/store/councilFocusStore.js";

/** 台账状态 → 徽章配色：复用专区词表；cancelled（全场派单失败的哑会）退灰。 */
const SIDEBAR_STATUS_BADGE_CLASS: Record<
  CouncilMeetingSummaryLike["status"],
  string
> = {
  proposed: COUNCIL_STATUS_BADGE_CLASS.running,
  running: COUNCIL_STATUS_BADGE_CLASS.running,
  approved: COUNCIL_STATUS_BADGE_CLASS.approved,
  rejected: COUNCIL_STATUS_BADGE_CLASS.rejected,
  deadlocked: COUNCIL_STATUS_BADGE_CLASS.deadlocked,
  cancelled: "bg-muted text-foreground-subtle",
};

/** 台账状态 → 徽章词：proposed 不会出现在台账（落账即 running），退 running 词。 */
function statusMessageId(status: CouncilMeetingSummaryLike["status"]): string {
  return `chat.council.status.${status === "proposed" ? "running" : status}`;
}

function CouncilMeetingRow({
  entry,
  nowMs,
  onOpen,
}: {
  entry: CouncilMeetingDirectoryEntry;
  nowMs: number;
  onOpen: (entry: CouncilMeetingDirectoryEntry) => void;
}) {
  const { intl } = useZCodeIntl();
  const subject = councilMeetingEntrySubject(entry);
  const label = intl.formatMessage({ id: "workspaceSidebar.council.rowAria" }, {
    subject:
      subject ??
      intl.formatMessage({ id: `chat.council.kind.${entry.kind}` }),
    status: intl.formatMessage({ id: statusMessageId(entry.status) }),
  });
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpen(entry)}
        aria-label={label}
        data-testid="workspace-council-row"
        className="flex h-7 w-full min-w-0 cursor-pointer items-center gap-2 rounded-md px-2.5 text-left outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
      >
        <span className="min-w-0 flex-1 truncate text-ui-base text-foreground-subtle">
          {subject ??
            intl.formatMessage({ id: `chat.council.kind.${entry.kind}` })}
        </span>
        <span
          className={cn(
            "shrink-0 rounded-[4px] px-1.5 py-0.5 text-ui-2xs font-medium leading-4",
            SIDEBAR_STATUS_BADGE_CLASS[entry.status],
          )}
        >
          {intl.formatMessage({ id: statusMessageId(entry.status) })}
        </span>
        <span className="shrink-0 text-ui-2xs tabular-nums text-foreground-subtlest">
          {formatRelativeToNow(entry.timeUpdated, nowMs, intl)}
        </span>
      </button>
    </li>
  );
}

function CouncilMeetingSubsection({
  title,
  entries,
  nowMs,
  onOpen,
}: {
  title: string;
  entries: CouncilMeetingDirectoryEntry[];
  nowMs: number;
  onOpen: (entry: CouncilMeetingDirectoryEntry) => void;
}) {
  if (entries.length === 0) return null;
  return (
    <div data-testid="workspace-council-subsection">
      <div className="px-2.5 text-ui-2xs font-medium text-foreground-subtlest">
        {title}
      </div>
      <ul className="mt-0.5 flex flex-col gap-0.5">
        {entries.map((entry) => (
          <CouncilMeetingRow
            key={entry.councilId}
            entry={entry}
            nowMs={nowMs}
            onOpen={onOpen}
          />
        ))}
      </ul>
    </div>
  );
}

export function WorkspaceCouncilSection({
  workspacePath,
  workspaceIdentity,
  onSelectTask,
}: {
  workspacePath: string;
  workspaceIdentity?: string;
  onSelectTask: (
    targetWorkspacePath: string,
    taskId: string,
    targetWorkspaceIdentity?: string,
  ) => void;
}) {
  const { intl } = useZCodeIntl();
  const [open, setOpen] = useState(true);
  const { meetings, loading, error, refresh } = useWorkspaceCouncilMeetings({
    workspacePath,
    workspaceIdentity,
  });
  const requestCouncilFocus = useCouncilFocusStore(
    (state) => state.requestCouncilFocus,
  );
  const directory = useMemo(
    () => buildCouncilMeetingDirectory(meetings),
    [meetings],
  );
  // 相对时间「x 分钟前」粒度：轮询 15s 与目录变化都会触发重渲染，渲染期取
  // 当前时刻即可，不需要额外时钟 state。
  const nowMs = Date.now();

  const handleOpen = (entry: CouncilMeetingDirectoryEntry) => {
    requestCouncilFocus(entry.councilId);
    onSelectTask(
      workspacePath,
      entry.sessionId,
      workspaceIdentity,
    );
  };

  return (
    <section
      aria-label={intl.formatMessage({ id: "workspaceSidebar.councilSection" })}
      data-testid="workspace-council-section"
      className="relative mt-1"
    >
      <button
        type="button"
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
        className="flex h-7 min-w-0 w-full items-center gap-1 px-2.5 text-left text-ui-base font-medium text-foreground-subtlest outline-none transition-colors hover:text-foreground focus-visible:text-foreground focus-visible:ring-2 focus-visible:ring-ring/30"
      >
        <UsersRound aria-hidden="true" className="size-3.5 shrink-0" />
        <span className="min-w-0 truncate">
          {intl.formatMessage({ id: "workspaceSidebar.councilSection" })}
        </span>
        {open ? (
          <ChevronDown aria-hidden="true" className="size-3.5 shrink-0" />
        ) : (
          <ChevronRight aria-hidden="true" className="size-3.5 shrink-0" />
        )}
      </button>
      {open ? (
        <div className="flex flex-col gap-2 pb-2">
          {loading ? (
            <div className="px-3 py-1.5 text-ui-base text-foreground-subtle">
              {intl.formatMessage({ id: "workspaceSidebar.council.loading" })}
            </div>
          ) : null}
          {!loading && error ? (
            <div className="flex items-center gap-2 px-3 py-1.5">
              <span className="min-w-0 flex-1 text-ui-base text-foreground-subtle">
                {intl.formatMessage({ id: "workspaceSidebar.council.error" })}
              </span>
              <button
                type="button"
                onClick={refresh}
                className="shrink-0 cursor-pointer rounded-[4px] text-ui-2xs text-foreground-subtle underline-offset-2 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
              >
                {intl.formatMessage({ id: "workspaceSidebar.council.retry" })}
              </button>
            </div>
          ) : null}
          {!loading && !error ? (
            <>
              <CouncilMeetingSubsection
                title={intl.formatMessage({
                  id: "workspaceSidebar.council.running",
                })}
                entries={directory.running}
                nowMs={nowMs}
                onOpen={handleOpen}
              />
              <CouncilMeetingSubsection
                title={intl.formatMessage({
                  id: "workspaceSidebar.council.closed",
                })}
                entries={directory.closed}
                nowMs={nowMs}
                onOpen={handleOpen}
              />
              {directory.running.length === 0 && directory.closed.length === 0 ? (
                <div className="px-3 py-1.5 text-ui-base text-foreground-subtle">
                  {intl.formatMessage({ id: "workspaceSidebar.council.empty" })}
                </div>
              ) : null}
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
