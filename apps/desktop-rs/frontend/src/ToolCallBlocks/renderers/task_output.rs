//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/task-output.tsx`（138 行）。
//!
//! TaskOutput 卡：任务号摘要 + 状态词（九级回落）+ 输出区（可折叠）。
//! 文案（zh-CN.ts:4849/4925-4935）。
//!
//! **已接线**：`ToolSnapshotFieldNoticeComponent`——refs 空或宿主未接回调时
//! 不渲染（真源 `return null` 语义；v4 投影下 snapshotRefs 无生产者，恒不显示）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};
use super::task_stop::to_record;

/// 真源 :9 —— `FileOutputIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const TASK_OUTPUT_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `readTaskId`（真源 :29-32）。
fn read_task_id(input: &Value) -> Option<String> {
    let record = to_record(input)?;
    let task_id = record.get("task_id").and_then(|v| v.as_str())?;
    (!task_id.trim().is_empty()).then(|| task_id.to_string())
}

/// `isFailedTaskStatus`（真源 :34-37）。
fn is_failed_task_status(status: Option<&str>) -> bool {
    matches!(
        status.map(|s| s.trim().to_lowercase()).as_deref(),
        Some("failed") | Some("lost")
    )
}

/// `isStoppedTaskStatus`（真源 :39-42）。
fn is_stopped_task_status(status: Option<&str>) -> bool {
    matches!(
        status.map(|s| s.trim().to_lowercase()).as_deref(),
        Some("cancelled") | Some("killed") | Some("stopped")
    )
}

/// 卡片模型（真源 :44-102 的取值与状态词回落）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutputModel {
    pub task_id: Option<String>,
    pub output: Option<String>,
    pub has_output: bool,
    pub truncated: bool,
    pub outcome_label: Option<String>,
    pub kind_label: &'static str,
    pub show_failure_status: bool,
    pub is_execution_failed: bool,
    pub is_running: bool,
}

/// 组装模型（真源 `TaskOutputToolCallBlock` 主体）。
pub fn build_task_output_model(
    input: &Value,
    raw: &Value,
    status: Option<&str>,
    is_running: bool,
    title: Option<&str>,
) -> TaskOutputModel {
    let display = read_tool_result_display(raw);
    let task_output_display = match display {
        Some(ToolResultDisplay::TaskOutput(d)) => Some(d),
        _ => None,
    };
    let task_id = read_task_id(input);
    let task_status = task_output_display
        .as_ref()
        .and_then(|d| d.task_status.clone());
    let normalized_task_status = task_status.as_ref().map(|s| s.trim().to_lowercase());
    let is_denied = status == Some("denied");
    let is_stopped = status == Some("stopped");
    let is_execution_failed = status == Some("failed");
    let is_task_failed = is_failed_task_status(task_status.as_deref());
    let show_failure_status = !is_denied && !is_stopped && (is_execution_failed || is_task_failed);
    let output = task_output_display.as_ref().and_then(|d| d.output.clone());
    let has_output = output.is_some();
    let retrieval_status = task_output_display
        .as_ref()
        .map(|d| d.retrieval_status.as_str());

    // 九级回落（真源 :58-91 逐条对应）。
    let outcome_label: Option<String> = if is_execution_failed {
        Some("执行失败".to_string())
    } else if is_denied {
        Some("已拒绝".to_string())
    } else if is_stopped {
        Some("已停止".to_string())
    } else if retrieval_status == Some("not_ready") {
        Some("任务运行中".to_string())
    } else if retrieval_status == Some("timeout") {
        Some("等待超时".to_string())
    } else if is_task_failed {
        Some("任务失败".to_string())
    } else if is_stopped_task_status(task_status.as_deref()) {
        Some("任务已停止".to_string())
    } else if matches!(
        normalized_task_status.as_deref(),
        Some("pending") | Some("running")
    ) {
        Some("任务运行中".to_string())
    } else if let Some(normalized) = normalized_task_status.as_deref() {
        if normalized != "completed" {
            // 原样展示未知状态（真源 :87-88 用 taskStatus ?? normalized）。
            Some(
                task_status
                    .clone()
                    .unwrap_or_else(|| normalized.to_string()),
            )
        } else {
            None
        }
    } else if retrieval_status == Some("success") {
        // 真源注释（:89-90）：类别标签只能说明这是 TaskOutput，
        // 不能替代 display 已确认的成功读取结果。
        Some("已获取".to_string())
    } else {
        None
    };

    let kind_label = if is_running {
        "正在获取任务输出"
    } else {
        "任务输出"
    };

    let _ = title;

    TaskOutputModel {
        task_id,
        output,
        has_output,
        truncated: task_output_display.as_ref().and_then(|d| d.truncated) == Some(true),
        outcome_label,
        kind_label,
        show_failure_status,
        is_execution_failed,
        is_running,
    }
}

/// `TaskOutputToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct TaskOutputBlockProps {
    pub tool_id: String,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub input: Value,
    pub raw: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
}

