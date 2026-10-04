import { ArrowDownIcon, ArrowUpIcon } from "lucide-react";

import { cn } from "../lib/utils.js";

/*
 * 滚动跳转胶囊钮（自实现 Uiverse "Back to Top" 的行为，配色走设计系统）：
 * 静置为墨色圆钮 + 品牌色淡光环；悬停横向展开成胶囊，箭头沿滚动方向飞出、
 * 文字浮现。展开态用当前设计风格的 --color-brand，不做固定的紫白配色。
 * 定位类 className 作用于外壳（光环一起跟随），按钮宽度动画向外扩展。
 */
export function ScrollJumpPillButton({
  direction,
  label,
  onClick,
  className,
  testId,
}: {
  direction: "up" | "down";
  label: string;
  onClick: () => void;
  className?: string;
  testId?: string;
}) {
  const Icon = direction === "up" ? ArrowUpIcon : ArrowDownIcon;
  return (
    <span className={cn("relative inline-flex", className)}>
      <span
        aria-hidden="true"
        className="pointer-events-none absolute -inset-1 rounded-full bg-brand/20"
      />
      <button
        type="button"
        aria-label={label}
        title={label}
        data-testid={testId}
        onClick={onClick}
        className="group relative flex h-9 w-9 cursor-pointer select-none items-center justify-center overflow-hidden rounded-full bg-foreground text-background shadow-sm transition-[width,background-color] duration-300 hover:w-40 hover:bg-brand focus-visible:bg-brand motion-reduce:transition-none"
      >
        <Icon
          aria-hidden="true"
          className={cn(
            "absolute size-4 transition-transform duration-300 motion-reduce:transition-none",
            direction === "up"
              ? "group-hover:-translate-y-[250%]"
              : "group-hover:translate-y-[250%]",
          )}
        />
        <span
          aria-hidden="true"
          className={cn(
            "absolute text-ui-sm font-semibold whitespace-nowrap opacity-0 transition-all duration-300 group-hover:translate-y-0 group-hover:opacity-100 motion-reduce:transition-none",
            direction === "up" ? "translate-y-[250%]" : "-translate-y-[250%]",
          )}
        >
          {label}
        </span>
      </button>
    </span>
  );
}
