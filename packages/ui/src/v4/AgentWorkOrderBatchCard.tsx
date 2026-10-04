/**
 * 批次派单卡（2026-10-02 重设计）：一张卡两种构图，按批次口味分流——
 * · 评审会批（batch.review）= 圆桌：评审对象坐中间（桌牌），评审员围坐两侧、横线
 *   联到桌；意见按「发言记录」列在桌下；合议是末席（Gavel）。像一场真的会。
 * · 施工批 = 工地竖轨：工作流时间线同款的纵向站轨（灯骑轨、芯片随行、证据内联），
 *   质检/合议是末站。
 * 两构图共用一套视觉词表：STATUS_DOT 四值灯、员工芯片色板、卡壳（图标+种类词+
 * 标题+右侧等宽进度）。数据来自 selectWorkOrderBatches（纯函数），这里只管画。
 * 「转交」已随 2026-10-02 的接力收口决策移除（见 workOrderForward.ts 头注）。
 */
import { useState, type ReactNode } from "react";
import {
  ChevronDown,
  ChevronRight,
  Clock,
  Gavel,
  UsersRound,
  Workflow as WorkflowIcon,
} from "lucide-react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import { stationLampClass } from "@/components/workflow-timeline/WorkflowTimelineLedge.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import { ReceiptFailureNotice } from "@/v4/ReceiptFailureNotice.js";
import { formatBatchDuration } from "@/v4/agentWorkOrderBatch.js";
import type { ConversationRowRenderContext } from "@/v4/conversationRowContext.js";
import type {
  ReviewVerdict,
  WorkOrderBatchModel,
  WorkOrderBatchOrder,
} from "@/v4/agentWorkOrderBatch.js";

const STATUS_MESSAGE_ID: Record<WorkOrderBatchOrder["status"], string> = {
  dispatched: "chat.workOrderBatch.status.dispatched",
  queued: "chat.workOrderBatch.status.queued",
  completed: "chat.workOrderBatch.status.completed",
  cancelled: "chat.workOrderBatch.status.cancelled",
  failed: "chat.workOrderBatch.status.failed",
};

/** 站状态 → 工作流状态灯词表（StepRunStatus 四值）。状态词一律安静灰，颜色只留给灯。 */
const STATUS_DOT_KEY: Record<WorkOrderBatchOrder["status"], keyof typeof STATUS_DOT> = {
  dispatched: "running",
  queued: "pending",
  completed: "done",
  cancelled: "pending",
  failed: "failed",
};

/** 收纳状态按批次记（会话流滚动会重挂卡片，组件内 useState 会丢）：会话生命周期内有效。 */
const collapsedBatches = new Map<string, boolean>();

const QC_STATE_DOT: Record<
  NonNullable<WorkOrderBatchModel["qc"]>["state"],
  keyof typeof STATUS_DOT
> = { running: "running", done: "done", failed: "failed" };

const QC_STATE_MESSAGE: Record<NonNullable<WorkOrderBatchModel["qc"]>["state"], string> = {
  running: "chat.workOrderBatch.qc.running",
  done: "chat.workOrderBatch.qc.done",
  failed: "chat.workOrderBatch.qc.failed",
};

/** 合议（评审会批）的状态词：同一盏灯、同一套三态，词表换成合议口味。 */
const QC_REVIEW_STATE_MESSAGE: Record<NonNullable<WorkOrderBatchModel["qc"]>["state"], string> = {
  running: "chat.workOrderBatch.qcReview.running",
  done: "chat.workOrderBatch.qcReview.done",
  failed: "chat.workOrderBatch.qcReview.failed",
};

const VERDICT_MESSAGE: Record<ReviewVerdict, string> = {
  pass: "chat.workOrderBatch.review.verdict.pass",
  fail: "chat.workOrderBatch.review.verdict.fail",
  conditional: "chat.workOrderBatch.review.verdict.conditional",
};

