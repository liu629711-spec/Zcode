//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowTimelineLedge.tsx`（258 行）。
//!
//! 边檐与滚动条。
//!
//! 边檐：折叠到视口一侧的站，画成一排同一枚 10px 灯（16px 一枚），之间是那条边的墨色
//! 小轨道段，靠内容的一端一段短轨道接到第一个开着的站，远端是超出五枚时的 `+n`。
//! 每枚灯是按钮：点一下，镜头把那一站带回来。折叠的站在内容里不画——檐上那枚灯**就是**它。
//!
//! 滚动条：时间线底部 2px 一根，轨道 border 色、拇指 foreground-subtlest；静止时
//! opacity 0，指针在卡上或正在滚时露出，悬停轨道加粗到 4px；拇指可拖，点轨道翻页。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::timeline_ledge::{
    ledge_lamps, rail_key, scrollbar_thumb, LedgeSide, LEDGE_LAMP, LEDGE_PITCH,
};
use crate::components::useTimelineViewport::TimelineViewport;
use crate::components::workflow_graph::types::StepRunStatus;
use crate::components::workflow_graph::run_status_presentation::status_dot_class;
use crate::ToolCallBlocks::i18n;

/// 站灯（真源 `stationLampClass`，:28-35）：`STATUS_DOT` 词汇表，running 外加呼吸光晕
/// ——它是画面上唯一发光的东西。
pub fn station_lamp_class(status: Option<StepRunStatus>) -> &'static str {
    let resolved = status.unwrap_or(StepRunStatus::Pending);
    // 全表缓存在 OnceLock：类名是常量词表，不该每次渲染新造（Box::leak 只发生一次）。
    static CACHE: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        [StepRunStatus::Pending, StepRunStatus::Running, StepRunStatus::Done, StepRunStatus::Failed]
            .iter()
            .map(|status| {
                let base = "wf-lamp size-2.5 shrink-0 rounded-full";
                let dot = status_dot_class(*status);
                let suffix = if *status == StepRunStatus::Running {
                    " wf-lamp-running motion-reduce:animate-none"
                } else {
                    ""
                };
                &*Box::leak(format!("{base} {dot}{suffix}").into_boxed_str())
            })
            .collect()
    });
    let index = match resolved {
        StepRunStatus::Pending => 0,
        StepRunStatus::Running => 1,
        StepRunStatus::Done => 2,
        StepRunStatus::Failed => 3,
    };
    cache[index]
}

pub fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|media| media.matches())
        .unwrap_or(false)
}

/// 檐间小段（真源 `Segment`，:49-81）：与静态主轨道同为 1px，避免滚动边缘出现粗细接缝。
/// 双线段：檐上两站并行时不是一条线，而是两条 1px、相距 2px——檐把带压扁了，
/// 两条并排的线是这里唯一还说得出「同时」的记号。
#[component]
fn Segment(rail: Option<crate::components::timeline_model::TimelineRail>, width: f64) -> impl IntoView {
    if rail.as_ref().and_then(|r| r.kind) == Some(crate::components::timeline_bands::TimelineRailKind::Twin) {
        let ink = rail.as_ref().map(|r| r.ink.as_str()).unwrap_or("none");
        return view! {
            <span
                aria-hidden="true"
                class="relative h-0 shrink-0"
                data-ledge-ink=ink
                data-ledge-twin="true"
                style=format!("width: {width}px;")
            >
                <span class="absolute inset-x-0 h-0 border-t border-foreground-subtlest" style="top: -1px;" />
                <span class="absolute inset-x-0 h-0 border-t border-foreground-subtlest" style="top: 1px;" />
            </span>
        }
        .into_any();
    }
    let ink = rail.as_ref().map(|r| r.ink.as_str()).unwrap_or("none");
    let invisible = rail.is_none();
    view! {
        <span
            aria-hidden="true"
            class=format!("h-0 shrink-0 border-t border-foreground-subtlest {}", if invisible { "invisible" } else { "" })
            data-ledge-ink=ink
            style=format!("width: {width}px;")
        />
    }
    .into_any()
}

