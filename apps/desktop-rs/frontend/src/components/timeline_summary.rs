//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/timeline-summary.ts`（224 行）。
//!
//! 卡片表头细节与页脚摘要行的文案素材。侧栏状态头的摘要行也读这里——
//! **同一个 run 在两个面上必须说同一句话**。
//!
//! ★真源 :9-10 注释——纯函数 + 注入的 formatMessage：本仓的轻量 intl 没有 ICU 复数，
//! 单复数各自一个 key。Rust 侧直接用全局 `i18n`（真源把 formatter 当参数注入是为了
//! 单测能塞假 formatter；Rust 侧 `i18n::text` / `i18n::format` 已经是同样的注入点）。

use std::collections::HashSet;

use crate::ToolCallBlocks::i18n;

use crate::components::timeline_model::WorkflowTimelineModel;
use crate::components::workflow_graph::run_state::{
    WorkflowRunActorStatus, WorkflowRunState, workflow_run_step_counts};
use crate::components::workflow_graph::types::{LaneClass, WorkflowCausalityGraphData};

/// `TimelineCounts`（真源 :16-21）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineCounts {
    pub phases: usize,
    /// 不重复的子代理车道数（合成车道不算）。
    pub agents: usize,
    pub steps: usize,
}

/// `timelineCounts`（真源 :23-34）。
pub fn timeline_counts(
    model: &WorkflowTimelineModel,
    graph: Option<&WorkflowCausalityGraphData>,
) -> TimelineCounts {
    let mut lanes: HashSet<&str> = HashSet::new();
    for station in &model.stations {
        for pill in &station.pills {
            if pill.lane_class == LaneClass::Agent {
                lanes.insert(pill.lane.id.as_str());
            }
        }
    }
    // ★真源 :31 —— 草稿不画药丸，子代理数由扫描器直接给。
    let agents = model.draft.map(|d| d.agents).unwrap_or(lanes.len());
    TimelineCounts {
        agents,
        phases: model.stations.len(),
        steps: graph.map(|g| g.steps.len()).unwrap_or(0),
    }
}

/// `timelineRounds`（真源 :37-43）：循环上的站到过的最多轮次；没有循环或还没到过时 0。
pub fn timeline_rounds(model: &WorkflowTimelineModel) -> i64 {
    let mut rounds = 0;
    for station in &model.stations {
        if station.on_loop && station.rounds > rounds {
            rounds = station.rounds;
        }
    }
    rounds
}

/// `count`（真源 :45-47）：单复数各自一个 key（本仓的轻量 intl 没有 ICU 复数）。
fn count(one: &str, many: &str, value: i64) -> String {
    let id = if value == 1 { one } else { many };
    i18n::format(id, &[("count".to_string(), value.to_string())])
}

/// `workflowPhasesDetail`（真源 :53-65）。
///
/// ★真源 :49-52 注释——确认窗表头右侧**只说阶段数**：子代理数与步数
/// 在下方时间线上一眼可见，表头再复述只是噪音。
pub fn workflow_phases_detail(
    model: &WorkflowTimelineModel,
    graph: Option<&WorkflowCausalityGraphData>,
) -> String {
    let counts = timeline_counts(model, graph);
    count(
        "chat.toolCall.workflow.card.phase",
        "chat.toolCall.workflow.card.phases",
        counts.phases as i64,
    )
}

/// `agentsPart`（真源 :68-89）：子代理那一段。
///
/// 跑着时数工作中的，结束后数总数（投影与静态图取大）。
fn agents_part(model: &WorkflowTimelineModel, run: Option<&WorkflowRunState>) -> String {
    if let Some(run) = run {
        if run.status.is_live() {
            let working = run
                .actors
                .iter()
                .filter(|a| a.status == WorkflowRunActorStatus::Running)
                .count() as i64;
            return count(
                "chat.toolCall.workflow.card.agentWorking",
                "chat.toolCall.workflow.card.agentsWorking",
                working,
            );
        }
    }
    let from_run = run.map(|r| r.actors.len() as i64).unwrap_or(0);
    let agents = from_run.max(timeline_counts(model, None).agents as i64);
    count(
        "chat.toolCall.workflow.card.agent",
        "chat.toolCall.workflow.card.agents",
        agents,
    )
}

