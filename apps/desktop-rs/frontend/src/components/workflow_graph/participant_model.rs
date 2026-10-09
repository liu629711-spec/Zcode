//! 1:1 翻译 `packages/ui/src/components/workflow-graph/participant-model.ts`
//! （369 行）的**纯选择器部分**。
//!
//! ★真源 :12-19 注释——参与者层的纯选择器。载荷的 `participants` 已经是
//! 分析器排好的**交接序**（第一张是开局者），`handoffs` 已经归约过；
//! 这里只做分桶、状态折叠、计数，以及 UI 自己的两件事：
//! - 无标记脚本合成一个隐式阶段（`with_implicit_phase`）；
//! - 运行中把 `many` 卡按真实实例拆开（`live_participant_view`）。
//!
//! **裁剪注明**——`liveParticipantView` / `phaseBinder` / `runIndex` 依赖
//! `WorkflowRunState` 协议类型（actors / nodes / phaseName / siteId），
//! 该类型在协议层尚未迁入，故本模块只落不依赖它的纯选择器；
//! 待协议类型落地后按文件末 TODO 补齐（与 `run_status.rs` 的同款裁剪对齐）。

use std::collections::HashMap;

use super::run_status::collapse_statuses;
use super::types::{
    IMPLICIT_PHASE_ID, StepRunStatus, StepStatusTable, WorkflowCausalityGraphData,
    WorkflowHandoffData, WorkflowParticipantData, WorkflowPhaseData,
};

pub use super::run_status::has_phase_vocabulary;

/// `withImplicitPhase`（真源 :34-47）。
///
/// 无 `phase()` 标记的脚本 → 一个隐式模块：阶段表只有 `workflow`
/// （无 name，UI 本地化为「Workflow」），没有阶段边，
/// 控制流到达正常完成**当且仅当**脚本有返回物；
/// 每个 step 与每张卡归入它（分析器给它们的 `unphased` 只是
/// 「没有阶段」的占位，**卡 id 不变**）。
///
/// 有词汇表的图**原样返回**（引用相等，memo 友好）。
pub fn with_implicit_phase(graph: &WorkflowCausalityGraphData) -> WorkflowCausalityGraphData {
    if has_phase_vocabulary(graph) {
        return graph.clone();
    }
    let mut out = graph.clone();
    // 真源 :39 —— exits：有返回物才有一个可正常完成的阶段。
    out.exits = Some(
        if graph.sink.as_ref().is_some_and(|s| !s.is_empty()) {
            vec![IMPLICIT_PHASE_ID.to_string()]
        } else {
            Vec::new()
        },
    );
    out.participants = graph
        .participants
        .iter()
        .map(|p| {
            let mut p = p.clone();
            p.phase = IMPLICIT_PHASE_ID.to_string();
            p
        })
        .collect();
    out.phase_edges = Some(Vec::new());
    out.phases = Some(vec![WorkflowPhaseData {
        id: IMPLICIT_PHASE_ID.to_string(),
        name: None,
        line: None,
        column: None,
        alongside: None,
    }]);
    out.steps = graph
        .steps
        .iter()
        .map(|s| {
            let mut s = s.clone();
            s.phase = Some(IMPLICIT_PHASE_ID.to_string());
            s
        })
        .collect();
    out
}

/// `participantsOfPhase`（真源 :50-56）：一个阶段的参与者，保持载荷顺序（= 交接序）。
pub fn participants_of_phase(
    graph: &WorkflowCausalityGraphData,
    phase_id: &str,
) -> Vec<WorkflowParticipantData> {
    graph
        .participants
        .iter()
        .filter(|p| p.phase == phase_id)
        .cloned()
        .collect()
}

/// `participantById`（真源 :58-63）。
pub fn participant_by_id<'a>(
    graph: &'a WorkflowCausalityGraphData,
    id: &str,
) -> Option<&'a WorkflowParticipantData> {
    graph.participants.iter().find(|p| p.id == id)
}

