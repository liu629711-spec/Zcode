//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/timeline-geometry.ts`（221 行）。
//!
//! 横向时间线的几何常量。与设计画布逐字相同：站宽 168、站距 24，
//! 灯落在站左缘 + 17（药丸头像列）。
//!
//! ★真源 :5-8 注释——单独成文件是因为边檐的纯几何要读它们，
//! 而它不该反过来依赖渲染组件。

use std::collections::HashMap;

use crate::components::timeline_bands::arc_lane_count;
use crate::components::timeline_model::{TimelineArc, TimelineBand};

/// 站宽（真源 :9）。
pub const STATION_WIDTH: f64 = 168.0;
/// 站距（真源 :10）。
pub const STATION_GAP: f64 = 24.0;
/// 站间距 = 站宽 + 站距（真源 :11）。
pub const STATION_PITCH: f64 = STATION_WIDTH + STATION_GAP;
/// 灯心距站左缘的距离：外边距 12 + 半径 5（真源 :12-13）。
pub const MARK_X: f64 = 17.0;

/// 轨道行高（真源 :16）。
pub const RAIL_ROW: f64 = 24.0;
/// 相邻两条弧道的高差（真源 :18）。
pub const ARC_LANE: f64 = 14.0;
/// ★真源 :19-23 —— 最低的弧道离轨道中线的高度。
///
/// 它必须装得下：圆角 8 + 落地的竖段（箭头 5.25 长，再留几像素的直线，
/// 箭头才像从上面落下来的）。之前是 16：圆角占掉 8 之后竖段只剩 −1px——
/// 最后一段反而向上走了一像素，`marker-end` 跟着朝上，箭头就「不见了」。
pub const ARC_BASE: f64 = 26.0;
/// 同一枚灯上相邻两个弧端（起飞或落地）的间距（真源 :26）。
pub const TERMINAL_PITCH: f64 = 10.0;
/// 药丸高（真源 :27）。
pub const PILL_HEIGHT: f64 = 32.0;
/// 药丸间距（真源 :28）。
pub const PILL_GAP: f64 = 6.0;

/// 站台行高（真源 :35）：只有带的时间线才有，24px 一行。
pub const PLATFORM_ROW: f64 = 24.0;
/// 站台上名字的左缘：与灯的外边距同值，于是灯、名字、药丸头像连成一条竖脊（真源 :37）。
pub const CAPTION_X: f64 = 12.0;
/// 分叉点离首站左缘的距离；分支轨道每高一层再左移 8（真源 :39）。
pub const FORK_BACK: f64 = 16.0;
/// 汇合点离汇合站左缘的距离（负值 = 在它左边）（真源 :41）。
pub const MERGE_BACK: f64 = 10.0;
/// 没有前驱时那截尾巴的长度（真源 :43）。
pub const TAIL: f64 = 4.0;
/// 没有汇合站时那截残段的长度（真源 :44）。
pub const STUB: f64 = 6.0;

/// `timelineWidth`（真源 :46-48）。
pub fn timeline_width(count: usize, inset: f64) -> f64 {
    if count == 0 {
        0.0
    } else {
        (count - 1) as f64 * STATION_PITCH + STATION_WIDTH + inset
    }
}

/// `stationX`（真源 :51-53）：第 `index` 站的左缘。
pub fn station_x(index: usize, inset: f64) -> f64 {
    index as f64 * STATION_PITCH + inset
}

/// `lampX`（真源 :56-58）：第 `index` 站的灯心。
pub fn lamp_x(index: usize, inset: f64) -> f64 {
    station_x(index, inset) + MARK_X
}

/// `bandForkX`（真源 :61-63）：一条带在主线上分叉的 x。
///
/// ★真源 :60 注释——带从站 0 起时没有左边的余地，落在 4（时间线整体右移 `inset`）。
pub fn band_fork_x(band_from: usize, inset: f64) -> f64 {
    if band_from == 0 {
        TAIL
    } else {
        station_x(band_from, inset) - FORK_BACK
    }
}

/// `bandMergeX`（真源 :66-70）：一条带在主线上汇合的 x。
///
/// ★真源 :65 注释——没有汇合站时落在末站槽尾之后 6px，
/// 那里仍是 strand 相遇的地方。
pub fn band_merge_x(band_to: usize, join: Option<usize>, inset: f64) -> f64 {
    match join {
        None => station_x(band_to, inset) + STATION_WIDTH + STUB,
        Some(join) => station_x(join, inset) - MERGE_BACK,
    }
}

