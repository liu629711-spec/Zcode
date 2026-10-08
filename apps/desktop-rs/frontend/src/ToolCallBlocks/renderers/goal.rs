//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/goal.tsx`（207 行）。
//!
//! Goal 卡：结果 payload（status / result / content / thought / error 去重
//! 合并）的 pretty JSON 展示。`ToolSnapshotFieldNotice` 在**展开区内**
//! （真源 :196-202 位于 renderContent 而非 Fragment 兄弟位）。
//!
//! **键序注意**：真源 `JSON.stringify(payload, null, 2)` 按插入序输出；
//! workspace 已启用 serde_json `preserve_order`（IndexMap）对齐该行为。
//!
//! **裁剪注明**：`canToggle ?? true` / `forceOpen ?? false` 走真源默认
//! （v1 Context 未承载宿主覆盖位）。

use leptos::prelude::*;
use serde_json::{Map, Value};

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

/// 真源 :5 —— `GoalIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const GOAL_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `readNestedValue`（真源 :13-22）：逐级下钻，非 record 即 undefined。
fn read_nested_value<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        if !current.is_object() {
            return None;
        }
        current = current.get(*key)?;
    }
    Some(current)
}

/// `normalizeResultValue`（真源 :24-39）：字符串 trim 空 → 空；JSON parse
/// 成功 → 解析值；失败 → **原字符串**（未 trim）。
fn normalize_result_value(value: Option<&Value>) -> Option<Value> {
    let Some(value) = value else {
        return None;
    };
    let Value::String(s) = value else {
        return Some(value.clone());
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(parsed) => Some(parsed),
        Err(_) => Some(value.clone()),
    }
}

/// `isEmptyResultValue`（真源 :41-51）：undefined / null / 空白字符串。
fn is_empty_result_value(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    }
}

/// `stringifyResultValue`（真源 :53-61）：字符串原样；其余紧凑 JSON。
fn stringify_result_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// `isSameResultValue`（真源 :63-71）：任一为空 → false；否则字符串化比较。
fn is_same_result_value(left: Option<&Value>, right: Option<&Value>) -> bool {
    if is_empty_result_value(left) || is_empty_result_value(right) {
        return false;
    }
    stringify_result_value(left.expect("非空已判"))
        == stringify_result_value(right.expect("非空已判"))
}

/// `readPrimaryResult`（真源 :73-88）：output → raw.result.content →
/// raw.rawOutput → raw.output 四路回落。
fn read_primary_result(output: &Value, raw: &Value) -> Option<Value> {
    let candidates = [
        Some(output.clone()),
        read_nested_value(raw, &["result", "content"]).cloned(),
        read_nested_value(raw, &["rawOutput"]).cloned(),
        read_nested_value(raw, &["output"]).cloned(),
    ];
    for candidate in &candidates {
        let normalized = normalize_result_value(candidate.as_ref());
        if !is_empty_result_value(normalized.as_ref()) {
            return normalized;
        }
    }
    None
}

/// `readAdditionalContent`（真源 :90-104）：content → raw.content →
/// raw.result.display，须非空且与 primary 不同。
fn read_additional_content(
    content: Option<&str>,
    raw: &Value,
    primary_result: Option<&Value>,
) -> Option<Value> {
    let content_value = content.map(|c| Value::String(c.to_string()));
    let candidates = [
        content_value,
        read_nested_value(raw, &["content"]).cloned(),
        read_nested_value(raw, &["result", "display"]).cloned(),
    ];
    for candidate in &candidates {
        let normalized = normalize_result_value(candidate.as_ref());
        if !is_empty_result_value(normalized.as_ref())
            && !is_same_result_value(normalized.as_ref(), primary_result)
        {
            return normalized;
        }
    }
    None
}

/// `buildGoalResultPayload`（真源 :106-137）。
pub fn build_goal_result_payload(
    status: &str,
    output: &Value,
    content: Option<&str>,
    thought: Option<&str>,
    raw: &Value,
    error_text: Option<&str>,
) -> Value {
    let primary_result = read_primary_result(output, raw);
    let additional_content = read_additional_content(content, raw, primary_result.as_ref());
    let thought_value = thought.map(|t| Value::String(t.to_string()));
    let thought = normalize_result_value(thought_value.as_ref());

    let mut payload = Map::new();
    payload.insert("status".to_string(), Value::String(status.to_string()));
    if !is_empty_result_value(primary_result.as_ref()) {
        payload.insert(
            "result".to_string(),
            primary_result.clone().expect("非空已判"),
        );
    }
    if !is_empty_result_value(additional_content.as_ref()) {
        payload.insert(
            "content".to_string(),
            additional_content.clone().expect("非空已判"),
        );
    }
    if !is_empty_result_value(thought.as_ref())
        && !is_same_result_value(thought.as_ref(), primary_result.as_ref())
        && !is_same_result_value(thought.as_ref(), additional_content.as_ref())
    {
        payload.insert("thought".to_string(), thought.clone().expect("非空已判"));
    }
    // 真源 :130-132 —— `if (errorText)`：空串 falsy 不插入。
    if let Some(err) = error_text.filter(|e| !e.is_empty()) {
        payload.insert("error".to_string(), Value::String(err.to_string()));
    }

    Value::Object(payload)
}

