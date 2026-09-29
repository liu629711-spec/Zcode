/**
 * 点将候选行（D24）：彩色名字牌工牌 + 一句"他是干什么的"。
 *
 * 真机反馈：这些候选走默认行渲染时只有一个光秃秃的名字，和下面的文件行混成一片，
 * 看不出"这是能派活的人"。这里直接复用侧栏会话行上那枚工牌的画法（Bot 图标 +
 * 档案色名字牌，档案没选色则按名字哈希取色），用户在列表里和在 @ 面板里看到的
 * 是同一个人的同一个颜色——颜色是跨界面认人的捷径，比读英文名快。
 */
import { Bot } from "lucide-react";
import { cn } from "@/components/lib/utils.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import type { MentionItem } from "@/mentions/mentionTypes.js";

export function AgentCallMentionOptionContent({ item }: { item: MentionItem }) {
  const name = item.label;
  return (
    <span className="min-w-0 flex flex-1 items-center gap-2">
      <span
        data-persona-chat-badge="true"
        className={cn(
          "flex min-w-0 shrink-0 items-center gap-1 rounded-[4px] px-1 leading-none",
          SUBAGENT_COLOR_CLASS[
            item.data?.agentColor ?? resolveSubagentColorFromName(name)
          ],
        )}
      >
        <Bot className="size-3 shrink-0" aria-hidden="true" />
        <span className="min-w-0 truncate text-ui-base font-medium">{name}</span>
      </span>
      <span className="min-w-0 truncate text-ui-xs text-foreground-subtlest">
        {item.description}
      </span>
    </span>
  );
}
