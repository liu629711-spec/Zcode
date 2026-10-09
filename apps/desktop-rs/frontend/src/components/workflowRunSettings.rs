//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/workflowRunSettings.ts`（231 行）。
//!
//! 「配置」弹层的纯规则。
//!
//! ★真源 :2-5 注释——弹层组件只管画与接线，这里是它的全部判断：哪些 run 能配、
//! 表单从哪儿起步、Apply 发什么、被拒的 ACK 说哪句话。纯函数、不碰 store，逐条可穷举。

use crate::components::subagent_model_label::{ModelSelection, parse_model_picker_value};

use crate::components::workflow_graph::run_state::{
    WorkflowRunState, WorkflowRunStatus, WorkflowRunStopReason,
};

// ===========================================================================
// 会话模型
// ===========================================================================

/// 一条模型选择（`providerId` + `modelId`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChoice {
    pub provider_id: String,
    pub model_id: String,
}

/// `workflowSessionModelOf`（真源 :20-33）：会话当前模型。
///
/// ★真源 :17-19 —— 优先会话持久的稀疏选择，退回 UI effective 的 provider / model 投影；
/// 两者都读不出即缺席（「会话模型」那一项的名字从它来）。
pub fn workflow_session_model_of(
    model_selection: Option<&ModelChoice>,
    provider: Option<&str>,
    model: Option<&str>,
) -> Option<ModelChoice> {
    if let Some(selection) = model_selection {
        return Some(selection.clone());
    }
    let provider_id = provider.unwrap_or("").trim();
    let model_id = model.unwrap_or("").trim();
    if !provider_id.is_empty() && !model_id.is_empty() {
        Some(ModelChoice {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        })
    } else {
        None
    }
}

// ===========================================================================
// 表单模型
// ===========================================================================

/// `WorkflowRunSettingsModel`（真源 :36-38）：会话模型，或一个具体模型（可带思考档）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRunSettingsModel {
    Session,
    Model {
        provider_id: String,
        model_id: String,
        level: Option<String>,
    },
}

/// `workflowRunSettingsModelOf`（真源 :78-95）：规范串 → 表单模型。
///
/// ★真源 :77 —— 解析不动（坏串）按会话模型处理**不成立**，所以原样保留成
/// 一个查不到的具体模型（`providerId` 空串）。
pub fn workflow_run_settings_model_of(canonical: Option<&str>) -> WorkflowRunSettingsModel {
    let text = canonical.map(str::trim).filter(|t| !t.is_empty());
    let Some(text) = text else {
        return WorkflowRunSettingsModel::Session;
    };
    match parse_model_picker_value(text) {
        Some(ModelSelection {
            provider_id,
            model_id,
            reasoning_level,
        }) => WorkflowRunSettingsModel::Model {
            provider_id,
            model_id,
            level: reasoning_level,
        },
        // 解析失败不抛（UI 不是第二个解析器），保留成查不到的具体模型。
        None => WorkflowRunSettingsModel::Model {
            provider_id: String::new(),
            model_id: text.to_string(),
            level: None,
        },
    }
}

/// `formatModelPickerValue`（真源 model-selection.ts:36-42）。
fn format_model_picker_value(selection: &ModelSelection) -> String {
    let base = format!("{}/{}", selection.provider_id, selection.model_id);
    match &selection.reasoning_level {
        Some(level) => format!("{base}${level}"),
        None => base,
    }
}

/// `workflowRunSettingsModelCanonical`（真源 :98-108）：表单模型 → 规范串。
///
/// 会话模型没有串（`None`）。
pub fn workflow_run_settings_model_canonical(
    model: &WorkflowRunSettingsModel,
) -> Option<String> {
    match model {
        WorkflowRunSettingsModel::Session => None,
        WorkflowRunSettingsModel::Model {
            provider_id,
            model_id,
            level,
        } => {
            // ★真源 :102 —— 空 providerId 时只给 modelId（那是个查不到的具体模型）。
            if provider_id.is_empty() {
                return Some(model_id.clone());
            }
            Some(format_model_picker_value(&ModelSelection {
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
                reasoning_level: level.clone(),
            }))
        }
    }
}

// ===========================================================================
// 草稿与变更
// ===========================================================================

