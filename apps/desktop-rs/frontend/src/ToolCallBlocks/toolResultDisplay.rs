//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/toolResultDisplay.ts`（320 行）。
//!
//! 工具结果的**装饰载荷**解析器：从 raw 的五个候选位置（result.display /
//! raw.display / metadata.display / rawOutput.display / output.display）
//! 找出第一个合法 display。真源用 shared 侧 strict schema——解析失败只丢
//! 这张卡的装饰载荷，不拒整条 row。
//!
//! ## 裁剪注明
//!
//! - 六个工作流 display schema（get_workflow_run / list_workflow_runs /
//!   eval_workflow_snippet / saved_workflow_list / list_models /
//!   resume_workflow_run）**全部** strict 解析，字段表逐条对
//!   `packages/shared/src/zcode-protocol-v4/workflow-observation-display.ts`。
//!   这些 struct 就地定义在本模块（与 `EvalWorkflowSnippetDisplay` /
//!   `CreateWorkflowDisplay` 的既有做法一致）：schema 文件的唯一消费面就是这里，
//!   另一侧消费者在 CLI contracts 进程，不在前端。
//! - 唯一放宽的是 `.strict()` 的「不容额外键」，理由见下面 `read_observation_status` 之前的说明。

use serde_json::Value;

use crate::components::workflow_graph::run_state::WorkflowRunStatus;

/// 本地 agent 消息结果（真源 :16-21）。
#[derive(Debug, Clone, PartialEq)]
pub struct LocalAgentMessageDisplay {
    pub status: String,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// TaskStop 结果（真源 :23-30）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskStopDisplay {
    pub task_id: String,
    pub task_type: String,
    pub command: Option<String>,
    pub message: String,
    pub truncated: Option<bool>,
}

/// TaskOutput 结果（真源 :32-38）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutputDisplay {
    pub retrieval_status: String,
    pub task_status: Option<String>,
    pub output: Option<String>,
    pub truncated: Option<bool>,
}

/// RespondToCoordinator 结果（真源 :40-43）。
#[derive(Debug, Clone, PartialEq)]
pub struct RespondToCoordinatorDisplay {
    pub status: String,
}

/// CUA 结果（真源 :45-65）的宽松形态：校验主字段，media/targetApp 保留原始 JSON。
#[derive(Debug, Clone, PartialEq)]
pub struct CuaDisplay {
    pub tool_name: String,
    pub status: String,
    pub structured_content: Option<String>,
    pub text: Option<String>,
    pub error_code: Option<String>,
    pub suggested_action: Option<String>,
    pub media: Value,
    pub truncated: Option<bool>,
    pub target_app: Value,
}

/// 一条 TS 诊断（真源 `diagnosticSchema`，workflow-observation-display.ts:116-123）。
///
/// 四字段全必填 + `.strict()`（不容额外键）；三个数值都是
/// `int().nonnegative()`；message 非空且 ≤2048。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowDiagnostic {
    pub line: i64,
    pub column: i64,
    pub code: i64,
    pub message: String,
}

/// `ToolCallEvalWorkflowSnippetDisplay`（真源 :195-207）。
///
/// 上限照抄 zod：diagnostics ≤100、logs ≤40 且每条 ≤1024、response ≤4000。
/// 超限即视为非法 display（真源 zod 会reject）。
#[derive(Debug, Clone, PartialEq)]
pub struct EvalWorkflowSnippetDisplay {
    pub ok: bool,
    pub diagnostics: Vec<WorkflowDiagnostic>,
    pub logs: Vec<String>,
    pub response: String,
    pub duration_ms: i64,
    pub truncated: Option<bool>,
}

/// zod 的 `.int().nonnegative()`：非整数或负数都非法。
fn is_nonnegative_int(value: Option<&Value>) -> Option<i64> {
    let n = value?.as_i64()?;
    (n >= 0).then_some(n)
}

/// `toolCallEvalWorkflowSnippetDisplaySchema`（真源 :195-207）的 strict 解析。
///
/// 独立成函数而非塞进 `parse_display` 的大分支：六个工作流 kind 里
/// 这个字段最复杂，单独成函数便于逐条对拍约束。
pub fn parse_eval_workflow_snippet_display(
    value: &Value,
) -> Option<EvalWorkflowSnippetDisplay> {
    if !is_record(value) {
        return None;
    }
    let ok = value.get("ok")?.as_bool()?;

    let raw_diagnostics = value.get("diagnostics")?.as_array()?;
    if raw_diagnostics.len() > 100 {
        return None;
    }
    let mut diagnostics = Vec::with_capacity(raw_diagnostics.len());
    for item in raw_diagnostics {
        if !is_record(item) {
            return None;
        }
        // strict()：只认这四个键。
        if item.as_object().is_some_and(|o| o.len() != 4) {
            return None;
        }
        let line = is_nonnegative_int(item.get("line"))?;
        let column = is_nonnegative_int(item.get("column"))?;
        let code = is_nonnegative_int(item.get("code"))?;
        let message = item.get("message")?.as_str()?;
        // z.string().min(1).max(2048)
        if message.is_empty() || message.chars().count() > 2048 {
            return None;
        }
        diagnostics.push(WorkflowDiagnostic {
            line,
            column,
            code,
            message: message.to_string(),
        });
    }

    let raw_logs = value.get("logs")?.as_array()?;
    if raw_logs.len() > 40 {
        return None;
    }
    let mut logs = Vec::with_capacity(raw_logs.len());
    for line in raw_logs {
        let s = line.as_str()?;
        // z.string().max(1024)
        if s.chars().count() > 1024 {
            return None;
        }
        logs.push(s.to_string());
    }

    let response = value.get("response")?.as_str()?;
    if response.chars().count() > 4000 {
        return None;
    }
    let duration_ms = is_nonnegative_int(value.get("durationMs"))?;
    // truncated: z.boolean().optional() —— zod 的 optional() **不接受 null**
    // （键必须缺席），存在但非 bool 即非法。
    let truncated = match value.get("truncated") {
        None => None,
        Some(v) => Some(v.as_bool()?),
    };

    Some(EvalWorkflowSnippetDisplay {
        ok,
        diagnostics,
        logs,
        response: response.to_string(),
        duration_ms,
        truncated,
    })
}

/// 工作流 display：六个 kind **全部** strict 解析（真源
/// `WORKFLOW_DISPLAY_PARSERS_BY_KIND` :89-102 的查表）。
///
/// 字段表逐条对 `packages/shared/src/zcode-protocol-v4/workflow-observation-display.ts`：
/// `z.object(...).strict()` 的必填/可选、`z.enum(...)` 的闭合词表、`z.number().int().nonnegative()`
/// 的数值界、`.max(n)` 的数组与字符串上限都在这里落。真源注释（该文件 :7-10）两侧都是 strict——
/// 少一个字段或多一个枚举值，整条 row/display 校验失败、工具卡退化成文本，所以这里也不能松。
///
/// ★与真源的唯一刻意差异：`.strict()` 不容额外键，这五个新解析器不判未知键
/// （同 `protocol/envelope.rs:7-9` 与 `conversation_stream.rs:188-190` 的既有约定：
/// agent 侧新增可选键不应让整块 display 失效、把卡退化成文本）。
/// 必填/可选、枚举词表、数值界与数组上限都照 zod 收紧，没有放宽。
/// （先迁的 `eval_workflow_snippet` 对嵌套 diagnostic 判了四键形状，保持原样不动。）

/// `WORKFLOW_RUN_OBSERVATION_STATUSES`（真源 :23-29）的词表就是
/// `components/workflow_graph/run_state.rs` 的 `WorkflowRunStatus`（同一份协议枚举），
/// 这里直接复用它——display 的 status 与投影 run 的 status、`readWorkflowRunStopReason`
/// 的入参必须是同一个类型，否则每张卡都要自己转一次，转换点就是漂移点。
fn read_observation_status(value: &Value) -> Option<WorkflowRunStatus> {
    match value.as_str()? {
        "pending" => Some(WorkflowRunStatus::Pending),
        "running" => Some(WorkflowRunStatus::Running),
        "completed" => Some(WorkflowRunStatus::Completed),
        "errored" => Some(WorkflowRunStatus::Errored),
        "stopped" => Some(WorkflowRunStatus::Stopped),
        _ => None,
    }
}