/** 评审结论徽章配色：绿=通过、红=不通过、琥珀=有条件（颜色只说结论，不说人）。 */
const VERDICT_CLASS: Record<ReviewVerdict, string> = {
  pass: "bg-success/10 text-success",
  fail: "bg-destructive/10 text-destructive",
  conditional: "bg-warning/10 text-warning",
};

/** 员工芯片（两构图共用）：档案色板按名取色，无名退灰。 */
function AgentChip({ name }: { name: string }) {
  return (
    <span
      className={cn(
        "flex h-5 max-w-full shrink-0 items-center rounded-[4px] px-1.5 leading-none",
        name
          ? SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(name)]
          : "bg-muted text-foreground-subtle",
      )}
    >
      <span className="max-w-28 truncate text-ui-xs font-medium">{name || "—"}</span>
    </span>
  );
}

/** 卡壳表头：图标 + 种类词 + 标题 + 右侧等宽进度与整卡状态灯 + 收纳开关（两构图共用）。 */
function CardShell({
  icon,
  kindId,
  title,
  completed,
  total,
  durationMs,
  overallDot,
  collapsed,
  onToggleCollapsed,
  children,
}: {
  icon: ReactNode;
  kindId: string;
  title?: string;
  completed: number;
  total: number;
  /** 收口后的墙钟工时；缺席（在跑/时间残缺）不渲染——不画假数字。 */
  durationMs?: number;
  overallDot: keyof typeof STATUS_DOT;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  children: ReactNode;
}) {
  const { intl } = useZCodeIntl();
  return (
    <div
      className="w-full overflow-hidden rounded-xl border border-card-border bg-card shadow-xs"
      data-testid="agent-work-order-batch-card"
    >
      {/* 整条头栏都是开关（老板拍板：不找小箭头）；chevron 只是状态指示器。 */}
      <button
        type="button"
        onClick={onToggleCollapsed}
        aria-expanded={!collapsed}
        data-testid="agent-work-order-batch-collapse"
        className="flex w-full min-w-0 cursor-pointer items-center gap-2 border-b border-card-border/60 px-3.5 py-3 text-left font-[inherit] transition-colors hover:bg-surface-hover/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
      >
        {icon}
        <span className="shrink-0 text-ui-base font-medium text-foreground-subtle">
          {intl.formatMessage({ id: kindId })}
        </span>
        {title ? (
          <span className="min-w-0 flex-1 truncate text-ui-base text-foreground" title={title}>
            {title}
          </span>
        ) : (
          <span className="min-w-0 flex-1" />
        )}
        <span className="flex shrink-0 items-center gap-1.5">
          <span
            aria-hidden="true"
            className={cn("size-1.5 shrink-0 rounded-full", STATUS_DOT[overallDot])}
          />
          <span className="font-mono text-ui-xs tabular-nums text-foreground-subtlest">
            {completed}/{total}
          </span>
          <span className="text-ui-sm text-foreground-subtle">
            {intl.formatMessage({ id: "chat.workOrderBatch.progress" })}
          </span>
          {durationMs !== undefined ? (
            <span
              className="flex items-center gap-1 font-mono text-ui-xs tabular-nums text-foreground-subtlest"
              data-testid="agent-work-order-batch-duration"
            >
              <Clock aria-hidden="true" className="size-3" />
              {formatBatchDuration(durationMs)}
            </span>
          ) : null}
        </span>
        <span
          aria-hidden="true"
          className="flex size-6 shrink-0 items-center justify-center text-foreground-subtle"
        >
          {collapsed ? (
            <ChevronRight className="size-3.5" />
          ) : (
            <ChevronDown className="size-3.5" />
          )}
        </span>
      </button>
      {collapsed ? null : children}
    </div>
  );
}

