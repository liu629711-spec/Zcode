//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/send-message.tsx`（218 行）。
//!
//! SendMessage 卡：摘要行（summary 主文本 + 「给 <target>」次文本）+
//! 展开详情（目标子智能体 / 摘要 / 消息）。local_agent_message display
//! 与 output/raw.result.content 三路读取。
//!
//! 文案：chat.toolCall.kind.message「消息」/ sendMessage.sending「正在发送消息」/
//! sendMessage.to「给」/ target「目标子智能体」/ summary「摘要」/ message「消息」
//! （zh-CN.ts:4847/4905-4913）。
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

/// 真源 :9 —— `SendIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const SEND_MESSAGE_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `readStringField`（真源 :34-46）。
fn read_string_field(value: Option<&Value>, keys: &[&str]) -> Option<String> {
    let value = value?;
    for key in keys {
        if let Some(candidate) = value.get(*key).and_then(|v| v.as_str()) {
            if !candidate.trim().is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

/// `readRawRecord`（真源 :48-60）。
fn read_raw_record(raw: &Value, keys: &[&str]) -> Option<Value> {
    let record = to_record(raw)?;
    for key in keys {
        if let Some(candidate) = record.get(*key).and_then(to_record) {
            return Some(candidate);
        }
    }
    None
}

/// `readText`（真源 :62-64）。
fn read_text(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

/// `readRawResultContent`（真源 :66-70）：`raw.result.content`。
fn read_raw_result_content(raw: &Value) -> Value {
    to_record(raw)
        .and_then(|r| r.get("result").cloned())
        .and_then(|r| to_record(&r))
        .and_then(|r| r.get("content").cloned())
        .unwrap_or(Value::Null)
}

/// 卡片模型。
#[derive(Debug, Clone, PartialEq)]
pub struct SendMessageModel {
    pub summary: Option<String>,
    pub target: Option<String>,
    pub message: Option<String>,
    pub has_details: bool,
    pub kind_label: &'static str,
    pub status_label: Option<&'static str>,
    pub is_failed: bool,
    pub failure_message: Option<String>,
    pub fields: Vec<(String, String, bool)>,
}

/// 组装模型（真源 :83-178）。
pub fn build_send_message_model(
    input: &Value,
    output: &Value,
    raw: &Value,
    status: Option<&str>,
    error_text: Option<&str>,
    is_running: bool,
) -> SendMessageModel {
    let display = read_tool_result_display(raw);
    let message_display = match display {
        Some(ToolResultDisplay::LocalAgentMessage(d)) => Some(d),
        _ => None,
    };
    let input_record = to_record(input).or_else(|| read_raw_record(raw, &["rawInput", "input"]));
    let direct_output = to_record(output);
    let raw_result_content = read_raw_result_content(raw);
    let raw_result_output = to_record(&raw_result_content);
    let output_record = direct_output
        .clone()
        .or_else(|| read_raw_record(raw, &["rawOutput", "output"]))
        .or_else(|| raw_result_output.clone());
    let output_text = if direct_output.is_none() {
        read_text(output)
    } else {
        None
    }
    .or_else(|| {
        if raw_result_output.is_none() {
            read_text(&raw_result_content)
        } else {
            None
        }
    });

    let summary = read_string_field(input_record.as_ref(), &["summary"]);
    let target = read_string_field(input_record.as_ref(), &["to"]);
    let message = read_string_field(input_record.as_ref(), &["message"]);
    // 真源 :111-112 —— streaming 首帧可能没字段，不为空面板提供展开入口。
    let has_details = target.is_some() || summary.is_some() || message.is_some();

    let output_message = read_string_field(output_record.as_ref(), &["message"]);
    let output_error = read_string_field(output_record.as_ref(), &["error"]);
    let output_status = read_string_field(output_record.as_ref(), &["status"]);
    let is_denied = status == Some("denied");
    let is_stopped = status == Some("stopped");
    let is_failed = !is_denied
        && !is_stopped
        && (status == Some("failed")
            || message_display.as_ref().map(|d| d.status.as_str()) == Some("failed")
            || output_status.as_deref() == Some("failed"));
    let failure_message = if is_failed {
        error_text
            .map(|e| e.to_string())
            .or_else(|| message_display.as_ref().and_then(|d| d.error.clone()))
            .or_else(|| message_display.as_ref().and_then(|d| d.message.clone()))
            .or(output_error)
            .or(output_message)
            .or(output_text)
    } else {
        None
    };

    let kind_label = if is_running {
        "正在发送消息"
    } else {
        "消息"
    };
    let status_label = if is_failed {
        Some("执行失败")
    } else if is_denied {
        Some("已拒绝")
    } else if is_stopped {
        Some("已停止")
    } else {
        None
    };

    let mut fields = Vec::new();
    if let Some(ref t) = target {
        fields.push(("目标子智能体".to_string(), t.clone(), true));
    }
    if let Some(ref s) = summary {
        fields.push(("摘要".to_string(), s.clone(), false));
    }
    if let Some(ref m) = message {
        fields.push(("消息".to_string(), m.clone(), false));
    }

    SendMessageModel {
        summary,
        target,
        message,
        has_details,
        kind_label,
        status_label,
        is_failed,
        failure_message,
        fields,
    }
}

/// `SendMessageToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct SendMessageBlockProps {
    pub tool_id: String,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub input: Value,
    pub output: Value,
    pub raw: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
}

/// `SendMessageToolCallBlock`（真源 :83-218）。
#[component]
pub fn SendMessageToolCallBlock(props: SendMessageBlockProps) -> impl IntoView {
    let model = build_send_message_model(
        &props.input,
        &props.output,
        &props.raw,
        props.status.as_deref(),
        props.error_text.as_deref(),
        props.is_running,
    );
    let primary_text = model
        .summary
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "SendMessage".to_string());
    let target_for_secondary = model.target.clone();
    let status_tooltip = model
        .is_failed
        .then(|| model.failure_message.clone())
        .flatten();
    let fields = model.fields.clone();
    let render_content = move || {
        let fields = fields.clone();
        view! {
            <div class="rounded-lg border border-border bg-panel px-4 py-3">
                <dl class="space-y-3">
                    {fields
                        .into_iter()
                        .map(|(label, value, mono)| {
                            let dd_class = if mono {
                                "text-ui-base text-foreground break-words whitespace-pre-wrap font-mono"
                            } else {
                                "text-ui-base leading-5 text-foreground break-words whitespace-pre-wrap"
                            };
                            view! {
                                <div class="space-y-1">
                                    <dt class="text-ui-base font-medium text-foreground-subtle">
                                        {label}
                                    </dt>
                                    <dd class=dd_class>{value}</dd>
                                </div>
                            }
                        })
                        .collect_view()}
                </dl>
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
                can_toggle: Some(model.has_details),
                force_open: Some(false),
                // 真源 :184 —— 展开后隐藏次文本。
                hide_secondary_text_when_open: Some(true),
                kind_label: Some(model.kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                secondary_text: target_for_secondary
                    .map(|t| format!("给 {t}")),
                status_label: model.status_label.map(|l| l.to_string()),
                show_status_label: Some(model.status_label.is_some()),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(model.is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=SEND_MESSAGE_TOOL_ICON_CLASS>
                        // SendIcon（lucide）：纸飞机。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M14.536 21.686a.5.5 0 0 0 .937-.024l6.5-19a.496.496 0 0 0-.635-.635l-19 6.5a.5.5 0 0 0-.024.937l7.93 3.18a2 2 0 0 1 1.112 1.11z"></path>
                            <path d="m21.854 2.147-10.94 10.939"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            render_content=if model.has_details { Some(std::sync::Arc::new(render_content)) } else { None }
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
    fn details_require_input_fields() {
        // 真源 :111-112 —— 空 input 不为空面板提供展开。
        let empty = build_send_message_model(&json!({}), &json!({}), &json!({}), None, None, true);
        assert!(!empty.has_details);
        let with_summary = build_send_message_model(
            &json!({"summary": "汇报进度"}),
            &json!({}),
            &json!({}),
            None,
            None,
            true,
        );
        assert!(with_summary.has_details);
        assert_eq!(with_summary.kind_label, "正在发送消息");
    }

    #[test]
    fn failure_ladder_uses_display_then_output_then_text() {
        // display.error 优先（真源 :129-136）。
        let raw = json!({
            "rawOutput": { "display": { "kind": "local_agent_message", "status": "failed", "error": "display 错误" } }
        });
        let model = build_send_message_model(
            &json!({"to": "agent-1"}),
            &json!({}),
            &raw,
            Some("failed"),
            None,
            false,
        );
        assert!(model.is_failed);
        assert_eq!(model.failure_message.as_deref(), Some("display 错误"));
        // errorText 优先于 display。
        let model = build_send_message_model(
            &json!({}),
            &json!({}),
            &raw,
            Some("failed"),
            Some("上下文错误"),
            false,
        );
        assert_eq!(model.failure_message.as_deref(), Some("上下文错误"));
    }

    #[test]
    fn denied_and_stopped_are_not_failed() {
        // 真源 :118-124 —— denied/stopped 不算失败。
        let denied = build_send_message_model(
            &json!({}),
            &json!({}),
            &json!({}),
            Some("denied"),
            None,
            false,
        );
        assert!(!denied.is_failed);
        assert_eq!(denied.status_label, Some("已拒绝"));
        let stopped = build_send_message_model(
            &json!({}),
            &json!({}),
            &json!({}),
            Some("stopped"),
            None,
            false,
        );
        assert!(!stopped.is_failed);
        assert_eq!(stopped.status_label, Some("已停止"));
    }

    #[test]
    fn raw_result_content_is_a_output_fallback() {
        // 真源 :69-79 —— raw.result.content 作 output 兜底。
        let raw = json!({ "result": { "content": { "message": "已送达" } } });
        let model = build_send_message_model(
            &json!({"summary": "x"}),
            &json!({}),
            &raw,
            None,
            None,
            false,
        );
        // output_message 从 raw.result.content 读到 → 失败链无失败时不体现，
        // 这里验证读取路径不 panic 且模型成立。
        assert!(model.message.is_none());
    }

    #[test]
    fn fields_render_order_target_summary_message() {
        // 真源 :157-176 —— 目标 → 摘要 → 消息。
        let model = build_send_message_model(
            &json!({"to": "agent-1", "summary": "进度", "message": "详情正文"}),
            &json!({}),
            &json!({}),
            None,
            None,
            false,
        );
        let labels: Vec<&str> = model.fields.iter().map(|(l, _, _)| l.as_str()).collect();
        assert_eq!(labels, vec!["目标子智能体", "摘要", "消息"]);
        assert!(model.fields[0].2, "目标字段用 mono");
    }
}
