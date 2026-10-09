//! 1:1 翻译 `packages/ui/src/components/workflow-graph/run-status.ts`（174 行）
//! 的**状态折叠部分**，以及 `participant-model.ts` 的 `collapseStatuses` /
//! `hasPhaseVocabulary`。
//!
//! ★裁剪注明——本模块只落「从一组 step 状态折成一个」的纯函数；
//! `run-status.ts` 后半的 `workflowRunOverlay`（把运行投影叠到静态图上，
//! 涉及 `WorkflowRunState` 协议类型与参与者卡归属）待协议类型迁入时补齐，
//! 见文件末 TODO。

use std::collections::HashMap;

use super::run_state::{
    WorkflowRunActor, WorkflowRunNode, WorkflowRunNodeOutcome, WorkflowRunNodePhase, WorkflowRunState,
};
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


// ---------------------------------------------------------------------------
// 引擎相位 → 四值StepRunStatus（真源 run-status.ts:43-57）
// ---------------------------------------------------------------------------

/// `statusOfRunNode`（真源 :43-57）。
///
/// ★真源 :37-42 注释——词汇表按**引擎实际发出的**事件写。
/// queued / dispatched / waiting 归入 pending 是**有意的**：这三段都是
/// 「还没有请求在 provider 那里跑」（FIFO 与 per-run 上限的等待、
/// 会话就绪但首个请求尚未准入、闸门排队或退避）。真正在动由executing 说。
pub fn status_of_run_node(node: &WorkflowRunNode) -> StepRunStatus {
    match node.phase {
        WorkflowRunNodePhase::Executing
        | WorkflowRunNodePhase::Repairing
        | WorkflowRunNodePhase::Nudged => StepRunStatus::Running,
        WorkflowRunNodePhase::Settled => {
            // ★真源 :50-52 —— 失败与取消都画成 failed（journal 里两者语义不同，
            // 但叠加视图只用四值词汇表）。outcome 缺省在引擎里不可达（settled 必带
            // outcome）；真出现时按「已结束」处理，因为谎报 pending（没开始）
            // 比少一格颜色更糟，而谎报 failed 会造成假警报。
            match node.outcome {
                Some(WorkflowRunNodeOutcome::Failed) | Some(WorkflowRunNodeOutcome::Cancelled) => {
                    StepRunStatus::Failed
                }
                _ => StepRunStatus::Done,
            }
        }
        _ => StepRunStatus::Pending,
    }
}

/// `WorkflowRunOverlay`（真源 :23-33）：运行投影叠到静态图上的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunOverlay {
    /// 按图上的 step id 索引的状态。**偏表**：没有观察到实例的 step 没有条目。
    pub statuses: StepStatusTable,
    /// 从 running 的 step 出发的排序边是否动画。
    pub animated_edges: bool,
}

/// actor 的稳定键（真源 participant-model 的 `actorKey`，真源同义）。
fn actor_key(lane: &str, ordinal: i64) -> String {
    format!("{lane}#{ordinal}")
}

/// `runIndex`（真源 participant-model 的私有函数）：一次视图算一遍的索引。
///
/// 整条车道只排一次序，与原来「先筛后排」同序——排序稳定，筛选保序。
#[derive(Debug, Clone, Default)]
pub struct RunIndex {
    /// lane → 该车道上的 actor（按出生戳稳定排序）。
    pub actors_by_site: HashMap<String, Vec<WorkflowRunActor>>,
    /// `lane#ordinal` → 该 actor 的节点。
    pub nodes_by_actor: HashMap<String, Vec<WorkflowRunNode>>,
}

/// `instance-phases.ts` 的 `runHasPhaseVocabulary`（真源 :15-20）。
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

/// 建索引（真源 participant-model 的 `runIndex`）。
pub fn build_run_index(run: &WorkflowRunState) -> RunIndex {
    let mut index = RunIndex::default();
    for actor in &run.actors {
        index
            .actors_by_site
            .entry(actor.site_id.clone())
            .or_default()
            .push(actor.clone());
    }
    for actor in &run.actors {
        let nodes: Vec<WorkflowRunNode> = run
            .nodes
            .iter()
            .filter(|n| {
                n.actor_site_id.as_deref() == Some(actor.site_id.as_str())
                    && n.actor_ordinal == Some(actor.ordinal)
            })
            .cloned()
            .collect();
        // 无节点时不开key（真源用 Map.get 查，开不开都能查到空，
        // 但不开更贴近「只索引有事实的条目」）。
        if !nodes.is_empty() {
            index
                .nodes_by_actor
                .insert(actor_key(&actor.site_id, actor.ordinal), nodes);
        }
    }
    index
}

