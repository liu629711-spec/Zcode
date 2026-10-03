/**
 * 圆桌画布（2026-10-03 刀2）：会话⇄圆桌两态中的「圆桌」态。
 *
 * 布局（自上而下）：
 * - 顶部局势带：退回会话 + 议题行 + ①局势条（复用专区段组件：状态灯/徽章/
 *   轮次 pill/票数条；分歧轴是刀4 的弦，此处只留位不留死 DOM）；
 * - 主区：DOM 绝对定位的椭圆桌（不引 canvas/SVG 库）——主席席正上方、席位
 *   环绕（头像+名牌+攻角徽章+状态灯三态）、桌心=当前+上一条发言气泡（点击
 *   滚动定位到右缘卷宗对应条目）；收口会议桌心换裁决石（结论一行+票型）+只读；
 * - 右缘卷宗抽屉：②裁决卡（共识/分歧/还没交票/盲区）+ ③过程流 + 主席结论，
 *   全部复用专区四段式组件；画布装饰 aria-hidden，数据在这里可读；
 * - 底栏：控场（暂停/催一句/插话）——v4 命令面没有圆桌控场命令，全部明确
 *   禁用+tooltip（同刀1 专区口径，不做假按钮）；右侧票数速览+收口徽章。
 *
 * 两态挂载见 CouncilStageLayer（会话 pane 顶部叠不透明层，退回即摘）。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowLeft, Gavel, PanelRightClose, PanelRightOpen } from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { Button } from "@/components/ui/button.js";
import {
  resolveSubagentColorFromName,
  SUBAGENT_COLOR_CLASS,
} from "@/lib/subagentColors.js";
import { councilMeetingEntrySubject } from "@/v4/councilMeetingDirectory.js";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";
import { lensMessageId } from "@/v4/councilMeetingVisuals.js";
import {
  CouncilModeratorSummary,
  CouncilStatusStrip,
} from "@/v4/CouncilMeetingZone.js";
import { CouncilVerdictCard } from "@/v4/CouncilMeetingVerdict.js";
import { CouncilProcessFlow } from "@/v4/CouncilMeetingFlow.js";
import {
  CouncilStageBubbleView,
  CouncilStageControlBar,
  CouncilVerdictStone,
} from "@/v4/councilStageVisuals.js";
import { useWorkspaceCouncilMeetings } from "@/hooks/useWorkspaceCouncilMeetings.js";
import { useCouncilFocusStore } from "@/store/councilFocusStore.js";
import { useCouncilStageStore } from "@/store/councilStageStore.js";
import {
  councilSeatLampState,
  councilSeatPositions,
  councilStageBubbles,
  COUNCIL_TABLE_CENTER,
  isCouncilMeetingClosed,
  type CouncilSeatLampState,
} from "@/v4/councilStageModel.js";

// ── 两态挂载层 ──────────────────────────────────────────────────────

/**
 * 会话 pane 根部的两态开关：本会话证据里有 openCouncilId 对应的会议就叠
 * 圆桌层（盖住会话视图，含 pane 头——独立界面感），没有就什么都不画（会话态）。
 * 侧栏定向打开沿用刀1 的 councilFocusStore 握手：焦点会议在本会话出现即进
 * 圆桌态（一次性消费；证据没到就继续等，最坏情况同刀1——晚到自动弹开一次）。
 */
export function CouncilStageLayer({
  sessionId,
  workspacePath,
  workspaceIdentity,
}: {
  sessionId: string | null;
  workspacePath: string;
  workspaceIdentity?: string;
}) {
  const openCouncilId = useCouncilStageStore((state) => state.openCouncilId);
  const openCouncilStage = useCouncilStageStore((state) => state.openCouncilStage);
  const closeCouncilStage = useCouncilStageStore((state) => state.closeCouncilStage);
  const meetings = useCouncilStageStore((state) =>
    sessionId ? state.meetingsBySession[sessionId] : undefined,
  );
  const focusCouncilId = useCouncilFocusStore(
    (state) => state.focus?.councilId ?? null,
  );
  const consumeCouncilFocus = useCouncilFocusStore(
    (state) => state.consumeCouncilFocus,
  );

  useEffect(() => {
    if (!focusCouncilId) return;
    if (!meetings?.some((meeting) => meeting.councilId === focusCouncilId)) return;
    consumeCouncilFocus(focusCouncilId);
    openCouncilStage(focusCouncilId);
  }, [consumeCouncilFocus, focusCouncilId, meetings, openCouncilStage]);

  const meeting = useMemo(
    () => meetings?.find((candidate) => candidate.councilId === openCouncilId) ?? null,
    [meetings, openCouncilId],
  );
  if (!meeting) return null;
  return (
    <div
      className="absolute inset-0 z-40 bg-background"
      data-testid="council-stage-layer"
    >
      <CouncilStageSession
        meeting={meeting}
        workspacePath={workspacePath}
        {...(workspaceIdentity ? { workspaceIdentity } : {})}
        onExit={closeCouncilStage}
      />
    </div>
  );
}