/** 失败行的大白话与一键重派（两构图共用；数据或能力缺席时组件自返回 null）。 */
function FailureNotice({
  order,
  batch,
  context,
}: {
  order: WorkOrderBatchOrder;
  batch: WorkOrderBatchModel;
  context?: ConversationRowRenderContext;
}) {
  if (!context || order.status !== "failed" || !order.receiptTitle) return null;
  return (
    <ReceiptFailureNotice
      inline
      title={order.receiptTitle}
      meta={order}
      context={context}
      unitKey={`batch:${batch.batchId}:${order.key}`}
      batchId={batch.batchId}
      batchTitle={batch.title}
    />
  );
}

// ── 工地（施工批）：纵向站轨 ────────────────────────────────────────
// 灯骑轨、芯片与状态词随行、交活摘要内联一行、失败行自带大白话与重派；
// 质检/合议是末站。批次通常 2~4 单，纵排高度可接受（ponytail：真到几十单再谈折叠）。

function BuildSiteBody({
  batch,
  context,
}: {
  batch: WorkOrderBatchModel;
  context?: ConversationRowRenderContext;
}) {
  const { intl } = useZCodeIntl();
  return (
    <div className="relative px-3.5 py-1" data-testid="agent-work-order-batch-rail">
      <span aria-hidden="true" className="absolute bottom-4 left-[21px] top-4 w-px bg-card-border/60" />
      <ul>
        {batch.orders.map((order) => (
          <li key={order.key} className="relative" data-testid="agent-work-order-batch-station">
            <span
              aria-hidden="true"
              className={cn(
                "absolute left-[21px] top-1/2 z-10 size-[9px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-card",
                stationLampClass(STATUS_DOT_KEY[order.status]),
              )}
            />
            <div className="flex min-w-0 items-center gap-2 py-2.5 pl-8 pr-1">
              <AgentChip name={order.agentName} />
              <span className="shrink-0 text-ui-xs text-foreground-subtlest">
                {intl.formatMessage({ id: STATUS_MESSAGE_ID[order.status] })}
              </span>
              {order.receiptText && order.status !== "failed" ? (
                <span className="min-w-0 flex-1 truncate text-ui-xs text-foreground-subtle">
                  {order.receiptText}
                </span>
              ) : (
                <span className="min-w-0 flex-1" />
              )}
            </div>
            {order.status === "failed" ? (
              <div className="pb-2 pl-8">
                <FailureNotice order={order} batch={batch} context={context} />
              </div>
            ) : null}
          </li>
        ))}
        {batch.qc ? (
          <li className="relative" data-testid="agent-work-order-batch-qc-station">
            <span
              aria-hidden="true"
              className={cn(
                "absolute left-[21px] top-1/2 z-10 size-[9px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-card",
                stationLampClass(QC_STATE_DOT[batch.qc.state]),
              )}
            />
            <div className="flex min-w-0 items-center gap-2 py-2.5 pl-8 pr-1">
              <span className="flex h-5 shrink-0 items-center rounded-[4px] bg-muted px-1.5 leading-none text-foreground-subtle">
                <span className="text-ui-xs font-medium">
                  {intl.formatMessage({
                    id: batch.qc.review
                      ? "chat.workOrderBatch.qcReview.station"
                      : "chat.workOrderBatch.qc.station",
                  })}
                </span>
              </span>
              <span className="text-ui-xs text-foreground-subtlest">
                {intl.formatMessage({
                  id: (batch.qc.review ? QC_REVIEW_STATE_MESSAGE : QC_STATE_MESSAGE)[batch.qc.state],
                })}
              </span>
            </div>
          </li>
        ) : null}
      </ul>
    </div>
  );
}

// ── 评审会（圆桌）───────────────────────────────────────────────────
// 评审对象坐中间（品牌色桌牌），评审员围坐两侧、横线联到桌；结论徽章只认回执
// 第一行格式行（提取不到不戴，不从自由文本猜）；意见按「发言记录」列出，失败的
// 评审自带大白话与重派且排最前（失败必须被看见）；合议是末席，评审到齐前明说
// 「自动合议」，不画假状态。

