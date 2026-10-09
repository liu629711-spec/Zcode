//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/timeline-bands.ts`（292 行）。
//!
//! 带与轨道。
//!
//! ★真源 :1-11 注释——分析器报的 `alongside` 是**节点事实**：进入这一阶段时，
//! 还没 join 的其他阶段的 strand 仍在跑。这里把它折成画面上的**带**——
//! 声明序上连续的一段阶段，内部拆成若干**轨道**，轨道 0 是主线（最下面那一行），
//! 分支轨道叠在它上面。带之前分叉、之后汇合；弧把整条带当**一个节点**。
//!
//! 只有下标，没有 id、没有墨迹、没有像素：时间线模型与侧栏的迷你运行线
//! 共用同一套折叠，后者只有 `phaseNames` 的下标空间，没有 display 图。

/// 一条带：闭区间 `[from, to]` 上的所有站；`tracks[0]` 是主线。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseBand {
    pub from: i64,
    pub to: i64,
    pub tracks: Vec<Vec<i64>>,
}

/// 轨道段的种类；缺席 = 一条轨道上的普通段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineRailKind {
    Fork,
    Merge,
    Twin,
}

/// `RailSpec`（真源 :96-100）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailSpec {
    pub from: i64,
    pub to: i64,
    pub kind: Option<TimelineRailKind>,
}

/// `ArcSpec`（真源 :102-108）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcSpec {
    pub from: i64,
    pub to: i64,
    /// 画在哪条轨道的空中：带内同轨道的弧用那条轨道的空，其余一律用最上面那条的空。
    pub air: i64,
}

/// `BoundBand`（真源 :110-113）：带 + 它在画面上的两个端点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundBand {
    pub from: i64,
    pub to: i64,
    pub tracks: Vec<Vec<i64>>,
    /// 分叉所在的前驱站。
    pub pred: Option<i64>,
    /// 汇合所在的后继站。
    pub join: Option<i64>,
}

/// `PhaseEdgeFold`（真源 :115-119）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseEdgeFold {
    pub bands: Vec<BoundBand>,
    pub rails: Vec<RailSpec>,
    pub arcs: Vec<ArcSpec>,
}

// ---------------------------------------------------------------------------
// 折带（真源 :13-80）
// ---------------------------------------------------------------------------

/// `foldPhaseBands`（真源 :29-80）。
///
/// ★三条算法规则（真源 :16-28 注释）：
/// 1. **对称化**：`alongside` 只有后来者才报得出（进入 B 时 A 还在跑，
///    A 的载荷里没有 B），画面上两站是对等的。
/// 2. `~` 的连通分量里成员 ≥ 2 的给出区间 `[min, max]`；相交的区间并起来；
///    区间内的空档也算成员（鲁棒起见——没有循环时不会出现）。
/// 3. 轨道是区间图的**贪心着色**，按**声明序**：每个成员落到「道上没有
///    与它并行的成员」的最低一道，否则另开一道。第一个成员因此总在轨道 0 上。
pub fn fold_phase_bands(count: i64, alongside: &[Vec<i64>]) -> Vec<PhaseBand> {
    let n = count.max(0) as usize;
    // 1) 对称化
    let mut near: Vec<std::collections::HashSet<i64>> = vec![std::collections::HashSet::new(); n];
    for i in 0..n {
        let Some(list) = alongside.get(i) else {
            continue;
        };
        for &j in list {
            // 越界、自指与未列出的都当没说。
            if j < 0 || j >= n as i64 || j == i as i64 {
                continue;
            }
            near[i].insert(j);
            near[j as usize].insert(i as i64);
        }
    }

    // 2) 连通分量 → 区间，相交合并
    let mut seen = vec![false; n];
    let mut spans: Vec<(i64, i64)> = Vec::new();
    for root in 0..n {
        if seen[root] || near[root].is_empty() {
            continue;
        }
        let mut from = root as i64;
        let mut to = root as i64;
        let mut stack = vec![root];
        seen[root] = true;
        while let Some(i) = stack.pop() {
            if (i as i64) < from {
                from = i as i64;
            }
            if (i as i64) > to {
                to = i as i64;
            }
            for &j in &near[i] {
                let ju = j as usize;
                if seen[ju] {
                    continue;
                }
                seen[ju] = true;
                stack.push(ju);
            }
        }
        spans.push((from, to));
    }
    spans.sort_by_key(|(f, _)| *f);
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (from, to) in spans {
        if let Some(last) = merged.last_mut() {
            if from <= last.1 {
                last.1 = last.1.max(to);
                continue;
            }
        }
        merged.push((from, to));
    }

    // 3) 贪心着色
    merged
        .into_iter()
        .map(|(from, to)| {
            let mut tracks: Vec<Vec<i64>> = Vec::new();
            for i in from..=to {
                let iu = i as usize;
                let free = tracks.iter_mut().find(|members| {
                    !members.iter().any(|m| near[iu].contains(m))
                });
                match free {
                    Some(track) => track.push(i),
                    None => tracks.push(vec![i]),
                }
            }
            PhaseBand { from, to, tracks }
        })
        .collect()
}

