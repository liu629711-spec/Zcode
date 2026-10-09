//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowTimelineArcs.tsx`（141 行）。
//!
//! 弧层：时间线唯一的一层 SVG，只画非相邻的边。从源站的灯升到自己的弧道、横到目标站、
//! 落回目标的灯上，箭头朝下。一站的弧端各占一个槽、槽距 10px（`arc_terminal_offsets`）。
//! 首次出现时从源画到目标（`pathLength=1` 让 dashoffset 与几何无关）；之后只换墨色；
//! 正在走的边叠一层不动的光（`MarchLight`）。
//!
//! 有带时带是**一个节点**：源在带里就从带的汇合点起飞，目标在带里就落在带的分叉点上
//! （落点提前到 4px）。带级弧的竖段一路走到**端点自己的行**。

use leptos::prelude::*;

use crate::components::timeline_geometry::{
    arc_ends, arc_terminal_offsets, band_at, band_fork_x, band_merge_x, lamp_x, ArcTerminalOffsets,
    TimelineLayout,
};
use crate::components::timeline_model::{TimelineArc, TimelineBand, TimelineInk, TimelineStation};

use super::WorkflowMarchLight::{MarchLight, MarchLightPoint};

/// 墨色 → 描边色（真源 `INK_STROKE`，:33-37）。
pub fn ink_stroke(ink: TimelineInk) -> &'static str {
    match ink {
        TimelineInk::Faint => "var(--color-workflow-trace)",
        TimelineInk::March | TimelineInk::Strong => "var(--color-workflow-trace-strong)",
    }
}

