//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/submit-result.tsx`（205 行）。
//!
//! actor 提交结果的卡（workflow family 里与 CreateWorkflow 卡面完全不同的那一张，
//! 见 `resolveRenderer.ts:148-156`）。
//!
//! 提交内容的读侧归一化（真源 :38-44）是**引擎侧实盘结论的镜像**——
//! `engine/scheduler.ts` 记档「真实模型常把 result 序列化成 JSON 字符串」并做同款
//! **单次**宽容 parse。不做多层递归 parse（引擎也只做一次），解析失败就按字符串对待，
//! 绝不抛错毁卡，也绝不改写任何数据。
//!
//! **裁剪注明**：
//! - `context.theme`（真源 :143）：桌面端单主题（见 ToolCallBlock.rs 文件头），
//!   代码块不再按主题取色。
//! - `canToggle/forceOpen ?? 默认` 走真源默认（同 escalate / plan_guidance）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::i18n;
use super::codeBlock::RichCodeBlock;
use crate::components::workflowIcons::icon_clipboard_check;

/// 真源 :9-11 —— `ClipboardCheckIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const SUBMIT_RESULT_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 折叠头部的单行概要上限（真源 :14）：概要只是「大概提交了什么」，整段内容在展开后的 body 里。
pub const INLINE_PREVIEW_MAX_LENGTH: usize = 160;

/// `toRecord`（真源 :16-32）。
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

/// `readText`（真源 :34-36）：非空白字符串（返回原值）。
pub fn read_text(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn read_text_opt(value: Option<&str>) -> Option<String> {
    let s = value?;
    read_text(&Value::String(s.to_string()))
}

/// `normalizeSubmittedResult`（真源 :45-60）：字符串先试**一次**宽容 `JSON.parse`，
/// 解析出对象/数组才用解析值。
///
/// ★真源 :55-56 —— 数字 / 布尔 / null 的字面量字符串保持原样：那是模型写的那句话，
/// 不是结构化载荷。所以 `"42"` 仍是散文 `"42"`，而 `"{\"a\":1}"` 变成结构化。
pub fn normalize_submitted_result(value: &Value) -> Value {
    let Some(s) = value.as_str() else {
        return value.clone();
    };
    if s.trim().is_empty() {
        return value.clone();
    }
    match serde_json::from_str::<Value>(s) {
        Ok(parsed) if parsed.is_object() || parsed.is_array() => parsed,
        _ => value.clone(),
    }
}

/// `stringifyResult`（真源 :63-69）：JSON 分支的缩进文本。
///
/// 真源 `JSON.stringify(value, null, 2) ?? String(value)` —— serde 的 pretty 同样是两空格缩进。
/// 循环引用之类的病态载荷在 Rust 侧不可能出现（`Value` 是树），但序列化仍可能失败
/// （如 NaN），失败时退回 `to_string`，卡片不能因载荷而崩。
pub fn stringify_result(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| js_string_value(value))
}

/// JS `String(value)` 对 JSON 值的形态：字符串不带引号，其余是紧凑 JSON。
fn js_string_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        other => other.to_string(),
    }
}

/// `toInlinePreview`（真源 :72-92）：字符串直接用，其余走 `JSON.stringify`（无缩进）；
/// 换行折叠成空格，超长截断。
pub fn to_inline_preview(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| js_string_value(other)),
    };
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
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

/// 卡片模型（真源 :97-141）。
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitResultModel {
    pub has_result: bool,
    pub normalized: Option<Value>,
    pub is_prose: bool,
    pub is_rejected: bool,
    pub is_stopped: bool,
    pub is_submitting: bool,
    pub kind_label_id: &'static str,
    pub rejection_text: Option<String>,
    pub has_details: bool,
    pub inline_preview: Option<String>,
}

/// 四相的种类词（真源 :114-120）。
///
/// ★真源 :108-109 —— 拒绝与停止在卡面上是同一句话：提交没走完。
/// spec 只定义四相，不为 denied 造第五个词条；但 `stopped` 与 `denied` 两个状态值都要认。
pub fn submit_result_kind_label_id(
    is_rejected: bool,
    is_stopped: bool,
    is_submitting: bool,
) -> &'static str {
    if is_rejected {
        "chat.toolCall.submitResult.rejected"
    } else if is_stopped {
        "chat.toolCall.submitResult.stopped"
    } else if is_submitting {
        "chat.toolCall.submitResult.submitting"
    } else {
        "chat.toolCall.submitResult.submitted"
    }
}

