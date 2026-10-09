//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowTimelineStation.tsx`（232 行）。
//!
//! 站头：名字 + 元数据，一枚没有背景的药丸。没有带时它排在轨道行上、灯的右边；
//! 有带时它搬到站台行，灯留在自己那条轨道上——两处的标记完全一样，所以从
//! `WorkflowTimeline.tsx` 拆出来共用一份。
//!
//! 不可点的站头是 `span` 而不是禁用的 `button`：浏览器对禁用控件不派发 click，
//! 整块开关就收不到；span 没有语义，点击照常冒泡。

use std::collections::HashMap;

use leptos::prelude::*;

use crate::components::timeline_geometry::{
    station_x, CAPTION_X, PLATFORM_ROW, RAIL_ROW, STATION_PITCH, STATION_WIDTH, TimelineLayout,
};
use crate::components::timeline_model::{TimelineRail, TimelineStation};
use crate::components::useTypewriter::TypewriterState;

use super::WorkflowStationMeta::StationMeta;
use super::WorkflowTimelineLedge::station_lamp_class;

/// 站头（真源 `StationHead`，:26-82）。
#[component]
pub fn StationHead(
    station: TimelineStation,
    name: String,
    /// 草稿里跟着笔走的光标；其余时候 `None`。
    caret: Option<AnyView>,
    fold_class: String,
    title: String,
    /// 缺席即站头不是控件——点击冒泡给宿主（轮尾摘要的整块开关）。
    on_select: Option<Callback<()>>,
) -> impl IntoView {
    let pending = station.status.is_none() || station.status == Some(crate::components::workflow_graph::types::StepRunStatus::Pending);
    let name_class = format!(
        "truncate text-ui-caption font-medium {}",
        if pending { "text-foreground-subtle" } else { "text-foreground" }
    );
    // 站头内部：名字 + 元数据（真源 :44-57）。caret 按 AnyView 不可克隆处理：
    // 两支共用同一构造函数、各传各的所有权（运行时只有一支会执行）。
    let head = move |name: String, caret: Option<AnyView>| -> AnyView {
        view! {
            <>
                <span class=name_class.clone()>
                    {name}
                    {caret}
                </span>
                <StationMeta station=station.clone() />
            </>
        }
        .into_any()
    };
    if on_select.is_none() {
        return view! {
            <span
                class=format!(
                    "wf-station flex h-6 min-w-0 shrink items-center gap-2 rounded-md text-left {fold_class}",
                )
                data-testid="workflow-timeline-station-head"
                title=title
            >
                {head(name, caret)}
            </span>
        }
        .into_any();
    }
    let on_select_cb = on_select.unwrap();
    view! {
        <button
            class=format!(
                "wf-station wf-station-open flex h-6 min-w-0 shrink cursor-pointer items-center gap-2 rounded-md text-left outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/40 {fold_class}",
            )
            on:click=move |_| on_select_cb.run(())
            title=title
            type="button"
        >
            {head(name, caret)}
        </button>
    }
    .into_any()
}