/// `WorkflowLedge`（真源 :83-173）。
///
/// `rails` 是相邻站之间的轨道段，按**一对站**索引（`railKey`）：带里一站会长出好几条段。
/// Rust 侧由调用方给一个查表闭包（卡层持有 HashMap）。
#[component]
pub fn WorkflowLedge(
    side: &'static str,
    /// 折叠到这一侧的站（升序）。
    indexes: Vec<usize>,
    stations: Vec<crate::components::timeline_model::TimelineStation>,
    /// 相邻站之间的轨道段，按 `railKey` 查。
    rails: std::collections::HashMap<String, crate::components::timeline_model::TimelineRail>,
    /// 檐到第一个开着的站之间那段短轨道的宽；0 = 不画。
    stub_width: f64,
    /// 轨道行的顶（檐与灯同高）。
    top: f64,
    name_of: Callback<usize, String>,
    on_select: Callback<usize>,
) -> impl IntoView {
    if indexes.is_empty() {
        return view! { <span class="hidden" /> }.into_any();
    }
    let ledge_side = if side == "left" { LedgeSide::Left } else { LedgeSide::Right };
    let lamps_info = ledge_lamps(&indexes, ledge_side);
    let stub_rail = if side == "left" {
        let last = *indexes.last().unwrap();
        rails.get(&rail_key(last, last + 1)).copied()
    } else {
        let first = indexes[0];
        rails.get(&rail_key(first - 1, first)).copied()
    };
    let more = lamps_info.more;
    // AnyView 不可克隆：count / stub 各自由闭包现建，供首尾两个位置各调一次。
    let count_view = move || -> AnyView {
        if more > 0 {
            view! {
                <span
                    class="shrink-0 text-center font-mono text-ui-2xs text-foreground-subtlest"
                    data-testid="workflow-timeline-ledge-more"
                    style="width: 26px;"
                >
                    {format!("+{more}")}
                </span>
            }
            .into_any()
        } else {
            view! { <span class="hidden" /> }.into_any()
        }
    };
    let stub_view = move || -> AnyView {
        if stub_width > 0.0 {
            view! { <Segment rail=stub_rail width=stub_width /> }.into_any()
        } else {
            view! { <span class="hidden" /> }.into_any()
        }
    };
    let lamp_views: Vec<AnyView> = lamps_info
        .shown
        .iter()
        .enumerate()
        .map(|(k, index)| {
            let index = *index;
            let station = &stations[index];
            let label = format!(
                "{} · {}",
                name_of.run(index),
                i18n::text(&format!(
                    "chat.toolCall.workflow.graph.status.{}",
                    station.status.map(|s| s.as_str()).unwrap_or("pending")
                )),
            );
            let segment = if k > 0 {
                let prev = lamps_info.shown[k - 1];
                Some(
                    view! {
                        <Segment
                            rail=rails.get(&rail_key(prev, index)).copied()
                            width=LEDGE_PITCH - LEDGE_LAMP
                        />
                    }
                    .into_any(),
                )
            } else {
                None
            };
            let on_select = on_select.clone();
            view! {
                <span class="flex items-center">
                    {segment}
                    <button
                        aria-label=label.clone()
                        class="wf-ledge-lamp relative flex shrink-0 cursor-pointer items-center justify-center rounded-full outline-none before:absolute before:-inset-1 before:content-[''] focus-visible:ring-2 focus-visible:ring-ring/40"
                        data-station-index=index
                        data-testid="workflow-timeline-ledge-lamp"
                        on:click=move |_| on_select.run(index)
                        style=format!("height: {LEDGE_LAMP}px; width: {LEDGE_LAMP}px;")
                        title=label
                        type="button"
                    >
                        <span
                            aria-hidden="true"
                            class=format!("{} wf-land", station_lamp_class(station.status))
                            data-lamp=station.status.map(|s| s.as_str()).unwrap_or("pending")
                        />
                    </button>
                </span>
            }
            .into_any()
        })
        .collect();

    let aria = i18n::format(
        if side == "left" {
            "chat.toolCall.workflow.timeline.ledge.earlier"
        } else {
            "chat.toolCall.workflow.timeline.ledge.later"
        },
        &[("count".to_string(), indexes.len().to_string())],
    );
    view! {
        <div
            aria-label=aria
            class=format!(
                "wf-ledge pointer-events-auto absolute flex h-6 items-center {}",
                if side == "left" { "left-0 pl-2" } else { "right-0 pr-2" },
            )
            data-testid=format!("workflow-timeline-ledge-{side}")
            role="group"
            style=format!("top: {top}px;")
        >
            {if side == "left" { count_view() } else { stub_view() }}
            {lamp_views.into_iter().collect_view()}
            {if side == "left" { stub_view() } else { count_view() }}
        </div>
    }
    .into_any()
}

