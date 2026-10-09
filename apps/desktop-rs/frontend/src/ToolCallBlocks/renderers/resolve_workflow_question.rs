//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/resolve-workflow-question.tsx`（173 行）。
//!
//! ResolveWorkflowQuestion 工具卡（真源 :47-56）：主代理按 qid 回答一个子代理从**正在跑的**
//! workflow 里升级上来的阻塞问题；答案会成为那次 escalate 调用的结果原文。
//!
//! 折叠行：kindLabel（answering / answered）+ 答案单行概要。展开：question_id（mono qid 行）
//! + 完整答案 + 工具输出文本（成功确认，或三种结构化拒绝之一）。
//!
//! 关键（真源 :54-55）：拒绝（未知 qid / 已作答 / run 不在飞 / 本会话无应答能力）走的是
//! **结构化失败**，会带上 `status==="failed"`，此时才走失败样式；成功确认是普通结果。
//!
//! **裁剪注明**：`canToggle/forceOpen ?? 默认` 走真源默认（Context 未承载宿主覆盖位，
//! 同 escalate / plan_guidance）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::i18n;
use crate::components::workflowIcons::icon_message_circle_reply;

/// 真源 :8-10 —— `MessageCircleReply className="size-4 shrink-0 text-foreground-subtle"`。
/// `shrink-0` 在 Tailwind 侧与 `flex-none` 同效，沿用本仓库既有写法（见 escalate.rs:25）。
pub const RESOLVE_QUESTION_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 折叠头部的单行概要上限（真源 :13）：概要只是「大概答了什么」，整段答案在展开后的 body 里。
pub const INLINE_PREVIEW_MAX_LENGTH: usize = 160;

/// `toRecord`（真源 :16-31）：record 直接过；非空字符串试**一次**宽容 `JSON.parse`
/// （照 submit-result 的读侧惯例），解析结果仍要非数组对象才收。
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

/// `readText`（真源 :33-35）：非空白字符串（返回**原值**，不 trim）。
pub fn read_text(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// 同一个 `readText`，只是输入已经是 `Option<String>`（legacy 行的 error / output 字段）。
fn read_text_opt(value: Option<&str>) -> Option<String> {
    let s = value?;
    read_text(&Value::String(s.to_string()))
}

/// `toInlinePreview`（真源 :38-45）：换行折叠成空格，超长截断加省略号。
///
/// 与真源的差异同 escalate.rs:59-60：JS 按 UTF-16 码元计数，Rust 按码点计数；中文无差异。
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

/// 卡片模型（真源 :60-100 的取值与状态推导）。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolveQuestionModel {
    pub question_id: Option<String>,
    pub answer: Option<String>,
    pub outcome_text: Option<String>,
    pub is_failed: bool,
    pub is_answering: bool,
    pub kind_label_id: &'static str,
    pub has_details: bool,
    pub inline_preview: Option<String>,
}

/// 真源 :60-100。`error` 是 legacy 行的失败正文字段，`error_text` 是 Context 的宿主覆盖位。
pub fn build_resolve_question_model(
    input: &Value,
    output: &Value,
    error: Option<&str>,
    error_text: Option<&str>,
    status: Option<&str>,
    is_running: bool,
) -> ResolveQuestionModel {
    let input_record = to_record(input);
    let question_id = input_record
        .as_ref()
        .and_then(|r| r.get("question_id"))
        .and_then(read_text);
    let answer = input_record
        .as_ref()
        .and_then(|r| r.get("answer"))
        .and_then(read_text);

    let is_failed = status == Some("failed");
    // 真源 :66-68 —— 未失败且（running / pending / in_progress）即在答。
    let is_answering =
        !is_failed && (is_running || matches!(status, Some("pending") | Some("in_progress")));
    let kind_label_id = if is_answering {
        "chat.toolCall.workflow.resolveQuestion.answering"
    } else {
        "chat.toolCall.workflow.resolveQuestion.answered"
    };

    // 真源 :86-90 —— 结果文本：失败走错误通道优先（errorText ?? error ?? output），
    // 成功确认是普通结果（output）；仍在答时没有结果。
    let outcome_text = if is_failed {
        error_text
            .map(str::to_string)
            .or_else(|| read_text_opt(error))
            .or_else(|| read_text_opt(output.as_str()))
    } else if is_answering {
        None
    } else {
        read_text_opt(output.as_str())
    };

    let inline_preview = to_inline_preview(answer.as_deref());
    // 真源 :100 —— 三个字段任一在场才有展开入口。
    let has_details =
        question_id.is_some() || answer.is_some() || outcome_text.is_some();

    ResolveQuestionModel {
        question_id,
        answer,
        outcome_text,
        is_failed,
        is_answering,
        kind_label_id,
        has_details,
        inline_preview,
    }
}

/// `ResolveWorkflowQuestionToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct ResolveQuestionBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub output: Value,
    /// legacy 行的失败正文（真源 `toolCall.error`）。
    pub error: Option<String>,
    pub status: Option<String>,
    pub is_running: bool,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `ResolveWorkflowQuestionToolCallBlock`（真源 :57-172）。
#[component]
pub fn ResolveWorkflowQuestionToolCallBlock(props: ResolveQuestionBlockProps) -> impl IntoView {
    let model = build_resolve_question_model(
        &props.input,
        &props.output,
        props.error.as_deref(),
        props.error_text.as_deref(),
        props.status.as_deref(),
        props.is_running,
    );
    // 真源 :95 —— inlinePreview ?? toolCall.title ?? fallbackName。
    let primary_text = model
        .inline_preview
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| i18n::text("chat.toolCall.workflow.resolveQuestion.fallbackName"));

    let question_id = model.question_id.clone();
    let answer = model.answer.clone();
    let outcome_text = model.outcome_text.clone();
    let is_failed = model.is_failed;
    let has_details = model.has_details;
    let render_content = move || {
        // 真源 :126-130 —— 失败段落换描边色与文字色，其余同常规段。
        let outcome_class = if is_failed {
            "whitespace-pre-wrap break-words rounded-lg border border-destructive/40 bg-panel px-4 py-3 text-ui-base leading-5 text-foreground"
        } else {
            "whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 text-ui-base leading-5 text-foreground-subtle"
        };
        view! {
            <div class="space-y-3">
                {question_id.clone().map(|qid| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                            {i18n::text("chat.toolCall.workflow.resolveQuestion.questionId")}
                        </h4>
                        // qid 是不透明标识键 → mono（真源 :108）。
                        <code class="block break-all rounded-lg border border-border bg-panel px-4 py-2 font-mono text-ui-sm text-foreground-subtle">
                            {qid}
                        </code>
                    </section>
                })}
                {answer.clone().map(|a| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                            {i18n::text("chat.toolCall.workflow.resolveQuestion.answer")}
                        </h4>
                        <p class="whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 text-ui-base leading-5 text-foreground">
                            {a}
                        </p>
                    </section>
                })}
                {outcome_text.clone().map(|o| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                            {i18n::text("chat.toolCall.workflow.resolveQuestion.outcome")}
                        </h4>
                        <p class=outcome_class>{o}</p>
                    </section>
                })}
            </div>
        }
        .into_any()
    };

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                // 真源 :147-148 —— canToggle/forceOpen 都以 hasDetails 门控。
                can_toggle: Some(has_details),
                force_open: Some(false),
                hide_secondary_text_when_open: Some(true),
                kind_label: Some(i18n::text(model.kind_label_id)),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                // 真源 :153-157 —— 失败才挂状态标签与 tooltip。
                status_label: model
                    .is_failed
                    .then(|| i18n::text("chat.toolCall.status.failed")),
                status_tooltip: model.is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(model.is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=RESOLVE_QUESTION_TOOL_ICON_CLASS>{icon_message_circle_reply()}</span>
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

    fn build(input: Value, output: Value, status: Option<&str>, is_running: bool) -> ResolveQuestionModel {
        build_resolve_question_model(&input, &output, None, None, status, is_running)
    }

    #[test]
    fn reads_qid_and_answer_from_input_record() {
        let m = build(
            json!({"question_id": "q-7", "answer": "用左连接"}),
            json!(""),
            Some("completed"),
            false,
        );
        assert_eq!(m.question_id.as_deref(), Some("q-7"));
        assert_eq!(m.answer.as_deref(), Some("用左连接"));
        assert!(!m.is_answering);
        assert_eq!(m.kind_label_id, "chat.toolCall.workflow.resolveQuestion.answered");
        assert!(m.has_details);
    }

    #[test]
    fn blank_strings_are_absent_not_empty() {
        // readText（真源 :33-35）—— 纯空白不算值，缺省即整段省略。
        let m = build(
            json!({"question_id": "   ", "answer": ""}),
            json!(""),
            Some("completed"),
            false,
        );
        assert_eq!(m.question_id, None);
        assert_eq!(m.answer, None);
        // ★三个字段都缺 → 没有展开入口（真源 :100）。
        assert!(!m.has_details);
    }

    #[test]
    fn string_input_is_parsed_once() {
        // 真源 :15-31 的宽容 parse：字符串形态的 input 试一次 JSON.parse。
        let m = build(
            json!("{\"question_id\":\"q-1\",\"answer\":\"按 B 走\"}"),
            json!("ok"),
            Some("completed"),
            false,
        );
        assert_eq!(m.question_id.as_deref(), Some("q-1"));
        assert_eq!(m.answer.as_deref(), Some("按 B 走"));
        assert_eq!(m.outcome_text.as_deref(), Some("ok"));
        // 解析不出对象 → 整份 input 作废，不抛错。
        let broken = build(json!("not json at all"), json!(""), Some("completed"), false);
        assert_eq!(broken.question_id, None);
        assert_eq!(broken.answer, None);
        // 数组不是 record（真源 :25 的 !Array.isArray）。
        let arr = build(json!("[{\"answer\":\"x\"}]"), json!(""), Some("completed"), false);
        assert_eq!(arr.answer, None);
    }

    #[test]
    fn answering_state_has_no_outcome() {
        // 真源 :88-89 —— 仍在答时结果文本是 undefined。
        for (status, running) in [("in_progress", false), ("pending", false), ("completed", true)] {
            let m = build(
                json!({"answer": "答案"}),
                json!("已确认"),
                Some(status),
                running,
            );
            assert!(m.is_answering, "{status}/running={running} 应判为正在回答");
            assert_eq!(m.outcome_text, None);
            assert_eq!(
                m.kind_label_id,
                "chat.toolCall.workflow.resolveQuestion.answering"
            );
        }
    }

    #[test]
    fn failed_reads_error_channel_in_priority_order() {
        // 真源 :87 —— errorText ?? toolCall.error ?? toolCall.output。
        let with_context = build_resolve_question_model(
            &json!({"question_id": "q"}),
            &json!("输出正文"),
            Some("行上的 error"),
            Some("宿主覆盖位"),
            Some("failed"),
            false,
        );
        assert_eq!(with_context.outcome_text.as_deref(), Some("宿主覆盖位"));

        let with_row_error = build_resolve_question_model(
            &json!({"question_id": "q"}),
            &json!("输出正文"),
            Some("行上的 error"),
            None,
            Some("failed"),
            false,
        );
        assert_eq!(with_row_error.outcome_text.as_deref(), Some("行上的 error"));

        let output_only = build_resolve_question_model(
            &json!({"question_id": "q"}),
            &json!("输出正文"),
            None,
            None,
            Some("failed"),
            false,
        );
        assert_eq!(output_only.outcome_text.as_deref(), Some("输出正文"));

        // ★失败压掉 answering：即使 status 是 in_progress 的兄弟态也不该并进「仍在答」分支。
        assert!(with_context.is_failed);
        assert!(!with_context.is_answering);
    }

    #[test]
    fn blank_error_text_is_still_the_preferred_channel() {
        // ★真源 :87 用的是 `??`（只跳 null/undefined），不是空白判定——
        // 宿主给的 errorText 就算是纯空格也优先于行上的 error / output。
        // （这里曾把断言写成「空白继续往下找」，是实现对了、断言错了。）
        let m = build_resolve_question_model(
            &json!({}),
            &json!("输出正文"),
            Some("行上的 error"),
            Some("  "),
            Some("failed"),
            false,
        );
        assert_eq!(m.outcome_text.as_deref(), Some("  "));
        // 缺席（None）才往下走 readText 链——readText 才会因空白而跳过。
        let m = build_resolve_question_model(
            &json!({}),
            &json!("输出正文"),
            Some("   "),
            None,
            Some("failed"),
            false,
        );
        assert_eq!(m.outcome_text.as_deref(), Some("输出正文"));
    }

    #[test]
    fn inline_preview_collapses_and_truncates() {
        assert_eq!(
            to_inline_preview(Some("多行\n答案  在这里")).as_deref(),
            Some("多行 答案 在这里")
        );
        let long = "字".repeat(200);
        let preview = to_inline_preview(Some(&long)).expect("preview");
        assert_eq!(preview.chars().count(), 161);
        assert!(preview.ends_with('…'));
        assert_eq!(to_inline_preview(Some("   \n ")), None);
        assert_eq!(to_inline_preview(None), None);
        // 恰好 160 码点不截断（真源 :42 是 `>` 不是 `>=`）。
        let exact = "字".repeat(INLINE_PREVIEW_MAX_LENGTH);
        assert_eq!(to_inline_preview(Some(&exact)), Some(exact));
    }

    #[test]
    fn preview_is_of_the_answer_not_the_qid() {
        // 概要来自答案（真源 :92 传的是 answer），qid 只出现在展开区。
        let m = build(
            json!({"question_id": "q-1", "answer": "第一段\n第二段"}),
            json!(""),
            Some("completed"),
            false,
        );
        assert_eq!(m.inline_preview.as_deref(), Some("第一段 第二段"));
    }
}
