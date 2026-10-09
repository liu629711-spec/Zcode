//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/createWorkflowInput.ts`（322 行）。
//!
//! CreateWorkflow / AmendWorkflow 的**入参读取规则**。
//! 真源注释反复强调一条纪律：这些字段走**入参通道**而不是 display，
//! 所以聊天卡片与运行确认窗共用这一份读取规则，两处不各自解析。

use serde_json::Value;

use super::createWorkflowDisplay::is_plain_record;

/// `readTrimmedString`（真源 :16-22）：非空字符串（trim 后）。
fn read_trimmed_string(value: Option<&Value>) -> Option<String> {
    let s = value?.as_str()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// 种类词表（真源 :74-102）
// ---------------------------------------------------------------------------

/// `WorkflowKindIds`（真源 :74-81）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowKindIds {
    pub writing: &'static str,
    pub revising: &'static str,
    pub awaiting_confirmation: &'static str,
    pub running: &'static str,
    pub ran: &'static str,
    pub draft: &'static str,
}

/// `CREATE_KIND_IDS`（真源 :83-90）。
pub const CREATE_KIND_IDS: WorkflowKindIds = WorkflowKindIds {
    writing: "chat.toolCall.workflow.writing",
    revising: "chat.toolCall.workflow.revising",
    awaiting_confirmation: "chat.toolCall.workflow.awaitingConfirmation",
    running: "chat.toolCall.workflow.running",
    ran: "chat.toolCall.workflow.ran",
    draft: "chat.toolCall.workflow.draft",
};

/// `AMEND_KIND_IDS`（真源 :94-101）。
///
/// ★真源注释：修订词汇的 `running`（校验中）没有专门的词——
/// 校验与创建同一件事，沿用 `running`。
pub const AMEND_KIND_IDS: WorkflowKindIds = WorkflowKindIds {
    writing: "chat.toolCall.workflow.amend.writing",
    revising: "chat.toolCall.workflow.amend.revising",
    awaiting_confirmation: "chat.toolCall.workflow.amend.awaitingConfirmation",
    running: "chat.toolCall.workflow.running",
    ran: "chat.toolCall.workflow.amend.ran",
    draft: "chat.toolCall.workflow.amend.draft",
};

/// 进行中的 kind 文案分相（真源 :32-63）。
///
/// ★真源注释：「校验」只在 `running` 相成立——脚本在 `prepareApproval` 里
/// 早于确认弹窗就被 analyze 过一遍，而模型写脚本的 `inputStreaming` 相
/// 可以持续数十秒。相位只认适配器原样挂在 raw 上的 v4 状态；**缺席就不猜**。
pub fn read_workflow_kind_message_id(
    raw: &Value,
    is_running: bool,
    amend: bool,
    // retuning：这次调用只在调并发上限（read_workflow_retune_call）——
    // 不写脚本也不编译，在途期一个字都不能提「校验」。修订词表的 writing
    // （「正在调整工作流」）对它恒真——无论最后是就地生效还是退回一次
    // 真修订——所以整个在途期都用它。
    retuning: bool,
) -> &'static str {
    let ids = if amend { AMEND_KIND_IDS } else { CREATE_KIND_IDS };
    if !is_running {
        return ids.ran;
    }
    let v4_status = if is_plain_record(raw) {
        raw.get("v4Status").and_then(|v| v.as_str())
    } else {
        None
    };
    match v4_status {
        Some("pendingApproval") => ids.awaiting_confirmation,
        _ if v4_status == Some("inputStreaming") || retuning => ids.writing,
        _ => ids.running,
    }
}

/// 启动前的相位（真源 :69-73）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrelaunchPhase {
    /// 编不过（有 display 且 `ok === false`）。
    pub compile_errors: bool,
    /// 失败但没有 display（被拒、工具报错）。
    pub failed: bool,
    /// 编写中。
    pub writing: bool,
    /// 第 2 稿起——模型在回应反馈，而不是从头开始。
    pub revising: bool,
}

/// `readWorkflowPrelaunchKindMessageId`（真源 :65-77）。
pub fn read_workflow_prelaunch_kind_message_id(
    phase: PrelaunchPhase,
    amend: bool,
) -> &'static str {
    let ids = if amend { AMEND_KIND_IDS } else { CREATE_KIND_IDS };
    if phase.compile_errors {
        return ids.draft;
    }
    if phase.failed {
        return ids.ran;
    }
    if phase.writing {
        return if phase.revising {
            ids.revising
        } else {
            ids.writing
        };
    }
    ids.awaiting_confirmation
}

