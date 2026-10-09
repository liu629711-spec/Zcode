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
    /// 界在各个**出生阶段**上花掉了多少。**一格都没有时整个键缺席**。
    /// 表长比 `maxPhases` 多一格：那一格是「无阶段」，与具名阶段共用同一张表。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlisted_by_phase: Option<Vec<WorkflowRunUnlistedPhase>>,
    /// 该实例有待答的升级问题。**零条是常态**，而且它会来回进出：
    /// 一被作答就从表里消失，答完最后一个又退回缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_questions: Option<Vec<WorkflowRunPendingQuestion>>,
    /// run 级停滞（driver 的 RunStallClock 观察，随 `run-stalled` 事件到达）：
    /// 整个 run 连续一段阈值没有一次**成功的模型请求**。**为真才在场**。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stalled: Option<bool>,
    /// run 级用量：观察面，不是控制面。摘要行的 token 数读它。
    #[serde(default)]
    pub usage: WorkflowRunUsage,
    /// 本 run 发布的**用户面产物**，按首次出现顺序。**零件时整个键缺席**。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<WorkflowRunArtifactSummary>>,
    /// 这次 run 的**子代理**跑在哪个模型上，规范串 `providerId/modelId[$reasoningLevel]`。
    /// **只在用户给这次 run 指定过模型时在场**——不指定的 run 里子代理跟随会话模型，
    /// 没有可说的。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_model: Option<String>,
    /// `status === "stopped"` 才在场。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<WorkflowRunStopReason>,
    /// lineage 的两端：本 run 被哪次修订停下并替代（只随
    /// `stopReason: "superseded"` 出现）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    /// 并发现状。只在**两条界里有一条低于天花板**时在场：收到过
    /// `concurrency-changed`（共享桶被限流压低），或 `run-started` 带来一个
    /// 低于天花板的 `limit`（用户给这次 run 定了上限）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<WorkflowRunConcurrency>,
    /// 本机的并发天花板（`run-started` 随带、恒在）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency_ceiling: Option<i64>,
}

/// `workflowRunStopReason`（真源 :367）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowRunStopReason {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "provider")]
    Provider,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "superseded")]
    Superseded,
}

/// `workflowRunConcurrencySchema`（真源 :289-295）。
///
/// ★`cap` / `ceiling` 是读数芯片自己的水位，`limit` 是用户给这次 run 定的上限
/// （agent 侧钳到 `[1, 天花板]`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunConcurrency {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub cap: i64,
    pub ceiling: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<i64>,
}

/// `workflowRunUsageSchema`（真源 :152-165）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunUsage {
    /// 引擎 `usage-updated` 事件携带的已花总量。
    pub spent_tokens: i64,
    /// 本 run 已派发（dispatched）的节点数。
    pub nodes_used: i64,
    /// 撞上 `maxNodes` 被**拒之表外**的实例数（`truncated` 只说得出「有东西没进来」）。
    /// 零时整个键缺席。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_unlisted: Option<i64>,
    /// 其中已结算的条数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_unlisted_settled: Option<i64>,
}

/// `workflowRunStepCounts`（真源 workflow-runs-caps.ts:118-125）的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowRunStepCounts {
    /// 表内 + 表外（撞界后没进表的实例仍算步数）。
    pub total: i64,
    pub settled: i64,
}

/// `workflowRunStepCounts`（真源 workflow-runs-caps.ts:118-125）。
///
/// ★真源 caps.ts:10 —— **唯一允许的步数读法**：表内 + 表外。
/// 读面三处（run 卡、时间线摘要、TUI 镜像）共用这一计算，
/// 保证同一条 run 的显示步数一致。
pub fn workflow_run_step_counts(run: &WorkflowRunState) -> WorkflowRunStepCounts {
    let settled_in_list = run
        .nodes
        .iter()
        .filter(|n| n.phase == WorkflowRunNodePhase::Settled)
        .count() as i64;
    WorkflowRunStepCounts {
        total: run.nodes.len() as i64 + run.usage.nodes_unlisted.unwrap_or(0),
        settled: settled_in_list + run.usage.nodes_unlisted_settled.unwrap_or(0),
    }
}