/// `WorkflowRunSettingsDraft`（真源 :41-44）：表单的两项设置。
///
/// ★`bound` 为 `None` 即「本 run 没有自己的界」（跑在本机上限上）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunSettingsDraft {
    pub model: WorkflowRunSettingsModel,
    pub bound: Option<i64>,
}

/// `WorkflowRunSettingsChange`（真源 :47）：Apply 发出去的那部分载荷
/// （`workId` 由宿主补）。
///
/// ★两项守工具的同一条三态规则：**省略 = 沿用**，`null` = 回到默认。
/// GUI 只发用户改过的那几项。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowRunSettingsChange {
    /// 规范串；`Some(None)` = 子代理回到会话模型。
    pub subagent_model: Option<Option<String>>,
    /// `Some(None)` = 解除本 run 自己的界（回到本机上限）。
    pub max_concurrency: Option<Option<i64>>,
}

impl WorkflowRunSettingsChange {
    /// 载荷里**只有**并发上限这一个键。
    fn is_concurrency_only(&self) -> bool {
        self.subagent_model.is_none() && self.max_concurrency.is_some()
    }

    /// 载荷是否为空（真源 :142 的 `Object.keys(change).length === 0`）。
    pub fn is_empty(&self) -> bool {
        self.subagent_model.is_none() && self.max_concurrency.is_none()
    }
}

/// `isWorkflowRunConfigurable`（真源 :55-67）：哪些 run 能配置。
///
/// ★真源 :50-54 —— pending / running 能（节流或换模型的主场景）；
/// stopped 能，除非是被一次修订替代掉的（活的是它的后继）；
/// errored 能（换个模型重试是最常见的修复）；
/// completed 不能（每个 ask 都会从缓存重放，没有东西会在新设置下跑）；
/// 不在投影里的 run 没有设置可显示。宿主回调与灰度门由调用方另叠。
pub fn is_workflow_run_configurable(run: Option<&WorkflowRunState>) -> bool {
    let Some(run) = run else {
        return false;
    };
    match run.status {
        WorkflowRunStatus::Pending | WorkflowRunStatus::Running | WorkflowRunStatus::Errored => {
            true
        }
        WorkflowRunStatus::Stopped => {
            run.superseded_by.is_none() && run.stop_reason != Some(WorkflowRunStopReason::Superseded)
        }
        WorkflowRunStatus::Completed => false,
    }
}

/// `workflowRunSettingsCeiling`（真源 :73-75）：本机的并发天花板。
///
/// ★优先 `run.concurrency_ceiling`（`run-started` 随带、恒在），
/// 老 CLI 没发它时退回读数芯片自己的水位 `concurrency.ceiling`。
/// 都没有即未知——步进器没有上限、不写提示。
pub fn workflow_run_settings_ceiling(run: &WorkflowRunState) -> Option<i64> {
    run.concurrency_ceiling
        .or_else(|| run.concurrency.as_ref().map(|c| c.ceiling))
}

/// `initialWorkflowRunSettingsDraft`（真源 :111-117）：打开弹层时的起点。
///
/// 两项都取 run 自己的当前设置。界缺席时停在天花板上（天花板也未知则为 `None`）。
pub fn initial_workflow_run_settings_draft(run: &WorkflowRunState) -> WorkflowRunSettingsDraft {
    let limit = run.concurrency.as_ref().and_then(|c| c.limit);
    WorkflowRunSettingsDraft {
        model: workflow_run_settings_model_of(run.subagent_model.as_deref()),
        bound: limit.or_else(|| workflow_run_settings_ceiling(run)),
    }
}

/// `normalizedBound`（真源 :120-123）：界的归一。
///
/// 达到或超过天花板即「没有自己的界」。
fn normalized_bound(bound: Option<i64>, ceiling: Option<i64>) -> Option<i64> {
    let bound = bound?;
    match ceiling {
        Some(c) if bound >= c => None,
        _ => Some(bound),
    }
}