/** 议题行：council/list 台账带 motion 原话（零协议改动）；退短标题、再退类型词。 */
function CouncilStageSubject({
  meeting,
  directoryEntry,
}: {
  meeting: CouncilMeetingModel;
  directoryEntry: { motion?: string; title?: string } | null;
}) {
  const { intl } = useZCodeIntl();
  const subject =
    (directoryEntry
      ? councilMeetingEntrySubject(directoryEntry)
      : undefined) ??
    (meeting.kind
      ? intl.formatMessage({ id: `chat.council.kind.${meeting.kind}` })
      : intl.formatMessage({ id: "chat.council.title" }));
  return (
    <span
      className="min-w-0 flex-1 truncate text-ui-base text-foreground-subtle"
      data-testid="council-stage-subject"
    >
      {subject}
    </span>
  );
}

/** 画布会话体：只在画布打开时挂载（台账查询随层启停，不给会话添常驻轮询）。 */
function CouncilStageSession({
  meeting,
  workspacePath,
  workspaceIdentity,
  onExit,
}: {
  meeting: CouncilMeetingModel;
  workspacePath: string;
  workspaceIdentity?: string;
  onExit: () => void;
}) {
  const { intl } = useZCodeIntl();
  const [drawerOpen, setDrawerOpen] = useState(true);
  const [flowExpanded, setFlowExpanded] = useState(true);
  const { meetings: directory } = useWorkspaceCouncilMeetings({
    workspacePath,
    ...(workspaceIdentity ? { workspaceIdentity } : {}),
  });
  const directoryEntry =
    directory.find((candidate) => candidate.councilId === meeting.councilId) ?? null;

  // 气泡→卷宗定位：先把抽屉/过程流展开再滚（DOM 提交在宏任务后才生效）。
  const locateInDrawer = useCallback((entryKey: string) => {
    setDrawerOpen(true);
    setFlowExpanded(true);
    window.setTimeout(() => {
      document
        .getElementById(`council-flow-entry-${entryKey}`)
        ?.scrollIntoView({ block: "center" });
    }, 0);
  }, []);

  return (
    <section
      aria-label={intl.formatMessage({ id: "chat.council.stage.stageAria" })}
      data-testid="council-table-stage"
      className="flex h-full min-h-0 flex-col bg-background"
    >
      <header className="flex flex-col gap-2 border-b border-card-border/60 px-4 py-2.5">
        <div className="flex min-w-0 items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onExit}
            data-testid="council-stage-exit"
          >
            <ArrowLeft aria-hidden="true" className="size-3.5" />
            {intl.formatMessage({ id: "chat.council.stage.exit" })}
          </Button>
          <CouncilStageSubject meeting={meeting} directoryEntry={directoryEntry} />
          <button
            type="button"
            onClick={() => setDrawerOpen((current) => !current)}
            aria-label={intl.formatMessage({
              id: drawerOpen
                ? "chat.council.stage.drawer.hide"
                : "chat.council.stage.drawer.show",
            })}
            aria-pressed={drawerOpen}
            data-testid="council-stage-drawer-toggle"
            className="flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-foreground-subtle outline-none transition-colors hover:bg-hover hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
          >
            {drawerOpen ? (
              <PanelRightClose aria-hidden="true" className="size-4" />
            ) : (
              <PanelRightOpen aria-hidden="true" className="size-4" />
            )}
          </button>
        </div>
        {/* 局势带：状态灯+徽章+轮次 pill+票数条（复用①局势条）；分歧轴=刀4。 */}
        <CouncilStatusStrip meeting={meeting} />
      </header>

      <div className="flex min-h-0 flex-1">
        <CouncilTableCanvas meeting={meeting} onLocate={locateInDrawer} />
        {drawerOpen ? (
          <aside
            aria-label={intl.formatMessage({ id: "chat.council.stage.drawer" })}
            data-testid="council-stage-drawer"
            className="flex w-80 min-w-0 shrink-0 flex-col border-l border-card-border/60 bg-card/30"
          >
            <div className="flex items-center gap-2 border-b border-card-border/60 px-3.5 py-2">
              <span className="text-ui-base font-medium text-foreground-subtle">
                {intl.formatMessage({ id: "chat.council.stage.drawer" })}
              </span>
            </div>
            <div
              data-testid="council-stage-drawer-body"
              className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-3.5 py-3"
            >
              <CouncilModeratorSummary meeting={meeting} />
              <CouncilVerdictCard meeting={meeting} />
              <CouncilProcessFlow
                meeting={meeting}
                expanded={flowExpanded}
                onExpandedChange={setFlowExpanded}
              />
            </div>
          </aside>
        ) : null}
      </div>

      <CouncilStageControlBar meeting={meeting} />
    </section>
  );
}

// ── 画布：椭圆桌 + 主席席 + 席位环绕 + 桌心 ────────────────────────

const SEAT_LAMP_DOT_CLASS: Record<CouncilSeatLampState, string> = {
  // 刀2 静态三态：思考中琥珀（刀3 接实时证据换 animate-pulse 呼吸）。
  thinking: "bg-warning",
  spoken: "bg-success",
  absent: "bg-foreground/25",
};

