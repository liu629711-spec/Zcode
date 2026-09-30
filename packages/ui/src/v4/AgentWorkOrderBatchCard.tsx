/**
 * 批次派单的工地卡：同批 2+ 张工单聚合为一张卡（批次标题 + 每单一行 + 总进度），
 * 取代发起方会话里散落的单张回执卡。视觉语言照 AgentWorkOrderTurnCard（同面、
 * 圆角边框、哑光底色）；行内工牌与侧栏/@ 面板同一枚画法。数据来自
 * selectWorkOrderBatches（纯函数，agentWorkOrderBatch.ts），这里只管画。
 */
import { cn } from "@/components/lib/utils.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import type { WorkOrderBatchModel, WorkOrderBatchOrder } from "@/v4/agentWorkOrderBatch.js";

const STATUS_MESSAGE_ID: Record<WorkOrderBatchOrder["status"], string> = {
  dispatched: "chat.workOrderBatch.status.dispatched",
  queued: "chat.workOrderBatch.status.queued",
  completed: "chat.workOrderBatch.status.completed",
  failed: "chat.workOrderBatch.status.failed",
};

export function AgentWorkOrderBatchCard({ batch }: { batch: WorkOrderBatchModel }) {
  const { intl } = useZCodeIntl();
  const title = batch.title?.trim();
  const completed = batch.orders.filter((order) => order.status === "completed").length;

  return (
    <div
      className="flex w-full flex-col gap-1.5 rounded-xl border border-border bg-surface/60 py-2.5 pl-3 pr-3"
      data-testid="agent-work-order-batch-card"
      data-batch-id={batch.batchId}
    >
      <div className="flex min-w-0 items-center gap-2">
        <span className="shrink-0 text-ui-xs font-medium text-foreground-subtle">
          {intl.formatMessage({ id: "chat.workOrderBatch.title" })}
        </span>
        {title ? (
          <span className="min-w-0 truncate text-ui-xs font-medium">{title}</span>
        ) : null}
        <span className="ml-auto shrink-0 text-ui-xs text-foreground-subtle">
          {intl.formatMessage(
            { id: "chat.workOrderBatch.progress" },
            { completed, total: batch.orders.length },
          )}
        </span>
      </div>
      <div role="list" className="flex flex-col gap-1.5">
        {batch.orders.map((order) => (
          <div
            role="listitem"
            key={order.key}
            data-order-key={order.key}
            className="flex min-w-0 items-center gap-2"
          >
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
            <span className="shrink-0 text-ui-xs text-foreground-subtle">
              {intl.formatMessage({ id: STATUS_MESSAGE_ID[order.status] })}
            </span>
            {order.receiptText ? (
              <span className="min-w-0 line-clamp-2 whitespace-pre-wrap break-words text-ui-xs leading-4 text-foreground-subtle">
                {order.receiptText}
              </span>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}
