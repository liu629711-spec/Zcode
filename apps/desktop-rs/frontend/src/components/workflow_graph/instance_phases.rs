//! 1:1 翻译 `packages/ui/src/components/workflow-graph/instance-phases.ts`（73 行）。
//!
//! 运行时实例的归属阶段。
//!
//! ★真源 :6-12 注释——静态的 may-set 拷贝按阶段索引（`ask#1~phase#3`），
//! 运行时实例带同一个坐标——引擎在铸造 ordinal 的那一刻记下的当前阶段名
//! （`phaseName`）。这里把「一个戳落在哪些 display 阶段上」收成一条规则，
//! 卡片绑定（`participant_model.rs`）与站的观察（`timeline_model.rs`）共用：
//! 一次**划分**，而不是一次广播。

use std::collections::HashMap;
use std::collections::HashSet;

use super::phase_name::phase_name_matches;
use super::run_state::WorkflowRunState;
use super::types::WorkflowCausalityGraphData;

/// `runHasPhaseVocabulary`（真源 :15-21）。
///
/// 这次 run 说过阶段的话吗：任一 actor / 节点带戳。
/// 旧 CLI、旧 run、无标记脚本都没有。
pub fn run_has_phase_vocabulary(run: Option<&WorkflowRunState>) -> bool {
    let Some(run) = run else {
        return false;
    };
    run.actors.iter().any(|a| a.phase_name.is_some())
        || run.nodes.iter().any(|n| n.phase_name.is_some())
}

/// `phasesOf`（真源 :31-43）：一个戳的归属阶段集。
///
/// ★真源 :24-30 注释——
/// - 有戳 → 名字匹配的 display 阶段（`phaseNameMatches`，截断兜底在那里）；
/// - 无戳且 run 有词汇 → **无名**的 display 阶段（`unphased` / 隐式 `workflow`）：
///   首个标记之前的 step 分析器正是放在无名的 `unphased` 里，「无戳 ↔ 无名」是
///   同一个事实的两面；
/// - 无戳且 run 无词汇 → 全部阶段（今天的行为）。
///
/// 任一分支结果为空 → 全部阶段：**宁可重复显示，也不把一个在跑的子代理藏起来**。
pub fn phases_of(
    phase_name: Option<&str>,
    graph: &WorkflowCausalityGraphData,
    run_has_vocabulary: bool,
) -> HashSet<String> {
    let phases = graph.phases.as_deref().unwrap_or_default();
    phases_of_in(phases, phase_name, run_has_vocabulary)
}

/// `phasesOf` 的规则体（真源 :36-42），只吃阶段表——`PhaseBinder` 已经把表握在手里，
/// 不必为了一次解析重建整个图。
fn phases_of_in(
    phases: &[super::types::WorkflowPhaseData],
    phase_name: Option<&str>,
    run_has_vocabulary: bool,
) -> HashSet<String> {
    let all = || -> HashSet<String> { phases.iter().map(|p| p.id.clone()).collect() };
    if phase_name.is_none() && !run_has_vocabulary {
        return all();
    }
    let matched: Vec<&super::types::WorkflowPhaseData> = phases
        .iter()
        .filter(|phase| match phase_name {
            None => phase.name.is_none(),
            Some(runtime) => phase_name_matches(phase.name.as_deref(), Some(runtime)),
        })
        .collect();
    if matched.is_empty() {
        all()
    } else {
        matched.into_iter().map(|p| p.id.clone()).collect()
    }
}

/// `PhaseBinder`（真源 :46-50）：一次视图算一遍的解析器。
///
/// 词汇表判定与每个戳的结果都只算一次（每个节点都要问一遍）。
#[derive(Debug, Clone)]
pub struct PhaseBinder {
    /// 图自己还没有阶段词汇表（无标记脚本的原图，UI 合成隐式阶段之前）：
    /// 没有可划分的坐标，一律算属于——否则「归属集恒为空」会把每张卡的
    /// 实例全部抹掉。
    ungrouped: bool,
    vocabulary: bool,
    phases: Vec<super::types::WorkflowPhaseData>,
    cache: HashMap<Option<String>, HashSet<String>>,
}

impl PhaseBinder {
    /// `resolve`（真源 :61-67）：按戳解析归属阶段集，带一层缓存。
    fn resolve(&mut self, phase_name: Option<&str>) -> &HashSet<String> {
        let key = phase_name.map(str::to_string);
        if !self.cache.contains_key(&key) {
            let resolved = phases_of_in(&self.phases, phase_name, self.vocabulary);
            self.cache.insert(key.clone(), resolved);
        }
        self.cache.get(&key).expect("刚写入的键必然在")
    }

