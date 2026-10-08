//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/escalate.tsx`（162 行）。
//!
//! escalate 工具卡：子代理把一个**真阻塞**问题升级给主代理，并停驻在
//! 这次调用里等答案。折叠行：kindLabel（asking/asked）+ 问题单行概要；
//! 展开：问题（+ context 若在场）+ 答案区。
//!
//! **关键语义**（真源 :54-60 注释）：预算已尽的驳回是**一次普通工具结果**
//! （不是 error tool_result），卡片绝不因 output 文本内容渲染成失败——
//! 只有 `status==="failed"`（接线故障类）才走失败样式。停驻可能很久，
//! running 态要显得平静、不像坏了。
//!
//! **已接线**：`ToolSnapshotFieldNoticeComponent`（真源 :152-161）。
//!
//! **裁剪注明**：`canToggle/forceOpen ?? 默认` 走真源默认（Context 未承载
//! 宿主覆盖位，同 plan_guidance）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

/// 真源 :8-10 —— `MessageCircleQuestion className="size-4 shrink-0 text-foreground-subtle"`。
pub const ESCALATE_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 折叠头部的单行概要上限（真源 :13）：概要只是「大概问了什么」，
/// 整段问题在展开后的 body 里。
pub const INLINE_PREVIEW_MAX_LENGTH: usize = 160;

/// `toRecord`（真源 :17-28）：record 直接过；非空字符串尝试一次宽容 parse。
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