/// `WORKFLOW_RUN_STOP_REASONS`（真源 :15-21）。
const WORKFLOW_RUN_STOP_REASONS: [&str; 5] =
    ["user", "model", "provider", "interrupted", "superseded"];

fn read_stop_reason(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    WORKFLOW_RUN_STOP_REASONS.contains(&s).then(|| s.to_string())
}

/// `usageSchema`（真源 :31-39）：五件全必填的 `z.number()`（不要求整数）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunUsage {
    pub spent_tokens: f64,
    pub nodes_observed: f64,
    pub nodes_running: f64,
    pub nodes_completed: f64,
    pub nodes_failed: f64,
}

/// `actorSchema`（真源 :41-47）：`{siteId, ordinal, name?}`。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunActorRef {
    pub site_id: String,
    pub ordinal: f64,
    pub name: Option<String>,
}

/// `workflowRunPhaseViewSchema` 的 `state`（真源 :56）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunPhaseState {
    Done,
    Current,
    Ahead,
    Unfinished,
}

impl WorkflowRunPhaseState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Current => "current",
            Self::Ahead => "ahead",
            Self::Unfinished => "unfinished",
        }
    }
}

/// `workflowRunPhaseViewSchema`（真源 :53-63）：一条阶段行。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunPhaseView {
    pub name: String,
    pub state: WorkflowRunPhaseState,
    pub rounds: i64,
    pub nodes_settled: i64,
    pub nodes_running: i64,
    pub entered_at: Option<f64>,
    pub exited_at: Option<f64>,
}

/// `workflowRunLastToolSchema`（真源 :65-71）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunLastTool {
    pub name: String,
    pub target: Option<String>,
    pub at: Option<f64>,
}

/// `workflowRunSubagentViewSchema` 的 `state`（真源 :78）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunSubagentState {
    Idle,
    Executing,
    Waiting,
    Parked,
    Done,
    Failed,
    Unfinished,
}

impl WorkflowRunSubagentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Executing => "executing",
            Self::Waiting => "waiting",
            Self::Parked => "parked",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Unfinished => "unfinished",
        }
    }
}

/// `workflowRunSubagentViewSchema` 的 `waitCause`（真源 :85）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunWaitCause {
    Slot,
    Backoff,
}

/// `workflowRunSubagentViewSchema`（真源 :73-94）：情势截面的一个子代理。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunSubagentView {
    pub site_id: String,
    pub ordinal: i64,
    pub name: Option<String>,
    pub state: WorkflowRunSubagentState,
    pub phase_name: Option<String>,
    pub instructions_head: Option<String>,
    pub started_at: Option<f64>,
    pub turn: Option<i64>,
    pub tool_calls: Option<i64>,
    pub last_tool: Option<WorkflowRunLastTool>,
    pub wait_cause: Option<WorkflowRunWaitCause>,
    pub retry_after_ms: Option<f64>,
    pub wait_since: Option<f64>,
    pub parked_on: Option<String>,
    pub steps_settled: i64,
    pub steps_failed: i64,
    pub tokens: i64,
    pub last_progress_at: Option<f64>,
}

/// `workflowRunHealthSchema.concurrency`（真源 :100-108）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunConcurrencyHealth {
    pub effective: i64,
    /// `cap` 是 `int().positive()`：0 与负数都非法。
    pub cap: i64,
    pub reason: Option<String>,
    pub since: Option<f64>,
}

/// `workflowRunHealthSchema`（真源 :96-114）：健康截面。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunHealth {
    pub last_progress_at: Option<f64>,
    pub stalled_since: Option<f64>,
    pub concurrency: Option<WorkflowRunConcurrencyHealth>,
    pub consecutive_failures: i64,
    pub cached_steps: i64,
    /// `leftoverRunning` 是 `int().positive()`（在场时）。
    pub leftover_running: Option<i64>,
    /// `pendingQuestionsKnown`：必填布尔。
    pub pending_questions_known: bool,
}

/// `workflowRunSummaryRowSchema` 的 `labelSource`（真源 :129）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunLabelSource {
    Name,
    Script,
}

/// `workflowRunSummaryRowSchema`（真源 :125-138）：ListWorkflowRuns 的一行。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunSummaryRow {
    pub run_id: String,
    pub label: String,
    pub label_source: WorkflowRunLabelSource,
    pub status: WorkflowRunStatus,
    pub stop_reason: Option<String>,
    pub owned_by_this_session: bool,
    pub possibly_interrupted: Option<bool>,
    pub created_at: f64,
    pub updated_at: f64,
    pub spent_tokens: f64,
}

/// `toolCallGetWorkflowRunDisplaySchema.logTail` 的成员（真源 :158-170）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunLogTailEntry {
    pub sequence: f64,
    pub message: String,
    /// 事件落 journal 的时刻（epoch ms）；卡上的「多久以前」对 generatedAt 算（真源 :164-166）。
    pub at: Option<f64>,
}

/// `toolCallGetWorkflowRunDisplaySchema.error`（真源 :172-178）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunErrorPayload {
    pub code: String,
    pub message: String,
}

/// `ToolCallGetWorkflowRunDisplay`（真源 :140-182）。
#[derive(Debug, Clone, PartialEq)]
pub struct GetWorkflowRunDisplay {
    pub run_id: String,
    pub label: String,
    pub status: WorkflowRunStatus,
    pub stop_reason: Option<String>,
    pub possibly_interrupted: Option<bool>,
    /// 情势截面五件（summary / generatedAt / phases / subagents / health）**全部可选**：
    /// 情势上线前持久化的 transcript 载荷没有这些键，而 schema 是 strict 的——
    /// 设成必填会让升级后打开的每一条历史会话里这张卡整块被剥、退化成纯文本（真源 :148-150）。
    /// 构造侧每次仍然全填。
    pub summary: Option<String>,
    pub generated_at: Option<f64>,
    pub usage: WorkflowRunUsage,
    pub phases: Option<Vec<WorkflowRunPhaseView>>,
    pub subagents: Option<Vec<WorkflowRunSubagentView>>,
    pub health: Option<WorkflowRunHealth>,
    pub actors: Vec<WorkflowRunActorRef>,
    pub log_tail: Vec<WorkflowRunLogTailEntry>,
    pub result: Option<String>,
    pub error: Option<WorkflowRunErrorPayload>,
    pub truncated: Option<bool>,
}

/// `ToolCallListWorkflowRunsDisplay`（真源 :184-191）。
#[derive(Debug, Clone, PartialEq)]
pub struct ListWorkflowRunsDisplay {
    pub runs: Vec<WorkflowRunSummaryRow>,
    pub truncated: Option<bool>,
}

/// `toolCallSavedWorkflowListDisplaySchema.workflows` 的成员（真源 :214-221）。
#[derive(Debug, Clone, PartialEq)]
pub struct SavedWorkflowEntry {
    pub name: String,
    pub description: Option<String>,
    pub when_to_use: Option<String>,
    pub scope: String,
    pub path: String,
    pub arg_names: Vec<String>,
}

/// `toolCallSavedWorkflowListDisplaySchema.invalid` 的成员（真源 :228-232）。
#[derive(Debug, Clone, PartialEq)]
pub struct InvalidSavedWorkflow {
    pub path: String,
    pub reason: Option<String>,
}

/// `ToolCallSavedWorkflowListDisplay`（真源 :208-238）。
#[derive(Debug, Clone, PartialEq)]
pub struct SavedWorkflowListDisplay {
    pub workflows: Vec<SavedWorkflowEntry>,
    pub invalid: Option<Vec<InvalidSavedWorkflow>>,
    pub truncated: Option<bool>,
}

/// `toolCallListModelsDisplaySchema.models` 的成员（真源 :252-261）。
#[derive(Debug, Clone, PartialEq)]
pub struct ListModelsEntry {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub provider_label: Option<String>,
    pub reasoning_levels: Vec<String>,
    pub default_reasoning_level: Option<String>,
    pub context_window: Option<f64>,
    pub disabled_reason: Option<String>,
}

/// `ToolCallListModelsDisplay`（真源 :245-268）。
#[derive(Debug, Clone, PartialEq)]
pub struct ListModelsDisplay {
    pub current: Option<String>,
    pub models: Vec<ListModelsEntry>,
    pub truncated: Option<bool>,
}

