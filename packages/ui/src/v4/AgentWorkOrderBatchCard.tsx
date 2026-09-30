/**
 * 批次派单的工地卡：同批 2+ 张工单聚合为一张卡（批次标题 + 每单一行 + 总进度），
 * 取代发起方会话里散落的单张回执卡。设计语言完全对齐工作流 run 卡
 * （WorkflowRunCompactCard 的卡壳/字阶 + run-status-presentation 的状态灯词表 +
 * 工作流时间线的站灯连轨）——全产品只有一套「运行中」的画法。数据来自
 * selectWorkOrderBatches（纯函数，agentWorkOrderBatch.ts），这里只管画。
 */
import { cn } from "@/components/lib/utils.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { WorkflowIcon } from "lucide-react";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import type { WorkOrderBatchModel, WorkOrderBatchOrder } from "@/v4/agentWorkOrderBatch.js";
import { ReceiptForwardControl } from "@/v4/ReceiptForwardControl.js";
import { ReceiptFailureNotice } from "@/v4/ReceiptFailureNotice.js";
import type { ConversationRowRenderContext } from "@/v4/conversationRowContext.js";

const STATUS_MESSAGE_ID: Record<WorkOrderBatchOrder["status"], string> = {
  dispatched: "chat.workOrderBatch.status.dispatched",
  queued: "chat.workOrderBatch.status.queued",
  completed: "chat.workOrderBatch.status.completed",
  failed: "chat.workOrderBatch.status.failed",
};

/** 状态词的语义色：与状态灯同一套判断（状态永远有词，不只靠颜色）。 */
const STATUS_TEXT: Record<WorkOrderBatchOrder["status"], string> = {
  dispatched: "text-warning",
  queued: "text-foreground-subtle",
  completed: "text-success",
  failed: "text-destructive",
};

/** 批次状态 → 工作流状态灯词表（StepRunStatus 四值）。 */
const STATUS_DOT_KEY: Record<WorkOrderBatchOrder["status"], keyof typeof STATUS_DOT> = {
  dispatched: "running",
  queued: "pending",
  completed: "done",
  failed: "failed",
};

export function AgentWorkOrderBatchCard({
  batch,
  context,
}: {
  batch: WorkOrderBatchModel;
  /** 行内转交（接力收口）：宿主轮的派单能力与名册都从这里来；缺席=不渲染转交。 */
  context?: ConversationRowRenderContext;
}) {
  const { intl } = useZCodeIntl();
  const title = batch.title?.trim();
  const completed = batch.orders.filter((order) => order.status === "completed").length;
  const failed = batch.orders.some((order) => order.status === "failed");
  const allSettled = completed + batch.orders.filter((o) => o.status === "failed").length === batch.orders.length;
  // 整卡状态灯：有失败且全部收口 → failed；全部收口 → done；否则在跑。
  const overallDot = allSettled && failed ? "failed" : allSettled ? "done" : "running";

  return (
    <div
      className="w-full overflow-hidden rounded-xl border border-card-border bg-card shadow-xs"
      data-testid="agent-work-order-batch-card"
      data-batch-id={batch.batchId}
    >
      {/* 头行照 WorkflowRunCompactCard：图标 + 标签 + 标题 + 右侧等宽进度/状态词。 */}
      <div className="flex w-full min-w-0 items-center gap-2 px-3 py-2.5">
        <WorkflowIcon className="size-3.5 shrink-0 text-foreground-subtle" aria-hidden="true" />
        <span className="shrink-0 text-ui-base font-medium text-foreground-subtle">
          {intl.formatMessage({ id: "chat.workOrderBatch.title" })}
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
            {completed}/{batch.orders.length}
          </span>
          <span className="text-ui-sm text-foreground-subtle">
            {intl.formatMessage({ id: "chat.workOrderBatch.progress" })}
          </span>
        </span>
      </div>
      {/* 行区：工作流时间线的站灯连轨——左侧灯列一线贯穿，首末行裁到灯位。 */}
      <div role="list" className="border-t border-card-border/60 px-3 py-1">
        {batch.orders.map((order, index) => (
          <div
            role="listitem"
            key={order.key}
            data-order-key={order.key}
            className="flex min-w-0 flex-col"
          >
          <div
            className="relative flex min-w-0 flex-wrap items-center gap-2 py-1.5"
          >
            <span aria-hidden="true" className="relative flex w-3 shrink-0 self-stretch">
              <span
                className={cn(
                  "absolute left-1/2 w-px -translate-x-1/2 bg-card-border/70",
                  index === 0
                    ? "bottom-0 top-1/2"
                    : index === batch.orders.length - 1
                      ? "top-0 h-1/2"
                      : "inset-y-0",
                )}
              />
              <span
                className={cn(
                  "absolute left-1/2 top-1/2 size-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full",
                  STATUS_DOT[STATUS_DOT_KEY[order.status]],
                )}
              />
            </span>
            <span
              className={cn(
                "flex h-5 shrink-0 items-center gap-1 rounded-[4px] px-1.5 leading-none",
                order.agentName
                  ? SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(order.agentName)]
                  : "bg-muted text-foreground-subtle",
              )}
            >
              <span className="max-w-24 truncate text-ui-xs font-medium">
                {order.agentName || "—"}
              </span>
            </span>
            <span className={cn("shrink-0 text-ui-xs", STATUS_TEXT[order.status])}>
              {intl.formatMessage({ id: STATUS_MESSAGE_ID[order.status] })}
            </span>
            {order.receiptText &&
            !(
              order.status === "failed" &&
              (order.failureCode || order.failureReason || order.retried)
            ) ? (
              // 失败行有结构化线索时改讲大白话（ReceiptFailureNotice），摘要让位；
              // 旧 CLI 的失败行没有线索，照旧展示摘要。
              <span className="min-w-0 flex-1 line-clamp-2 whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtlest">
                {order.receiptText}
              </span>
            ) : (
              <span className="min-w-0 flex-1" />
            )}
            {context && order.status === "completed" && order.receiptTitle && order.receiptAnswer ? (
              // 接力收口：批次成员的散回执卡已被工地卡代言，转交入口跟着搬进行内
              // （inline 变体：收起=行尾按钮，展开=flex-wrap 换行占满整行）。
              <ReceiptForwardControl
                inline
                title={order.receiptTitle}
                answer={order.receiptAnswer}
                context={context}
                unitKey={`batch:${batch.batchId}:${order.key}`}
              />
            ) : null}
            {context && order.status === "failed" && order.receiptTitle ? (
              // 一键重派（2026-10-01 员工可靠性批）：失败行讲清大白话失败原因，
              // 原员工 + 原任务一键再派；无结构化线索（旧 CLI）时组件返回 null。
              <ReceiptFailureNotice
                inline
                title={order.receiptTitle}
                meta={order}
                context={context}
                unitKey={`batch:${batch.batchId}:${order.key}`}
              />
            ) : null}
          </div>
          </div>
        ))}
      </div>
    </div>
  );
}