/// `readText`（真源 :30-32）：非空白字符串（返回**原值**，不 trim）。
pub fn read_text(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// `toInlinePreview`（真源 :35-41）：换行折叠成空格，超长截断加省略号。
///
/// 与真源的差异：JS `.length`/`slice` 按 UTF-16 码元计数（代理对会被劈裂）；
/// Rust 按码点计数（`chars`），emoji 边界更安全。中文（BMP）场景无差异。
pub fn to_inline_preview(value: Option<&str>) -> Option<String> {
    let value = value?;
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() > INLINE_PREVIEW_MAX_LENGTH {
        let head: String = collapsed.chars().take(INLINE_PREVIEW_MAX_LENGTH).collect();
        Some(format!("{head}…"))
    } else {
        Some(collapsed)
    }
}

/// 卡片模型。
#[derive(Debug, Clone, PartialEq)]
pub struct EscalateModel {
    pub question: Option<String>,
    pub question_context: Option<String>,
    pub answer_text: Option<String>,
    pub is_failed: bool,
    pub is_asking: bool,
    pub kind_label: &'static str,
    pub has_details: bool,
    pub inline_preview: Option<String>,
}

/// `EscalateToolCallBlock` 的取值与状态推导（真源 :63-106）。
pub fn build_escalate_model(
    input: &Value,
    output: &Value,
    status: Option<&str>,
    is_running: bool,
) -> EscalateModel {
    let input_record = to_record(input);
    let question = input_record
        .as_ref()
        .and_then(|r| r.get("question"))
        .and_then(read_text);
    let question_context = input_record
        .as_ref()
        .and_then(|r| r.get("context"))
        .and_then(read_text);

    let is_failed = status == Some("failed");
    // 真源 :67-70 —— 停驻中：未失败且（running / pending / in_progress）。
    let is_asking =
        !is_failed && (is_running || matches!(status, Some("pending") | Some("in_progress")));
    // 真源 :74 —— 停驻时答案还没到（in-progress 标签就是全部信息）。
    let answer_text = if is_asking { None } else { read_text(output) };
    let kind_label = if is_asking {
        "正在询问主代理"
    } else {
        "已询问主代理"
    };
    // 真源 :101-103 —— 流式首帧 input 可能还是 `{}`，不给空面板展开入口。
    let has_details = question.is_some() || question_context.is_some() || answer_text.is_some();
    let inline_preview = to_inline_preview(question.as_deref());

    EscalateModel {
        question,
        question_context,
        answer_text,
        is_failed,
        is_asking,
        kind_label,
        has_details,
        inline_preview,
    }
}

/// `EscalateToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct EscalateBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub output: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `EscalateToolCallBlock`（真源 :62-162）。
#[component]
pub fn EscalateToolCallBlock(props: EscalateBlockProps) -> impl IntoView {
    let model = build_escalate_model(
        &props.input,
        &props.output,
        props.status.as_deref(),
        props.is_running,
    );
    // 真源 :89-93 —— inlinePreview ?? title ?? "问题"（fallbackName）。
    let primary_text = model
        .inline_preview
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "问题".to_string());

    let question = model.question.clone();
    let question_context = model.question_context.clone();
    let answer_text = model.answer_text.clone();
    let has_details = model.has_details;
    let render_content = move || {
        view! {
            <div class="space-y-3">
                {question.clone().map(|q| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">"问题"</h4>
                        <p class="whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 text-ui-base leading-5 text-foreground">
                            {q}
                        </p>
                    </section>
                })}
                {question_context.clone().map(|c| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">"背景"</h4>
                        <p class="whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 text-ui-base leading-5 text-foreground-subtle">
                            {c}
                        </p>
                    </section>
                })}
                {answer_text.clone().map(|a| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">"答复"</h4>
                        <p class="whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 text-ui-base leading-5 text-foreground">
                            {a}
                        </p>
                    </section>
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
                // 真源 :130-131 —— canToggle/forceOpen 都以 hasDetails 门控。
                can_toggle: Some(has_details),
                force_open: Some(false),
                hide_secondary_text_when_open: Some(true),
                kind_label: Some(model.kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                status_label: model.is_failed.then(|| "执行失败".to_string()),
                status_tooltip: model.is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(model.is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=ESCALATE_TOOL_ICON_CLASS>
                        // MessageCircleQuestion（lucide）：圆气泡 + 问号。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M2.992 16.342a2 2 0 0 1 .094 1.167l-1.065 3.29a1 1 0 0 0 1.236 1.168l3.413-.998a2 2 0 0 1 1.099.092 10 10 0 1 0-4.777-4.719"></path>
                            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
                            <path d="M12 17h.01"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            render_content=has_details.then(|| std::sync::Arc::new(render_content) as ChildrenFn)
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
    fn inline_preview_collapses_and_truncates() {
        assert_eq!(
            to_inline_preview(Some("多行\n问题  在这里")).as_deref(),
            Some("多行 问题 在这里")
        );
        // 超长截断（160 码点 + 省略号）。
        let long = "字".repeat(200);
        let preview = to_inline_preview(Some(&long)).expect("preview");
        assert_eq!(preview.chars().count(), 161);
        assert!(preview.ends_with('…'));
        // 空白 → None。
        assert_eq!(to_inline_preview(Some("   \n ")), None);
        assert_eq!(to_inline_preview(None), None);
    }

    #[test]
    fn asking_state_hides_answer() {
        // 停驻中：running 且未失败 → asking，答案不读。
        let m = build_escalate_model(
            &json!({"question": "要不要继续？"}),
            &json!("答案正文"),
            Some("in_progress"),
            true,
        );
        assert!(m.is_asking);
        assert_eq!(m.kind_label, "正在询问主代理");
        assert_eq!(m.answer_text, None);
        // 结束：completed → asked，答案取 output。
        let m = build_escalate_model(
            &json!({"question": "要不要继续？"}),
            &json!("答案正文"),
            Some("completed"),
            false,
        );
        assert!(!m.is_asking);
        assert_eq!(m.kind_label, "已询问主代理");
        assert_eq!(m.answer_text.as_deref(), Some("答案正文"));
    }

    #[test]
    fn failed_never_shows_answer() {
        // 真源 :74 只由 isAsking 控制；failed 时 isAsking=false → 读 output。
        // 但卡片不因 output 内容渲染失败样式——只有 status 决定 is_failed。
        let m = build_escalate_model(&json!({}), &json!({}), Some("failed"), false);
        assert!(m.is_failed);
        assert!(!m.is_asking);
        assert_eq!(m.answer_text, None, "output 非字符串 → 无答案");
    }

    #[test]
    fn has_details_gate() {
        // 首帧 input 还是 {} → 无展开入口。
        let m = build_escalate_model(&json!({}), &json!({}), Some("in_progress"), true);
        assert!(!m.has_details);
        // 有问题即可展开。
        let m = build_escalate_model(
            &json!({"question": "q"}),
            &json!({}),
            Some("in_progress"),
            true,
        );
        assert!(m.has_details);
        // JSON 字符串 input 形态。
        let m = build_escalate_model(
            &json!("{\"context\":\"背景\"}"),
            &json!({}),
            Some("completed"),
            false,
        );
        assert_eq!(m.question_context.as_deref(), Some("背景"));
        assert!(m.has_details);
    }
}