/// `bandAt`（真源 :73-75）：第 `index` 站所在的带；带外 `None`。
///
/// ★真源 :72 注释——`timeline_bands.rs` 的同名函数只认下标带，
/// 这里认**模型的**带。
pub fn band_at(bands: &[TimelineBand], index: usize) -> Option<&TimelineBand> {
    bands.iter().find(|band| band.from <= index && index <= band.to)
}

/// `ArcEnds`（真源 :83-87）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcEnds {
    pub in_track: bool,
    pub from_band: bool,
    pub to_band: bool,
}

/// `arcEnds`（真源 :89-107）：一条弧的两端挂在哪。
///
/// ★真源 :77-82 注释——带内同一条轨道的弧两端都是灯（`in_track`）；
/// 其余的弧把带当一个节点：源在带里就从带的汇合点起飞（`from_band`），
/// 目标在带里就落在带的分叉点上（`to_band`）。带的自环两端也在同一条带里，
/// 但它的 `air` 是最上面那层空，落点又总在轨道 0 上，所以不会被认成 `in_track`。
pub fn arc_ends(
    arc_air: i64,
    arc_from: usize,
    arc_to: usize,
    bands: &[TimelineBand],
    station_tracks: &[i64],
) -> ArcEnds {
    let source = band_at(bands, arc_from);
    let target = band_at(bands, arc_to);
    let track_of = |index: usize| -> i64 { station_tracks.get(index).copied().unwrap_or(0) };
    let in_track = source.is_some()
        && source == target
        && track_of(arc_from) == arc_air
        && track_of(arc_to) == arc_air;
    ArcEnds {
        from_band: !in_track && source.is_some(),
        in_track,
        to_band: !in_track && target.is_some(),
    }
}

/// 弧端的槽位结果（真源 :126-168 的返回）。
#[derive(Debug, Clone, PartialEq)]
pub struct ArcTerminalOffsets {
    /// 与 `arcs` 一一对应的起飞偏移（相对灯心，px）。
    pub takeoff: Vec<f64>,
    /// 与 `arcs` 一一对应的落地偏移。
    pub landing: Vec<f64>,
}

/// 弧端在某一站上的身份（真源 :133-139）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Terminal {
    arc: usize,
    takeoff: bool,
    /// 远端在左 −1、在右 1。
    side: i64,
    lane: i64,
}

/// `arcTerminalOffsets`（真源 :126-168）。
///
/// ★真源 :109-125 注释——弧在灯上的两端——源站的**起飞**、目标站的**落地**——
/// 都是这一站的弧端。一站的弧端各占一个槽，槽距 `TERMINAL_PITCH`，以灯心为中；
/// 只有一个弧端的站仍用灯心，于是没有冲突的时间线逐像素不动。
///
/// 之前只有落地会错开，起飞一律在灯心：一站既有进来的弧又有出去的弧时，
/// 出去的竖段正好穿过进来的箭头，两条竖段叠成一条线、箭头卡在半腰。
///
/// 槽的次序，自左向右：先是远端在这一站**左边**的弧端，再是远端在**右边**的；
/// 每一侧里**车道最低的在最外面**。带的分叉点与汇合点不占槽：
/// 分叉点上只有落地、汇合点上只有起飞。
pub fn arc_terminal_offsets(
    arcs: &[TimelineArc],
    bands: &[TimelineBand],
    station_tracks: &[i64],
) -> ArcTerminalOffsets {
    let mut takeoff = vec![0.0; arcs.len()];
    let mut landing = vec![0.0; arcs.len()];
    let mut by_station: HashMap<usize, Vec<Terminal>> = HashMap::new();

    for (j, arc) in arcs.iter().enumerate() {
        let ends = arc_ends(arc.air, arc.from, arc.to, bands, station_tracks);
        if !ends.from_band {
            by_station.entry(arc.from).or_default().push(Terminal {
                arc: j,
                takeoff: true,
                side: if arc.to > arc.from { 1 } else { -1 },
                lane: arc.lane,
            });
        }
        if !ends.to_band {
            by_station.entry(arc.to).or_default().push(Terminal {
                arc: j,
                takeoff: false,
                side: if arc.from > arc.to { 1 } else { -1 },
                lane: arc.lane,
            });
        }
    }

    for list in by_station.values_mut() {
        // 同一站、同一层空里的弧两两相交（闭区间），车道必不相同；稳定排序让弧序兜底。
        list.sort_by(|left, right| {
            left.side.cmp(&right.side).then_with(|| {
                if left.side < 0 {
                    left.lane.cmp(&right.lane)
                } else {
                    right.lane.cmp(&left.lane)
                }
            })
        });
        let last = list.len() as f64 - 1.0;
        for (k, terminal) in list.iter().enumerate() {
            let offset = (k as f64 - last / 2.0) * TERMINAL_PITCH;
            if terminal.takeoff {
                takeoff[terminal.arc] = offset;
            } else {
                landing[terminal.arc] = offset;
            }
        }
    }
    ArcTerminalOffsets { takeoff, landing }
}

