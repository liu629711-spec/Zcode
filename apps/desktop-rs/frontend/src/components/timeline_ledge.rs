//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/timeline-ledge.ts`（210 行）。
//!
//! 边檐的纯几何：一根轨道两端压缩。
//!
//! ★真源 :3-12 注释——时间线自由滚动；灯滚到视口边缘之外的站**折叠**到那一侧的边檐上
//! ——同一枚 10px 灯、16px 一枚、之间是同一种墨的小轨道段，落在视口边缘干净的底上。
//! 轨道行在檐旁渐隐 40px；药丸只在视口边界渐隐。折叠是不动点：边檐越宽（灯越多），
//! 压在它下面的站就越多，所以反复算到集合不再变化。
//!
//! 全部以视口坐标计（x = 站的灯心 − scrollLeft）。无 React、无 DOM。

use crate::components::timeline_geometry::{STATION_WIDTH, lamp_x, station_x};

/// 边檐上每枚灯的直径（真源 :13）。
pub const LEDGE_LAMP: f64 = 10.0;
/// 灯的间距（真源 :14）。
pub const LEDGE_PITCH: f64 = 16.0;
/// 边檐的内边距（真源 :15）。
pub const LEDGE_PAD: f64 = 8.0;
/// 轨道行在檐旁的渐隐宽（真源 :16）。
pub const LEDGE_FADE: f64 = 40.0;
/// 边檐最多露几枚灯（真源 :17）。
pub const LEDGE_MAX_LAMPS: usize = 3;
/// `+n` 计数占的宽（真源 :19）。
const LEDGE_MORE: f64 = 26.0;
/// 边檐到内容侧的短轨道段短于它就不画（画出来是一个点）（真源 :21）。
const STUB_MIN: f64 = 6.0;
/// 滚动条拇指的最短长度（真源 :23）。
const THUMB_MIN: f64 = 24.0;

/// `TimelineFold`（真源 :25-30）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineFold {
    /// 折叠到左檐的站（升序）。
    pub left: Vec<usize>,
    /// 折叠到右檐的站（升序）。
    pub right: Vec<usize>,
}

impl TimelineFold {
    /// `NO_FOLD`（真源 :32）。
    pub fn empty() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    pub fn is_empty_fold(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}

/// `ledgeWidth`（真源 :35-39）：一侧边檐占的宽（含两侧内边距；0 枚灯 = 没有边檐）。
pub fn ledge_width(count: usize) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let lamps = count.min(LEDGE_MAX_LAMPS) as f64 * LEDGE_PITCH - (LEDGE_PITCH - LEDGE_LAMP);
    LEDGE_PAD + lamps + if count > LEDGE_MAX_LAMPS { LEDGE_MORE } else { 0.0 } + LEDGE_PAD
}

/// `foldStations`（真源 :47-70）：折叠集。
///
/// 灯在「边檐 + 渐隐」之下的站；迭代到不动点。视口没量到（宽 0）时什么都不折。
///
/// ★真源 :44-45 —— `keep` 是镜头正飞向的站：视口在 `scroll_to` 之前量过，
/// 平滑滚动在飞的那几百毫秒里它按陈旧的 scrollLeft 算在视野外——可它正要进来，
/// 檐上闪一枚灯是误报，所以两侧都不收它。`inset` 是有带时整根轨道的右移：
/// 灯跟着走，折叠的判据也跟着走。
pub fn fold_stations(
    count: usize,
    scroll_left: f64,
    client_width: f64,
    keep: Option<usize>,
    inset: f64,
) -> TimelineFold {
    if count == 0 || client_width <= 0.0 {
        return TimelineFold::empty();
    }
    let xs: Vec<f64> = (0..count)
        .map(|i| lamp_x(i, inset) - scroll_left)
        .collect();
    let mut left: Vec<usize> = Vec::new();
    let mut right: Vec<usize> = Vec::new();
    for _ in 0..8 {
        let lw = if left.is_empty() {
            0.0
        } else {
            ledge_width(left.len()) + LEDGE_FADE
        };
        let rw = if right.is_empty() {
            0.0
        } else {
            ledge_width(right.len()) + LEDGE_FADE
        };
        // `flatMap((x, i) => (x < lw && i !== keep ? [i] : []))` —— 天然升序。
        let next_left: Vec<usize> = xs
            .iter()
            .enumerate()
            .filter(|(i, x)| **x < lw && keep != Some(*i))
            .map(|(i, _)| i)
            .collect();
        let next_right: Vec<usize> = xs
            .iter()
            .enumerate()
            .filter(|(i, x)| {
                **x > client_width - rw && keep != Some(*i) && !next_left.contains(i)
            })
            .map(|(i, _)| i)
            .collect();
        if next_left.len() == left.len() && next_right.len() == right.len() {
            break;
        }
        left = next_left;
        right = next_right;
    }
    if left.is_empty() && right.is_empty() {
        TimelineFold::empty()
    } else {
        TimelineFold {
            left,
            right,
        }
    }
}