/// `ToolCallResumeWorkflowRunDisplay`（真源 :273-278）。
///
/// 载荷刻意最小 `{runId}`（真源 :270-272：理由见 contracts 侧注释）。
#[derive(Debug, Clone, PartialEq)]
pub struct ResumeWorkflowRunDisplay {
    pub run_id: String,
}

// ── display 读侧的小原语（逐条对 zod）──

/// 必填非空字符串（`z.string().min(1)`）。
fn read_min1_string(value: &Value, key: &str) -> Option<String> {
    let s = value.get(key)?.as_str()?;
    (!s.is_empty()).then(|| s.to_string())
}

/// 可选字符串：键缺席 → `None`；键在场但不是字符串 → 非法（整体拒）。
/// 返回 `Option<Option<String>>`：外层 None 表示「非法，整体拒」，内层 None 表示「缺席」。
fn opt_str(value: &Value, key: &str, max: Option<usize>) -> Option<Option<String>> {
    let Some(candidate) = value.get(key) else {
        return Some(None);
    };
    let Some(s) = candidate.as_str() else {
        return None;
    };
    if max.is_some_and(|m| s.chars().count() > m) {
        return None;
    }
    Some(Some(s.to_string()))
}

/// 必填字符串（可带 min/max）。
fn req_str(value: &Value, key: &str, min1: bool, max: Option<usize>) -> Option<String> {
    let s = value.get(key)?.as_str()?;
    if min1 && s.is_empty() {
        return None;
    }
    if max.is_some_and(|m| s.chars().count() > m) {
        return None;
    }
    Some(s.to_string())
}

/// 必填 `z.number()`（不要求整数）。
fn req_number(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64()
}

/// 必填 `z.number().int().nonnegative()`。
fn req_nonnegative_int(value: &Value, key: &str) -> Option<i64> {
    let n = value.get(key)?.as_i64()?;
    (n >= 0).then_some(n)
}

/// 必填 `z.number().int().positive()`。
fn req_positive_int(value: &Value, key: &str) -> Option<i64> {
    let n = value.get(key)?.as_i64()?;
    (n > 0).then_some(n)
}

/// 可选 `z.number()`。
fn opt_number(value: &Value, key: &str) -> Option<Option<f64>> {
    let Some(candidate) = value.get(key) else {
        return Some(None);
    };
    Some(Some(candidate.as_f64()?))
}

/// 可选 `z.number().int().nonnegative()`。
fn opt_nonnegative_int(value: &Value, key: &str) -> Option<Option<i64>> {
    let Some(candidate) = value.get(key) else {
        return Some(None);
    };
    let n = candidate.as_i64()?;
    if n < 0 {
        return None;
    }
    Some(Some(n))
}

/// 可选 `z.boolean()`。
fn opt_bool(value: &Value, key: &str) -> Option<Option<bool>> {
    let Some(candidate) = value.get(key) else {
        return Some(None);
    };
    Some(Some(candidate.as_bool()?))
}

/// 必填 `z.boolean()`。
fn req_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

/// `z.array(...).max(n)` 的取数组与长度界。
fn read_array<'a>(value: &'a Value, key: &str, max: usize) -> Option<&'a [Value]> {
    let arr = value.get(key)?.as_array()?;
    (arr.len() <= max).then_some(arr.as_slice())
}

/// `usageSchema`（真源 :31-39）。
fn read_usage(value: &Value) -> Option<WorkflowRunUsage> {
    let usage = value.get("usage")?;
    if !is_record(usage) {
        return None;
    }
    Some(WorkflowRunUsage {
        spent_tokens: req_number(usage, "spentTokens")?,
        nodes_observed: req_number(usage, "nodesObserved")?,
        nodes_running: req_number(usage, "nodesRunning")?,
        nodes_completed: req_number(usage, "nodesCompleted")?,
        nodes_failed: req_number(usage, "nodesFailed")?,
    })
}

/// `actorSchema`（真源 :41-47）。
fn read_actor_ref(item: &Value) -> Option<WorkflowRunActorRef> {
    if !is_record(item) {
        return None;
    }
    Some(WorkflowRunActorRef {
        site_id: req_str(item, "siteId", false, None)?,
        ordinal: req_number(item, "ordinal")?,
        name: opt_str(item, "name", None)?,
    })
}

/// `workflowRunPhaseViewSchema`（真源 :53-63）。
fn read_phase_view(item: &Value) -> Option<WorkflowRunPhaseView> {
    if !is_record(item) {
        return None;
    }
    let state = match item.get("state")?.as_str()? {
        "done" => WorkflowRunPhaseState::Done,
        "current" => WorkflowRunPhaseState::Current,
        "ahead" => WorkflowRunPhaseState::Ahead,
        "unfinished" => WorkflowRunPhaseState::Unfinished,
        _ => return None,
    };
    Some(WorkflowRunPhaseView {
        name: read_min1_string(item, "name")?,
        state,
        rounds: req_nonnegative_int(item, "rounds")?,
        nodes_settled: req_nonnegative_int(item, "nodesSettled")?,
        nodes_running: req_nonnegative_int(item, "nodesRunning")?,
        entered_at: opt_number(item, "enteredAt")?,
        exited_at: opt_number(item, "exitedAt")?,
    })
}

/// `workflowRunLastToolSchema`（真源 :65-71）。
fn read_last_tool(value: &Value) -> Option<WorkflowRunLastTool> {
    if !is_record(value) {
        return None;
    }
    Some(WorkflowRunLastTool {
        name: read_min1_string(value, "name")?,
        target: opt_str(value, "target", Some(120))?,
        at: opt_number(value, "at")?,
    })
}

/// `workflowRunSubagentViewSchema`（真源 :73-94）。
fn read_subagent_view(item: &Value) -> Option<WorkflowRunSubagentView> {
    if !is_record(item) {
        return None;
    }
    let state = match item.get("state")?.as_str()? {
        "idle" => WorkflowRunSubagentState::Idle,
        "executing" => WorkflowRunSubagentState::Executing,
        "waiting" => WorkflowRunSubagentState::Waiting,
        "parked" => WorkflowRunSubagentState::Parked,
        "done" => WorkflowRunSubagentState::Done,
        "failed" => WorkflowRunSubagentState::Failed,
        "unfinished" => WorkflowRunSubagentState::Unfinished,
        _ => return None,
    };
    let wait_cause = match item.get("waitCause") {
        None => None,
        Some(v) => {
            let s = v.as_str()?;
            match s {
                "slot" => Some(WorkflowRunWaitCause::Slot),
                "backoff" => Some(WorkflowRunWaitCause::Backoff),
                _ => return None,
            }
        }
    };
    Some(WorkflowRunSubagentView {
        site_id: read_min1_string(item, "siteId")?,
        ordinal: req_nonnegative_int(item, "ordinal")?,
        name: opt_str(item, "name", Some(128))?,
        state,
        phase_name: opt_str(item, "phaseName", Some(128))?,
        instructions_head: opt_str(item, "instructionsHead", Some(240))?,
        started_at: opt_number(item, "startedAt")?,
        turn: opt_nonnegative_int(item, "turn")?,
        tool_calls: opt_nonnegative_int(item, "toolCalls")?,
        last_tool: match item.get("lastTool") {
            None => None,
            Some(v) => Some(read_last_tool(v)?),
        },
        wait_cause,
        retry_after_ms: opt_number(item, "retryAfterMs")?,
        wait_since: opt_number(item, "waitSince")?,
        parked_on: opt_str(item, "parkedOn", None)?,
        steps_settled: req_nonnegative_int(item, "stepsSettled")?,
        steps_failed: req_nonnegative_int(item, "stepsFailed")?,
        tokens: req_nonnegative_int(item, "tokens")?,
        last_progress_at: opt_number(item, "lastProgressAt")?,
    })
}

