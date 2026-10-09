//! 1:1 翻译 `packages/ui/src/components/workflow-graph/run-status.ts`（174 行）。
//!
//! 实时叠加视图（live view v1）的纯选择器：引擎相位 → 四值状态、唯一的折叠、
//! `workflowRunOverlay` 的收状态、`workflowRunActorsForLane` 的下钻。
//! 另含 `participant-model.ts` 的 `collapseStatuses` / `hasPhaseVocabulary`
//! （真源就地定义，只是逻辑上属参与者层）。
//!
//! ★`runIndex` / `actorKey` 不在这里——真源它们是 `participant-model.ts` 的
//! 私有函数，住`participant_model.rs`。

use std::collections::HashMap;

use super::run_state::{WorkflowRunActor, WorkflowRunNode, WorkflowRunNodeOutcome, WorkflowRunNodePhase};
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

/// `WorkflowRunOverlay`（真源 :22-31）：运行投影叠到静态图上的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunOverlay {
    /// ★真源 :24-28 —— 按图上的 step id 索引的状态。**偏表**：没有观察到实例的
    /// step **没有条目**。「每个 step 都有值」曾is给 React Flow 板面的承诺；
    /// 板面退役后没有消费者需要它，而它让「从没跑过」与「排队中」共用一个 `pending`。
    pub statuses: StepStatusTable,
    /// 从 running 的 step 出发的排序边是否动画。
    pub animated_edges: bool,
}

/// `OverlayTarget`（真源 :88-92）：一张卡片的收状态口子。
///
/// ★真源 :84-87 注释——`lane` 只有 may-set 展开的拷贝才有，它是这张卡片的**全部**
/// 主张（「这次 ask 可能跑在这条车道上」），所以别的车道上的实例与它无关。
/// `phase` 是阶段拷贝的同一种主张（「这次 ask 是在这个阶段里发的」）：
/// 出生在别的阶段的实例与它无关。
#[derive(Debug, Clone, Default)]
struct OverlayTarget {
    /// 该拷贝收进来的实例状态。
    instances: Vec<StepRunStatus>,
    lane: Option<String>,
    phase: Option<String>,
}

/// `workflowRunOverlay`（真源 :94-157）。
///
/// ★真源 :15-21 注释——这个视图刻意**不展开**成实例图：它就是提交时那张静态因果图，
/// 加一层状态装饰。由此得到的最强性质是**运行期间图的节点与边集合绝不变化**
/// ——rank 单调、无重排按构造成立，因为叠加从不摆卡片。代价只有一个：
/// 一个静态 step 对应 N 个运行时实例（循环 / fan-out），所以状态必须坍缩。
///
/// ★真源 :101-105 注释——关联键是**站点 id**，不是卡片 id：may-set 车道展开后一个站点
/// 对应每候选车道一张卡片，阶段拷贝后一个站点对应每认领阶段一张卡片
/// （`ask#1~phase#3`）；`source` 记下展开自的站点，两种拷贝都按它收，
/// 再各按自己的主张收窄。漏掉后一条，一个共享 helper 从五个阶段各派一批子代理时，
/// 第一批一动五站的灯全亮。
pub fn workflow_run_overlay(
    run: Option<&super::run_state::WorkflowRunState>,
    graph: &WorkflowCausalityGraphData,
) -> WorkflowRunOverlay {
    // ★真源 :99 —— 没有 run 就是静态渲染：返回空表而不是一张全 pending 的表，
    // 让组件保持「零运行时数据」的样子。
    let Some(run) = run else {
        return WorkflowRunOverlay {
            statuses: StepStatusTable::new(),
            animated_edges: false,
        };
    };

    // `byStepId` 保插入序（真源用 Map 的插入序迭代）。
    let mut by_step_id: Vec<(String, Vec<StepRunStatus>)> = Vec::new();
    let mut targets_by_site_id: HashMap<String, Vec<usize>> = HashMap::new();
    let mut targets: Vec<OverlayTarget> = Vec::new();
    for step in &graph.steps {
        let slot = targets.len();
        targets.push(OverlayTarget {
            instances: Vec::new(),
            // ★真源 :111-112 —— 不带 source 的 step **不做**车道收窄：
            // >4 候选（或含 unknown）回退的单卡画在 lanes[0]，实例却可能落在
            // 任一候选车道上，收窄会把这类卡片永久熄灭。
            lane: step.source.as_ref().map(|_| step.lane.clone()),
            phase: step.phase.clone(),
        });
        by_step_id.push((step.id.clone(), Vec::new()));
        // 两种拷贝（车道 / 阶段）都按站点收（真源 :118）。
        let site_id = step.source.clone().unwrap_or_else(|| step.id.clone());
        targets_by_site_id.entry(site_id).or_default().push(slot);
    }

    let mut binder = super::instance_phases::phase_binder(graph, Some(run));
    for node in &run.nodes {
        // ★真源 :127 —— 图里不存在的 site id 一律忽略：叠加绝不增删节点。
        let Some(slots) = targets_by_site_id.get(&node.site_id) else {
            continue;
        };
        let status = status_of_run_node(node);
        for &slot in slots {
            let target = &mut targets[slot];
            // actorSiteId 缺席的实例进该站点的**全部**拷贝：退化成旧单卡的过度点亮，
            // 而不是死图——liveness 线索宁可多亮一格，也不能一格都不亮。
            let belongs_to_other_lane = target.lane.is_some()
                && node.actor_site_id.is_some()
                && node.actor_site_id.as_deref() != target.lane.as_deref();
            if belongs_to_other_lane {
                continue;
            }
            // 出生阶段同理：无戳的实例（旧 run、标记前出生）由 binder 按既有规则归位——
            // 无词汇的 run 落全部阶段，退化成今天的过度点亮，而不是熄灯。
            if let Some(phase) = &target.phase {
                if !binder.has(phase, node.phase_name.as_deref()) {
                    continue;
                }
            }
            target.instances.push(status);
        }
    }

    let mut statuses = StepStatusTable::new();
    let mut any_running = false;
    for (slot, (step_id, _)) in by_step_id.iter().enumerate() {
        let Some(status) = aggregate_run_statuses(&targets[slot].instances) else {
            continue;
        };
        statuses.insert(step_id.clone(), status);
        if status == StepRunStatus::Running {
            any_running = true;
        }
    }

    WorkflowRunOverlay {
        statuses,
        animated_edges: any_running,
    }
}

