//! 1:1 翻译 `packages/shared/src/zcode-protocol-v4/workflow-runs.ts` 中
//! **toolCall 渲染层要读的**那部分 run 状态类型（actor / node / run 骨架）。
//!
//! ★裁剪注明——`workflowRunSchema` 共 40+ 字段（usage / artifacts / reports /
//! pendingQuestions / concurrency / phases / unlistedByPhase…），
//! 它们服务于侧板、检视器、产物区等**别的消费面**，toolCall 渲染层不读。
//! 本模块只落 toolCall 侧真正用到的：
//! - `WorkflowRunActor`（完整）——卡片实例拆分要用 siteId / ordinal / name /
//!   status / phaseName
//! - `WorkflowRunNode`（部分）——站点收窄要用 siteId / actorSiteId /
//!   actorOrdinal / phase / outcome / phaseName
//! - `WorkflowRunState` 骨架——runId / status / actors / nodes 四项
//!
//! 其余字段待对应消费面迁移时补齐（见文件末 TODO）。

use serde::{Deserialize, Serialize};

/// `WORKFLOW_RUN_OBSERVATION_STATUSES`（真源 workflow-runs.ts）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowRunStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "errored")]
    Errored,
    #[serde(rename = "stopped")]
    Stopped,
}

impl WorkflowRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Errored => "errored",
            Self::Stopped => "stopped",
        }
    }

    /// 该状态下 run 是否仍在飞（用于「数工作中的子代理」）。
    pub fn is_live(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }
}

/// `workflowRunActorSchema` 的 `status`（真源 :186）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowRunActorStatus {
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
}

/// `WorkflowRunActor`（真源 :181-195）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunActor {
    pub site_id: String,
    pub ordinal: i64,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub status: WorkflowRunActorStatus,
    /// ★真源 :187-193 —— 这个实例**出生**在哪个阶段：ordinal 被铸造的那一刻，
    /// 控制流所在的 `phase("…")` 标记名。UI 按**名字**与 `phases[].name` 关联
    /// ——名字是引擎与分析器唯一共享的词汇。
    ///
    /// 缺席有两种读法，消费者都要认：出生在任何标记之前（脚本没写 `phase()`，
    /// 或写在后面），或者发事件的是不带这个键的旧 CLI。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_name: Option<String>,
}

/// `workflowRunNodeSchema` 的 `phase`（真源 :226）。
///
/// ★真源 :212-216 注释——这就是引擎**实际发出的**节点事件：
/// `queued → dispatched → executing ⇄ waiting → (repairing | nudged) → settled`。
/// `dispatched` 是「会话就绪、首个请求尚未准入」的短暂相位，
/// 读面把它与 queued / waiting 同归「等待」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowRunNodePhase {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "dispatched")]
    Dispatched,
    #[serde(rename = "executing")]
    Executing,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "repairing")]
    Repairing,
    #[serde(rename = "nudged")]
    Nudged,
    #[serde(rename = "settled")]
    Settled,
}

/// `workflowRunNodeSchema` 的 `outcome`（真源 :227）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowRunNodeOutcome {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// `WorkflowRunNode`（真源 :222-）的 toolCall 侧字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunNode {
    pub site_id: String,
    pub ordinal: i64,
    /// ★真源 :218-220 —— kind 可缺省：resume 的完结命中短路直接发
    /// `node-settled`，不经 `node-queued`，而 kind 只在 queued 上携带。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub phase: WorkflowRunNodePhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<WorkflowRunNodeOutcome>,
    /// 该节点所属 actor 的站点 id（**world-read 无 actor**）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_site_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_ordinal: Option<i64>,
    /// ★真源 :232-238 —— 这个实例**出生**在哪个阶段。
    /// ⚠与上面的 `phase` **不是**一回事：`phase` 是节点的生命周期相位，
    /// `phaseName` 是脚本阶段坐标。字段特意不叫 `phase` 就是为了不把两个概念揉在一起。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_name: Option<String>,
    /// 这次 ask 走到第几个已解析轮次（1 起）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<i64>,
    /// 累计工具调用数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<i64>,
    /// 最近一次工具调用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tool: Option<WorkflowRunNodeLastTool>,
}

/// `workflowRunNodeLastToolSchema`（真源 :205-208）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunNodeLastTool {
    pub name: String,
    /// 文件工具的路径、Bash 的命令头；读不出时缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// `WorkflowRunState`（真源 workflow-runs.ts 的 `workflowRunSchema`）骨架。
