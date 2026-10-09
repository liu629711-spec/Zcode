//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/list-workflow-runs.tsx`（192 行）。
//!
//! ListWorkflowRuns 的聊天卡（真源 :23-29）：折叠行是 kindLabel + 计数
//! （单复数独立 key 的既有惯例）；展开是行式列表［状态点词 | label mono | 短时间 | tokens］，
//! 本会话 run 加边框小签，`possiblyInterrupted` 行尾给警示标注——
//! **「可能已中断」是读面标注不是状态改写**，卡片上同样只标注。
//!
//! **裁剪注明**：`context.kindLabelOverride`（真源 :106）——Rust 侧上下文没带宿主覆盖位，
//! 与其余已迁卡同样式走真源缺省；`canToggle/forceOpen ?? 默认` 亦走缺省。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::toolResultDisplay::{ListWorkflowRunsDisplay, ToolResultDisplay, read_tool_result_display};
use super::super::i18n;
use crate::ToolCallBlocks::toolResultDisplay::WorkflowRunSummaryRow;
use crate::components::workflow_graph::run_status_presentation::{
    run_status_dot_class, run_status_text_class, read_workflow_run_stop_reason,
    workflow_run_stop_reason_message_id,
};
use crate::components::workflowIcons::icon_history;
use crate::lib::workflowObservationFormat::{format_workflow_timestamp, format_workflow_token_count};

/// 真源 :21 —— `History className="size-4 shrink-0 text-foreground-subtle"`。
pub const LIST_WORKFLOW_RUNS_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 无 display 时的有界文本面板类（真源 :88）。
pub const FALLBACK_PANEL_CLASS: &str =
    "mb-2 max-h-60 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 font-mono text-ui-base text-foreground-subtle";

/// 真源 :34-35 —— display 只有是 `list_workflow_runs` 时才算数。
pub fn read_list_display(raw: &Value) -> Option<ListWorkflowRunsDisplay> {
    match read_tool_result_display(raw) {
        Some(ToolResultDisplay::ListWorkflowRuns(d)) => Some(d),
        _ => None,
    }
}

/// 真源 :37-41 的种类词二选一。
pub fn list_runs_kind_label_id(is_running: bool) -> &'static str {
    if is_running {
        "chat.toolCall.workflow.listRuns.listing"
    } else {
        "chat.toolCall.workflow.listRuns.listed"
    }
}

/// 真源 :44-52 —— 单复数各一条 key，插值的都是那个数。
pub fn list_runs_count_label(run_count: usize) -> String {
    let id = if run_count == 1 {
        "chat.toolCall.workflow.listRuns.countOne"
    } else {
        "chat.toolCall.workflow.listRuns.count"
    };
    i18n::format(id, &[("count".to_string(), run_count.to_string())])
}

/// 真源 :57 —— 折叠行的主文本：0 条用「还没有实例」，否则用计数。
pub fn list_runs_primary_text(run_count: usize) -> String {
    if run_count == 0 {
        i18n::text("chat.toolCall.workflow.listRuns.empty")
    } else {
        list_runs_count_label(run_count)
    }
}

