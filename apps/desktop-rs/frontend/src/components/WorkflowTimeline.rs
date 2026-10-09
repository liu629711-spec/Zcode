//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowTimeline.tsx`（426 行）。
//!
//! 横向时间线主容器：一根轨道、站在轨道上、药丸挂在站下、弧在轨道上方的空气里。
//! 纯 DOM + 一层 SVG（只画弧），没有画布库。
//!
//! 宽过容器就自由滚动：灯滚出视口的站折叠到那一侧的**边檐**上——同一枚灯、16px 一枚，
//! 落在视口边缘干净的底上。镜头对准正在运行的站；镜头在飞时目标站不折（视口在
//! scrollTo 之前量过，飞行中按陈旧位置算它在视野外是误报）。草稿（`model.draft`）
//! 由笔写出：站按声明序一站一站揭示，轨道段从左伸出、灯落地、名字逐字写出。
//!
//! Rust 侧：视口钩子（`use_timeline_viewport`）量不到（宽 0）时什么都不折——静态测试
//! 结果逐像素保持不变。镜头飞行（`flight`）由滚动位置结算，不用计时器。

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::roster_model::{roster_more, station_roster_of, ROSTER_PINS_CARD};
use crate::components::timeline_geometry::{
    station_x, timeline_layout, timeline_width, PILL_GAP, PILL_HEIGHT, RAIL_ROW, STATION_PITCH,
    STATION_WIDTH,
};
use crate::components::timeline_ledge::{
    flight_after_scroll, fold_stations, ledge_stub_width, rail_key, station_camera_left,
    timeline_mask_style, CameraFlight, EdgeOverflow, LedgeSide, TimelineFold, TimelineMaskStyle,
};
use crate::components::timeline_model::{TimelinePill, TimelineStation, WorkflowTimelineModel};
use crate::components::useTimelineViewport::use_timeline_viewport;
use crate::components::useTypewriter::use_typewriter;
use crate::components::workflow_graph::phase_name::phase_display_name;
use crate::ToolCallBlocks::i18n;

use super::WorkflowAgentPill::{
    PillSize, WorkflowAgentPill, WorkflowAgentPillOpenData, WorkflowAgentPillVisual,
};
use super::WorkflowMoreRow::WorkflowMoreRow;
use super::WorkflowTimelineArcs::WorkflowTimelineArcs;
use super::WorkflowTimelineLedge::{
    prefers_reduced_motion, WorkflowLedge, WorkflowTimelineScrollbar,
};
use super::WorkflowTimelineStation::{WorkflowStationPlatform, WorkflowStationRow};
use super::WorkflowTimelineTracks::{TracksModel, WorkflowTimelineLamps, WorkflowTimelineTracks};

/// 药丸依次落地的间隔（与设计画布同值，真源 :88）。
pub const PILL_STAGGER_MS: i64 = 30;

/// 过了阈值的站是五枚钉住的药丸加一行「还有 n 个」：那一行就是第六枚药丸，
/// 所以一站永远不高于六枚药丸（真源 `stationHeight`，:68-74）。
pub fn station_height(station: &TimelineStation) -> f64 {
    let roster = station_roster_of(station, ROSTER_PINS_CARD);
    let n = match roster {
        None => station.pills.len(),
        Some(roster) => roster.pinned.len() + 1,
    };
    if n == 0 {
        0.0
    } else {
        n as f64 * PILL_HEIGHT + (n - 1) as f64 * PILL_GAP
    }
}

/// 时间线的整体高度（真源 `timelineHeight`，:82-85）：弧道 + 轨道行 + 站台行 + 最高的
/// 一列药丸。轮尾摘要展开 / 收起时把外框的高度在两个值之间过渡，所以它必须能在渲染
/// 之外算出来。末尾 8px 留白是滚动条的家（药丸下 6px、2px 一根，悬停 4px）。
pub fn timeline_height(model: &WorkflowTimelineModel) -> f64 {
    let rows = timeline_layout(&model.arcs, &model.bands);
    rows.pills_top
        + model.stations.iter().map(station_height).fold(0.0f64, f64::max)
        + 8.0
}