/// `workflowRunArtifactKindSchema`（真源 workflow-artifacts.ts:31-38）。
///
/// ★闭集枚举——加值是**破坏性**的偏斜（旧读端整帧拒收），
/// 与 `workflowRuns[].status` 同一档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowRunArtifactKind {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "chart")]
    Chart,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "metrics")]
    Metrics,
    #[serde(rename = "board")]
    Board,
}

/// `workflowRunArtifactSummarySchema`（真源 workflow-artifacts.ts:125-137）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunArtifactSummary {
    pub id: String,
    pub kind: WorkflowRunArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_count: Option<i64>,
    /// run 的交付物（至多一件）。UI 据它排先后与选形态；缺席即不是。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
}

/// `workflowRunPhaseSchema`（已进入的阶段）。
///
/// ★真源 :115-118 —— `name` 是作者原词（时间线按它关联 display 的 `phases[].name`）；
/// `rounds` 是**进入次数**——单调（reducer 取 max），所以 resume 重放的前缀
/// 不会把它加倍。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunPhase {
    pub name: String,
    pub rounds: i64,
}

/// `workflowRunUnlistedPhaseSchema`（真源 :137-143）：界在某个**出生阶段**上花掉了多少。
///
/// ★真源 :124-135 注释——两个 run 级计数器说得出「总共少列了多少」，
/// 说不出少在**哪一站**——而读面是按站画的。`phaseName` 缺席= 无阶段那一格
/// （出生在任何 `phase()` 标记之前，或旧 CLI 没打戳）。`actors` 可加可减：
/// 一个被淘汰的子代理在下次被派活时会回到表上。零值的可选子键**缺席**——
/// 缺席说的是「零个」，不是「不知道」。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunUnlistedPhase {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_name: Option<String>,
    pub actors: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actors_settled: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actors_failed: Option<i64>,
    pub settled: i64,
}

