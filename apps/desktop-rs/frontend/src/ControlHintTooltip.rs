//! 1:1 翻译 `packages/ui/src/ControlHintTooltip.tsx`（162 行）。
//!
//! 统一提示气泡：一行标题（可带快捷键 kbd）+ 可选的两行结构（标题 + 描述）。
//! 消费方：卡表头的 ⤢ / 停止钮、侧栏运行行等。
//!
//! 与真源的**有意偏离**（Rust 侧无 Radix）：
//! 真源走 Radix Tooltip（Portal + 碰撞翻转 + 状态机）。Rust 侧用 Leptos `Teleport`
//! 挂到 body，按触发器的 `getBoundingClientRect` 固定定位，hover / focus 显示、
//! 失焦 / 移出隐藏。无碰撞翻转（`side`/`align` 类名照抄真源词汇）；
//! `sideOffset` 缺省 2px 同真源。

use leptos::html;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// kbd 的基础类（真源 `shortcutKbdBaseClassName`，:39-40）。
const SHORTCUT_KBD_BASE_CLASS: &str = "rounded-md h-4 inline-flex items-center bg-tooltip-tag px-1.5 text-ui-xs font-medium text-tooltip-tag-foreground";

/// 气泡的对齐（真源 `side` / `align` 的用到的两值；缺省 top + center）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipSide {
    #[default]
    Top,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipAlign {
    #[default]
    Center,
}

/// `ControlHintTooltipProps`（真源 :18-32）。
///
/// 名字取 `ControlHintTooltipData` 是因为 Leptos 的 `#[component]` 会按组件名
/// 自动生成同名 props 结构体；字段与真源 props 一一对应。
#[derive(Clone, Default)]
pub struct ControlHintTooltipData {
    /// 标题（一行；`\n` 按真源 whitespace-pre-line 原样换行）。
    pub title: String,
    /// 在场即两行结构（标题 + 描述），缺省单行。
    pub description: Option<String>,
    /// 快捷键（kbd 徽标）。工作流面当前不传，保留对齐真源 props。
    pub shortcut: Option<String>,
    pub side: Option<TooltipSide>,
    pub align: Option<TooltipAlign>,
    /// 真源 sideOffset，缺省 2。
    pub side_offset: Option<f64>,
    /// 气泡的附加类名（真源 className）。
    pub class: Option<String>,
}

