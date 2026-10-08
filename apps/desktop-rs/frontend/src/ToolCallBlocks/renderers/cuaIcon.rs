//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaIcon.tsx`（8 行）。
//!
//! CUA 的 fallback 图标（MousePointerClick）。真源注释：兼容 CUA 分组摘要的
//! 语义名称；底层视觉统一用同一个 fallback。

/// 真源 :3-5 —— `MousePointerClick className="size-4 shrink-0 text-foreground-subtle"`。
pub const CUA_FALLBACK_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `CUA_TOOL_ICON`（真源 :8）——语义别名，视觉同 fallback。
pub const CUA_TOOL_ICON_CLASS: &str = CUA_FALLBACK_ICON_CLASS;

/// MousePointerClick（lucide）的五段 path（真源 :3-5 的图标本体）。
pub const MOUSE_POINTER_CLICK_PATHS: [&str; 5] = [
    "M14 4.1 12 6",
    "m5.1 8-2.9-.8",
    "m6 12-1.9 2",
    "M7.2 2.2 8 5.1",
    "M9.037 9.69a.498.498 0 0 1 .653-.653l11 4.5a.5.5 0 0 1-.074.949l-4.349 1.041a1 1 0 0 0-.74.739l-1.04 4.35a.5.5 0 0 1-.95.074z",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_icon_aliases_fallback() {
        assert_eq!(CUA_TOOL_ICON_CLASS, CUA_FALLBACK_ICON_CLASS);
        assert_eq!(MOUSE_POINTER_CLICK_PATHS.len(), 5);
    }
}
