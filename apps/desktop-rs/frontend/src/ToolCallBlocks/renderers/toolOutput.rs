//! 1:1 翻译 `packages/ui/src/components/ai-elements/tool.tsx` 的
//! `ToolOutput` 组件（:128-170）。
//!
//! 通用输出回退块：Error/Result 小标 + 蒙层盒子。switch_mode 等卡的
//! 「没有 markdown 正文」时的最小呈现。

use leptos::prelude::*;
use serde_json::Value;

/// `ToolOutput`（真源 :133-170）。
///
/// 真源对对象输出用 CodeBlock（JSON pretty）；Rust 侧用 pre 文本等价呈现。
#[component]
pub fn ToolOutputBlock(
    /// `ToolPart["output"]`——散文 text 或任意 JSON。
    output: Option<Value>,
    error_text: Option<String>,
) -> impl IntoView {
    // 真源 :134-136 —— 两者皆空不渲染。
    let has_error = error_text.as_deref().is_some_and(|e| !e.is_empty());
    let output_text = output
        .as_ref()
        .map(render_output_text)
        .filter(|t| !t.is_empty());
    if !has_error && output_text.is_none() {
        return ().into_any();
    }

    let label = if has_error { "Error" } else { "Result" };
    let body = if has_error {
        error_text.clone().unwrap_or_default()
    } else {
        output_text.unwrap_or_default()
    };
    let body_class = if has_error {
        "max-h-60 overflow-auto rounded-md bg-destructive/10 p-3 font-mono text-ui-base whitespace-pre-wrap text-destructive"
    } else {
        "max-h-60 overflow-auto rounded-md bg-muted/50 p-3 font-mono text-ui-base whitespace-pre-wrap text-foreground"
    };
    view! {
        <div class="space-y-2">
            <h4 class="font-medium tracking-wide text-muted-foreground text-ui-base uppercase">
                {label}
            </h4>
            // 真源注释（:161-163）：失败态只保留可读错误文本，不再渲染
            // JSON result——避免「报错被参数淹没」。
            <div class=body_class>{body}</div>
        </div>
    }
    .into_any()
}

/// 输出 → 展示文本（真源 :142-146：对象/数组走 JSON pretty 的 CodeBlock）。
fn render_output_text(output: &Value) -> String {
    match output {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_output_passes_through() {
        assert_eq!(render_output_text(&json!("done")), "done");
    }

    #[test]
    fn object_output_is_pretty_json() {
        let text = render_output_text(&json!({"a": 1}));
        assert!(text.contains("\"a\": 1"));
    }

    #[test]
    fn null_output_is_empty() {
        assert_eq!(render_output_text(&Value::Null), "");
    }
}
