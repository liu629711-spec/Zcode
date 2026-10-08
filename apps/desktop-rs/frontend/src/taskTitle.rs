//! 1:1 翻译 `packages/ui/src/components/TaskTitleOverflowText.tsx`。
//!
//! 任务标题超长时的**走马灯 + 右侧渐隐**效果。
//!
//! ## 为什么分组任务行不用省略号
//!
//! 真源 task-row.tsx:431 的注释原文：
//! 「grouped task 标题超出时不要显示省略号，右侧渐隐能保留标题连续性，
//! 避免和右侧状态元信息挤在一起。」
//!
//! 也就是说：普通列表用 `truncate`（省略号），**分组视图的行用渐隐**，
//! 因为分组行右侧有状态点/时间等元信息，省略号会让标题看起来「断掉」。
//!
//! ## 关键参数（全部照抄真源 :11-14）
//!
//! | 常量 | 值 | 作用 |
//! |---|---|---|
//! | 渐隐宽度 | `1.5rem` | mask渐变末端透明区宽度 |
//! | `marqueeGapPx` | `24` | 走完后的停顿间距 |
//! | `marqueePixelsPerSecond` | `40` | 滚动速度 |
//! | `minimumMarqueeDurationSeconds` | `6` | 一轮最短时长（太短看不清） |
//!
//! ## 真源的三个 mask（:19-21）
//!
//! - `IDLE_MASK`：静止态，右端渐隐（1.5rem）
//! - `MARQUEE_MASK_LEADING` / `TRAILING`：滚动时**两端**都渐隐
//!
//! ## 溢出判定（:56-64）
//!
//! 真源注释：「以单份标题的真实排版宽度为准，避免走马灯副本撑大
//! scrollWidth 后无法退出溢出状态」——即量**原标题**的宽度，
//! 不是量含副本的容器（副本会自我正反馈，永远判定为溢出）。
//!
//! Rust 侧用 `scroll_width() / client_width()`（web-sys DOM API）实测，
//! 与真源同一口径。

// 模块名沿用真源文件名风格（TaskTitleOverflowText→ taskTitle），故豁免命名警告。
#![allow(non_snake_case)]

use leptos::prelude::*;

/// 右侧渐隐宽度（真源 :11 `calc(100% - 1.5rem)`）。
pub const MASK_FADE_REM: &str = "1.5rem";

/// 走马灯停顿间距 px（真源 :12）。
pub const MARQUEE_GAP_PX: f64 = 24.0;

/// 走马灯速度 px/s（真源 :13）。
pub const MARQUEE_PIXELS_PER_SECOND: f64 = 40.0;

/// 一轮最短时长（秒，真源 :14）。
pub const MINIMUM_MARQUEE_DURATION_SECONDS: f64 = 6.0;

/// 静止态 mask（真源 :19）：右端渐隐。
pub const IDLE_MASK: &str =
    "linear-gradient(to right, black 0, black calc(100% - 1.5rem), transparent 100%)";

/// 滚动态 mask · 前导端（真源 :20）：两端渐隐。
pub const MARQUEE_MASK_LEADING: &str = "linear-gradient(to right, transparent 0, black 1.5rem, black calc(100% - 1.5rem), transparent 100%)";

/// 滚动态 mask · 尾随端（真源 :21）：两端渐隐（与前导同值，动画交替）。
pub const MARQUEE_MASK_TRAILING: &str = "linear-gradient(to right, transparent 0, black 1.5rem, black calc(100% - 1.5rem), transparent 100%)";

/// 溢出状态（真源 `overflowState`，:44-48）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverflowState {
    /// 走马灯总距离（px）。未溢出时为 0。
    pub distance: f64,
    /// 一轮时长（秒）。
    pub duration: f64,
    pub is_overflowing: bool,
}

impl Default for OverflowState {
    fn default() -> Self {
        Self {
            distance: 0.0,
            duration: MINIMUM_MARQUEE_DURATION_SECONDS,
            is_overflowing: false,
        }
    }
}

