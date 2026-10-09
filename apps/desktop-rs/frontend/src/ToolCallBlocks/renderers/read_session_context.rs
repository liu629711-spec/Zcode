//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/read-session-context.tsx`（288 行）。
//!
//! ReadSessionContext 工具卡：主代理把**另一个会话**的上下文读回来的一次查询。
//! 折叠行是 query + sessionId chip；展开是「查询」段 + 「结果」段（结果是模型写的 markdown）。
//!
//! ★真源 :184 —— 失败时可见正文换成 `errorText`（可能缺席，那时结果段走「没有返回上下文」），
//! 不是把失败前的旧结果继续画着。
//!
//! **裁剪注明**：
//! - `MessageResponse`（真源 :219-229）是带宿主回调的 markdown 渲染器
//!   （onOpenCodeViewer / onOpenFileLink / onOpenExternalUrl + codePreviewSettings）。
//!   Rust 侧同 `agentPromptSection.rs:7-8` 的做法：用 `app::render_markdown`（pulldown-cmark）
//!   输出 HTML——结构等价，交互回调未迁不挂。
//! - `context.theme`（真源 :222）：桌面端单主题。
//! - `canToggle/forceOpen` 走真源缺省。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::i18n;
use crate::components::workflowIcons::icon_book_open_text;

/// 真源 :9-11 —— `BookOpenTextIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const READ_SESSION_CONTEXT_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 结果段兜底正文的键序（真源 :70-77）。
const DIRECT_TEXT_KEYS: [&str; 6] = ["content", "output", "result", "text", "message", "error"];
/// 嵌套 record 的键序（真源 :82）。
const NESTED_RECORD_KEYS: [&str; 2] = ["rawOutput", "output"];

fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// `readStringField`（真源 :17-29）：按键序取第一个「非空白字符串」，返回**原值**（不 trim）。
pub fn read_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(candidate) = value.get(key).and_then(|v| v.as_str()) {
            if !candidate.trim().is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

/// `readNestedRecordField`（真源 :31-47）：按键序取第一个 record。
fn read_nested_record_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    if !is_plain_record(value) {
        return None;
    }
    keys.iter().find_map(|key| {
        let candidate = value.get(key)?;
        is_plain_record(candidate).then_some(candidate)
    })
}

/// `extractTextContent`（真源 :49-99）：递归把载荷读成一段文本。
///
/// 四条路：字符串（非空白）→ 原值；数组 → 逐项递归后 `"\n"` 拼接；
/// 数字/布尔 → `String(value)`；record → 直接键 / 嵌套 record / `content` 数组三跳。
/// 读不出来就是 None（不抛错、不 dump JSON）。
pub fn extract_text_content(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .filter_map(extract_text_content)
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        }
        // 真源 :62-68 —— 不是 record 也不是数组/字符串时，数字与布尔写成字面量，其余没有文本。
        // （真源还列了 bigint，serde_json 的 Number 已覆盖整数与浮点，无独立 bigint 形态。）
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => {
            if !is_plain_record(other) {
                return None;
            }
            if let Some(direct) = read_string_field(other, &DIRECT_TEXT_KEYS) {
                return Some(direct);
            }
            if let Some(nested) = read_nested_record_field(other, &NESTED_RECORD_KEYS) {
                if let Some(text) = extract_text_content(nested) {
                    return Some(text);
                }
            }
            if let Some(content) = other.get("content").filter(|c| c.is_array()) {
                if let Some(text) = extract_text_content(content) {
                    return Some(text);
                }
            }
            None
        }
    }
}

/// `readRawInput` / `readRawOutput`（真源 :101-115）：raw 上的 `rawInput ?? input`
/// 与 `rawOutput ?? output`。
fn raw_field<'a>(raw: &'a Value, first: &str, second: &str) -> Option<&'a Value> {
    if !is_plain_record(raw) {
        return None;
    }
    raw.get(first).or_else(|| raw.get(second))
}