/// 哪一侧（真源 :75 的 `"left" | "right"`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgeSide {
    Left,
    Right,
}

/// `ledgeLamps` 的结果（真源 :76）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgeLamps {
    /// 最多三枚，靠内容的一端优先。
    pub shown: Vec<usize>,
    pub more: usize,
}

/// `ledgeLamps`（真源 :73-85）：边檐上露出的灯与计数。
pub fn ledge_lamps(indexes: &[usize], side: LedgeSide) -> LedgeLamps {
    let more = indexes.len().saturating_sub(LEDGE_MAX_LAMPS);
    let shown = if more == 0 {
        indexes.to_vec()
    } else if side == LedgeSide::Left {
        // 左檐：靠内容的一端是**尾部**（站号大的那几枚）。
        indexes[more..].to_vec()
    } else {
        indexes[..LEDGE_MAX_LAMPS].to_vec()
    };
    LedgeLamps { shown, more }
}

/// `ledgeStubWidth`（真源 :91-110）：边檐到第一个开着的站之间的短轨道段的宽。
///
/// ★真源 :88-90 —— 左檐从檐的内缘到那站的灯前 6px；右檐从最后一个开着的站的**槽尾**
/// （不是灯——否则会横穿它的站头文字）到檐的内缘。短于 6 不画。
pub fn ledge_stub_width(
    fold: &TimelineFold,
    side: LedgeSide,
    scroll_left: f64,
    client_width: f64,
    inset: f64,
) -> f64 {
    let indexes = match side {
        LedgeSide::Left => &fold.left,
        LedgeSide::Right => &fold.right,
    };
    if indexes.is_empty() {
        return 0.0;
    }
    let inner = ledge_width(indexes.len()) - LEDGE_PAD;
    let width = match side {
        LedgeSide::Left => {
            let open = indexes[indexes.len() - 1] + 1;
            lamp_x(open, inset) - scroll_left - STUB_MIN - inner
        }
        LedgeSide::Right => {
            let open = indexes[0] - 1;
            client_width - inner - (station_x(open, inset) + STATION_WIDTH - scroll_left)
        }
    };
    if width < STUB_MIN {
        0.0
    } else {
        width.round()
    }
}

/// `EdgeOverflow`（真源 :113-116）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeOverflow {
    /// 左：滚过了。
    pub left: bool,
    /// 右：没滚到底。
    pub right: bool,
}

/// `timelineMaskStyle` 的结果（真源 :126-157）。
///
/// 两条 mask 层各占一条带（`no-repeat`，按位置 / 尺寸切开），默认 add 合成 = 并集。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineMaskStyle {
    pub image: String,
    pub position: String,
    pub repeat: String,
    pub size: String,
}