/// `workflowRunActorsForLane`（真源 :166-174）。
///
/// ★真源 :159-165 注释——车道 → 该车道上的 actor 实例（单实例直接开，
/// 多实例弹选择器）。车道 site id 是**身份**，显示名只是展示。
/// `workspace` 与 `unknown` 是合成车道，上面没有会话，所以恒返回空——
/// world-read step 因此只有选中态、没有下钻。
pub fn workflow_run_actors_for_lane(
    run: Option<&super::run_state::WorkflowRunState>,
    lane_id: &str,
) -> Vec<WorkflowRunActor> {
    let Some(run) = run else {
        return Vec::new();
    };
    if super::types::is_synthetic_lane_id(lane_id) {
        return Vec::new();
    }
    let mut actors: Vec<WorkflowRunActor> = run
        .actors
        .iter()
        .filter(|a| a.site_id == lane_id)
        .cloned()
        .collect();
    actors.sort_by_key(|a| a.ordinal);
    actors
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

    // ── workflowRunOverlay ──

    fn graph_from_str(raw: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(raw).unwrap()
    }

    fn run_from_str(raw: &str) -> super::super::run_state::WorkflowRunState {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn overlay_without_run_is_empty_not_all_pending() {
        // ★真源 :99 —— 没有 run 就是静态渲染：返回空表而不是一张全 pending 的表，
        // 让组件保持「零运行时数据」的样子。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let overlay = workflow_run_overlay(None, &graph);
        assert!(overlay.statuses.is_empty(), "空表 ≠ 全 pending");
        assert!(!overlay.animated_edges);
    }

    #[test]
    fn overlay_folds_instances_onto_their_site_step() {
        // 真源 :106-156 —— 一个静态 step 对应 N 个实例 → 状态坍缩。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [
                {"siteId": "s1", "ordinal": 0, "status": "running"},
                {"siteId": "s1", "ordinal": 1, "status": "completed"}
            ], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "executing",
                 "actorSiteId": "s1", "actorOrdinal": 0},
                {"siteId": "s1", "ordinal": 1, "phase": "settled", "outcome": "ok",
                 "actorSiteId": "s1", "actorOrdinal": 1}
            ]}"#,
        );
        let overlay = workflow_run_overlay(Some(&run), &graph);
        // running + done → running（真源 :77「有东西在动就报 running」）。
        assert_eq!(overlay.statuses.get("s1"), Some(&StepRunStatus::Running));
        assert!(overlay.animated_edges, "有 running → 边动画");
    }

    #[test]
    fn overlay_is_a_sparse_table() {
        // ★真源 :24-28 —— 偏表：没观察到实例的 step 没有条目。
        // 缺席与pending 是两件事。
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "问", "lane": "l1"},
                {"id": "s2", "kind": "ask", "label": "问", "lane": "l1"}
            ], "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "s1", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "s1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "s1", "actorOrdinal": 0}]}"#,
        );
        let overlay = workflow_run_overlay(Some(&run), &graph);
        assert_eq!(overlay.statuses.len(), 1);
        assert!(!overlay.statuses.contains_key("s2"), "s2 没跑过 → 无条目");
    }

    #[test]
    fn overlay_ignores_sites_absent_from_the_graph() {
        // ★真源 :127 —— 图里不存在的 site id 一律忽略：叠加绝不增删节点。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "ghost", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "ghost", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "ghost", "actorOrdinal": 0}]}"#,
        );
        let overlay = workflow_run_overlay(Some(&run), &graph);
        assert!(overlay.statuses.is_empty());
    }

    #[test]
    fn overlay_splits_a_shared_helper_across_phases() {
        // ★真源 :102-105 —— 漏掉阶段收窄，一个共享 helper 从五个阶段各派一批子代理时，
        // 第一批一动五站的灯全亮。站点相同、阶段不同的两张拷贝只各收自己那批。
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "ask#1~phase#1", "kind": "ask", "label": "问", "lane": "l1",
                 "source": "ask#1", "phase": "p1"},
                {"id": "ask#1~phase#2", "kind": "ask", "label": "问", "lane": "l1",
                 "source": "ask#1", "phase": "p2"}
            ], "lanes": [], "participants": [], "handoffs": [],
            "phases": [{"id": "p1", "name": "阶段一"}, {"id": "p2", "name": "阶段二"}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running",
                            "phaseName": "阶段一"}],
                "nodes": [{"siteId": "ask#1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "l1", "actorOrdinal": 0,
                           "phaseName": "阶段一"}]}"#,
        );
        let overlay = workflow_run_overlay(Some(&run), &graph);
        assert_eq!(
            overlay.statuses.get("ask#1~phase#1"),
            Some(&StepRunStatus::Running),
            "它出生的阶段那张卡亮"
        );
        assert_eq!(
            overlay.statuses.get("ask#1~phase#2"),
            None,
            "★兄弟阶段**没有条目**（稀疏表），而不是跟着亮——第一批一动五站的灯全亮"
        );
    }

    #[test]
    fn overlay_does_not_narrow_lane_without_source() {
        // ★真源 :111-112 —— 不带 source 的 step **不做**车道收窄：
        // >4 候选回退的单卡画在 lanes[0]，实例却可能落在任一候选车道上，
        // 收窄会把这类卡片永久熄灭。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "fallback"}],
                "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "other", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "s1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "other", "actorOrdinal": 0}]}"#,
        );
        let overlay = workflow_run_overlay(Some(&run), &graph);
        assert_eq!(
            overlay.statuses.get("s1"),
            Some(&StepRunStatus::Running),
            "无 source 的卡不按车道收窄"
        );
    }

    #[test]
    fn overlay_overlights_rather_than_going_dark_for_unstamped_nodes() {
        // ★真源 :131-132 /138-140 —— actorSiteId 缺席的实例进该站点的**全部**拷贝，
        // 退化成旧单卡的过度点亮而不是死图：liveness 线索宁可多亮一格。
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "r#1~phase#1", "kind": "ask", "label": "读", "lane": "l1",
                 "source": "r#1", "phase": "p1"},
                {"id": "r#1~phase#2", "kind": "ask", "label": "读", "lane": "l1",
                 "source": "r#1", "phase": "p2"}
            ], "lanes": [], "participants": [], "handoffs": [],
            "phases": [{"id": "p1", "name": "阶段一"}, {"id": "p2", "name": "阶段二"}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [],
                "nodes": [{"siteId": "r#1", "ordinal": 0, "phase": "executing"}]}"#,
        );
        let overlay = workflow_run_overlay(Some(&run), &graph);
        assert_eq!(
            overlay.statuses.get("r#1~phase#1"),
            Some(&StepRunStatus::Running)
        );
        assert_eq!(
            overlay.statuses.get("r#1~phase#2"),
            Some(&StepRunStatus::Running),
            "无戳 run → 落全部阶段（过度点亮，不是熄灯）"
        );
    }

    #[test]
    fn overlay_never_adds_or_removes_edges() {
        // ★真源 :18-19 —— 运行期间图的节点与边集合绝不变化。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "participants": [],
                "handoffs": [{"from": "p1", "to": "p2"}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "s1", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "s1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "s1", "actorOrdinal": 0}]}"#,
        );
        let _ = workflow_run_overlay(Some(&run), &graph);
        assert_eq!(graph.handoffs.len(), 1, "overlay 不碰图本身");
    }

    // ── workflowRunActorsForLane ──

    #[test]
    fn actors_for_lane_sorted_by_ordinal() {
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [
                {"siteId": "l1", "ordinal": 3, "status": "running"},
                {"siteId": "l1", "ordinal": 1, "status": "completed"},
                {"siteId": "l2", "ordinal": 0, "status": "running"}
            ], "nodes": []}"#,
        );
        let actors = workflow_run_actors_for_lane(Some(&run), "l1");
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0].ordinal, 1, "按 ordinal 升序（认领顺序）");
        assert_eq!(actors[1].ordinal, 3);
    }

    #[test]
    fn synthetic_lanes_have_no_actors() {
        // ★真源 :163-165 —— workspace / unknown 是合成车道，上面没有会话，
        // 所以恒返回空——world-read step 因此只有选中态、没有下钻。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "workspace", "ordinal": 0, "status": "running"}],
                "nodes": []}"#,
        );
        assert!(
            workflow_run_actors_for_lane(Some(&run), super::super::types::WORKSPACE_LANE_ID)
                .is_empty()
        );
        assert!(
            workflow_run_actors_for_lane(Some(&run), super::super::types::UNKNOWN_LANE_ID)
                .is_empty()
        );
        assert!(
            workflow_run_actors_for_lane(None, "l1").is_empty(),
            "无 run → 空"
        );
    }
}