/// `extractQuery`（真源 :117-142）：input 本身是字符串时优先——
/// 那是「查询就是一句话」的载荷形态，短路后不再往候选里翻。
/// 否则按真源 :125-130 的四候选序找 query / prompt / question：
/// input → output → raw 的入参侧 → raw 的输出侧。
pub fn extract_query(
    input: Option<&Value>,
    output: Option<&Value>,
    raw: &Value,
) -> Option<String> {
    if let Some(Value::String(s)) = input {
        return if s.trim().is_empty() { None } else { Some(s.clone()) };
    }
    let keys = ["query", "prompt", "question"];
    for candidate in [
        input,
        output,
        raw_field(raw, "rawInput", "input"),
        raw_field(raw, "rawOutput", "output"),
    ] {
        let Some(candidate) = candidate.filter(|v| is_plain_record(v)) else {
            continue;
        };
        if let Some(query) = read_string_field(candidate, &keys) {
            return Some(query);
        }
    }
    None
}

/// `extractSessionId`（真源 :144-164）：四候选里找 sessionId / session_id。
pub fn extract_session_id(
    input: Option<&Value>,
    output: Option<&Value>,
    raw: &Value,
) -> Option<String> {
    let keys = ["sessionId", "session_id"];
    for candidate in [
        input,
        output,
        raw_field(raw, "rawInput", "input"),
        raw_field(raw, "rawOutput", "output"),
    ] {
        let Some(candidate) = candidate.filter(|v| is_plain_record(v)) else {
            continue;
        };
        if let Some(session_id) = read_string_field(candidate, &keys) {
            return Some(session_id);
        }
    }
    None
}

/// `extractResultContent`（真源 :166-175）：output 优先，再 raw 的输出侧。
pub fn extract_result_content(output: Option<&Value>, raw: &Value) -> Option<String> {
    if let Some(text) = output.and_then(extract_text_content) {
        return Some(text);
    }
    raw_field(raw, "rawOutput", "output").and_then(extract_text_content)
}

/// `ReadSessionContextToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct ReadSessionContextBlockProps {
    pub tool_id: String,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub raw: Value,
    pub status: String,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `ReadSessionContextToolCallBlock`（真源 :177-287）。