/// `workflowRunPendingQuestionSchema`（真源 :342-358）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunPendingQuestion {
    /// 全局唯一的问题 id（形如 `dwfq-<runId 片段>-<seq>`）。
    pub qid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_site_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_ordinal: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_name: Option<String>,
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// 提问时刻（epoch 毫秒）。渲染侧按「有则显示等待时长」处理，缺席不是错误。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked_at: Option<i64>,
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
    fn step_counts_add_unlisted_nodes() {
        // ★真源 caps.ts:10 —— 唯一允许的步数读法：表内 + 表外。
        // 一个 3000 路 fan-out 的 run 显示成「1024 步」是一句假话。
        let raw = serde_json::json!({
            "runId": "r", "status": "running",
            "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok"},
                {"siteId": "s1", "ordinal": 1, "phase": "executing"}
            ],
            "usage": { "spentTokens": 42118, "nodesUsed": 2,
                       "nodesUnlisted": 1976, "nodesUnlistedSettled": 1900 }
        });
        let run: WorkflowRunState = serde_json::from_value(raw).unwrap();
        let counts = workflow_run_step_counts(&run);
        assert_eq!(counts.total, 2 + 1976);
        assert_eq!(counts.settled, 1 + 1900);
        assert_eq!(run.usage.spent_tokens, 42118);
    }

    #[test]
    fn step_counts_treat_absent_counters_as_zero() {
        let raw = serde_json::json!({
            "runId": "r", "status": "running", "actors": [], "nodes": [],
            "usage": { "spentTokens": 0, "nodesUsed": 0 }
        });
        let run: WorkflowRunState = serde_json::from_value(raw).unwrap();
        let counts = workflow_run_step_counts(&run);
        assert_eq!(counts.total, 0);
        assert_eq!(counts.settled, 0);
    }

    #[test]
    fn artifact_summary_parses_all_six_kinds() {
        // ★真源 artifacts.ts:31-38 —— 闭集枚举，加值是破坏性偏斜。
        for wire in ["file", "markdown", "chart", "table", "metrics", "board"] {
            let raw = serde_json::json!({
                "id": "a1", "kind": wire, "version": 2, "primary": true,
                "title": "报告", "contentType": "text/markdown", "bytes": 120, "itemCount": 4
            });
            let a: WorkflowRunArtifactSummary = serde_json::from_value(raw).unwrap();
            assert_eq!(a.id, "a1");
            assert_eq!(a.version, 2);
            assert_eq!(a.primary, Some(true));
            assert_eq!(a.item_count, Some(4));
        }
        // 表外枚举值必须被拒。
        let bad = serde_json::json!({ "id": "a", "kind": "pdf", "version": 1 });
        assert!(serde_json::from_value::<WorkflowRunArtifactSummary>(bad).is_err());
    }

    #[test]
    fn run_state_reads_artifacts_and_subagent_model() {
        let raw = serde_json::json!({
            "runId": "r", "status": "completed",
            "actors": [], "nodes": [],
            "usage": { "spentTokens": 10, "nodesUsed": 1 },
            "artifacts": [{ "id": "a1", "kind": "markdown", "version": 1 }],
            "subagentModel": "uuid-provider/sonnet$high"
        });
        let run: WorkflowRunState = serde_json::from_value(raw).unwrap();
        assert_eq!(run.artifacts.as_ref().unwrap().len(), 1);
        assert_eq!(
            run.subagent_model.as_deref(),
            Some("uuid-provider/sonnet$high"),
            "规范串：providerId/modelId[$reasoningLevel]"
        );
        // 缺席时是 None（跟随会话模型，没有可说的）。
        let bare = run_from_min();
        assert!(bare.artifacts.is_none());
        assert!(bare.subagent_model.is_none());
    }

    fn run_from_min() -> WorkflowRunState {
        serde_json::from_value(serde_json::json!({
            "runId": "r", "status": "running", "actors": [], "nodes": []
        }))
        .unwrap()
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
            "phases": [{ "name": "预备", "rounds": 2 }],
            "currentPhase": "预备",
            "phaseNames": ["预备", "主体"],
            "phaseAlongside": [[], [0]]
        });
        let run: WorkflowRunState = serde_json::from_value(raw).unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);
        assert_eq!(run.actors.len(), 1);
        assert_eq!(run.nodes[0].actor_site_id.as_deref(), Some("ask"));
        assert_eq!(run.phases.as_ref().unwrap()[0].name, "预备");
        assert_eq!(
            run.phases.as_ref().unwrap()[0].rounds,
            2,
            "★真源 :117 —— rounds 是进入次数"
        );
        assert_eq!(run.phase_names.as_ref().unwrap().len(), 2);
        // phaseAlongside[i] 落在 phaseNames 这张表上。
        assert_eq!(run.phase_alongside.as_ref().unwrap()[1], vec![0]);
    }

    #[test]
    fn run_state_no_run_blocks_absent() {
        // ★真源 :472/:476/:480 —— phases / phaseNames 零条时**整个键缺席**
        // （不是空数组）；unlistedByPhase / pendingQuestions / stalled 同规。
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
        assert_eq!(run.unlisted_by_phase, None);
        assert_eq!(run.pending_questions, None);
        assert_eq!(run.stalled, None);
    }

    #[test]
    fn unlisted_phase_zero_subkeys_are_absent() {
        // ★真源 :135 —— 零值的可选子键缺席：缺席说的是「零个」，
        // 不是「不知道」——还没跑完的那些因此留在 pending。
        let raw = serde_json::json!({
            "phaseName": "预备", "actors": 3, "settled": 7
        });
        let u: WorkflowRunUnlistedPhase = serde_json::from_value(raw).unwrap();
        assert_eq!(u.actors, 3);
        assert_eq!(u.settled, 7);
        assert_eq!(u.actors_settled, None, "零值子键缺席");
        assert_eq!(u.actors_failed, None);
    }

    #[test]
    fn unlisted_phase_may_have_no_stamp() {
        // ★真源 :132 —— phaseName 缺席 = 无阶段那一格。
        let raw = serde_json::json!({ "actors": 0, "settled": 0 });
        let u: WorkflowRunUnlistedPhase = serde_json::from_value(raw).unwrap();
        assert_eq!(u.phase_name, None);
    }

    #[test]
    fn pending_question_actor_binding_is_optional() {
        // 真源 :343-347 —— actorSiteId / actorOrdinal 各自可缺。
        let raw = serde_json::json!({
            "qid": "dwfq-abc-1", "question": "要继续吗？",
            "actorSiteId": "ask", "actorOrdinal": 2, "askedAt": 1730000000000i64
        });
        let q: WorkflowRunPendingQuestion = serde_json::from_value(raw).unwrap();
        assert_eq!(q.qid, "dwfq-abc-1");
        assert_eq!(q.actor_ordinal, Some(2));
        assert_eq!(q.actor_name, None);
        assert_eq!(q.asked_at, Some(1730000000000));
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