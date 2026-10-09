//! 1:1 翻译 `packages/ui/src/components/workflow-graph/phase-model.ts`（56 行）。
//!
//! 阶段层的纯选择器。
//!
//! ★真源 :9-15 注释——阶段是图**上面的一层商**：这里一个字都不改图本身，
//! 只按 `Step.phase` 把 step 分桶。第二层（参与者与交接）的选择器在
//! `participant_model.rs`。无 React、无 DOM。

use std::collections::HashMap;

use super::run_status::collapse_statuses;
use super::types::{
    StepRunStatus, StepStatusTable, WorkflowCausalityGraphData, WorkflowPhaseData,
    WorkflowStepData,
};

// 真源 `export { hasPhaseVocabulary }` 是再导出（phase-model.ts:17）——
// 该函数住在 participant-model.ts，Rust 侧对应 run_status.rs。
pub use super::run_status::has_phase_vocabulary;

/// `phaseMembers`（真源 :20-32）。
///
/// 阶段 id → 成员 step，按 `phases` 的顺序建桶
/// （**阶段表的顺序就是画面上的先后语义**）。跨阶段拷贝各自算它所在阶段的成员。
pub fn phase_members(
    graph: &WorkflowCausalityGraphData,
) -> HashMap<String, Vec<WorkflowStepData>> {
    let mut members: HashMap<String, Vec<WorkflowStepData>> = graph
        .phases
        .as_ref()
        .map(|phases| {
            phases
                .iter()
                .map(|p| (p.id.clone(), Vec::new()))
                .collect()
        })
        .unwrap_or_default();

    for step in &graph.steps {
        let Some(phase) = &step.phase else {
            continue;
        };
        if let Some(bucket) = members.get_mut(phase) {
            bucket.push(step.clone());
        }
    }
    members
}

/// `collapsePhaseStatus`（真源 :37-43）。
///
/// 成员 step 状态的再折叠。没有条目的成员不参与，
/// 一个条目都没有 = 静态渲染或整站没被观察到，返回 `None`。
pub fn collapse_phase_status(
    members: &[WorkflowStepData],
    statuses: Option<&StepStatusTable>,
) -> Option<StepRunStatus> {
    let ids: Vec<String> = members.iter().map(|s| s.id.clone()).collect();
    collapse_statuses(&ids, statuses)
}

/// `findPhase`（真源 :48-53）：按 id 取阶段。
///
/// 检视器与宿主要用它拿显示名素材。
pub fn find_phase<'a>(
    graph: &'a WorkflowCausalityGraphData,
    phase_id: &str,
) -> Option<&'a WorkflowPhaseData> {
    graph
        .phases
        .as_ref()?
        .iter()
        .find(|p| p.id == phase_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::types::StepRunStatus;

    /// 测试里写 graph 的简写：直接给 JSON 文本。
    ///
    /// ★不用 `json!` 宏 —— 嵌套对象字面量会被宏当成自己的参数解析
    /// （`{ "steps": [...] }` 里的 `{` 不是 json 的语法而是宏的分隔）。
    fn graph_from_str(raw: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn buckets_steps_by_phase_in_table_order() {
        // ★真源 :21-23 —— 阶段表的顺序就是画面上的先后语义。
        let graph = graph_from_str(r#"{"steps": [
                { "id": "s1", "kind": "ask", "label": "问A", "lane": "l1", "phase": "preflight" },
                { "id": "s2", "kind": "ask", "label": "问B", "lane": "l1", "phase": "main" },
                { "id": "s3", "kind": "ask", "label": "问C", "lane": "l1", "phase": "preflight" }
            ],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [
                { "id": "preflight", "name": "预备" },
                { "id": "main", "name": "主体" }
            ]}"#);
        let members = phase_members(&graph);
        assert_eq!(members.len(), 2);
        assert_eq!(members["preflight"].len(), 2, "声明序在前");
        assert_eq!(members["main"].len(), 1);
        // 桶内保持 steps 的声明序。
        assert_eq!(members["preflight"][0].id, "s1");
        assert_eq!(members["preflight"][1].id, "s3");
    }

    #[test]
    fn steps_without_phase_are_skipped() {
        // 真源 :29 —— 没有 phase 的 step 不进任何桶。
        let graph = graph_from_str(r#"{"steps": [{ "id": "s1", "kind": "ask", "label": "问", "lane": "l1" }],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [{ "id": "p1" }]}"#);
        let members = phase_members(&graph);
        assert_eq!(members["p1"].len(), 0, "无 phase 的 step 不入桶");
    }

    #[test]
    fn steps_for_unknown_phase_are_dropped() {
        // 桶只按 phases 建；phase 字段指向表外 id 时无处可放。
        let graph = graph_from_str(r#"{"steps": [
                { "id": "s1", "kind": "ask", "label": "问", "lane": "l1", "phase": "ghost" }
            ],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [{ "id": "p1" }]}"#);
        let members = phase_members(&graph);
        assert_eq!(members["p1"].len(), 0);
        assert_eq!(members.len(), 1, "只按 phases 建桶");
    }

    #[test]
    fn absent_phases_yields_empty_map() {
        // 零标记脚本的图没有 phases → 没有任何桶。
        let graph = graph_from_str(r#"{"steps": [{ "id": "s1", "kind": "ask", "label": "问", "lane": "l1" }],
            "lanes": [],
            "participants": [],
            "handoffs": []}"#);
        assert!(phase_members(&graph).is_empty());
    }

    #[test]
    fn zero_member_phase_stays_in_table() {
        // ★真源 :107-108 —— 零成员阶段也在表里。
        let graph = graph_from_str(r#"{"steps": [],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [{ "id": "empty" }, { "id": "full" }]}"#);
        let members = phase_members(&graph);
        assert_eq!(members.len(), 2, "零成员阶段也在表里");
        assert!(members["empty"].is_empty());
    }

    #[test]
    fn collapse_phase_status_folds_member_steps() {
        let graph = graph_from_str(r#"{"steps": [
                { "id": "s1", "kind": "ask", "label": "问A", "lane": "l1", "phase": "p" },
                { "id": "s2", "kind": "ask", "label": "问B", "lane": "l1", "phase": "p" }
            ],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [{ "id": "p" }]}"#);
        let members = phase_members(&graph);
        let ids = vec!["s1".to_string()];
        let statuses: StepStatusTable = ids
            .iter()
            .map(|k| (k.clone(), StepRunStatus::Running))
            .collect();
        assert_eq!(
            collapse_phase_status(&members["p"], Some(&statuses)),
            Some(StepRunStatus::Running)
        );
    }

    #[test]
    fn collapse_without_statuses_is_none() {
        let graph = graph_from_str(r#"{"steps": [{ "id": "s1", "kind": "ask", "label": "问", "lane": "l1", "phase": "p" }],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [{ "id": "p" }]}"#);
        let members = phase_members(&graph);
        assert_eq!(collapse_phase_status(&members["p"], None), None);
    }

    #[test]
    fn find_phase_by_id() {
        let graph = graph_from_str(r#"{"steps": [],
            "lanes": [],
            "participants": [],
            "handoffs": [],
            "phases": [{ "id": "preflight", "name": "预备" }]}"#);
        let found = find_phase(&graph, "preflight").unwrap();
        assert_eq!(found.name.as_deref(), Some("预备"));
        assert!(find_phase(&graph, "nope").is_none());
        // 没有 phases 表时返回 None。
        let bare = graph_from_str(r#"{"steps": [], "lanes": [], "participants": [], "handoffs": []}"#);
        assert!(find_phase(&bare, "preflight").is_none());
    }
}