#[component]
pub fn ReadSessionContextToolCallBlock(props: ReadSessionContextBlockProps) -> impl IntoView {
    let is_failed = props.status == "failed";
    // ★三件取值全部先在 view! 之外算好（children 先于属性求值）。
    let query = extract_query(props.input.as_ref(), props.output.as_ref(), &props.raw);
    let session_id = extract_session_id(props.input.as_ref(), props.output.as_ref(), &props.raw);
    let result_content = extract_result_content(props.output.as_ref(), &props.raw);
    // 真源 :184 —— 失败时可见正文换成 errorText（可能缺席）。
    let visible_result = if is_failed {
        props.error_text.clone()
    } else {
        result_content.clone()
    };
    let fallback_title = i18n::text("chat.toolCall.sessionContext.title");
    let primary_text = query
        .clone()
        .or_else(|| props.title.clone())
        .unwrap_or(fallback_title);
    let kind_label = i18n::text(if props.is_running {
        "chat.toolCall.sessionContext.reading"
    } else {
        "chat.toolCall.kind.sessionContext"
    });

    let query_for_body = query.clone();
    let visible_for_body = visible_result.clone();
    let render_content = move || {
        let result_body = match visible_for_body.clone() {
            // MessageResponse 的 Rust 等价：markdown → HTML（见文件头裁剪注明）。
            Some(text) if !text.is_empty() => {
                let html = crate::app::render_markdown(&text);
                view! {
                    <div class="min-w-0 break-words text-ui-base" inner_html=html></div>
                }
                .into_any()
            }
            _ => view! {
                <p class="text-ui-base text-foreground-subtle">
                    {i18n::text("chat.toolCall.sessionContext.noResult")}
                </p>
            }
            .into_any(),
        };
        view! {
            <div class="space-y-3 rounded-xl border border-border bg-panel px-4 py-3">
                {query_for_body.clone().map(|q| view! {
                    <section class="space-y-1.5">
                        <h4 class="text-ui-base font-medium uppercase text-foreground-subtle">
                            {i18n::text("chat.toolCall.sessionContext.query")}
                        </h4>
                        <p class="whitespace-pre-wrap break-words text-ui-base text-foreground">
                            {q}
                        </p>
                    </section>
                })}
                <section class="space-y-1.5">
                    <h4 class="text-ui-base font-medium uppercase text-foreground-subtle">
                        {i18n::text("chat.toolCall.sessionContext.result")}
                    </h4>
                    {result_body}
                </section>
            </div>
        }
        .into_any()
    };

    let secondary_view = session_id.map(|id| {
        std::sync::Arc::new(move || {
            view! {
                <code class="min-w-0 truncate rounded-md bg-surface px-1.5 py-0.5 font-mono text-ui-base text-foreground-subtle">
                    {id.clone()}
                </code>
            }
            .into_any()
        }) as ChildrenFn
    });

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(true),
                force_open: Some(false),
                hide_secondary_text_when_open: Some(true),
                kind_label: Some(kind_label),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                secondary_text_view: secondary_view,
                status_label: props.status_label.clone(),
                status_tooltip: if is_failed { props.error_text.clone() } else { None },
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=READ_SESSION_CONTEXT_TOOL_ICON_CLASS>{icon_book_open_text()}</span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content) as ChildrenFn)
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
    fn string_field_returns_the_original_not_the_trimmed_value() {
        // 真源 :23 —— 判定看 trim，取值用原值（保留缩进/换行是有意的）。
        let value = json!({"query": "  多行\n查询  "});
        assert_eq!(
            read_string_field(&value, &["query"]).as_deref(),
            Some("  多行\n查询  ")
        );
        // 纯空白不算值。
        assert_eq!(read_string_field(&json!({"query": "   "}), &["query"]), None);
        // 键序：先 content 再 output…
        let value = json!({"text": "后面的", "content": "前面的"});
        assert_eq!(
            read_string_field(&value, &DIRECT_TEXT_KEYS).as_deref(),
            Some("前面的")
        );
        // 非字符串跳过。
        assert_eq!(read_string_field(&json!({"query": 7}), &["query"]), None);
    }

    #[test]
    fn text_content_walks_the_four_shapes() {
        // 字符串
        assert_eq!(extract_text_content(&json!("一句话")).as_deref(), Some("一句话"));
        assert_eq!(extract_text_content(&json!("   ")), None);
        // 数组：逐项递归，`"\n"` 拼接，空项丢掉
        assert_eq!(
            extract_text_content(&json!(["甲", "  ", "乙"])).as_deref(),
            Some("甲\n乙")
        );
        assert_eq!(extract_text_content(&json!([])), None);
        // 嵌套的 content 块数组（Anthropic 形态）
        assert_eq!(
            extract_text_content(&json!({"content": [{"text": "块一"}, {"text": "块二"}]}))
                .as_deref(),
            Some("块一\n块二")
        );
        // 数字 / 布尔写成字面量
        assert_eq!(extract_text_content(&json!(42)).as_deref(), Some("42"));
        assert_eq!(extract_text_content(&json!(true)).as_deref(), Some("true"));
        // null 与 record 读不出来时是 None，不是 "null"
        assert_eq!(extract_text_content(&Value::Null), None);
        assert_eq!(extract_text_content(&json!({"unrelated": 1})), None);
    }

    #[test]
    fn text_content_falls_through_direct_then_nested_then_content() {
        // 直接键命中
        assert_eq!(
            extract_text_content(&json!({"result": "直接结果"})).as_deref(),
            Some("直接结果")
        );
        // 直接键没有 → 嵌套 rawOutput
        assert_eq!(
            extract_text_content(&json!({"rawOutput": {"text": "嵌套正文"}})).as_deref(),
            Some("嵌套正文")
        );
        // 嵌套也没有 → content 数组
        assert_eq!(
            extract_text_content(&json!({"content": ["数组正文"]})).as_deref(),
            Some("数组正文")
        );
    }

    #[test]
    fn string_input_short_circuits_the_query() {
        // 真源 :120-123 —— input 本身是字符串时，查询就是那句话。
        assert_eq!(
            extract_query(Some(&json!("看看上次那个会话")), None, &json!({}))
                .as_deref(),
            Some("看看上次那个会话")
        );
        // 空白字符串 input 也是「短路后没有」，不会再往候选里翻。
        assert_eq!(extract_query(Some(&json!("   ")), Some(&json!({"query": "不该被读"})), &json!({})), None);
    }

    #[test]
    fn query_looks_through_input_then_the_raw_pair_positions() {
        assert_eq!(
            extract_query(Some(&json!({"query": "从 input"})), None, &json!({})).as_deref(),
            Some("从 input")
        );
        // input 没有 → output（真源候选第二位）
        assert_eq!(
            extract_query(
                Some(&json!({"other": 1})),
                Some(&json!({"question": "从 output"})),
                &json!({})
            )
            .as_deref(),
            Some("从 output")
        );
        // 再退到 raw.rawInput
        assert_eq!(
            extract_query(
                Some(&json!({"other": 1})),
                None,
                &json!({"rawInput": {"question": "从 rawInput"}})
            )
            .as_deref(),
            Some("从 rawInput")
        );
        // 最后退回 raw.output（readRawOutput 的 rawOutput ?? output）
        assert_eq!(
            extract_query(
                Some(&json!({})),
                None,
                &json!({"output": {"prompt": "从 raw.output"}})
            )
            .as_deref(),
            Some("从 raw.output")
        );
        // 四个候选都没有 → None
        assert_eq!(extract_query(Some(&json!({})), None, &json!([])), None);
        // 字符串形态的候选不是 record，跳过（真源 :131-133 的 continue）
        assert_eq!(
            extract_query(Some(&json!({})), Some(&json!("散文 output")), &json!({})),
            None
        );
    }

    #[test]
    fn session_id_accepts_both_spellings() {
        assert_eq!(
            extract_session_id(Some(&json!({"sessionId": "s-1"})), None, &json!({})).as_deref(),
            Some("s-1")
        );
        assert_eq!(
            extract_session_id(None, Some(&json!({"session_id": "s-2"})), &json!({})).as_deref(),
            Some("s-2")
        );
        // 非 record 的候选跳过（字符串 output 不带 sessionId）。
        assert_eq!(extract_session_id(None, Some(&json!("散文")), &json!("raw")), None);
    }

    #[test]
    fn result_content_prefers_output_over_raw() {
        let output = json!({"text": "来自 output"});
        let raw = json!({"rawOutput": {"text": "来自 raw"}});
        assert_eq!(
            extract_result_content(Some(&output), &raw).as_deref(),
            Some("来自 output")
        );
        assert_eq!(
            extract_result_content(None, &raw).as_deref(),
            Some("来自 raw")
        );
        assert_eq!(extract_result_content(None, &json!({})), None);
    }

    #[test]
    fn raw_helpers_need_a_record() {
        // 真源 :102-106 / :110-114 —— raw 不是 record 时什么都读不出。
        assert!(raw_field(&json!([{"input": 1}]), "rawInput", "input").is_none());
        assert!(raw_field(&json!("s"), "rawInput", "input").is_none());
        assert_eq!(
            raw_field(&json!({"input": {"a": 1}}), "rawInput", "input"),
            Some(&json!({"a": 1}))
        );
        // rawInput 优先于 input
        assert_eq!(
            raw_field(&json!({"rawInput": {"q": 1}, "input": {"q": 2}}), "rawInput", "input"),
            Some(&json!({"q": 1}))
        );
    }

    #[test]
    fn icon_class_matches_source() {
        assert_eq!(
            READ_SESSION_CONTEXT_TOOL_ICON_CLASS,
            "size-4 flex-none text-foreground-subtle"
        );
        assert_eq!(DIRECT_TEXT_KEYS.len(), 6);
        assert_eq!(NESTED_RECORD_KEYS.len(), 2);
    }
}