/// 药丸名（真源 `pillName`，:90-92）：运行时名 > 车道显示名。
pub fn pill_name(pill: &TimelinePill) -> String {
    if let Some(name) = &pill.runtime_name {
        return name.clone();
    }
    crate::components::workflow_graph::lane_name::lane_display_name(
        &pill.lane.naming,
        &|id: &str| i18n::text(id),
    )
}

/// 渲染用的裁剪模型（真源 :150 `draft ? { ...model, stations }`）。
pub fn clipped_model(model: &WorkflowTimelineModel, visible: usize) -> WorkflowTimelineModel {
    let mut clipped = model.clone();
    clipped.stations = model.stations.iter().take(visible).cloned().collect();
    clipped
}

/// 滚动行为（reduced-motion 下瞬时滚动）。
fn scroll_behavior() -> web_sys::ScrollBehavior {
    if prefers_reduced_motion() {
        web_sys::ScrollBehavior::Auto
    } else {
        web_sys::ScrollBehavior::Smooth
    }
}

fn scroll_to(element: &web_sys::Element, left: f64) {
    let options = web_sys::ScrollToOptions::new();
    options.set_behavior(scroll_behavior());
    options.set_left(left);
    let _ = element.scroll_to_with_scroll_to_options(&options);}

/// 实例计数器：真源 `useId`（:124）的 Rust 对应物——SVG 渐变 id 要整树唯一。
pub fn next_instance_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