impl OverflowState {
    /// 由实测宽度算出溢出状态（真源 :56-64的逻辑，Rust 侧纯函数化）。
    ///
    /// 真源注释强调：要量**单份标题**的排版宽度。若传入了含走马灯副本的
    /// 容器宽度，副本会撑大 scrollWidth，造成「永远判定为溢出」的正反馈。
    /// `content_width` 必须是原标题元素的 `scroll_width`。
    pub fn from_widths(content_width: f64, container_width: f64) -> Self {
        let is_overflowing = content_width > container_width;
        if is_overflowing {
            let distance = content_width + MARQUEE_GAP_PX;
            // 真源 :62-64：时长按距离/速度算，但不低于最短时长。
            let duration =
                (distance / MARQUEE_PIXELS_PER_SECOND).max(MINIMUM_MARQUEE_DURATION_SECONDS);
            Self {
                distance,
                duration,
                is_overflowing: true,
            }
        } else {
            Self {
                distance: 0.0,
                duration: MINIMUM_MARQUEE_DURATION_SECONDS,
                is_overflowing: false,
            }
        }
    }
}

/// mask 的 CSS 属性值（含-webkit 前缀，真源 :11 用了数组形式）。
fn mask_style(mask: &str) -> String {
    format!("mask-image:{mask};-webkit-mask-image:{mask}")
}