/// `workflowRunHealthSchema`（真源 :96-114）。
fn read_health(value: &Value) -> Option<WorkflowRunHealth> {
    if !is_record(value) {
        return None;
    }
    let concurrency = match value.get("concurrency") {
        None => None,
        Some(raw) => {
            if !is_record(raw) {
                return None;
            }
            Some(WorkflowRunConcurrencyHealth {
                effective: req_nonnegative_int(raw, "effective")?,
                cap: req_positive_int(raw, "cap")?,
                reason: opt_str(raw, "reason", Some(240))?,
                since: opt_number(raw, "since")?,
            })
        }
    };
    Some(WorkflowRunHealth {
        last_progress_at: opt_number(value, "lastProgressAt")?,
        stalled_since: opt_number(value, "stalledSince")?,
        concurrency,
        consecutive_failures: req_nonnegative_int(value, "consecutiveFailures")?,
        cached_steps: req_nonnegative_int(value, "cachedSteps")?,
        leftover_running: match value.get("leftoverRunning") {
            None => None,
            Some(v) => Some(v.as_i64().filter(|n| *n > 0)?),
        },
        pending_questions_known: req_bool(value, "pendingQuestionsKnown")?,
    })
}

/// `workflowRunSummaryRowSchema`（真源 :125-138）。
fn read_summary_row(item: &Value) -> Option<WorkflowRunSummaryRow> {
    if !is_record(item) {
        return None;
    }
    let label_source = match item.get("labelSource")?.as_str()? {
        "name" => WorkflowRunLabelSource::Name,
        "script" => WorkflowRunLabelSource::Script,
        _ => return None,
    };
    Some(WorkflowRunSummaryRow {
        run_id: read_min1_string(item, "runId")?,
        label: req_str(item, "label", false, None)?,
        label_source,
        status: read_observation_status(item.get("status")?)?,
        stop_reason: match item.get("stopReason") {
            None => None,
            Some(v) => Some(read_stop_reason(v)?),
        },
        owned_by_this_session: req_bool(item, "ownedByThisSession")?,
        possibly_interrupted: opt_bool(item, "possiblyInterrupted")?,
        created_at: req_number(item, "createdAt")?,
        updated_at: req_number(item, "updatedAt")?,
        spent_tokens: req_number(item, "spentTokens")?,
    })
}

/// `toolCallGetWorkflowRunDisplaySchema`（真源 :140-182）的 strict 解析。
pub fn parse_get_workflow_run_display(value: &Value) -> Option<GetWorkflowRunDisplay> {
    if !is_record(value) {
        return None;
    }
    if value.get("kind")?.as_str() != Some("get_workflow_run") {
        return None;
    }
    let phases = match value.get("phases") {
        None => None,
        Some(v) => {
            let items = v.as_array()?;
            if items.len() > 32 {
                return None;
            }
            Some(
                items
                    .iter()
                    .map(read_phase_view)
                    .collect::<Option<Vec<_>>>()?,
            )
        }
    };
    let subagents = match value.get("subagents") {
        None => None,
        Some(v) => {
            let items = v.as_array()?;
            if items.len() > 64 {
                return None;
            }
            Some(
                items
                    .iter()
                    .map(read_subagent_view)
                    .collect::<Option<Vec<_>>>()?,
            )
        }
    };
    let health = match value.get("health") {
        None => None,
        Some(v) => Some(read_health(v)?),
    };
    let log_tail = read_array(value, "logTail", 40)?;
    let mut log = Vec::with_capacity(log_tail.len());
    for item in log_tail {
        if !is_record(item) {
            return None;
        }
        log.push(WorkflowRunLogTailEntry {
            sequence: req_number(item, "sequence")?,
            message: req_str(item, "message", false, Some(1_024))?,
            at: opt_number(item, "at")?,
        });
    }
    Some(GetWorkflowRunDisplay {
        run_id: read_min1_string(value, "runId")?,
        label: req_str(value, "label", false, None)?,
        status: read_observation_status(value.get("status")?)?,
        stop_reason: match value.get("stopReason") {
            None => None,
            Some(v) => Some(read_stop_reason(v)?),
        },
        possibly_interrupted: opt_bool(value, "possiblyInterrupted")?,
        summary: opt_str(value, "summary", Some(400))?,
        generated_at: opt_number(value, "generatedAt")?,
        usage: read_usage(value)?,
        phases,
        subagents,
        health,
        actors: {
            let items = read_array(value, "actors", 32)?;
            items.iter().map(read_actor_ref).collect::<Option<Vec<_>>>()?
        },
        log_tail: log,
        result: opt_str(value, "result", Some(4_000))?,
        error: match value.get("error") {
            None => None,
            Some(raw) => {
                if !is_record(raw) {
                    return None;
                }
                Some(WorkflowRunErrorPayload {
                    code: req_str(raw, "code", false, None)?,
                    message: req_str(raw, "message", false, None)?,
                })
            }
        },
        truncated: opt_bool(value, "truncated")?,
    })
}

/// `toolCallListWorkflowRunsDisplaySchema`（真源 :184-191）的 strict 解析。
pub fn parse_list_workflow_runs_display(value: &Value) -> Option<ListWorkflowRunsDisplay> {
    if !is_record(value) {
        return None;
    }
    if value.get("kind")?.as_str() != Some("list_workflow_runs") {
        return None;
    }
    let runs = read_array(value, "runs", 50)?;
    Some(ListWorkflowRunsDisplay {
        runs: runs
            .iter()
            .map(read_summary_row)
            .collect::<Option<Vec<_>>>()?,
        truncated: opt_bool(value, "truncated")?,
    })
}

/// `toolCallSavedWorkflowListDisplaySchema`（真源 :208-238）的 strict 解析。
pub fn parse_saved_workflow_list_display(value: &Value) -> Option<SavedWorkflowListDisplay> {
    if !is_record(value) {
        return None;
    }
    if value.get("kind")?.as_str() != Some("saved_workflow_list") {
        return None;
    }
    let workflows = read_array(value, "workflows", 50)?;
    let mut items = Vec::with_capacity(workflows.len());
    for item in workflows {
        if !is_record(item) {
            return None;
        }
        let arg_names = read_array(item, "argNames", 32)?;
        items.push(SavedWorkflowEntry {
            name: read_min1_string(item, "name")?,
            description: opt_str(item, "description", Some(2_048))?,
            when_to_use: opt_str(item, "whenToUse", Some(2_048))?,
            scope: req_str(item, "scope", false, None)?,
            path: read_min1_string(item, "path")?,
            arg_names: arg_names
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()?,
        });
    }
    let invalid = match value.get("invalid") {
        None => None,
        Some(raw) => {
            let list = raw.as_array()?;
            if list.len() > 50 {
                return None;
            }
            let mut rows = Vec::with_capacity(list.len());
            for item in list {
                if !is_record(item) {
                    return None;
                }
                rows.push(InvalidSavedWorkflow {
                    path: read_min1_string(item, "path")?,
                    reason: opt_str(item, "reason", Some(1_024))?,
                });
            }
            Some(rows)
        }
    };
    Some(SavedWorkflowListDisplay {
        workflows: items,
        invalid,
        truncated: opt_bool(value, "truncated")?,
    })
}

/// `toolCallListModelsDisplaySchema`（真源 :245-268）的 strict 解析。
pub fn parse_list_models_display(value: &Value) -> Option<ListModelsDisplay> {
    if !is_record(value) {
        return None;
    }
    if value.get("kind")?.as_str() != Some("list_models") {
        return None;
    }
    let models = read_array(value, "models", 100)?;
    let mut rows = Vec::with_capacity(models.len());
    for item in models {
        if !is_record(item) {
            return None;
        }
        let levels = item.get("reasoningLevels")?.as_array()?;
        rows.push(ListModelsEntry {
            id: read_min1_string(item, "id")?,
            provider_id: read_min1_string(item, "providerId")?,
            model_id: read_min1_string(item, "modelId")?,
            provider_label: opt_str(item, "providerLabel", Some(2_048))?,
            reasoning_levels: levels
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()?,
            default_reasoning_level: opt_str(item, "defaultReasoningLevel", None)?,
            context_window: opt_number(item, "contextWindow")?,
            disabled_reason: opt_str(item, "disabledReason", Some(2_048))?,
        });
    }
    Some(ListModelsDisplay {
        current: opt_str(value, "current", None)?,
        models: rows,
        truncated: opt_bool(value, "truncated")?,
    })
}