function CouncilTableCanvas({
  meeting,
  onLocate,
}: {
  meeting: CouncilMeetingModel;
  onLocate: (entryKey: string) => void;
}) {
  const { intl } = useZCodeIntl();
  const seats = meeting.seats;
  const positions = useMemo(
    () => councilSeatPositions(seats.length),
    [seats.length],
  );
  const bubbles = useMemo(() => councilStageBubbles(meeting), [meeting]);
  const closed = isCouncilMeetingClosed(meeting);
  return (
    <div
      aria-label={intl.formatMessage({ id: "chat.council.stage.canvasAria" })}
      data-testid="council-stage-canvas"
      className="relative min-h-64 min-w-0 flex-1 overflow-hidden"
    >
      {/* 椭圆桌（纯装饰）：尺寸与席位轨道半径在 councilStageModel.ts 同源。 */}
      <div
        aria-hidden="true"
        className="absolute -translate-x-1/2 -translate-y-1/2 rounded-[50%] border-2 border-brand/20 bg-gradient-to-b from-brand/[0.06] to-transparent shadow-[inset_0_2px_16px_rgba(0,0,0,0.06)]"
        style={{
          width: "72%",
          height: "58%",
          left: "50%",
          top: `${COUNCIL_TABLE_CENTER.yPct}%`,
        }}
      />

      {/* 主席席：正上方独占槽位。 */}
      <div
        data-testid="council-stage-moderator"
        className="absolute left-1/2 top-1.5 flex -translate-x-1/2 flex-col items-center gap-1"
      >
        <span
          aria-hidden="true"
          className="flex size-9 items-center justify-center rounded-full border border-brand/30 bg-brand/10 text-foreground shadow-xs"
        >
          <Gavel className="size-4" />
        </span>
        <span className="rounded-full bg-background/85 px-1.5 py-0.5 text-ui-2xs font-medium text-foreground-subtle">
          {intl.formatMessage({ id: "chat.council.stage.moderator" })}
        </span>
      </div>

      {/* 席位环绕：位置由纯函数算（councilSeatPositions），DOM 绝对定位落点。 */}
      <ul
        aria-label={intl.formatMessage({ id: "chat.council.stage.seatsAria" })}
        data-testid="council-stage-seats"
        className="absolute inset-0 list-none"
      >
        {seats.map((seat, index) => {
          const position = positions[index];
          if (!position) return null;
          const lamp = councilSeatLampState(seat);
          const colorClass = seat.agentName
            ? SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(seat.agentName)]
            : "bg-muted text-foreground-subtle";
          return (
            <li
              key={seat.index}
              data-testid="council-stage-seat"
              className="absolute flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1"
              style={{ left: `${position.xPct}%`, top: `${position.yPct}%` }}
            >
              <span className="relative">
                <span
                  aria-hidden="true"
                  className={cn(
                    "flex size-9 items-center justify-center rounded-full border border-card-border text-ui-sm font-semibold shadow-xs",
                    colorClass,
                  )}
                >
                  {seat.agentName
                    ? seat.agentName.slice(0, 1).toUpperCase()
                    : "—"}
                </span>
                <span
                  aria-hidden="true"
                  className={cn(
                    "absolute -right-0.5 -top-0.5 size-2.5 rounded-full ring-2 ring-background",
                    SEAT_LAMP_DOT_CLASS[lamp],
                  )}
                />
              </span>
              <span className="max-w-24 truncate rounded-full bg-background/85 px-1.5 py-0.5 text-ui-2xs text-foreground-subtle">
                {seat.agentName || "—"}
              </span>
              <span className="rounded-[4px] bg-muted px-1.5 text-ui-2xs leading-4 text-foreground-subtle">
                {intl.formatMessage({ id: lensMessageId(seat.lens) })}
              </span>
              <span className="sr-only">
                {intl.formatMessage({ id: `chat.council.stage.lamp.${lamp}` })}
              </span>
            </li>
          );
        })}
      </ul>

      {/* 桌心：进行中=当前+上一条发言气泡；收口=裁决石+只读回放。 */}
      <div
        className="absolute left-1/2 flex w-[52%] min-w-48 -translate-x-1/2 -translate-y-1/2 flex-col gap-2"
        style={{ left: "50%", top: `${COUNCIL_TABLE_CENTER.yPct}%` }}
      >
        {closed ? (
          <CouncilVerdictStone meeting={meeting} />
        ) : bubbles.current ? (
          <>
            {bubbles.previous ? (
              <CouncilStageBubbleView
                bubble={bubbles.previous}
                variant="previous"
                onLocate={onLocate}
              />
            ) : null}
            <CouncilStageBubbleView
              bubble={bubbles.current}
              variant="current"
              onLocate={onLocate}
            />
          </>
        ) : (
          <p className="text-center text-ui-xs text-foreground-subtlest">
            {intl.formatMessage({ id: "chat.council.flow.empty" })}
          </p>
        )}
      </div>
    </div>
  );
}
