//! 1:1 翻译 `packages/ui/src/lib/workflowRunLine.ts`（149 行）。
//!
//! 侧栏工作流运行行的纯模型。输入是 sessions-index 下发的 workflowActivity 与渲染端的
//! 已确认集合，输出是「画哪几行、每行几盏灯」。无时钟、无 DOM：已结束的行何时折叠只看
//! 确认集合，不看时间。
//!
//! Rust 侧的输入类型按 wire 结构建模（`SessionWorkflowPhaseSummary` 等，
//! 对齐真源 `sessions-index-workflow-activity.ts` 的 schema），由宿主从会话索引下发。

use crate::components::timeline_bands::{band_of, fold_phase_bands, track_of};

/// 一个会话最多画的运行行数；其余折成「+n」（真源 :13）。
const WORKFLOW_RUN_LINE_MAX_LINES: usize = 2;
/// 迷你轨道最多画的站点数；更多时折到运行站 ±2 并带「+n」尾（真源 :15）。
const WORKFLOW_RUN_RAIL_MAX_STATIONS: usize = 6;
/// 折叠时保留在运行站两侧的站点数（真源 :17）。
const RAIL_FOLD_RADIUS: usize = 2;

/// 站点灯的四态（真源 `SessionWorkflowPhaseSummary["status"]`，与卡片时间线同一词汇）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionWorkflowPhaseStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "failed")]
    Failed,
}

impl SessionWorkflowPhaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionWorkflowPhaseStatus::Pending => "pending",
            SessionWorkflowPhaseStatus::Running => "running",
            SessionWorkflowPhaseStatus::Done => "done",
            SessionWorkflowPhaseStatus::Failed => "failed",
        }
    }
}

/// 一站（真源 `SessionWorkflowPhaseSummary`）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWorkflowPhaseSummary {
    pub name: String,
    pub status: SessionWorkflowPhaseStatus,
    /// 进入本站时仍在跑的其他站的**下标**（声明序）。零条时键缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alongside: Option<Vec<i64>>,
}

/// 一条运行行的 run 摘要（真源 `SessionWorkflowRunSummary` 的 Rust 视图，
/// 只装运行行真正读的字段；完整 wire 结构由宿主投影层给出）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWorkflowRunSummary {
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 工作流后台工作的标题（= run 的展示名）；投影里没有对应后台工作时缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `pending | running | completed | errored | stopped`（run 状态五值，逐字 wire）。
    pub status: String,
    /// `stopped` 的原因词（user / model / provider / interrupted / superseded）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// 后台工作的开始时刻，tooltip 的 elapsed 用；没有后台工作时缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    /// 站点表，声明序。空数组 = 画一个隐含站点「Workflow」。
    pub phases: Vec<SessionWorkflowPhaseSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,
    /// status === "running" 的子代理数（tooltip 的「{n} agents working」）。
    #[serde(default)]
    pub agents_working: i64,
}

/// 会话的工作流活动（真源 `SessionWorkflowActivity`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionWorkflowActivity {
    pub runs: Vec<SessionWorkflowRunSummary>,
}

/// 迷你轨道的一站（真源 `WorkflowRunRailStation`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunRailStation {
    pub name: String,
    pub status: SessionWorkflowPhaseStatus,
    /// 进入本站的那一段轨道是否已被控制流走过（强色段）。
    pub reached: bool,
    /// 进入本站的那一段是**双线段**：本站与前一站同属一条带、却在不同轨道上。
    /// 标志挂在**站**上而不是段上，所以它能活过窗口折叠。
    pub twin: bool,
}

/// 迷你轨道（真源 `WorkflowRunRail`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunRail {
    pub stations: Vec<WorkflowRunRailStation>,
    /// 被折叠掉、不画的站点数（「+n」尾）；0 时不画尾。
    pub hidden: usize,
    /// 脚本没有阶段词汇表：画一个隐含站点「Workflow」。
    pub implicit: bool,
}

/// 控制流是否到过这一站：running / done / failed 都算（真源 :41-43）。
fn is_workflow_run_station_reached(status: SessionWorkflowPhaseStatus) -> bool {
    status != SessionWorkflowPhaseStatus::Pending
}