// ---------------------------------------------------------------------------
// 基础字段读取
// ---------------------------------------------------------------------------

/// `readWorkflowName`（真源 :105-113）：展示名（聊天卡与确认窗共用）。
pub fn read_workflow_name(input: &Value) -> Option<String> {
    if is_plain_record(input) {
        if let Some(name) = input.get("name").and_then(|v| v.as_str()) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// `readWorkflowScript`（真源 :116-123）：脚本原文。
///
/// ★只判 `length > 0`，**不 trim** —— 脚本内容逐字保留。
pub fn read_workflow_script(input: &Value) -> Option<String> {
    if is_plain_record(input) {
        if let Some(script) = input.get("script").and_then(|v| v.as_str()) {
            if !script.is_empty() {
                return Some(script.to_string());
            }
        }
    }
    None
}

/// `readWorkflowAmendTarget`（真源 :133-135）：被修订的前驱 run（`run_id`）。
///
/// 真源注释：在场即本次是 supersede——以新脚本铸新 run，从这个前驱导入缓存，
/// 前驱还在跑就先停下它。走**入参通道**所以 lineage 事实零新载荷。
pub fn read_workflow_amend_target(input: &Value) -> Option<String> {
    if !is_plain_record(input) {
        return None;
    }
    read_trimmed_string(input.get("run_id"))
}

/// `WorkflowAmendPredecessor`（真源 :146-149）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowAmendPredecessor {
    pub status: Option<String>,
    pub name: Option<String>,
}

/// `readWorkflowAmendPredecessor`（真源 :154-161）。
pub fn read_workflow_amend_predecessor(input: &Value) -> Option<WorkflowAmendPredecessor> {
    if !is_plain_record(input) {
        return None;
    }
    let predecessor = input.get("predecessor")?;
    if !is_plain_record(predecessor) {
        return None;
    }
    Some(WorkflowAmendPredecessor {
        status: read_trimmed_string(predecessor.get("status")),
        name: read_trimmed_string(predecessor.get("name")),
    })
}

/// `readWorkflowMaxConcurrency`（真源 :172-178）。
///
/// ★真源注释：只认正整数；Amend 的 `null`（去掉上限）与缺席在确认窗里
/// 是同一件事——没有上限可说，不摆这一行。到这里的值**已经是会生效的那个**
/// （两个工具的 `resolveInput` 在开确认窗之前就 clamp 过了），
/// 所以这里不重做 clamp。
pub fn read_workflow_max_concurrency(input: &Value) -> Option<i64> {
    if !is_plain_record(input) {
        return None;
    }
    let value = input.get("max_concurrency")?.as_f64()?;
    if value > 0.0 && value.fract() == 0.0 {
        Some(value as i64)
    } else {
        None
    }
}

/// `WorkflowRetuneCall`（真源 :189-198）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRetuneCall {
    pub run_id: String,
    /// 用户要求的上限；`None` = 解除本 run 自己的界（回到本机上限）。
    ///
    /// ⚠真源注释：**未经钳制**——CLI 的 `resolveInput` 会把它钳进 `[1, 天花板]`，
    /// 这里读到的是模型发出的那个数。展示方必须拿本机天花板去比，
    /// 绝不能当成「实际生效的并发」原样念出来。
    pub requested: Option<i64>,
}

/// `readWorkflowRetuneCall`（真源 :200-215）。
///
/// 这次调用**只在调并发上限**：`run_id` + `max_concurrency`，没有脚本来源、
/// 也不改模型、不改名字。
///
/// ★真源注释（:207-213）：入参是这条事实在线上的**唯一落点**——
/// 工具的结构化输出不过 v4（只有 `text` 与 `display`），而三处
/// `create_workflow` display schema 都是 `.strict()` 的冻结字段集——
/// 多一个键会让旧端把整条工具结果丢掉。所以**按入参形状裁，零协议改动**。
pub fn read_workflow_retune_call(input: &Value) -> Option<WorkflowRetuneCall> {
    if !is_plain_record(input) {
        return None;
    }
    let run_id = read_workflow_amend_target(input)?;
    let bound_null = matches!(input.get("max_concurrency"), Some(Value::Null));
    let requested = if bound_null {
        None
    } else {
        Some(read_workflow_max_concurrency(input)?)
    };
    // ★真源 :211 —— 任何一个别的意图在场都不是「只调并发上限」：
    // 脚本与 path 要编译，模型与名字要换一条 run。
    if read_workflow_script(input).is_some() || read_trimmed_string(input.get("path")).is_some() {
        return None;
    }
    if input.get("subagent_model").is_some() || input.get("name").is_some() {
        return None;
    }
    Some(WorkflowRetuneCall { run_id, requested })
}

