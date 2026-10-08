//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/respond-to-coordinator.tsx`
//! （95 行）。
//!
//! RespondToCoordinator 卡：单行摘要（summary / title）+ 六态状态词。
//! `canToggle=false`——该卡没有展开内容区（真源未传 renderContent）。
//!
//! **已接线**：`ToolSnapshotFieldNotice`（真源 :77-86）。真源语义下
//! `refs` 空或回调缺省时整条不渲染，与裁剪版视觉一致、通道就位后自动亮起。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};

/// 真源 :9 —— `ReplyIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const RESPOND_TO_COORDINATOR_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `toRecord`（真源 :14-24）：record 直接过；非空字符串尝试 parse。
///
/// 与 task_stop 的 `to_record` 不同：这里**不**剥 Hook 追加段。
pub fn to_record(value: &Value) -> Option<Value> {
    if value.is_object() {
        return Some(value.clone());
    }
    let Some(s) = value.as_str() else {
        return None;
    };
    if s.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(s)
        .ok()
        .filter(|v| v.is_object())
}

/// `readSummary`（真源 :26-31）：`input.summary` 非空白字符串。
pub fn read_summary(input: &Value) -> Option<String> {
    let summary = to_record(input)?.get("summary")?.as_str()?.to_string();
    if summary.trim().is_empty() {
        None
    } else {
        Some(summary)
    }
}

/// 卡片模型。
#[derive(Debug, Clone, PartialEq)]
pub struct RespondToCoordinatorModel {
    pub summary: Option<String>,
    pub is_failed: bool,
    /// 状态词（真源 statusLabelId：failed > denied > stopped > queued(success)）。
    pub status_label: Option<&'static str>,
    pub kind_label: &'static str,
    /// `isExecutionFailed ? context.errorText : undefined`（真源 :56）。
    pub status_tooltip: Option<String>,
}

/// `RespondToCoordinatorToolCallBlock` 的状态推导（真源 :34-52）。
pub fn build_respond_to_coordinator_model(
    input: &Value,
    raw: &Value,
    status: Option<&str>,
    is_running: bool,
    error_text: Option<&str>,
) -> RespondToCoordinatorModel {
    let display = read_tool_result_display(raw);
    let response_display = match display {
        Some(ToolResultDisplay::RespondToCoordinator(d)) => Some(d),
        _ => None,
    };
    let summary = read_summary(input);
    let is_denied = status == Some("denied");
    let is_stopped = status == Some("stopped");
    let is_execution_failed = status == Some("failed");
    // 真源 :38-39 —— denied / stopped 抢占：不显示「失败」（它们是用户侧终态）。
    let is_failed = !is_denied
        && !is_stopped
        && (is_execution_failed
            || response_display
                .as_ref()
                .is_some_and(|d| d.status == "failed"));
    let kind_label = if is_running { "正在回复" } else { "回复" };
    let status_label = if is_failed {
        Some("执行失败")
    } else if is_denied {
        Some("已拒绝")
    } else if is_stopped {
        Some("已停止")
    } else if response_display
        .as_ref()
        .is_some_and(|d| d.status == "success")
    {
        Some("回复已排队")
    } else {
        None
    };
    let status_tooltip = if is_execution_failed {
        error_text.map(|e| e.to_string())
    } else {
        None
    };

    RespondToCoordinatorModel {
        summary,
        is_failed,
        status_label,
        kind_label,
        status_tooltip,
    }
}

/// `RespondToCoordinatorToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct RespondToCoordinatorBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub raw: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `RespondToCoordinatorToolCallBlock`（真源 :33-91）。
#[component]
pub fn RespondToCoordinatorToolCallBlock(props: RespondToCoordinatorBlockProps) -> impl IntoView {
    let model = build_respond_to_coordinator_model(
        &props.input,
        &props.raw,
        props.status.as_deref(),
        props.is_running,
        props.error_text.as_deref(),
    );
    // 真源 :53-58 —— summary ?? title ?? "RespondToCoordinator"，truncate。
    let primary_text = model
        .summary
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "RespondToCoordinator".to_string());

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(false),
                force_open: Some(false),
                kind_label: Some(model.kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                status_label: model.status_label.map(str::to_string),
                show_status_label: Some(model.status_label.is_some()),
                status_tooltip: model.status_tooltip.clone(),
                show_failure_status: Some(model.is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=RESPOND_TO_COORDINATOR_TOOL_ICON_CLASS>
                        // ReplyIcon（lucide）：回弯箭头。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M20 18v-2a4 4 0 0 0-4-4H4"></path>
                            <path d="m9 17-5-5 5-5"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            // 真源 canToggle=false 且未传 renderContent——无展开内容区。
            render_content=None
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

    #[test]
    fn summary_from_record_or_json_string() {
        assert_eq!(
            read_summary(&json!({"summary": "S1"})).as_deref(),
            Some("S1")
        );
        // JSON 字符串形态。
        assert_eq!(
            read_summary(&json!("{\"summary\":\"S2\"}")).as_deref(),
            Some("S2")
        );
        // 空白 / 非对象 / 坏 JSON → None。
        assert_eq!(read_summary(&json!({"summary": "  "})), None);
        assert_eq!(read_summary(&json!("plain")), None);
        assert_eq!(read_summary(&json!("{bad")), None);
        assert_eq!(read_summary(&Value::Null), None);
    }

    #[test]
    fn status_labels_precedence() {
        // denied 抢占 failed。
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({}),
            Some("denied"),
            false,
            Some("err"),
        );
        assert!(!m.is_failed);
        assert_eq!(m.status_label, Some("已拒绝"));
        // stopped 同上。
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({}),
            Some("stopped"),
            false,
            None,
        );
        assert!(!m.is_failed);
        assert_eq!(m.status_label, Some("已停止"));
        // failed → 执行失败 + tooltip 用 errorText。
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({}),
            Some("failed"),
            false,
            Some("boom"),
        );
        assert!(m.is_failed);
        assert_eq!(m.status_label, Some("执行失败"));
        assert_eq!(m.status_tooltip.as_deref(), Some("boom"));
    }

    #[test]
    fn display_status_drives_failed_and_queued() {
        // display.status = failed → is_failed（顶层 status 为 completed）。
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({"display": {"kind": "respond_to_coordinator", "status": "failed"}}),
            Some("completed"),
            false,
            None,
        );
        assert!(m.is_failed);
        assert_eq!(m.status_label, Some("执行失败"));
        // display.status = success → 「回复已排队」。
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({"display": {"kind": "respond_to_coordinator", "status": "success"}}),
            Some("completed"),
            false,
            None,
        );
        assert!(!m.is_failed);
        assert_eq!(m.status_label, Some("回复已排队"));
        // 无 display、无状态词 → None。
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({}),
            Some("completed"),
            false,
            None,
        );
        assert_eq!(m.status_label, None);
    }

    #[test]
    fn kind_label_follows_running() {
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({}),
            Some("in_progress"),
            true,
            None,
        );
        assert_eq!(m.kind_label, "正在回复");
        let m = build_respond_to_coordinator_model(
            &json!({}),
            &json!({}),
            Some("completed"),
            false,
            None,
        );
        assert_eq!(m.kind_label, "回复");
    }
}