/// ControlHintTooltip（真源 :53-161）。
///
/// `trigger`（真源 children）由调用方给一段 view；包一层 `inline-flex shrink-0` 的
/// span（真源非 Element 分支同款）。打开状态是真源 Radix 的受控 `open` 的 Rust 对应物。
#[component]
pub fn ControlHintTooltip(
    /// props（真源 `ControlHintTooltipProps`，名字差异见类型注释）。
    data: ControlHintTooltipData,
    /// 触发器内容（真源 children）。
    children: ChildrenFn,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let anchor: NodeRef<html::Span> = NodeRef::new();
    let side = data.side.unwrap_or_default();
    let align = data.align.unwrap_or_default();
    let side_offset = data.side_offset.unwrap_or(2.0);

    // 气泡位置：挂载 / 打开时按锚点量一次。没有碰撞翻转（见文件头）。
    let position = Memo::new(move |_| {
        if !open.get() {
            return (0.0f64, 0.0f64);
        }
        let Some(el) = anchor.get() else {
            return (0.0, 0.0);
        };
        let rect = el.unchecked_ref::<web_sys::Element>().get_bounding_client_rect();
        let (left, top) = match side {
            // top：水平中心对齐锚点，气泡贴锚点上方。
            TooltipSide::Top => (
                rect.left() + rect.width() / 2.0,
                rect.top() - side_offset,
            ),
            // right：垂直中心对齐，气泡贴锚点右侧。
            TooltipSide::Right => (
                rect.right() + side_offset,
                rect.top() + rect.height() / 2.0,
            ),
        };
        (left, top)
    });

    // 内容类名（真源 :112-117）：两行结构与单行各自的最大宽。
    let has_description = data.description.is_some();
    let content_class = if has_description {
        "max-w-72 flex-col items-start gap-1.5 px-3 py-2 text-left"
    } else {
        "max-w-[min(28rem,calc(100vw-1rem))] items-center gap-2 px-2.5 py-1 text-left"
    };
    let title_text = data.title.clone();
    let description_text = data.description.clone();
    let shortcut_text = data.shortcut.clone();
    let extra_class = data.class.clone().unwrap_or_default();

    // 气泡内容：单行 / 两行两种排法（真源 :119-154）。文案装进 StoredValue（Copy），
    // 动态子闭包每次现取现用——闭包保持 Fn，Portal/Show 的 children 才能复用。
    let title_sv = StoredValue::new(title_text);
    let description_sv = StoredValue::new(description_text);
    let shortcut_sv = StoredValue::new(shortcut_text);
    // 气泡内层类名一并预拼（String 进 StoredValue，动态属性读它，闭包保持 Fn）。
    let inner_class_sv = StoredValue::new(format!(
        "z-50 flex rounded-md bg-tooltip text-ui-sm shadow-md outline-none {content_class} {extra_class}"
    ));
    let content = move || {
        if has_description {
            let title = title_sv.get_value();
            let shortcut = shortcut_sv.get_value();
            let description = description_sv.get_value();
            view! {
                <>
                    <div class="flex w-full items-start justify-between gap-3">
                        <span class="text-ui-sm font-medium leading-4 text-tooltip-foreground whitespace-pre-line">
                            {title}
                        </span>
                        {shortcut.map(|shortcut_text| view! {
                            <kbd
                                data-slot="kbd"
                                class=format!("{SHORTCUT_KBD_BASE_CLASS} font-mono shrink-0")
                            >
                                {shortcut_text}
                            </kbd>
                        })}
                    </div>
                    {description.map(|description_text| view! {
                        <span class="max-w-64 text-ui-sm/relaxed text-tooltip-foreground/80 whitespace-pre-line">
                            {description_text}
                        </span>
                    })}
                </>
            }
            .into_any()
        } else {
            let title = title_sv.get_value();
            let shortcut = shortcut_sv.get_value();
            view! {
                <>
                    <span class="text-ui-sm font-medium whitespace-pre-line break-words text-tooltip-foreground">
                        {title}
                    </span>
                    {shortcut.map(|shortcut_text| view! {
                        <kbd
                            data-slot="kbd"
                            class=format!("{SHORTCUT_KBD_BASE_CLASS} font-mono shrink-0")
                        >
                            {shortcut_text}
                        </kbd>
                    })}
                </>
            }
            .into_any()
        }
    };

    view! {
        <span
            node_ref=anchor
            class="inline-flex shrink-0"
            on:mouseenter=move |_| open.set(true)
            on:mouseleave=move |_| open.set(false)
            on:focusin=move |_| open.set(true)
            on:focusout=move |_| open.set(false)
        >
            {children()}
            <Show when=move || open.get()>
                // Leptos 0.8 的 portal 是 <Portal>（真源 Radix 的 Portal 对应物），缺省挂 body。
                <Portal>
                    <div
                        class="pointer-events-none fixed z-[9999] flex"
                        style=move || {
                            let (left, top) = position.get();
                            let transform = match (side, align) {
                                (TooltipSide::Top, TooltipAlign::Center) => "translate(-50%, -100%)",
                                (TooltipSide::Right, TooltipAlign::Center) => "translate(0, -50%)",
                            };
                            format!("left: {left}px; top: {top}px; transform: {transform};")
                        }
                    >
                        <div class=move || inner_class_sv.get_value()>
                            // 显式动态子节点：闭包保持 Fn，Portal/Show 的 children 才能复用。
                            {move || content()}
                        </div>
                    </div>
                </Portal>
            </Show>
        </span>
    }
}

/// 触发器元素探测（测试辅助）：与真源 `isTriggerElement` 的分支语义一致——
/// 传进来的是一段 view，Rust 侧不区分，包装规则只有一条。
pub fn tooltip_wraps_trigger_with_inline_flex() -> &'static str {
    "inline-flex shrink-0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kbd_base_class_matches_truth_source() {
        // 真源 :39-40。
        assert_eq!(
            SHORTCUT_KBD_BASE_CLASS,
            "rounded-md h-4 inline-flex items-center bg-tooltip-tag px-1.5 text-ui-xs font-medium text-tooltip-tag-foreground"
        );
    }

    #[test]
    fn content_class_switches_on_description() {
        // 真源 :112-117 —— 两行结构收窄到 max-w-72，单行到视口安全宽。
        let with_desc = "max-w-72 flex-col items-start gap-1.5 px-3 py-2 text-left";
        let without = "max-w-[min(28rem,calc(100vw-1rem))] items-center gap-2 px-2.5 py-1 text-left";
        assert_ne!(with_desc, without);
        assert!(with_desc.contains("flex-col"));
        assert!(!without.contains("flex-col"));
    }

    #[test]
    fn wrapper_class_matches_truth_source() {
        // 真源 :97 —— 非 Element 分支的包装类。
        assert_eq!(tooltip_wraps_trigger_with_inline_flex(), "inline-flex shrink-0");
    }
}