///
/// ★**这是 toolCall 渲染层读的那部分**，不是完整 schema——
/// 完整 schema 有 40+ 字段（usage / artifacts / reports / pendingQuestions /
/// concurrency / phases / phaseNames / unlistedByPhase…），
/// 服务于侧板、检视器、产物区等别的消费面。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunState {
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub status: WorkflowRunStatus,
    pub actors: Vec<WorkflowRunActor>,
    pub nodes: Vec<WorkflowRunNode>,
    /// ★真源 :472 —— 被进入过的阶段，按首次进入顺序。**零条时整个键缺席**
    /// （不是空数组）。时间线据它给零成员的站点灯。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<WorkflowRunPhase>>,
    /// 控制流最后进入的阶段名；从未进入过任何阶段时缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<String>,
    /// 脚本**声明**的阶段表，按声明序。零条 / 旧 CLI / 无标记脚本时缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_names: Option<Vec<String>>,
    /// 与 `phaseNames` **按位置对齐**的「同时在跑」表。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_alongside: Option<Vec<Vec<i64>>>,
}

/// `workflowRunPhaseSchema`（已进入的阶段）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunPhase {
    pub name: String,
}

// TODO(后续迁移)：workflowRunSchema 的其余字段（按消费面逐块补）。
// - `usage` / `error` / `stopReason` / `resumable` / `stalled`：摘要行与
//   失败态展示要用（timeline-summary.ts / workflow-run 家族）。
// - `reports` / `artifacts` / `pendingQuestions`：产物区、问题区（app-shell 侧）。
// - `concurrency` / `concurrencyCeiling` / `subagentModel`：设置轮那行
//   （WorkflowRetuneRow / WorkflowSettingsChangeRow）。
// - `resultPreview` / `lastEventSequence` / `truncated` / `askedAt`：卡头与摘要。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_liveness() {
        // run-status-presentation 的 RUN_STATUS_DOT 用它分「跑着/结束」。
        assert!(WorkflowRunStatus::Pending.is_live());
        assert!(WorkflowRunStatus::Running.is_live());
        assert!(!WorkflowRunStatus::Completed.is_live());
        assert!(!WorkflowRunStatus::Errored.is_live());
        assert!(!WorkflowRunStatus::Stopped.is_live());
    }

    #[test]
    fn run_status_serde_uses_wire_values() {
        // 真源 z.enum(["pending", "running", "completed", "errored", "stopped"])
        let all = [
            ("pending", WorkflowRunStatus::Pending),
            ("running", WorkflowRunStatus::Running),
            ("completed", WorkflowRunStatus::Completed),
            ("errored", WorkflowRunStatus::Errored),
            ("stopped", WorkflowRunStatus::Stopped),
        ];
        for (wire, expected) in all {
            // wire 值要先成合法 JSON 文本（裸串不是）。
            let back: WorkflowRunStatus =
                serde_json::from_value(serde_json::Value::from(wire)).unwrap();
            assert_eq!(back, expected);
            assert_eq!(expected.as_str(), wire);
        }
    }

    #[test]
    fn actor_parses_minimal_payload() {
        // 真源 :181-195 —— 只有 siteId/ordinal/status 必填。
        let raw = serde_json::json!({
            "siteId": "ask", "ordinal": 1, "status": "running"
        });
        let a: WorkflowRunActor = serde_json::from_value(raw).unwrap();
        assert_eq!(a.site_id, "ask");
        assert_eq!(a.ordinal, 1);
        assert_eq!(a.status, WorkflowRunActorStatus::Running);
        assert_eq!(a.name, None);
        assert_eq!(a.phase_name, None, "旧 CLI 不发这个键");
    }

    #[test]
    fn node_kind_is_optional_for_resume_settle() {
        // ★真源 :218-220 —— resume 完结命中短路直接发 node-settled，
        // 不经 node-queued，而 kind 只在 queued 上携带。
        let raw = serde_json::json!({
            "siteId": "ask", "ordinal": 0,
            "phase": "settled", "outcome": "ok", "cached": true
        });
        let n: WorkflowRunNode = serde_json::from_value(raw).unwrap();
        assert_eq!(n.kind, None, "settled 不带 kind");
        assert_eq!(n.phase, WorkflowRunNodePhase::Settled);
        assert_eq!(n.outcome, Some(WorkflowRunNodeOutcome::Ok));
        assert_eq!(n.actor_site_id, None, "world-read 无 actor");
    }

    #[test]
    fn node_all_seven_phases_parse() {
        for (wire, expected) in [
            ("queued", WorkflowRunNodePhase::Queued),
            ("dispatched", WorkflowRunNodePhase::Dispatched),
            ("executing", WorkflowRunNodePhase::Executing),
            ("waiting", WorkflowRunNodePhase::Waiting),
            ("repairing", WorkflowRunNodePhase::Repairing),
            ("nudged", WorkflowRunNodePhase::Nudged),
            ("settled", WorkflowRunNodePhase::Settled),
        ] {
            let raw = serde_json::json!({
                "siteId": "ask", "ordinal": 0, "phase": wire
            });
            let n: WorkflowRunNode = serde_json::from_value(raw).unwrap();
            assert_eq!(n.phase, expected);
        }
    }

    #[test]
    fn node_outcome_three_values() {
        for wire in ["ok", "failed", "cancelled"] {
            let raw = serde_json::json!({
                "siteId": "ask", "ordinal": 0, "phase": "settled", "outcome": wire
            });
            assert!(serde_json::from_value::<WorkflowRunNode>(raw).is_ok());
        }
    }

    #[test]
    fn run_state_optional_blocks_may_be_absent() {
        // ★真源 :472/:476/:480 —— phases / phaseNames 零条时**整个键缺席**
        // （不是空数组）。
        let raw = serde_json::json!({
            "runId": "r1", "status": "running",
            "actors": [], "nodes": []
        });
        let run: WorkflowRunState = serde_json::from_value(raw).unwrap();
        assert_eq!(run.phases, None);
        assert_eq!(run.phase_names, None);
        assert_eq!(run.phase_alongside, None);
        assert_eq!(run.current_phase, None);
        assert_eq!(run.tool_call_id, None);
    }

    #[test]
    fn run_state_parses_with_phases() {
        let raw = serde_json::json!({
            "runId": "r1", "status": "completed",
            "actors": [{ "siteId": "ask", "ordinal": 0, "status": "completed" }],
            "nodes": [{
                "siteId": "ask", "ordinal": 0, "phase": "settled",
                "outcome": "ok", "actorSiteId": "ask", "actorOrdinal": 0
            }],
            "phases": [{ "name": "预备" }],
            "currentPhase": "预备",
            "phaseNames": ["预备", "主体"],
            "phaseAlongside": [[], [0]]
        });
        let run: WorkflowRunState = serde_json::from_value(raw).unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);
        assert_eq!(run.actors.len(), 1);
        assert_eq!(run.nodes[0].actor_site_id.as_deref(), Some("ask"));
        assert_eq!(run.phases.as_ref().unwrap()[0].name, "预备");
        assert_eq!(run.phase_names.as_ref().unwrap().len(), 2);
        // phaseAlongside[i] 落在 phaseNames 这张表上。
        assert_eq!(run.phase_alongside.as_ref().unwrap()[1], vec![0]);
    }

    #[test]
    fn node_turn_and_tool_count_are_optional() {
        // 真源 :258-260 —— 三者一起回答「它在动吗」。
        let raw = serde_json::json!({
            "siteId": "ask", "ordinal": 0, "phase": "executing"
        });
        let n: WorkflowRunNode = serde_json::from_value(raw).unwrap();
        assert_eq!(n.turn, None);
        assert_eq!(n.tool_calls, None);
        assert_eq!(n.last_tool, None);

        let raw2 = serde_json::json!({
            "siteId": "ask", "ordinal": 0, "phase": "executing",
            "turn": 3, "toolCalls": 7,
            "lastTool": { "name": "Read", "target": "a.ts" }
        });
        let n2: WorkflowRunNode = serde_json::from_value(raw2).unwrap();
        assert_eq!(n2.turn, Some(3));
        assert_eq!(n2.tool_calls, Some(7));
        assert_eq!(n2.last_tool.unwrap().name, "Read");
    }

    #[test]
    fn node_phase_name_is_distinct_from_phase() {
        // ★真源 :234-237 —— `phase` 是生命周期相位，`phaseName` 是脚本阶段坐标，
        // 字段特意不叫 phase 就是为了不把两个概念揉在一起。
        let raw = serde_json::json!({
            "siteId": "ask", "ordinal": 0,
            "phase": "executing", "phaseName": "预备"
        });
        let n: WorkflowRunNode = serde_json::from_value(raw).unwrap();
        assert_eq!(n.phase, WorkflowRunNodePhase::Executing);
        assert_eq!(n.phase_name.as_deref(), Some("预备"));
    }
}