function ReviewSeat({ order, align }: { order: WorkOrderBatchOrder; align: "start" | "end" }) {
  const { intl } = useZCodeIntl();
  return (
    <div
      className={cn("flex max-w-44 flex-col gap-1", align === "end" ? "items-end" : "items-start")}
      data-testid="agent-work-order-review-seat"
    >
      <AgentChip name={order.agentName} />
      <span className="flex items-center gap-1.5">
        <span
          aria-hidden="true"
          className={cn("size-1.5 rounded-full", STATUS_DOT[STATUS_DOT_KEY[order.status]])}
        />
        <span className="text-ui-2xs text-foreground-subtlest">
          {intl.formatMessage({ id: STATUS_MESSAGE_ID[order.status] })}
        </span>
        {order.verdict ? (
          <span
            className={cn(
              "rounded-[4px] px-1.5 text-ui-2xs font-medium leading-4",
              VERDICT_CLASS[order.verdict],
            )}
          >
            {intl.formatMessage({ id: VERDICT_MESSAGE[order.verdict] })}
          </span>
        ) : null}
      </span>
    </div>
  );
}

function ReviewCouncilBody({
  batch,
  context,
}: {
  batch: WorkOrderBatchModel;
  context?: ConversationRowRenderContext;
}) {
  const { intl } = useZCodeIntl();
  const title = batch.title?.trim();
  const noted = batch.orders.filter(
    (order) => order.receiptText || order.status === "failed",
  );
  // ponytail: 评审会 taught 2~3 人；席位最多画 4 位，超出折一张 +N（真出现再谈布局）。
  const seats = batch.orders.slice(0, 4);
  const overflow = batch.orders.length - seats.length;
  const leftSeats = seats.filter((_, index) => index % 2 === 0);
  const rightSeats = seats.filter((_, index) => index % 2 === 1);
  const failedFirst = [...noted].sort(
    (a, b) => Number(a.status === "failed") - Number(b.status === "failed"),
  );
  return (
    <div>
      <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-x-3 px-3.5 pb-1 pt-3">
        <div className="flex flex-col items-end gap-2.5">
          {leftSeats.map((order) => (
            <ReviewSeat key={order.key} order={order} align="end" />
          ))}
        </div>
        <div className="relative flex flex-col items-center">
          <span
            aria-hidden="true"
            className="absolute right-full top-1/2 mr-3 h-px w-4 bg-card-border/70"
          />
          <span
            aria-hidden="true"
            className="absolute left-full top-1/2 ml-3 h-px w-4 bg-card-border/70"
          />
          <div className="min-w-36 max-w-48 rounded-2xl border border-brand/25 bg-brand/5 px-4 py-2.5 text-center">
            <div className="text-ui-2xs text-foreground-subtlest">
              {intl.formatMessage({ id: "chat.workOrderBatch.review.subject" })}
            </div>
            <div className="mt-0.5 line-clamp-2 text-ui-sm font-medium text-foreground" title={title}>
              {title || "—"}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-start gap-2.5">
          {rightSeats.map((order) => (
            <ReviewSeat key={order.key} order={order} align="start" />
          ))}
        </div>
      </div>
      {overflow > 0 ? (
        <div className="px-3.5 pb-1 text-center text-ui-2xs text-foreground-subtlest">+{overflow}</div>
      ) : null}
      {/* 末席：合议。 */}
      <div className="mx-3.5 mt-2 flex items-center gap-2 border-t border-card-border/60 py-2">
        <Gavel className="size-3.5 shrink-0 text-foreground-subtle" aria-hidden="true" />
        <span className="text-ui-xs font-medium text-foreground-subtle">
          {intl.formatMessage({ id: "chat.workOrderBatch.qcReview.station" })}
        </span>
        <span className="min-w-0 flex-1" />
        {batch.qc ? (
          <span className="flex items-center gap-1.5">
            <span
              aria-hidden="true"
              className={cn("size-1.5 rounded-full", STATUS_DOT[QC_STATE_DOT[batch.qc.state]])}
            />
            <span className="text-ui-xs text-foreground-subtlest">
              {intl.formatMessage({ id: QC_REVIEW_STATE_MESSAGE[batch.qc.state] })}
            </span>
          </span>
        ) : (
          <span className="text-ui-2xs text-foreground-subtlest">
            {intl.formatMessage({ id: "chat.workOrderBatch.review.councilWaiting" })}
          </span>
        )}
      </div>
      {failedFirst.length > 0 ? (
        <div className="flex flex-col gap-2 border-t border-card-border/60 px-3.5 py-2.5">
          <div className="text-ui-2xs font-medium text-foreground-subtlest">
            {intl.formatMessage({ id: "chat.workOrderBatch.review.notes" })}
          </div>
          {failedFirst.map((order) => (
            <div key={order.key} data-testid="agent-work-order-review-note">
              {order.status === "failed" ? (
                <FailureNotice order={order} batch={batch} context={context} />
              ) : (
                <div className="flex min-w-0 items-start gap-2">
                  <AgentChip name={order.agentName} />
                  {order.verdict ? (
                    <span
                      className={cn(
                        "mt-0.5 shrink-0 rounded-[4px] px-1.5 text-ui-2xs font-medium leading-4",
                        VERDICT_CLASS[order.verdict],
                      )}
                    >
                      {intl.formatMessage({ id: VERDICT_MESSAGE[order.verdict] })}
                    </span>
                  ) : null}
                  <p className="min-w-0 flex-1 whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtle">
                    {order.receiptText}
                  </p>
                </div>
              )}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function AgentWorkOrderBatchCard({
  batch,
  context,
}: {
  batch: WorkOrderBatchModel;
  /** 行内重派能力与名册都从这里来；缺席=动作位不渲染。 */
  context?: ConversationRowRenderContext;
}) {
  const [collapsed, setCollapsed] = useState(() => collapsedBatches.get(batch.batchId) ?? false);
  const toggleCollapsed = () => {
    setCollapsed((current) => {
      const next = !current;
      collapsedBatches.set(batch.batchId, next);
      return next;
    });
  };
  const completed = batch.orders.filter((order) => order.status === "completed").length;
  const failed = batch.orders.some((order) => order.status === "failed");
  const settledCount = batch.orders.filter(
    (order) => order.status !== "dispatched" && order.status !== "queued",
  ).length;
  const allSettled = settledCount === batch.orders.length;
  const overallDot: keyof typeof STATUS_DOT = allSettled && failed ? "failed" : allSettled ? "done" : "running";
  // 用时只在批次收口后画（AgentCore 状态条同款）：在跑时不显倒计时假数字。
  const durationMs =
    allSettled && batch.startedAtMs !== undefined && batch.endedAtMs !== undefined
      ? Math.max(0, batch.endedAtMs - batch.startedAtMs)
      : undefined;
  return (
    <CardShell
      icon={
        batch.review ? (
          <UsersRound className="size-4 shrink-0 text-foreground-subtle" aria-hidden="true" />
        ) : (
          <WorkflowIcon className="size-4 shrink-0 text-foreground-subtle" aria-hidden="true" />
        )
      }
      kindId={batch.review ? "chat.workOrderBatch.review.title" : "chat.workOrderBatch.title"}
      title={batch.title}
      completed={completed}
      total={batch.orders.length}
      durationMs={durationMs}
      overallDot={overallDot}
      collapsed={collapsed}
      onToggleCollapsed={toggleCollapsed}
    >
      {batch.review ? (
        <ReviewCouncilBody batch={batch} context={context} />
      ) : (
        <BuildSiteBody batch={batch} context={context} />
      )}
    </CardShell>
  );
}
