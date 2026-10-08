//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/task-stop.tsx`（215 行）。
//!
//! TaskStop 卡：taskId 摘要行 + 展开详情（任务类型 / 命令或描述 / 结果）。
//!
//! **已接线**：`ToolSnapshotFieldNoticeComponent`——refs 空或宿主未接回调时
//! 不渲染（真源 `return null` 语义；v4 投影下 snapshotRefs 无生产者，恒不显示）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};

/// 真源 :10 —— `CircleStopIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const TASK_STOP_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `HOOK_ADDITIONAL_CONTEXT_MARKER`（真源 :12）。
const HOOK_ADDITIONAL_CONTEXT_MARKER: &str = "\n\n[Hook additional context]";

/// `toRecord`（真源 :14-32）：record 直接过；字符串剥 Hook 追加段后 parse。
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
    let serialized = match s.find(HOOK_ADDITIONAL_CONTEXT_MARKER) {
        Some(index) => &s[..index],
        None => s,
    };
    serde_json::from_str::<Value>(serialized)
        .ok()
        .filter(|v| v.is_object())
}

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

/// `compactLegacyTaskStopResult`（真源 :62-75）：旧快照的标准成功文案去重。
///
/// 真源注释：旧 snapshot 只保存了会重复 command/prompt 的标准成功文案；
/// **仅做完整模板匹配**，避免裁剪 provider 返回的自定义结果。
pub fn compact_legacy_task_stop_result(
    message: Option<&str>,
    task_id: Option<&str>,
    command: Option<&str>,
) -> Option<String> {
    let (Some(message), Some(task_id), Some(command)) = (message, task_id, command) else {
        return message.map(|m| m.to_string());
    };
    let standard_message = format!("Successfully stopped task: {task_id} ({command})");
    if message == standard_message {
        Some(format!("Successfully stopped task: {task_id}"))
    } else {
        Some(message.to_string())
    }
}

/// `DetailField` 的数据（真源 :77-101）。
#[derive(Debug, Clone, PartialEq)]
pub struct DetailFieldData {
    pub label: String,
    pub value: String,
    pub mono: bool,
}

/// 卡的全部取值（供组件与测试共用；真源在组件体里直接算）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskStopModel {
    pub task_id: Option<String>,
    pub task_type: Option<String>,
    /// 展开详情里的任务描述/命令（local_agent 只认 display 的短 description）。
    pub task_detail: Option<String>,
    pub is_local_agent_task: bool,
    pub result_message: Option<String>,
    pub details_truncated: bool,
    pub has_details: bool,
    pub kind_label: &'static str,
    pub status_label: Option<&'static str>,
    pub is_failed: bool,
    pub is_denied: bool,
    pub is_stopped: bool,
    pub fields: Vec<DetailFieldData>,
}