/// `workflowRunSettingsChange`（真源 :130-143）：Apply 发什么。
///
/// ★只发**改过的**那几项。模型按规范串比较——只改思考档也算改了模型；
/// 回到会话模型发 `null`。界等于天花板发 `null`（解除本 run 自己的界）。
/// 两项都没变 → `None`（Apply 禁用）。
pub fn workflow_run_settings_change(
    initial: &WorkflowRunSettingsDraft,
    draft: &WorkflowRunSettingsDraft,
    ceiling: Option<i64>,
) -> Option<WorkflowRunSettingsChange> {
    let mut change = WorkflowRunSettingsChange::default();
    let from_model = workflow_run_settings_model_canonical(&initial.model);
    let to_model = workflow_run_settings_model_canonical(&draft.model);
    if from_model != to_model {
        change.subagent_model = Some(to_model);
    }
    let from_bound = normalized_bound(initial.bound, ceiling);
    let to_bound = normalized_bound(draft.bound, ceiling);
    if from_bound != to_bound {
        change.max_concurrency = Some(to_bound);
    }
    if change.is_empty() { None } else { Some(change) }
}

/// `clampWorkflowRunSettingsBound`（真源 :146-149）：步进器夹界。
///
/// 下限 1，上限天花板（未知则不设上限）。
pub fn clamp_workflow_run_settings_bound(value: f64, ceiling: Option<i64>) -> i64 {
    let floor = 1.0_f64.max(value.floor());
    match ceiling {
        None => floor as i64,
        Some(c) => (floor.min(c as f64)) as i64,
    }
}

/// `workflowRunSettingsConsequenceId`（真源 :167-183）：后果句的文案 key。
///
/// ★真源 :161-165 —— 随 run 状态换最后一句（completed 不会走到这里）。
/// **正在跑**的 run 只改并发上限时会就地生效，不停止或另起 run，
/// 因此这里显示并发调整的后果说明。`pending` 不算在内：它的引擎可能还没建起来，
/// 就地设不上就照旧退回一次真正的修订，那时原句仍然是对的。
pub fn workflow_run_settings_consequence_id(
    status: WorkflowRunStatus,
    change: Option<&WorkflowRunSettingsChange>,
) -> &'static str {
    if status == WorkflowRunStatus::Running
        && change.is_some_and(|c| c.is_concurrency_only())
    {
        return "chat.toolCall.workflow.run.settings.consequence.concurrencyLive";
    }
    match status {
        WorkflowRunStatus::Pending => "chat.toolCall.workflow.run.settings.consequence.pending",
        WorkflowRunStatus::Stopped => "chat.toolCall.workflow.run.settings.consequence.stopped",
        WorkflowRunStatus::Errored => "chat.toolCall.workflow.run.settings.consequence.errored",
        _ => "chat.toolCall.workflow.run.settings.consequence.running",
    }
}

// ===========================================================================
// 拒绝
// ===========================================================================

/// 与 Stop / Resume 同一个能力缺席 fault（网关对 V4CapabilityUnsupportedError 的
/// reasonCode，真源 :186）。
const CAPABILITY_UNSUPPORTED_FAULT: &str = "fault.command.capabilityUnsupported";

/// `WORKFLOW_RUN_SETTINGS_REJECTED_FAULT_PREFIX`（真源 :61-62）。
pub const WORKFLOW_RUN_SETTINGS_REJECTED_FAULT_PREFIX: &str =
    "fault.command.workflowRunSettingsRejected.";

/// `workflowRunSettingsRejectionReasonSchema` 的词表（真源 :46-55）。
///
/// ★所有拒绝都发生在停下或新建任何东西之前。
pub const WORKFLOW_RUN_SETTINGS_REJECTION_REASONS: [&str; 8] = [
    "not_found",
    "not_configurable",
    "unchanged",
    "script_missing",
    "model_unavailable",
    "compile_failed",
    "missing_boundaries",
    "start_failed",
];

/// `WorkflowRunSettingsRejection`（真源 :191-198）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunSettingsRejection {
    /// 词表内的 reason，或 `unsupported`（能力缺席）/ `generic`（词表外，文案带 code）。
    pub reason: String,
    /// 原始 reasonCode（缺席时是 `ack.status`）。
    pub code: String,
    /// ACK 携带的人可读细节（`compile_failed` 的有界诊断、`start_failed` 的原因）。
    pub message: Option<String>,
}