/// `handoffsWithin`（真源 :65-71）：两端都在给定集合里的交接边（阶段内部的边）。
pub fn handoffs_within(
    graph: &WorkflowCausalityGraphData,
    ids: &std::collections::HashSet<String>,
) -> Vec<WorkflowHandoffData> {
    graph
        .handoffs
        .iter()
        .filter(|e| ids.contains(&e.from) && ids.contains(&e.to))
        .cloned()
        .collect()
}

/// `handoffsAround`（真源 :73-85）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffsAround {
    pub incoming: Vec<WorkflowHandoffData>,
    pub outgoing: Vec<WorkflowHandoffData>,
}

pub fn handoffs_around(
    graph: &WorkflowCausalityGraphData,
    id: &str,
) -> HandoffsAround {
    HandoffsAround {
        incoming: graph.handoffs.iter().filter(|e| e.to == id).cloned().collect(),
        outgoing: graph.handoffs.iter().filter(|e| e.from == id).cloned().collect(),
    }
}

/// `ParticipantCounts`（真源 :87-91）：卡片次行的计数素材。
///
/// ★真源 :87 ——ask 数与工作区读取数（**一张卡只会有其中一种非零**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParticipantCounts {
    pub asks: i64,
    pub reads: i64,
}

/// `participantCounts`（真源 :93-105）。
pub fn participant_counts(
    graph: &WorkflowCausalityGraphData,
    participant: &WorkflowParticipantData,
) -> ParticipantCounts {
    let kinds: HashMap<&str, &str> = graph
        .steps
        .iter()
        .map(|s| (s.id.as_str(), s.kind.as_str()))
        .collect();
    let mut asks = 0;
    let mut reads = 0;
    for id in &participant.steps {
        if kinds.get(id.as_str()) == Some(&"world-read") {
            reads += 1;
        } else {
            asks += 1;
        }
    }
    ParticipantCounts { asks, reads }
}

/// `participantRepeats`（真源 :107-118）。
///
/// ★真源 :107 注释——任一 step 带 `repeat` ⇒ 卡片显示重复图元
/// （自环在参与者粒度上消失，这是它留下的线索）。
pub fn participant_repeats(
    graph: &WorkflowCausalityGraphData,
    participant: &WorkflowParticipantData,
) -> bool {
    let repeating: std::collections::HashSet<&str> = graph
        .steps
        .iter()
        .filter(|s| s.repeat.is_some())
        .map(|s| s.id.as_str())
        .collect();
    participant.steps.iter().any(|id| repeating.contains(id.as_str()))
}

/// `participantStatus`（真源 :128-134）。
///
/// ★真源 :121-122 注释——实时视图给出的按实例收窄的覆盖值优先
/// （成员卡 / 拆分出的实例卡），否则由它的 step 折叠。
pub fn participant_status(
    participant: &WorkflowParticipantData,
    statuses: Option<&StepStatusTable>,
    participant_statuses: Option<&HashMap<String, StepRunStatus>>,
) -> Option<StepRunStatus> {
    participant_statuses
        .and_then(|m| m.get(&participant.id).copied())
        .or_else(|| collapse_statuses(&participant.steps, statuses))
}

