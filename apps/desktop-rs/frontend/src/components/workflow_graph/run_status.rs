//! 1:1 翻译 `packages/ui/src/components/workflow-graph/run-status.ts`（174 行）
//! 的**状态折叠部分**，以及 `participant-model.ts` 的 `collapseStatuses` /
//! `hasPhaseVocabulary`。
//!
//! ★裁剪注明——本模块只落「从一组 step 状态折成一个」的纯函数；
//! `run-status.ts` 后半的 `workflowRunOverlay`（把运行投影叠到静态图上，
//! 涉及 `WorkflowRunState` 协议类型与参与者卡归属）待协议类型迁入时补齐，
//! 见文件末 TODO。

use super::types::{StepRunStatus, StepStatusTable, WorkflowCausalityGraphData};

/// `aggregateRunStatuses`（真源 run-status.ts:72-81）。
///
/// 多个 step 状态的折叠。规则（照抄）：
/// 1. 空 → `None`（一个条目都没有 = 静态渲染或整站没被观察到）
/// 2. 有 running → running（**优先于一切**）
/// 3. 有 pending 且有结算（done/failed）→ running（有东西在动就报running）
/// 4. 只有 pending → pending
/// 5. 有 failed → failed
/// 6. 其余 → done
pub fn aggregate_run_statuses(statuses: &[StepRunStatus]) -> Option<StepRunStatus> {
    if statuses.is_empty() {
        return None;
    }
    if statuses.contains(&StepRunStatus::Running) {
        return Some(StepRunStatus::Running);
    }
    let queued = statuses.contains(&StepRunStatus::Pending);
    let settled = statuses
        .iter()
        .any(|s| matches!(s, StepRunStatus::Done | StepRunStatus::Failed));
    if queued {
        return Some(if settled {
            StepRunStatus::Running
        } else {
            StepRunStatus::Pending
        });
    }
    Some(if statuses.contains(&StepRunStatus::Failed) {
        StepRunStatus::Failed
    } else {
        StepRunStatus::Done
    })
}

/// `collapseStatuses`（真源 participant-model.ts:123-134）。
///
/// ★真源 :119-122 注释——收集名下**存在条目**的 step 的状态：
/// 没有条目的成员不参与（缺席 ≠ pending，见 `types.rs` 的 StepStatusTable 注释）。
pub fn collapse_statuses(
    step_ids: &[String],
    statuses: Option<&StepStatusTable>,
) -> Option<StepRunStatus> {
    let statuses = statuses?;
    let values: Vec<StepRunStatus> = step_ids
        .iter()
        .filter_map(|id| statuses.get(id).copied())
        .collect();
    aggregate_run_statuses(&values)
}

/// `hasPhaseVocabulary`（真源 participant-model.ts:29-31）。
///
/// 图自己有阶段词汇表吗（`phases` 存在且非空）。
pub fn has_phase_vocabulary(graph: &WorkflowCausalityGraphData) -> bool {
    graph
        .phases
        .as_ref()
        .is_some_and(|phases| !phases.is_empty())
}