/// `actorKey` 的公开形式（participant-model 的节点收窄要用）。
pub fn actor_key_of(lane: &str, ordinal: i64) -> String {
    actor_key(lane, ordinal)
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

    // ── 引擎相位 → 四值 ──

    fn node(phase: WorkflowRunNodePhase, outcome: Option<WorkflowRunNodeOutcome>) -> WorkflowRunNode {
        WorkflowRunNode {
            site_id: "ask".into(),
            ordinal: 0,
            kind: None,
            phase,
            outcome,
            actor_site_id: None,
            actor_ordinal: None,
            phase_name: None,
            turn: None,
            tool_calls: None,
            last_tool: None,
        }
    }

    #[test]
    fn moving_phases_are_running() {
        // ★真源 :45-48 —— executing / repairing / nudged → running。
        for p in [
            WorkflowRunNodePhase::Executing,
            WorkflowRunNodePhase::Repairing,
            WorkflowRunNodePhase::Nudged,
        ] {
            assert_eq!(status_of_run_node(&node(p, None)), StepRunStatus::Running);
        }
    }

    #[test]
    fn waiting_phases_are_pending() {
        // ★真源 :37-42 —— queued / dispatched / waiting 归入 pending 是有意的：
        // 这三段都是「还没有请求在 provider 那里跑」。
        for p in [
            WorkflowRunNodePhase::Queued,
            WorkflowRunNodePhase::Dispatched,
            WorkflowRunNodePhase::Waiting,
        ] {
            assert_eq!(
                status_of_run_node(&node(p, None)),
                StepRunStatus::Pending,
                "{p:?} 应算 pending"
            );
        }
    }

    #[test]
    fn settled_ok_is_done() {
        assert_eq!(
            status_of_run_node(&node(WorkflowRunNodePhase::Settled, Some(WorkflowRunNodeOutcome::Ok))),
            StepRunStatus::Done
        );
    }

    #[test]
    fn settled_failed_and_cancelled_both_failed() {
        // ★真源 :50 —— 失败与取消都画成 failed（journal 里语义不同，
        // 但叠加视图只用四值词汇表）。
        for o in [WorkflowRunNodeOutcome::Failed, WorkflowRunNodeOutcome::Cancelled] {
            assert_eq!(
                status_of_run_node(&node(WorkflowRunNodePhase::Settled, Some(o))),
                StepRunStatus::Failed,
                "{o:?} 应画成 failed"
            );
        }
    }

    #[test]
    fn settled_without_outcome_is_done_not_pending() {
        // ★真源 :51-52 —— outcome 缺省在引擎里不可达；真出现时按「已结束」处理，
        // 因为谎报 pending（没开始）比少一格颜色更糟，而谎报 failed 会造成假警报。
        assert_eq!(
            status_of_run_node(&node(WorkflowRunNodePhase::Settled, None)),
            StepRunStatus::Done
        );
    }

    // ── run 索引 ──

    fn run_with_actors() -> super::WorkflowRunState {
        use crate::components::workflow_graph::run_state::{
            WorkflowRunActorStatus, WorkflowRunStatus,
        };
        super::WorkflowRunState {
            run_id: "r1".into(),
            tool_call_id: None,
            status: WorkflowRunStatus::Running,
            actors: vec![
                WorkflowRunActor {
                    site_id: "ask".into(),
                    ordinal: 0,
                    name: None,
                    session_id: None,
                    status: WorkflowRunActorStatus::Running,
                    phase_name: Some("预备".into()),
                },
                WorkflowRunActor {
                    site_id: "ask".into(),
                    ordinal: 1,
                    name: None,
                    session_id: None,
                    status: WorkflowRunActorStatus::Waiting,
                    phase_name: None,
                },
            ],
            nodes: vec![node(WorkflowRunNodePhase::Executing, None)],
            phases: None,
            current_phase: None,
            phase_names: None,
            phase_alongside: None,
        }
    }

    #[test]
    fn run_index_groups_actors_by_site() {
        let index = build_run_index(&run_with_actors());
        assert_eq!(index.actors_by_site.get("ask").unwrap().len(), 2);
        assert_eq!(index.actors_by_site.len(), 1, "同站点合到一组");
    }

    #[test]
    fn run_index_associates_nodes_by_actor_key() {
        // 节点没带 actor_siteId → 不该被认领。
        let index = build_run_index(&run_with_actors());
        assert!(
            index.nodes_by_actor.is_empty(),
            "无 actor 归属的节点不进索引"
        );
    }

    #[test]
    fn actor_key_is_lane_plus_ordinal() {
        assert_eq!(actor_key_of("ask", 2), "ask#2");
    }

    #[test]
    fn run_phase_vocabulary_needs_a_stamp() {
        // ★真源 :15-20 —— 任一 actor / 节点带戳才算有词汇。
        assert!(run_has_phase_vocabulary(Some(&run_with_actors())));
        assert!(!run_has_phase_vocabulary(None), "无 run → 无词汇");

        let mut bare = run_with_actors();
        bare.actors.iter_mut().for_each(|a| a.phase_name = None);
        assert!(!run_has_phase_vocabulary(Some(&bare)));
    }
}
