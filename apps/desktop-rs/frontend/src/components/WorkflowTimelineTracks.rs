//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowTimelineTracks.tsx`（290 行）。
//!
//! 轨道层。没有带的时间线里轨道段还是站头行里的一截 border（逐像素不动）。
//! **有带**时整条轨道搬进弧那一层 SVG：主线走最下面一行、分支轨道叠在它上面，
//! 带前分叉、带后汇合，两段 8px 的四分之一圆把分支抬上去再放下来（分支向上）。
//! 灯仍是 DOM（状态类要用），绝对定位落在自己那条轨道的行上；站头搬到站台行。

use leptos::prelude::*;

use crate::components::timeline_bands::TimelineRailKind;
use crate::components::timeline_geometry::{band_at, band_fork_x, band_merge_x, lamp_x, TimelineLayout};
use crate::components::timeline_model::{
    TimelineBand, TimelineInk, TimelineRail, TimelineStation, WorkflowTimelineModel,
};

use super::WorkflowMarchLight::{MarchLight, MarchLightPoint};
use super::WorkflowTimelineLedge::station_lamp_class;

/// 墨色 → 描边色（真源 `INK_STROKE`，:29-33）。
pub fn ink_stroke(ink: TimelineInk) -> &'static str {
    match ink {
        TimelineInk::Faint => "var(--color-workflow-trace)",
        TimelineInk::March | TimelineInk::Strong => "var(--color-workflow-trace-strong)",
    }
}

/// 画布上的一个点（真源 `TimelinePoint`，:36-39）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelinePoint {
    pub x: f64,
    pub y: f64,
}

/// 段的种类（真源 :53 的字面量并集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailPieceKind {
    Fork,
    Merge,
    Tail,
    Stub,
}

impl RailPieceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RailPieceKind::Fork => "fork",
            RailPieceKind::Merge => "merge",
            RailPieceKind::Tail => "tail",
            RailPieceKind::Stub => "stub",
        }
    }
}

/// 画面上的一段轨道（真源 `TimelineRailPiece`，:42-56）：一条路径、一种墨，外加它接的
/// 两站（测试与调试的抓手）。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineRailPiece {
    pub key: String,
    pub d: String,
    /// 路径的首尾两点，与 `d` 出自同一组数——行进边的光要按这两点铺渐变（`MarchLight`），
    /// 而不是回头去解析 `d`。
    pub start: TimelinePoint,
    pub end: TimelinePoint,
    pub ink: TimelineInk,
    /// `fork` / `merge` 是带两端的曲线；`tail` / `stub` 是没有前驱 / 汇合站时主线的那一小截。
    pub kind: Option<RailPieceKind>,
    pub from: Option<usize>,
    pub to: Option<usize>,
}

/// 路径与它的首尾两点（真源 `RailShape`，:59）：每种段都先算出这三样，再配上墨与两站。
#[derive(Debug, Clone, PartialEq)]
pub struct RailShape {
    pub d: String,
    pub start: TimelinePoint,
    pub end: TimelinePoint,
}

impl TimelineRailPiece {
    fn from_shape(
        key: String,
        kind: Option<RailPieceKind>,
        from: Option<usize>,
        to: Option<usize>,
        ink: TimelineInk,
        shape: RailShape,
    ) -> Self {
        TimelineRailPiece {
            key,
            d: shape.d,
            start: shape.start,
            end: shape.end,
            ink,
            kind,
            from,
            to,
        }
    }
}