/// `readWorkflowSubagentModel`（真源 :224-226）。
///
/// ★真源注释：只认非空字符串；Amend 的 `null`（退回会话模型）与缺席
/// 在确认窗里是同一件事。到这里的值**已经是会生效的那个**，不做形状校验——
/// UI 不是第二个解析器。
pub fn read_workflow_subagent_model(input: &Value) -> Option<String> {
    if !is_plain_record(input) {
        return None;
    }
    read_trimmed_string(input.get("subagent_model"))
}

/// `readWorkflowAmendScriptInherited`（真源 :241-250）。
///
/// 这次修订沿用前驱的脚本。两个面各有一条证据：
/// - 确认窗读 CLI `resolveInput` 回填过的入参：`predecessor.script_inherited` 说它是沿用来的；
/// - 聊天卡读**模型发出的**入参：两个脚本来源 `script` 与 `path` **都**没有，
///   才是省略了脚本。只缺 `script` 不算——`path` 修订同样不带 `script`，
///   而它交上来的正是一份改过的脚本。
pub fn read_workflow_amend_script_inherited(input: &Value) -> bool {
    if !is_plain_record(input) {
        return false;
    }
    if let Some(pred) = input.get("predecessor") {
        if is_plain_record(pred) && pred.get("script_inherited") == Some(&Value::Bool(true)) {
            return true;
        }
    }
    read_workflow_script(input).is_none() && read_trimmed_string(input.get("path")).is_none()
}

/// `readWorkflowCardKeptScript`（真源 :256-261）。
///
/// 入参被快照裁剪成预览时**不说**——那时缺的是字节，不是脚本。
pub fn read_workflow_card_kept_script(input: &Value, input_truncated: bool) -> bool {
    !input_truncated && read_workflow_amend_script_inherited(input)
}

/// `isWorkflowAmendPredecessorLive`（真源 :265-267）：前驱仍在飞。
pub fn is_workflow_amend_predecessor_live(
    predecessor: Option<&WorkflowAmendPredecessor>,
) -> bool {
    predecessor.is_some_and(|p| {
        p.status.as_deref() == Some("running") || p.status.as_deref() == Some("pending")
    })
}

/// `WorkflowSavedSource`（真源 :279-285）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowSavedSource {
    pub name: String,
    pub path: Option<String>,
    pub scope: Option<String>,
    /// 校验回填后的实参袋。缺席或形状不符时为空对象，**绝不是 None**。
    pub args: Value,
}

/// `readWorkflowSaved`（真源 :287-309）。
pub fn read_workflow_saved(input: &Value) -> Option<WorkflowSavedSource> {
    if !is_plain_record(input) {
        return None;
    }
    let saved = input.get("saved")?;
    if !is_plain_record(saved) {
        return None;
    }
    // ★真源 :299-302 —— 名字是这条来源的身份：读不出名字就当作没有来源，
    // 而不是渲染一个空徽标。
    let name = saved
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    if name.is_empty() {
        return None;
    }
    Some(WorkflowSavedSource {
        name: name.to_string(),
        path: read_trimmed_string(saved.get("path")),
        scope: read_trimmed_string(saved.get("scope")),
        args: saved
            .get("args")
            .filter(|v| is_plain_record(v))
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
    })
}

