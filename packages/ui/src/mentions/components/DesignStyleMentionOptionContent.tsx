/**
 * 设计风格候选行（V4.6）：风格名 + 主色色板 + 一句描述。
 * 真机反馈：走普通文件行渲染时全是「DESIGN.md + 路径」，143 套风格完全无法区分；
 * 色板是语言无关的辨识特征，风格名 + 描述补足语义。
 */
import { PaletteIcon } from "lucide-react";
import type { MentionItem } from "@/mentions/mentionTypes.js";

export function DesignStyleMentionOptionContent({ item }: { item: MentionItem }) {
  const palette = item.data?.palette ?? [];
  return (
    <span className="min-w-0 flex flex-1 items-center gap-2">
      <PaletteIcon className="size-3.5 shrink-0 text-foreground" aria-hidden="true" />
      <span className="shrink-0 whitespace-nowrap text-ui-base font-medium text-foreground">
        {item.label}
      </span>
      {palette.length > 0 ? (
        <span className="flex shrink-0 items-center gap-0.5" aria-hidden="true">
          {palette.map((color) => (
            <span
              key={color}
              className="size-2.5 rounded-full border border-border"
              style={{ backgroundColor: color }}
            />
          ))}
        </span>
      ) : null}
      <span className="min-w-0 truncate text-ui-xs text-foreground-subtlest">
        {item.description}
      </span>
    </span>
  );
}