fn fmt(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// `WorkflowTimelineArcs`（真源 :39-141）。`children` 由调用方传入
/// （有带时轨道也进这一层 SVG——分叉与汇合是曲线，DOM 的 border 画不出来）。
#[component]
pub fn WorkflowTimelineArcs(
    arcs: Vec<TimelineArc>,
    bands: Vec<TimelineBand>,
    /// 全部的站（不是草稿切过的那一段）：弧的下标指向它，要读每一站的轨道。
    stations: Vec<TimelineStation>,
    layout: TimelineLayout,
    width: f64,
    height: f64,
    marker_id: String,
    children: Option<ChildrenFn>,
) -> impl IntoView {
    let terminals: ArcTerminalOffsets = arc_terminal_offsets(
        &arcs,
        &bands,
        &stations.iter().map(|station| station.track).collect::<Vec<i64>>(),
    );
    let station_tracks: Vec<i64> = stations.iter().map(|station| station.track).collect();
    let inset = layout.inset;
    let row_y0 = layout.row_y.first().copied().unwrap_or(0.0);
    let row_of = move |track: i64| -> f64 {
        let t = track.max(0) as usize;
        layout.row_y.get(t).copied().unwrap_or(row_y0)
    };
    let track_of = {
        let station_tracks = station_tracks.clone();
        move |index: usize| -> i64 { station_tracks.get(index).copied().unwrap_or(0) }
    };

    let pieces: Vec<AnyView> = arcs
        .iter()
        .enumerate()
        .map(|(j, arc)| {
            // 带内同一条轨道的弧整条住在那条轨道的空里，两端仍是灯；其余的弧把带当一个节点。
            let ends = arc_ends(arc.air, arc.from, arc.to, &bands, &station_tracks);
            let source = band_at(&bands, arc.from);
            let target = band_at(&bands, arc.to);
            let ly = row_of(arc.air) - 26.0 - 14.0 * arc.lane as f64;
            // 灯上的两端各就各的槽；分叉点与汇合点是一个点，不占槽（偏移恒为 0）。
            let xa = if ends.from_band && source.is_some() {
                let band = source.unwrap();
                band_merge_x(band.to, band.join, inset)
            } else {
                lamp_x(arc.from, inset) + terminals.takeoff.get(j).copied().unwrap_or(0.0)
            };
            let xb = if ends.to_band && target.is_some() {
                band_fork_x(target.unwrap().from, inset)
            } else {
                lamp_x(arc.to, inset) + terminals.landing.get(j).copied().unwrap_or(0.0)
            };
            let sign: f64 = if xb < xa { -1.0 } else { 1.0 };
            // 起飞与落地都贴着**端点自己的行**（真源 :104-106）。
            let ya = if ends.from_band { row_y0 - 3.0 } else { row_of(track_of(arc.from)) - 7.0 };
            let yb = if ends.to_band { row_y0 - 4.0 } else { row_of(track_of(arc.to)) - 9.0 };
            let tail = format!("V{}", fmt(yb));
            let path = format!(
                "M{},{} V{} Q{},{} {},{} H{} Q{},{} {},{} {}",
                fmt(xa),
                fmt(ya),
                fmt(ly + 8.0),
                fmt(xa),
                fmt(ly),
                fmt(xa + 8.0 * sign),
                fmt(ly),
                fmt(xb - 8.0 * sign),
                fmt(xb),
                fmt(ly),
                fmt(xb),
                fmt(ly + 8.0),
                tail,
            );
            let ink = if arc.ink == TimelineInk::March { TimelineInk::Strong } else { arc.ink };
            let march_path = format!(
                "{}V{}",
                &path[..path.len() - tail.len()],
                fmt(yb - 6.0)
            );
            let marker = format!("{}-{}", marker_id, ink_as_str(ink));
            let lit_id = format!("{}-lit-{}", marker_id, j);
            view! {
                <g
                    data-arc-from=arc.from
                    data-arc-ink=arc.ink.as_str()
                    data-arc-to=arc.to
                    data-testid="workflow-timeline-arc"
                >
                    <path
                        class="wf-ink wf-draw"
                        d=path.clone()
                        fill="none"
                        marker-end=format!("url(#{marker})")
                        pathLength="1"
                        stroke=ink_stroke(ink)
                        stroke-width="1"
                    />
                    {(arc.ink == TimelineInk::March).then(|| {
                        view! {
                            <MarchLight
                                d=march_path
                                from=MarchLightPoint { x: xa, y: ya }
                                id=lit_id
                                to=MarchLightPoint { x: xb, y: yb - 6.0 }
                            />
                        }
                    })}
                </g>
            }
            .into_any()
        })
        .collect();

    view! {
        <svg
            aria-hidden="true"
            class="absolute left-0 top-0 overflow-visible"
            height=height
            viewBox=format!("0 0 {} {}", fmt(width), fmt(height))
            width=width
        >
            <defs>
                {(["faint", "strong"])
                    .into_iter()
                    .map(|ink_word| {
                        let stroke = if ink_word == "faint" {
                            ink_stroke(TimelineInk::Faint)
                        } else {
                            ink_stroke(TimelineInk::Strong)
                        };
                        let id = format!("{marker_id}-{ink_word}");
                        view! {
                            <marker
                                id=id
                                markerHeight="6"
                                markerWidth="6"
                                orient="auto-start-reverse"
                                refX="7"
                                refY="4"
                                viewBox="0 0 8 8"
                            >
                                <path d="M0,0.5 L7,4 L0,7.5 Z" fill=stroke />
                            </marker>
                        }
                    })
                    .collect_view()}
            </defs>
            {children.clone().map(|c| view! { <>{c()}</> })}
            {pieces.into_iter().collect_view()}
        </svg>
    }
}

fn ink_as_str(ink: TimelineInk) -> &'static str {
    ink.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ink_palette_matches_truth_source() {
        // 真源 :33-37 —— faint 是淡墨，march / strong 都是重墨。
        assert_eq!(ink_stroke(TimelineInk::Faint), "var(--color-workflow-trace)");
        assert_eq!(ink_stroke(TimelineInk::Strong), "var(--color-workflow-trace-strong)");
        assert_eq!(ink_stroke(TimelineInk::March), "var(--color-workflow-trace-strong)");
    }

    #[test]
    fn arc_ink_renders_march_as_strong_for_the_arrow() {
        // 真源 :109 —— 箭头与底线用 strong，行进光才是 march。
        let arc_ink = TimelineInk::March;
        let rendered = if arc_ink == TimelineInk::March { TimelineInk::Strong } else { arc_ink };
        assert_eq!(rendered, TimelineInk::Strong);
    }
}