/// `TaskOutputToolCallBlock`（真源 :44-138）。
#[component]
pub fn TaskOutputToolCallBlock(props: TaskOutputBlockProps) -> impl IntoView {
    let model = build_task_output_model(
        &props.input,
        &props.raw,
        props.status.as_deref(),
        props.is_running,
        props.title.as_deref(),
    );
    let primary_text = model
        .task_id
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "TaskOutput".to_string());
    let status_tooltip = model
        .is_execution_failed
        .then(|| props.error_text.clone())
        .flatten();
    let output_text = model.output.clone().unwrap_or_default();
    let truncated = model.truncated;
    let has_output = model.has_output;
    let render_content = move || {
        let output_text = output_text.clone();
        view! {
            <div class="rounded-lg border border-border bg-panel px-4 py-3">
                <pre class="max-h-25 overflow-auto font-mono text-ui-base break-words whitespace-pre-wrap text-foreground-subtle">
                    {output_text}
                </pre>
                {truncated.then(|| view! {
                    <p class="mt-3 text-ui-xs text-foreground-subtle">"后续内容已省略。"</p>
                })}
            </div>
        }
        .into_any()
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(has_output),
                force_open: Some(false),
                kind_label: Some(model.kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text.clone()),
                status_label: model.outcome_label.clone(),
                show_status_label: Some(model.outcome_label.is_some()),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(model.show_failure_status),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=TASK_OUTPUT_TOOL_ICON_CLASS>
                        // FileOutputIcon（lucide）：文件 + 出箭头。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M4 7V4h16v3"></path>
                            <path d="M4 14h6"></path>
                            <path d="M10 10v8"></path>
                            <path d="m14 14 3 3 3-3"></path>
                            <path d="M20 21H4v-5"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            render_content=if has_output { Some(std::sync::Arc::new(render_content)) } else { None }
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
    use serde_json::json;

    fn model_with(raw: Value, status: Option<&str>, running: bool) -> TaskOutputModel {
        build_task_output_model(&json!({"task_id": "t1"}), &raw, status, running, None)
    }

    #[test]
    fn outcome_ladder_execution_states_first() {
        // 真源 :58-68 —— 执行态优先于任务态。
        assert_eq!(
            model_with(json!({}), Some("failed"), false)
                .outcome_label
                .as_deref(),
            Some("执行失败")
        );
        assert_eq!(
            model_with(json!({}), Some("denied"), false)
                .outcome_label
                .as_deref(),
            Some("已拒绝")
        );
        assert_eq!(
            model_with(json!({}), Some("stopped"), false)
                .outcome_label
                .as_deref(),
            Some("已停止")
        );
    }

    #[test]
    fn outcome_ladder_retrieval_and_task_states() {
        // not_ready → 任务运行中（真源 :69-70）。
        let raw = json!({"rawOutput": {"display": {"kind": "task_output", "retrievalStatus": "not_ready"}}});
        assert_eq!(
            model_with(raw, None, false).outcome_label.as_deref(),
            Some("任务运行中")
        );
        // timeout（真源 :71-72）。
        let raw = json!({"rawOutput": {"display": {"kind": "task_output", "retrievalStatus": "timeout"}}});
        assert_eq!(
            model_with(raw, None, false).outcome_label.as_deref(),
            Some("等待超时")
        );
        // taskStatus failed → 任务失败。
        let raw = json!({"rawOutput": {"display": {"kind": "task_output", "retrievalStatus": "success", "taskStatus": "failed"}}});
        assert_eq!(
            model_with(raw, None, false).outcome_label.as_deref(),
            Some("任务失败")
        );
        // retrieval success → 已获取（真源 :89-91）。
        let raw = json!({"rawOutput": {"display": {"kind": "task_output", "retrievalStatus": "success"}}});
        assert_eq!(
            model_with(raw, None, false).outcome_label.as_deref(),
            Some("已获取")
        );
        // 全落空 → 无状态词。
        assert_eq!(model_with(json!({}), None, false).outcome_label, None);
    }

    #[test]
    fn unknown_task_status_shown_verbatim() {
        // 真源 :87-88 —— completed 之外的原样展示。
        let raw = json!({"rawOutput": {"display": {"kind": "task_output", "retrievalStatus": "success", "taskStatus": "paused"}}});
        assert_eq!(
            model_with(raw, None, false).outcome_label.as_deref(),
            Some("paused")
        );
    }

    #[test]
    fn failure_status_gate_excludes_denied_stopped() {
        // 真源 :53 —— denied/stopped 不显失败态。
        assert!(!model_with(json!({}), Some("denied"), false).show_failure_status);
        assert!(!model_with(json!({}), Some("stopped"), false).show_failure_status);
        assert!(model_with(json!({}), Some("failed"), false).show_failure_status);
    }

    #[test]
    fn output_gates_foldability() {
        // 有输出才可折叠（真源 :100-101）。
        let with_output = json!({"rawOutput": {"display": {"kind": "task_output", "retrievalStatus": "success", "output": "结果文本"}}});
        let model = model_with(with_output, None, false);
        assert!(model.has_output);
        assert_eq!(model.output.as_deref(), Some("结果文本"));
        assert!(!model_with(json!({}), None, false).has_output);
    }
}
