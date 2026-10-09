//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/station-observation.ts`（126 行）。
//!
//! 一站观察到的东西：落在这一站站点上的实例，加上界在这一站花掉的**表外**条目。
//!
//! ★真源 :9-10 注释——与 `timeline_bands.rs` 一样只计算展示模型，
//! 不依赖 React、DOM 或时钟。

use std::collections::HashSet;

use crate::components::workflow_graph::instance_phases::PhaseBinder;
use crate::components::workflow_graph::phase_name::phase_name_matches;
use crate::components::workflow_graph::run_state::{WorkflowRunNode, WorkflowRunPhase, WorkflowRunState};
use crate::components::workflow_graph::types::WorkflowStepData;

/// `ObservedPhase`（真源 :12-19）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObservedPhase {
    pub visited: bool,
    pub rounds: i64,
    pub settled: i64,
    pub observed: i64,
    /// 控制流进入过这一站（`run.phases` 里有它的进入记录）。
    pub entered: bool,
}

/// `StationUnlisted`（真源 :27-32）。
///
/// ★真源 :21-26 注释——归约列不出来的那些（`run.unlistedByPhase`）。`actors` 是这一站
/// **出生、此刻不在表里**的子代理——出生就被拒的、排队时被淘汰的、跑完被淘汰的都算，
/// 它们没有药丸、没有脸、没有转录；`settled` 是其中已知跑完的，`failed ⊆ settled`；
/// `nodes_settled` 是记在这一格上的表外已结算**节点**数。
/// 缺席 = 这一站一条都没少。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StationUnlisted {
    pub actors: i64,
    pub settled: i64,
    pub failed: i64,
    pub nodes_settled: i64,
}

/// `stationUnlisted`（真源 :39-56）。
///
/// ★真源 :34-38 注释——表外那一格按**出生阶段的戳**归位：与药丸绑定、站的观察
/// 同一个 `phaseBinder`——一次划分，不是一次广播。无 `phaseName` 的那一格因此落在
/// 无名的站上（与无戳实例同一条规则），而不是凭空挑一站。
/// 同名再入的两站各拿一份，与节点的算法一致。
pub fn station_unlisted(
    run: Option<&WorkflowRunState>,
    binder: &mut PhaseBinder,
    phase_id: &str,
) -> Option<StationUnlisted> {
    let buckets = run?.unlisted_by_phase.as_ref()?;
    let mut total = StationUnlisted::default();
    for bucket in buckets {
        if !binder.has(phase_id, bucket.phase_name.as_deref()) {
            continue;
        }
        total.actors += bucket.actors;
        // ★真源 :50 —— 归零的子键在线上缺席：缺席说的是「零个」，
        // 不是「不知道」——还没跑完的那些因此留在 pending。
        total.settled += bucket.actors_settled.unwrap_or(0);
        total.failed += bucket.actors_failed.unwrap_or(0);
        total.nodes_settled += bucket.settled;
    }
    // 四个数全零的格子整个不在（真源 :135，与本文件其余「无则缺席」同规）。
    if total.actors == 0 && total.settled == 0 && total.nodes_settled == 0 {
        None
    } else {
        Some(total)
    }
}

/// `phaseEntryFor`（真源 :64-74）：一站的进入记录。
///
/// ★真源 :60-63 注释——按名字关联（`phaseNameMatches`，与实例绑定共用
/// `phase-name.rs` 中的规则）。同一个 128 字前缀下可能有两条记录，**精确的那条优先**。
pub fn phase_entry_for<'a>(
    run: Option<&'a WorkflowRunState>,
    name: Option<&str>,
) -> Option<&'a WorkflowRunPhase> {
    let entries = run?.phases.as_ref()?;
    let name = name?;
    entries
        .iter()
        .find(|entry| entry.name == name)
        .or_else(|| {
            entries
                .iter()
                .find(|entry| phase_name_matches(Some(name), Some(&entry.name)))
        })
}

/// `siteIdsOf`（真源 :80-84）：成员 step 的 `source ?? id`。
///
/// ★真源 :76-79 注释——may-set 拷贝报的是**站点 id**，与 `run-status.rs` 的关联键同源；
/// 漏掉 `source` 会让拷贝站永远「未到」。
pub fn site_ids_of(steps: &[WorkflowStepData]) -> HashSet<String> {
    steps
        .iter()
        .map(|step| step.source.clone().unwrap_or_else(|| step.id.clone()))
        .collect()
}