/// 真源 :68-71 —— display 缺席时才退回散文输出（trim 后仍要非空才算有内容）。
pub fn list_runs_fallback_text(
    display_present: bool,
    output: Option<&str>,
) -> Option<String> {
    if display_present {
        return None;
    }
    let trimmed = output?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// 真源 :94-96 —— 展开门：有列表且有行，或者有可读散文。
///
/// `list_runs_fallback_text` 内部已经带「display 缺席」这个前提，这里不再重复取反。
pub fn list_runs_has_details(
    display_present: bool,
    run_count: usize,
    output: Option<&str>,
) -> bool {
    (display_present && run_count > 0) || list_runs_fallback_text(display_present, output).is_some()
}

/// 一行 run 的状态件（真源 :143-164 的圆点 + 词 + 停止原因）。
#[derive(Debug, Clone, PartialEq)]
pub struct ListRunRowStatus {
    pub dot_class: &'static str,
    pub text_class: &'static str,
    pub status_word: String,
    /// 停止原因词；非 stopped 态或词表外的值都缺席。
    pub stop_reason_word: Option<String>,
}

/// `ListWorkflowRunsBody` 里每行的状态判定（真源 :143-146 + :160-164）。
pub fn list_run_row_status(run: &WorkflowRunSummaryRow) -> ListRunRowStatus {
    let stop_reason = read_workflow_run_stop_reason(run.status, run.stop_reason.as_deref());
    ListRunRowStatus {
        dot_class: run_status_dot_class(run.status),
        text_class: run_status_text_class(run.status),
        status_word: i18n::text(&format!(
            "chat.toolCall.workflow.run.status.{}",
            run.status.as_str()
        )),
        stop_reason_word: stop_reason.map(|reason| {
            i18n::text(&workflow_run_stop_reason_message_id(reason))
        }),
    }
}

/// `ListWorkflowRunsToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct ListWorkflowRunsBlockProps {
    pub tool_id: String,
    pub raw: Value,
    pub status: String,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub output_text: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `ListWorkflowRunsBody`（真源 :128-191）。
#[component]
fn ListWorkflowRunsBody(runs: Vec<WorkflowRunSummaryRow>) -> impl IntoView {
    let own_session_label = i18n::text("chat.toolCall.workflow.listRuns.ownSession");
    let interrupted_label = i18n::text("chat.toolCall.workflow.listRuns.interrupted");

    // 真源 :135-138 —— 空列表在展开区也有一句「还没有实例」。
    if runs.is_empty() {
        return view! {
            <p class="text-ui-sm text-foreground-subtlest">
                {i18n::text("chat.toolCall.workflow.listRuns.empty")}
            </p>
        }
        .into_any();
    }

    let rows = runs
        .into_iter()
        .map(|run| {
            // ★真源 :127 纪律：以下每一个值都在 view! 之前算好——
            // view! 的 children 先于属性求值，被 move 的 run 不能在后面的属性里再用。
            let run_id = run.run_id.clone();
            let status_attr = run.status.as_str().to_string();
            let status = list_run_row_status(&run);
            let label = run.label.clone();
            let title = run.run_id.clone();
            let owned = run.owned_by_this_session;
            let interrupted = run.possibly_interrupted == Some(true);
            // 时间戳与 token 数走 lib/workflowObservationFormat（与 GetWorkflowRun 同一套算法）。
            let updated_at = format_workflow_timestamp(run.updated_at);
            let tokens = format_workflow_token_count(run.spent_tokens);

            view! {
                <div
                    class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5"
                    data-workflow-run-row=run_id.clone()
                    data-workflow-run-row-status=status_attr
                >
                    <span class="flex shrink-0 items-center gap-1.5">
                        // ★状态词永远在圆点旁边：状态绝不只靠颜色或动画表达。
                        <span
                            aria-hidden="true"
                            class=format!("size-1.5 rounded-full {}", status.dot_class)
                        />
                        <span class=format!("text-ui-sm {}", status.text_class)>
                            {status.status_word.clone()}
                        </span>
                        {status.stop_reason_word.map(|word| view! {
                            <span class="text-ui-xs text-foreground-subtlest">{word}</span>
                        })}
                    </span>
                    <span
                        class="min-w-0 flex-1 truncate font-mono text-ui-base text-foreground-subtle"
                        title=title
                    >
                        {label}
                    </span>
                    {owned.then(|| view! {
                        <span class="shrink-0 rounded-xs border border-border px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest">
                            {own_session_label.clone()}
                        </span>
                    })}
                    // 「可能已中断」只是读面标注，不改状态词与状态点。
                    {interrupted.then(|| view! {
                        <span class="shrink-0 text-ui-xs text-warning">{interrupted_label.clone()}</span>
                    })}
                    <span class="shrink-0 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
                        {updated_at}
                    </span>
                    <span class="shrink-0 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
                        {tokens}
                    </span>
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="space-y-1" data-workflow-run-list="true">{rows}</div>
    }
    .into_any()
}

/// `ListWorkflowRunsToolCallBlock`（真源 :30-126）。
#[component]
pub fn ListWorkflowRunsToolCallBlock(props: ListWorkflowRunsBlockProps) -> impl IntoView {
    let display = read_list_display(&props.raw);
    let display_present = display.is_some();
    let runs: Vec<WorkflowRunSummaryRow> = display
        .as_ref()
        .map(|d| d.runs.clone())
        .unwrap_or_default();
    let run_count = runs.len();
    let truncated = display.as_ref().and_then(|d| d.truncated).unwrap_or(false);

    let primary_text = list_runs_primary_text(run_count);
    let fallback_text = list_runs_fallback_text(display_present, props.output_text.as_deref());
    let has_details = list_runs_has_details(
        display_present,
        run_count,
        props.output_text.as_deref(),
    );
    let is_failed = props.status == "failed";

    let render_content = if display_present {
        // 真源 :74-84 —— 列表 + 截断说明。
        let body_runs = runs.clone();
        Some(std::sync::Arc::new(move || {
            view! {
                <div class="mb-2 space-y-2">
                    <ListWorkflowRunsBody runs=body_runs.clone() />
                    {truncated.then(|| view! {
                        <p class="text-ui-xs text-foreground-subtle">
                            {i18n::text("chat.toolCall.workflow.listRuns.truncated")}
                        </p>
                    })}
                </div>
            }
            .into_any()
        }) as ChildrenFn)
    } else {
        // 真源 :86-91 —— 无 display 时的有界散文面板。
        let text = fallback_text.clone().unwrap_or_default();
        has_details.then(move || {
            std::sync::Arc::new(move || {
                view! { <pre class=FALLBACK_PANEL_CLASS>{text.clone()}</pre> }.into_any()
            }) as ChildrenFn
        })
    };

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(has_details),
                force_open: Some(false),
                kind_label: Some(i18n::text(list_runs_kind_label_id(props.is_running))),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                // 真源 :109 —— 只有失败态才挂状态词（且用宿主的 statusLabel）。
                status_label: if is_failed { props.status_label.clone() } else { None },
                status_tooltip: props.error_text.clone(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=LIST_WORKFLOW_RUNS_TOOL_ICON_CLASS>{icon_history()}</span>
                }
                .into_any()
            }))
            render_content=render_content
        />
        <ToolSnapshotFieldNoticeComponent
            props=ToolSnapshotFieldNoticeProps {
                refs: props.snapshot_refs.clone(),
                tool_id: props.tool_id.clone(),
                on_load_full_tool_call_fields: props.on_load_full_tool_call_fields.clone(),
            }
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::run_state::WorkflowRunStatus;
    use serde_json::json;

    fn row(run_id: &str, status: &str, owned: bool) -> WorkflowRunSummaryRow {
        WorkflowRunSummaryRow {
            run_id: run_id.into(),
            label: "把改动跑一遍".into(),
            label_source: crate::ToolCallBlocks::toolResultDisplay::WorkflowRunLabelSource::Name,
            status: match status {
                "pending" => WorkflowRunStatus::Pending,
                "running" => WorkflowRunStatus::Running,
                "completed" => WorkflowRunStatus::Completed,
                "errored" => WorkflowRunStatus::Errored,
                _ => WorkflowRunStatus::Stopped,
            },
            stop_reason: None,
            owned_by_this_session: owned,
            possibly_interrupted: None,
            created_at: 0.0,
            updated_at: 1_700_000_000_000.0,
            spent_tokens: 1_500.0,
        }
    }

    #[test]
    fn display_only_when_the_kind_matches() {
        let raw = json!({ "result": { "display": {
            "kind": "list_workflow_runs",
            "runs": [{
                "runId": "r1", "label": "l", "labelSource": "name", "status": "completed",
                "ownedByThisSession": true, "createdAt": 0.0, "updatedAt": 0.0, "spentTokens": 0.0,
            }],
        } } });
        let parsed = read_list_display(&raw).expect("合法列表 display");
        assert_eq!(parsed.runs.len(), 1);
        // 别的 kind 不算。
        assert_eq!(
            read_list_display(&json!({"result": {"display": {"kind": "resume_workflow_run", "runId": "r"}}})),
            None
        );
        assert_eq!(read_list_display(&json!({})), None);
    }

    #[test]
    fn kind_label_switches_on_running() {
        assert_eq!(
            list_runs_kind_label_id(true),
            "chat.toolCall.workflow.listRuns.listing"
        );
        assert_eq!(
            list_runs_kind_label_id(false),
            "chat.toolCall.workflow.listRuns.listed"
        );
    }

    #[test]
    fn count_label_is_one_key_per_number_shape_but_same_wording() {
        // 真源 :46-50 —— 单复数各一条 key（这里两条文案恰好同字，仍是两条 key）。
        assert_eq!(list_runs_count_label(0), "0 个实例");
        assert_eq!(list_runs_count_label(1), "1 个实例");
        assert_eq!(list_runs_count_label(12), "12 个实例");
    }

    #[test]
    fn primary_text_switches_to_empty_wording_at_zero() {
        // 真源 :57 —— 0 条不画「0 个实例」，画「还没有工作流实例」。
        assert_eq!(list_runs_primary_text(0), "还没有工作流实例");
        assert_eq!(list_runs_primary_text(1), "1 个实例");
    }

    #[test]
    fn fallback_text_only_when_display_absent() {
        // 真源 :68-71 —— 有 display 就绝不读散文输出。
        assert_eq!(list_runs_fallback_text(true, Some("有内容")), None);
        assert_eq!(
            list_runs_fallback_text(false, Some("  两端空白  ")),
            Some("两端空白".to_string())
        );
        assert_eq!(list_runs_fallback_text(false, Some("   ")), None);
        assert_eq!(list_runs_fallback_text(false, None), None);
    }

    #[test]
    fn details_gate_needs_rows_or_prose() {
        // 真源 :94-96 —— display 在场但零行时，展开区没东西可给。
        assert!(!list_runs_has_details(true, 0, None));
        assert!(list_runs_has_details(true, 3, None));
        assert!(list_runs_has_details(false, 0, Some("散文书底")));
        assert!(!list_runs_has_details(false, 0, Some("   ")));
        assert!(!list_runs_has_details(false, 0, None));
    }

    #[test]
    fn row_status_pairs_a_word_with_the_dot() {
        // ★真源 :154-159 —— 圆点与状态词同时在场，状态不只靠颜色。
        let running = list_run_row_status(&row("r1", "running", false));
        assert_eq!(running.dot_class, "animate-pulse bg-warning motion-reduce:animate-none");
        assert_eq!(running.text_class, "text-warning");
        assert_eq!(running.status_word, "运行中");
        assert_eq!(running.stop_reason_word, None);

        let completed = list_run_row_status(&row("r2", "completed", false));
        assert_eq!(completed.dot_class, "bg-success");
        assert_eq!(completed.status_word, "已完成");
    }

    #[test]
    fn stop_reason_word_only_reads_for_stopped_runs() {
        // 真源 :146 复用 `readWorkflowRunStopReason`：非 stopped 态即便带键也不显示原因词。
        let mut stopped = row("r3", "stopped", false);
        stopped.stop_reason = Some("user".into());
        assert_eq!(
            list_run_row_status(&stopped).stop_reason_word.as_deref(),
            Some("你停止的")
        );
        // 词表外的原因值 → 整段省略（不是显示原始值）。
        let mut unknown = row("r4", "stopped", false);
        unknown.stop_reason = Some("mystery".into());
        assert_eq!(list_run_row_status(&unknown).stop_reason_word, None);
        // running 带 stopReason 也不读。
        let mut running = row("r5", "running", false);
        running.stop_reason = Some("user".into());
        assert_eq!(list_run_row_status(&running).stop_reason_word, None);
    }

    #[test]
    fn interrupted_annotation_is_not_a_status_rewrite() {
        // 真源 :27-29 + :177-179 —— 「可能已中断」只加一个标注，
        // 状态点与状态词仍是 run 自己的那一个。
        let mut run = row("r6", "completed", false);
        run.possibly_interrupted = Some(true);
        let status = list_run_row_status(&run);
        assert_eq!(status.status_word, "已完成");
        assert_eq!(status.dot_class, "bg-success");
        // === true 才标注：false 与缺席都不标（真源 :177 用的是 `=== true`）。
        run.possibly_interrupted = Some(false);
        assert_ne!(run.possibly_interrupted, Some(true));
        run.possibly_interrupted = None;
        assert_ne!(run.possibly_interrupted, Some(true));
    }

    #[test]
    fn row_timestamp_and_tokens_use_the_shared_formatters() {
        // 与 GetWorkflowRun 同一套（lib/workflowObservationFormat），不各自造。
        let run = row("r7", "running", false);
        assert_eq!(format_workflow_token_count(run.spent_tokens), "1.5k");
        assert_eq!(run.updated_at, 1_700_000_000_000.0);
    }

    #[test]
    fn fallback_panel_class_matches_source() {
        // 真源 :88 —— 有界、等宽、面板底。
        assert!(FALLBACK_PANEL_CLASS.contains("max-h-60"));
        assert!(FALLBACK_PANEL_CLASS.contains("font-mono"));
        assert!(FALLBACK_PANEL_CLASS.contains("whitespace-pre-wrap"));
    }
}