/// `formatWorkflowArgValue`（真源 :311-322）。
///
/// ★真源注释：字符串原样（引号只会给中文实参添噪），其余走 JSON——
/// 声明的四种类型（string/number/boolean/json）由此都可读。
/// 循环引用等不可序列化的值不该让整块确认窗崩掉。
pub fn format_workflow_arg_value(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        return s.to_string();
    }
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── 种类词表 ──

    #[test]
    fn kind_ids_tables_match_source() {
        assert_eq!(CREATE_KIND_IDS.writing, "chat.toolCall.workflow.writing");
        assert_eq!(AMEND_KIND_IDS.ran, "chat.toolCall.workflow.amend.ran");
        // ★修订词表的 running 沿用创建那张（真源 :92-93 注释）。
        assert_eq!(
            AMEND_KIND_IDS.running, CREATE_KIND_IDS.running,
            "校验与创建同一件事，沿用 running"
        );
    }

    #[test]
    fn kind_id_terminal_ignores_v4_status() {
        for amend in [false, true] {
            let raw = json!({ "v4Status": "inputStreaming" });
            assert_eq!(
                read_workflow_kind_message_id(&raw, false, amend, false),
                if amend {
                    AMEND_KIND_IDS.ran
                } else {
                    CREATE_KIND_IDS.ran
                },
                "非运行态一律 ran"
            );
        }
    }

    #[test]
    fn kind_id_pending_approval_maps_to_awaiting() {
        let raw = json!({ "v4Status": "pendingApproval" });
        assert_eq!(
            read_workflow_kind_message_id(&raw, true, false, false),
            CREATE_KIND_IDS.awaiting_confirmation
        );
    }

    #[test]
    fn kind_id_input_streaming_maps_to_writing() {
        let raw = json!({ "v4Status": "inputStreaming" });
        assert_eq!(
            read_workflow_kind_message_id(&raw, true, false, false),
            CREATE_KIND_IDS.writing
        );
    }

    #[test]
    fn kind_id_running_phase_maps_to_running() {
        // 真源 :60-62 —— 缺席/其他状态都落 running（缺席不猜更细的相位）。
        for raw in [json!({}), json!({ "v4Status": "running" })] {
            assert_eq!(
                read_workflow_kind_message_id(&raw, true, false, false),
                CREATE_KIND_IDS.running
            );
        }
    }

    #[test]
    fn retuning_uses_wording_throughout_running() {
        // ★真源 :38-44 —— 调并发上限时在途期一个字都不能提「校验」，
        // 修订词表的 writing 对它恒真。
        let raw = json!({ "v4Status": "running" });
        assert_eq!(
            read_workflow_kind_message_id(&raw, true, true, true),
            AMEND_KIND_IDS.writing,
            "retuning 时不该出现 running（校验）"
        );
        // 非 retuning 时同一 raw 走 running。
        assert_eq!(
            read_workflow_kind_message_id(&raw, true, true, false),
            AMEND_KIND_IDS.running
        );
    }

    #[test]
    fn prelaunch_phase_priority() {
        // 真源 :72-76 —— compileErrors > failed > writing > 待确认。
        let cases = [
            (
                PrelaunchPhase {
                    compile_errors: true,
                    failed: true,
                    writing: true,
                    revising: true,
                },
                CREATE_KIND_IDS.draft,
            ),
            (
                PrelaunchPhase {
                    compile_errors: false,
                    failed: true,
                    writing: true,
                    revising: true,
                },
                CREATE_KIND_IDS.ran,
            ),
            (
                PrelaunchPhase {
                    compile_errors: false,
                    failed: false,
                    writing: true,
                    revising: true,
                },
                CREATE_KIND_IDS.revising,
            ),
            (
                PrelaunchPhase {
                    compile_errors: false,
                    failed: false,
                    writing: true,
                    revising: false,
                },
                CREATE_KIND_IDS.writing,
            ),
            (
                PrelaunchPhase {
                    compile_errors: false,
                    failed: false,
                    writing: false,
                    revising: false,
                },
                CREATE_KIND_IDS.awaiting_confirmation,
            ),
        ];
        for (phase, expected) in cases {
            assert_eq!(
                read_workflow_prelaunch_kind_message_id(phase, false),
                expected,
                "phase={phase:?}"
            );
        }
    }

    // ── 基础字段 ──

    #[test]
    fn name_requires_non_empty() {
        assert_eq!(
            read_workflow_name(&json!({ "name": "  工作流  " })).as_deref(),
            Some("工作流")
        );
        assert_eq!(read_workflow_name(&json!({ "name": "  " })), None);
        assert_eq!(read_workflow_name(&json!({ "name": 1 })), None);
        assert_eq!(read_workflow_name(&json!({})), None);
    }

    #[test]
    fn script_is_not_trimmed() {
        // ★真源 :120 —— 只判 length > 0，脚本逐字保留。
        assert_eq!(
            read_workflow_script(&json!({ "script": "\n  code\n" })).as_deref(),
            Some("\n  code\n")
        );
        assert_eq!(read_workflow_script(&json!({ "script": "" })), None);
        assert_eq!(read_workflow_script(&json!({})), None);
    }

    #[test]
    fn amend_target_reads_run_id() {
        assert_eq!(
            read_workflow_amend_target(&json!({ "run_id": " run-1 " })).as_deref(),
            Some("run-1")
        );
        assert_eq!(read_workflow_amend_target(&json!({})), None);
    }

    #[test]
    fn amend_predecessor_reads_two_fields() {
        let input = json!({
            "predecessor": { "status": " running ", "name": " 前任 " }
        });
        let p = read_workflow_amend_predecessor(&input).unwrap();
        assert_eq!(p.status.as_deref(), Some("running"));
        assert_eq!(p.name.as_deref(), Some("前任"));
        // predecessor 非 record → None。
        assert_eq!(read_workflow_amend_predecessor(&json!({ "predecessor": 1 })), None);
        assert_eq!(read_workflow_amend_predecessor(&json!({})), None);
    }

    #[test]
    fn max_concurrency_requires_positive_integer() {
        assert_eq!(
            read_workflow_max_concurrency(&json!({ "max_concurrency": 3 })),
            Some(3)
        );
        // 非正整数一律不认（真源 :175）。
        assert_eq!(read_workflow_max_concurrency(&json!({ "max_concurrency": 0 })), None);
        assert_eq!(read_workflow_max_concurrency(&json!({ "max_concurrency": -1 })), None);
        assert_eq!(
            read_workflow_max_concurrency(&json!({ "max_concurrency": 1.5 })),
            None,
            "非整数不认"
        );
        assert_eq!(
            read_workflow_max_concurrency(&json!({ "max_concurrency": null })),
            None
        );
    }

    #[test]
    fn subagent_model_trims_and_rejects_empty() {
        assert_eq!(
            read_workflow_subagent_model(&json!({ "subagent_model": " gpt-5 " })).as_deref(),
            Some("gpt-5")
        );
        assert_eq!(
            read_workflow_subagent_model(&json!({ "subagent_model": " " })),
            None
        );
    }

    // ── retune 判定 ──

    #[test]
    fn retune_requires_run_id_and_concurrency() {
        let input = json!({ "run_id": "r1", "max_concurrency": 4 });
        let call = read_workflow_retune_call(&input).unwrap();
        assert_eq!(call.run_id, "r1");
        assert_eq!(call.requested, Some(4));

        assert_eq!(read_workflow_retune_call(&json!({ "max_concurrency": 4 })), None);
        assert_eq!(read_workflow_retune_call(&json!({ "run_id": "r1" })), None);
    }

    #[test]
    fn retune_null_concurrency_means_remove_bound() {
        let input = json!({ "run_id": "r1", "max_concurrency": null });
        let call = read_workflow_retune_call(&input).unwrap();
        assert_eq!(call.requested, None, "null = 解除本 run 自己的界");
    }

    #[test]
    fn retune_rejected_when_other_intent_present() {
        // ★真源 :211 —— 脚本 / path / 模型 / 名字任一在场都不是「只调并发上限」。
        let base = json!({ "run_id": "r1", "max_concurrency": 2 });
        for extra in [
            json!({ "script": "code" }),
            json!({ "path": "wf.ts" }),
            json!({ "subagent_model": "gpt" }),
            json!({ "name": "新名" }),
        ] {
            let mut input = base.clone();
            for (k, v) in extra.as_object().unwrap() {
                input[k] = v.clone();
            }
            assert_eq!(
                read_workflow_retune_call(&input),
                None,
                "带{extra} 时不该判为 retune"
            );
        }
    }

    #[test]
    fn retune_accepts_pure_call() {
        let input = json!({ "run_id": "r1", "max_concurrency": 8 });
        assert!(read_workflow_retune_call(&input).is_some());
    }

    // ── 脚本沿用 ──

    #[test]
    fn script_inherited_from_predecessor_flag() {
        // 真源 :245-247 —— 确认窗读的证据。
        let input = json!({
            "run_id": "r1",
            "predecessor": { "script_inherited": true },
        });
        assert!(read_workflow_amend_script_inherited(&input));
    }

    #[test]
    fn script_inferred_when_both_sources_absent() {
        // 真源 :249 —— script 与 path **都**没有才算省略。
        assert!(read_workflow_amend_script_inherited(&json!({ "run_id": "r1" })));
        // 只缺 script 不算（path 修订同样不带 script）。
        assert!(!read_workflow_amend_script_inherited(&json!({ "path": "wf.ts" })));
        assert!(!read_workflow_amend_script_inherited(&json!({ "script": "x" })));
    }

    #[test]
    fn card_kept_script_respects_truncation() {
        // ★真源 :257-259 —— 入参被快照裁剪时不说（缺的是字节，不是脚本）。
        let input = json!({ "run_id": "r1" });
        assert!(read_workflow_card_kept_script(&input, false));
        assert!(
            !read_workflow_card_kept_script(&input, true),
            "裁剪后不断言"
        );
    }

    #[test]
    fn predecessor_live_statuses() {
        for status in ["running", "pending"] {
            let p = WorkflowAmendPredecessor {
                status: Some(status.into()),
                name: None,
            };
            assert!(
                is_workflow_amend_predecessor_live(Some(&p)),
                "{status} 应算在飞"
            );
        }
        for status in ["completed", "failed", "stopped"] {
            let p = WorkflowAmendPredecessor {
                status: Some(status.into()),
                name: None,
            };
            assert!(!is_workflow_amend_predecessor_live(Some(&p)), "{status} 已结算");
        }
        assert!(!is_workflow_amend_predecessor_live(None));
    }

    // ── saved 来源 ──

    #[test]
    fn saved_requires_name() {
        // ★真源 :299-302 —— 读不出名字就当作没有来源，不渲染空徽标。
        assert_eq!(read_workflow_saved(&json!({ "saved": { "path": "a.ts" } })), None);
        assert_eq!(read_workflow_saved(&json!({ "saved": { "name": " " } })), None);
        assert!(read_workflow_saved(&json!({})).is_none());
        assert!(read_workflow_saved(&json!({ "saved": 1 })).is_none());
    }

    #[test]
    fn saved_reads_all_fields() {
        let input = json!({
            "saved": {
                "name": "  我的工作流 ",
                "path": " wfs/a.ts ",
                "scope": " project ",
                "args": { "x": 1 },
            }
        });
        let s = read_workflow_saved(&input).unwrap();
        assert_eq!(s.name, "我的工作流");
        assert_eq!(s.path.as_deref(), Some("wfs/a.ts"));
        assert_eq!(s.scope.as_deref(), Some("project"));
        assert_eq!(s.args["x"], 1);
    }

    #[test]
    fn saved_args_default_to_empty_object_not_none() {
        // ★真源 :283-284 —— 缺席或形状不符时为空对象，绝不是 undefined。
        let input = json!({ "saved": { "name": "n" } });
        let s = read_workflow_saved(&input).unwrap();
        assert!(s.args.is_object());
        assert_eq!(s.args.as_object().unwrap().len(), 0);

        let bad = json!({ "saved": { "name": "n", "args": [1, 2] } });
        assert!(read_workflow_saved(&bad).unwrap().args.is_object());
    }

    // ── 实参格式化 ──

    #[test]
    fn arg_value_keeps_strings_verbatim() {
        // ★真源 :313-315 —— 字符串原样（引号只会给中文实参添噪）。
        assert_eq!(format_workflow_arg_value(&json!("中文值")), "中文值");
        assert_eq!(
            format_workflow_arg_value(&json!("  带空格  ")),
            "  带空格  ",
            "字符串不 trim"
        );
    }

    #[test]
    fn arg_value_json_encodes_other_types() {
        assert_eq!(format_workflow_arg_value(&json!(3)), "3");
        assert_eq!(format_workflow_arg_value(&json!(true)), "true");
        assert_eq!(format_workflow_arg_value(&json!(null)), "null");
        assert_eq!(
            format_workflow_arg_value(&json!({ "k": [1, 2] })),
            r#"{"k":[1,2]}"#
        );
    }
}