/// 轨道行（没有带时，真源 `WorkflowStationRow`，:97-189）：站头（灯 + 名字 + 元数据）
/// 与到下一站的轨道段，一站一格。折到檐上的站只留轨道段；药丸列不折（用户修订：
/// 两侧对称）——它们只随滚动走，在视口真正的边界处渐隐。
#[component]
pub fn WorkflowStationRow(
    stations: Vec<TimelineStation>,
    /// 站的全名，按下标；草稿里笔只写出前几个字。
    full_names: Vec<String>,
    folded: Vec<usize>,
    title_of: Callback<usize, String>,
    on_select_station: Option<Callback<usize>>,
    /// 相邻两站之间的轨道段，按 `railKey` 查。
    rails: HashMap<String, TimelineRail>,
    draft: bool,
    pen: TypewriterState,
    top: f64,
    width: f64,
) -> impl IntoView {
    let n = stations.len();
    view! {
        <div class="absolute left-0 flex" style=format!("height: {RAIL_ROW}px; top: {top}px; width: {width}px;")>
            {stations
                .iter()
                .enumerate()
                .map(|(i, station)| {
                    let rail = rails.get(&crate::components::timeline_ledge::rail_key(i, i + 1)).copied();
                    let full = full_names[i].clone();
                    let shown_count = pen.shown.get(i).copied().unwrap_or(0);
                    let name = if draft {
                        full.chars().take(shown_count).collect::<String>()
                    } else {
                        full.clone()
                    };
                    let pen_here = draft && i == n - 1;
                    let folded_here = folded.contains(&i);
                    let fold_class = format!(
                        "wf-foldable {}",
                        if folded_here { "wf-folded" } else { "" },
                    );
                    let status_word = station.status.map(|s| s.as_str()).unwrap_or("pending");
                    let ink_word = rail.as_ref().map(|r| r.ink.as_str()).unwrap_or("none");
                    let is_march = rail.as_ref().map(|r| r.ink == crate::components::timeline_model::TimelineInk::March).unwrap_or(false);
                    let rail_view = (i < n - 1).then(|| {
                        view! {
                            <span
                                aria-hidden="true"
                                class=format!(
                                    "wf-ink relative h-0 min-w-3 flex-1 rounded-full border-t border-foreground-subtlest {} {} {}",
                                    if draft { "wf-rail-grow" } else { "" },
                                    if rail.is_none() { "invisible" } else { "" },
                                    // 行进的段照常画底线，再叠一道不动的光（`.wf-rail-march::after`）：朝着灯渐亮。
                                    if is_march { "wf-rail-march" } else { "" },
                                )
                                data-rail-from=i
                                data-rail-ink=ink_word
                                data-rail-to=i + 1
                                data-testid="workflow-timeline-rail"
                                style="margin-left: 10px; margin-right: -6px;"
                            />
                        }
                    });
                    let caret = pen_here.then(|| {
                        // 光标跟着笔：写字时稳住，追上流时闪烁（真源 :146-157）。
                        view! {
                            <span
                                aria-hidden="true"
                                class=format!(
                                    "ml-px inline-block h-3 w-px bg-foreground align-[-1px] {}",
                                    if pen.idle { "wf-caret" } else { "" },
                                )
                                data-pen=if pen.idle { "idle" } else { "writing" }
                                data-testid="workflow-timeline-caret"
                            />
                        }
                        .into_any()
                    });
                    view! {
                        <div
                            class="flex h-6 min-w-0 items-center"
                            data-station-folded=folded_here.then_some("true")
                            data-station-status=status_word
                            data-testid="workflow-timeline-station"
                            style=format!(
                                "width: {}px;",
                                if i < n - 1 { STATION_PITCH } else { STATION_WIDTH },
                            )
                        >
                            <span
                                aria-hidden="true"
                                class=format!(
                                    "{} mx-3 {fold_class} {}",
                                    station_lamp_class(station.status),
                                    if draft { "wf-land" } else { "" },
                                )
                                data-lamp=status_word
                            />
                            {match on_select_station.clone() {
                                Some(on_select) => view! {
                                    <StationHead
                                        caret=caret
                                        fold_class=fold_class.clone()
                                        name=name
                                        on_select=Some(Callback::new(move |_| on_select.run(i)))
                                        station=station.clone()
                                        title=title_of.run(i)
                                    />
                                }
                                .into_any(),
                                None => view! {
                                    <StationHead
                                        caret=caret
                                        fold_class=fold_class.clone()
                                        name=name
                                        on_select=None
                                        station=station.clone()
                                        title=title_of.run(i)
                                    />
                                }
                                .into_any(),
                            }}
                            {rail_view}
                        </div>
                    }
                    .into_any()
                })
                .collect_view()}
        </div>
    }
}

/// 站台行（有带时，真源 `WorkflowStationPlatform`，:196-232）：站头一律搬到轨道下面
/// 这一行，灯留在自己的轨道上，分支轨道的站由一条点状引线接回名字——主线的站不需要，
/// 它的灯就在名字正上方。草稿永远没有带，所以这里不必管笔。
#[component]
pub fn WorkflowStationPlatform(
    stations: Vec<TimelineStation>,
    full_names: Vec<String>,
    folded: Vec<usize>,
    layout: TimelineLayout,
    title_of: Callback<usize, String>,
    on_select_station: Option<Callback<usize>>,
) -> impl IntoView {
    view! {
        {stations
            .iter()
            .enumerate()
            .map(|(i, station)| {
                let folded_here = folded.contains(&i);
                let fold_class = format!(
                    "wf-foldable {}",
                    if folded_here { "wf-folded" } else { "" },
                );
                let status_word = station.status.map(|s| s.as_str()).unwrap_or("pending");
                let left = station_x(i, layout.inset) + CAPTION_X;
                let top = layout.cap_y - PLATFORM_ROW / 2.0;
                let width = STATION_WIDTH - CAPTION_X;
                view! {
                    <div
                        class="absolute flex h-6 min-w-0 items-center"
                        data-station-folded=folded_here.then_some("true")
                        data-station-status=status_word
                        data-station-track=station.track
                        data-testid="workflow-timeline-station"
                        style=format!("left: {left}px; top: {top}px; width: {width}px;")
                    >
                        {match on_select_station.clone() {
                            Some(on_select) => view! {
                                <StationHead
                                    caret=None
                                    fold_class=fold_class.clone()
                                    name=full_names[i].clone()
                                    on_select=Some(Callback::new(move |_| on_select.run(i)))
                                    station=station.clone()
                                    title=title_of.run(i)
                                />
                            }
                            .into_any(),
                            None => view! {
                                <StationHead
                                    caret=None
                                    fold_class=fold_class.clone()
                                    name=full_names[i].clone()
                                    on_select=None
                                    station=station.clone()
                                    title=title_of.run(i)
                                />
                            }
                            .into_any(),
                        }}
                    </div>
                }
                .into_any()
            })
            .collect_view()}
    }
}
