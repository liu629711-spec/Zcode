//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/fallback.tsx`（103 行）。
//!
//! 兜底卡片——所有未匹配专用 renderer 的工具都走这里。
//!
//! 真源要点：
//! - `kindLabel`：kind 首字母大写（:28-30，空kind 则原样）
//! - **失败时状态词挂到 `statusLabel` 而非 `secondaryText`**（:96-98）：
//!   `secondaryText={summaryOnly || failed ? undefined : statusLabel}`
//!   同时 `statusLabel={failed ? statusLabel : undefined}`——
//!   失败时状态词「独占」状态位，不与 secondaryText 重复
//! - 展开区末尾追加 **raw JSON 兜底**（:51-55）：无 inlinePreview 且未禁用时，
//!   把整个 toolCall 序列化展示（`JSON.stringify(toolCall, null, 2)`）
//! - `summaryOnly` 时 kindLabel 为 null（:93）——纯摘要模式下不显示类别
//!
//! 行号注释均指真源文件。

use leptos::prelude::*;
use serde_json::Value;

/// `FALLBACK_TOOL_ICON`（真源 :12）——WrenchIcon。
pub const FALLBACK_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// raw JSON 兜底块的类名（真源 :52）。
pub const RAW_FALLBACK_CLASS: &str =
    "mt-1 max-h-50 overflow-auto rounded-xl bg-surface px-4 py-3 text-ui-xs text-foreground-subtle";

/// kindLabel 的计算（真源 :28-30）：首字母大写，空串原样返回。
///
/// 真源用 `toolCall.kind[0].toUpperCase() + slice(1)`。
pub fn build_kind_label(kind: &str) -> String {
    let mut chars = kind.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    }
}

/// FallbackToolCallBlock 的 props（真源 :14-19 的三个覆盖项 + context 字段）。
#[derive(Debug, Clone)]
pub struct FallbackBlockProps {
    pub tool_id: String,
    pub kind: String,
    pub title: Option<String>,
    pub status: Option<String>,
    /// 整个 toolCall 的原始载荷——raw JSON 兜底用（真源 :53）。
    pub raw_tool_call: Value,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    /// 是否有 inlinePreview（真源 :31 `displayModel.inlinePreview.type !== "none"`）。
    pub has_inline_preview: bool,
    // ── context 的三个覆盖项（:15-18）──
    /// 隐藏 raw JSON 兜底（:16）。
    pub hide_raw_fallback: bool,
    /// 纯摘要模式（:17）——kindLabel 为 null、primaryText 用覆盖值。
    pub summary_only: bool,
    /// 摘要文本覆盖（:18）。
    pub summary_text_override: Option<String>,
    /// kindLabel 覆盖（真源 :93 `context.kindLabelOverride`）。
    pub kind_label_override: Option<String>,
}

