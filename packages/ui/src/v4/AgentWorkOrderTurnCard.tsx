/**
 * 目标会话的工单卡（D29/D5）：「来自 <源智能体> 的工单」+ 任务正文。
 *
 * 来源信封式呈现（照 Claude Code 来源信封铁律）：整块卡片缩进 + 边框，来源行用
 * 智能体名牌（与侧栏/@ 面板同一枚画法）标注谁派的单——防对方输出冒充本会话指令。
 * 发起方是用户本人（UI 派单）时改标「来自用户的工单」。卡片是这轮在目标会话的
 * 全部呈现（该轮没有可见用户行），轮内助手回答照常走既有流程渲染。
 */
import { UserIcon } from "lucide-react";
import { cn } from "@/components/lib/utils.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import type { AgentWorkOrderMeta } from "@zcode/shared/zcode-protocol-v4";

export function AgentWorkOrderTurnCard({ meta }: { meta: AgentWorkOrderMeta }) {
  const { intl } = useZCodeIntl();
  const fromAgentName = meta.fromAgentName.trim();
  const isFromUser = fromAgentName.length === 0;

  return (
    <div
      className="flex w-full flex-col gap-1.5 rounded-xl border border-border bg-surface/60 py-2.5 pl-3 pr-3"
      data-testid="agent-work-order-turn-card"
      data-work-order-id={meta.workOrderId}
      data-from-agent={fromAgentName || "user"}
    >
      <div className="flex min-w-0 items-center gap-2">
        {isFromUser ? (
          <span className="flex h-5 shrink-0 items-center gap-1 rounded-[4px] bg-muted px-1.5 leading-none text-foreground-subtle">
            <UserIcon className="size-3 shrink-0" aria-hidden="true" />
            <span className="text-ui-xs font-medium">
              {intl.formatMessage({ id: "chat.agentWorkOrder.card.fromUserName" })}
            </span>
          </span>
        ) : (
          <span
            className={cn(
              "flex h-5 shrink-0 items-center gap-1 rounded-[4px] px-1.5 leading-none",
              SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(fromAgentName)],
            )}
          >
            <span className="text-ui-xs font-medium">{fromAgentName}</span>
          </span>
        )}
        <span className="min-w-0 truncate text-ui-xs text-foreground-subtle">
          {isFromUser
            ? intl.formatMessage({ id: "chat.agentWorkOrder.card.fromUserTitle" })
            : intl.formatMessage(
                { id: "chat.agentWorkOrder.card.fromTitle" },
                { name: fromAgentName },
              )}
        </span>
      </div>
      <div className="whitespace-pre-wrap break-words border-l-2 border-border pl-2.5 text-left text-ui-base leading-5 text-foreground-subtle">
        {meta.task}
      </div>
    </div>
  );
}