/// `WorkflowTimeline`（真源 :114-426）。
#[component]
pub fn WorkflowTimeline(
    model: WorkflowTimelineModel,
    #[prop(optional, into)] class: String,
    /// 点一个站；缺席即站头不是控件——点击冒泡给宿主（轮尾摘要的整块开关）。
    on_select_station: Option<Callback<usize>>,
    /// 点名册站的「还有 n 个」那一行：开 run 详情、落到这一站。
    on_open_more: Option<Callback<usize>>,
    /// 点一枚药丸（有会话开 transcript，没有开占位）；缺席即不可点。
    on_open_pill: Option<Callback<TimelinePill>>,
    /// 点脚本药丸：开整个 run 的脚本 transcript，落到这一站；缺席即脚本药丸不可点。
    on_open_workspace: Option<Callback<TimelinePill>>,
) -> impl IntoView {
    let marker_id = format!("wf-timeline-{}", next_instance_id());
    // 真源 :125-132 —— 滚动层元素同时进状态：callback ref 出现就接线、离开就拆。
    // Rust 侧：NodeRef + Effect 把元素写进信号（视口钩子以信号为依赖）。
    let scroll_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let scroll_element: RwSignal<Option<web_sys::Element>> = RwSignal::new(None);
    {
        let element_signal = scroll_element;
        Effect::new(move |_| {
            let element = scroll_ref
                .get()
                .and_then(|div| div.unchecked_ref::<web_sys::Element>().clone().into());
            element_signal.set(element);
        });
    }
    // 镜头的一班飞行：起飞时记下，落地或用户接手时清空；飞行中目标站不折（真源 :134）。
    let flight: RwSignal<Option<CameraFlight>> = RwSignal::new(None);

    let draft = model.draft.is_some();
    let full_names: Vec<String> = model
        .stations
        .iter()
        .map(|station| phase_display_name(&station.naming))
        .collect();
    let pen_memo = use_typewriter(draft.then(|| full_names.clone()));
    let pen_state = pen_memo.get_untracked();
    let stations: Vec<TimelineStation> = if draft {
        model.stations.iter().take(pen_state.visible).cloned().collect()
    } else {
        model.stations.clone()
    };
    let n = stations.len();
    let rails: Vec<crate::components::timeline_model::TimelineRail> = if draft {
        model
            .rails
            .iter()
            .filter(|rail| rail.to < n)
            .copied()
            .collect()
    } else {
        model.rails.clone()
    };
    // 行的排布：有带时主线落在最下面一行、分支叠在它上面、站头搬到站台行（真源 :143-148）。
    let layout = timeline_layout(&model.arcs, &model.bands);
    let banded = layout.banded;
    let inset = layout.inset;
    let pills_top = layout.pills_top;
    let top = layout.row_y.first().copied().unwrap_or(0.0) - RAIL_ROW / 2.0;
    let width = timeline_width(n, inset);
    let model_for_height = if draft {
        clipped_model(&model, pen_state.visible)
    } else {
        model.clone()
    };
    let height = timeline_height(&model_for_height);
    // 轨道段按**一对站**查：带里一站可以同时长出主线的一条、分叉的一条与双线段
    // （真源 :160-163）。
    let rail_by_pair: HashMap<String, crate::components::timeline_model::TimelineRail> = rails
        .iter()
        .map(|rail| (rail_key(rail.from, rail.to), *rail))
        .collect();

    // 视口：滚到哪、多宽、内容多宽。量不到时三者为 0，下面什么都不折（真源 :165-166）。
    let viewport_memo = use_timeline_viewport(scroll_element.read_only(), n);
    let viewport = viewport_memo.get_untracked();
    // 每次滚动采样后结算飞行：落地或偏离即结束。只由位置决定，不用计时器（真源 :168-170）。
    Effect::new(move |_| {
        let scroll_left = viewport_memo.get().scroll_left;
        let current = flight.get_untracked();
        flight.set(flight_after_scroll(current, scroll_left));
    });
    let overflow = viewport.client_width > 0.0 && viewport.scroll_width > viewport.client_width;
    let fold: TimelineFold = if overflow {
        let keep = flight.get_untracked().map(|f| f.index);
        fold_stations(n, viewport.scroll_left, viewport.client_width, keep, inset)
    } else {
        TimelineFold::empty()
    };
    let folded: Vec<usize> = fold.left.iter().chain(fold.right.iter()).copied().collect();
    // 遮罩分两带：轨道带在檐旁渐隐，药丸带只在视口真正的边界渐隐（真源 :176-182）。
    let mask: Option<TimelineMaskStyle> = overflow.then(|| {
        timeline_mask_style(
            &fold,
            pills_top - 8.0,
            EdgeOverflow {
                left: viewport.scroll_left > 0.0,
                right: viewport.scroll_left
                    < viewport.scroll_width - viewport.client_width - 1.0,
            },
        )
    })
    .flatten();
    let mask_style = mask.map(|mask| {
        // 真源 :302-311 的 style 对象展开成 CSS（mask 双前缀：Safari 需要 -webkit-）。
        format!(
            "-webkit-mask-image: {img}; mask-image: {img}; -webkit-mask-position: {pos}; mask-position: {pos}; -webkit-mask-repeat: {rep}; mask-repeat: {rep}; -webkit-mask-size: {sz}; mask-size: {sz};",
            img = mask.image,
            pos = mask.position,
            rep = mask.repeat,
            sz = mask.size,
        )
    });

    // 镜头：宽过容器时把焦点站滚进视野；用户自己滚过就不再抢（真源 :209-225）。
    // run 里焦点是正在运行的站（居中）；草稿里焦点跟着笔走——最新揭示的站贴着右缘。
    // 草稿的落点是 index·PITCH + STATION_WIDTH − clientWidth（不能多加一个站距）。
    let focus_index: Option<usize> = if draft {
        if n > 0 { Some(n - 1) } else { None }
    } else {
        model.running_index
    };
    Effect::new(move |_| {
        let Some(element) = scroll_element.get_untracked() else {
            return;
        };
        let Some(focus) = focus_index else {
            return;
        };
        if element.scroll_width() <= element.client_width() {
            return;
        }
        let element_width = element.client_width() as f64;
        let left = if draft {
            (station_x(focus, inset) + STATION_WIDTH - element_width).max(0.0)
        } else {
            station_camera_left(focus, element_width, inset)
        };
        // 起飞：目标按可滚范围夹紧；已在目标上就不算飞。
        let target = left.min(element.scroll_width() as f64 - element_width);
        let from = element.scroll_left() as f64;
        flight.set(if (from - target).abs() <= 1.0 {
            None
        } else {
            Some(CameraFlight { from, index: focus, target })
        });
        scroll_to(&element, left);
    });

    if n == 0 {
        return view! { <span class="hidden" /> }.into_any();
    }

    // 站标题（真源 stationTitle，:233-238）：循环上的站带「已到访 n 次」。
    let station_title = move |station: &TimelineStation| -> String {
        let name = phase_display_name(&station.naming);
        if station.on_loop && station.rounds > 0 {
            format!(
                "{name} · {}",
                i18n::format(
                    "chat.toolCall.workflow.timeline.rounds",
                    &[("count".to_string(), station.rounds.to_string())],
                )
            )
        } else {
            name
        }
    };
    let stations_for_title = stations.clone();
    let title_of = Callback::new(move |i: usize| -> String {
        stations_for_title
            .get(i)
            .map(&station_title)
            .unwrap_or_default()
    });

    // 药丸按站从左到右、站内从上到下依次落地：第 k 枚延迟 k × 30 ms
    // （「还有 n 个」那一行也排队）（真源 :151-157）。
    let pill_ordinal = Cell::new(0usize);
    let next_delay = || -> i64 {
        let delay = PILL_STAGGER_MS * pill_ordinal.get() as i64;
        pill_ordinal.set(pill_ordinal.get() + 1);
        delay
    };

    // 檐上的灯：镜头把那一站带回正中（与运行站的镜头同一条规则，真源 :190-194）。
    let select_folded = Callback::new(move |index: usize| {
        if let Some(element) = scroll_element.get_untracked() {
            let left = station_camera_left(index, element.client_width() as f64, inset);
            scroll_to(&element, left);
        }
    });

    // 药丸渲染（真源 renderPill，:240-275）。活的 run 里的子代理药丸一律可开：
    // 有会话开 transcript，还没启动的开同一个 tab 的占位——槽位身份（`pill.slot`）
    // 就是它的抓手。脚本药丸同一套打开语法。没有 run 就没有槽位。
    let render_pill = |pill: &TimelinePill, enter_delay_ms: i64| -> AnyView {
        let label = pill_name(pill);
        let open_data = if on_open_pill.is_some() && pill.slot.is_some() {
            Some(WorkflowAgentPillOpenData {
                label: i18n::format(
                    "chat.toolCall.workflow.timeline.openAgent",
                    &[("name".to_string(), label.clone())],
                ),
                test_id: Some("workflow-timeline-pill-open".to_string()),
                data_pairs: Vec::new(),
            })
        } else if on_open_workspace.is_some() && pill.workspace.is_some() {
            // 落到这一站的第一张卡（真源 :258-262）。
            let phase_id = pill
                .workspace
                .as_ref()
                .map(|workspace| workspace.phase_id.clone())
                .unwrap_or_default();
            let phase_title = model
                .stations
                .iter()
                .find(|candidate| candidate.id == phase_id)
                .map(&station_title)
                .unwrap_or_else(|| phase_id.clone());
            Some(WorkflowAgentPillOpenData {
                label: i18n::format(
                    "chat.toolCall.workflow.timeline.openScript",
                    &[("phase".to_string(), phase_title)],
                ),
                test_id: Some("workflow-timeline-workspace-open".to_string()),
                data_pairs: Vec::new(),
            })
        } else {
            None
        };
        let on_open: Option<Callback<()>> = if open_data.is_some() {
            if pill.slot.is_some() && on_open_pill.is_some() {
                let cb = on_open_pill.clone().unwrap();
                let owned = pill.clone();
                Some(Callback::new(move |_| cb.run(owned.clone())))
            } else if on_open_workspace.is_some() {
                let cb = on_open_workspace.clone().unwrap();
                let owned = pill.clone();
                Some(Callback::new(move |_| cb.run(owned.clone())))
            } else {
                None
            }
        } else {
            None
        };
        view! {
            <WorkflowAgentPill
                children=None
                on_open=on_open
                open_data=open_data
                trailing=None
                visual=WorkflowAgentPillVisual {
                    avatar_index: pill.avatar_index,
                    enter_delay_ms: Some(enter_delay_ms),
                    name: label,
                    lane_class: pill.lane_class,
                    status: pill.status,
                    title: None,
                    class: String::new(),
                    inert_title: None,
                    size: PillSize::Md,
                }
            />
        }
        .into_any()
    };

    // 名册站与药丸列（真源 :367-400）。
    let pill_columns: Vec<AnyView> = stations
        .iter()
        .enumerate()
        .filter_map(|(i, station)| {
            let roster = station_roster_of(station, ROSTER_PINS_CARD);
            // 名册在就还有话说：一站的药丸全被界淘汰掉时剩下「还有 n 个」那一行，
            // 而不是凭空消失（真源 :369-370）。
            if station.pills.is_empty() && roster.is_none() {
                return None;
            }
            let left = station_x(i, inset);
            let pinned_views: Vec<AnyView> = match &roster {
                // 名册站：钉住的五枚沿用药丸的接线（真源 :387-390）。
                Some(roster) => roster
                    .pinned
                    .iter()
                    .map(|pill| render_pill(pill, next_delay()))
                    .collect(),
                None => station.pills.iter().map(|pill| render_pill(pill, next_delay())).collect(),
            };
            let more_view: Option<AnyView> = roster.as_ref().map(|roster| {
                // 「还有 n 个」——点它开 run 详情、落到这一站（`onOpenMore`，自己的门）。
                let on_open = on_open_more.clone().map(|cb| {
                    let index = i;
                    Callback::new(move |_| cb.run(index))
                });
                view! {
                    <WorkflowMoreRow
                        door=None
                        enter_delay_ms=Some(next_delay())
                        more=roster_more(roster, crate::components::roster_model::ROSTER_DECK)
                        on_open=on_open
                    />
                }
                .into_any()
            });
            let column: AnyView = view! {
                <div
                    class="absolute flex flex-col"
                    data-station-roster=roster.as_ref().map(|_| "true")
                    data-testid="workflow-timeline-pills"
                    style=format!(
                        "gap: {PILL_GAP}px; left: {left}px; top: {pills_top}px; width: {STATION_WIDTH}px;",
                    )
                >
                    {pinned_views.into_iter().collect_view()}
                    {more_view}
                </div>
            }
            .into_any();
            Some(column)
        })
        .collect();

    let ledge_name_of = {
        let full_names = full_names.clone();
        Callback::new(move |index: usize| -> String {
            full_names.get(index).cloned().unwrap_or_default()
        })
    };
    let stub_left = ledge_stub_width(
        &fold,
        LedgeSide::Left,
        viewport.scroll_left,
        viewport.client_width,
        inset,
    );
    let stub_right = ledge_stub_width(
        &fold,
        LedgeSide::Right,
        viewport.scroll_left,
        viewport.client_width,
        inset,
    );

    // 键盘：焦点在站头或檐上的灯时，← → 各滚一站（真源 :196-207）。
    let on_key_down = move |event: leptos::ev::KeyboardEvent| {
        if event.key() != "ArrowLeft" && event.key() != "ArrowRight" {
            return;
        }
        let Some(target) = event.target() else {
            return;
        };
        let Ok(element_target) = target.dyn_into::<web_sys::Element>() else {
            return;
        };
        if element_target
            .closest("[data-testid='workflow-timeline-station'], .wf-ledge")
            .ok()
            .flatten()
            .is_none()
        {
            return;
        }
        let Some(element) = scroll_element.get_untracked() else {
            return;
        };
        if element.scroll_width() <= element.client_width() {
            return;
        }
        event.prevent_default();
        let delta = if event.key() == "ArrowLeft" {
            -STATION_PITCH
        } else {
            STATION_PITCH
        };
        scroll_to(&element, element.scroll_left() as f64 + delta);
    };

    let data_timeline_fade = if !fold.left.is_empty() && !fold.right.is_empty() {
        "both"
    } else if !fold.left.is_empty() {
        "left"
    } else if !fold.right.is_empty() {
        "right"
    } else {
        ""
    };
    let stalled = model.stalled;
    let root_class = format!("wf-motion wf-timeline min-w-0 {class}");
    let stalled_title = stalled.then(|| i18n::text("chat.toolCall.workflow.timeline.runStalled"));

    // 有带时轨道进弧那一层 SVG 的 children（真源 :324-332）。
    // ChildrenFn = Arc<dyn Fn() -> AnyView + Send + Sync>：先在 let 上完成 unsize 强转，
    // 再装进 Option（泛型内不传播强转）。
    let tracks_children: Option<ChildrenFn> = banded.then_some({
        let folded = folded.clone();
        let layout = layout.clone();
        let model = model.clone();
        let marker = marker_id.clone();
        let build: ChildrenFn = Arc::new(move || {
            view! {
                <WorkflowTimelineTracks
                    folded=folded.clone()
                    gradient_base=format!("{marker}-")
                    layout=layout.clone()
                    model=TracksModel::from_model(&model)
                />
            }
            .into_any()
        });
        build
    });

    view! {
        <div
            class=root_class
            data-stalled=stalled.then_some("true")
            data-testid="workflow-timeline"
            on:keydown=on_key_down
            title=stalled_title
        >
            // 檐与滚动条叠在滚动层之上，以它（而不是带 padding 的外框）为基准定位（真源 :297）。
            <div class="relative">
                <div
                    class="wf-scroller overflow-x-auto overflow-y-hidden"
                    data-testid="workflow-timeline-scroller"
                    data-timeline-fade=data_timeline_fade
                    node_ref=scroll_ref
                    style=mask_style
                >
                    <div class="relative" style=format!("height: {height}px; width: {width}px;")>
                        <WorkflowTimelineArcs
                            arcs=model.arcs.clone()
                            bands=model.bands.clone()
                            children=tracks_children
                            height=height
                            layout=layout.clone()
                            marker_id=marker_id.clone()
                            stations=model.stations.clone()
                            width=width
                        />

                        {banded.then(|| {
                            view! {
                                <WorkflowTimelineLamps
                                    draft=draft
                                    folded=folded.clone()
                                    layout=layout.clone()
                                    stations=stations.clone()
                                />
                            }
                        })}

                        {if banded {
                            view! {
                                <WorkflowStationPlatform
                                    folded=folded.clone()
                                    full_names=full_names.clone()
                                    layout=layout.clone()
                                    on_select_station=on_select_station
                                    stations=stations.clone()
                                    title_of=title_of
                                />
                            }
                            .into_any()
                        } else {
                            view! {
                                <WorkflowStationRow
                                    draft=draft
                                    folded=folded.clone()
                                    full_names=full_names.clone()
                                    on_select_station=on_select_station
                                    pen=pen_state.clone()
                                    rails=rail_by_pair.clone()
                                    stations=stations.clone()
                                    title_of=title_of
                                    top=top
                                    width=width
                                />
                            }
                            .into_any()
                        }}

                        {pill_columns.into_iter().collect_view()}
                    </div>
                </div>
                // 边檐：折到视口两侧的站，一排灯落在轨道行上，檐下的内容已被遮罩清空（真源 :403）。
                <WorkflowLedge
                    indexes=fold.left.clone()
                    name_of=ledge_name_of
                    on_select=select_folded
                    rails=rail_by_pair.clone()
                    side="left"
                    stations=stations.clone()
                    stub_width=stub_left
                    top=top
                />
                <WorkflowLedge
                    indexes=fold.right.clone()
                    name_of=ledge_name_of
                    on_select=select_folded
                    rails=rail_by_pair.clone()
                    side="right"
                    stations=stations.clone()
                    stub_width=stub_right
                    top=top
                />
                {overflow.then(|| {
                    view! {
                        <WorkflowTimelineScrollbar
                            scroll_element=scroll_element
                            viewport=viewport
                        />
                    }
                })}
            </div>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_station(pill_count: usize) -> TimelineStation {
        let mut pills = Vec::new();
        for i in 0..pill_count {
            pills.push(TimelinePill {
                key: format!("p{i}"),
                lane: crate::components::workflow_graph::lane_name::LaneRef {
                    id: format!("agent-{i}"),
                    naming: Default::default(),
                },
                lane_class: crate::components::workflow_graph::types::LaneClass::Agent,
                runtime_name: Some(format!("子代理 {i}")),
                avatar_index: Some(i),
                status: Some(crate::components::workflow_graph::types::StepRunStatus::Done),
                instance: None,
                slot: Some(crate::components::timeline_model::PillSlot {
                    site_id: "s1".into(),
                    ordinal: i as i64,
                }),
                workspace: None,
                step_ids: Vec::new(),
                asking: false,
            });
        }
        TimelineStation {
            id: "phase-1".into(),
            naming: Default::default(),
            pills,
            status: None,
            visited: false,
            rounds: 0,
            on_loop: false,
            track: 0,
            fraction: None,
            unlisted: None,
            typing: false,
        }
    }

    #[test]
    fn pill_stagger_matches_truth_source() {
        // 真源 :88 —— 30ms 一枚。
        assert_eq!(PILL_STAGGER_MS, 30);
    }

    #[test]
    fn station_height_never_exceeds_six_pills() {
        // 真源 :68-74 —— 过阈值的站是五枚钉住 + 「还有 n 个」 = 六枚药丸的高度。
        let mut station = test_station(12);
        let expected = 6.0 * PILL_HEIGHT + 5.0 * PILL_GAP;
        assert_eq!(station_height(&station), expected);
        // 普通站：pills.len() 行。
        station.pills.truncate(3);
        let expected = 3.0 * PILL_HEIGHT + 2.0 * PILL_GAP;
        assert_eq!(station_height(&station), expected);
        // 空站：0。
        station.pills.clear();
        assert_eq!(station_height(&station), 0.0);
    }

    #[test]
    fn clipped_model_slices_stations() {
        let mut model = WorkflowTimelineModel {
            stations: (0..5).map(|_| test_station(0)).collect(),
            rails: Vec::new(),
            arcs: Vec::new(),
            bands: Vec::new(),
            running_index: None,
            live: false,
            stalled: false,
            draft: None,
        };
        for (i, station) in model.stations.iter_mut().enumerate() {
            station.id = format!("p{i}");
        }
        let clipped = clipped_model(&model, 2);
        assert_eq!(clipped.stations.len(), 2);
        assert_eq!(clipped.stations[1].id, "p1");
        // 其余部分不动。
        assert_eq!(clipped.rails.len(), model.rails.len());
        assert_eq!(clipped.arcs.len(), model.arcs.len());
    }

    #[test]
    fn instance_ids_are_unique() {
        let a = next_instance_id();
        let b = next_instance_id();
        assert_ne!(a, b);
    }
}