// TODO(后续迁移)：run-status.ts 后半的 `workflowRunOverlay`（:83-174）——
// 把运行投影叠到静态图上。依赖 `WorkflowRunState` 协议类型
// （actors / nodes / phaseName / mailbox）与参与者卡归属判定，
// 待协议类型迁入后补齐。当前只落了不依赖协议的状态折叠。

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const PENDING: StepRunStatus = StepRunStatus::Pending;
    const RUNNING: StepRunStatus = StepRunStatus::Running;
    const DONE: StepRunStatus = StepRunStatus::Done;
    const FAILED: StepRunStatus = StepRunStatus::Failed;

    #[test]
    fn empty_is_none() {
        // 真源 :75 —— 一个条目都没有 = 静态渲染或整站没被观察到。
        assert_eq!(aggregate_run_statuses(&[]), None);
    }

    #[test]
    fn running_wins_over_everything() {
        // ★真源 :76 —— 有 running 就报 running，优先于一切。
        assert_eq!(
            aggregate_run_statuses(&[PENDING, RUNNING, DONE, FAILED]),
            Some(RUNNING)
        );
        assert_eq!(aggregate_run_statuses(&[RUNNING]), Some(RUNNING));
    }

    #[test]
    fn pending_plus_settled_becomes_running() {
        // ★真源 :77-79 —— 有 pending 且有结算 → running（有东西在动）。
        assert_eq!(
            aggregate_run_statuses(&[PENDING, DONE]),
            Some(RUNNING)
        );
        assert_eq!(
            aggregate_run_statuses(&[PENDING, FAILED]),
            Some(RUNNING),
            "排队中且已有失败 → 仍在动"
        );
    }

    #[test]
    fn pending_alone_stays_pending() {
        assert_eq!(aggregate_run_statuses(&[PENDING]), Some(PENDING));
        assert_eq!(
            aggregate_run_statuses(&[PENDING, PENDING]),
            Some(PENDING)
        );
    }

    #[test]
    fn failed_beats_done() {
        // 真源 :80 —— 有 failed 报 failed。
        assert_eq!(aggregate_run_statuses(&[DONE, FAILED]), Some(FAILED));
        assert_eq!(aggregate_run_statuses(&[FAILED]), Some(FAILED));
    }

    #[test]
    fn all_done_is_done() {
        assert_eq!(aggregate_run_statuses(&[DONE, DONE]), Some(DONE));
        assert_eq!(aggregate_run_statuses(&[DONE]), Some(DONE));
    }

    // ── collapse ──

    fn table(pairs: &[(&str, StepRunStatus)]) -> StepStatusTable {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<HashMap<_, _>>()
    }

    #[test]
    fn no_status_table_is_none() {
        // 真源 :127 —— statuses 为 undefined 直接 None。
        let ids = vec!["s1".to_string()];
        assert_eq!(collapse_statuses(&ids, None), None);
    }

    #[test]
    fn members_without_entries_do_not_participate() {
        // ★真源 :119-122 —— 没有条目的成员不参与。
        //缺席 ≠ pending（见 types.rs 的 StepStatusTable 注释）。
        let ids = vec!["s1".to_string(), "s2".to_string()];
        let statuses = table(&[("s1", DONE)]);
        assert_eq!(
            collapse_statuses(&ids, Some(&statuses)),
            Some(DONE),
            "只有 s1 有条目"
        );
    }

    #[test]
    fn table_with_no_matching_entries_is_none() {
        let ids = vec!["s9".to_string()];
        let statuses = table(&[("s1", DONE)]);
        assert_eq!(collapse_statuses(&ids, Some(&statuses)), None);
    }

    #[test]
    fn collapse_applies_aggregate_rules() {
        let ids = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let statuses = table(&[("a", PENDING), ("b", RUNNING), ("c", DONE)]);
        assert_eq!(collapse_statuses(&ids, Some(&statuses)), Some(RUNNING));
    }

    // ── 阶段词汇 ──

    #[test]
    fn phase_vocabulary_requires_non_empty_phases() {
        // 真源 :29-31 —— phases 存在且非空才算有词汇表。
        let mut graph = WorkflowCausalityGraphData {
            steps: Vec::new(),
            lanes: Vec::new(),
            participants: Vec::new(),
            handoffs: Vec::new(),
            phases: None,
            phase_edges: None,
            exits: None,
            sink: None,
            truncated: None,
        };
        assert!(!has_phase_vocabulary(&graph), "None = 无词汇");

        graph.phases = Some(Vec::new());
        assert!(!has_phase_vocabulary(&graph), "空数组 = 无词汇");

        graph.phases = Some(vec![super::super::types::WorkflowPhaseData {
            id: "preflight".into(),
            name: Some("预备".into()),
            line: None,
            column: None,
            alongside: None,
        }]);
        assert!(has_phase_vocabulary(&graph), "有阶段 = 有词汇");
    }
}