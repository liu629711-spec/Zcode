/**
 * 圆桌会独立页（2026-10-03 刀5）：与「素材库」平级的一级功能页。
 *
 * 布局（自上而下）：
 * - 顶栏：退回按钮（回 chat 视图）——独立页不进 taskNavHistory，退回即显式；
 * - 左缘：会议室台账（council/list 轮询复用，进行中/已收口两小节，条目=议题+
 *   状态徽章+时间，选中高亮；选中对账走 resolveSelectedCouncilEntry）；
 * - 主区：选中会议的圆桌画布（复用会话内 CouncilStageSession：同一张桌、同一
 *   套控场）；未选中=空态引导。
 *
 * 数据洞补口：会议模型只能从召集方会话的轮证据聚合，而会话内 timeline 只在
 * 打开过的会话里喂 store——这里用 council/evidence（只读）直接取选中会议的
 * 证据单元，selectCouncilMeetings 派生模型后写回 councilStageStore（会话内
 * 画布此后也吃得到），画布即渲染。
 */
import { useMemo, useState } from "react";
import { ArrowLeft, UsersRound } from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { Button } from "@/components/ui/button.js";
import {
  buildCouncilMeetingDirectory,
  councilMeetingEntrySubject,
  resolveSelectedCouncilEntry,
  type CouncilMeetingDirectoryEntry,
  type CouncilMeetingSummaryLike,
} from "@/v4/councilMeetingDirectory.js";
import { COUNCIL_STATUS_BADGE_CLASS } from "@/v4/councilMeetingVisuals.js";
import { formatRelativeToNow } from "@/settings/automationFormat.js";
import {
  useWorkspaceCouncilMeetings,
} from "@/hooks/useWorkspaceCouncilMeetings.js";
import { useCouncilEvidence } from "@/hooks/useCouncilEvidence.js";
import { CouncilStageSession } from "@/v4/CouncilTableStage.js";

/** 台账状态 → 徽章配色：同侧栏分组词表；cancelled（哑会）退灰。 */
const DIRECTORY_STATUS_BADGE_CLASS: Record<
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

/** 台账状态 → 徽章词（proposed 不会出现在台账，退 running 词）。 */
function statusMessageId(status: CouncilMeetingSummaryLike["status"]): string {
  return `chat.council.status.${status === "proposed" ? "running" : status}`;
}