/// `WorkflowTimelineScrollbar`（真源 :184-258）。
#[component]
pub fn WorkflowTimelineScrollbar(
    scroll_element: RwSignal<Option<web_sys::Element>>,
    viewport: TimelineViewport,
) -> impl IntoView {
    let thumb = scrollbar_thumb(viewport.scroll_left, viewport.client_width, viewport.scroll_width);
    let Some(thumb) = thumb else {
        return view! { <span class="hidden" /> }.into_any();
    };
    let range = viewport.scroll_width - viewport.client_width;
    let ratio = viewport.scroll_width / viewport.client_width;
    let dragging = RwSignal::new(false);
    let drag = StoredValue::new(None::<(f64, f64, f64)>); // pointerId, startX, startLeft

    let aria_now = (100.0 * viewport.scroll_left.min(range) / range).round() as i64;

    // 拇指拖动（真源 onThumbDown / onThumbMove / onThumbUp）。
    let on_thumb_down = move |event: leptos::ev::PointerEvent| {
        event.stop_propagation();
        event.prevent_default();
        drag.set_value(Some((
            event.pointer_id() as f64,
            event.client_x() as f64,
            viewport.scroll_left,
        )));
        if let Some(target) = event.target() {
            if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                let _ = el.set_pointer_capture(event.pointer_id());
            }
        }
        dragging.set(true);
    };
    let on_thumb_move = move |event: leptos::ev::PointerEvent| {
        let Some(state) = drag.get_value() else {
            return;
        };
        if state.0 != event.pointer_id() as f64 {
            return;
        }
        if let Some(element) = scroll_element.get_untracked() {
            element.set_scroll_left((state.2 + (event.client_x() as f64 - state.1) * ratio) as i32);
        }
    };
    let on_thumb_up = move |event: leptos::ev::PointerEvent| {
        let Some(state) = drag.get_value() else {
            return;
        };
        if state.0 != event.pointer_id() as f64 {
            return;
        }
        drag.set_value(None);
        if let Some(target) = event.target() {
            if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                let _ = el.release_pointer_capture(event.pointer_id());
            }
        }
        dragging.set(false);
    };
    let on_track_down = move |event: leptos::ev::PointerEvent| {
        event.stop_propagation();
        let Some(element) = scroll_element.get_untracked() else {
            return;
        };
        let rect = event
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .map(|el| el.get_bounding_client_rect());
        let Some(rect) = rect else {
            return;
        };
        let x = event.client_x() as f64 - rect.left();
        let direction = if x < thumb.left { -1.0 } else { 1.0 };
        // element.scrollBy({left, behavior})：web_sys 的 ScrollToOptions。
        let behavior = if prefers_reduced_motion() {
            web_sys::ScrollBehavior::Auto
        } else {
            web_sys::ScrollBehavior::Smooth
        };
        let options = web_sys::ScrollToOptions::new();
        options.set_behavior(behavior);
        options.set_left(direction * viewport.client_width);
        let _ = element.scroll_by_with_scroll_to_options(&options);
    };

    view! {
        <div
            aria-label=i18n::text("chat.toolCall.workflow.timeline.scrollbar")
            aria-orientation="horizontal"
            aria-valuemax="100"
            aria-valuemin="0"
            aria-valuenow=aria_now
            class="wf-sb absolute inset-x-0 bottom-0 h-0.5 cursor-pointer rounded-full bg-border"
            data-dragging=dragging.get().then_some("true")
            data-scrolling=viewport.scrolling.then_some("true")
            data-testid="workflow-timeline-scrollbar"
            on:click=move |event| event.stop_propagation()
            on:pointerdown=on_track_down
            role="scrollbar"
        >
            <span
                class="wf-sb-thumb absolute inset-y-0 rounded-full bg-foreground-subtlest"
                data-testid="workflow-timeline-scrollbar-thumb"
                on:pointerdown=on_thumb_down
                on:pointermove=on_thumb_move
                on:pointerup=on_thumb_up
                style=format!("left: {}px; width: {}px;", thumb.left, thumb.width)
            />
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_lamp_class_covers_all_statuses() {
        // 真源 :28-35 —— pending / done / failed / running 各有词；running 带搏动。
        for status in [
            Some(StepRunStatus::Pending),
            Some(StepRunStatus::Running),
            Some(StepRunStatus::Done),
            Some(StepRunStatus::Failed),
            None,
        ] {
            let class = station_lamp_class(status);
            assert!(class.contains("wf-lamp"), "站灯应带 wf-lamp：{class}");
            assert!(class.contains("size-2.5"));
        }
        let running = station_lamp_class(Some(StepRunStatus::Running));
        assert!(running.contains("wf-lamp-running"));
        assert!(running.contains("motion-reduce:animate-none"));
        let pending = station_lamp_class(Some(StepRunStatus::Pending));
        assert!(pending.contains("border-foreground-subtlest"));
        // 缺席与 pending 同词（真源 :29 的 ??）。
        assert_eq!(station_lamp_class(None), station_lamp_class(Some(StepRunStatus::Pending)));
    }
}
