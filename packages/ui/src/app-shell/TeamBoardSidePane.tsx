/**
 * 团队看板侧板页签（团队看板批 2026-10-05，参照件 dsh-agent-teams 活动面板的原生落法）。
 *
 * 数据 = teamBoardState 命令快照（CLI 聚合发起会话台账 + 工位在跑状态），聚焦时 2s
 * 轮询——与参照件 1s 轮询同哲学，慢一半省电；快照缺席/失败沿用上一帧（不闪空态）。
 * 纪律与工地卡同源：只画 CLI 权威下发的结构化字段，不从文本反推；聚焦描边用既有
 * focus-visible 灰环（彩色聚焦边框永久禁）。
 * @module team-board pane
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { SquareKanbanIcon } from "lucide-react";
import { TeamBoardPlanCard } from "@/app-shell/TeamBoardPlanCard.js";
import type {
  TeamBoardOrder,
  TeamBoardSnapshot,
  TeamBoardTeam,
  TeamPlan,
  TeamPlanTask,
} from "@zcode/shared/zcode-protocol-v4";
import { createCommandEnvelope } from "@/v4/commandFactory.js";
import { useV4Conversation } from "@/v4/V4ConversationContext.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import type { StepRunStatus } from "@/components/workflow-graph/types.js";
import type { TeamBoardSidePaneTab } from "@/lib/workspaceSidePane.js";
import { computeDagDepths, teamHasDependencies, teamProgress } from "@/app-shell/teamBoardModel.js";

const POLL_MS = 2_000;

/** 工单状态 → 状态灯词表（工地卡同款四值灯：颜色只留给灯，字一律安静灰）。 */
const ORDER_STATUS_DOT: Record<TeamBoardOrder["status"], keyof typeof STATUS_DOT> = {
  in_flight: "running",
  completed: "done",
  cancelled: "pending",
  failed: "failed",
};

const ORDER_STATUS_MESSAGE: Record<TeamBoardOrder["status"], string> = {
  in_flight: "chat.teamBoard.status.in_flight",
  completed: "chat.teamBoard.status.completed",
  cancelled: "chat.teamBoard.status.cancelled",
  failed: "chat.teamBoard.status.failed",
};

const QC_MESSAGE: Record<NonNullable<TeamBoardTeam["qc"]>, string> = {
  skipped: "chat.teamBoard.qc.skipped",
  ran: "chat.teamBoard.qc.ran",
  pending: "chat.teamBoard.qc.pending",
};

const QC_CLASS: Record<NonNullable<TeamBoardTeam["qc"]>, string> = {
  skipped: "bg-success/10 text-success",
  ran: "bg-muted text-foreground-subtle",
  pending: "bg-warning/10 text-warning",
};

const LIVE_DOT: Record<TeamBoardSnapshot["teams"][number]["members"][number]["live"], string> = {
  running: STATUS_DOT.running,
  idle: "bg-foreground-subtlest/50",
  unknown: "border-[1.5px] border-foreground-subtlest/40 bg-transparent",
};

function MemberChip({
  name,
  live,
  model,
}: {
  name: string;
  live: TeamBoardSnapshot["teams"][number]["members"][number]["live"];
  model?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-5 max-w-full items-center gap-1.5 rounded-[4px] px-1.5 leading-none",
        name ? SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(name)] : "bg-muted text-foreground-subtle",
      )}
      title={model}
    >
      <span aria-hidden="true" className={cn("size-1.5 shrink-0 rounded-full", LIVE_DOT[live])} />
      <span className="max-w-28 truncate text-ui-xs font-medium">{name || "—"}</span>
    </span>
  );
}