/// `participantAlsoIn`（真源 :136-149）。
///
/// 同一条车道还出现在哪些其他阶段（检视器「also in」一行）。
pub fn participant_also_in(
    graph: &WorkflowCausalityGraphData,
    participant: &WorkflowParticipantData,
) -> Vec<WorkflowPhaseData> {
    let phases: std::collections::HashSet<&str> = graph
        .participants
        .iter()
        .filter(|o| o.lane == participant.lane && o.phase != participant.phase)
        .map(|o| o.phase.as_str())
        .collect();
    match graph.phases.as_ref() {
        Some(ps) => ps
            .iter()
            .filter(|p| phases.contains(p.id.as_str()))
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}

/// `instanceCardId`（真源 :151-154）：拆分出的实例卡 id。
pub fn instance_card_id(participant_id: &str, ordinal: i64) -> String {
    format!("{participant_id}@{ordinal}")
}

/// `ParticipantInstance`（真源 :156-181）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantInstance {
    /// 拆分前的参与者 id（绑定时就是这张卡自己的 id）。
    pub participant: String,
    pub ordinal: i64,
    /// 名字就是引擎发出的有效名，卡面优先念它。
    pub name: Option<String>,
    /// ★真源 :172-176 ——原卡绑定而非拆分：卡 id 没变，标签规则也不变
    /// （成员卡仍 `#i`，单卡无标签）——运行时名不做视觉标记。
    /// `None` = 拆分出的实例卡，标签念 `#ordinal`。
    pub bound: bool,
}