/// `formatResultPayload`（真源 :139-148）：字符串原样；其余 `JSON.stringify(v, null, 2)`。
fn format_result_payload(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// `GoalToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct GoalBlockProps {
    pub tool_id: String,
    pub output: Value,
    /// `toolCall.content`（Agent/Task 行桥接的活动结果；真源 :92）。
    pub content: Option<String>,
    /// `toolCall.thought`（真源 :110）——v4 adapter 不产（真源同），恒 None。
    pub thought: Option<String>,
    pub raw: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `GoalToolCallBlock`（真源 :150-207）。
#[component]
pub fn GoalToolCallBlock(props: GoalBlockProps) -> impl IntoView {
    let status = props.status.clone().unwrap_or_default();
    let result_payload = build_goal_result_payload(
        &status,
        &props.output,
        props.content.as_deref(),
        props.thought.as_deref(),
        &props.raw,
        props.error_text.as_deref(),
    );
    let result_text = format_result_payload(&result_payload);
    let is_failed = props.status.as_deref() == Some("failed");
    // 真源 :155-157 —— title ?? "Goal"。
    let primary_text = props.title.clone().unwrap_or_else(|| "Goal".to_string());

    let refs = props.snapshot_refs.clone();
    let tool_id_for_notice = props.tool_id.clone();
    let callback = props.on_load_full_tool_call_fields.clone();
    let render_content = move || {
        view! {
            <div class="space-y-2">
                <h4 class="text-ui-base font-medium uppercase tracking-wide text-foreground-subtle">
                    "结果"
                </h4>
                <pre class="max-h-60 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-surface px-3 py-3 font-mono text-ui-base text-foreground">
                    {result_text.clone()}
                </pre>
            </div>
            <ToolSnapshotFieldNoticeComponent
                props=ToolSnapshotFieldNoticeProps {
                    refs: refs.clone(),
                    tool_id: tool_id_for_notice.clone(),
                    on_load_full_tool_call_fields: callback.clone(),
                }
            />
        }
        .into_any()
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(true),
                force_open: Some(false),
                kind_label: Some("Goal".to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                // 真源 :187-188 —— failed 时次文本让位给状态词（二选一）。
                secondary_text: if is_failed {
                    None
                } else {
                    props.status_label.clone()
                },
                status_label: is_failed.then(|| props.status_label.clone()).flatten(),
                status_tooltip: is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=GOAL_TOOL_ICON_CLASS>
                        // GoalIcon（lucide）：旗帜 + 双环。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 13V2l8 4-8 4"></path>
                            <path d="M20.561 10.222a9 9 0 1 1-12.55-5.29"></path>
                            <path d="M8.002 9.997a5 5 0 1 0 8.9 2.02"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content))
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_parses_json_keeps_raw_on_failure() {
        assert_eq!(
            normalize_result_value(Some(&json!("{\"a\":1}"))),
            Some(json!({"a": 1}))
        );
        // trim 空 → None。
        assert_eq!(normalize_result_value(Some(&json!("  "))), None);
        // 坏 JSON → 原字符串（未 trim）。
        assert_eq!(
            normalize_result_value(Some(&json!(" not json "))),
            Some(json!(" not json "))
        );
        // 非字符串原样。
        assert_eq!(normalize_result_value(Some(&json!(5))), Some(json!(5)));
        assert_eq!(normalize_result_value(None), None);
    }

    #[test]
    fn primary_result_fallback_chain() {
        // output 优先。
        assert_eq!(
            read_primary_result(&json!("输出"), &json!({"rawOutput": "raw"})),
            Some(json!("输出"))
        );
        // 空白 output → raw.result.content。
        assert_eq!(
            read_primary_result(&json!(" "), &json!({"result": {"content": "rc"}})),
            Some(json!("rc"))
        );
        // → raw.rawOutput。
        assert_eq!(
            read_primary_result(&Value::Null, &json!({"rawOutput": "ro"})),
            Some(json!("ro"))
        );
        // → raw.output。
        assert_eq!(
            read_primary_result(&Value::Null, &json!({"output": "rout"})),
            Some(json!("rout"))
        );
        assert_eq!(read_primary_result(&Value::Null, &json!({})), None);
    }

    #[test]
    fn payload_dedupes_thought_and_content() {
        let payload = build_goal_result_payload(
            "completed",
            &json!("主结果"),
            Some("主结果"),
            Some("主结果"),
            &json!({}),
            None,
        );
        // content 与 result 相同 → 不插；thought 与 result 相同 → 不插。
        assert_eq!(payload, json!({"status": "completed", "result": "主结果"}));

        let payload = build_goal_result_payload(
            "completed",
            &json!("主结果"),
            Some("附加"),
            Some("想法"),
            &json!({}),
            Some("出错了"),
        );
        assert_eq!(
            payload,
            json!({"status": "completed", "result": "主结果", "content": "附加", "thought": "想法", "error": "出错了"})
        );
        // 空串 error 不插入（真源 falsy）。
        let payload =
            build_goal_result_payload("completed", &Value::Null, None, None, &json!({}), Some(""));
        assert_eq!(payload, json!({"status": "completed"}));
    }

    #[test]
    fn payload_key_order_preserved() {
        // preserve_order：插入序 = status → result → error。
        let payload =
            build_goal_result_payload("failed", &json!("r"), None, None, &json!({}), Some("e"));
        let text = format_result_payload(&payload);
        let status_pos = text.find("\"status\"").expect("status");
        let result_pos = text.find("\"result\"").expect("result");
        let error_pos = text.find("\"error\"").expect("error");
        assert!(status_pos < result_pos && result_pos < error_pos);
        // 字符串值原样输出（不套 pretty）。
        assert_eq!(format_result_payload(&json!(" 裸文本 ")), " 裸文本 ");
    }
}