/// `workflowHeaderDetail`（真源 :97-114）。
///
/// ★真源 :91-96 注释——卡片表头右侧的细节：**只说阶段数与子代理数**
/// （卡上不要出现「步」，只留阶段与子代理）。编写中 / 待确认按静态图数；
/// 联接到 run 后阶段数不变、子代理改成「n 个工作中」（跑着）或总数（结束）。
/// 步数、token、轮次、产物数都不再上表头——它们留在 run 详情页的摘要行。
pub fn workflow_header_detail(
    model: &WorkflowTimelineModel,
    graph: Option<&WorkflowCausalityGraphData>,
    run: Option<&WorkflowRunState>,
    // 子代理模型名（已解析，见 `subagent_model_label.rs`）：细节串已经在说「几个子代理」，
    // 模型名跟在它后面当最后一段，同一段淡色文字。不加芯片、不加前缀。
    // 没指定过模型的 run 缺席这一段（`None`）。
    subagent_model_name: Option<&str>,
) -> String {
    let mut parts = vec![
        workflow_phases_detail(model, graph),
        agents_part(model, run),
    ];
    if let Some(name) = subagent_model_name {
        parts.push(name.to_string());
    }
    parts.join(" · ")
}

/// `workflowCardDetail`（真源 :120-139）的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardDetail {
    pub detail: String,
    pub title: Option<String>,
}

/// `workflowCardDetail`（真源 :120-139）：表头细节串 + 它的 tooltip，一次算完。
///
/// 两张卡（v4 轮尾摘要、旧宿主运行卡）必须说同一句话，所以「模型名加不加」
/// 「tooltip 里放什么」只有这一份实现。建不出时间线模型时整块缺席。
pub fn workflow_card_detail(
    model: Option<&WorkflowTimelineModel>,
    graph: Option<&WorkflowCausalityGraphData>,
    run: Option<&WorkflowRunState>,
    // 子代理模型的规范串（`run.subagentModel`）；`None` = 没指定过模型。
    subagent_model_canonical: Option<&str>,
    provider_name: Option<&dyn Fn(&str) -> Option<String>>,
) -> Option<CardDetail> {
    let model = model?;
    let subagent_model =
        crate::components::subagent_model_label::workflow_subagent_model_card_label(
            subagent_model_canonical,
            provider_name,
        );
    let (name, title) = match subagent_model {
        None => (None, None),
        Some(label) => (Some(label.name), Some(label.title)),
    };
    Some(CardDetail {
        detail: workflow_header_detail(model, graph, run, name.as_deref()),
        title,
    })
}

/// `workflowSummaryParts` 的选项（真源 :153-160）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryOptions {
    /// 默认 `true`（真源 `tokens?: boolean`，只有显式 `false` 才关）。
    pub tokens: bool,
    /// 子代理模型名：在场时是摘要行的**第一段**——这一行本来就是「这条 run 的几个数」，
    /// 模型是它的第一个词。
    pub subagent_model_name: Option<&'static str>,
}

impl Default for SummaryOptions {
    /// ★tokens 缺省**在场**（真源 `tokens?: boolean`）——Rust 的 `bool` 默认是 `false`，
    /// 直接 `derive(Default)` 会把 token 段悄悄关掉。
    fn default() -> Self {
        Self {
            tokens: true,
            subagent_model_name: None,
        }
    }
}