fn fmt(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// `timelineRailPieces`（真源 :66-185）：轨道段 → 路径（纯函数）。
///
/// 一条轨道上的普通段是那一行上的一条直线（灯 + 8 → 灯 − 8）；主线的那几条穿过
/// 分叉点与汇合点，于是主线自然是一条直线。两端都在**不同的带**里的普通段只可能是
/// 相邻两带之间的那一段，画在主线上、从前一带的汇合点到后一带的分叉点。
/// 双线段不画在卡上。
pub fn timeline_rail_pieces(
    bands: &[TimelineBand],
    rails: &[TimelineRail],
    stations: &[TimelineStation],
    layout: &TimelineLayout,
) -> Vec<TimelineRailPiece> {
    let row_y0 = layout.row_y.first().copied().unwrap_or(0.0);
    let row_y = layout.row_y.clone();
    let row_of = move |track: i64| -> f64 {
        let t = track.max(0) as usize;
        row_y.get(t).copied().unwrap_or(row_y0)
    };
    let inset = layout.inset;
    let lx = move |i: usize| lamp_x(i, inset);
    let fork_x_of = move |band: &TimelineBand| band_fork_x(band.from, inset);
    let merge_x_of = move |band: &TimelineBand| band_merge_x(band.to, band.join, inset);
    let track_of_station = |i: usize| stations.get(i).map(|s| s.track).unwrap_or(0);

    // 一行上的一条直线段（真源 straight，:80-84）。
    let straight = |x1: f64, x2: f64, y: f64| -> RailShape {
        RailShape {
            d: format!("M{},{} H{}", fmt(x1), fmt(y), fmt(x2)),
            start: TimelinePoint { x: x1, y },
            end: TimelinePoint { x: x2, y },
        }
    };
    // 分支轨道 t 在 forkX − 8(t−1) 处离开主线，两段四分之一圆升到自己的行，
    // 再横到第一枚灯前 8px（真源 forkPath，:86-94）。
    let fork_path = |band: &TimelineBand, track: i64, head: usize| -> RailShape {
        let xf = fork_x_of(band) - 8.0 * (track - 1) as f64;
        let yt = row_of(track);
        RailShape {
            d: format!(
                "M{},{} Q{},{} {},{} V{} Q{},{} {},{} H{}",
                fmt(xf),
                fmt(row_y0),
                fmt(xf + 8.0),
                fmt(row_y0),
                fmt(xf + 8.0),
                fmt(row_y0 - 8.0),
                fmt(yt + 8.0),
                fmt(xf + 8.0),
                fmt(yt),
                fmt(xf + 16.0),
                fmt(yt),
                fmt(lx(head) - 8.0),
            ),
            start: TimelinePoint { x: xf, y: row_y0 },
            end: TimelinePoint {
                x: lx(head) - 8.0,
                y: yt,
            },
        }
    };
    // 汇合是分叉的镜像：从末站的灯后 8px 横到 xm − 8，落回主线。
    // 带里轨道越高，落点越靠左（真源 mergePath，:96-104）。
    let merge_path = |band: &TimelineBand, track: i64, tail: usize| -> RailShape {
        let track_count = band.tracks.len() as i64;
        let xm = merge_x_of(band) - 8.0 * (track_count - 1 - track) as f64;
        let yt = row_of(track);
        RailShape {
            d: format!(
                "M{},{} H{} Q{},{} {},{} V{} Q{},{} {},{}",
                fmt(lx(tail) + 8.0),
                fmt(yt),
                fmt(xm - 8.0),
                fmt(xm),
                fmt(yt),
                fmt(xm),
                fmt(yt + 8.0),
                fmt(row_y0 - 8.0),
                fmt(xm),
                fmt(row_y0),
                fmt(xm + 8.0),
                fmt(row_y0),
            ),
            start: TimelinePoint {
                x: lx(tail) + 8.0,
                y: yt,
            },
            end: TimelinePoint {
                x: xm + 8.0,
                y: row_y0,
            },
        }
    };

    let mut pieces: Vec<TimelineRailPiece> = Vec::new();
    // 没有前驱 / 没有汇合站的带自己长出两端：主线的一小截尾巴 / 残段，和分支轨道的
    // 曲线——模型里没有对应的轨道段（分叉与汇合都要有那一站才成段），墨色取轨道自己的
    // entry / exit（真源 :107-153）。
    for band in bands {
        let main = band.tracks.first();
        if band.pred.is_none() {
            if let (Some(track0), Some(head)) =
                (main, main.and_then(|track| track.stations.first()))
            {
                let head = *head;
                let shape = straight(fork_x_of(band) - 4.0, lx(head) - 8.0, row_y0);
                pieces.push(TimelineRailPiece::from_shape(
                    format!("tail:{}", band.from),
                    Some(RailPieceKind::Tail),
                    None,
                    Some(head),
                    track0.entry,
                    shape,
                ));
            }
            for (t, track) in band.tracks.iter().enumerate() {
                if t == 0 {
                    continue;
                }
                if let Some(first) = track.stations.first() {
                    let first = *first;
                    let shape = fork_path(band, t as i64, first);
                    pieces.push(TimelineRailPiece::from_shape(
                        format!("fork:{}:{}", band.from, t),
                        Some(RailPieceKind::Fork),
                        None,
                        Some(first),
                        track.entry,
                        shape,
                    ));
                }
            }
        }
        if band.join.is_none() {
            if let (Some(track0), Some(last)) =
                (main, main.and_then(|track| track.stations.last()))
            {
                let last = *last;
                let shape = straight(lx(last) + 8.0, merge_x_of(band) + 6.0, row_y0);
                pieces.push(TimelineRailPiece::from_shape(
                    format!("stub:{}", band.to),
                    Some(RailPieceKind::Stub),
                    Some(last),
                    None,
                    track0.exit,
                    shape,
                ));
            }
            for (t, track) in band.tracks.iter().enumerate() {
                if t == 0 {
                    continue;
                }
                if let Some(end) = track.stations.last() {
                    let end = *end;
                    let shape = merge_path(band, t as i64, end);
                    pieces.push(TimelineRailPiece::from_shape(
                        format!("merge:{}:{}", band.to, t),
                        Some(RailPieceKind::Merge),
                        Some(end),
                        None,
                        track.exit,
                        shape,
                    ));
                }
            }
        }
    }

    for rail in rails {
        // 双线段只说「这两站并行」，卡上不画——分叉与汇合已经把并行说清楚了。
        if rail.kind == Some(TimelineRailKind::Twin) {
            continue;
        }
        let key = format!("{}>{}", rail.from, rail.to);
        if rail.kind == Some(TimelineRailKind::Fork) {
            if let Some(band) = band_at(bands, rail.to) {
                let shape = fork_path(band, track_of_station(rail.to), rail.to);
                pieces.push(TimelineRailPiece::from_shape(
                    key,
                    Some(RailPieceKind::Fork),
                    Some(rail.from),
                    Some(rail.to),
                    rail.ink,
                    shape,
                ));
            }
            continue;
        }
        if rail.kind == Some(TimelineRailKind::Merge) {
            if let Some(band) = band_at(bands, rail.from) {
                let shape = merge_path(band, track_of_station(rail.from), rail.from);
                pieces.push(TimelineRailPiece::from_shape(
                    key,
                    Some(RailPieceKind::Merge),
                    Some(rail.from),
                    Some(rail.to),
                    rail.ink,
                    shape,
                ));
            }
            continue;
        }
        let source = band_at(bands, rail.from);
        let target = band_at(bands, rail.to);
        if source.is_some() && target.is_some() && !std::ptr::eq(source.unwrap(), target.unwrap())
        {
            let (source, target) = (source.unwrap(), target.unwrap());
            let shape = straight(merge_x_of(source), fork_x_of(target), row_y0);
            pieces.push(TimelineRailPiece::from_shape(
                key,
                None,
                Some(rail.from),
                Some(rail.to),
                rail.ink,
                shape,
            ));
            continue;
        }
        let yt = row_of(track_of_station(rail.from));
        let shape = straight(lx(rail.from) + 8.0, lx(rail.to) - 8.0, yt);
        pieces.push(TimelineRailPiece::from_shape(
            key,
            None,
            Some(rail.from),
            Some(rail.to),
            rail.ink,
            shape,
        ));
    }
    pieces
}

/// `WorkflowTimelineTracks` 的入参（真源 `Pick<WorkflowTimelineModel, …>`）。
#[derive(Clone)]
pub struct TracksModel {
    pub bands: Vec<TimelineBand>,
    pub rails: Vec<TimelineRail>,
    pub stations: Vec<TimelineStation>,
}

impl TracksModel {
    pub fn from_model(model: &WorkflowTimelineModel) -> Self {
        TracksModel {
            bands: model.bands.clone(),
            rails: model.rails.clone(),
            stations: model.stations.clone(),
        }
    }
}

/// 轨道与引线，画在弧那一层 SVG 里（只有带时才挂上，真源 :191-250）。行进的段照弧的
/// 老规矩叠一层，只是那一层如今是**不动的**光（`MarchLight`）：从段的起点淡入、
/// 在灯那一头最亮。
///
/// 轨道层挂在弧那一层 SVG 里，渐变 id 要在整张 SVG 里唯一：实例前缀 + 段的 key
/// （去掉 `:` `>` 这些）。
#[component]
pub fn WorkflowTimelineTracks(
    model: TracksModel,
    layout: TimelineLayout,
    /// 折到檐上的站：它的引线跟着灯一起淡出。
    folded: Vec<usize>,
    /// 实例级 id 前缀（React useId 的 Rust 对应物：调用方传，与弧层共用同一命名空间）。
    gradient_base: String,
) -> impl IntoView {
    let pieces = timeline_rail_pieces(&model.bands, &model.rails, &model.stations, &layout);
    view! {
        <g>
            {pieces
                .iter()
                .map(|piece| {
                    let ink_word = piece.ink.as_str();
                    let kind_word = piece.kind.map(|k| k.as_str().to_string());
                    let gradient_id = format!(
                        "{}lit-{}",
                        gradient_base,
                        piece.key.replace(|c: char| !c.is_alphanumeric() && c != '-', "-"),
                    );
                    let d = piece.d.clone();
                    view! {
                        <g
                            data-rail-from=piece.from
                            data-rail-ink=ink_word
                            data-rail-kind=kind_word
                            data-rail-to=piece.to
                            data-testid="workflow-timeline-rail"
                        >
                            <path
                                class="wf-ink"
                                d=piece.d.clone()
                                fill="none"
                                stroke=ink_stroke(piece.ink)
                                stroke-width="1"
                            />
                            {(piece.ink == TimelineInk::March).then(|| {
                                view! {
                                    <MarchLight
                                        d=d
                                        from=MarchLightPoint { x: piece.start.x, y: piece.start.y }
                                        id=gradient_id
                                        to=MarchLightPoint { x: piece.end.x, y: piece.end.y }
                                    />
                                }
                            })}
                        </g>
                    }
                    .into_any()
                })
                .collect_view()}
            {model
                .stations
                .iter()
                .enumerate()
                .filter(|(_, station)| station.track != 0)
                .map(|(i, station)| {
                    let folded_class = if folded.contains(&i) { "wf-folded" } else { "" };
                    let track = station.track.max(0) as usize;
                    let y_top = layout
                        .row_y
                        .get(track)
                        .copied()
                        .unwrap_or_else(|| layout.row_y.first().copied().unwrap_or(0.0));
                    let x = lamp_x(i, layout.inset);
                    let d = format!("M{},{} V{}", fmt(x), fmt(y_top + 7.0), fmt(layout.cap_y - 10.0));
                    view! {
                        <path
                            class=format!("wf-foldable {folded_class}")
                            d=d
                            data-leader-station=i
                            data-testid="workflow-timeline-leader"
                            fill="none"
                            stroke="var(--color-workflow-trace)"
                            stroke-dasharray="1 2"
                            stroke-linecap="round"
                            stroke-width="1"
                        />
                    }
                })
                .collect_view()}
        </g>
    }
}

/// 灯（有带时，真源 `WorkflowTimelineLamps`，:256-289）：还是 DOM 的 span——状态类、
/// 光晕与搏动都在 CSS 里——只是绝对定位到自己那条轨道的行上，而不再跟着站头排。
#[component]
pub fn WorkflowTimelineLamps(
    stations: Vec<TimelineStation>,
    layout: TimelineLayout,
    folded: Vec<usize>,
    draft: bool,
) -> impl IntoView {
    view! {
        {stations
            .iter()
            .enumerate()
            .map(|(i, station)| {
                let folded_class = if folded.contains(&i) { "wf-folded" } else { "" };
                let track = station.track.max(0) as usize;
                let y_top = layout
                    .row_y
                    .get(track)
                    .copied()
                    .unwrap_or_else(|| layout.row_y.first().copied().unwrap_or(0.0));
                let left = lamp_x(i, layout.inset) - 5.0;
                let top = y_top - 5.0;
                let status_word = station.status.map(|s| s.as_str()).unwrap_or("pending");
                view! {
                    <span
                        aria-hidden="true"
                        class=format!(
                            "{} absolute wf-foldable {} {}",
                            station_lamp_class(station.status),
                            folded_class,
                            if draft { "wf-land" } else { "" },
                        )
                        data-lamp=status_word
                        data-lamp-track=station.track
                        style=format!("left: {left}px; top: {top}px;")
                    />
                }
            })
            .collect_view()}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_with_rows(rows: &[f64]) -> TimelineLayout {
        TimelineLayout {
            row_y: rows.to_vec(),
            cap_y: 60.0,
            pills_top: 90.0,
            inset: 0.0,
            banded: true,
        }
    }

    fn station(track: i64) -> TimelineStation {
        TimelineStation {
            id: "p".into(),
            naming: Default::default(),
            pills: Vec::new(),
            status: None,
            visited: false,
            rounds: 0,
            on_loop: false,
            track,
            fraction: None,
            unlisted: None,
            typing: false,
        }
    }

    #[test]
    fn plain_segment_is_a_straight_line_between_lamps() {
        // 真源 :182 —— 普通（非带间）段：from 灯 +8 → to 灯 −8，同一行。
        let layout = layout_with_rows(&[24.0]);
        let stations = vec![station(0), station(0)];
        let rails = vec![TimelineRail {
            from: 0,
            to: 1,
            ink: TimelineInk::Strong,
            kind: None,
        }];
        let pieces = timeline_rail_pieces(&[], &rails, &stations, &layout);
        let piece = pieces.iter().find(|p| p.key == "0>1").unwrap();
        // lamp_x(0, 0) = 17，lamp_x(1, 0) = 192（站距 192 − 24 + 17 + 1 = 193？不：MARK_X=17，
        // PITCH=192 → 灯 1 在 209 − 17 = 192…以实现为准锁住两端数值）。
        assert_eq!(piece.start.x, 25.0);
        assert_eq!(piece.start.y, 24.0);
        assert_eq!(piece.end.x, piece.start.x + (lamp_x(1, 0.0) - 8.0) - 25.0);
        assert_eq!(piece.end.y, 24.0);
    }

    #[test]
    fn twin_rails_are_not_drawn_on_the_card() {
        // 真源 :156-158 —— 双线段只说「并行」，卡上不画。
        let layout = layout_with_rows(&[24.0]);
        let stations = vec![station(0), station(0)];
        let rails = vec![TimelineRail {
            from: 0,
            to: 1,
            ink: TimelineInk::Strong,
            kind: Some(TimelineRailKind::Twin),
        }];
        let pieces = timeline_rail_pieces(&[], &rails, &stations, &layout);
        assert!(pieces.is_empty());
    }

    #[test]
    fn ink_palette_matches_truth_source() {
        // 真源 :29-33。
        assert_eq!(ink_stroke(TimelineInk::Faint), "var(--color-workflow-trace)");
        assert_eq!(ink_stroke(TimelineInk::March), "var(--color-workflow-trace-strong)");
        assert_eq!(ink_stroke(TimelineInk::Strong), "var(--color-workflow-trace-strong)");
    }

    #[test]
    fn piece_kinds_serialize_to_wire_words() {
        assert_eq!(RailPieceKind::Fork.as_str(), "fork");
        assert_eq!(RailPieceKind::Merge.as_str(), "merge");
        assert_eq!(RailPieceKind::Tail.as_str(), "tail");
        assert_eq!(RailPieceKind::Stub.as_str(), "stub");
    }
}