function OrderRow({ order }: { order: TeamBoardOrder }) {
  const { intl } = useZCodeIntl();
  return (
    <li className="flex min-w-0 flex-col gap-0.5 py-1" data-testid="team-board-order">
      <div className="flex min-w-0 items-center gap-2">
        <span
          aria-hidden="true"
          className={cn("size-1.5 shrink-0 rounded-full", STATUS_DOT[ORDER_STATUS_DOT[order.status]])}
        />
        {order.taskKey ? (
          <span className="shrink-0 rounded-[4px] bg-muted px-1.5 font-mono text-ui-2xs leading-4 text-foreground-subtle">
            {order.taskKey}
          </span>
        ) : null}
        <span className="min-w-0 flex-1 truncate text-ui-xs text-foreground" title={order.taskPreview}>
          {order.taskPreview ?? order.workOrderId}
        </span>
        {order.repairRound !== undefined ? (
          <span className="shrink-0 rounded-[4px] bg-warning/10 px-1.5 text-ui-2xs font-medium leading-4 text-warning">
            {intl.formatMessage({ id: "chat.teamBoard.repairChip" }, { round: String(order.repairRound) })}
          </span>
        ) : null}
        <span className="shrink-0 text-ui-2xs text-foreground-subtlest">
          {intl.formatMessage({ id: ORDER_STATUS_MESSAGE[order.status] })}
        </span>
      </div>
      <div className="flex min-w-0 items-center gap-2 pl-4">
        <span className="truncate text-ui-2xs text-foreground-subtlest">
          {order.agentName}
          {order.model ? ` · ${order.model}` : ""}
        </span>
        {order.dependsOn?.length ? (
          <span className="flex min-w-0 items-center gap-1 text-ui-2xs text-foreground-subtlest">
            <span className="shrink-0">{intl.formatMessage({ id: "chat.teamBoard.deps" })}:</span>
            <span className="truncate font-mono">{order.dependsOn.join(", ")}</span>
            {order.status === "in_flight" ? (
              <span
                className={cn(
                  "shrink-0 rounded-[4px] px-1 leading-4",
                  order.unlocked ? "bg-success/10 text-success" : "bg-warning/10 text-warning",
                )}
              >
                {intl.formatMessage({
                  id: order.unlocked ? "chat.teamBoard.unlocked" : "chat.teamBoard.locked",
                })}
              </span>
            ) : null}
          </span>
        ) : null}
      </div>
    </li>
  );
}

/** DAG 画布几何：定宽列 × 定高行，边画在绝对定位 SVG 里（灰边不抢戏）。 */
const COL_W = 176;
const NODE_H = 44;
const ROW_GAP = 10;

