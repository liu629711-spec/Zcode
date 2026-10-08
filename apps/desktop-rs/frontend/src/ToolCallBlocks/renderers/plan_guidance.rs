//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/plan-guidance.tsx`
//! （143 行）。
//!
//! PlanGuidance 卡：进入计划模式的引导正文（markdown）或 `ToolOutput` 回退。
//!
//! **已接线**：`ToolSnapshotFieldNotice`（真源 :136-145）。
//!
//! **裁剪注明**：`canToggle ?? true` / `forceOpen ?? false` 走真源默认——
//! v1 `ToolCallBlockContext` 未承载这两个宿主覆盖位。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::toolOutput::ToolOutputBlock;

/// 真源 :12 —— `NotepadText className="size-4 shrink-0 text-foreground-subtle"`。
pub const PLAN_GUIDANCE_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `readStringField`（真源 :22-34）：按序取非空白字符串并 trim。
fn read_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(candidate) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = candidate.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// `extractGuidanceMarkdown`（真源 :36-81）：三层回落。
///
/// 1. `output` / `input`：字符串（trim 非空）或记录的 text/content/output；
/// 2. `raw.rawOutput`：字符串（trim 非空）或记录的 text/content/output；
/// 3. `raw.content` 数组：逐项取 `item.content`（记录时）或 item 自身的
///    text/content/output。
pub fn extract_guidance_markdown(output: &Value, input: &Value, raw: &Value) -> Option<String> {
    for candidate in [output, input] {
        if let Some(s) = candidate.as_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        if candidate.is_object() {
            if let Some(text) = read_string_field(candidate, &["text", "content", "output"]) {
                return Some(text);
            }
        }
    }

    if !raw.is_object() {
        return None;
    }

    if let Some(raw_output) = raw.get("rawOutput") {
        if let Some(s) = raw_output.as_str() {
            // 真源 :57-59 —— 字符串分支直接 trim；空串 falsy 继续下探。
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        } else if raw_output.is_object() {
            if let Some(text) = read_string_field(raw_output, &["text", "content", "output"]) {
                return Some(text);
            }
        }
    }

    let content = raw
        .get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in &content {
        if !item.is_object() {
            continue;
        }
        // 真源 :72-77 —— item.content 为记录时下探一层，否则用 item 自身。
        let nested = item
            .get("content")
            .filter(|v| v.is_object())
            .unwrap_or(item);
        if let Some(text) = read_string_field(nested, &["text", "content", "output"]) {
            return Some(text);
        }
    }

    None
}

/// MessageResponse 合并后的类名（真源 base `size-full text-ui-base
/// leading-[1.75] tracking-wide [&>*:first-child]:mt-0
/// [&>*:last-child]:mb-0` + className 的 subtlest 覆盖）。
const PLAN_GUIDANCE_MARKDOWN_CLASS: &str = "size-full text-ui-base leading-[1.75] tracking-wide [&>*:first-child]:mt-0 [&>*:last-child]:mb-0 min-w-0 break-words [&_h1]:text-foreground-subtlest [&_h2]:text-foreground-subtlest [&_h3]:text-foreground-subtlest [&_li]:text-foreground-subtlest [&_p]:text-foreground-subtlest";

/// `PlanGuidanceToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct PlanGuidanceBlockProps {
    pub tool_id: String,
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
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `PlanGuidanceToolCallBlock`（真源 :83-143）。
#[component]
pub fn PlanGuidanceToolCallBlock(props: PlanGuidanceBlockProps) -> impl IntoView {
    let guidance_markdown = extract_guidance_markdown(&props.output, &props.input, &props.raw);
    let is_failed = props.status.as_deref() == Some("failed");
    let error_for_output = props.error_text.clone();
    let output_for_fallback = props.output.clone();

    let render_content = move || {
        if let Some(markdown) = guidance_markdown.clone() {
            let html = crate::app::render_markdown(&markdown);
            // 真源 :91-92 —— 左竖线容器（真源 className 含两次 border-border）。
            view! {
                <div class="ml-2 space-y-2 border-border border-l pl-3.5 border-border">
                    <div class=PLAN_GUIDANCE_MARKDOWN_CLASS inner_html=html></div>
                </div>
            }
            .into_any()
        } else {
            // 真源 :100 —— 失败正文进 errorText 时不再渲染 output。
            let output = if error_for_output.is_some() {
                None
            } else {
                Some(output_for_fallback.clone())
            };
            view! {
                <ToolOutputBlock output=output error_text=error_for_output.clone() />
            }
            .into_any()
        }
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(true),
                force_open: Some(false),
                kind_label: Some("已开启 Plan Mode".to_string()),
                source_label: props.source_label.clone(),
                primary_text: None,
                secondary_text: None,
                status_label: is_failed.then(|| props.status_label.clone()).flatten(),
                status_tooltip: is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=PLAN_GUIDANCE_TOOL_ICON_CLASS>
                        // NotepadText（lucide）：便签 + 三行文字。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M8 2v4"></path>
                            <path d="M12 2v4"></path>
                            <path d="M16 2v4"></path>
                            <rect width="16" height="18" x="4" y="4" rx="2"></rect>
                            <path d="M8 10h6"></path>
                            <path d="M8 14h8"></path>
                            <path d="M8 18h5"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content))
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
    fn output_string_wins_over_input() {
        assert_eq!(
            extract_guidance_markdown(&json!(" 正文 "), &json!("input 正文"), &json!({}))
                .as_deref(),
            Some("正文")
        );
        // output 空字符串 → 落到 input。
        assert_eq!(
            extract_guidance_markdown(&json!(""), &json!("input 正文"), &json!({})).as_deref(),
            Some("input 正文")
        );
    }

    #[test]
    fn record_fields_by_priority() {
        assert_eq!(
            extract_guidance_markdown(
                &json!({"content": "从 content", "text": "从 text"}),
                &Value::Null,
                &json!({})
            )
            .as_deref(),
            Some("从 text")
        );
        // text 空白 → content。
        assert_eq!(
            extract_guidance_markdown(
                &json!({"text": "  ", "output": "从 output"}),
                &Value::Null,
                &json!({})
            )
            .as_deref(),
            Some("从 output")
        );
    }

    #[test]
    fn raw_output_then_content_array() {
        // output/input 皆无 → raw.rawOutput。
        assert_eq!(
            extract_guidance_markdown(
                &Value::Null,
                &Value::Null,
                &json!({"rawOutput": {"text": "raw 正文"}})
            )
            .as_deref(),
            Some("raw 正文")
        );
        // rawOutput 空串（不短路）→ raw.content 数组。
        assert_eq!(
            extract_guidance_markdown(
                &Value::Null,
                &Value::Null,
                &json!({"rawOutput": "  ", "content": [{"content": {"text": "块 1"}}]})
            )
            .as_deref(),
            Some("块 1")
        );
        // item 自身字段（无嵌套 content）。
        assert_eq!(
            extract_guidance_markdown(
                &Value::Null,
                &Value::Null,
                &json!({"content": [{"output": "块 2"}]})
            )
            .as_deref(),
            Some("块 2")
        );
    }

    #[test]
    fn no_source_returns_none() {
        assert_eq!(
            extract_guidance_markdown(&Value::Null, &json!({}), &json!({"content": []})),
            None
        );
        // raw 非 record → 直接 None。
        assert_eq!(
            extract_guidance_markdown(&Value::Null, &Value::Null, &json!("str")),
            None
        );
    }
}