/// `TimelineLayout`（真源 :171-182）。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineLayout {
    /// 每条轨道的中线 y，下标 = 轨道号；`row_y[0]` 是主线（最下面那一行）。
    pub row_y: Vec<f64>,
    /// 站台行的中线；没有带时退化成主线行——那时站头还在轨道行上。
    pub cap_y: f64,
    /// 药丸列的顶。
    pub pills_top: f64,
    /// 整根轨道右移的量：有带从站 0 起时 12（分叉要有落脚的地方），否则 0。
    pub inset: f64,
    /// 画面上有带。
    pub banded: bool,
}

/// `timelineLayout`（真源 :190-220）。
///
/// ★真源 :184-189 注释——行的排布：自上而下 `t = R−1 … 0`，每条轨道先留自己的**空气**
/// （弧道），再一行 24px 的轨道；主线因此落在最下面、紧挨着站台行。空气按那层空里的弧道数算：
/// 没有弧时最上面一层留 6px 呼吸，其余为 0。没有带时 R = 1，整套式子逐像素退回从前的
/// 「弧道 + 轨道行」。
pub fn timeline_layout(arcs: &[TimelineArc], bands: &[TimelineBand]) -> TimelineLayout {
    let banded = !bands.is_empty();
    let tracks = if banded {
        bands
            .iter()
            .map(|band| band.tracks.len())
            .max()
            .unwrap_or(1)
            .max(2)
    } else {
        1
    };
    let mut row_y = vec![0.0; tracks];
    let mut y = 0.0f64;
    for t in (0..tracks).rev() {
        let lanes = arc_lane_count(
            &arcs
                .iter()
                .filter(|arc| arc.air == t as i64)
                .map(|arc| arc.lane)
                .collect::<Vec<i64>>(),
        ) as usize;
        // ★真源 :200 —— 最高一道的顶线落在 y = 8：
        // rowY = air + RAIL_ROW/2 = 8 + ARC_BASE + ARC_LANE × (lanes − 1)。
        let air = if lanes == 0 {
            if t == tracks - 1 { 6.0 } else { 0.0 }
        } else {
            8.0 + ARC_BASE - RAIL_ROW / 2.0 + ARC_LANE * (lanes as f64 - 1.0)
        };
        y += air;
        row_y[t] = y + RAIL_ROW / 2.0;
        y += RAIL_ROW;
    }
    let cap_y = if banded {
        y + PLATFORM_ROW / 2.0
    } else {
        row_y.first().copied().unwrap_or(0.0)
    };
    if banded {
        y += PLATFORM_ROW;
    }
    TimelineLayout {
        banded,
        cap_y,
        inset: if banded && bands.iter().any(|band| band.from == 0) {
            CAPTION_X
        } else {
            0.0
        },
        pills_top: y + 8.0,
        row_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::timeline_model::{TimelineInk, TimelineTrack};

    fn band(from: usize, to: usize, join: Option<usize>) -> TimelineBand {
        TimelineBand {
            from,
            to,
            pred: None,
            join,
            tracks: vec![TimelineTrack {
                stations: (from..=to).collect(),
                entry: TimelineInk::Faint,
                exit: TimelineInk::Faint,
            }],
        }
    }

    fn arc(from: usize, to: usize, air: i64, lane: i64) -> TimelineArc {
        TimelineArc {
            from,
            to,
            lane,
            ink: TimelineInk::Faint,
            air,
        }
    }

    // ── 常量 ──

    #[test]
    fn constants_match_the_design_canvas() {
        // ★真源 :6 —— 与设计画布逐字相同。
        assert_eq!(STATION_WIDTH, 168.0);
        assert_eq!(STATION_GAP, 24.0);
        assert_eq!(STATION_PITCH, 192.0);
        assert_eq!(MARK_X, 17.0, "外边距 12 + 半径 5");
        assert_eq!(RAIL_ROW, 24.0);
        assert_eq!(ARC_BASE, 26.0, "★不能退回 16：竖段会只剩 −1px");
        assert_eq!(TERMINAL_PITCH, 10.0);
        assert_eq!(CAPTION_X, 12.0, "与灯的外边距同值");
    }

    // ── 站位 ──

    #[test]
    fn station_positions() {
        assert_eq!(station_x(0, 0.0), 0.0);
        assert_eq!(station_x(1, 0.0), 192.0);
        assert_eq!(station_x(1, 12.0), 204.0, "inset 整体右移");
        assert_eq!(lamp_x(0, 0.0), 17.0);
        assert_eq!(lamp_x(2, 12.0), 384.0 + 12.0 + 17.0);
    }

    #[test]
    fn timeline_width_of_zero_is_zero() {
        // 真源 :47 —— 没有站就没有宽度。
        assert_eq!(timeline_width(0, 0.0), 0.0);
        assert_eq!(timeline_width(1, 0.0), 168.0);
        assert_eq!(timeline_width(3, 0.0), 384.0 + 168.0);
        assert_eq!(timeline_width(3, 12.0), 384.0 + 168.0 + 12.0);
    }

    // ── 带端点 ──

    #[test]
    fn fork_x_collapses_to_tail_at_station_zero() {
        // ★真源 :60 —— 带从站 0 起时没有左边的余地，落在 4。
        assert_eq!(band_fork_x(0, 0.0), 4.0);
        assert_eq!(band_fork_x(2, 0.0), 384.0 - 16.0);
        assert_eq!(band_fork_x(2, 12.0), 384.0 + 12.0 - 16.0);
    }

    #[test]
    fn merge_x_uses_join_or_a_stub_past_the_last_slot() {
        // 真源 :65-70 —— 没有汇合站时落在末站槽尾之后 6px。
        assert_eq!(band_merge_x(2, Some(3), 0.0), 576.0 - 10.0);
        assert_eq!(band_merge_x(2, None, 0.0), 384.0 + 168.0 + 6.0);
    }

    #[test]
    fn band_at_finds_the_containing_band() {
        let bands = vec![band(0, 1, None), band(3, 4, Some(5))];
        assert_eq!(band_at(&bands, 0).unwrap().from, 0);
        assert_eq!(band_at(&bands, 1).unwrap().to, 1);
        assert!(band_at(&bands, 2).is_none(), "带外");
        assert_eq!(band_at(&bands, 4).unwrap().from, 3);
        assert!(band_at(&bands, 9).is_none());
    }

    // ── 弧端 ──

    #[test]
    fn arc_inside_one_band_track_ends_on_lamps() {
        // ★真源 :77-79 —— 带内同一条轨道的弧两端都是灯。
        let bands = vec![band(0, 3, None)];
        let tracks = vec![0, 1, 1, 0];
        let ends = arc_ends(1, 1, 2, &bands, &tracks);
        assert!(ends.in_track);
        assert!(!ends.from_band);
        assert!(!ends.to_band);
    }

    #[test]
    fn arc_leaving_a_band_takes_off_from_the_band() {
        // 真源 :79-80 —— 源在带里就从带的汇合点起飞。
        let bands = vec![band(0, 1, None)];
        let tracks = vec![0, 0];
        let ends = arc_ends(1, 1, 5, &bands, &tracks);
        assert!(!ends.in_track);
        assert!(ends.from_band, "源在带里");
        assert!(!ends.to_band, "目标在带外");
    }

    #[test]
    fn arc_landing_in_a_band_lands_on_the_fork() {
        let bands = vec![band(4, 5, None)];
        let tracks = vec![0, 0, 0, 0, 0, 0];
        let ends = arc_ends(1, 0, 4, &bands, &tracks);
        assert!(!ends.in_track);
        assert!(!ends.from_band);
        assert!(ends.to_band, "目标在带里");
    }

    #[test]
    fn band_self_loop_is_not_in_track() {
        // ★真源 :80-82 —— 带的自环两端也在同一条带里，但它的 air 是最上面那层空，
        // 落点又总在轨道 0 上，所以不会被认成 inTrack。
        let bands = vec![band(0, 2, None)];
        let tracks = vec![0, 0, 0];
        let ends = arc_ends(5, 0, 2, &bands, &tracks);
        assert!(!ends.in_track);
        assert!(ends.from_band && ends.to_band);
    }

    // ── 弧端槽位 ──

    #[test]
    fn single_terminal_station_stays_on_the_lamp() {
        // ★真源 :113-114 —— 只有一个弧端的站仍用灯心，
        // 于是没有冲突的时间线逐像素不动。
        let offsets = arc_terminal_offsets(&[arc(0, 3, 0, 0)], &[], &[0; 4]);
        assert_eq!(offsets.takeoff, vec![0.0]);
        assert_eq!(offsets.landing, vec![0.0]);
    }

    #[test]
    fn two_terminals_split_around_the_lamp() {
        // 真源 :111-113 —— 槽距 TERMINAL_PITCH，以灯心为中。
        // 站 1 既有进来的弧（落地）又有出去的弧（起飞）。
        let arcs = vec![arc(0, 1, 0, 0), arc(1, 2, 0, 0)];
        let offsets = arc_terminal_offsets(&arcs, &[], &[0; 3]);
        // 站 1：起飞（远端在右，side=+1）与落地（远端在左，side=-1）。
        // 排序后 side=-1 在左、side=+1 在右 → 起飞拿到 +5、落地拿到 −5。
        assert_eq!(offsets.takeoff[1], 5.0);
        assert_eq!(offsets.landing[0], -5.0);
    }

    #[test]
    fn takeoff_also_offsets_when_a_station_has_both() {
        // ★真源 :114-117 —— 之前只有落地会错开，起飞一律在灯心：
        // 出去的竖段正好穿过进来的箭头。
        let arcs = vec![arc(0, 1, 0, 0), arc(1, 2, 0, 1)];
        let offsets = arc_terminal_offsets(&arcs, &[], &[0; 3]);
        assert_ne!(offsets.takeoff[1], 0.0, "★起飞也错开");
        assert_ne!(offsets.landing[0], 0.0);
    }

    #[test]
    fn band_terminals_do_not_claim_slots() {
        // ★真源 :120-122 —— 带的分叉点与汇合点不占槽：
        // 分叉点上只有落地、汇合点上只有起飞，同一个点上的竖段像 strand 一样共用一条线。
        let bands = vec![band(0, 1, None)];
        let tracks = vec![0, 0];
        // 这条弧从带里起飞（from_band）→ 起飞端不登记，站 4 上只有落地一个端。
        let arcs = vec![arc(1, 4, 0, 0)];
        let offsets = arc_terminal_offsets(&arcs, &bands, &tracks);
        assert_eq!(offsets.takeoff[0], 0.0, "起飞端挂在带点上，不占站 1 的槽");
        assert_eq!(
            offsets.landing[0], 0.0,
            "★站 4 只有一个弧端 → 仍用灯心，没有冲突的时间线逐像素不动"
        );
        // 对照：同一条弧去掉带（两端都在带外）→ 起飞端登记到站 1。
        let mut arcs_two_ends = arcs.clone();
        arcs_two_ends[0].from = 0;
        let plain = arc_terminal_offsets(&arcs_two_ends, &[], &tracks);
        assert_eq!(plain.takeoff[0], 0.0, "带外起飞在站 0，只有它一个端");
    }

    #[test]
    fn band_terminal_leaves_the_far_station_unpinned() {
        // ★真源 :121-122 的可观测后果：带的那一端不占槽，
        // 于是对侧站上若还挤着别的弧端，它不会被推歪。
        let bands = vec![band(0, 1, None)];
        let tracks = vec![0; 6];
        // 弧 A：从带里起飞、落在站 4。弧 B：从站 3 起飞、也落在站 4。
        let arcs = vec![arc(1, 4, 0, 0), arc(3, 4, 0, 1)];
        let offsets = arc_terminal_offsets(&arcs, &bands, &tracks);
        // 站 4 上有 A 的落地 + B 的落地，两个端 → 对称错开 ±5。
        assert_eq!(offsets.landing[0], -5.0);
        assert_eq!(offsets.landing[1], 5.0);
    }

    #[test]
    fn lower_lane_sits_outermost_on_each_side() {
        // ★真源 :118-120 —— 每一侧里**车道最低的在最外面**。
        // 两条弧都落在站 3、远端都在左边（side=−1）：lane 0 应比 lane 1 更靠外（更负）。
        let arcs = vec![arc(2, 3, 0, 1), arc(1, 3, 0, 0)];
        let tracks = vec![0; 6];
        let offsets = arc_terminal_offsets(&arcs, &[], &tracks);
        // arcs[0] 的 lane 是 1，arcs[1] 的 lane 是 0。
        assert_eq!(offsets.landing[1], -5.0, "lane 0 更靠外");
        assert_eq!(offsets.landing[0], 5.0, "lane 1 更靠里");
    }

    #[test]
    fn side_wins_over_lane_when_ordering_slots() {
        // ★真源 :117-118 —— 次序先看**哪一侧**，同一侧里才比车道。
        // 两条弧都落在站 3，但远端一左一右：左边的排前面，不看车道。
        let arcs = vec![arc(4, 3, 0, 0), arc(0, 3, 0, 9)];
        let tracks = vec![0; 6];
        let offsets = arc_terminal_offsets(&arcs, &[], &tracks);
        assert_eq!(
            offsets.landing[1], -5.0,
            "远端在左的那条（lane 9）仍排最前"
        );
        assert_eq!(offsets.landing[0], 5.0, "远端在右的那条排后");
    }

    #[test]
    fn lanes_on_opposite_sides_split_around_the_lamp() {
        // 真源 :117-118 —— 先是远端在**左边**的弧端，再是远端在**右边**的。
        let arcs = vec![arc(0, 3, 0, 0), arc(4, 3, 0, 0)];
        let tracks = vec![0; 6];
        let offsets = arc_terminal_offsets(&arcs, &[], &tracks);
        // arcs[0] 的远端在左 → side=−1，排在前面拿 −5。
        assert_eq!(offsets.landing[0], -5.0);
        assert_eq!(offsets.landing[1], 5.0);
    }

    // ── 行布局 ──

    #[test]
    fn layout_without_bands_is_one_rail_row() {
        // ★真源 :188-189 —— 没有带时 R = 1，整套式子逐像素退回从前的「弧道 + 轨道行」。
        let layout = timeline_layout(&[], &[]);
        assert!(!layout.banded);
        assert_eq!(layout.row_y.len(), 1);
        // 顶线留 6px 呼吸 + 半行。
        assert_eq!(layout.row_y[0], 6.0 + 12.0);
        assert_eq!(layout.cap_y, layout.row_y[0], "没有带时站台行退化成主线行");
        assert_eq!(layout.inset, 0.0);
        assert_eq!(layout.pills_top, 6.0 + 24.0 + 8.0);
    }

    #[test]
    fn layout_with_a_band_adds_a_platform_row_and_inset() {
        let layout = timeline_layout(&[], &[band(0, 1, None)]);
        assert!(layout.banded);
        assert_eq!(layout.inset, 12.0, "带从站 0 起 → 整体右移，分叉要有落脚的地方");
        // cap_y 在轨道行之下。
        assert!(layout.cap_y > layout.row_y[0]);
        assert!(layout.pills_top > layout.cap_y);
    }

    #[test]
    fn layout_keeps_inset_zero_when_no_band_starts_at_zero() {
        let layout = timeline_layout(&[], &[band(2, 3, Some(4))]);
        assert!(layout.banded);
        assert_eq!(layout.inset, 0.0, "带不从站 0 起 → 不需要给分叉留地方");
    }

    #[test]
    fn layout_reserves_arc_air_above_each_rail() {
        // 真源 :199-206 —— 空气按那层空里的弧道数算。
        let arcs = vec![arc(0, 2, 0, 0), arc(0, 2, 0, 1)];
        let layout = timeline_layout(&arcs, &[]);
        // 有弧：air = 8 + ARC_BASE − RAIL_ROW/2 + ARC_LANE × (lanes−1)
        //      = 8 + 26 − 12 + 14 × 1 = 36。
        assert_eq!(layout.row_y[0], 36.0 + 12.0);
    }

    #[test]
    fn layout_gives_two_tracks_minimum_when_banded() {
        // 真源 :195 —— Math.max(2, ...)：有带时至少两条轨道。
        let bands = vec![TimelineBand {
            from: 1,
            to: 1,
            pred: None,
            join: None,
            tracks: vec![TimelineTrack {
                stations: vec![1],
                entry: TimelineInk::Faint,
                exit: TimelineInk::Faint,
            }],
        }];
        let layout = timeline_layout(&[], &bands);
        assert_eq!(layout.row_y.len(), 2, "单成员带也占两条轨道");
    }
}