function TeamDag({ team }: { team: TeamBoardTeam }) {
  const { intl } = useZCodeIntl();
  const depths = useMemo(() => computeDagDepths(team.orders), [team.orders]);
  const byDepth = useMemo(() => {
    const columns = new Map<number, TeamBoardOrder[]>();
    for (const order of team.orders) {
      const depth = depths.get(order.workOrderId) ?? 0;
      const column = columns.get(depth) ?? [];
      column.push(order);
      columns.set(depth, column);
    }
    return [...columns.entries()].sort((a, b) => a[0] - b[0]);
  }, [team.orders, depths]);
  const rowOf = useMemo(() => {
    const map = new Map<string, { col: number; row: number }>();
    for (const [col, column] of byDepth) {
      column.forEach((order, row) => map.set(order.workOrderId, { col, row }));
    }
    return map;
  }, [byDepth]);
  const byKey = useMemo(() => {
    const map = new Map<string, TeamBoardOrder>();
    for (const order of team.orders) {
      if (order.taskKey && !map.has(order.taskKey)) map.set(order.taskKey, order);
    }
    return map;
  }, [team.orders]);
  const width = byDepth.length * COL_W;
  const height = Math.max(...byDepth.map(([, column]) => column.length), 1) * (NODE_H + ROW_GAP);
  const nodeX = (col: number) => col * COL_W + COL_W / 2 - 72;
  const nodeY = (row: number) => row * (NODE_H + ROW_GAP);

  return (
    <div className="overflow-x-auto" data-testid="team-board-dag">
      <div className="relative" style={{ width, height }}>
        <svg aria-hidden="true" className="absolute inset-0" width={width} height={height}>
          {team.orders.flatMap((order) =>
            (order.dependsOn ?? []).flatMap((key) => {
              const dep = byKey.get(key);
              const from = dep ? rowOf.get(dep.workOrderId) : undefined;
              const to = rowOf.get(order.workOrderId);
              if (!from || !to) return [];
              const x1 = nodeX(from.col) + 144;
              const y1 = nodeY(from.row) + NODE_H / 2;
              const x2 = nodeX(to.col);
              const y2 = nodeY(to.row) + NODE_H / 2;
              const bend = Math.max(24, Math.abs(x2 - x1) / 2);
              return [
                <path
                  key={`${order.workOrderId}:${key}`}
                  d={`M ${x1} ${y1} C ${x1 + bend} ${y1}, ${x2 - bend} ${y2}, ${x2} ${y2}`}
                  fill="none"
                  stroke="var(--color-card-border)"
                  strokeWidth="1.5"
                />,
              ];
            }),
          )}
        </svg>
        {team.orders.map((order) => {
          const pos = rowOf.get(order.workOrderId);
          if (!pos) return null;
          return (
            <div
              key={order.workOrderId}
              className="absolute flex flex-col justify-center gap-0.5 rounded-lg border border-card-border bg-card px-2 py-1"
              style={{ left: nodeX(pos.col), top: nodeY(pos.row), width: 144, minHeight: NODE_H }}
              title={order.taskPreview}
            >
              <span className="flex min-w-0 items-center gap-1.5">
                <span
                  aria-hidden="true"
                  className={cn("size-1.5 shrink-0 rounded-full", STATUS_DOT[ORDER_STATUS_DOT[order.status]])}
                />
                <span className="min-w-0 truncate font-mono text-ui-2xs text-foreground-subtle">
                  {order.taskKey ?? order.workOrderId.slice(0, 8)}
                </span>
              </span>
              <span className="truncate text-ui-2xs text-foreground-subtlest">
                {order.agentName}
                {order.status === "in_flight" && order.dependsOn?.length
                  ? ` · ${intl.formatMessage({
                      id: order.unlocked ? "chat.teamBoard.unlocked" : "chat.teamBoard.locked",
                    })}`
                  : ""}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function TeamCard({
  team,
  sessionId,
  onToggleAutoFlow,
  onOpenTeamDesk,
}: {
  team: TeamBoardTeam;
  sessionId: string;
  onToggleAutoFlow?: (batchId: string, enabled: boolean) => void;
  onOpenTeamDesk?: (request: { sessionId: string; deskSessionId: string; deskName: string }) => void;
}) {
  const { intl } = useZCodeIntl();
  const progress = teamProgress(team);
  return (
    <section
      className="flex flex-col gap-2 rounded-xl border border-card-border bg-card p-3 shadow-xs"
      data-testid="team-board-team"
    >
      <div className="flex min-w-0 items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-ui-sm font-medium text-foreground" title={team.title}>
          {team.title ?? team.batchId.slice(0, 8)}
        </span>
        {team.review ? (
          <span className="shrink-0 rounded-[4px] bg-muted px-1.5 text-ui-2xs leading-4 text-foreground-subtle">
            {intl.formatMessage({ id: "chat.teamBoard.reviewChip" })}
          </span>
        ) : null}
        {onToggleAutoFlow ? (
          <button
            type="button"
            onClick={() => onToggleAutoFlow(team.batchId, team.autoFlow !== true)}
            data-testid="team-board-autoflow"
            className={cn(
              "shrink-0 cursor-pointer rounded-[4px] px-1.5 text-ui-2xs font-medium leading-4 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30",
              team.autoFlow === true
                ? "bg-brand/10 text-brand"
                : "bg-muted text-foreground-subtle hover:bg-surface-hover hover:text-foreground",
            )}
            title={intl.formatMessage({ id: "chat.teamBoard.autoFlowHint" })}
          >
            {intl.formatMessage({
              id: team.autoFlow === true ? "chat.teamBoard.autoFlowOn" : "chat.teamBoard.autoFlowOff",
            })}
          </button>
        ) : null}
        {team.escalated ? (
          <span
            className="shrink-0 rounded-[4px] bg-warning/10 px-1.5 text-ui-2xs font-medium leading-4 text-warning"
            title={intl.formatMessage({ id: "chat.teamBoard.escalatedHint" })}
          >
            {intl.formatMessage({ id: "chat.teamBoard.escalated" })}
          </span>
        ) : null}
        {team.qc ? (
          <span
            className={cn(
              "shrink-0 rounded-[4px] px-1.5 text-ui-2xs font-medium leading-4",
              QC_CLASS[team.qc],
            )}
          >
            {intl.formatMessage({ id: QC_MESSAGE[team.qc] })}
          </span>
        ) : null}
        <span className="shrink-0 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
          {progress.completed}/{progress.total}
        </span>
      </div>
      {team.members.length ? (
        <div className="flex min-w-0 flex-wrap items-center gap-1.5" data-testid="team-board-members">
          {team.members.map((member) => (
            <MemberChip key={member.agentId ?? member.name} name={member.name} live={member.live} model={member.model} />
          ))}
        </div>
      ) : null}
      {teamHasDependencies(team.orders) ? <TeamDag team={team} /> : null}
      <ul className="flex flex-col divide-y divide-card-border/40">
        {team.orders.map((order) => (
          <OrderRow key={order.workOrderId} order={order} />
        ))}
      </ul>
      {team.messages?.length ? (
        <div className="flex flex-col gap-1 rounded-lg bg-muted/40 p-2" data-testid="team-board-comms">
          <span className="text-ui-2xs font-medium text-foreground-subtle">
            {intl.formatMessage({ id: "chat.teamBoard.comms" })}
          </span>
          {team.messages.slice(-6).map((entry, index) => (
            <button
              key={`${entry.createdAt}:${index}`}
              type="button"
              disabled={!entry.toSessionId || !onOpenTeamDesk}
              onClick={() =>
                entry.toSessionId &&
                onOpenTeamDesk?.({
                  sessionId,
                  deskSessionId: entry.toSessionId,
                  deskName: entry.to,
                })
              }
              className={cn(
                "min-w-0 break-words text-left text-ui-2xs leading-4 text-foreground-subtlest transition-colors",
                entry.toSessionId && onOpenTeamDesk
                  ? "cursor-pointer hover:text-foreground"
                  : "cursor-default",
              )}
            >
              <span className="font-medium text-foreground-subtle">{entry.from}</span>
              {" → "}
              <span className="font-medium text-foreground-subtle">{entry.to}</span>
              {": "}
              {entry.text}
            </button>
          ))}
        </div>
      ) : null}
    </section>
  );
}

export function TeamBoardSidePane({
  tab,
  focused,
  onOpenTeamDesk,
}: {
  tab: TeamBoardSidePaneTab;
  focused: boolean;
  onOpenTeamDesk?: (request: {
    sessionId: string;
    deskSessionId: string;
    deskName: string;
  }) => void;
}) {
  const { intl } = useZCodeIntl();
  const { sendCommand } = useV4Conversation();
  const [snapshot, setSnapshot] = useState<TeamBoardSnapshot | undefined>(undefined);
  const fetchSnapshot = useCallback(async () => {
    try {
      const ack = await sendCommand(
        createCommandEnvelope({
          type: "teamBoardState",
          payload: {},
          sessionId: tab.sessionId,
        }),
      );
      if (ack.status === "accepted" && ack.result?.type === "teamBoardState") {
        setSnapshot(ack.result.snapshot);
      }
    } catch {
      // 拉取失败沿用上一帧（面板不闪空态），下个轮询窗口再试。
    }
  }, [sendCommand, tab.sessionId]);
  useEffect(() => {
    void fetchSnapshot();
  }, [fetchSnapshot]);
  useEffect(() => {
    if (!focused) return;
    const timer = setInterval(() => {
      void fetchSnapshot();
    }, POLL_MS);
    return () => clearInterval(timer);
  }, [focused, fetchSnapshot]);
  const toggleAutoFlow = useCallback(
    (batchId: string, enabled: boolean) => {
      void sendCommand(
        createCommandEnvelope({
          type: "teamAutoFlow",
          payload: { batchId, enabled },
          sessionId: tab.sessionId,
        }),
      )
        .then((ack) => {
          if (ack.status === "accepted" && ack.result?.type === "teamAutoFlow") {
            const enabled = ack.result.enabled;
            setSnapshot((previous) =>
              previous === undefined
                ? previous
                : {
                    ...previous,
                    teams: previous.teams.map((team) =>
                      team.batchId === batchId ? { ...team, autoFlow: enabled } : team,
                    ),
                  },
            );
          }
        })
        .catch(() => {
          // 下个轮询窗口以台账权威状态刷新，本地不猜。
        });
    },
    [sendCommand, tab.sessionId],
  );

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto px-4 py-3" data-testid="team-board-side-pane">
      <div className="flex items-center gap-2 text-ui-sm font-medium text-foreground-subtle">
        <SquareKanbanIcon className="size-4 shrink-0" aria-hidden="true" />
        {intl.formatMessage({ id: "sidePane.teamBoard" })}
      </div>
      {!snapshot ? (
        <p className="text-ui-xs text-foreground-subtlest">
          {intl.formatMessage({ id: "sidePane.teamBoard.loading" })}
        </p>
      ) : (
        <>
          {(snapshot.plans?.length ?? 0) > 0 ? (
            <p className="text-ui-2xs font-medium uppercase tracking-wide text-foreground-subtlest">
              {intl.formatMessage({ id: "chat.teamBoard.plans" })}
            </p>
          ) : null}
          {snapshot.plans?.map((plan) => (
            <TeamBoardPlanCard
              key={plan.planId}
              plan={plan}
              roster={snapshot.roster ?? []}
              sessionId={tab.sessionId}
              sendCommand={sendCommand}
              onSettled={fetchSnapshot}
            />
          ))}
          {snapshot.teams.length === 0 && (snapshot.plans?.length ?? 0) === 0 ? (
            <p className="text-ui-xs text-foreground-subtlest">
              {intl.formatMessage({ id: "sidePane.teamBoard.empty" })}
            </p>
          ) : null}
          {snapshot.teams.map((team) => (
            <TeamCard
              key={team.batchId}
              team={team}
              sessionId={tab.sessionId}
              onToggleAutoFlow={toggleAutoFlow}
              onOpenTeamDesk={onOpenTeamDesk}
            />
          ))}
        </>
      )}
    </div>
  );
}