/// `timelineMaskStyle`（真源 :126-157）：滚动层的遮罩。
///
/// ★真源 :118-124 注释——分**两条横带**（用户修订：药丸只在真正的边界处渐隐，
/// 与自己那站的灯同进退）：
/// - 轨道带（弧的空气 + 轨道行，高 `rail_band`）：有檐的一侧檐下全透、再 40px 渐到不透
///   ——檐要落在干净的底上；没檐但溢出的一侧从视口边缘起 40px 渐隐（站头文字不硬切）。
/// - 药丸带（其余高度）：只在视口边缘 40px 渐隐，而且只在那一侧真有内容在外面时。
///   药丸不随灯折叠（两侧对称），檐下照亮，一直亮到边界。
///
/// 两侧都不溢出时**没有遮罩**（`None`）。
pub fn timeline_mask_style(
    fold: &TimelineFold,
    rail_band: f64,
    edges: EdgeOverflow,
) -> Option<TimelineMaskStyle> {
    if !edges.left && !edges.right {
        return None;
    }
    let edge_left = if edges.left {
        "transparent 0px, #000 40px".to_string()
    } else {
        "#000 0px".to_string()
    };
    let edge_right = if edges.right {
        "#000 calc(100% - 40px), transparent 100%".to_string()
    } else {
        "#000 100%".to_string()
    };
    let rail_left = if !fold.left.is_empty() {
        let inner = ledge_width(fold.left.len()) + LEDGE_PAD;
        format!("transparent {inner}px, #000 {}px", inner + LEDGE_FADE)
    } else {
        edge_left.clone()
    };
    let rail_right = if !fold.right.is_empty() {
        let inner = ledge_width(fold.right.len()) + LEDGE_PAD;
        format!(
            "#000 calc(100% - {}px), transparent calc(100% - {inner}px)",
            inner + LEDGE_FADE
        )
    } else {
        edge_right.clone()
    };
    Some(TimelineMaskStyle {
        image: format!(
            "linear-gradient(90deg, {rail_left}, {rail_right}), linear-gradient(90deg, {edge_left}, {edge_right})"
        ),
        position: format!("0 0, 0 {rail_band}px"),
        repeat: "no-repeat, no-repeat".to_string(),
        size: format!("100% {rail_band}px, 100% calc(100% - {rail_band}px)"),
    })
}

/// `scrollbarThumb` 的结果（真源 :160）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarThumb {
    pub left: f64,
    pub width: f64,
}

/// `scrollbarThumb`（真源 :160-170）：滚动条拇指。
///
/// 长 = 视口² / 内容（不短于 24），位置按滚动比例；不溢出时没有。
pub fn scrollbar_thumb(
    scroll_left: f64,
    client_width: f64,
    scroll_width: f64,
) -> Option<ScrollbarThumb> {
    if client_width <= 0.0 || scroll_width <= client_width {
        return None;
    }
    let width = THUMB_MIN.max(client_width * client_width / scroll_width);
    let range = scroll_width - client_width;
    let left = (client_width - width) * scroll_left.clamp(0.0, range) / range;
    Some(ScrollbarThumb {
        left: left.round(),
        width: width.round(),
    })
}

/// `CameraFlight`（真源 :176-181）：镜头的一班飞行。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFlight {
    pub index: usize,
    /// 已按 `scrollWidth − clientWidth` 夹紧的落点。
    pub target: f64,
    /// 上一次采到的 scrollLeft。
    pub from: f64,
}

/// `flightAfterScroll`（真源 :187-196）：一次 scroll 采样之后飞行还在不在。
///
/// ★真源 :183-185 —— 落地（±1px）或偏离（用户接手）即结束，否则记下这次位置继续飞。
/// 只由滚动位置决定，不用计时器——被用户中断的平滑滚动永远到不了目标，
/// 计时器兜底会让目标站错过折叠。
pub fn flight_after_scroll(
    flight: Option<CameraFlight>,
    scroll_left: f64,
) -> Option<CameraFlight> {
    let flight = flight?;
    let distance = (scroll_left - flight.target).abs();
    if distance <= 1.0 {
        return None;
    }
    if distance > (flight.from - flight.target).abs() + 1.0 {
        return None;
    }
    if scroll_left == flight.from {
        Some(flight)
    } else {
        Some(CameraFlight {
            from: scroll_left,
            ..flight
        })
    }
}