/// `describeWorkflowRunSettingsRejection`（真源 :201-213）。
///
/// accepted / noop 不是拒绝 → `None`；其余按 fault 前缀反查词表。
pub fn describe_workflow_run_settings_rejection(
    status: &str,
    reason_code: Option<&str>,
    message: Option<&str>,
) -> Option<WorkflowRunSettingsRejection> {
    if status == "accepted" || status == "noop" {
        return None;
    }
    let code = reason_code.unwrap_or(status).to_string();
    let mut reason = "generic".to_string();
    if reason_code == Some(CAPABILITY_UNSUPPORTED_FAULT) {
        reason = "unsupported".to_string();
    } else if let Some(prefixed) = reason_code {
        if let Some(suffix) = prefixed.strip_prefix(WORKFLOW_RUN_SETTINGS_REJECTED_FAULT_PREFIX)
            && WORKFLOW_RUN_SETTINGS_REJECTION_REASONS.contains(&suffix)
        {
            reason = suffix.to_string();
        }
    }
    Some(WorkflowRunSettingsRejection {
        reason,
        code,
        message: message.filter(|m| !m.is_empty()).map(str::to_string),
    })
}

/// `workflowRunSettingsRejectionMessageId`（真源 :216-220）。
pub fn workflow_run_settings_rejection_message_id(
    rejection: &WorkflowRunSettingsRejection,
) -> String {
    format!(
        "chat.toolCall.workflow.run.settings.rejection.{}",
        rejection.reason
    )
}