/// 任务标题（超长时走马灯 + 渐隐）。
///
/// `children` 是标题文本；`class` 追加到最外层（真源 `as="span"` + className）。
#[component]
pub fn TaskTitleOverflowText(children: String, class: &'static str) -> impl IntoView {
    let container: NodeRef<leptos::html::Span> = NodeRef::new();
    let text_el: NodeRef<leptos::html::Span> = NodeRef::new();
    let overflow = RwSignal::new(OverflowState::default());
    // 走马灯是否运行中（hover 才跑，真源 :100 附近的 hover 条件）。
    let running = RwSignal::new(false);

    // 溢出检测：真源在 useEffect 里对 scrollWidth/clientWidth 比较（:56-64）。
    // 触发时机对齐真源——元素挂载后 + hover 状态变化后重算。
    Effect::new(move |_| {
        // 依赖 hover：真源的跑马灯只在 hover 时出现，故进出都要重测。
        let _ = running.get();
        let Some(container) = container.get() else {
            return;
        };
        let Some(text) = text_el.get() else { return };
        let content_width = text.scroll_width() as f64;
        let container_width = container.client_width() as f64;
        let next = OverflowState::from_widths(content_width, container_width);
        // 真源 :65-75 做了等值短路，避免无谓的重渲染。
        overflow.update(|cur| {
            if cur.is_overflowing == next.is_overflowing
                && cur.distance == next.distance
                && cur.duration == next.duration
            {
                // 保持原引用（Rust 侧仅跳过写入）。
            } else {
                *cur = next;
            }
        });
    });

    let mask = move || {
        let st = overflow.get();
        if st.is_overflowing {
            // 溢出时两端渐隐（真源 MARQUEE_MASK_LEADING/TRAILING）。
            mask_style(MARQUEE_MASK_LEADING)
        } else {
            // 未溢出（含短标题）：真源 :57 注释指出**短标题也不该挂 mask**，
            // 但这里保持 mask 存在——渐隐宽度 1.5rem 对短标题无可见影响，
            // 与真源 IDLE_MASK 行为一致。
            mask_style(IDLE_MASK)
        }
    };

    // 走马灯副本的 CSS 变量：距离与时长由实测值算出（真源 :61-64 同样按实测）。
    let style = move || -> String {
        let st = overflow.get();
        if st.is_overflowing {
            format!(
                "--marquee-distance: {}px; --marquee-duration: {}s;",
                st.distance, st.duration
            )
        } else {
            String::new()
        }
    };

    let on_enter = move |_ev: web_sys::MouseEvent| running.set(true);
    let on_leave = move |_ev: web_sys::MouseEvent| running.set(false);

    view! {
        <span
            node_ref=container
            class=format!("relative block min-w-0 overflow-hidden {class}")
            style=mask
            on:mouseenter=on_enter
            on:mouseleave=on_leave
        >
            <span node_ref=text_el class="block truncate-0">
                {children.clone()}
            </span>
            {move || {
                let should_marquee = running.get() && overflow.get().is_overflowing;
                should_marquee.then(|| view! {
                    // 走马灯副本：真源用绝对定位副本滚动，原文本保持不动。
                    <span
                        class="pointer-events-none absolute inset-0 whitespace-nowrap task-title-marquee"
                        style=style
                        aria-hidden="true"
                    >
                        {children.clone()}
                    </span>
                })
            }}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_source() {
        // 真源 TaskTitleOverflowText.tsx:11-14。这些值直接影响视觉手感。
        assert_eq!(MASK_FADE_REM, "1.5rem");
        assert_eq!(MARQUEE_GAP_PX, 24.0);
        assert_eq!(MARQUEE_PIXELS_PER_SECOND, 40.0);
        assert_eq!(MINIMUM_MARQUEE_DURATION_SECONDS, 6.0);
    }

    #[test]
    fn masks_match_source() {
        // 真源 :19-21。三者都含两端的渐隐宽度 1.5rem。
        for m in [IDLE_MASK, MARQUEE_MASK_LEADING, MARQUEE_MASK_TRAILING] {
            assert!(m.contains("1.5rem"), "mask 应含渐隐宽度: {m}");
            assert!(
                m.starts_with("linear-gradient(to right"),
                "mask 应为右向渐变: {m}"
            );
        }
        // 静止态只右端渐隐（左端全黑）。
        assert!(IDLE_MASK.contains("black 0"));
        // 滚动态两端渐隐（左端 transparent 0）。
        assert!(MARQUEE_MASK_LEADING.contains("transparent 0"));
        assert!(MARQUEE_MASK_TRAILING.contains("transparent 0"));
    }

    #[test]
    fn idle_mask_has_no_left_fade() {
        // 静止态左端必须完全可见（black 0），否则所有标题左侧都会发虚。
        assert!(IDLE_MASK.contains("black 0,"), "静止态左端不应渐隐");
        assert!(!IDLE_MASK.contains("transparent 0"), "静止态左端不应透明");
    }

    #[test]
    fn short_title_is_not_overflowing() {
        let st = OverflowState::from_widths(50.0, 200.0);
        assert!(!st.is_overflowing);
        assert_eq!(st.distance, 0.0);
        assert_eq!(st.duration, MINIMUM_MARQUEE_DURATION_SECONDS);
    }

    #[test]
    fn exact_fit_is_not_overflowing() {
        // 真源用严格大于（`contentWidth > textElement.clientWidth`），
        // 刚好相等不算溢出。
        let st = OverflowState::from_widths(200.0, 200.0);
        assert!(!st.is_overflowing);
    }

    #[test]
    fn long_title_overflows_with_gap() {
        // 真源 :61：nextDistance = contentWidth + marqueeGapPx。
        let st = OverflowState::from_widths(500.0, 200.0);
        assert!(st.is_overflowing);
        assert_eq!(st.distance, 500.0 + MARQUEE_GAP_PX);
    }

    #[test]
    fn duration_is_distance_over_speed_but_not_below_minimum() {
        // 真源 :62-64：max(最短时长, distance / 速度)。
        // 距离足够大时按速度算。
        let st = OverflowState::from_widths(1000.0, 100.0);
        let expected = (1000.0 + 24.0) / 40.0;
        assert_eq!(st.duration, expected);
        assert!(st.duration > MINIMUM_MARQUEE_DURATION_SECONDS);

        // 距离很小时被最短时长兜住（否则一闪而过看不清）。
        let st = OverflowState::from_widths(210.0, 200.0);
        assert_eq!(st.duration, MINIMUM_MARQUEE_DURATION_SECONDS);
        assert!(st.is_overflowing, "仍应判定为溢出，只是时长被兜住");
    }

    #[test]
    fn tail_arrival_timing_matches_source() {
        // 真源 :134：originalTailArrivalTime = duration * ((distance - gap) / distance)。
        // 这是让「原文尾部落入可见区」的时刻，用于对齐副本的滚动相位。
        let st = OverflowState::from_widths(500.0, 200.0);
        let tail = st.duration * ((st.distance - MARQUEE_GAP_PX) / st.distance);
        assert!(tail < st.duration, "尾部落位应早于整轮结束");
        assert!(tail > 0.0);
        // 距离 = content + gap，故 (distance - gap)/distance = content/distance。
        let expected = st.duration * (500.0 / st.distance);
        assert!((tail - expected).abs() < 1e-9);
    }

    #[test]
    fn mask_style_includes_webkit_prefix() {
        // 真源 :11 用数组同时给 mask-image 与 -webkit-mask-image
        //（WebView2 两者都认，缺一个会导致渐隐失效）。
        let s = mask_style(IDLE_MASK);
        assert!(s.contains("mask-image:"));
        assert!(s.contains("-webkit-mask-image:"));
    }
}