/// `stationCameraLeft`（真源 :199-201）：把一站滚到视口正中的 scrollLeft。
///
/// 左端不越 0。边檐上的灯点了就走这条。
pub fn station_camera_left(index: usize, client_width: f64, inset: f64) -> f64 {
    (station_x(index, inset) - (client_width - STATION_WIDTH) / 2.0).max(0.0)
}

/// `railKey`（真源 :207-209）：轨道段的查表键：**一对站**，不是起点。
///
/// ★真源 :203-206 —— 带里一站可以同时长出好几条段——主线的一条、分叉的一条、
/// 双线段的一条——按起点查会拿到先排到的那一条。
pub fn rail_key(from: usize, to: usize) -> String {
    format!("{from}>{to}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fold(left: &[usize], right: &[usize]) -> TimelineFold {
        TimelineFold {
            left: left.to_vec(),
            right: right.to_vec(),
        }
    }

    // ── 边檐宽 ──

    #[test]
    fn ledge_width_grows_with_lamp_count_then_saturates() {
        // 真源 :37-38 —— 超过三枚只多出 `+n` 那一段的宽，灯不再增加。
        assert_eq!(ledge_width(0), 0.0, "0 枚灯 = 没有边檐");
        let one = ledge_width(1);
        assert_eq!(one, LEDGE_PAD + LEDGE_LAMP + LEDGE_PAD);
        let two = ledge_width(2);
        assert!(two > one);
        let three = ledge_width(3);
        assert!(three > two);
        // 第四枚只加 LEDGE_MORE。
        assert!((ledge_width(4) - three - LEDGE_MORE).abs() < 1e-9);
        // 第五枚不再变宽。
        assert!((ledge_width(5) - ledge_width(4)).abs() < 1e-9);
    }

    // ── 折叠 ──

    #[test]
    fn no_fold_when_viewport_is_unmeasured() {
        // ★真源 :42 —— 视口没量到（宽 0）时什么都不折。
        assert!(fold_stations(10, 0.0, 0.0, None, 0.0).is_empty_fold());
        assert!(fold_stations(0, 0.0, 800.0, None, 0.0).is_empty_fold(), "没有站");
    }

    #[test]
    fn no_fold_when_everything_is_in_view() {
        // 五站（5×192 = 960）在 2000px 视口里全部可见。
        assert!(fold_stations(5, 0.0, 2000.0, None, 0.0).is_empty_fold());
    }

    #[test]
    fn stations_left_of_the_viewport_fold_left() {
        // 滚到第 4 站（scrollLeft = 4×192 = 768）。
        // 第一趟收进灯心在视口左边的 0..3；不动点再迭代时左檐变宽
        // （4 枚灯 → ledge_width 84 + 渐隐 40 = 124 > 站 4 的灯心 17），
        // 站 4 也被收进去 —— 这就是「折叠是不动点」的含义。
        let f = fold_stations(10, 768.0, 800.0, None, 0.0);
        assert_eq!(f.left, vec![0, 1, 2, 3, 4]);
        // 视口 800 宽，站 8 的灯心1249 也在外面 → 右檐同样有折叠。
        assert_eq!(f.right, vec![8, 9]);
    }

    #[test]
    fn stations_right_of_the_viewport_fold_right() {
        // 视口 800 宽、scrollLeft 0：站 5 的左缘 960 已经在外面。
        let f = fold_stations(10, 0.0, 800.0, None, 0.0);
        assert!(f.left.is_empty());
        // 第一趟收进站 5..9（灯心 > 800）；右檐变宽后站 4（灯心 785）也进来。
        assert_eq!(f.right, vec![4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn fold_is_a_fixed_point_growing_the_ledge() {
        // ★真源 :9 —— 边檐越宽（灯越多），压在它下面的站就越多，
        // 所以反复算到集合不再变化。
        // 左檐随灯数变宽 → 更多站落进「檐 + 渐隐」之下 → 集合单调增长到不动点。
        let f = fold_stations(20, 2000.0, 800.0, None, 0.0);
        // 单趟至少收进站 0（灯心 17 < 800）；不动点比单趟更宽。
        assert!(!f.left.is_empty());
        // 迭代到不动点：再算一次得到同一集合。
        let again = fold_stations(20, 2000.0, 800.0, None, 0.0);
        assert_eq!(f, again);
        // 集合是前缀（升序、不连续）。
        let mut expected = 0;
        for i in &f.left {
            assert_eq!(*i, expected, "左檐收的是连续前缀");
            expected += 1;
        }
    }

    #[test]
    fn a_station_cannot_fold_on_both_sides() {
        // 真源 :63 —— 右檐排除已在左檐里的站。
        let f = fold_stations(30, 1500.0, 300.0, None, 0.0);
        for i in &f.right {
            assert!(!f.left.contains(i), "站 {i} 不能两侧都收");
        }
    }

    #[test]
    fn keep_is_excluded_from_both_sides() {
        // ★真源 :44-45 —— keep 是镜头正飞向的站：檐上闪一枚灯是误报，
        // 所以两侧都不收它。
        let f = fold_stations(20, 2000.0, 800.0, Some(3), 0.0);
        assert!(!f.left.contains(&3), "镜头正飞向的站不折到左檐");
        assert!(!f.right.contains(&3));
        // ★真源 :61 —— 判定是逐站的 `i !== keep`，所以集合里留一个洞，邻站照折。
        assert!(f.left.contains(&2) && f.left.contains(&4), "邻站照折");
    }

    #[test]
    fn inset_moves_the_lamps_with_the_rail() {
        // 真源 :45 —— 有带时整根轨道右移，灯跟着走，折叠的判据也跟着走。
        // ★视口 910 宽：站 4 的灯心 785（inset 0）/ 797（inset 12）分别落在
        // 阈值 910 − 124 = 786 的两侧 —— 灯**右移**让它更靠视口右缘，
        // 于是它**进**折叠区。折叠的判据跟着灯走，方向就是灯动的方向。
        let width = 910.0;
        assert!(!fold_stations(10, 0.0, width, None, 0.0).right.contains(&4));
        assert!(
            fold_stations(10, 0.0, width, None, 12.0).right.contains(&4),
            "灯跟着轨道右移 → 折叠的判据也跟着走，4 号进右檐"
        );
    }

    // ── 露灯 ──

    #[test]
    fn ledge_lamps_show_the_content_side_first() {
        // ★真源 :73 —— 边檐上露出的灯（最多三枚，**靠内容的一端优先**）。
        let indexes = [0, 1, 2, 3, 4, 5];
        // 左檐：靠内容的一端是尾部。
        let left = ledge_lamps(&indexes, LedgeSide::Left);
        assert_eq!(left.more, 3);
        assert_eq!(left.shown, vec![3, 4, 5]);
        // 右檐：靠内容的一端是头部。
        let right = ledge_lamps(&indexes, LedgeSide::Right);
        assert_eq!(right.more, 3);
        assert_eq!(right.shown, vec![0, 1, 2]);
    }

    #[test]
    fn ledge_lamps_show_all_when_within_the_cap() {
        let l = ledge_lamps(&[4, 5], LedgeSide::Left);
        assert_eq!(l.more, 0);
        assert_eq!(l.shown, vec![4, 5], "不到三枚就全露");
    }

    // ── 短轨道段 ──

    #[test]
    fn stub_width_is_zero_without_a_fold() {
        assert_eq!(ledge_stub_width(&fold(&[], &[]), LedgeSide::Left, 0.0, 800.0, 0.0), 0.0);
        assert_eq!(ledge_stub_width(&fold(&[], &[]), LedgeSide::Right, 0.0, 800.0, 0.0), 0.0);
    }

    #[test]
    fn stub_is_dropped_when_shorter_than_six() {
        // ★真源 :90 —— 短于 6 不画（画出来是一个点）。
        let f = fold(&[0], &[]);
        // 左檐一盏：inner = ledge_width(1) − PAD = (8 + 10 + 8) − 8 = 18。
        // 站 1 的灯心 192 + 17 = 209；width = 209 − 0 − 6 − 18 = 185 → 画。
        assert_eq!(ledge_stub_width(&f, LedgeSide::Left, 0.0, 800.0, 0.0), 185.0);
        // 把视口拉到站 1 灯心之前 → 宽度不足 6 → 不画。
        let narrow = ledge_stub_width(&f, LedgeSide::Left, 190.0, 200.0, 0.0);
        assert_eq!(narrow, 0.0);
    }

    #[test]
    fn right_stub_ends_at_the_slot_tail_not_the_lamp() {
        // ★真源 :89-90 —— 从最后一个开着的站的**槽尾**（不是灯，
        // 否则会横穿它的站头文字）到檐的内缘。
        // 站 0 折进右檐不可能（它在最左），这里验站 9 折右檐时的算法形状。
        let f = fold(&[], &[9]);
        let width = ledge_stub_width(&f, LedgeSide::Right, 0.0, 4000.0, 0.0);
        // open = 8；inner = 18；width = 4000 − 18 − (8×192 + 168) = 2278。
        assert_eq!(width, 2278.0);
    }

    // ── 遮罩 ──

    #[test]
    fn no_mask_when_nothing_overflows() {
        // ★真源 :131 —— 两侧都不溢出时没有遮罩。
        let edges = EdgeOverflow {
            left: false,
            right: false,
        };
        assert!(timeline_mask_style(&fold(&[], &[]), 40.0, edges).is_none());
    }

    #[test]
    fn mask_splits_into_two_bands() {
        // ★真源 :124-125 —— 两层 mask 各占一条带，no-repeat，按位置 / 尺寸切开，
        // 默认 add 合成 = 并集。
        let style = timeline_mask_style(
            &fold(&[], &[]),
            40.0,
            EdgeOverflow {
                left: true,
                right: false,
            },
        )
        .unwrap();
        assert_eq!(style.position, "0 0, 0 40px");
        assert_eq!(style.size, "100% 40px, 100% calc(100% - 40px)");
        assert_eq!(style.repeat, "no-repeat, no-repeat");
        // 两侧渐隐：左渐入、右恒不透。
        assert!(style.image.contains("linear-gradient(90deg, transparent 0px, #000 40px, #000 100%"));
    }

    #[test]
    fn rail_band_keeps_the_ledge_fully_transparent() {
        // ★真源 :121 —— 有檐的一侧檐下全透、再 40px 渐到不透：
        // 檐要落在干净的底上。
        let f = fold(&[0, 1], &[]);
        let style = timeline_mask_style(&f, 40.0, EdgeOverflow { left: true, right: false }).unwrap();
        // inner = ledge_width(2) + PAD
        let inner = ledge_width(2) + LEDGE_PAD;
        assert!(
            style.image.starts_with(&format!(
                "linear-gradient(90deg, transparent {inner}px, #000 {}px",
                inner + LEDGE_FADE
            )),
            "轨道带在檐下全透"
        );
        // 药丸带不受檐影响：仍然从视口边缘 40px 渐入。
        assert!(style.image.contains("linear-gradient(90deg, transparent 0px, #000 40px"));
    }

    #[test]
    fn mask_without_a_ledge_still_fades_from_the_viewport_edge() {
        // ★真源 :122 —— 没檐但溢出的一侧从视口边缘起 40px 渐隐
        //（站头文字不硬切）。两层此时同形。
        let style = timeline_mask_style(
            &fold(&[], &[]),
            40.0,
            EdgeOverflow {
                left: false,
                right: true,
            },
        )
        .unwrap();
        assert!(style.image.starts_with("linear-gradient(90deg, #000 0px, #000 calc(100% - 40px)"));
    }

    // ── 滚动条 ──

    #[test]
    fn thumb_absent_without_overflow() {
        // 真源 :165 —— 不溢出时没有。
        assert!(scrollbar_thumb(0.0, 800.0, 800.0).is_none());
        assert!(scrollbar_thumb(0.0, 800.0, 700.0).is_none(), "内容比视口还窄");
        assert!(scrollbar_thumb(0.0, 0.0, 2000.0).is_none(), "视口没量到");
    }

    #[test]
    fn thumb_length_is_viewport_squared_over_content() {
        let t = scrollbar_thumb(0.0, 800.0, 3200.0).unwrap();
        assert_eq!(t.width, (800.0_f64 * 800.0 / 3200.0).round());
        assert_eq!(t.left, 0.0, "在起点");
    }

    #[test]
    fn thumb_never_shorter_than_twenty_four() {
        let t = scrollbar_thumb(0.0, 100.0, 100000.0).unwrap();
        assert_eq!(t.width, 24.0, "再长也保底 24");
    }

    #[test]
    fn thumb_position_follows_scroll_ratio() {
        // 滚到底：left = 视口 − 拇指。
        let width: f64 = 200.0;
        let t = scrollbar_thumb(2400.0, 800.0, 3200.0).unwrap();
        assert_eq!(t.width, width.round());
        assert_eq!(t.left, (800.0 - width).round());
        // 超出范围的 scrollLeft 被夹紧。
        let over = scrollbar_thumb(99999.0, 800.0, 3200.0).unwrap();
        assert_eq!(over.left, t.left, "夹紧后与滚到底同位");
    }

    // ── 镜头 ──

    #[test]
    fn camera_centers_a_station_without_crossing_zero() {
        // 真源 :199-201 —— 左端不越 0。
        assert_eq!(station_camera_left(0, 800.0, 0.0), 0.0);
        // 站 5：192×5 = 960 − (800−168)/2 = 960 − 316 = 644。
        assert_eq!(station_camera_left(5, 800.0, 0.0), 644.0);
    }

    #[test]
    fn flight_ends_on_landing() {
        // ★真源 :184-185 —— 落地（±1px）即结束。
        let flight = CameraFlight {
            index: 5,
            target: 644.0,
            from: 0.0,
        };
        assert!(flight_after_scroll(Some(flight), 644.0).is_none());
        assert!(flight_after_scroll(Some(flight), 643.0).is_none(), "±1px 内算落地");
        assert!(flight_after_scroll(None, 0.0).is_none(), "本来就没在飞");
    }

    #[test]
    fn flight_ends_when_the_user_takes_over() {
        // ★真源 :184 —— 偏离（比上一次更远）即结束：被用户中断的平滑滚动
        // 永远到不了目标。
        let flight = CameraFlight {
            index: 5,
            target: 644.0,
            from: 0.0,
        };
        // 第一次采样 300：还在飞，记下新位置。
        let next = flight_after_scroll(Some(flight), 300.0).unwrap();
        assert_eq!(next.from, 300.0);
        // 反向跑到 100：比上一次更远 → 用户接手，飞行结束。
        assert!(flight_after_scroll(Some(next), 100.0).is_none());
    }

    #[test]
    fn flight_records_an_unchanged_position_as_is() {
        let flight = CameraFlight {
            index: 5,
            target: 644.0,
            from: 300.0,
        };
        assert_eq!(flight_after_scroll(Some(flight), 300.0), Some(flight));
    }

    // ── 查表键 ──

    #[test]
    fn rail_key_is_a_pair_not_a_start() {
        // ★真源 :203-206 —— 带里一站可以同时长出好几条段：
        // 主线的一条、分叉的一条、双线段的一条——按起点查会拿到先排到的那一条。
        assert_eq!(rail_key(3, 4), "3>4");
        assert_ne!(rail_key(3, 4), rail_key(3, 5), "同一站的不同段是不同的键");
    }

    #[test]
    fn ledge_constants_match_the_design() {
        assert_eq!(LEDGE_LAMP, 10.0);
        assert_eq!(LEDGE_PITCH, 16.0);
        assert_eq!(LEDGE_PAD, 8.0);
        assert_eq!(LEDGE_FADE, 40.0);
        assert_eq!(LEDGE_MAX_LAMPS, 3);
    }
}