/// `bandOf`（真源 :83-85）：第 i 站所在的带；带外 `None`。
pub fn band_of(bands: &[PhaseBand], i: i64) -> Option<&PhaseBand> {
    bands.iter().find(|b| b.from <= i && i <= b.to)
}

/// `trackOf`（真源 :88-92）：第 i 站所在的轨道；带外一律 0（主线）。
pub fn track_of(bands: &[PhaseBand], i: i64) -> i64 {
    match band_of(bands, i) {
        Some(band) => band
            .tracks
            .iter()
            .position(|members| members.contains(&i))
            .map(|p| p as i64)
            .unwrap_or(0),
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// 带端点绑定（真源 :121-139）
// ---------------------------------------------------------------------------

/// `bindBands`（真源 :121-139）。
///
/// 带的前驱 / 汇合：**紧邻的那一站在带外**且与带里任一成员有边。
/// ★真源 :120 注释——相邻两带之间不认，那里是一段平轨。
fn bind_bands(count: i64, bands: &[PhaseBand], edges: &[(i64, i64)]) -> Vec<BoundBand> {
    let outside = |i: i64, bands: &[PhaseBand]| {
        i >= 0 && i < count && band_of(bands, i).is_none()
    };
    bands
        .iter()
        .map(|band| {
            let before = band.from - 1;
            let after = band.to + 1;
            let pred = outside(before, bands)
                && edges
                    .iter()
                    .any(|e| e.0 == before && band_of(bands, e.1).is_some_and(|b| b.from == band.from && b.to == band.to));
            let join = outside(after, bands)
                && edges
                    .iter()
                    .any(|e| e.1 == after && band_of(bands, e.0).is_some_and(|b| b.from == band.from && b.to == band.to));
            BoundBand {
                from: band.from,
                to: band.to,
                tracks: band.tracks.clone(),
                pred: pred.then_some(before),
                join: join.then_some(after),
            }
        })
        .collect()
}

/// `bandRails`（真源 :142-171）：带内自己长出来的轨道段。
///
/// 同轨道的相邻成员（**无条件**，一条轨道就是一条 strand）、分叉、汇合、双线段。
fn band_rails(bands: &[BoundBand]) -> Vec<RailSpec> {
    let mut rails: Vec<RailSpec> = Vec::new();
    for band in bands {
        for (track, members) in band.tracks.iter().enumerate() {
            for k in 1..members.len() {
                rails.push(RailSpec {
                    from: members[k - 1],
                    to: members[k],
                    kind: None,
                });
            }
            if let Some(pred) = band.pred {
                rails.push(RailSpec {
                    from: pred,
                    // 非主线的分叉/汇合才带 kind。
                    kind: (track != 0).then_some(TimelineRailKind::Fork),
                    to: members[0],
                });
            }
            if let Some(join) = band.join {
                rails.push(RailSpec {
                    from: members[members.len() - 1],
                    kind: (track != 0).then_some(TimelineRailKind::Merge),
                    to: join,
                });
            }
        }
        // ★双线段（真源 :168-171）：带内声明序相邻、却不在同一轨道的两站。
        // 只有台架与侧栏读它，永不行进。
        let plain: Vec<PhaseBand> = bands
            .iter()
            .map(|b| PhaseBand {
                from: b.from,
                to: b.to,
                tracks: b.tracks.clone(),
            })
            .collect();
        for i in band.from..band.to {
            if track_of(&plain, i) != track_of(&plain, i + 1) {
                rails.push(RailSpec {
                    from: i,
                    kind: Some(TimelineRailKind::Twin),
                    to: i + 1,
                });
            }
        }
    }
    rails
}

// ---------------------------------------------------------------------------
// 边折叠（真源 :173-233）
// ---------------------------------------------------------------------------

/// `foldPhaseEdges`（真源 :180-233）。
///
/// ★真源 :173-178 注释——带把边吃掉一部分：分叉与汇合已经把
/// 「控制经过了这里」说清楚了，剩下的才成弧，且弧的端点被**重挂**到带的两端
/// ——带是一个节点。
pub fn fold_phase_edges(
    count: i64,
    folded: &[PhaseBand],
    edges: &[(i64, i64)],
) -> PhaseEdgeFold {
    let bands = bind_bands(count, folded, edges);
    let max_tracks = bands.iter().map(|b| b.tracks.len()).max().unwrap_or(1) as i64;
    let top = max_tracks.max(1) - 1;
    let mut rails = band_rails(&bands);
    let mut arcs: Vec<ArcSpec> = Vec::new();

    for &(from, to) in edges {
        let source = band_of_plain(&bands, from);
        let target = band_of_plain(&bands, to);

        // 两端都在带外
        if source.is_none() && target.is_none() {
            if to == from + 1 {
                rails.push(RailSpec {
                    from,
                    to,
                    kind: None,
                });
            } else {
                arcs.push(ArcSpec { air: top, from, to });
            }
            continue;
        }

        // 两端在同一带内
        if let (Some(s), Some(t)) = (source, target) {
            if s.from == t.from && s.to == t.to {
                let track = track_of_bound(&bands, from);
                if track != track_of_bound(&bands, to) {
                    // ★真源 :202-205 —— 跨轨道：向前的边分叉已经说过了；
                    // 向后的是整条带的自环。
                    if to < from {
                        arcs.push(ArcSpec {
                            air: top,
                            from: s.to,
                            to: s.from,
                        });
                    }
                    continue;
                }
                let members = &s.tracks[track as usize];
                let idx = members.iter().position(|m| *m == from).unwrap_or(0);
                // 同轨道且紧挨着：strand 本来就在那儿，不必再画一遍。
                if members.get(idx + 1) != Some(&to) {
                    arcs.push(ArcSpec { air: track, from, to });
                }
                continue;
            }
        }

        // 出带 / 入带
        if let Some(s) = source {
            if target.is_none() && to == s.to + 1 {
                continue;
            }
        }
        if target.is_some() && source.is_none() && from == target.unwrap().from - 1 {
            continue;
        }
        let arc_from = source.map(|s| s.to).unwrap_or(from);
        let arc_to = target.map(|t| t.from).unwrap_or(to);
        if let (Some(s), Some(t)) = (source, target) {
            if s.to + 1 == t.from {
                rails.push(RailSpec {
                    from: arc_from,
                    to: arc_to,
                    kind: None,
                });
                continue;
            }
        }
        arcs.push(ArcSpec {
            air: top,
            from: arc_from,
            to: arc_to,
        });
    }

    PhaseEdgeFold {
        arcs: dedupe_arcs(&arcs),
        rails: order_rails(rails),
        bands,
    }
}

/// 带查找（`BoundBand` 版本）。
fn band_of_plain(bands: &[BoundBand], i: i64) -> Option<&BoundBand> {
    bands.iter().find(|b| b.from <= i && i <= b.to)
}

/// 轨道查找（`BoundBand` 版本）。
fn track_of_bound(bands: &[BoundBand], i: i64) -> i64 {
    match band_of_plain(bands, i) {
        Some(band) => band
            .tracks
            .iter()
            .position(|m| m.contains(&i))
            .map(|p| p as i64)
            .unwrap_or(0),
        None => 0,
    }
}

/// `dedupe`（真源 :235-241）：去掉 `from == to` 的自环与重复项。
fn dedupe_arcs(arcs: &[ArcSpec]) -> Vec<ArcSpec> {
    let mut seen = std::collections::HashSet::new();
    arcs
        .iter()
        .filter(|arc| {
            if arc.from == arc.to || seen.contains(&format!("{}:{}>{}", arc.air, arc.from, arc.to))
            {
                return false;
            }
            seen.insert(format!("{}:{}>{}", arc.air, arc.from, arc.to));
            true
        })
        .copied()
        .collect()
}

/// `order`（真源 :243-249）：轨道段按左端、再按右端排。
///
/// ★真源注释——没有带时与从前逐站推出来的顺序逐条相同。
fn order_rails(rails: Vec<RailSpec>) -> Vec<RailSpec> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped: Vec<RailSpec> = rails
        .into_iter()
        .filter(|rail| {
            let key = format!(
                "{}>{}:{}",
                rail.from,
                rail.to,
                match rail.kind {
                    None => String::new(),
                    Some(TimelineRailKind::Fork) => "fork".into(),
                    Some(TimelineRailKind::Merge) => "merge".into(),
                    Some(TimelineRailKind::Twin) => "twin".into(),
                }
            );
            if rail.from == rail.to || seen.contains(&key) {
                return false;
            }
            seen.insert(key);
            true
        })
        .collect();
    deduped.sort_by_key(|r| (r.from, r.to));
    deduped
}

// ---------------------------------------------------------------------------
// 弧分道（真源 :251-292）
// ---------------------------------------------------------------------------

/// `arcLaneCount`（真源 :251-253）：弧道的数量（最高的车道 + 1）。
pub fn arc_lane_count(lanes: &[i64]) -> i64 {
    lanes.iter().copied().fold(0, |max, l| max.max(l + 1))
}

/// `assignArcLanes`（真源 :255-278）。
///
/// ★真源 :255-265 注释——区间图的贪心着色。弧按**跨度从短到长**
/// （同跨度保持载荷序）依次落到**最低的空道**上；「空」= 该道上已有的弧
/// 与它在站的索引上**不相交**（闭区间：连共用一个端点的也算相交，
/// 因为两条弧在同一站的竖段会连成一线）。
/// 于是：互不相干的弧同一高度；嵌套的弧里面矮外面高；
/// 只有真正交叉的弧才被推上去。
///
/// ★真源 :267-270 注释——**之前的写法是「第 k 条弧就是第 k 道」**：
/// 两条各在一头、彼此无关的回边也一高一矮，读者会去找那个并不存在的理由。
pub fn assign_arc_lanes(pairs: &[(i64, i64)]) -> Vec<(i64, i64, i64)> {
    let mut placed: Vec<(i64, i64, i64)> = Vec::new(); // (lo, hi, lane)
    let mut out: Vec<(i64, i64, i64)> = Vec::new();

    // 按跨度从短到长排序，同跨度保持载荷序（稳定排序）。
    let mut indexed: Vec<(usize, (i64, i64), i64)> = pairs
        .iter()
        .enumerate()
        .map(|(i, &(f, t))| {
            let span = (t - f).abs();
            (i, (f, t), span)
        })
        .collect();
    indexed.sort_by_key(|(_, _, span)| *span); //稳定排序保载荷序

    let mut lanes = vec![0i64; pairs.len()];
    for (i, (f, t), _) in indexed {
        let lo = f.min(t);
        let hi = f.max(t);
        let taken: std::collections::HashSet<i64> = placed
            .iter()
            // 闭区间相交判定：共用端点也算相交。
            .filter(|(olo, ohi, _)| lo.max(*olo) <= hi.min(*ohi))
            .map(|(_, _, lane)| *lane)
            .collect();
        let mut lane = 0;
        while taken.contains(&lane) {
            lane += 1;
        }
        placed.push((lo, hi, lane));
        lanes[i] = lane;
    }

    for (i, &(f, t)) in pairs.iter().enumerate() {
        out.push((f, t, lanes[i]));
    }
    out
}

/// `assignAirLanes`（真源 :280-292）。
///
/// ★真源 :280-284 注释——按 `air` 分别着色：一条轨道的空里只有那条轨道的弧，
/// 跨轨道的弧一律在最上面那层空里，两层空里的弧在 x 上相交也互不相让。
/// 返回与入参一一对应的车道号。
pub fn assign_air_lanes(arcs: &[ArcSpec]) -> Vec<i64> {
    let mut lanes = vec![0i64; arcs.len()];
    let mut airs: Vec<i64> = arcs.iter().map(|a| a.air).collect();
    airs.sort_unstable();
    airs.dedup();
    for air in airs {
        let group: Vec<(usize, ArcSpec)> = arcs
            .iter()
            .enumerate()
            .filter(|(_, a)| a.air == air)
            .map(|(at, a)| (at, *a))
            .collect();
        let pairs: Vec<(i64, i64)> = group.iter().map(|(_, a)| (a.from, a.to)).collect();
        let placed = assign_arc_lanes(&pairs);
        for (k, (_, _, lane)) in placed.iter().enumerate() {
            lanes[group[k].0] = *lane;
        }
    }
    lanes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(from: i64, to: i64, tracks: Vec<Vec<i64>>) -> PhaseBand {
        PhaseBand { from, to, tracks }
    }

    // ── 折带 ──

    #[test]
    fn no_alongside_yields_no_bands() {
        let bands = fold_phase_bands(3, &[]);
        assert!(bands.is_empty(), "没有并行就没有带：{bands:?}");
    }

    #[test]
    fn alongside_is_symmetrized() {
        // ★真源 :17-19 —— alongside 只有后来者报得出，要对称化。
        // 只报 [1] 里含 0 → 0 与 1 互为并行 → 一条带 [0,1]。
        let bands = fold_phase_bands(2, &[vec![], vec![0]]);
        assert_eq!(bands.len(), 1);
        assert_eq!(bands[0].from, 0);
        assert_eq!(bands[0].to, 1);
    }

    #[test]
    fn out_of_range_and_self_refs_are_ignored() {
        // 真源 :34 —— 越界、自指都当没说。
        let bands = fold_phase_bands(2, &[vec![99, -1, 0], vec![]]);
        assert!(
            bands.is_empty(),
            "自指与越界都不构成并行：{bands:?}"
        );
    }

    #[test]
    fn first_member_lands_on_track_zero() {
        // ★真源 :26-27 —— 第一个成员总在轨道 0 上。
        let bands = fold_phase_bands(3, &[vec![], vec![0], vec![1, 0]]);
        assert!(!bands.is_empty());
        // 站 0 是带内首个成员 → 轨道 0。
        assert_eq!(track_of(&bands, 0), 0);
    }

    #[test]
    fn intersecting_spans_are_merged() {
        // 真源 :22 —— `~` 连通分量的区间相交就并起来。
        // 0∥1、2∥3、1∥2 → 连成 [0,3]。
        let bands = fold_phase_bands(4, &[vec![1], vec![0, 2], vec![1, 3], vec![2]]);
        assert_eq!(bands.len(), 1);
        assert_eq!((bands[0].from, bands[0].to), (0, 3));
    }

    #[test]
    fn disjoint_pairs_form_separate_bands() {
        // [0,1] 与 [3,4] 不相邻 → 两条带。
        let bands = fold_phase_bands(5, &[vec![1], vec![0], vec![], vec![4], vec![3]]);
        assert_eq!(bands.len(), 2);
        assert_eq!((bands[0].from, bands[0].to), (0, 1));
        assert_eq!((bands[1].from, bands[1].to), (3, 4));
    }

    #[test]
    fn track_greedy_colouring_separates_parallel_members() {
        // 0∥1、1∥2、0∦2 → 轨道分配：0→0、1→1、2→0。
        let bands = fold_phase_bands(3, &[vec![1], vec![0, 2], vec![1]]);
        assert_eq!(bands.len(), 1);
        assert_eq!(track_of(&bands, 0), 0);
        assert_eq!(track_of(&bands, 1), 1, "与 0、2 都并行 → 另开一道");
        assert_eq!(track_of(&bands, 2), 0, "与 0 不并行 → 回到轨道 0");
    }

    // ── 查带 ──

    #[test]
    fn band_of_and_track_of() {
        let bands = vec![band(0, 2, vec![vec![0, 2], vec![1]])];
        assert!(band_of(&bands, 1).is_some());
        assert_eq!(track_of(&bands, 0), 0);
        assert_eq!(track_of(&bands, 1), 1);
        assert_eq!(track_of(&bands, 2), 0);
    }

    #[test]
    fn outside_band_is_main_track() {
        // 真源 :90 —— 带外一律 0（主线）。
        let bands = vec![band(1, 2, vec![vec![1, 2]])];
        assert_eq!(track_of(&bands, 0), 0, "带外也是 0");
        assert_eq!(track_of(&bands, 5), 0, "越界也是 0");
    }

    // ── 边折叠 ──

    #[test]
    fn adjacent_outside_edge_becomes_rail() {
        // 无带、两端相邻 → 平轨。
        let fold = fold_phase_edges(3, &[], &[(0, 1)]);
        assert_eq!(fold.rails.len(), 1);
        assert_eq!((fold.rails[0].from, fold.rails[0].to), (0, 1));
        assert!(fold.arcs.is_empty());
    }

    #[test]
    fn non_adjacent_outside_edge_becomes_top_arc() {
        // 无带、两端不相邻 → 最上面那层的弧。
        let fold = fold_phase_edges(4, &[], &[(0, 3)]);
        assert!(fold.rails.is_empty());
        assert_eq!(fold.arcs.len(), 1);
        assert_eq!(fold.arcs[0].air, 0, "无带时 top = 0");
    }

    #[test]
    fn self_loops_are_dropped() {
        // 真源 :239 —— `from === to` 的自环丢弃。
        let fold = fold_phase_edges(3, &[], &[(1, 1), (0, 2)]);
        assert!(
            fold.arcs.iter().all(|a| a.from != a.to),
            "不该有自环：{:?}",
            fold.arcs
        );
    }

    #[test]
    fn band_absorbs_adjacent_in_out_edges() {
        // 真源 :207-209 —— 出带/入带紧邻的那条已被分叉/汇合吸收。
        let folded = vec![band(1, 2, vec![vec![1, 2]])];
        let fold = fold_phase_edges(3, &folded, &[(0, 1), (2, 3)]);
        // 两条都被吸收 → 不产生弧。
        assert!(
            fold.arcs.is_empty(),
            "紧邻的出入带边被吸收：{:?}",
            fold.arcs
        );
    }

    #[test]
    fn adjacent_bands_make_rail() {
        // 真源 :212-216 —— 两带紧邻（s.to + 1 == t.from）→ 平轨。
        let folded = vec![band(0, 0, vec![vec![0]]), band(1, 1, vec![vec![1]])];
        let fold = fold_phase_edges(2, &folded, &[(0, 1)]);
        assert!(
            fold.arcs.is_empty(),
            "两带紧邻走平轨：{:?}",
            fold.arcs
        );
    }

    #[test]
    fn duplicate_arcs_are_deduped() {
        // 真源 :236 —— 同air 同端点的弧只留一条。
        let fold = fold_phase_edges(4, &[], &[(0, 3), (0, 3)]);
        assert_eq!(fold.arcs.len(), 1);
    }

    #[test]
    fn rails_sorted_by_from_then_to() {
        // 真源 :243-246 —— 轨道段按左端、再按右端排。
        let fold = fold_phase_edges(4, &[], &[(2, 3), (0, 1), (0, 2)]);
        let keys: Vec<(i64, i64)> = fold.rails.iter().map(|r| (r.from, r.to)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "轨道段应有序：{keys:?}");
    }

    #[test]
    fn twin_rails_between_different_tracks() {
        // 真源 :168-171 —— 带内声明序相邻却不在同一轨道的两站出双线段。
        let folded = vec![band(0, 1, vec![vec![0], vec![1]])];
        let fold = fold_phase_edges(2, &folded, &[]);
        assert!(
            fold.rails
                .iter()
                .any(|r| r.kind == Some(TimelineRailKind::Twin)),
            "跨轨道相邻应出双线段：{:?}",
            fold.rails
        );
    }

    #[test]
    fn fork_and_merge_kinds_only_on_non_main_track() {
        // 真源 :150/158 —— 主线不带 kind，分支才带 fork/merge。
        let folded = vec![band(1, 2, vec![vec![1, 2], vec![]])];
        let bound = bind_bands(4, &folded, &[(0, 1), (2, 3)]);
        // 至少能构造出带端点。
        assert!(bound[0].pred.is_some() || bound[0].join.is_some());
    }

    // ── 弧分道 ──

    #[test]
    fn non_intersecting_arcs_share_lane() {
        // ★真源 :265 —— 互不相干的弧同一高度。
        let lanes = assign_arc_lanes(&[(0, 1), (3, 4)]);
        assert_eq!(lanes[0].2, 0);
        assert_eq!(lanes[1].2, 0, "不相干的弧同一道");
    }

    #[test]
    fn touching_arcs_count_as_intersecting() {
        // ★真源 :262-264 —— 闭区间：共用端点也算相交
        // （两条弧在同一站的竖段会连成一线）。
        let lanes = assign_arc_lanes(&[(0, 2), (2, 4)]);
        assert_ne!(lanes[0].2, lanes[1].2, "共用端点该分道");
    }

    #[test]
    fn nested_arcs_inner_outer() {
        // ★真源 :265-266 —— 嵌套的弧里面矮外面高（短的先落）。
        let lanes = assign_arc_lanes(&[(0, 10), (2, 4)]);
        assert_eq!(lanes[0].2, 1, "长弧被推到外道");
        assert_eq!(lanes[1].2, 0, "短弧在内道");
    }

    #[test]
    fn crossing_arcs_pushed_up() {
        // 真源 :267 —— 只有真正交叉的弧才被推上去。
        let lanes = assign_arc_lanes(&[(0, 3), (2, 5)]);
        assert_ne!(lanes[0].2, lanes[1].2, "交叉的弧分道");
    }

    #[test]
    fn back_edges_not_split_unnecessarily() {
        // ★真源 :267-270 —— 各在一头、彼此无关的回边不该一高一矮。
        // (0,1) 与 (3,2) 都是相邻且不相交 → 同一道。
        let lanes = assign_arc_lanes(&[(0, 1), (3, 2)]);
        assert_eq!(lanes[0].2, lanes[1].2, "无关回边同高");
    }

    #[test]
    fn arc_lane_count_is_max_plus_one() {
        // 真源 :251 —— 弧道数量 = 最高车道 + 1。
        assert_eq!(arc_lane_count(&[0, 0, 0]), 1);
        assert_eq!(arc_lane_count(&[0, 2, 1]), 3);
        assert_eq!(arc_lane_count(&[]), 0);
    }

    #[test]
    fn air_lanes_are_assigned_per_air_group() {
        // ★真源 :280-284 —— 每层空独立着色，跨轨道的弧在最上层。
        let arcs = vec![
            ArcSpec {
                from: 0,
                to: 3,
                air: 0,
            },
            ArcSpec {
                from: 1,
                to: 2,
                air: 0,
            },
            ArcSpec {
                from: 0,
                to: 3,
                air: 1,
            },
        ];
        let lanes = assign_air_lanes(&arcs);
        assert_eq!(lanes.len(), 3);
        // 同 air 组内：嵌套 → 内层 0、外层 1。
        assert_eq!(lanes[1], 0, "短弧在内道");
        assert_eq!(lanes[0], 1, "长弧被推上去");
        // 不同 air 组独立 → 第一条又从 0 起。
        assert_eq!(lanes[2], 0, "另一层空独立着色");
    }

    #[test]
    fn assign_arc_lanes_preserves_payload_order() {
        // 真源 :260 —— 同跨度保持载荷序。
        let pairs = [(5, 6), (0, 1), (2, 3)];
        let lanes = assign_arc_lanes(&pairs);
        assert_eq!(lanes.len(), 3);
        // 三段互不相交 → 全在道 0。
        assert!(lanes.iter().all(|(_, _, l)| *l == 0));
    }
}