/// `isSessionWorkflowRunLive`（真源 shared sessions-index :73-75）。
pub fn is_session_workflow_run_live(status: &str) -> bool {
    status == "pending" || status == "running"
}

/// `foldWorkflowRunRail`（真源 :49-93）：迷你轨道的折叠。
///
/// ≤ 6 站全画；更多时以运行站为中心（没有运行站就取最后一个到过的站，再没有就取首站）
/// 保留 ±2 共 5 站，其余合成「+n」尾。这是**固定窗口**而不是滚动：侧栏没有横向手势。
///
/// ★真源 :55-57 —— 折带必须在**全表**上做：窗口只取其中一段，而带是声明序上的连通分量，
/// 按窗口内的下标重折会把跨窗口边界的带拆断。折完再窗口化，双线段标志随站走。
pub fn fold_workflow_run_rail(phases: &[SessionWorkflowPhaseSummary]) -> WorkflowRunRail {
    if phases.is_empty() {
        return WorkflowRunRail {
            stations: Vec::new(),
            hidden: 0,
            implicit: true,
        };
    }
    let alongside: Vec<Vec<i64>> = phases
        .iter()
        .map(|phase| phase.alongside.clone().unwrap_or_default())
        .collect();
    let bands = fold_phase_bands(phases.len() as i64, &alongside);
    let mut all: Vec<WorkflowRunRailStation> = Vec::with_capacity(phases.len());
    for (index, phase) in phases.iter().enumerate() {
        let band = if index == 0 {
            None
        } else {
            band_of(&bands, index as i64)
        };
        let twin = band.is_some()
            && band_of(&bands, index as i64 - 1) == band
            && track_of(&bands, index as i64) != track_of(&bands, index as i64 - 1);
        all.push(WorkflowRunRailStation {
            name: phase.name.clone(),
            status: phase.status,
            reached: is_workflow_run_station_reached(phase.status),
            twin,
        });
    }
    if all.len() <= WORKFLOW_RUN_RAIL_MAX_STATIONS {
        return WorkflowRunRail {
            stations: all,
            hidden: 0,
            implicit: false,
        };
    }
    let anchor = all
        .iter()
        .position(|station| station.status == SessionWorkflowPhaseStatus::Running)
        .unwrap_or_else(|| {
            (0..all.len())
                .rev()
                .find(|&index| all[index].reached)
                .unwrap_or(0)
        });
    let window_size = RAIL_FOLD_RADIUS * 2 + 1;
    let mut start = anchor.saturating_sub(RAIL_FOLD_RADIUS);
    let end = all.len().min(start + window_size);
    start = end.saturating_sub(window_size);
    let stations = all[start..end].to_vec();
    WorkflowRunRail {
        hidden: all.len() - stations.len(),
        stations,
        implicit: false,
    }
}

/// 并行阶段之间的连接词（真源 :96）。
const WORKFLOW_RUN_PARALLEL_SEPARATOR: &str = " ∥ ";

/// `workflowRunParallelPhaseLabel`（真源 :104-111）：同时在跑的站名，连成 tooltip 里的
/// 一段。只有一个（或零个）站在跑时返回 `None`：调用方退回 `currentPhase`，文案一字不变。
pub fn workflow_run_parallel_phase_label(
    phases: &[SessionWorkflowPhaseSummary],
) -> Option<String> {
    let running: Vec<&str> = phases
        .iter()
        .filter(|phase| phase.status == SessionWorkflowPhaseStatus::Running)
        .map(|phase| phase.name.as_str())
        .collect();
    if running.len() > 1 {
        Some(running.join(WORKFLOW_RUN_PARALLEL_SEPARATOR))
    } else {
        None
    }
}

/// 行选择（真源 `WorkflowRunLineSelection`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunLineSelection {
    pub lines: Vec<SessionWorkflowRunSummary>,
    /// 没画出来的行数（「+n」词）。
    pub overflow: usize,
}