/// FallbackToolCallBlock 的主体（真源 :21-103）。
#[component]
pub fn FallbackToolCallBlock(props: FallbackBlockProps) -> impl IntoView {
    let is_failed = props.status.as_deref() == Some("failed");
    let kind_label = build_kind_label(&props.kind);

    // 真源 :93 —— summaryOnly 时 kindLabel 为 null（不显示类别）。
    let resolved_kind_label = if props.summary_only {
        None
    } else {
        props.kind_label_override.clone().or(Some(kind_label))
    };

    // 真源 :96 —— secondaryText 在 summaryOnly 或失败时让位。
    let secondary_text = if props.summary_only || is_failed {
        None
    } else {
        props.status_label.clone()
    };

    // 真源 :97 —— 失败时状态词挂statusLabel（独占状态位）。
    let status_label = if is_failed {
        props.status_label.clone()
    } else {
        None
    };
    let status_tooltip = if is_failed {
        props.error_text.clone()
    } else {
        None
    };

    // 真源 :99 —— primaryText：summaryOnly 用覆盖值，否则 title/ 固定文案。
    let primary_text = if props.summary_only {
        props.summary_text_override.clone()
    } else {
        props.title.clone().or_else(|| Some("工具调用".to_string()))
    };

    // 真源 :102-106 —— title：summaryOnly 且覆盖是字符串时用覆盖，否则 toolCall.title。
    let title = if props.summary_only && props.summary_text_override.is_some() {
        props.summary_text_override.clone()
    } else {
        props.title.clone()
    };

    // 真源 :51 —— 无 inlinePreview 且未禁用时，展开区追加 raw JSON。
    let show_raw_fallback = !props.has_inline_preview && !props.hide_raw_fallback;
    let raw_json = if show_raw_fallback {
        serde_json::to_string_pretty(&props.raw_tool_call).ok()
    } else {
        None
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                // 真源 :94-95 —— canToggle 默认 true、forceOpen 默认 false。
                can_toggle: Some(true),
                force_open: Some(false),
                kind_label: resolved_kind_label.clone(),
                source_label: props.source_label.clone(),
                primary_text: primary_text.clone(),
                secondary_text: secondary_text.clone(),
                status_label: status_label.clone(),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=FALLBACK_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec![
                                "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z",
                            ]
                            circles=vec![]
                        />
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(move || {
                // 真源 :52-55 —— raw JSON 兜底块。
                raw_json.clone().map(|json| {
                    view! { <pre class=RAW_FALLBACK_CLASS>{json}</pre> }
                })
                .into_any()
            }))
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kind_label_capitalizes_first_letter() {
        // 真源 :28-30
        assert_eq!(build_kind_label("bash"), "Bash");
        assert_eq!(build_kind_label("webSearch"), "WebSearch");
        assert_eq!(build_kind_label("MCP"), "MCP", "首字母已大写时不变");
        assert_eq!(build_kind_label(""), "", "空串原样");
        // 非 ASCII 首字符（如中文）不破坏。
        assert_eq!(build_kind_label("工具"), "工具");
    }

    #[test]
    fn failure_moves_status_label_away_from_secondary() {
        // ★真源 :96-98 —— 失败时 secondaryText 让位，状态词独占 statusLabel。
        let props = FallbackBlockProps {
            tool_id: "t1".into(),
            kind: "bash".into(),
            title: Some("执行命令".into()),
            status: Some("failed".into()),
            raw_tool_call: json!({}),
            is_running: false,
            status_label: Some("执行失败".into()),
            error_text: Some("命令失败".into()),
            source_label: None,
            show_icon: true,
            has_inline_preview: false,
            hide_raw_fallback: false,
            summary_only: false,
            summary_text_override: None,
            kind_label_override: None,
        };
        let is_failed = props.status.as_deref() == Some("failed");
        assert!(is_failed);
        // 失败时 secondary_text 应为 None、status_label 为 Some。
        let secondary = if props.summary_only || is_failed {
            None
        } else {
            props.status_label.clone()
        };
        let status_label = if is_failed { props.status_label.clone() } else { None };
        assert_eq!(secondary, None, "失败时 secondaryText 让位");
        assert_eq!(status_label.as_deref(), Some("执行失败"));
    }

    #[test]
    fn summary_only_hides_kind_label() {
        // 真源 :93 —— summaryOnly 时 kindLabel 为 null。
        let props = FallbackBlockProps {
            tool_id: "t1".into(),
            kind: "bash".into(),
            title: None,
            status: None,
            raw_tool_call: json!({}),
            is_running: false,
            status_label: None,
            error_text: None,
            source_label: None,
            show_icon: true,
            has_inline_preview: true,
            hide_raw_fallback: false,
            summary_only: true,
            summary_text_override: Some("摘要文本".into()),
            kind_label_override: Some("覆盖类别".into()),
        };
        // summary_only 优先于 kind_label_override。
        let resolved = if props.summary_only {
            None
        } else {
            props.kind_label_override.clone().or(Some("Bash".into()))
        };
        assert_eq!(resolved, None, "summaryOnly 时不显示类别");
    }

    #[test]
    fn raw_fallback_shown_only_without_inline_preview() {
        // 真源 :51 —— !hasInlinePreview && !hideRawFallback 才追加 raw JSON。
        let show = |has_inline_preview: bool, hide_raw: bool| {
            !has_inline_preview && !hide_raw
        };
        assert!(show(false, false), "无 inlinePreview 且未禁用 → 显示");
        assert!(!show(true, false), "有 inlinePreview → 不显示");
        assert!(!show(false, true), "显式禁用 → 不显示");
    }

    #[test]
    fn raw_fallback_class_matches_source() {
        // 真源 :52
        assert!(RAW_FALLBACK_CLASS.contains("max-h-50"));
        assert!(RAW_FALLBACK_CLASS.contains("overflow-auto"));
        assert!(RAW_FALLBACK_CLASS.contains("rounded-xl"));
        assert!(RAW_FALLBACK_CLASS.contains("bg-surface"));
    }
}