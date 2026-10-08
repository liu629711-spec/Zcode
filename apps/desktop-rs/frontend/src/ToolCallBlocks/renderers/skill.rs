//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/skill.tsx`（224 行）。
//!
//! 技能卡：技能名单行摘要（mono）+ 参数次文本 + 展开详情（技能名徽章 /
//! 参数 pre / 输出 pre）。技能名从 input.skill/name 读；参数从
//! input 字符串或 args/arg/path/prompt/input 字段读。
//!
//! 文案（zh-CN.ts:4844/4946-4951）：kind.skill「技能」/ running「正在运行技能」/
//! label「技能」/ args「参数」/ unknown「未知技能」/ noOutput「没有输出。」。
//!
//! **已接线**：`ToolSnapshotFieldNoticeComponent`——refs 空或宿主未接回调时
//! 不渲染（真源 `return null` 语义；v4 投影下 snapshotRefs 无生产者，恒不显示）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

/// 真源 :9 —— WandSparkles 图标类名。
pub const SKILL_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// `readFirstStringField`（真源 :15-29）——返回 trim 后值。
fn read_first_string_field(record: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(candidate) = record.get(*key).and_then(|v| v.as_str()) {
            let trimmed = candidate.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// `getSkillName`（真源 :31-40）。
pub fn get_skill_name(input: &Value) -> Option<String> {
    if is_plain_record(input) {
        return read_first_string_field(input, &["skill", "name"]);
    }
    None
}

/// `getSkillArgs`（真源 :42-55）。
pub fn get_skill_args(input: &Value) -> Option<String> {
    if let Some(s) = input.as_str() {
        let trimmed = s.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if is_plain_record(input) {
        return read_first_string_field(input, &["args", "arg", "path", "prompt", "input"]);
    }
    None
}

/// `extractSkillText`（真源 :57-105）。
pub fn extract_skill_text(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(array) = value.as_array() {
        let text = array
            .iter()
            .filter_map(extract_skill_text)
            .collect::<Vec<_>>()
            .join("\n");
        return (!text.is_empty()).then_some(text);
    }
    if !is_plain_record(value) {
        return None;
    }
    if let Some(direct) = read_first_string_field(
        value,
        &[
            "output",
            "text",
            "content",
            "message",
            "rawOutput",
            "result",
        ],
    ) {
        return Some(direct);
    }
    if let Some(entries) = value.get("content").and_then(|c| c.as_array()) {
        let text = entries
            .iter()
            .map(|item| {
                if !is_plain_record(item) {
                    extract_skill_text(item)
                } else {
                    extract_skill_text(item.get("content").unwrap_or(item))
                }
            })
            .filter_map(|t| t)
            .collect::<Vec<_>>()
            .join("\n");
        return (!text.is_empty()).then_some(text);
    }
    None
}

/// 卡片模型（真源 :106-141 的取值）。
#[derive(Debug, Clone, PartialEq)]
pub struct SkillModel {
    pub skill_name: Option<String>,
    pub skill_args: Option<String>,
    /// 详情正文：失败时 error → output → rawOutput；否则 output → rawOutput。
    pub detail_text: Option<String>,
    pub is_failed: bool,
    pub kind_label: &'static str,
}

/// 组装模型。
pub fn build_skill_model(
    input: &Value,
    output: &Value,
    raw: &Value,
    status: Option<&str>,
    error_text: Option<&str>,
    is_running: bool,
) -> SkillModel {
    let skill_name = get_skill_name(input);
    let skill_args = get_skill_args(input);
    let is_failed = status == Some("failed");

    // 真源 :110-116 —— output（adapter 的 output.text 字符串）→ raw.rawOutput。
    let output_text = extract_skill_text(output);
    let raw_output_text = is_plain_record(raw)
        .then(|| raw.get("rawOutput").cloned())
        .flatten()
        .and_then(|v| extract_skill_text(&v));
    let detail_text = if is_failed {
        error_text
            .map(|e| e.to_string())
            .or(output_text)
            .or(raw_output_text)
    } else {
        output_text.or(raw_output_text)
    };

    let kind_label = if is_running {
        "正在运行技能"
    } else {
        "技能"
    };

    SkillModel {
        skill_name,
        skill_args,
        detail_text,
        is_failed,
        kind_label,
    }
}

/// `SkillToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct SkillBlockProps {
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
    pub status_label: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
}

/// `SkillToolCallBlock`（真源 :106-224）。
#[component]
pub fn SkillToolCallBlock(props: SkillBlockProps) -> impl IntoView {
    let model = build_skill_model(
        &props.input,
        &props.output,
        &props.raw,
        props.status.as_deref(),
        props.error_text.as_deref(),
        props.is_running,
    );
    // 主文本（真源 :118-124）：技能名 ?? title ?? 「技能」。
    let primary_text = model
        .skill_name
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "技能".to_string());
    let args_for_secondary = model.skill_args.clone();
    let skill_name_for_detail = model
        .skill_name
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "未知技能".to_string());
    let args_for_detail = model.skill_args.clone();
    let detail_text = model.detail_text.clone();
    let is_running = props.is_running;
    let status_tooltip = model.is_failed.then(|| model.detail_text.clone()).flatten();
    let render_content = move || {
        let skill_name = skill_name_for_detail.clone();
        let args = args_for_detail.clone();
        let detail = detail_text.clone();
        view! {
            <div class="mb-2 space-y-3 rounded-xl border border-border bg-panel px-4 py-3">
                <div class="space-y-1">
                    <div class="flex flex-wrap items-center gap-2 text-ui-base text-foreground">
                        <span class="text-foreground-subtle">"技能"</span>
                        <code class="rounded-md bg-surface px-2 py-1 font-mono text-ui-base text-foreground">
                            {skill_name}
                        </code>
                    </div>
                    {args.map(|args| view! {
                        <div class="flex items-start gap-2 font-mono text-ui-base text-foreground">
                            <span class="flex-none text-foreground-subtle">"参数"</span>
                            <pre class="min-w-0 flex-1 break-words whitespace-pre-wrap text-foreground-subtle">
                                {args}
                            </pre>
                        </div>
                    })}
                </div>
                {match detail {
                    Some(detail) => view! {
                        <div class="space-y-1">
                            <pre class="max-h-25 overflow-auto font-mono text-ui-base break-words whitespace-pre-wrap text-foreground-subtle">
                                {detail}
                            </pre>
                        </div>
                    }
                    .into_any(),
                    // 真源 :176-181 —— 结束后仍无输出显示「没有输出。」。
                    None => (!is_running)
                        .then(|| view! {
                            <div class="space-y-1">
                                <p class="font-mono text-ui-base text-foreground-subtle">
                                    "没有输出。"
                                </p>
                            </div>
                        })
                        .into_any(),
                }}
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
                can_toggle: Some(true),
                force_open: Some(false),
                hide_secondary_text_when_open: Some(true),
                kind_label: Some(model.kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                secondary_text: args_for_secondary.map(|args| format!("`{args}`")),
                status_label: props.status_label.clone(),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(model.is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=SKILL_TOOL_ICON_CLASS>
                        // WandSparkles（lucide）。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72"></path>
                            <path d="m14 7 3 3"></path>
                            <path d="M5 6v4"></path>
                            <path d="M19 14v4"></path>
                            <path d="M10 2v2"></path>
                            <path d="M7 8H3"></path>
                            <path d="M21 16h-4"></path>
                            <path d="M11 3H9"></path>
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
    fn name_and_args_from_input_shapes() {
        assert_eq!(
            get_skill_name(&json!({"skill": "commit"})).as_deref(),
            Some("commit")
        );
        assert_eq!(
            get_skill_name(&json!({"name": "review-pr"})).as_deref(),
            Some("review-pr")
        );
        // 字符串 input 即 args。
        assert_eq!(
            get_skill_args(&json!("--dry-run")).as_deref(),
            Some("--dry-run")
        );
        assert_eq!(
            get_skill_args(&json!({"args": "-m fix"})).as_deref(),
            Some("-m fix")
        );
    }

    #[test]
    fn extract_text_shapes() {
        // 字符串。
        assert_eq!(extract_skill_text(&json!("done")).as_deref(), Some("done"));
        // 数组 join。
        assert_eq!(
            extract_skill_text(&json!(["a", "b"])).as_deref(),
            Some("a\nb")
        );
        // 记录的 output 字段。
        assert_eq!(
            extract_skill_text(&json!({"output": "结果"})).as_deref(),
            Some("结果")
        );
        // content 数组嵌套。
        assert_eq!(
            extract_skill_text(&json!({"content": [{"content": {"text": "块"}}]})).as_deref(),
            Some("块")
        );
        // 空值。
        assert_eq!(extract_skill_text(&Value::Null), None);
        assert_eq!(extract_skill_text(&json!("  ")), None);
    }

    #[test]
    fn failed_uses_error_first() {
        let model = build_skill_model(
            &json!({"skill": "x"}),
            &json!("输出"),
            &json!({}),
            Some("failed"),
            Some("错误正文"),
            false,
        );
        assert!(model.is_failed);
        assert_eq!(model.detail_text.as_deref(), Some("错误正文"));
        // 无 error 用 output。
        let model = build_skill_model(
            &json!({}),
            &json!("输出"),
            &json!({}),
            Some("failed"),
            None,
            false,
        );
        assert_eq!(model.detail_text.as_deref(), Some("输出"));
    }

    #[test]
    fn raw_output_is_fallback_source() {
        // 真源 :112-116 —— output 空时读 raw.rawOutput。
        let model = build_skill_model(
            &json!({}),
            &Value::Null,
            &json!({"rawOutput": "raw 结果"}),
            None,
            None,
            false,
        );
        assert_eq!(model.detail_text.as_deref(), Some("raw 结果"));
    }
}