/// `toolCallResumeWorkflowRunDisplaySchema`（真源 :273-278）的 strict 解析。
pub fn parse_resume_workflow_run_display(value: &Value) -> Option<ResumeWorkflowRunDisplay> {
    if !is_record(value) {
        return None;
    }
    if value.get("kind")?.as_str() != Some("resume_workflow_run") {
        return None;
    }
    Some(ResumeWorkflowRunDisplay {
        run_id: read_min1_string(value, "runId")?,
    })
}

/// 解析结果（真源 `ToolResultDisplay`，:67-78）。
///
/// 真源的 union 里六个工作流 display 各自是一个成员（不是一个 `Workflow` 装箱），
/// Rust 侧同样平铺成六个变体——消费方 `match` 到kind即拿到typed字段，
/// 不需要再从原始 JSON 里二次取键。
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResultDisplay {
    LocalAgentMessage(LocalAgentMessageDisplay),
    TaskStop(TaskStopDisplay),
    TaskOutput(TaskOutputDisplay),
    RespondToCoordinator(RespondToCoordinatorDisplay),
    Cua(CuaDisplay),
    GetWorkflowRun(GetWorkflowRunDisplay),
    ListWorkflowRuns(ListWorkflowRunsDisplay),
    EvalWorkflowSnippet(EvalWorkflowSnippetDisplay),
    SavedWorkflowList(SavedWorkflowListDisplay),
    ListModels(ListModelsDisplay),
    ResumeWorkflowRun(ResumeWorkflowRunDisplay),
}

fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// `readOptionalString`（真源 :112-121）：
/// 三态——None=字段缺失，Some(None)=存在但非法/空（null 哨兵），Some(Some(s))=合法。
fn read_optional_string(value: &Value, key: &str) -> Option<Option<String>> {
    let candidate = value.get(key)?;
    if candidate.is_null() {
        // TS: candidate === undefined 才算缺失；null 会走 typeof !== "string" → null 哨兵。
        return Some(None);
    }
    let Some(s) = candidate.as_str() else {
        return Some(None);
    };
    let normalized = s.trim();
    if normalized.is_empty() {
        Some(None)
    } else {
        // 真源返回未 trim 的原始值（:120）。
        Some(Some(s.to_string()))
    }
}