/// `workflowRunSettingsRejectionDetail`（真源 :226-230）：细节块给不给。
///
/// ★`start_failed` 的原因已经嵌进那句话（`{message}`），不再重复一遍；
/// 其余带 message 的拒绝（`compile_failed` 的诊断）放进有界等宽块。
pub fn workflow_run_settings_rejection_detail(
    rejection: &WorkflowRunSettingsRejection,
) -> Option<&str> {
    if rejection.reason == "start_failed" {
        None
    } else {
        rejection.message.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::run_state::{WorkflowRunActor, WorkflowRunConcurrency};

    fn run_from_str(raw: &str) -> WorkflowRunState {
        serde_json::from_str(raw).unwrap()
    }

    fn base_run(status: &str) -> WorkflowRunState {
        run_from_str(&format!(
            r#"{{"runId": "r", "status": "{status}", "actors": [], "nodes": [],
                "usage": {{"spentTokens": 0, "nodesUsed": 0}}}}"#
        ))
    }

    fn model(provider: &str, model_id: &str, level: Option<&str>) -> WorkflowRunSettingsModel {
        WorkflowRunSettingsModel::Model {
            provider_id: provider.to_string(),
            model_id: model_id.to_string(),
            level: level.map(str::to_string),
        }
    }

    fn draft(model: WorkflowRunSettingsModel, bound: Option<i64>) -> WorkflowRunSettingsDraft {
        WorkflowRunSettingsDraft { model, bound }
    }

    // ── 会话模型 ──

    #[test]
    fn session_model_prefers_the_sparse_selection() {
        // ★真源 :28-29 —— 优先会话持久的稀疏选择。
        let selection = ModelChoice {
            provider_id: "acme".into(),
            model_id: "model-x".into(),
        };
        let got = workflow_session_model_of(Some(&selection), Some("ignored"), Some("ignored"));
        assert_eq!(got, Some(selection));
    }

    #[test]
    fn session_model_falls_back_to_provider_and_model_projection() {
        // 真源 :30-32 —— 退回 UI effective 的 provider / model 投影。
        let got = workflow_session_model_of(None, Some(" acme "), Some(" model-x "));
        assert_eq!(
            got,
            Some(ModelChoice {
                provider_id: "acme".into(),
                model_id: "model-x".into()
            }),
            "两边都 trim"
        );
    }

    #[test]
    fn session_model_absent_when_neither_side_reads_out() {
        // 真源 :32 —— 两者都读不出即缺席。
        assert_eq!(workflow_session_model_of(None, Some("acme"), Some("")), None);
        assert_eq!(workflow_session_model_of(None, Some(""), Some("model-x")), None);
        assert_eq!(workflow_session_model_of(None, None, None), None);
    }

    // ── 表单模型 ──

    #[test]
    fn model_of_reads_canonical_string() {
        let got = workflow_run_settings_model_of(Some("acme/model-x$high"));
        assert_eq!(got, model("acme", "model-x", Some("high")));
        let plain = workflow_run_settings_model_of(Some("acme/model-x"));
        assert_eq!(plain, model("acme", "model-x", None));
    }

    #[test]
    fn model_of_falls_back_to_session_for_absent_or_blank() {
        // 真源 :82 —— 空串与缺席都按会话模型处理。
        assert_eq!(
            workflow_run_settings_model_of(None),
            WorkflowRunSettingsModel::Session
        );
        assert_eq!(
            workflow_run_settings_model_of(Some("   ")),
            WorkflowRunSettingsModel::Session
        );
    }

    #[test]
    fn unparsable_string_becomes_an_unfindable_specific_model() {
        // ★真源 :77 —— 解析不动（坏串）按会话模型处理**不成立**，
        // 所以原样保留成一个查不到的具体模型（providerId 空串）。
        let got = workflow_run_settings_model_of(Some("garbage"));
        assert_eq!(got, model("", "garbage", None));
    }

    #[test]
    fn canonical_round_trips() {
        for canonical in ["acme/model-x", "acme/model-x$high", "solo-model"] {
            let form = workflow_run_settings_model_of(Some(canonical));
            assert_eq!(
                workflow_run_settings_model_canonical(&form).as_deref(),
                Some(canonical),
                "往返一致：{canonical}"
            );
        }
    }

    #[test]
    fn session_model_has_no_canonical_string() {
        // 真源 :101 —— 会话模型没有串（undefined）。
        assert_eq!(
            workflow_run_settings_model_canonical(&WorkflowRunSettingsModel::Session),
            None
        );
    }

    // ── 能不能配 ──

    #[test]
    fn configurable_covers_live_and_errored() {
        // ★真源 :51-53 —— pending / running 能（主场景）；errored 能（换模型重试）。
        for status in ["pending", "running", "errored"] {
            assert!(is_workflow_run_configurable(Some(&base_run(status))), "{status}");
        }
    }

    #[test]
    fn completed_is_never_configurable() {
        // 真源 :54 —— 每个 ask 都会从缓存重放，没有东西会在新设置下跑。
        assert!(!is_workflow_run_configurable(Some(&base_run("completed"))));
    }

    #[test]
    fn superseded_stopped_is_not_configurable() {
        // 真源 :52-53 —— stopped 能，除非是被一次修订替代掉的
        // （活的是它的后继）。
        let mut run = base_run("stopped");
        assert!(is_workflow_run_configurable(Some(&run)), "普通 stopped 能配");
        run.stop_reason = Some(WorkflowRunStopReason::Superseded);
        assert!(!is_workflow_run_configurable(Some(&run)), "stopReason = superseded");
        run.stop_reason = None;
        run.superseded_by = Some("r2".into());
        assert!(!is_workflow_run_configurable(Some(&run)), "supersededBy 在场");
    }

    #[test]
    fn run_absent_from_projection_is_not_configurable() {
        // 真源 :54 —— 不在投影里的 run 没有设置可显示。
        assert!(!is_workflow_run_configurable(None));
    }

    // ── 天花板 ──

    #[test]
    fn ceiling_prefers_the_run_started_value() {
        // ★真源 :70-71 —— 优先 `run.concurrencyCeiling`（随带、恒在），
        // 老 CLI 没发它时退回 `concurrency.ceiling`。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "usage": {"spentTokens": 0, "nodesUsed": 0},
                "concurrency": {"cap": 2, "ceiling": 8, "limit": 3},
                "concurrencyCeiling": 16}"#,
        );
        assert_eq!(workflow_run_settings_ceiling(&run), Some(16), "天花板取 run-started 的");
    }

    #[test]
    fn ceiling_falls_back_to_the_gauge() {
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "usage": {"spentTokens": 0, "nodesUsed": 0},
                "concurrency": {"cap": 2, "ceiling": 8}}"#,
        );
        assert_eq!(workflow_run_settings_ceiling(&run), Some(8));
        assert_eq!(workflow_run_settings_ceiling(&base_run("running")), None, "都没有即未知");
    }

    // ── 起点 ──

    #[test]
    fn draft_starts_at_the_runs_own_settings() {
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "usage": {"spentTokens": 0, "nodesUsed": 0},
                "subagentModel": "acme/model-x$high",
                "concurrency": {"cap": 2, "ceiling": 8, "limit": 3},
                "concurrencyCeiling": 16}"#,
        );
        let d = initial_workflow_run_settings_draft(&run);
        assert_eq!(d.model, model("acme", "model-x", Some("high")));
        assert_eq!(d.bound, Some(3), "界缺席时用自己的 limit");
    }

    #[test]
    fn draft_without_limit_stops_at_the_ceiling() {
        // ★真源 :110 —— 界缺席时停在天花板上（天花板也未知则为 null）。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "usage": {"spentTokens": 0, "nodesUsed": 0},
                "concurrency": {"cap": 2, "ceiling": 8}}"#,
        );
        assert_eq!(initial_workflow_run_settings_draft(&run).bound, Some(8));
        assert_eq!(
            initial_workflow_run_settings_draft(&base_run("running")).bound,
            None,
            "天花板未知则为 null"
        );
    }

    // ── 变更 ──

    #[test]
    fn no_change_disables_apply() {
        // 真源 :128 —— 两项都没变 → undefined（Apply 禁用）。
        let d = draft(model("acme", "model-x", None), Some(3));
        assert!(workflow_run_settings_change(&d, &d.clone(), Some(8)).is_none());
    }

    #[test]
    fn only_the_changed_key_is_sent() {
        // ★真源 :126-127 —— 只发**改过的**那几项（省略 = 沿用）。
        let initial = draft(model("acme", "model-x", None), Some(3));
        let edited = draft(model("acme", "model-y", None), Some(3));
        let change = workflow_run_settings_change(&initial, &edited, Some(8)).unwrap();
        assert_eq!(
            change.subagent_model,
            Some(Some("acme/model-y".to_string()))
        );
        assert_eq!(change.max_concurrency, None, "界没改 → 不发");
    }

    #[test]
    fn changing_only_the_reasoning_level_counts_as_a_model_change() {
        // ★真源 :127 —— 模型按规范串比较：只改思考档也算改了模型。
        let initial = draft(model("acme", "model-x", None), Some(3));
        let edited = draft(model("acme", "model-x", Some("high")), Some(3));
        let change = workflow_run_settings_change(&initial, &edited, Some(8)).unwrap();
        assert_eq!(
            change.subagent_model,
            Some(Some("acme/model-x$high".to_string()))
        );
    }

    #[test]
    fn back_to_session_model_sends_null() {
        // 真源 :127 —— 回到会话模型发 `null`。
        let initial = draft(model("acme", "model-x", None), Some(3));
        let edited = draft(WorkflowRunSettingsModel::Session, Some(3));
        let change = workflow_run_settings_change(&initial, &edited, Some(8)).unwrap();
        assert_eq!(change.subagent_model, Some(None), "null = 回到会话模型");
    }

    #[test]
    fn bound_equal_to_ceiling_sends_null_to_release_it() {
        // ★真源 :127-128 —— 界等于天花板发 `null`（解除本 run 自己的界）。
        // 起点 3 是「自己的界」，抬到 8（= 天花板）归一成「没有自己的界」→ 发 null。
        let initial = draft(WorkflowRunSettingsModel::Session, Some(3));
        let edited = draft(WorkflowRunSettingsModel::Session, Some(8));
        let change = workflow_run_settings_change(&initial, &edited, Some(8)).unwrap();
        assert_eq!(change.max_concurrency, Some(None), "解除本 run 自己的界");
        // 没发 subagentModel。
        assert_eq!(change.subagent_model, None);
    }

    #[test]
    fn bound_at_ceiling_is_already_released_so_raising_it_changes_nothing() {
        // 起点已经在天花板上（= 没有自己的界），再抬不构成变更。
        let initial = draft(WorkflowRunSettingsModel::Session, Some(8));
        let edited = draft(WorkflowRunSettingsModel::Session, Some(16));
        assert!(
            workflow_run_settings_change(&initial, &edited, Some(8)).is_none(),
            "两边都归一成 None"
        );
    }

    #[test]
    fn both_keys_can_change_at_once() {
        let initial = draft(model("acme", "model-x", None), Some(3));
        let edited = draft(WorkflowRunSettingsModel::Session, Some(5));
        let change = workflow_run_settings_change(&initial, &edited, Some(8)).unwrap();
        assert_eq!(change.subagent_model, Some(None));
        assert_eq!(change.max_concurrency, Some(Some(5)));
    }

    // ── 步进器 ──

    #[test]
    fn bound_clamps_between_one_and_the_ceiling() {
        // 真源 :145-148 —— 下限 1，上限天花板（未知则不设上限）。
        assert_eq!(clamp_workflow_run_settings_bound(0.0, Some(8)), 1, "下限 1");
        assert_eq!(clamp_workflow_run_settings_bound(-5.0, Some(8)), 1);
        assert_eq!(clamp_workflow_run_settings_bound(3.7, Some(8)), 3, "向下取整");
        assert_eq!(clamp_workflow_run_settings_bound(99.0, Some(8)), 8, "上限天花板");
        assert_eq!(clamp_workflow_run_settings_bound(99.0, None), 99, "未知 → 不设上限");
    }

    // ── 后果句 ──

    #[test]
    fn live_run_changing_only_concurrency_says_it_applies_in_place() {
        // ★真源 :162-165 —— 正在跑的 run 只改并发上限时会就地生效，
        // 不停止或另起 run。
        let concurrency_only = WorkflowRunSettingsChange {
            subagent_model: None,
            max_concurrency: Some(Some(4)),
        };
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Running, Some(&concurrency_only)),
            "chat.toolCall.workflow.run.settings.consequence.concurrencyLive"
        );
        // 带模型变更就不是「只改并发」。
        let both = WorkflowRunSettingsChange {
            subagent_model: Some(None),
            max_concurrency: Some(Some(4)),
        };
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Running, Some(&both)),
            "chat.toolCall.workflow.run.settings.consequence.running"
        );
        // 解除本 run 自己的界（null）也算「只改并发」。
        let release = WorkflowRunSettingsChange {
            subagent_model: None,
            max_concurrency: Some(None),
        };
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Running, Some(&release)),
            "chat.toolCall.workflow.run.settings.consequence.concurrencyLive",
            "null = 解除本 run 自己的界，也算"
        );
    }

    #[test]
    fn pending_does_not_get_the_in_place_wording() {
        // ★真源 :164-165 —— `pending` 不算在内：它的引擎可能还没建起来，
        // 就地设不上就照旧退回一次真正的修订，那时原句仍然是对的。
        let concurrency_only = WorkflowRunSettingsChange {
            subagent_model: None,
            max_concurrency: Some(Some(4)),
        };
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Pending, Some(&concurrency_only)),
            "chat.toolCall.workflow.run.settings.consequence.pending"
        );
    }

    #[test]
    fn consequence_id_per_status() {
        let none: Option<&WorkflowRunSettingsChange> = None;
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Stopped, none),
            "chat.toolCall.workflow.run.settings.consequence.stopped"
        );
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Errored, none),
            "chat.toolCall.workflow.run.settings.consequence.errored"
        );
        // completed 不会走到这里（不能配），落到 default 分支。
        assert_eq!(
            workflow_run_settings_consequence_id(WorkflowRunStatus::Completed, none),
            "chat.toolCall.workflow.run.settings.consequence.running"
        );
    }

    // ── 拒绝 ──

    #[test]
    fn accepted_and_noop_are_not_rejections() {
        // 真源 :204 —— accepted / noop 不是拒绝 → undefined。
        assert!(describe_workflow_run_settings_rejection("accepted", None, None).is_none());
        assert!(describe_workflow_run_settings_rejection("noop", None, None).is_none());
    }

    #[test]
    fn known_reason_is_read_back_from_the_fault_prefix() {
        // 真源 :208-210 —— 按 fault 前缀反查词表。
        let code = "fault.command.workflowRunSettingsRejected.compile_failed";
        let r = describe_workflow_run_settings_rejection("error", Some(code), Some("第 3 行"))
            .unwrap();
        assert_eq!(r.reason, "compile_failed");
        assert_eq!(r.code, code);
        assert_eq!(r.message.as_deref(), Some("第 3 行"));
    }

    #[test]
    fn capability_unsupported_is_its_own_reason() {
        // 真源 :207 —— 与 Stop / Resume 同一个能力缺席 fault。
        let r = describe_workflow_run_settings_rejection(
            "error",
            Some("fault.command.capabilityUnsupported"),
            None,
        )
        .unwrap();
        assert_eq!(r.reason, "unsupported");
        assert_eq!(r.message, None);
    }

    #[test]
    fn reason_outside_the_vocabulary_falls_back_to_generic() {
        // 真源 :206/209 —— 词表外 → generic（文案带 code）。
        let code = "fault.command.workflowRunSettingsRejected.brand_new_reason";
        let r = describe_workflow_run_settings_rejection("error", Some(code), None).unwrap();
        assert_eq!(r.reason, "generic", "词表外不反查");
        assert_eq!(r.code, code, "code 仍带原文");
        assert_eq!(
            workflow_run_settings_rejection_message_id(&r),
            "chat.toolCall.workflow.run.settings.rejection.generic"
        );
    }

    #[test]
    fn code_falls_back_to_the_ack_status() {
        // 真源 :205 —— reasonCode 缺席时 code 是 ack.status。
        let r = describe_workflow_run_settings_rejection("error", None, None).unwrap();
        assert_eq!(r.code, "error");
        assert_eq!(r.reason, "generic");
    }

    #[test]
    fn detail_is_skipped_only_for_start_failed() {
        // ★真源 :222-224 —— `start_failed` 的原因已经嵌进那句话（`{message}`），
        // 不再重复一遍；其余带 message 的拒绝（compile_failed 的诊断）
        // 放进有界等宽块。
        let compile = describe_workflow_run_settings_rejection(
            "error",
            Some("fault.command.workflowRunSettingsRejected.compile_failed"),
            Some("第 3 行：期望表达式"),
        )
        .unwrap();
        assert_eq!(
            workflow_run_settings_rejection_detail(&compile),
            Some("第 3 行：期望表达式"),
            "诊断进等宽块"
        );
        let start = describe_workflow_run_settings_rejection(
            "error",
            Some("fault.command.workflowRunSettingsRejected.start_failed"),
            Some("引擎端口缺席"),
        )
        .unwrap();
        assert_eq!(
            workflow_run_settings_rejection_detail(&start),
            None,
            "已经嵌进那句话"
        );
        // 没带 message 的拒绝也没有细节块。
        let bare = describe_workflow_run_settings_rejection("error", None, None).unwrap();
        assert_eq!(workflow_run_settings_rejection_detail(&bare), None);
        // 空串视同没有。
        let blank = describe_workflow_run_settings_rejection("error", None, Some("")).unwrap();
        assert_eq!(workflow_run_settings_rejection_detail(&blank), None);
    }

    #[test]
    fn rejection_vocabulary_is_the_shared_eight() {
        // 真源 :46-55 —— 词表是共享的（bootstrap 铸 fault code、UI 反查文案）。
        assert_eq!(WORKFLOW_RUN_SETTINGS_REJECTION_REASONS.len(), 8);
        for reason in WORKFLOW_RUN_SETTINGS_REJECTION_REASONS {
            let code = format!("{WORKFLOW_RUN_SETTINGS_REJECTED_FAULT_PREFIX}{reason}");
            let r = describe_workflow_run_settings_rejection("error", Some(&code), None).unwrap();
            assert_eq!(r.reason, reason, "{reason} 应能反查回自己");
        }
    }

    // ── actor 字段不被本模块消费 ──

    #[test]
    fn actors_do_not_affect_configurability() {
        // 回归：settings 规则只读 status / stopReason / supersededBy / concurrency /
        // subagentModel，actor 表的规模不该影响任何一条判断。
        let mut run = base_run("running");
        run.actors = vec![WorkflowRunActor {
            site_id: "l1".into(),
            ordinal: 0,
            name: None,
            session_id: None,
            status: crate::components::workflow_graph::run_state::WorkflowRunActorStatus::Running,
            phase_name: None,
        }];
        assert!(is_workflow_run_configurable(Some(&run)));
        assert_eq!(workflow_run_settings_ceiling(&run), None);
    }

    #[test]
    fn concurrency_concurrency_type_is_reachable() {
        // `WorkflowRunConcurrency` 是本模块签名的一部分（ceiling / limit 的来源）。
        let c = WorkflowRunConcurrency {
            key: None,
            cap: 3,
            ceiling: 8,
            limit: Some(2),
            cooldown_ms: None,
        };
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "usage": {"spentTokens": 0, "nodesUsed": 0}}"#,
        );
        let mut with = run.clone();
        with.concurrency = Some(c.clone());
        assert_eq!(workflow_run_settings_ceiling(&with), Some(8));
        assert_eq!(
            initial_workflow_run_settings_draft(&with).bound,
            Some(2),
            "limit 优先于天花板"
        );
    }
}