/// 真源 :97-141。`error` / `output_text` 是 legacy 行的两个字段。
pub fn build_submit_result_model(
    input: &Value,
    error: Option<&str>,
    output_text: Option<&str>,
    error_text: Option<&str>,
    status: &str,
    is_running: bool,
) -> SubmitResultModel {
    let record = to_record(input);
    // 真源 :100 —— 门是「result 键在场」而不是「result 有值」：提交 null 也是一次真实提交。
    let has_result = record.as_ref().is_some_and(|r| r.get("result").is_some());
    let normalized = has_result
        .then(|| record.as_ref().and_then(|r| r.get("result")))
        .flatten()
        .map(normalize_submitted_result);
    let is_prose = normalized.as_ref().is_some_and(|v| v.is_string());

    let is_rejected = status == "failed";
    let is_stopped = status == "stopped" || status == "denied";
    let is_submitting =
        !is_rejected && !is_stopped && (is_running || matches!(status, "pending" | "in_progress"));
    let kind_label_id = submit_result_kind_label_id(is_rejected, is_stopped, is_submitting);

    // 真源 :122-126 —— 驳回原文：错误通道优先，其次工具自己的错误字段与纯文本输出。
    // 接受态的输出恒为「The result was accepted.」，没有信息量，从不读。
    let rejection_text = is_rejected.then(|| {
        error_text
            .map(str::to_string)
            .or_else(|| read_text_opt(error))
            .or_else(|| read_text_opt(output_text))
    }).flatten();

    // 真源 :129 —— 驳回态是扁平行，不给展开入口。
    let has_details = !is_rejected && has_result;
    let inline_preview = if has_result {
        normalized.as_ref().and_then(to_inline_preview)
    } else {
        None
    };

    SubmitResultModel {
        has_result,
        normalized,
        is_prose,
        is_rejected,
        is_stopped,
        is_submitting,
        kind_label_id,
        rejection_text,
        has_details,
        inline_preview,
    }
}

/// `SubmitResultToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct SubmitResultBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub status: String,
    pub is_running: bool,
    pub error: Option<String>,
    pub error_text: Option<String>,
    pub output_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `SubmitResultToolCallBlock`（真源 :94-204）。