/// `observePhase`（真源 :91-125）：一站观察到的节点。
///
/// ★真源 :86-90 注释——站点相同还不够：同一个站点被 k 个阶段再入时 k 张卡共享站点 id，
/// 节点还要按实例的出生戳落到这一站（`belongs`），否则 visited / rounds /
/// fraction 一起虚高 k 倍。
///
/// `belongs` 是闭包（真源传的是 `(node) => binder.has(phase.id, node.phaseName)`）——
/// `binder` 需要 `&mut`（内部有缓存），闭包拿不到，所以 Rust 侧把判定交回调用方
/// 逐节点问，语义与真源一致。
pub fn observe_phase(
    run: Option<&WorkflowRunState>,
    site_ids: &HashSet<String>,
    entry: Option<&WorkflowRunPhase>,
    mut belongs: impl FnMut(&WorkflowRunNode) -> bool,
    unlisted: Option<StationUnlisted>,
) -> ObservedPhase {
    let mut result = ObservedPhase::default();
    let Some(run) = run else {
        return result;
    };
    for node in &run.nodes {
        if !site_ids.contains(&node.site_id) || !belongs(node) {
            continue;
        }
        result.visited = true;
        result.observed += 1;
        if node.ordinal > result.rounds {
            result.rounds = node.ordinal;
        }
        if node.phase == crate::components::workflow_graph::run_state::WorkflowRunNodePhase::Settled {
            result.settled += 1;
        }
    }
    // ★真源 :113-114 —— 表外已结算的**节点**：分子与分母一起抬。它们确实跑过，
    // 只是详情停在界上——少算分母会让「300/300」变成「1/1」，那是一句假话；
    // visited / rounds 不动，那两个说的是控制流。
    let nodes_settled = unlisted.map(|u| u.nodes_settled).unwrap_or(0);
    result.observed += nodes_settled;
    result.settled += nodes_settled;
    // ★真源 :117-118 —— 进入记录：到过 = 有节点落在这站 ∨ 控制流进入过；
    // 轮次取两者之大（单阶段循环体的第二轮由节点数出来，
    // 零成员站的第二轮只有进入记录知道）。
    if let Some(entry) = entry {
        result.entered = true;
        result.visited = true;
        if entry.rounds > result.rounds {
            result.rounds = entry.rounds;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_from_str(raw: &str) -> WorkflowRunState {
        serde_json::from_str(raw).unwrap()
    }

    fn graph_with_steps(steps: &str) -> crate::components::workflow_graph::types::WorkflowCausalityGraphData {
        serde_json::from_str(&format!(
            r#"{{"steps": {steps}, "lanes": [], "participants": [], "handoffs": []}}"#
        ))
        .unwrap()
    }

    fn binder_for(graph: &crate::components::workflow_graph::types::WorkflowCausalityGraphData, run: Option<&WorkflowRunState>) -> PhaseBinder {
        crate::components::workflow_graph::instance_phases::phase_binder(graph, run)
    }

    // ── siteIdsOf ──

    #[test]
    fn site_ids_prefer_source_over_step_id() {
        // ★真源 :76-79 —— may-set 拷贝报的是站点 id。
        let graph = graph_with_steps(
            r#"[
                {"id": "ask#1~lane#2", "kind": "ask", "label": "问", "lane": "l2", "source": "ask#1"},
                {"id": "ask#2", "kind": "ask", "label": "问", "lane": "l1"}
            ]"#,
        );
        let ids = site_ids_of(&graph.steps);
        assert!(ids.contains("ask#1"), "拷贝报展开自的站点 id");
        assert!(!ids.contains("ask#1~lane#2"));
        assert!(ids.contains("ask#2"), "无 source 时用自己的 id");
    }

    // ── phaseEntryFor ──

    #[test]
    fn phase_entry_prefers_exact_name_over_prefix() {
        // ★真源 :61-63 —— 同一个 128 字前缀下可能有两条记录，精确的那条优先。
        let long = "阶".repeat(crate::components::workflow_graph::phase_name::DISPLAY_PHASE_NAME_BOUND);
        let run = run_from_str(&format!(
            r#"{{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "phases": [{{"name": "{long}甲", "rounds": 5}}, {{"name": "{long}乙", "rounds": 2}}]}}"#
        ));
        let entry = phase_entry_for(Some(&run), Some(&format!("{long}乙"))).unwrap();
        assert_eq!(entry.rounds, 2, "精确匹配优先");
    }

    #[test]
    fn phase_entry_falls_back_to_truncated_prefix() {
        // ★真源 :46-47 —— display 名（被 boundGraphText 截到上界）与运行时全名
        // 的关联靠前缀兜底。方向别搞反：被截的是 display 侧。
        let long = "阶".repeat(crate::components::workflow_graph::phase_name::DISPLAY_PHASE_NAME_BOUND);
        let run = run_from_str(&format!(
            r#"{{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "phases": [{{"name": "{long}后续", "rounds": 3}}]}}"#
        ));
        let entry = phase_entry_for(Some(&run), Some(&long)).unwrap();
        assert_eq!(entry.rounds, 3, "顶到上界的 display 名按前缀兜底");
    }

    #[test]
    fn phase_entry_absent_on_either_side() {
        // 真源 :68 —— entries 为 undefined 或 name 为 undefined 都返回 undefined。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "phases": [{"name": "预备", "rounds": 1}]}"#,
        );
        assert!(phase_entry_for(Some(&run), None).is_none(), "无名阶段不匹配具名记录");
        assert!(phase_entry_for(None, Some("预备")).is_none(), "无 run");
        let bare = run_from_str(r#"{"runId": "r", "status": "running", "actors": [], "nodes": []}"#);
        assert!(phase_entry_for(Some(&bare), Some("预备")).is_none(), "无进入记录");
    }

    // ── stationUnlisted ──

    #[test]
    fn unlisted_absent_without_the_key() {
        let run = run_from_str(r#"{"runId": "r", "status": "running", "actors": [], "nodes": []}"#);
        let graph = graph_with_steps("[]");
        let mut binder = binder_for(&graph, Some(&run));
        assert!(station_unlisted(Some(&run), &mut binder, "p1").is_none());
        assert!(station_unlisted(None, &mut binder, "p1").is_none(), "无 run → 缺席");
    }

    #[test]
    fn unlisted_is_partitioned_by_birth_stamp() {
        // ★真源 :34-38 —— 一次划分，不是一次广播：同名再入的两站各拿一份。
        let graph = serde_json::from_str(
            r#"{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                "phases": [{"id": "p1", "name": "阶段一"}, {"id": "p2", "name": "阶段二"}]}"#,
        )
        .unwrap();
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "unlistedByPhase": [
                  {"phaseName": "阶段一", "actors": 4, "actorsSettled": 2, "settled": 9},
                  {"phaseName": "阶段二", "actors": 3, "settled": 5}
                ]}"#,
        );
        let mut binder = binder_for(&graph, Some(&run));
        let one = station_unlisted(Some(&run), &mut binder, "p1").unwrap();
        assert_eq!(one.actors, 4);
        assert_eq!(one.settled, 2);
        assert_eq!(one.nodes_settled, 9);
        let two = station_unlisted(Some(&run), &mut binder, "p2").unwrap();
        assert_eq!(two.actors, 3);
        assert_eq!(two.settled, 0, "归零的子键缺席 → 按零算，不是「不知道」");
        assert_eq!(two.nodes_settled, 5);
    }

    #[test]
    fn unlisted_zero_bucket_is_dropped() {
        // ★真源 :55 —— 四个数全零的格子整个不在。
        let graph = graph_with_steps("[]");
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "unlistedByPhase": [{"phaseName": "预备", "actors": 0, "settled": 0}]}"#,
        );
        let mut binder = binder_for(&graph, Some(&run));
        assert!(station_unlisted(Some(&run), &mut binder, "预备").is_none());
    }

    #[test]
    fn unlisted_unstamped_bucket_lands_on_nameless_phase() {
        // ★真源 :135 —— 无 stamp 的那一格落在无名的站上，而不是凭空挑一站。
        // run 里有别的戳（vocabulary=true），所以无戳查询才走无名分支。
        let graph = serde_json::from_str(
            r#"{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                "phases": [{"id": "unphased"}, {"id": "p1", "name": "阶段一"}]}"#,
        )
        .unwrap();
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running",
                            "phaseName": "阶段一"}],
                "nodes": [],
                "unlistedByPhase": [{"actors": 2, "settled": 1}]}"#,
        );
        let mut binder = binder_for(&graph, Some(&run));
        assert_eq!(
            station_unlisted(Some(&run), &mut binder, "unphased")
                .unwrap()
                .actors,
            2
        );
        assert!(
            station_unlisted(Some(&run), &mut binder, "p1").is_none(),
            "无 stamp 的格子不广播给具名阶段"
        );
    }

    #[test]
    fn unlisted_unstamped_bucket_broadcasts_when_run_has_no_vocabulary() {
        // ★真源 instance-phases :38 —— 无戳且 run 无词汇 → 全部阶段。
        // 旧run 逐字节不变：表外那一格在这种run 里也是按车道广播的。
        let graph = serde_json::from_str(
            r#"{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                "phases": [{"id": "unphased"}, {"id": "p1", "name": "阶段一"}]}"#,
        )
        .unwrap();
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "unlistedByPhase": [{"actors": 2, "settled": 1}]}"#,
        );
        let mut binder = binder_for(&graph, Some(&run));
        assert_eq!(
            station_unlisted(Some(&run), &mut binder, "unphased")
                .unwrap()
                .actors,
            2
        );
        assert_eq!(
            station_unlisted(Some(&run), &mut binder, "p1")
                .unwrap()
                .actors,
            2,
            "旧 run：落全部阶段"
        );
    }

    // ── observePhase ──

    #[test]
    fn observe_without_run_is_all_zero() {
        // 真源 :105 —— 无 run 直接返回零值（visited 不假点灯）。
        let result = observe_phase(None, &HashSet::new(), None, |_| true, None);
        assert_eq!(result, ObservedPhase::default());
        assert!(!result.visited);
        assert!(!result.entered);
    }

    #[test]
    fn observe_counts_nodes_settled_and_max_ordinal() {
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok"},
                {"siteId": "s1", "ordinal": 2, "phase": "executing"},
                {"siteId": "s1", "ordinal": 1, "phase": "settled", "outcome": "failed"}
              ]}"#,
        );
        let sites: HashSet<String> = ["s1".to_string()].into_iter().collect();
        let result = observe_phase(Some(&run), &sites, None, |_| true, None);
        assert!(result.visited);
        assert_eq!(result.observed, 3);
        assert_eq!(result.settled, 2, "settled 相位才计数");
        assert_eq!(result.rounds, 2, "取最大 ordinal");
        assert!(!result.entered, "没有进入记录");
    }

    #[test]
    fn observe_filters_by_site_and_belongs() {
        // ★真源 :107 —— 站点相同还不够，还要按实例的出生戳落到这一站。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "executing", "phaseName": "阶段一"},
                {"siteId": "s1", "ordinal": 1, "phase": "executing", "phaseName": "阶段二"},
                {"siteId": "other", "ordinal": 9, "phase": "executing"}
              ]}"#,
        );
        let sites: HashSet<String> = ["s1".to_string()].into_iter().collect();
        let result = observe_phase(Some(&run), &sites, None, |n| n.phase_name.as_deref() == Some("阶段一"), None);
        assert_eq!(result.observed, 1, "只收本阶段的节点");
        assert_eq!(result.rounds, 0);
    }

    #[test]
    fn unlisted_nodes_lift_both_numerator_and_denominator() {
        // ★真源 :113-114 —— 分子与分母一起抬：少算分母会让「300/300」变成「1/1」。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok"}
              ]}"#,
        );
        let sites: HashSet<String> = ["s1".to_string()].into_iter().collect();
        let unlisted = StationUnlisted {
            actors: 30,
            settled: 30,
            failed: 0,
            nodes_settled: 299,
        };
        let result = observe_phase(Some(&run), &sites, None, |_| true, Some(unlisted));
        assert_eq!(result.observed, 300, "1 + 299");
        assert_eq!(result.settled, 300, "分母一起抬");
        assert_eq!(result.rounds, 0, "★rounds 不动——它说的是控制流");
        assert!(result.visited, "★visited 也不动");
    }

    #[test]
    fn phase_entry_marks_entered_and_lifts_rounds() {
        // ★真源 :117-118 —— 到过 = 有节点落在这站 ∨ 控制流进入过；
        // 轮次取两者之大（零成员站的第二轮只有进入记录知道）。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "phases": [{"name": "预备", "rounds": 3}]}"#,
        );
        let entry = phase_entry_for(Some(&run), Some("预备")).unwrap();
        let result = observe_phase(Some(&run), &HashSet::new(), Some(entry), |_| true, None);
        assert!(result.entered);
        assert!(result.visited);
        assert_eq!(result.rounds, 3);
        assert_eq!(result.observed, 0, "进入记录不造节点");
    }

    #[test]
    fn node_rounds_win_over_entry_when_larger() {
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [],
                "nodes": [{"siteId": "s1", "ordinal": 5, "phase": "executing"}],
                "phases": [{"name": "预备", "rounds": 2}]}"#,
        );
        let entry = phase_entry_for(Some(&run), Some("预备")).unwrap();
        let sites: HashSet<String> = ["s1".to_string()].into_iter().collect();
        let result = observe_phase(Some(&run), &sites, Some(entry), |_| true, None);
        assert_eq!(result.rounds, 5, "取两者之大");
    }
}