/// `selectWorkflowRunLines`（真源 :123-135）：选出要画的行。在跑的永远画；已结束的只在
/// **未确认**时画（确认 = 会话被打开过）。输入顺序已由投影排好，这里只过滤与截断。
pub fn select_workflow_run_lines(
    activity: Option<&SessionWorkflowActivity>,
    is_acknowledged: &dyn Fn(&str) -> bool,
) -> WorkflowRunLineSelection {
    let Some(activity) = activity else {
        return WorkflowRunLineSelection {
            lines: Vec::new(),
            overflow: 0,
        };
    };
    let visible: Vec<SessionWorkflowRunSummary> = activity
        .runs
        .iter()
        .filter(|run| is_session_workflow_run_live(&run.status) || !is_acknowledged(&run.run_id))
        .cloned()
        .collect();
    let overflow = visible.len().saturating_sub(WORKFLOW_RUN_LINE_MAX_LINES);
    WorkflowRunLineSelection {
        lines: visible.into_iter().take(WORKFLOW_RUN_LINE_MAX_LINES).collect(),
        overflow,
    }
}

/// `settledWorkflowRunIds`（真源 :138-143）：已结束（可被确认）的 run id，打开会话时整批确认。
pub fn settled_workflow_run_ids(activity: Option<&SessionWorkflowActivity>) -> Vec<String> {
    let Some(activity) = activity else {
        return Vec::new();
    };
    activity
        .runs
        .iter()
        .filter(|run| !is_session_workflow_run_live(&run.status))
        .map(|run| run.run_id.clone())
        .collect()
}