/// `parseDisplay`（真源 :123-267）。
fn parse_display(value: &Value) -> Option<ToolResultDisplay> {
    if !is_record(value) {
        return None;
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("local_agent_message") {
        let status = value.get("status").and_then(|s| s.as_str());
        if status != Some("success") && status != Some("failed") {
            return None;
        }
        // error/message：null 哨兵（存在但非法）拒；缺失合法（真源 :128-130）。
        let error = match read_optional_string(value, "error") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        let message = match read_optional_string(value, "message") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        return Some(ToolResultDisplay::LocalAgentMessage(
            LocalAgentMessageDisplay {
                status: status.unwrap().to_string(),
                error,
                message,
            },
        ));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("task_stop") {
        let task_id = read_optional_string(value, "taskId");
        let task_type = read_optional_string(value, "taskType");
        let command = read_optional_string(value, "command");
        let message = read_optional_string(value, "message");
        let truncated = value.get("truncated");
        // taskId/taskType/message 必填且必须合法（Some(None) 是 null 哨兵）。
        let (Some(Some(task_id)), Some(Some(task_type)), Some(Some(message))) =
            (task_id, task_type, message)
        else {
            return None;
        };
        // command：缺失合法、null 哨兵拒（真源 :148 `command === null`）。
        let command = match command {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        if let Some(t) = truncated {
            if !t.is_null() && !t.is_boolean() {
                return None;
            }
        }
        return Some(ToolResultDisplay::TaskStop(TaskStopDisplay {
            task_id,
            task_type,
            command,
            message,
            truncated: truncated.and_then(|t| t.as_bool()),
        }));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("task_output") {
        let retrieval = value.get("retrievalStatus").and_then(|s| s.as_str());
        if !matches!(
            retrieval,
            Some("success") | Some("not_ready") | Some("timeout")
        ) {
            return None;
        }
        // null 哨兵拒；缺失合法（真源 :172-173）。
        let task_status = match read_optional_string(value, "taskStatus") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        let output = match read_optional_string(value, "output") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        let truncated = value.get("truncated");
        // 长度守卫（真源 :177-179）：taskStatus ≤ 64、output ≤ 2000。
        if task_status.as_ref().is_some_and(|s| s.chars().count() > 64) {
            return None;
        }
        if output.as_ref().is_some_and(|s| s.chars().count() > 2_000) {
            return None;
        }
        // truncated 只允许 true 或缺省（真源 :180）。
        if let Some(t) = truncated {
            if !t.is_null() && t.as_bool() != Some(true) {
                return None;
            }
        }
        return Some(ToolResultDisplay::TaskOutput(TaskOutputDisplay {
            retrieval_status: retrieval.unwrap().to_string(),
            task_status,
            output,
            truncated: truncated.and_then(|t| t.as_bool()),
        }));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("respond_to_coordinator") {
        let status = value.get("status").and_then(|s| s.as_str());
        if status != Some("success") && status != Some("failed") {
            return None;
        }
        return Some(ToolResultDisplay::RespondToCoordinator(
            RespondToCoordinatorDisplay {
                status: status.unwrap().to_string(),
            },
        ));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("cua") {
        // 必填字段：缺失或 null 哨兵都拒（TS `!toolName` 语义）。
        let tool_name = match read_optional_string(value, "toolName") {
            Some(Some(s)) => s,
            _ => return None,
        };
        // 可缺省字段：缺失合法（缺省），null 哨兵拒（真源 :205/:233-236——
        // `legacyInput === null` 拒的是「存在但非法」，undefined 合法）。
        let optional_or_reject = |key: &str| -> Option<Option<String>> {
            match read_optional_string(value, key) {
                None => Some(None),
                Some(None) => None, // null 哨兵 → 整体拒
                Some(Some(s)) => Some(Some(s)),
            }
        };
        let Some(structured_content) = optional_or_reject("structuredContent") else {
            return None;
        };
        let Some(text) = optional_or_reject("text") else {
            return None;
        };
        let Some(error_code) = optional_or_reject("errorCode") else {
            return None;
        };
        let Some(suggested_action) = optional_or_reject("suggestedAction") else {
            return None;
        };
        // input 同可选语义（仅存在性校验，值不进模型）。
        if optional_or_reject("input").is_none() {
            return None;
        }
        let target_app = value.get("targetApp").cloned().unwrap_or(Value::Null);
        let media = value.get("media").cloned().unwrap_or(Value::Null);
        let status = value.get("status").and_then(|s| s.as_str());
        let schema_ok = value.get("schemaVersion").and_then(|v| v.as_i64()) == Some(1);
        if !schema_ok || !matches!(status, Some("success") | Some("failed")) {
            return None;
        }
        let truncated = value.get("truncated");
        if let Some(t) = truncated {
            if !t.is_null() && !t.is_boolean() {
                return None;
            }
        }
        return Some(ToolResultDisplay::Cua(CuaDisplay {
            tool_name,
            status: status.unwrap().to_string(),
            structured_content,
            text,
            error_code,
            suggested_action,
            media,
            truncated: truncated.and_then(|t| t.as_bool()),
            target_app,
        }));
    }

    // 工作流 kind（真源 `WORKFLOW_DISPLAY_PARSERS_BY_KIND` :89-102 的查表，新增 kind 只改这一处）。
    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .unwrap_or_default();
    match kind {
        "get_workflow_run" => {
            parse_get_workflow_run_display(value).map(ToolResultDisplay::GetWorkflowRun)
        }
        "list_workflow_runs" => {
            parse_list_workflow_runs_display(value).map(ToolResultDisplay::ListWorkflowRuns)
        }
        "eval_workflow_snippet" => {
            parse_eval_workflow_snippet_display(value).map(ToolResultDisplay::EvalWorkflowSnippet)
        }
        "saved_workflow_list" => {
            parse_saved_workflow_list_display(value).map(ToolResultDisplay::SavedWorkflowList)
        }
        "list_models" => parse_list_models_display(value).map(ToolResultDisplay::ListModels),
        "resume_workflow_run" => {
            parse_resume_workflow_run_display(value).map(ToolResultDisplay::ResumeWorkflowRun)
        }
        _ => None,
    }
}

/// `readToolResultDisplay`（真源 :299-320）：五候选位置按序取第一个合法者。
pub fn read_tool_result_display(raw: &Value) -> Option<ToolResultDisplay> {
    if !is_record(raw) {
        return None;
    }

    let result = raw.get("result").filter(|v| is_record(v));
    let metadata = raw.get("metadata").filter(|v| is_record(v));
    let raw_output = raw.get("rawOutput").filter(|v| is_record(v));
    let output = raw.get("output").filter(|v| is_record(v));
    let candidates = [
        result.and_then(|r| r.get("display")),
        raw.get("display"),
        metadata.and_then(|m| m.get("display")),
        raw_output.and_then(|r| r.get("display")),
        output.and_then(|o| o.get("display")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Some(display) = parse_display(candidate) {
            return Some(display);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_from_five_candidate_positions_in_order() {
        // 真源 :306-312 —— result.display 优先。
        let raw = json!({
            "result": { "display": { "kind": "respond_to_coordinator", "status": "success" } },
            "display": { "kind": "respond_to_coordinator", "status": "failed" },
        });
        assert_eq!(
            read_tool_result_display(&raw),
            Some(ToolResultDisplay::RespondToCoordinator(
                RespondToCoordinatorDisplay {
                    status: "success".into()
                }
            ))
        );
        // 无 result 时取 raw.display。
        let raw = json!({
            "display": { "kind": "respond_to_coordinator", "status": "failed" },
        });
        assert_eq!(
            read_tool_result_display(&raw),
            Some(ToolResultDisplay::RespondToCoordinator(
                RespondToCoordinatorDisplay {
                    status: "failed".into()
                }
            ))
        );
        // metadata.display 第三候选。
        let raw = json!({
            "metadata": { "display": { "kind": "respond_to_coordinator", "status": "success" } },
        });
        assert!(read_tool_result_display(&raw).is_some());
    }

    #[test]
    fn invalid_first_candidate_falls_through() {
        // 真源 :314-317 —— 非法候选跳过继续找。
        let raw = json!({
            "result": { "display": { "kind": "task_output", "retrievalStatus": "bogus" } },
            "rawOutput": { "display": { "kind": "task_output", "retrievalStatus": "success" } },
        });
        match read_tool_result_display(&raw) {
            Some(ToolResultDisplay::TaskOutput(d)) => assert_eq!(d.retrieval_status, "success"),
            other => panic!("期望 TaskOutput，得 {other:?}"),
        }
    }

    #[test]
    fn task_stop_requires_core_fields() {
        // 真源 :139-161 —— taskId/taskType/message 必填。
        let ok = json!({
            "kind": "task_stop", "taskId": "t1", "taskType": "agent", "message": "已停止",
        });
        match parse_display(&ok) {
            Some(ToolResultDisplay::TaskStop(d)) => {
                assert_eq!(d.task_id, "t1");
                assert_eq!(d.command, None);
            }
            other => panic!("期望 TaskStop，得 {other:?}"),
        }
        // 缺 message → 拒。
        let bad = json!({ "kind": "task_stop", "taskId": "t1", "taskType": "agent" });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn task_output_length_guards() {
        // 真源 :177-179 —— output 超 2000 字符拒。
        let long_output = "x".repeat(2_001);
        let bad = json!({
            "kind": "task_output", "retrievalStatus": "success", "output": long_output,
        });
        assert_eq!(parse_display(&bad), None);
        // taskStatus 超 64 拒。
        let long_status = "x".repeat(65);
        let bad = json!({
            "kind": "task_output", "retrievalStatus": "success", "taskStatus": long_status,
        });
        assert_eq!(parse_display(&bad), None);
        // truncated 只允许 true。
        let bad = json!({
            "kind": "task_output", "retrievalStatus": "success", "truncated": false,
        });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn workflow_kinds_now_require_their_real_fields() {
        // 本轮把六个 kind 全部换成 strict 字段表：只带 kind 的骨架不再被认领，
        // 真源 zod safeParse 同样 reject（工具卡退化成文本，不是画一张空卡）。
        for kind in [
            "get_workflow_run",
            "list_workflow_runs",
            "eval_workflow_snippet",
            "saved_workflow_list",
            "list_models",
            "resume_workflow_run",
        ] {
            let skeleton = json!({ "kind": kind });
            assert_eq!(
                parse_display(&skeleton),
                None,
                "{kind} 缺必填字段应整块拒收"
            );
        }
        // 未知 kind → None。
        let unknown = json!({ "kind": "no_such_kind" });
        assert_eq!(parse_display(&unknown), None);
    }

    #[test]
    fn resume_display_is_minimal_run_id() {
        // 真源 :273-278 —— 载荷刻意最小 {kind, runId}，runId 非空。
        let ok = json!({ "kind": "resume_workflow_run", "runId": "run-9" });
        assert_eq!(
            parse_display(&ok),
            Some(ToolResultDisplay::ResumeWorkflowRun(
                ResumeWorkflowRunDisplay {
                    run_id: "run-9".into()
                }
            ))
        );
        // runId 空串非法（.min(1)）。
        assert_eq!(
            parse_display(&json!({ "kind": "resume_workflow_run", "runId": "" })),
            None
        );
        // runId 缺席非法。
        assert_eq!(
            parse_display(&json!({ "kind": "resume_workflow_run" })),
            None
        );
    }

    /// 一张合法的 GetWorkflowRun display（必填件齐，可选件先都不给）。
    fn minimal_get_run_display() -> Value {
        json!({
            "kind": "get_workflow_run",
            "runId": "run-1",
            "label": "把改动跑一遍",
            "status": "running",
            "usage": {
                "spentTokens": 1200,
                "nodesObserved": 4,
                "nodesRunning": 1,
                "nodesCompleted": 2,
                "nodesFailed": 0,
            },
            "actors": [{ "siteId": "a1", "ordinal": 0 }],
            "logTail": [{ "sequence": 3, "message": "node started" }],
        })
    }

    #[test]
    fn get_run_display_situation_section_is_all_optional() {
        // ★真源 :148-150 —— summary / generatedAt / phases / subagents / health 全可选：
        // 情势上线前的历史 transcript 没有这些键，设成必填会让老会话里这张卡整块被剥。
        let parsed = parse_get_workflow_run_display(&minimal_get_run_display())
            .expect("最小合法载荷必须解析成功");
        assert_eq!(parsed.run_id, "run-1");
        assert_eq!(parsed.status, WorkflowRunStatus::Running);
        assert_eq!(parsed.usage.spent_tokens, 1200.0);
        assert_eq!(parsed.phases, None);
        assert_eq!(parsed.subagents, None);
        assert_eq!(parsed.health, None);
        assert_eq!(parsed.summary, None);
        assert_eq!(parsed.actors.len(), 1);
        assert_eq!(parsed.log_tail[0].at, None, "at 是情势上线后才有的列（真源 :164-166）");

        // 补齐可选件后逐字段读回。
        let mut full = minimal_get_run_display();
        let obj = full.as_object_mut().unwrap();
        obj.insert("phases".into(), json!([{
            "name": "compile", "state": "current", "rounds": 2,
            "nodesSettled": 3, "nodesRunning": 1, "enteredAt": 100.0,
        }]));
        obj.insert("health".into(), json!({
            "concurrency": { "effective": 2, "cap": 4 },
            "consecutiveFailures": 0, "cachedSteps": 1, "pendingQuestionsKnown": true,
        }));
        let parsed = parse_get_workflow_run_display(&full).expect("带情势截面的载荷");
        let phases = parsed.phases.expect("phases");
        assert_eq!(phases[0].state, WorkflowRunPhaseState::Current);
        assert_eq!(phases[0].entered_at, Some(100.0));
        assert_eq!(phases[0].exited_at, None);
        let health = parsed.health.expect("health");
        assert!(health.pending_questions_known);
        assert_eq!(health.concurrency.as_ref().expect("concurrency").cap, 4);
    }

    #[test]
    fn get_run_display_rejects_out_of_vocabulary_enums() {
        // z.enum 是闭合词表：词表外的值整块非法，不是读成 None。
        for (key, bad) in [
            ("status", "cancelled"),
            ("stopReason", "mystery"),
            ("runId", ""),
        ] {
            let mut value = minimal_get_run_display();
            value[key] = json!(bad);
            assert!(
                parse_get_workflow_run_display(&value).is_none(),
                "{key}={bad} 应被词表拦下"
            );
        }
        // phases[].state 同样闭合。
        let mut value = minimal_get_run_display();
        value["phases"] = json!([{
            "name": "compile", "state": "paused", "rounds": 0,
            "nodesSettled": 0, "nodesRunning": 0,
        }]);
        assert!(parse_get_workflow_run_display(&value).is_none());
        // status 词表五值都能过。
        for status in ["pending", "running", "completed", "errored", "stopped"] {
            let mut value = minimal_get_run_display();
            value["status"] = json!(status);
            assert!(
                parse_get_workflow_run_display(&value).is_some(),
                "{status} 是合法态"
            );
        }
    }

    #[test]
    fn get_run_display_enforces_numeric_and_size_bounds() {
        // 逐条对 zod：int().nonnegative() / int().positive() / .max(n) / 字符串上限。
        let mut missing_usage = minimal_get_run_display();
        missing_usage.as_object_mut().unwrap().remove("usage");
        assert!(parse_get_workflow_run_display(&missing_usage).is_none(), "usage 必填");

        // usage 的五个数全必填（漏一个就整块拒——真源同款）。
        for key in [
            "spentTokens",
            "nodesObserved",
            "nodesRunning",
            "nodesCompleted",
            "nodesFailed",
        ] {
            let mut value = minimal_get_run_display();
            value["usage"].as_object_mut().unwrap().remove(key);
            assert!(
                parse_get_workflow_run_display(&value).is_none(),
                "usage.{key} 必填"
            );
        }

        // rounds 是 int().nonnegative()：负数与非整数都非法。
        for bad in [-1.0, 1.5] {
            let mut value = minimal_get_run_display();
            value["phases"] = json!([{
                "name": "compile", "state": "done", "rounds": bad,
                "nodesSettled": 0, "nodesRunning": 0,
            }]);
            assert!(
                parse_get_workflow_run_display(&value).is_none(),
                "rounds={bad} 应非法"
            );
        }

        // concurrency.cap 是 int().positive()：0 非法。
        let mut zero_cap = minimal_get_run_display();
        zero_cap["health"] = json!({
            "consecutiveFailures": 0, "cachedSteps": 0, "pendingQuestionsKnown": false,
            "concurrency": { "effective": 0, "cap": 0 },
        });
        assert!(parse_get_workflow_run_display(&zero_cap).is_none(), "cap=0 不是正整数");

        // pendingQuestionsKnown 必填布尔。
        let mut no_pending = minimal_get_run_display();
        no_pending["health"] = json!({
            "consecutiveFailures": 0, "cachedSteps": 0,
        });
        assert!(parse_get_workflow_run_display(&no_pending).is_none());

        // logTail ≤40、actors ≤32、phases ≤32、subagents ≤64。
        let mut too_many_logs = minimal_get_run_display();
        too_many_logs["logTail"] = json!(vec![json!({"sequence": 1, "message": "x"}); 41]);
        assert!(parse_get_workflow_run_display(&too_many_logs).is_none());

        let mut too_many_actors = minimal_get_run_display();
        too_many_actors["actors"] = json!(vec![json!({"siteId": "s", "ordinal": 0}); 33]);
        assert!(parse_get_workflow_run_display(&too_many_actors).is_none());

        // summary ≤400。
        let mut long_summary = minimal_get_run_display();
        long_summary["summary"] = json!("x".repeat(401));
        assert!(parse_get_workflow_run_display(&long_summary).is_none());

        // logTail.message ≤1024。
        let mut long_log = minimal_get_run_display();
        long_log["logTail"] = json!([{ "sequence": 1, "message": "x".repeat(1025) }]);
        assert!(parse_get_workflow_run_display(&long_log).is_none());
    }

    #[test]
    fn get_run_display_subagent_section_is_strict() {
        let mut value = minimal_get_run_display();
        value["subagents"] = json!([{
            "siteId": "a1", "ordinal": 0, "state": "executing",
            "stepsSettled": 2, "stepsFailed": 0, "tokens": 300,
            "lastTool": { "name": "Bash", "target": "cargo test", "at": 55.0 },
            "waitCause": "backoff", "retryAfterMs": 2000.0,
        }]);
        let parsed = parse_get_workflow_run_display(&value).expect("合法子代理截面");
        let subagent = &parsed.subagents.as_ref().expect("subagents")[0];
        assert_eq!(subagent.state, WorkflowRunSubagentState::Executing);
        assert_eq!(subagent.tokens, 300);
        assert_eq!(
            subagent.last_tool.as_ref().expect("lastTool").target.as_deref(),
            Some("cargo test")
        );
        assert_eq!(subagent.wait_cause, Some(WorkflowRunWaitCause::Backoff));
        assert_eq!(subagent.retry_after_ms, Some(2000.0));

        // stepsSettled 必填（不在 optional 名单里）——漏了就整块拒。
        let mut missing = value.clone();
        missing["subagents"][0]
            .as_object_mut()
            .unwrap()
            .remove("stepsSettled");
        assert!(parse_get_workflow_run_display(&missing).is_none());
        // siteId 是 .min(1)。
        let mut blank = value.clone();
        blank["subagents"][0]["siteId"] = json!("");
        assert!(parse_get_workflow_run_display(&blank).is_none());
    }

    #[test]
    fn list_runs_display_reads_summary_rows() {
        // 真源 :184-191 + :125-138。
        let ok = json!({
            "kind": "list_workflow_runs",
            "runs": [{
                "runId": "r1", "label": "名字", "labelSource": "name",
                "status": "completed", "ownedByThisSession": true,
                "createdAt": 1.0, "updatedAt": 2.0, "spentTokens": 10.0,
            }],
        });
        let parsed = parse_list_workflow_runs_display(&ok).expect("合法列表载荷");
        assert_eq!(parsed.runs.len(), 1);
        assert_eq!(parsed.runs[0].label_source, WorkflowRunLabelSource::Name);
        assert_eq!(parsed.runs[0].status, WorkflowRunStatus::Completed);
        assert_eq!(parsed.runs[0].possibly_interrupted, None);
        assert_eq!(parsed.truncated, None);

        // runs 必填且 ≤50。
        assert_eq!(
            parse_list_workflow_runs_display(&json!({ "kind": "list_workflow_runs" })),
            None,
            "runs 键缺席不是「空列表」，而是非法载荷"
        );
        let too_many = json!({
            "kind": "list_workflow_runs",
            "runs": vec![json!({
                "runId": "r", "label": "l", "labelSource": "script", "status": "pending",
                "ownedByThisSession": false, "createdAt": 0.0, "updatedAt": 0.0,
                "spentTokens": 0.0,
            }); 51],
        });
        assert!(parse_list_workflow_runs_display(&too_many).is_none());
        // labelSource 词表只有 name / script。
        let bad_source = json!({
            "kind": "list_workflow_runs",
            "runs": [{
                "runId": "r", "label": "l", "labelSource": "guess", "status": "pending",
                "ownedByThisSession": false, "createdAt": 0.0, "updatedAt": 0.0,
                "spentTokens": 0.0,
            }],
        });
        assert!(parse_list_workflow_runs_display(&bad_source).is_none());
    }

    #[test]
    fn saved_workflow_list_display_reads_entries_and_invalid() {
        // 真源 :208-238。
        let ok = json!({
            "kind": "saved_workflow_list",
            "workflows": [{
                "name": "review", "scope": "workspace", "path": ".zcode/wf/review.js",
                "argNames": ["target"], "description": "看一遍",
            }],
            "invalid": [{ "path": ".zcode/wf/broken.js" }],
        });
        let parsed = parse_saved_workflow_list_display(&ok).expect("合法可复用清单");
        assert_eq!(parsed.workflows[0].name, "review");
        assert_eq!(parsed.workflows[0].when_to_use, None);
        assert_eq!(parsed.workflows[0].arg_names, vec!["target".to_string()]);
        assert_eq!(
            parsed.invalid.as_ref().expect("invalid")[0].path,
            ".zcode/wf/broken.js"
        );
        assert!(parsed.invalid.as_ref().unwrap()[0].reason.is_none());

        // argNames 必填（可以为空数组，但键不能缺）。
        let mut missing_args = ok.clone();
        missing_args["workflows"][0]
            .as_object_mut()
            .unwrap()
            .remove("argNames");
        assert!(parse_saved_workflow_list_display(&missing_args).is_none());
        // workflows ≤50。
        let too_many = json!({
            "kind": "saved_workflow_list",
            "workflows": vec![json!({
                "name": "n", "scope": "s", "path": "p", "argNames": [],
            }); 51],
        });
        assert!(parse_saved_workflow_list_display(&too_many).is_none());
    }

    #[test]
    fn list_models_display_reads_the_catalog() {
        // 真源 :245-268。
        let ok = json!({
            "kind": "list_models",
            "current": "openai:gpt-5",
            "models": [{
                "id": "openai:gpt-5", "providerId": "openai", "modelId": "gpt-5",
                "providerLabel": "OpenAI", "reasoningLevels": ["low", "high"],
                "defaultReasoningLevel": "low", "contextWindow": 400_000.0,
            }],
        });
        let parsed = parse_list_models_display(&ok).expect("合法模型目录");
        assert_eq!(parsed.current.as_deref(), Some("openai:gpt-5"));
        let model = &parsed.models[0];
        assert_eq!(model.provider_id, "openai");
        assert_eq!(model.provider_label.as_deref(), Some("OpenAI"));
        assert_eq!(model.reasoning_levels, vec!["low", "high"]);
        assert_eq!(model.context_window, Some(400_000.0));
        assert_eq!(model.disabled_reason, None);

        // id / providerId / modelId 都是 .min(1)：任一为空整块拒。
        for key in ["id", "providerId", "modelId"] {
            let mut blank = ok.clone();
            blank["models"][0][key] = json!("");
            assert!(
                parse_list_models_display(&blank).is_none(),
                "{key} 空串应非法"
            );
        }
        // reasoningLevels 必填数组（可为空）。
        let mut missing_levels = ok.clone();
        missing_levels["models"][0]
            .as_object_mut()
            .unwrap()
            .remove("reasoningLevels");
        assert!(parse_list_models_display(&missing_levels).is_none());
        // models ≤100。
        let too_many = json!({
            "kind": "list_models",
            "models": vec![json!({
                "id": "i", "providerId": "p", "modelId": "m", "reasoningLevels": [],
            }); 101],
        });
        assert!(parse_list_models_display(&too_many).is_none());
    }

    #[test]
    fn five_workflow_displays_reach_the_render_layer_typed() {
        // 消费方 match 到变体即拿到字段，不需要再从原始 JSON 二次取键。
        let raw = json!({ "result": { "display": minimal_get_run_display() } });
        match read_tool_result_display(&raw) {
            Some(ToolResultDisplay::GetWorkflowRun(d)) => assert_eq!(d.run_id, "run-1"),
            other => panic!("期望 GetWorkflowRun 形态，得 {other:?}"),
        }
        // 非法载荷在 parse_display 这一层就被拦掉（等价于真源的 safeParse 失败）。
        let bad = json!({ "result": { "display": { "kind": "get_workflow_run" } } });
        assert_eq!(read_tool_result_display(&bad), None);
    }

    fn valid_snippet_display() -> Value {
        json!({
            "kind": "eval_workflow_snippet",
            "ok": true,
            "diagnostics": [],
            "logs": ["a"],
            "response": "r",
            "durationMs": 12,
        })
    }

    #[test]
    fn eval_snippet_display_uses_strict_schema() {
        // 真源 zod schema：kind/ok/diagnostics/logs/response/durationMs 全必填。
        let parsed = parse_eval_workflow_snippet_display(&valid_snippet_display()).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.duration_ms, 12);
        assert_eq!(parsed.truncated, None, "truncated 可选");

        // 缺任一必填字段 → None。
        for missing in ["ok", "diagnostics", "logs", "response", "durationMs"] {
            let mut value = valid_snippet_display();
            value.as_object_mut().unwrap().remove(missing);
            assert!(
                parse_eval_workflow_snippet_display(&value).is_none(),
                "缺 {missing} 应非法"
            );
        }
    }

    #[test]
    fn eval_snippet_display_enforces_limits() {
        // 真源：diagnostics ≤100、logs ≤40（每条 ≤1024）、response ≤4000。
        let mut too_many_diags = valid_snippet_display();
        let diag = json!({ "line": 1, "column": 1, "code": 9001, "message": "x" });
        too_many_diags["diagnostics"] = json!(vec![diag.clone(); 101]);
        assert!(parse_eval_workflow_snippet_display(&too_many_diags).is_none());

        let mut too_many_logs = valid_snippet_display();
        too_many_logs["logs"] = json!(vec!["x"; 41]);
        assert!(parse_eval_workflow_snippet_display(&too_many_logs).is_none());

        let mut long_log = valid_snippet_display();
        long_log["logs"] = json!(["x".repeat(1025)]);
        assert!(parse_eval_workflow_snippet_display(&long_log).is_none());

        let mut long_response = valid_snippet_display();
        long_response["response"] = json!("x".repeat(4001));
        assert!(parse_eval_workflow_snippet_display(&long_response).is_none());
    }

    #[test]
    fn eval_snippet_display_rejects_invalid_scalars() {
        // 真源 zod：三个数值都是 int().nonnegative()。
        for field in ["line", "column", "code"] {
            let mut value = valid_snippet_display();
            value["diagnostics"] = json!([{ "line": 1, "column": 1, "code": 9001, "message": "x" }]);
            value["diagnostics"][0][field] = json!(-1);
            assert!(
                parse_eval_workflow_snippet_display(&value).is_none(),
                "{field} 为负应非法"
            );
        }
        // message 非空（min(1)）。
        let mut empty_msg = valid_snippet_display();
        empty_msg["diagnostics"] =
            json!([{ "line": 1, "column": 1, "code": 9001, "message": "" }]);
        assert!(parse_eval_workflow_snippet_display(&empty_msg).is_none());

        // durationMs 负数非法。
        let mut neg_duration = valid_snippet_display();
        neg_duration["durationMs"] = json!(-1);
        assert!(parse_eval_workflow_snippet_display(&neg_duration).is_none());

        // truncated 存在但非 bool → 非法（zod optional() 不接受 null）。
        let mut bad_truncated = valid_snippet_display();
        bad_truncated["truncated"] = json!(null);
        assert!(parse_eval_workflow_snippet_display(&bad_truncated).is_none());
    }

    #[test]
    fn eval_snippet_diagnostic_is_strict() {
        // 真源 diagnosticSchema.strict() —— 不容额外键。
        let mut extra = valid_snippet_display();
        extra["diagnostics"] =
            json!([{ "line": 1, "column": 1, "code": 9001, "message": "x", "extra": 1 }]);
        assert!(parse_eval_workflow_snippet_display(&extra).is_none());
    }

    #[test]
    fn eval_snippet_display_reaches_render_layer() {
        // 经 parse_display 也要能拿到（不是只有直调解析函数才行）。
        match parse_display(&valid_snippet_display()) {
            Some(ToolResultDisplay::EvalWorkflowSnippet(d)) => {
                assert!(d.ok);
                assert_eq!(d.duration_ms, 12);
            }
            other => panic!("期望 EvalWorkflowSnippet 形态，得 {other:?}"),
        }
        // 非法 payload 走parse_display 时被 strict 拦掉。
        let bad = json!({ "kind": "eval_workflow_snippet", "ok": true });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn cua_display_core_validation() {
        let ok = json!({
            "kind": "cua", "schemaVersion": 1, "toolName": "screenshot",
            "status": "success", "text": "done",
        });
        assert!(matches!(
            parse_display(&ok),
            Some(ToolResultDisplay::Cua(_))
        ));
        // schemaVersion 必须 1。
        let bad = json!({
            "kind": "cua", "schemaVersion": 2, "toolName": "screenshot",
            "status": "success",
        });
        assert_eq!(parse_display(&bad), None);
        // toolName 非字符串 → 拒。
        let bad = json!({ "kind": "cua", "schemaVersion": 1, "toolName": 42, "status": "success" });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn malformed_raw_is_safe() {
        for bad in [json!(null), json!("字符串"), json!(42), json!({})] {
            assert_eq!(read_tool_result_display(&bad), None);
        }
    }
}