function DirectoryEntryRow({
  entry,
  selected,
  nowMs,
  onSelect,
}: {
  entry: CouncilMeetingDirectoryEntry;
  selected: boolean;
  nowMs: number;
  onSelect: (entry: CouncilMeetingDirectoryEntry) => void;
}) {
  const { intl } = useZCodeIntl();
  const subject =
    councilMeetingEntrySubject(entry) ??
    intl.formatMessage({ id: `chat.council.kind.${entry.kind}` });
  return (
    <li>
      <button
        type="button"
        onClick={() => onSelect(entry)}
        aria-current={selected || undefined}
        data-testid="council-workspace-directory-row"
        className={cn(
          "flex h-8 w-full min-w-0 cursor-pointer items-center gap-2 rounded-md px-2.5 text-left outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30",
          selected && "bg-selected",
        )}
      >
        <span
          className={cn(
            "min-w-0 flex-1 truncate text-ui-base",
            selected ? "text-foreground" : "text-foreground-subtle",
          )}
        >
          {subject}
        </span>
        <span
          className={cn(
            "shrink-0 rounded-[4px] px-1.5 py-0.5 text-ui-2xs font-medium leading-4",
            DIRECTORY_STATUS_BADGE_CLASS[entry.status],
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

function DirectorySubsection({
  title,
  entries,
  selectedCouncilId,
  nowMs,
  onSelect,
}: {
  title: string;
  entries: CouncilMeetingDirectoryEntry[];
  selectedCouncilId: string | null;
  nowMs: number;
  onSelect: (entry: CouncilMeetingDirectoryEntry) => void;
}) {
  if (entries.length === 0) return null;
  return (
    <div data-testid="council-workspace-directory-subsection">
      <div className="px-2.5 text-ui-2xs font-medium text-foreground-subtlest">
        {title}
      </div>
      <ul className="mt-0.5 flex flex-col gap-0.5">
        {entries.map((entry) => (
          <DirectoryEntryRow
            key={entry.councilId}
            entry={entry}
            selected={entry.councilId === selectedCouncilId}
            nowMs={nowMs}
            onSelect={onSelect}
          />
        ))}
      </ul>
    </div>
  );
}

export function CouncilWorkspacePage({
  workspacePath,
  workspaceIdentity,
  onBack,
}: {
  workspacePath: string;
  workspaceIdentity?: string;
  onBack: () => void;
}) {
  const { intl } = useZCodeIntl();
  const [selectedCouncilId, setSelectedCouncilId] = useState<string | null>(null);
  const { meetings, loading, error, refresh } = useWorkspaceCouncilMeetings({
    workspacePath,
    workspaceIdentity,
  });
  const directory = useMemo(
    () => buildCouncilMeetingDirectory(meetings),
    [meetings],
  );
  // 选中对账：目录刷新后会议消失（会话被清）就回空态，不按陈旧 id 硬画。
  const selected = resolveSelectedCouncilEntry(directory, selectedCouncilId);
  const evidence = useCouncilEvidence({
    workspacePath,
    workspaceIdentity,
    sessionId: selected?.sessionId ?? null,
    councilId: selected?.councilId ?? null,
    timeUpdated: selected?.timeUpdated,
  });
  // 相对时间「x 分钟前」粒度：渲染期取当前时刻即可（同侧栏分组纪律）。
  const nowMs = Date.now();

  const stage = (() => {
    if (!selected) return null;
    if (evidence.meeting) {
      return (
        <CouncilStageSession
          meeting={evidence.meeting}
          sessionId={selected.sessionId}
          workspacePath={workspacePath}
          {...(workspaceIdentity ? { workspaceIdentity } : {})}
          onExit={() => setSelectedCouncilId(null)}
          exitMessageId="councilWorkspace.backToDirectory"
        />
      );
    }
    if (evidence.loading) {
      return (
        <div className="flex flex-1 items-center justify-center text-ui-sm text-foreground-subtle">
          {intl.formatMessage({ id: "councilWorkspace.evidence.loading" })}
        </div>
      );
    }
    if (evidence.error) {
      return (
        <div className="flex flex-1 flex-col items-center justify-center gap-2">
          <p className="text-ui-sm text-foreground-subtle">
            {intl.formatMessage({ id: "councilWorkspace.evidence.error" })}
          </p>
          <Button type="button" variant="outline" size="sm" onClick={evidence.refresh}>
            {intl.formatMessage({ id: "workspaceSidebar.council.retry" })}
          </Button>
        </div>
      );
    }
    return (
      <div className="flex flex-1 items-center justify-center text-ui-sm text-foreground-subtle">
        {intl.formatMessage({ id: "councilWorkspace.evidence.empty" })}
      </div>
    );
  })();

  return (
    <section
      aria-label={intl.formatMessage({ id: "councilWorkspace.title" })}
      data-testid="council-workspace-page"
      className="flex h-full min-h-0 flex-col bg-background"
    >
      <header className="flex items-center gap-2 border-b border-card-border/60 px-4 py-2.5">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onBack}
          data-testid="council-workspace-back"
        >
          <ArrowLeft aria-hidden="true" className="size-3.5" />
          {intl.formatMessage({ id: "chat.council.stage.exit" })}
        </Button>
        <span className="text-ui-base font-medium text-foreground">
          {intl.formatMessage({ id: "councilWorkspace.title" })}
        </span>
      </header>

      <div className="flex min-h-0 flex-1">
        <aside
          aria-label={intl.formatMessage({ id: "workspaceSidebar.councilSection" })}
          data-testid="council-workspace-directory"
          className="flex w-72 shrink-0 flex-col overflow-y-auto border-r border-card-border/60 bg-card/30"
        >
          <div className="flex flex-col gap-2 px-1.5 py-2">
            {loading ? (
              <div className="px-2.5 py-1.5 text-ui-base text-foreground-subtle">
                {intl.formatMessage({ id: "workspaceSidebar.council.loading" })}
              </div>
            ) : null}
            {!loading && error ? (
              <div className="flex items-center gap-2 px-2.5 py-1.5">
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
                <DirectorySubsection
                  title={intl.formatMessage({ id: "workspaceSidebar.council.running" })}
                  entries={directory.running}
                  selectedCouncilId={selectedCouncilId}
                  nowMs={nowMs}
                  onSelect={(entry) => setSelectedCouncilId(entry.councilId)}
                />
                <DirectorySubsection
                  title={intl.formatMessage({ id: "workspaceSidebar.council.closed" })}
                  entries={directory.closed}
                  selectedCouncilId={selectedCouncilId}
                  nowMs={nowMs}
                  onSelect={(entry) => setSelectedCouncilId(entry.councilId)}
                />
                {directory.running.length === 0 && directory.closed.length === 0 ? (
                  <div className="px-2.5 py-1.5 text-ui-base text-foreground-subtle">
                    {intl.formatMessage({ id: "workspaceSidebar.council.empty" })}
                  </div>
                ) : null}
              </>
            ) : null}
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col">
          {stage ?? (
            <div
              data-testid="council-workspace-empty"
              className="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center"
            >
              <div className="flex size-12 items-center justify-center rounded-full bg-muted">
                <UsersRound aria-hidden="true" className="size-6 text-foreground-subtle" />
              </div>
              <p className="text-ui-base text-foreground-subtle">
                {intl.formatMessage({ id: "councilWorkspace.empty" })}
              </p>
              <p className="text-ui-sm text-foreground-subtlest">
                {intl.formatMessage({ id: "councilWorkspace.emptyHint" })}
              </p>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