#[component]
pub fn SubmitResultToolCallBlock(props: SubmitResultBlockProps) -> impl IntoView {
    let model = build_submit_result_model(
        &props.input,
        props.error.as_deref(),
        props.output_text.as_deref(),
        props.error_text.as_deref(),
        &props.status,
        props.is_running,
    );
    // 真源 :138 —— inlinePreview ?? title ?? "submit_result"（工具名兜底，不走文案表）。
    let primary_text = model
        .inline_preview
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "submit_result".to_string());

    let has_result = model.has_result;
    let is_prose = model.is_prose;
    // 真源 :150-165 —— 散文 vs 结构化两条呈现路，先算好文本再进 view!（子节点先于属性求值）。
    let prose_text = model
        .normalized
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let structured_code = if has_result && !is_prose {
        model
            .normalized
            .as_ref()
            .map(stringify_result)
            .unwrap_or_default()
    } else {
        String::new()
    };
    let result_label = i18n::text("chat.toolCall.submitResult.resultHeading");
    let has_details = model.has_details;
    let render_content = move || {
        if !has_result {
            return view! { <div class="space-y-3"></div> }.into_any();
        }
        let body = if is_prose {
            // 人话是 prose：DESIGN.md 把 mono 留给路径/命令/代码/标识符/终端数据。
            // 刻意不做 markdown 渲染——result 字符串没有 markdown 契约（真源 :151-152）。
            let text = prose_text.clone();
            view! {
                <p class="whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 text-ui-base leading-5 text-foreground">
                    {text}
                </p>
            }
            .into_any()
        } else {
            // 结构化载荷走 CodeBlock（尊重用户的代码字号设置），容器逐字照 mcp.tsx（真源 :157-158）。
            let code = structured_code.clone();
            view! {
                <div class="max-h-72 overflow-auto rounded-xl border border-border bg-card">
                    <RichCodeBlock
                        code=code
                        language="json".to_string()
                        label=None
                        copy_label=None
                        wrap_label=None
                    />
                </div>
            }
            .into_any()
        };
        view! {
            <div class="space-y-3">
                <section class="space-y-1.5">
                    <h4 class="text-ui-sm font-medium text-foreground-subtlest">{result_label.clone()}</h4>
                    {body}
                </section>
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
                can_toggle: Some(has_details),
                force_open: Some(false),
                hide_secondary_text_when_open: Some(true),
                kind_label: Some(i18n::text(model.kind_label_id)),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                status_label: model
                    .is_rejected
                    .then(|| i18n::text("chat.toolCall.status.failed")),
                status_tooltip: model.rejection_text.clone(),
                show_failure_status: Some(model.is_rejected),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=SUBMIT_RESULT_TOOL_ICON_CLASS>{icon_clipboard_check()}</span>
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

    fn model(input: Value, status: &str, is_running: bool) -> SubmitResultModel {
        build_submit_result_model(&input, None, None, None, status, is_running)
    }

    #[test]
    fn gate_is_the_presence_of_the_result_key_not_its_value() {
        // 真源 :99-100 —— 提交 null 也是一次真实提交。
        let m = model(json!({"result": null}), "completed", false);
        assert!(m.has_result);
        assert!(m.has_details);
        assert_eq!(m.normalized, Some(Value::Null));
        assert!(!m.is_prose);
        // 空字符串同样算在场（门不看值）。
        assert!(model(json!({"result": ""}), "completed", false).has_result);
        // 键缺席 → 没有展开入口（streaming 首帧 input 还是 `{}` 的形态）。
        let empty = model(json!({}), "in_progress", true);
        assert!(!empty.has_result);
        assert!(!empty.has_details);
        assert_eq!(empty.inline_preview, None);
    }

    #[test]
    fn lenient_parse_is_single_pass_and_shape_checked() {
        // 真源 :45-60 —— 只有解析出对象/数组才换成解析值。
        assert_eq!(
            normalize_submitted_result(&json!("{\"a\":1}")),
            json!({"a": 1})
        );
        assert_eq!(normalize_submitted_result(&json!("[1,2]")), json!([1, 2]));
        // ★数字 / 布尔 / null 的字面量字符串保持原样：那是模型写的那句话。
        assert_eq!(normalize_submitted_result(&json!("42")), json!("42"));
        assert_eq!(normalize_submitted_result(&json!("true")), json!("true"));
        assert_eq!(normalize_submitted_result(&json!("null")), json!("null"));
        // 解析失败按字符串对待，不抛错。
        assert_eq!(normalize_submitted_result(&json!("not json")), json!("not json"));
        // 空白串不试 parse（真源 :49-51）。
        assert_eq!(normalize_submitted_result(&json!("   ")), json!("   "));
        // 非字符串原样返回。
        assert_eq!(normalize_submitted_result(&json!({"a": 1})), json!({"a": 1}));
        assert_eq!(normalize_submitted_result(&json!(7)), json!(7));
    }

    #[test]
    fn prose_and_structured_split() {
        // 散文 → `<p>`，结构化 → CodeBlock(json)。
        let prose = model(json!({"result": "接口改完了，测试通过"}), "completed", false);
        assert!(prose.is_prose);
        assert_eq!(prose.inline_preview.as_deref(), Some("接口改完了，测试通过"));

        let structured = model(json!({"result": "{\"ok\":true}"}), "completed", false);
        assert!(!structured.is_prose);
        // 概要取紧凑 JSON 形态（真源 :78 的 JSON.stringify 无缩进）。
        assert_eq!(structured.inline_preview.as_deref(), Some(r#"{"ok":true}"#));
    }

    #[test]
    fn stringify_uses_two_space_indent() {
        // 真源 :65 —— JSON.stringify(value, null, 2)。
        let text = stringify_result(&json!({"a": [1, 2]}));
        assert_eq!(text, "{\n  \"a\": [\n    1,\n    2\n  ]\n}");
    }

    #[test]
    fn inline_preview_collapses_and_truncates() {
        assert_eq!(
            to_inline_preview(&json!("多行\n结果  在这里")).as_deref(),
            Some("多行 结果 在这里")
        );
        let long = "字".repeat(200);
        let preview = to_inline_preview(&json!(long)).expect("preview");
        assert_eq!(preview.chars().count(), 161);
        assert!(preview.ends_with('…'));
        assert_eq!(to_inline_preview(&json!("   \n ")), None);
        // null 的概要就是 "null"（String(null) 形态）。
        assert_eq!(to_inline_preview(&Value::Null).as_deref(), Some("null"));
    }

    #[test]
    fn four_phase_labels() {
        // 真源 :114-120 的四相。
        assert_eq!(
            submit_result_kind_label_id(true, false, false),
            "chat.toolCall.submitResult.rejected"
        );
        assert_eq!(
            submit_result_kind_label_id(false, true, false),
            "chat.toolCall.submitResult.stopped"
        );
        assert_eq!(
            submit_result_kind_label_id(false, false, true),
            "chat.toolCall.submitResult.submitting"
        );
        assert_eq!(
            submit_result_kind_label_id(false, false, false),
            "chat.toolCall.submitResult.submitted"
        );
    }

    #[test]
    fn stopped_and_denied_share_the_stopped_phase() {
        // 真源 :109 —— `stopped` 或 `denied` 都算停止；拒绝优先级最高。
        for status in ["stopped", "denied"] {
            let m = model(json!({"result": "x"}), status, false);
            assert!(m.is_stopped, "{status} 应判为停止");
            assert!(!m.is_submitting);
            assert_eq!(m.kind_label_id, "chat.toolCall.submitResult.stopped");
            // 停止态仍给展开入口（只有驳回态收成扁平行）。
            assert!(m.has_details);
        }
        // failed 压过一切：既不是 stopped 也不是 submitting。
        let rejected = model(json!({"result": "x"}), "failed", true);
        assert!(rejected.is_rejected);
        assert!(!rejected.is_stopped);
        assert!(!rejected.is_submitting);
        assert_eq!(rejected.rejection_text, None, "没给任何错误通道时驳回原文缺席");
    }

    #[test]
    fn submitting_requires_running_or_early_status() {
        // 真源 :110-113。
        assert!(model(json!({}), "in_progress", false).is_submitting);
        assert!(model(json!({}), "pending", false).is_submitting);
        assert!(model(json!({}), "completed", true).is_submitting);
        assert!(!model(json!({}), "completed", false).is_submitting);
    }

    #[test]
    fn rejection_text_prefers_the_host_channel() {
        // 真源 :125 —— errorText ?? toolCall.error ?? toolCall.output；非驳回态恒缺席。
        let m = build_submit_result_model(
            &json!({"result": "x"}),
            Some("行上的 error"),
            Some("输出正文"),
            Some("宿主覆盖位"),
            "failed",
            false,
        );
        assert_eq!(m.rejection_text.as_deref(), Some("宿主覆盖位"));

        let m = build_submit_result_model(
            &json!({"result": "x"}),
            Some("行上的 error"),
            Some("输出正文"),
            None,
            "failed",
            false,
        );
        assert_eq!(m.rejection_text.as_deref(), Some("行上的 error"));

        let m = build_submit_result_model(
            &json!({"result": "x"}),
            None,
            Some("输出正文"),
            None,
            "failed",
            false,
        );
        assert_eq!(m.rejection_text.as_deref(), Some("输出正文"));

        // ★接受态的输出从不读（真源 :123-124：那句「The result was accepted.」没有信息量）。
        let accepted = build_submit_result_model(
            &json!({"result": "x"}),
            None,
            Some("The result was accepted."),
            None,
            "completed",
            false,
        );
        assert_eq!(accepted.rejection_text, None);
    }

    #[test]
    fn string_input_is_parsed_once() {
        // 真源 :16-32 —— input 本身可能是 JSON 字符串。
        let m = model(json!("{\"result\":\"跑通了\"}"), "completed", false);
        assert!(m.has_result);
        assert!(m.is_prose);
        assert_eq!(m.inline_preview.as_deref(), Some("跑通了"));
        // 非对象的字符串 input 不认。
        assert!(!model(json!("plain"), "completed", false).has_result);
        // 数组也不是 record。
        assert!(!model(json!([{"result": "x"}]), "completed", false).has_result);
    }
}