    /// `phasesOf(phaseName)`（真源 :47）。
    pub fn phases_of(&mut self, phase_name: Option<&str>) -> &HashSet<String> {
        self.resolve(phase_name)
    }

    /// `has(phaseId, phaseName)`（真源 :69）：这个戳属于这一站 / 这张卡吗。
    pub fn has(&mut self, phase_id: &str, phase_name: Option<&str>) -> bool {
        self.ungrouped || self.resolve(phase_name).contains(phase_id)
    }
}

/// `phaseBinder`（真源 :52-72）：建一次解析器。
pub fn phase_binder(graph: &WorkflowCausalityGraphData, run: Option<&WorkflowRunState>) -> PhaseBinder {
    let vocabulary = run_has_phase_vocabulary(run);
    let phases = graph.phases.clone().unwrap_or_default();
    // ★真源 :57-58 —— 无阶段词汇表的原图一律算属于。
    let ungrouped = phases.is_empty();
    PhaseBinder {
        ungrouped,
        vocabulary,
        phases,
        cache: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with(phases: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(&format!(
            r#"{{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                "phases": {phases}}}"#
        ))
        .unwrap()
    }

    fn run_with_actor_phase(name: Option<&str>) -> WorkflowRunState {
        let mut actor = serde_json::json!({
            "siteId": "ask", "ordinal": 0, "status": "running"
        });
        if let Some(name) = name {
            actor["phaseName"] = serde_json::Value::from(name);
        }
        serde_json::from_value(serde_json::json!({
            "runId": "r1", "status": "running",
            "actors": [actor],
            "nodes": []
        }))
        .unwrap()
    }

    fn ids(set: &HashSet<String>) -> Vec<&str> {
        let mut v: Vec<&str> = set.iter().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    // ── runHasPhaseVocabulary ──

    #[test]
    fn vocabulary_needs_a_stamp() {
        // ★真源 :17-20 —— 任一 actor / 节点带戳才算有词汇。
        assert!(run_has_phase_vocabulary(Some(&run_with_actor_phase(Some("预备")))));
        assert!(!run_has_phase_vocabulary(Some(&run_with_actor_phase(None))));
        assert!(!run_has_phase_vocabulary(None), "无 run → 无词汇");

        // 节点带戳也算。
        let mut bare = run_with_actor_phase(None);
        bare.nodes.push(
            serde_json::from_value(serde_json::json!({
                "siteId": "ask", "ordinal": 0,
                "phase": "executing", "phaseName": "主体"
            }))
            .unwrap(),
        );
        assert!(run_has_phase_vocabulary(Some(&bare)));
    }

    // ── phasesOf ──

    #[test]
    fn unstamped_run_without_vocabulary_claims_all_phases() {
        // ★真源 :38 —— 无戳且 run 无词汇 → 全部阶段（今天的行为）。
        let graph = graph_with(r#"[{"id": "pre"}, {"id": "main"}]"#);
        let set = phases_of(None, &graph, false);
        assert_eq!(ids(&set), vec!["main", "pre"]);
    }

    #[test]
    fn unstamped_with_vocabulary_claims_nameless_phase() {
        // ★真源 :39-41 —— 无戳且 run 有词汇 → 无名的 display 阶段
        //（unphased / 隐式 workflow）。「无戳 ↔ 无名」是同一个事实的两面。
        let graph = graph_with(
            r#"[{"id": "unphased"}, {"id": "main", "name": "主体"}]"#,
        );
        let set = phases_of(None, &graph, true);
        assert_eq!(ids(&set), vec!["unphased"], "只认无名阶段");
    }

    #[test]
    fn stamped_matches_display_phase_by_name() {
        // ★真源 :39-41 —— 有戳 → 名字匹配的 display 阶段。
        let graph = graph_with(
            r#"[{"id": "unphased"}, {"id": "p1", "name": "预备"}, {"id": "p2", "name": "主体"}]"#,
        );
        assert_eq!(ids(&phases_of(Some("预备"), &graph, true)), vec!["p1"]);
        assert_eq!(ids(&phases_of(Some("主体"), &graph, true)), vec!["p2"]);
    }

    #[test]
    fn empty_match_falls_back_to_all_phases() {
        // ★真源 :42 —— 任一分支结果为空 → 全部阶段：
        // 宁可重复显示，也不把一个在跑的子代理藏起来。
        let graph = graph_with(r#"[{"id": "p1", "name": "预备"}]"#);
        let set = phases_of(Some("完全无关的名字"), &graph, true);
        assert_eq!(ids(&set), vec!["p1"], "落回全部阶段而不是空集");
    }

    #[test]
    fn truncated_display_name_still_matches() {
        // phase-name 的截断兜底在这里生效：顶到上界的 display 名按前缀认。
        let long = "阶".repeat(super::super::phase_name::DISPLAY_PHASE_NAME_BOUND);
        let graph = graph_with(&format!(r#"[{{"id": "p1", "name": "{long}"}}]"#));
        let runtime = format!("{long}后续");
        assert_eq!(ids(&phases_of(Some(&runtime), &graph, true)), vec!["p1"]);
    }

    #[test]
    fn graph_without_phases_claims_nothing() {
        // 图没有阶段表时 all() 是空集（真源 :36 的 `?? []`）。
        let graph = graph_with("[]");
        assert!(phases_of(None, &graph, false).is_empty());
    }

    // ── phaseBinder ──

    #[test]
    fn binder_on_ungrouped_graph_claims_everything() {
        // ★真源 :57-59 —— 原图没有阶段词汇表：一律算属于，
        // 否则「归属集恒为空」会把每张卡的实例全部抹掉。
        let graph = graph_with("[]");
        let mut binder = phase_binder(&graph, Some(&run_with_actor_phase(Some("预备"))));
        assert!(binder.has("any-phase", Some("预备")));
        assert!(binder.has("another", None));
    }

    #[test]
    fn binder_has_answers_the_card_question() {
        // 真源 :69 —— `has(phaseId, phaseName)`。
        let graph = graph_with(r#"[{"id": "pre", "name": "预备"}, {"id": "main", "name": "主体"}]"#);
        let run = run_with_actor_phase(None);
        let mut binder = phase_binder(&graph, Some(&run));
        // run 无戳（vocabulary=false）→ 任何戳都认全部阶段。
        assert!(binder.has("main", None));
        assert!(binder.has("pre", None));
    }

    #[test]
    fn binder_splits_an_actor_by_its_birth_stamp() {
        // ★真源 :8-11 —— 一次**划分**，不是一次广播：
        // 同一个 actor 落在它出生时的那个阶段，不点亮其他阶段。
        let graph = graph_with(r#"[{"id": "pre", "name": "预备"}, {"id": "main", "name": "主体"}]"#);
        let run = run_with_actor_phase(Some("预备"));
        let mut binder = phase_binder(&graph, Some(&run));
        assert!(binder.has("pre", Some("预备")));
        assert!(!binder.has("main", Some("预备")), "不该点亮兄弟阶段");
        assert!(binder.has("main", Some("主体")));
    }

    #[test]
    fn binder_caches_per_stamp() {
        // 真源 :45/60 —— 每个节点都要问一遍，结果只算一次。
        // 这里通过「同一戳两次结果一致且视图复用」间接验证缓存不串味。
        let graph = graph_with(r#"[{"id": "unphased"}, {"id": "pre", "name": "预备"}]"#);
        // 旧 run：任一 actor/节点都不带戳 → vocabulary=false。
        let run = run_with_actor_phase(None);
        let mut binder = phase_binder(&graph, Some(&run));
        // ★真源 :38 —— 无戳且 run 无词汇 → 全部阶段（今天的行为）。
        assert_eq!(ids(binder.phases_of(None)), vec!["pre", "unphased"]);
        // 再问一次，缓存命中但结果不变。
        assert_eq!(ids(binder.phases_of(None)), vec!["pre", "unphased"]);
        // 有戳 → 走名字匹配分支，与无戳不同。
        assert_eq!(ids(binder.phases_of(Some("预备"))), vec!["pre"]);
        assert_eq!(ids(binder.phases_of(Some("预备"))), vec!["pre"]);
    }

    #[test]
    fn unstamped_with_vocabulary_cache_hits_the_nameless_branch() {
        // 有戳的 run + 无戳查询 → 无名分支（与上面「旧 run 落全部阶段」是两回事）。
        let graph = graph_with(r#"[{"id": "unphased"}, {"id": "pre", "name": "预备"}]"#);
        let run = run_with_actor_phase(Some("预备"));
        let mut binder = phase_binder(&graph, Some(&run));
        assert_eq!(ids(binder.phases_of(None)), vec!["unphased"]);
        assert_eq!(ids(binder.phases_of(None)), vec!["unphased"]);
    }
}