/// 组装卡片模型（真源 :103-183 的取值与展开区逻辑）。
pub fn build_task_stop_model(
    input: &Value,
    output: &Value,
    raw: &Value,
    status: Option<&str>,
    error_text: Option<&str>,
    is_running: bool,
    title: Option<&str>,
) -> TaskStopModel {
    let display = read_tool_result_display(raw);
    let task_stop_display = match display {
        Some(ToolResultDisplay::TaskStop(d)) => Some(d),
        _ => None,
    };
    let input_record = to_record(input).or_else(|| read_raw_record(raw, &["rawInput", "input"]));
    let output_record =
        to_record(output).or_else(|| read_raw_record(raw, &["rawOutput", "output"]));

    let task_id = task_stop_display
        .as_ref()
        .map(|d| d.task_id.clone())
        .or_else(|| read_string_field(output_record.as_ref(), &["task_id"]))
        .or_else(|| read_string_field(input_record.as_ref(), &["task_id", "shell_id"]));
    let task_type = task_stop_display
        .as_ref()
        .map(|d| d.task_type.clone())
        .or_else(|| read_string_field(output_record.as_ref(), &["task_type"]));
    let display_command = task_stop_display.as_ref().and_then(|d| d.command.clone());
    let legacy_command = read_string_field(output_record.as_ref(), &["command"]);
    let is_local_agent_task = task_type.as_deref() == Some("local_agent");
    // 真源 :124-126：旧 local_agent 快照的 command 可能是完整 prompt，
    // 只有 Core 投影的 display 才能作为短 description 展示。
    let task_detail = if is_local_agent_task {
        display_command.clone()
    } else {
        display_command.clone().or_else(|| legacy_command.clone())
    };
    let result_command = display_command.or(legacy_command);
    let output_message = task_stop_display
        .as_ref()
        .map(|d| d.message.clone())
        .or_else(|| read_string_field(output_record.as_ref(), &["message"]));
    let details_truncated = task_stop_display.as_ref().and_then(|d| d.truncated) == Some(true);

    let is_denied = status == Some("denied");
    let is_stopped = status == Some("stopped");
    let is_failed = status == Some("failed");
    let is_unsuccessful = is_failed || is_denied || is_stopped;
    let result_message = if is_unsuccessful {
        error_text
            .map(|e| e.to_string())
            .or_else(|| output_message.clone())
    } else {
        compact_legacy_task_stop_result(
            output_message.as_deref(),
            task_id.as_deref(),
            result_command.as_deref(),
        )
    };
    let has_details = task_type.is_some()
        || task_detail.is_some()
        || result_message.is_some()
        || details_truncated;

    let kind_label = if is_running {
        "正在停止任务"
    } else {
        "停止任务"
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

    // 展开区字段（真源 :141-166）。
    let mut fields = Vec::new();
    if let Some(ref tt) = task_type {
        fields.push(DetailFieldData {
            label: "任务类型".to_string(),
            value: tt.clone(),
            mono: true,
        });
    }
    if let Some(ref detail) = task_detail {
        fields.push(DetailFieldData {
            label: if is_local_agent_task {
                "任务描述".to_string()
            } else {
                "命令".to_string()
            },
            value: detail.clone(),
            mono: !is_local_agent_task,
        });
    }
    if let Some(ref msg) = result_message {
        fields.push(DetailFieldData {
            label: "结果".to_string(),
            value: msg.clone(),
            mono: false,
        });
    }

    let _ = title; // 摘要行 title 由组件就近取（保持模型纯度）。

    TaskStopModel {
        task_id,
        task_type,
        task_detail,
        is_local_agent_task,
        result_message,
        details_truncated,
        has_details,
        kind_label,
        status_label,
        is_failed,
        is_denied,
        is_stopped,
        fields,
    }
}

/// `TaskStopToolCallBlock` 的 props（真源从 context 解构的字段）。
#[derive(Debug, Clone)]
pub struct TaskStopBlockProps {
    pub tool_id: String,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub input: Value,
    pub output: Value,
    pub raw: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
}

/// `TaskStopToolCallBlock`（真源 :103-215）。
#[component]
pub fn TaskStopToolCallBlock(props: TaskStopBlockProps) -> impl IntoView {
    let model = build_task_stop_model(
        &props.input,
        &props.output,
        &props.raw,
        props.status.as_deref(),
        props.error_text.as_deref(),
        props.is_running,
        props.title.as_deref(),
    );
    let primary_text = model
        .task_id
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "TaskStop".to_string());
    let status_tooltip = model
        .is_failed
        .then(|| model.result_message.clone())
        .flatten();
    let fields = model.fields.clone();
    let truncated = model.details_truncated;
    let render_content = move || {
        let fields = fields.clone();
        view! {
            <div class="rounded-lg border border-border bg-panel px-4 py-3">
                <dl class="space-y-3">
                    {fields
                        .into_iter()
                        .map(|field| {
                            let dd_class = if field.mono {
                                "text-ui-base text-foreground break-words whitespace-pre-wrap font-mono"
                            } else {
                                "text-ui-base leading-5 text-foreground break-words whitespace-pre-wrap"
                            };
                            view! {
                                <div class="space-y-1">
                                    <dt class="text-ui-base font-medium text-foreground-subtle">
                                        {field.label}
                                    </dt>
                                    <dd class=dd_class>{field.value}</dd>
                                </div>
                            }
                        })
                        .collect_view()}
                </dl>
                {truncated.then(|| view! {
                    <p class="mt-3 text-ui-xs text-foreground-subtle">"部分任务详情已截断。"</p>
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
                can_toggle: Some(model.has_details),
                force_open: Some(false),
                kind_label: Some(model.kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
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
                    <span class=TASK_STOP_TOOL_ICON_CLASS>
                        // CircleStopIcon（lucide）：圆 + 方。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="10"></circle>
                            <rect x="9" y="9" width="6" height="6" rx="1"></rect>
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
    fn hook_marker_is_stripped_before_parse() {
        // 真源 :22-30 —— Hook 追加段剥除后再 parse。
        let with_marker = json!("{\"task_id\":\"t1\"}\n\n[Hook additional context]\n备注");
        let record = to_record(&with_marker).unwrap();
        assert_eq!(record.get("task_id").and_then(|v| v.as_str()), Some("t1"));
        // 无 marker 原样 parse。
        let plain = json!("{\"task_id\":\"t2\"}");
        assert_eq!(
            to_record(&plain)
                .unwrap()
                .get("task_id")
                .and_then(|v| v.as_str()),
            Some("t2")
        );
    }

    #[test]
    fn compact_result_strips_standard_template_only() {
        // 真源 :62-75 —— 只对完整模板去重。
        assert_eq!(
            compact_legacy_task_stop_result(
                Some("Successfully stopped task: t1 (npm run dev)"),
                Some("t1"),
                Some("npm run dev")
            )
            .as_deref(),
            Some("Successfully stopped task: t1")
        );
        // 自定义结果不裁剪。
        assert_eq!(
            compact_legacy_task_stop_result(Some("自定义消息"), Some("t1"), Some("cmd")).as_deref(),
            Some("自定义消息")
        );
        // message 缺失原样 None。
        assert_eq!(
            compact_legacy_task_stop_result(None, Some("t1"), Some("cmd")),
            None
        );
    }

    #[test]
    fn display_takes_priority_over_output_records() {
        // 真源 :110-116 —— display 优先于 output/input。
        let raw = json!({
            "rawOutput": {
                "display": { "kind": "task_stop", "taskId": "d1", "taskType": "local_agent", "message": "已停止" }
            }
        });
        let model = build_task_stop_model(
            &json!({"task_id": "i1"}),
            &json!({}),
            &raw,
            Some("stopped"),
            None,
            false,
            None,
        );
        assert_eq!(model.task_id.as_deref(), Some("d1"));
        assert_eq!(model.task_type.as_deref(), Some("local_agent"));
        assert!(model.is_local_agent_task);
    }

    #[test]
    fn local_agent_command_falls_back_to_display_only() {
        // 真源 :124-126 —— local_agent 时旧 command 可能是完整 prompt，不采用。
        // 注意 output 传 Null（字段缺失）——空对象 {} 会短路 read_raw_record
        // （真源 toRecord({}) 亦 truthy 短路，夹具要模拟「字段缺失」而非「空对象」）。
        let raw = json!({
            "rawOutput": { "task_id": "t1", "task_type": "local_agent", "command": "完整 prompt 文本" }
        });
        let model = build_task_stop_model(&json!({}), &Value::Null, &raw, None, None, false, None);
        assert_eq!(
            model.task_detail, None,
            "无 display 时 local_agent 不采用旧 command"
        );
        // 非 local_agent 时旧 command 可用。
        let raw = json!({
            "rawOutput": { "task_id": "t1", "task_type": "shell", "command": "npm run dev" }
        });
        let model = build_task_stop_model(&json!({}), &Value::Null, &raw, None, None, false, None);
        assert_eq!(model.task_detail.as_deref(), Some("npm run dev"));
    }

    #[test]
    fn status_labels_and_failure_semantics() {
        // 真源 :172-181 —— 失败/拒绝/停止的状态词。
        let failed = build_task_stop_model(
            &json!({}),
            &json!({}),
            &json!({}),
            Some("failed"),
            Some("错误正文"),
            false,
            None,
        );
        assert_eq!(failed.status_label, Some("执行失败"));
        assert!(failed.is_failed);
        // 失败时结果用 errorText 优先。
        assert_eq!(failed.result_message.as_deref(), Some("错误正文"));
        let denied = build_task_stop_model(
            &json!({}),
            &json!({}),
            &json!({}),
            Some("denied"),
            None,
            false,
            None,
        );
        assert_eq!(denied.status_label, Some("已拒绝"));
        let stopped = build_task_stop_model(
            &json!({}),
            &json!({}),
            &json!({}),
            Some("stopped"),
            None,
            false,
            None,
        );
        assert_eq!(stopped.status_label, Some("已停止"));
    }

    #[test]
    fn kind_label_switches_on_running() {
        let running =
            build_task_stop_model(&json!({}), &json!({}), &json!({}), None, None, true, None);
        assert_eq!(running.kind_label, "正在停止任务");
        let done =
            build_task_stop_model(&json!({}), &json!({}), &json!({}), None, None, false, None);
        assert_eq!(done.kind_label, "停止任务");
        assert!(!done.has_details, "空数据无详情可展开");
    }
}