/// `workflowSummaryParts`（真源 :149-224）。
///
/// run 详情页摘要行的各段：`1 个子代理工作中 · 4/7 步 · 42,118 tokens · 第 2 轮`；
/// 终态换成 `3 个子代理 · 11/11 步 · … · 3 轮 · 2 个产物`。返回值只是字符串，
/// 间隔符由渲染方画。
///
/// ★末段数的是**产物**（脚本经 `artifact.*` 交付给用户的产出），不是 `report` 条目：
/// Results 区已撤走，「results」在屏幕上再没有落点。聊天里的卡片不再用它——
/// 卡上只说阶段与子代理（`workflow_header_detail`）；这一行只剩详情页在读。
pub fn workflow_summary_parts(
    model: &WorkflowTimelineModel,
    run: &WorkflowRunState,
    options: SummaryOptions,
) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(name) = options.subagent_model_name {
        parts.push(i18n::format(
            "chat.toolCall.workflow.run.subagentModel.label",
            &[("model".to_string(), name.to_string())],
        ));
    }
    let active = run.status.is_live();
    if active {
        let working = run
            .actors
            .iter()
            .filter(|a| a.status == WorkflowRunActorStatus::Running)
            .count() as i64;
        parts.push(count(
            "chat.toolCall.workflow.card.agentWorking",
            "chat.toolCall.workflow.card.agentsWorking",
            working,
        ));
    } else {
        let agents = (run.actors.len() as i64).max(timeline_counts(model, None).agents as i64);
        parts.push(count(
            "chat.toolCall.workflow.card.agent",
            "chat.toolCall.workflow.card.agents",
            agents,
        ));
    }
    // 步数走 @zcode/shared 的唯一实现：表内 + 表外（撞界后没进表的实例仍算步数）。
    let steps = workflow_run_step_counts(run);
    parts.push(i18n::format(
        "chat.toolCall.workflow.card.steps",
        &[
            ("done".to_string(), steps.settled.to_string()),
            ("total".to_string(), steps.total.to_string()),
        ],
    ));
    if options.tokens {
        parts.push(i18n::format(
            "chat.toolCall.workflow.card.tokens",
            &[("count".to_string(), run.usage.spent_tokens.to_string())],
        ));
    }
    let rounds = timeline_rounds(model);
    // 一轮不上表：没循环或只走过一遍时「第 1 轮」是废话。
    if rounds >= 2 {
        parts.push(i18n::format(
            if active {
                "chat.toolCall.workflow.card.round"
            } else {
                "chat.toolCall.workflow.card.rounds"
            },
            &[("count".to_string(), rounds.to_string())],
        ));
    }
    let artifacts = run.artifacts.as_ref().map(|a| a.len()).unwrap_or(0) as i64;
    if artifacts > 0 {
        parts.push(count(
            "chat.toolCall.workflow.card.artifact",
            "chat.toolCall.workflow.card.artifacts",
            artifacts,
        ));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::timeline_model::{build_workflow_timeline, TimelineInk};

    fn graph_from_str(raw: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(raw).unwrap()
    }

    fn run_from_str(raw: &str) -> WorkflowRunState {
        serde_json::from_str(raw).unwrap()
    }

    /// 两个阶段、一个 agent 车道、无带。
    fn sample_graph() -> WorkflowCausalityGraphData {
        graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "问A", "lane": "l1", "phase": "p1"},
                {"id": "s2", "kind": "ask", "label": "问B", "lane": "l1", "phase": "p2"}
            ], "lanes": [{"id": "l1", "name": "研究员"}], "participants": [
                {"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]},
                {"id": "b", "phase": "p2", "lane": "l1", "steps": ["s2"]}
            ], "handoffs": [{"from": "a", "to": "b"}],
            "phases": [{"id": "p1", "name": "一"}, {"id": "p2", "name": "二"}],
            "phaseEdges": [{"from": "p1", "to": "p2"}]}"#,
        )
    }

    fn empty_run(status: &str) -> WorkflowRunState {
        run_from_str(&format!(
            r#"{{"runId": "r", "status": "{status}",
                "actors": [
                  {{"siteId": "l1", "ordinal": 0, "status": "running"}},
                  {{"siteId": "l1", "ordinal": 1, "status": "waiting"}},
                  {{"siteId": "l1", "ordinal": 2, "status": "completed"}}
                ],
                "nodes": [],
                "usage": {{"spentTokens": 42118, "nodesUsed": 7}}}}"#
        ))
    }

    // ── timelineCounts ──

    #[test]
    fn counts_phases_agents_and_steps() {
        let graph = sample_graph();
        let model = build_workflow_timeline(&graph, None);
        let counts = timeline_counts(&model, Some(&graph));
        assert_eq!(counts.phases, 2);
        assert_eq!(counts.agents, 1, "两条药丸同车道 → 只数一次");
        assert_eq!(counts.steps, 2);
        // 无图时步数是 0（确认窗那种只给模型的场合）。
        assert_eq!(timeline_counts(&model, None).steps, 0);
    }

    #[test]
    fn counts_skip_synthetic_lanes() {
        // 真源 :29 —— 只有 agent 车道算数。
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "r1", "kind": "world-read", "label": "读", "lane": "workspace", "phase": "p1"}
            ], "lanes": [], "participants": [
                {"id": "w", "phase": "p1", "lane": "workspace", "steps": ["r1"]}
            ], "handoffs": [], "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert_eq!(timeline_counts(&model, Some(&graph)).agents, 0, "合成车道不算子代理");
    }

    #[test]
    fn draft_agent_count_wins_over_lanes() {
        // ★真源 :31 —— 草稿不画药丸，子代理数由扫描器直接给。
        let graph = sample_graph();
        let mut model = build_workflow_timeline(&graph, None);
        model.draft = Some(crate::components::timeline_model::DraftAgents { agents: 7 });
        assert_eq!(timeline_counts(&model, None).agents, 7);
    }

    // ── timelineRounds ──

    #[test]
    fn rounds_take_the_max_over_loop_stations() {
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "问A", "lane": "l1", "phase": "p1"},
                {"id": "s2", "kind": "ask", "label": "问B", "lane": "l1", "phase": "p2"}
            ], "lanes": [], "participants": [
                {"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]},
                {"id": "b", "phase": "p2", "lane": "l1", "steps": ["s2"]}
            ], "handoffs": [],
            "phases": [{"id": "p1", "name": "一"}, {"id": "p2", "name": "二"}],
            "phaseEdges": [{"from": "p2", "to": "p1"}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 3, "phase": "executing",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "一"}
              ], "usage": {"spentTokens": 0, "nodesUsed": 1}}"#,
        );
        let model = build_workflow_timeline(&graph, Some(&run));
        assert!(model.stations[0].on_loop, "回边两端上环");
        assert!(model.stations[1].on_loop);
        assert_eq!(timeline_rounds(&model), 3);
    }

    #[test]
    fn rounds_are_zero_without_a_loop() {
        let model = build_workflow_timeline(&sample_graph(), None);
        assert_eq!(timeline_rounds(&model), 0);
    }

    // ── 计数文案 ──

    #[test]
    fn count_switches_key_by_value() {
        // 真源 :45-47 —— 本仓的轻量 intl 没有 ICU 复数，单复数各自一个 key。
        let graph = sample_graph();
        let one = build_workflow_timeline(
            &graph_from_str(
                r#"{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                    "phases": [{"id": "p1", "name": "一"}]}"#,
            ),
            None,
        );
        assert!(workflow_phases_detail(&one, None).contains('1'));
        let two = build_workflow_timeline(&graph, None);
        assert!(workflow_phases_detail(&two, Some(&graph)).contains('2'));
        // 两个 key 的文案相同（中文没有单复数），但都插上了数字。
        assert!(!count("chat.toolCall.workflow.card.phase", "chat.toolCall.workflow.card.phases", 1).is_empty());
    }

    // ── 表头细节 ──

    #[test]
    fn header_detail_says_only_phases_and_agents() {
        // ★真源 :91-96 —— 卡上不要出现「步」，只留阶段与子代理。
        let graph = sample_graph();
        let model = build_workflow_timeline(&graph, None);
        let detail = workflow_header_detail(&model, Some(&graph), None, None);
        assert!(detail.contains('2'), "两个阶段");
        assert!(detail.contains(" · "), "两段用间隔符连");
        assert!(!detail.contains("步"), "★不该说步");
        assert!(!detail.contains("token"));
    }

    #[test]
    fn header_detail_appends_model_name_last() {
        let graph = sample_graph();
        let model = build_workflow_timeline(&graph, None);
        let detail = workflow_header_detail(&model, Some(&graph), None, Some("sonnet"));
        assert!(detail.ends_with("sonnet"), "模型名跟在最后当最后一段");
    }

    #[test]
    fn header_detail_counts_working_agents_while_live() {
        let run = empty_run("running");
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let detail = workflow_header_detail(&model, None, Some(&run), None);
        assert!(detail.contains("工作中"), "跑着时说工作中");
        assert_eq!(
            run.actors
                .iter()
                .filter(|a| a.status == WorkflowRunActorStatus::Running)
                .count(),
            1
        );
    }

    #[test]
    fn header_detail_counts_total_agents_when_settled() {
        let run = empty_run("completed");
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let detail = workflow_header_detail(&model, None, Some(&run), None);
        assert!(!detail.contains("工作中"), "结束后数总数");
        assert!(detail.contains('3'), "投影里有 3 个 actor");
    }

    #[test]
    fn header_detail_takes_the_larger_of_projection_and_graph() {
        // 真源 :82 —— 投影与静态图取大（拆开的实例卡可能多于 actor 表里的行数）。
        let graph = sample_graph();
        let run = run_from_str(
            r#"{"runId": "r", "status": "completed", "actors": [],
                "nodes": [], "usage": {"spentTokens": 0, "nodesUsed": 0}}"#,
        );
        let model = build_workflow_timeline(&graph, Some(&run));
        let detail = workflow_header_detail(&model, Some(&graph), Some(&run), None);
        assert!(detail.contains('1'), "静态图那一条 agent 车道");
    }

    // ── workflowCardDetail ──

    #[test]
    fn card_detail_absent_without_model() {
        // 真源 :128-130 —— 建不出时间线模型时整块缺席。
        assert!(workflow_card_detail(None, None, None, None, None).is_none());
    }

    #[test]
    fn card_detail_has_no_title_without_subagent_model() {
        // 真源 :137 —— 没指定过模型的 run 缺席 tooltip。
        let graph = sample_graph();
        let model = build_workflow_timeline(&graph, None);
        let detail = workflow_card_detail(Some(&model), Some(&graph), None, None, None).unwrap();
        assert!(detail.title.is_none());
        assert!(detail.detail.contains('2'));
    }

    #[test]
    fn card_detail_carries_title_when_model_present() {
        let graph = sample_graph();
        let model = build_workflow_timeline(&graph, None);
        let detail = workflow_card_detail(
            Some(&model),
            Some(&graph),
            None,
            Some("anthropic/claude-sonnet-4"),
            None,
        )
        .unwrap();
        assert!(detail.title.is_some(), "规范串只进 tooltip");
        assert!(
            detail.detail.contains("claude-sonnet-4"),
            "细节串里是屏幕上的名字"
        );
    }

    // ── 摘要行 ──

    #[test]
    fn summary_parts_order_for_a_live_run() {
        let run = empty_run("running");
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let parts = workflow_summary_parts(&model, &run, SummaryOptions::default());
        assert_eq!(parts.len(), 3, "工作中 · 步 · tokens");
        assert!(parts[0].contains("工作中"));
        assert!(parts[1].contains('/'), "步是 done/total");
        assert!(parts[2].contains("tokens"));
    }

    #[test]
    fn summary_parts_put_model_first() {
        // ★真源 :156-158 —— 模型是摘要行的**第一段**：这一行本来就是「这条 run 的几个数」。
        let run = empty_run("running");
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let parts = workflow_summary_parts(
            &model,
            &run,
            SummaryOptions {
                subagent_model_name: Some("sonnet"),
                ..SummaryOptions::default()
            },
        );
        assert!(parts[0].contains("sonnet"));
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn summary_tokens_can_be_switched_off() {
        let run = empty_run("running");
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let parts = workflow_summary_parts(
            &model,
            &run,
            SummaryOptions {
                tokens: false,
                ..SummaryOptions::default()
            },
        );
        assert!(parts.iter().all(|p| !p.contains("token")));
    }

    #[test]
    fn summary_step_counts_include_unlisted_nodes() {
        // ★真源 :193-195 —— 步数走 @zcode/shared 的唯一实现：表内 + 表外。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok"},
                {"siteId": "s1", "ordinal": 1, "phase": "executing"}
              ], "usage": {"spentTokens": 0, "nodesUsed": 2,
                          "nodesUnlisted": 1976, "nodesUnlistedSettled": 1900}}"#,
        );
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let parts = workflow_summary_parts(&model, &run, SummaryOptions::default());
        assert!(parts.iter().any(|p| p.contains("1978")), "2 + 1976 = 1978 步");
        assert!(parts.iter().any(|p| p.contains("1901")), "1 + 1900 = 1901 已结算");
    }

    #[test]
    fn summary_omits_a_single_round() {
        // 真源 :205 —— rounds >= 2 才上表。
        let run = empty_run("running");
        let model = build_workflow_timeline(&sample_graph(), Some(&run));
        let parts = workflow_summary_parts(&model, &run, SummaryOptions::default());
        assert!(parts.iter().all(|p| !p.contains("轮")), "没循环 → 不说轮次");
    }

    #[test]
    fn summary_counts_artifacts_when_present() {
        // ★真源 :144-145,212 —— 末段数的是**产物**，不是 report 条目。
        let mut run = empty_run("completed");
        assert!(
            workflow_summary_parts(
                &build_workflow_timeline(&sample_graph(), Some(&run)),
                &run,
                SummaryOptions::default()
            )
            .iter()
            .all(|p| !p.contains("产物")),
            "零产物时整段缺席"
        );
        run.artifacts = Some(vec![
            serde_json::from_value(serde_json::json!({
                "id": "a1", "kind": "markdown", "version": 1
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "id": "a2", "kind": "chart", "version": 3
            }))
            .unwrap(),
        ]);
        let parts = workflow_summary_parts(
            &build_workflow_timeline(&sample_graph(), Some(&run)),
            &run,
            SummaryOptions::default(),
        );
        assert_eq!(parts.last().unwrap().contains('2'), true, "两个产物");
    }

    #[test]
    fn summary_singular_and_plural_use_distinct_keys() {
        // 两个 key 都存在且文案可渲染；单数走单数那条。
        let one = count(
            "chat.toolCall.workflow.card.agent",
            "chat.toolCall.workflow.card.agents",
            1,
        );
        let many = count(
            "chat.toolCall.workflow.card.agent",
            "chat.toolCall.workflow.card.agents",
            5,
        );
        assert!(one.contains('1'));
        assert!(many.contains('5'));
    }

    #[test]
    fn ink_still_reexported_for_callers() {
        // timeline_model的 TimelineInk 是墨迹词汇表；这里只确认它在摘要层可读。
        assert_eq!(TimelineInk::Faint.as_str(), "faint");
    }
}