// TODO(后续迁移)：以下真源部分依赖 `WorkflowRunState` 协议类型，
// 待该类型从协议层迁入后补齐（与 run_status.rs 的 workflowRunOverlay 同款裁剪）。
// - `liveParticipantView`（真源 :220-369）：把 many 卡按真实实例拆开、
//   成员卡按 ordinal 收窄、单卡绑定唯一实例。约 150 行，是这个模块的主体。
// - `phaseBinder` / `phasesOf` / `runHasPhaseVocabulary`（instance-phases.ts:21-72）：
//   一个戳的归属阶段集——有戳按名字匹配、无戳且 run 有词汇则落无名阶段、
//   任一分支结果为空则全阶段（「宁可重复显示，也不把一个在跑的子代理藏起来」）。
// - `runIndex` / `actorKey` / `statusOfRunNode`（run-status.ts:83-174）。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::run_status::aggregate_run_statuses;

    fn graph_from_str(raw: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(raw).unwrap()
    }

    fn bare_graph() -> WorkflowCausalityGraphData {
        graph_from_str(
            r#"{"steps": [
                { "id": "s1", "kind": "ask", "label": "问A", "lane": "agent-1" },
                { "id": "s2", "kind": "world-read", "label": "读文件", "lane": "agent-1" },
                { "id": "s3", "kind": "ask", "label": "问B", "lane": "agent-2", "repeat": "serial" }
            ], "lanes": [], "handoffs": [],
            "participants": [
                { "id": "p1", "phase": "unphased", "lane": "agent-1", "steps": ["s1", "s2"] },
                { "id": "p2", "phase": "unphased", "lane": "agent-2", "steps": ["s3"] }
            ],
            "sink": ["s1"]}"#,
        )
    }

    // ── 隐式阶段 ──

    #[test]
    fn implicit_phase_created_when_no_vocabulary() {
        // ★真源 :38-47 —— 无标记脚本合成一个隐式模块 workflow。
        let graph = bare_graph();
        assert!(!has_phase_vocabulary(&graph));
        let out = with_implicit_phase(&graph);
        assert!(has_phase_vocabulary(&out));
        assert_eq!(out.phases.as_ref().unwrap().len(), 1);
        assert_eq!(out.phases.as_ref().unwrap()[0].id, IMPLICIT_PHASE_ID);
        // ★真源 :39 —— 有返回物才有一个可正常完成的阶段。
        assert_eq!(out.exits.as_deref(), Some(&[IMPLICIT_PHASE_ID.to_string()][..]));
    }

    #[test]
    fn implicit_phase_exits_empty_without_sink() {
        let mut graph = bare_graph();
        graph.sink = None;
        let out = with_implicit_phase(&graph);
        assert_eq!(
            out.exits.as_deref(),
            Some(&[][..]),
            "无返回物 → 没有可正常完成的阶段"
        );
    }

    #[test]
    fn implicit_phase_reassigns_steps_and_cards_but_keeps_ids() {
        // ★真源 :41-46 —— 分析器给的 unphased 只是占位，**卡 id 不变**。
        let out = with_implicit_phase(&bare_graph());
        assert!(out.steps.iter().all(|s| s.phase.as_deref() == Some(IMPLICIT_PHASE_ID)));
        assert!(out
            .participants
            .iter()
            .all(|p| p.phase == IMPLICIT_PHASE_ID));
        assert_eq!(out.participants[0].id, "p1", "卡 id 不变");
        assert_eq!(out.participants[1].id, "p2");
        assert_eq!(out.steps[0].id, "s1", "step id 不变");
    }

    #[test]
    fn implicit_phase_no_phase_edges() {
        let out = with_implicit_phase(&bare_graph());
        assert_eq!(
            out.phase_edges.as_deref(),
            Some(&[][..]),
            "隐式模块没有阶段边"
        );
    }

    #[test]
    fn graph_with_vocabulary_returned_unchanged() {
        // 真源 :47 —— 有词汇表的图原样返回（引用相等，memo 友好）。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [], "participants": [],
                "phases": [{"id": "preflight", "name": "预备"}]}"#,
        );
        let out = with_implicit_phase(&graph);
        assert_eq!(out, graph, "有词汇表时不该改");
    }

    // ── 选择器 ──

    #[test]
    fn participants_of_phase_keep_payload_order() {
        // 真源 :52-55 —— 保持载荷顺序（= 交接序）。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [],
                "participants": [
                    { "id": "p1", "phase": "main", "lane": "a", "steps": ["s1"] },
                    { "id": "p2", "phase": "pre", "lane": "a", "steps": ["s2"] },
                    { "id": "p3", "phase": "main", "lane": "a", "steps": ["s3"] }
                ],
                "phases": [{"id": "main"}, {"id": "pre"}]}"#,
        );
        let main = participants_of_phase(&graph, "main");
        assert_eq!(main.len(), 2);
        assert_eq!(main[0].id, "p1");
        assert_eq!(main[1].id, "p3", "保持声明序");
    }

    #[test]
    fn participant_by_id_finds_or_none() {
        let graph = bare_graph();
        assert_eq!(participant_by_id(&graph, "p1").unwrap().lane, "agent-1");
        assert!(participant_by_id(&graph, "nope").is_none());
    }

    #[test]
    fn handoffs_within_filters_both_ends() {
        // 真源 :67-69 —— 两端都在集合里（阶段内部的边）。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [],
                "handoffs": [
                    { "from": "p1", "to": "p2" },
                    { "from": "p2", "to": "p1" },
                    { "from": "p1", "to": "p9" }
                ]}"#,
        );
        let ids: std::collections::HashSet<String> =
            ["p1".to_string(), "p2".to_string()].into_iter().collect();
        let within = handoffs_within(&graph, &ids);
        assert_eq!(within.len(), 2, "p1→p9 有一端在外");
    }

    #[test]
    fn handoffs_around_splits_in_out() {
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [],
                "handoffs": [
                    { "from": "p1", "to": "p2" },
                    { "from": "p2", "to": "p3" },
                    { "from": "p1", "to": "p3" }
                ]}"#,
        );
        let around = handoffs_around(&graph, "p2");
        assert_eq!(around.incoming.len(), 1);
        assert_eq!(around.incoming[0].from, "p1");
        assert_eq!(around.outgoing.len(), 1);
        assert_eq!(around.outgoing[0].to, "p3");
    }

    // ── 计数 ──

    #[test]
    fn counts_split_asks_and_reads() {
        // ★真源 :87 —— 一张卡只会有其中一种非零。
        let graph = bare_graph();
        let p1 = participant_by_id(&graph, "p1").unwrap();
        let counts = participant_counts(&graph, p1);
        assert_eq!(counts.asks, 1, "s1 是 ask");
        assert_eq!(counts.reads, 1, "s2 是 world-read");
    }

    #[test]
    fn counts_treat_unknown_kind_as_ask() {
        // 真源 :102 —— 只有 world-read 记reads，其余都记 asks。
        let graph = graph_from_str(
            r#"{"steps": [
                { "id": "s1", "kind": "mystery", "label": "?", "lane": "a" }
            ], "lanes": [], "handoffs": [],
            "participants": [{ "id": "p1", "phase": "x", "lane": "a", "steps": ["s1"] }]}"#,
        );
        let p = participant_by_id(&graph, "p1").unwrap();
        let counts = participant_counts(&graph, p);
        assert_eq!(counts.asks, 1);
        assert_eq!(counts.reads, 0);
    }

    #[test]
    fn repeat_flag_marks_card() {
        // ★真源 :107 —— 任一 step 带 repeat ⇒ 卡片显示重复图元。
        let graph = bare_graph();
        assert!(!participant_repeats(&graph, participant_by_id(&graph, "p1").unwrap()));
        assert!(participant_repeats(&graph, participant_by_id(&graph, "p2").unwrap()));
    }

    // ── 状态 ──

    #[test]
    fn participant_status_prefers_instance_override() {
        // ★真源 :121-122 —— 实时视图的按实例收窄覆盖值优先。
        let graph = bare_graph();
        let p = participant_by_id(&graph, "p1").unwrap();
        let statuses: StepStatusTable = [("s1".to_string(), StepRunStatus::Done)]
            .into_iter()
            .collect();
        let overrides: HashMap<String, StepRunStatus> =
            [("p1".to_string(), StepRunStatus::Running)].into_iter().collect();
        assert_eq!(
            participant_status(p, Some(&statuses), Some(&overrides)),
            Some(StepRunStatus::Running),
            "覆盖值优先于step 折叠"
        );
        // 没有覆盖时回落 step 折叠。
        assert_eq!(
            participant_status(p, Some(&statuses), None),
            Some(StepRunStatus::Done)
        );
        // 都没有 → None。
        assert_eq!(participant_status(p, None, None), None);
    }

    #[test]
    fn participant_also_in_lists_other_phases_of_same_lane() {
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [],
                "participants": [
                    { "id": "p1", "phase": "main", "lane": "agent-1", "steps": ["s1"] },
                    { "id": "p2", "phase": "pre", "lane": "agent-1", "steps": ["s2"] },
                    { "id": "p3", "phase": "post", "lane": "agent-2", "steps": ["s3"] }
                ],
                "phases": [{"id": "pre"}, {"id": "main"}, {"id": "post"}]}"#,
        );
        let p1 = participant_by_id(&graph, "p1").unwrap();
        let also = participant_also_in(&graph, p1);
        assert_eq!(also.len(), 1);
        assert_eq!(also[0].id, "pre", "同车道另一个阶段");
        // 不同车道不进列表。
        assert_eq!(also[0].id, "pre");
    }

    #[test]
    fn instance_card_id_format() {
        // 真源 :153 —— `${participant.id}@${ordinal}`。
        assert_eq!(instance_card_id("p1", 3), "p1@3");
    }

    #[test]
    fn aggregate_statuses_still_reachable_from_here() {
        // 再导出检查：participant-model 的 collapseStatuses 已搬到 run_status.rs，
        // 这里确认两条路径给出同一答案。
        let ids = vec!["a".to_string(), "b".to_string()];
        let statuses: StepStatusTable = [
            ("a".to_string(), StepRunStatus::Pending),
            ("b".to_string(), StepRunStatus::Done),
        ]
        .into_iter()
        .collect();
        assert_eq!(collapse_statuses(&ids, Some(&statuses)), Some(StepRunStatus::Running));
        assert_eq!(
            aggregate_run_statuses(&[StepRunStatus::Pending, StepRunStatus::Done]),
            Some(StepRunStatus::Running),
            "两条路径一致"
        );
    }
}