/// `countLiveWorkflowRuns`（真源 :146-149）：在跑的 run 数（收起的项目组头旁的脉冲灯与数量）。
pub fn count_live_workflow_runs(activity: Option<&SessionWorkflowActivity>) -> usize {
    let Some(activity) = activity else {
        return 0;
    };
    activity
        .runs
        .iter()
        .filter(|run| is_session_workflow_run_live(&run.status))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(name: &str, status: SessionWorkflowPhaseStatus) -> SessionWorkflowPhaseSummary {
        SessionWorkflowPhaseSummary {
            name: name.to_string(),
            status,
            alongside: None,
        }
    }

    fn summary(run_id: &str, status: &str) -> SessionWorkflowRunSummary {
        SessionWorkflowRunSummary {
            run_id: run_id.to_string(),
            tool_call_id: None,
            name: None,
            status: status.to_string(),
            stop_reason: None,
            started_at: None,
            phases: Vec::new(),
            current_phase: None,
            agents_working: 0,
        }
    }

    #[test]
    fn empty_phases_is_implicit() {
        // 真源 :52-54 —— 没有阶段词汇表：画一个隐含站点「Workflow」。
        let rail = fold_workflow_run_rail(&[]);
        assert!(rail.implicit);
        assert!(rail.stations.is_empty());
        assert_eq!(rail.hidden, 0);
    }

    #[test]
    fn six_stations_or_fewer_are_all_drawn() {
        let phases: Vec<_> = (0..6)
            .map(|i| phase(&format!("p{i}"), SessionWorkflowPhaseStatus::Pending))
            .collect();
        let rail = fold_workflow_run_rail(&phases);
        assert_eq!(rail.stations.len(), 6);
        assert!(!rail.implicit);
        assert_eq!(rail.hidden, 0);
    }

    #[test]
    fn fold_keeps_running_station_in_a_five_window() {
        // 8 站、running 在 6：anchor=6 → start=4,end=8 → 再平衡 start=3 → 窗口 [3..8]，
        // hidden = 3；anchor p6 落在窗口下标 3。
        let mut phases: Vec<_> = (0..8)
            .map(|i| phase(&format!("p{i}"), SessionWorkflowPhaseStatus::Done))
            .collect();
        phases[6].status = SessionWorkflowPhaseStatus::Running;
        let rail = fold_workflow_run_rail(&phases);
        assert_eq!(rail.stations.len(), 5);
        assert_eq!(rail.hidden, 3);
        assert_eq!(rail.stations[3].name, "p6");
        assert_eq!(rail.stations[3].status, SessionWorkflowPhaseStatus::Running);
    }

    #[test]
    fn fold_anchor_falls_back_to_last_reached_then_first() {
        // 没有 running：退到最后一个 reached 的站（真源 :77-85）。
        let mut phases: Vec<_> = (0..8)
            .map(|i| phase(&format!("p{i}"), SessionWorkflowPhaseStatus::Pending))
            .collect();
        phases[5].status = SessionWorkflowPhaseStatus::Failed;
        let rail = fold_workflow_run_rail(&phases);
        // anchor=5 → start=3,end=8 → start=3；p5 在窗口下标 2。
        assert_eq!(rail.stations[2].name, "p5");
        // 全 pending：锚 = 0（真源 :86）。
        let all_pending: Vec<_> = (0..8)
            .map(|i| phase(&format!("p{i}"), SessionWorkflowPhaseStatus::Pending))
            .collect();
        let rail = fold_workflow_run_rail(&all_pending);
        assert_eq!(rail.stations[0].name, "p0");
    }

    #[test]
    fn twin_flag_follows_band_membership_across_the_fold() {
        // alongside 让 0 与 1 同带不同轨 → 站 1 是双线段；窗口折叠后标志仍在。
        let mut phases: Vec<_> = (0..8)
            .map(|i| phase(&format!("p{i}"), SessionWorkflowPhaseStatus::Pending))
            .collect();
        phases[1].alongside = Some(vec![0]);
        let rail = fold_workflow_run_rail(&phases);
        assert!(!rail.stations[0].twin);
        assert!(rail.stations[1].twin);
    }

    #[test]
    fn parallel_label_needs_two_or_more_running() {
        // 真源 :104-111 —— 一个（或零个）在跑返回 None。
        let phases = vec![
            phase("a", SessionWorkflowPhaseStatus::Running),
            phase("b", SessionWorkflowPhaseStatus::Pending),
        ];
        assert_eq!(workflow_run_parallel_phase_label(&phases), None);
        let phases = vec![
            phase("a", SessionWorkflowPhaseStatus::Running),
            phase("b", SessionWorkflowPhaseStatus::Running),
            phase("c", SessionWorkflowPhaseStatus::Done),
        ];
        assert_eq!(
            workflow_run_parallel_phase_label(&phases).as_deref(),
            Some("a ∥ b")
        );
    }

    #[test]
    fn selection_keeps_live_and_unacknowledged_lines_capped_at_two() {
        // 真源 :123-135 —— 在跑的永远画；已结束的只在未确认时画；最多两行。
        let activity = SessionWorkflowActivity {
            runs: vec![
                summary("r1", "running"),
                summary("r2", "completed"),
                summary("r3", "stopped"),
                summary("r4", "errored"),
            ],
        };
        let selection = select_workflow_run_lines(Some(&activity), &|_| false);
        assert_eq!(selection.lines.len(), 2);
        assert_eq!(selection.overflow, 2);
        // r2 已确认、r3/r4 没有：可见 = r1 + r3 + r4 → 画 2 行、溢出 1。
        let selection = select_workflow_run_lines(Some(&activity), &|id| id == "r2");
        assert_eq!(selection.lines.iter().map(|r| r.run_id.as_str()).collect::<Vec<_>>(), vec!["r1", "r3"]);
        assert_eq!(selection.overflow, 1);
    }

    #[test]
    fn settled_ids_are_the_non_live_runs() {
        let activity = SessionWorkflowActivity {
            runs: vec![summary("r1", "running"), summary("r2", "completed")],
        };
        assert_eq!(settled_workflow_run_ids(Some(&activity)), vec!["r2".to_string()]);
        assert!(settled_workflow_run_ids(None).is_empty());
        assert_eq!(count_live_workflow_runs(Some(&activity)), 1);
        assert_eq!(count_live_workflow_runs(None), 0);
    }

    #[test]
    fn live_predicate_matches_truth_source() {
        // 真源 shared :73-75 —— pending / running 都算 live。
        assert!(is_session_workflow_run_live("pending"));
        assert!(is_session_workflow_run_live("running"));
        assert!(!is_session_workflow_run_live("completed"));
        assert!(!is_session_workflow_run_live("errored"));
        assert!(!is_session_workflow_run_live("stopped"));
    }
}
