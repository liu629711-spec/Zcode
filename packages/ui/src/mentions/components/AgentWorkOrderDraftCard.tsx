/**
 * 派单小输入框（D30/D5）：@ 菜单智能体行点「派单」后弹出。
 *
 * 交互口径：任务写完点「提交工单」（或 Enter，Shift+Enter 换行）直达宿主派单能力，
 * 不经模型绕圈、不往 composer 塞文本（塞了就变点将语义）。Esc/取消回到编辑器。
 * 提交中禁重复提交；失败 toast 由插件层统一处理（与设计风格落盘失败同一处理位）。
 */
import { useEffect, useRef } from "react";
import { Bot, SendIcon, XIcon } from "lucide-react";
import { cn } from "@/components/lib/utils.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";

export function AgentWorkOrderDraftCard({
  agentName,
  task,
  pending,
  onTaskChange,
  onSubmit,
  onCancel,
}: {
  agentName: string;
  task: string;
  pending: boolean;
  onTaskChange: (task: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
}) {
  const { intl } = useZCodeIntl();
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  return (
    <div
      className="mb-1 rounded-2xl border border-border bg-menu p-3 shadow-xs"
      data-testid="agent-work-order-draft-card"
      data-agent={agentName}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={cn(
              "flex h-6 shrink-0 items-center gap-1 rounded-[4px] px-1.5 leading-none",
              SUBAGENT_COLOR_CLASS[resolveSubagentColorFromName(agentName)],
            )}
          >
            <Bot className="size-3.5 shrink-0" aria-hidden="true" />
            <span className="text-ui-xs font-medium">{agentName}</span>
          </span>
          <span className="min-w-0 truncate text-ui-base font-medium text-foreground">
            {intl.formatMessage({ id: "chat.mention.agents.dispatchTitle" }, { name: agentName })}
          </span>
        </div>
        <button
          type="button"
          aria-label={intl.formatMessage({ id: "chat.mention.agents.dispatchCancel" })}
          className="flex size-6 shrink-0 items-center justify-center rounded-md text-foreground-subtle transition-colors hover:bg-hover hover:text-foreground"
          onClick={onCancel}
        >
          <XIcon className="size-3.5" aria-hidden="true" />
        </button>
      </div>
      <textarea
        ref={textareaRef}
        value={task}
        disabled={pending}
        placeholder={intl.formatMessage({ id: "chat.mention.agents.dispatchPlaceholder" })}
        rows={3}
        data-testid="agent-work-order-draft-input"
        className="mt-2 w-full resize-none rounded-xl border border-border bg-surface px-2.5 py-2 text-ui-base leading-5 text-foreground outline-none placeholder:text-foreground-subtlest focus:border-[var(--color-ring)] disabled:opacity-60"
        onChange={(event) => onTaskChange(event.target.value)}
        onKeyDown={(event) => {
          // 中文 IME 组合期（选词/撤词）的 Enter/Esc 属于输入法操作：
          // Enter 不能把没打完的任务提交出去（LexicalChatInput.shouldSubmitLexicalEnter
          // 同一守卫），Esc 不能顺手关卡片。
          if (event.nativeEvent.isComposing) {
            return;
          }
          if (event.key === "Escape") {
            event.stopPropagation();
            onCancel();
            return;
          }
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            onSubmit();
          }
        }}
      />
      <div className="mt-2 flex items-center justify-between gap-2">
        <span className="min-w-0 truncate text-ui-xs text-foreground-subtlest">
          {intl.formatMessage({ id: "chat.mention.agents.dispatchHint" })}
        </span>
        <button
          type="button"
          disabled={pending || task.trim().length === 0}
          data-testid="agent-work-order-draft-submit"
          className={cn(
            "flex h-7 shrink-0 items-center gap-1 rounded-lg px-2.5 text-ui-xs font-medium transition-colors",
            pending || task.trim().length === 0
              ? "cursor-not-allowed bg-muted text-foreground-subtlest"
              : "bg-primary text-primary-foreground hover:opacity-90",
          )}
          onClick={onSubmit}
        >
          <SendIcon className="size-3" aria-hidden="true" />
          {intl.formatMessage({ id: "chat.mention.agents.dispatchSubmit" })}
        </button>
      </div>
    </div>
  );
}
