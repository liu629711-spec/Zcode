//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/mcp.tsx`（340 行）。
//!
//! 通用 MCP 工具卡：把结果当一级内容（紧凑短文或可复制/换行的代码块），
//! 调用元数据（描述 + 参数）收进透明的二级折叠区——像 API 调试器。
//!
//! **裁剪注明**：
//! - 语法高亮经 `RichCodeBlock` 的 hljs 后处理（见 codeBlock.rs 文件头）。
//! - 二级折叠区（真源 Radix `Collapsible`）用 RwSignal + 条件挂载等价：
//!   无开合动画，`data-[state=open]` 的 chevron 旋转改为条件类名。
//! - `hasPrimaryResult`（真源 :178）只进了 useCallback 依赖、没进 JSX——死变量，不搬。
//! - 真源 `toolCall.output` 是 unknown（可为对象）；Rust 侧 v4 适配层把输出
//!   压平为 `output.text` 散文（toolCallRowAdapter），对象结果只有以 JSON 文本
//!   形态落在 output 里才可见——上游适配器的形态约束，不是本文件的分支缺失。
//! - 长度判定（`isCompactResult` 的 ≤160）按 Unicode 标量数（chars().count()）
//!   近似 JS 的 UTF-16 长度（增补平面字符差 1，影响可忽略）。

use leptos::prelude::*;
use serde_json::Value;

use super::codeBlock::RichCodeBlock;
use super::super::ToolSnapshotFieldNotice::ToolSnapshotFieldNoticeProps;
use crate::ToolCallBlocks::i18n;
use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;

/// `MCP_TOOL_ICON`（真源 :29 —— PlugIcon，lucide plug）。
pub fn mcp_tool_icon() -> impl IntoView {
    view! {
        <span class="inline-flex size-4 shrink-0 text-foreground-subtle">
            <crate::app::Icon
                circles=vec![]
                paths=vec![
                    "M12 22v-5",
                    "M15 8V2",
                    "M17 8a1 1 0 0 1 1 1v4a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1z",
                    "M9 8V2",
                ]
            />
        </span>
    }
}

/// `COMPACT_RESULT_MAX_LENGTH`（真源 :30）。
pub const COMPACT_RESULT_MAX_LENGTH: usize = 160;

/// `looksLikeJson`（真源 :36-45）：`{`/`[` 开头且能整体解析成 JSON。
pub fn looks_like_json(value: &str) -> bool {
    let trimmed = value.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return false;
    }
    serde_json::from_str::<serde::de::IgnoredAny>(trimmed).is_ok()
}

/// `isCompactResult`（真源 :47-55）：非空、无换行、非 JSON、≤160。
pub fn is_compact_result(value: &str) -> bool {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    length > 0
        && length <= COMPACT_RESULT_MAX_LENGTH
        && !trimmed.contains('\n')
        && !looks_like_json(trimmed)
}

/// `stringifyMcpResult`（真源 :57-65）：字符串原样；缺席/空 → None；
/// 其余 JSON 化（两空格缩进）。`String(value)` 兜底分支在 Rust 里不可达
/// （serde_json 序列化不会抛错），略。
pub fn stringify_mcp_result(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        other => serde_json::to_string_pretty(other).ok(),
    }
}

/// `McpToolPresentation`（真源 :22-27；`kind: "mcp_tool"` 是常量判据，Rust 侧省略字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolPresentation {
    pub server_name: String,
    pub tool_name: String,
    /// display 载荷可携带的描述（legacy 拆分不产出）。
    pub description: Option<String>,
}

/// `isMcpToolPresentation`（真源 :67-77 的守卫语义）：serverName / toolName 都必须是
/// 非空字符串——interface 里 serverName 虽标 optional，守卫要求**在场**。
fn parse_mcp_tool_presentation(display: Option<&Value>) -> Option<McpToolPresentation> {
    let display = display?;
    if display.get("kind").and_then(|k| k.as_str()) != Some("mcp_tool") {
        return None;
    }
    let server_name = display
        .get("serverName")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let tool_name = display
        .get("toolName")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some(McpToolPresentation {
        server_name: server_name.to_string(),
        tool_name: tool_name.to_string(),
        description: display
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// `readLegacyMcpToolPresentation`（真源 :79-115）。
///
/// display 持久化上线前的 MCP 历史记录只剩协议执行名，通用 renderer 会把内部 raw
/// JSON 整块暴露出来。这里只按协议 envelope 和编码 token 做机械拆分；新记录仍以
/// tools/list 的 discovery display 为权威，不让 legacy 规则覆盖它。
pub fn read_legacy_mcp_tool_presentation(tool_name: Option<&str>) -> Option<McpToolPresentation> {
    let tool_name = tool_name?;
    let segments: Vec<&str> = tool_name.split("__").collect();
    if segments.len() != 3 || segments[0] != "mcp" || segments[1].is_empty() || segments[2].is_empty()
    {
        return None;
    }

    let encoded_server_name = segments[1];
    let encoded_tool_name = segments[2];
    let server_tokens: Vec<&str> = encoded_server_name.split('_').filter(|s| !s.is_empty()).collect();
    let tool_tokens: Vec<&str> = encoded_tool_name.split('_').filter(|s| !s.is_empty()).collect();

    // 找 server 尾部与 tool 头部的最长共享 token 段（大小写无关）。
    let mut shared_token_count = 0usize;
    let maximum_shared_tokens = server_tokens.len().min(tool_tokens.len());
    for token_count in (1..=maximum_shared_tokens).rev() {
        let server_tail = server_tokens[server_tokens.len() - token_count..].join("_");
        let tool_head = tool_tokens[..token_count].join("_");
        if server_tail.to_lowercase() == tool_head.to_lowercase() {
            shared_token_count = token_count;
            break;
        }
    }

    let plugin_scoped = server_tokens
        .first()
        .map(|t| t.to_lowercase() == "plugin")
        .unwrap_or(false);
    let server_name = if shared_token_count > 0 {
        server_tokens[server_tokens.len() - shared_token_count..].join("_")
    } else if plugin_scoped {
        // serverTokens.at(-1)!——空 token 已被 filter(Boolean) 排除，恒有尾元素。
        server_tokens
            .last()
            .copied()
            .unwrap_or(encoded_server_name)
            .to_string()
    } else {
        encoded_server_name.to_string()
    };
    let action_name = if shared_token_count > 0 && shared_token_count < tool_tokens.len() {
        tool_tokens[shared_token_count..].join("_")
    } else {
        encoded_tool_name.to_string()
    };

    Some(McpToolPresentation {
        server_name,
        tool_name: action_name,
        description: None,
    })
}

/// `readMcpToolPresentation`（真源 :117-126）：display 权威，legacy 兜底。
pub fn read_mcp_tool_presentation(tool_call: &LegacyToolCall) -> Option<McpToolPresentation> {
    if let Some(presentation) =
        parse_mcp_tool_presentation(tool_call.raw.get("display"))
    {
        return Some(presentation);
    }
    read_legacy_mcp_tool_presentation(tool_call.tool_name.as_deref())
}

/// `formatMcpIdentifier`（真源 :128-132）：`[-_]+` → 空格、折叠空白、首字母大写。
pub fn format_mcp_identifier(value: &str) -> String {
    // `[-_]+` → " "，再 `\s+` → " "：等价于把分隔符段折叠成单个空格。
    let mut words = String::new();
    let mut pending_space = false;
    for c in value.trim().chars() {
        if c == '-' || c == '_' || c.is_whitespace() {
            pending_space = !words.is_empty();
        } else {
            if pending_space {
                words.push(' ');
                pending_space = false;
            }
            words.push(c);
        }
    }
    if words.is_empty() {
        return value.to_string();
    }
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
        None => words,
    }
}

/// `formatMcpServerLabel`（真源 :134-146）。
///
/// 插件 MCP 的 configured server key 带有 plugin:<plugin>:<server> 命名空间，
/// 直接作为 UI 文案会暴露内部路由标识。末段才是用户配置的 server 名称。
pub fn format_mcp_server_label(value: &str) -> String {
    let namespace_segments: Vec<&str> = value
        .split(':')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let display_identifier = if namespace_segments
        .first()
        .map(|s| s.to_lowercase() == "plugin")
        .unwrap_or(false)
        && namespace_segments.len() > 1
    {
        namespace_segments
            .last()
            .copied()
            .unwrap_or(value)
    } else {
        value
    };
    format_mcp_identifier(display_identifier)
}

/// `formatMcpToolLabel`（真源 :148-156）。
///
/// 不少 MCP 工具会再次用 server 名作为 tool 前缀；summary 同时展示 server
/// 来源文字时会出现 Firebase / Firebase get environment。只做大小写无关的机械去重。
pub fn format_mcp_tool_label(tool_name: &str, server_label: &str) -> String {
    let formatted_tool_name = format_mcp_identifier(tool_name);
    let repeated_prefix = format!("{server_label} ");
    if formatted_tool_name
        .to_lowercase()
        .starts_with(&repeated_prefix.to_lowercase())
    {
        format_mcp_identifier(&formatted_tool_name[repeated_prefix.len()..])
    } else {
        formatted_tool_name
    }
}

/// 摘要里 stopped 的追加节点（真源 `stoppedSummaryStatus`，:180-186）。
fn summary_status_suffix(status_label: &str) -> String {
    format!("· {status_label}")
}

/// `McpToolCallBlock`（真源 :158-356）。
#[component]
pub fn McpToolCallBlock(
    tool_call: LegacyToolCall,
    is_running: bool,
    show_icon: bool,
    can_toggle: Option<bool>,
    force_open: Option<bool>,
    /// 宿主解析好的失败正文（真源 `context.errorText`）。
    error_text: Option<String>,
    /// 状态词（真源 `context.statusLabel`，已本地化）。
    status_label: Option<String>,
    on_load_full_tool_call_fields: Option<Callback<String, bool>>,
) -> impl IntoView {
    let presentation = read_mcp_tool_presentation(&tool_call);
    let status_word = status_label.unwrap_or_default();

    let Some(presentation) = presentation else {
        // 真源 :328 —— 无 presentation 返回 null。分流层已保证在场，此处兜底。
        return ().into_any();
    };

    let server_label = format_mcp_server_label(&presentation.server_name);
    let tool_label = format_mcp_tool_label(&presentation.tool_name, &server_label);

    let call_details_label = i18n::text("chat.toolCall.mcp.callDetails");
    let result_label = i18n::text("chat.toolCall.mcp.result");
    let copy_result_label = i18n::text("chat.toolCall.mcp.copyResult");
    let wrap_lines_label = i18n::text("chat.toolCall.mcp.wrapLines");
    let description_label = i18n::text("chat.toolCall.mcp.description");
    let parameters_label = i18n::text("chat.toolCall.mcp.parameters");

    // hasCallDetails = description 或 input 在场（真源 :174）。
    let has_call_details = presentation.description.is_some() || tool_call.input.is_some();
    let output_value = tool_call
        .output
        .clone()
        .map(Value::String)
        .unwrap_or(Value::Null);
    let result_text = stringify_mcp_result(&output_value);
    let visible_error = if tool_call.status == "failed" {
        tool_call.error.clone().or(error_text.clone())
    } else {
        None
    };
    // Pending / stopped 没有可消费结果，展开只会重复状态或暴露诊断参数。
    // 即使父层请求 forceOpen，也必须遵守这两个生命周期的 summary-only 语义（真源 :336-338）。
    let is_summary_only_lifecycle = tool_call.status == "pending" || tool_call.status == "stopped";
    let can_toggle = !is_summary_only_lifecycle && can_toggle.unwrap_or(true);
    let force_open = !is_summary_only_lifecycle && force_open.unwrap_or(false);

    // 结果面板（真源 :200-256）：失败面 > 紧凑短文 > 代码块 > pending 面 > 状态行。
    let has_result_code_block = result_text
        .as_ref()
        .map(|text| !is_compact_result(text))
        .unwrap_or(false);
    let compact_text = result_text
        .as_ref()
        .filter(|text| is_compact_result(text))
        .map(|text| text.trim().to_string());
    let result_for_code = result_text.clone();
    let result_is_json = result_text
        .as_deref()
        .map(looks_like_json)
        .unwrap_or(false);
    let is_pending = tool_call.status == "pending";
    let status_line = status_word.clone();

    // 参数 JSON（真源 :282-295）：`JSON.stringify(input, null, 2)`。
    let parameters_json = tool_call
        .input
        .as_ref()
        .map(|input| serde_json::to_string_pretty(input).unwrap_or_default());
    let has_parameters = parameters_json.is_some();

    // 二级折叠区开合（真源 Radix Collapsible 的 Rust 等价；无动画）。
    let details_open = RwSignal::new(false);
    let description = presentation.description.clone();

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: tool_call.tool_id.clone(),
                show_icon: Some(show_icon),
                can_toggle: Some(can_toggle),
                force_open: Some(force_open),
                kind_label: Some("MCP".to_string()),
                primary_text: Some(tool_label),
                summary_content_separator: Some("·".to_string()),
                // stopped 是异常终态：显式分隔节点避免非动画摘要粘连（真源 :345-347）。
                secondary_text: (tool_call.status == "stopped")
                    .then(|| summary_status_suffix(&status_word)),
                status_label: (tool_call.status == "failed")
                    .then(|| summary_status_suffix(&status_word)),
                status_tooltip: if tool_call.status == "failed" {
                    error_text.clone()
                } else {
                    None
                },
                show_failure_status: Some(tool_call.status == "failed"),
                is_running: Some(is_running),
                title: Some(
                    presentation.description.clone().unwrap_or(presentation.tool_name.clone()),
                ),
                // kindDetail 是带色 span（真源 :340-342）——走 props 里的 view 槽。
                kind_detail_view: Some({
                    let server_label = server_label.clone();
                    std::sync::Arc::new(move || {
                        let server_label = server_label.clone();
                        view! {
                            <span class="text-foreground-subtle">{server_label}</span>
                        }
                        .into_any()
                    })
                        as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>
                }),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! { {mcp_tool_icon()} }.into_any()
            })
                as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>)
            render_content=Some(std::sync::Arc::new(move || {
                // MCP 详情之前先展示 description 和参数，用户展开后仍像 API 调试器。
                // 参考 BUA，把结果作为一级内容；调用元数据收进透明的二级折叠区（真源 :195-198）。
                view! {
                    <div class="mb-2 space-y-3 py-1" data-testid="mcp-expanded-content">
                        {if let Some(visible_error) = visible_error.clone() {
                            view! {
                                <section class="space-y-1.5">
                                    <h4 class="text-ui-base font-medium text-destructive">
                                        {result_label.clone()}
                                    </h4>
                                    <p
                                        class="whitespace-pre-wrap break-words rounded-lg bg-destructive/10 px-3 py-2 text-ui-base text-destructive"
                                        data-testid="mcp-error-surface"
                                    >
                                        {visible_error}
                                    </p>
                                </section>
                            }
                                .into_any()
                        } else if let Some(compact_text) = compact_text.clone() {
                            view! {
                                <p
                                    class="break-words rounded-xl border border-border bg-card px-3 py-2 text-ui-base text-foreground-subtle"
                                    data-testid="mcp-result-surface"
                                >
                                    {compact_text}
                                </p>
                            }
                                .into_any()
                        } else if has_result_code_block {
                            view! {
                                <div
                                    class="max-h-72 overflow-auto rounded-xl border border-border bg-card"
                                    data-testid="mcp-result-surface"
                                >
                                    <RichCodeBlock
                                        class="bg-card".to_string()
                                        code=result_for_code.clone().unwrap_or_default()
                                        copy_label=Some(copy_result_label.clone())
                                        label=Some(result_label.clone())
                                        language=if result_is_json { "json".to_string() } else { "log".to_string() }
                                        wrap_label=Some(wrap_lines_label.clone())
                                        wrap_long_lines=true
                                    />
                                </div>
                            }
                                .into_any()
                        } else if is_pending {
                            view! {
                                <p
                                    class="break-words rounded-xl border border-border bg-card px-3 py-2 text-ui-base text-foreground-subtle"
                                    data-testid="mcp-pending-surface"
                                >
                                    {status_line.clone()}
                                </p>
                            }
                                .into_any()
                        } else {
                            // 无 result 的 pending/running/stopped MCP 展开后 ToolCallBody 为空，
                            // 用户无法判断当前阶段。这里沿用 summary 的国际化状态，不另造生命周期文案。
                            view! {
                                <p class="text-ui-base text-foreground-subtle">{status_line.clone()}</p>
                            }
                                .into_any()
                        }}
                        {has_call_details.then(|| {
                            view! {
                                <div class="group/details">
                                    <button
                                        class="group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding font-medium whitespace-nowrap transition-colors outline-none select-none text-foreground-subtlest hover:bg-transparent hover:text-foreground h-6 gap-1 px-2 -ml-2 text-ui-base/relaxed"
                                        type="button"
                                        on:click=move |_| details_open.update(|v| *v = !*v)
                                    >
                                        <span class=if details_open.get() {
                                            "inline-flex size-3.5 rotate-90 transition-transform"
                                        } else {
                                            "inline-flex size-3.5 transition-transform"
                                        }>
                                            <crate::app::Icon circles=vec![] paths=vec!["m9 18 6-6-6-6"] />
                                        </span>
                                        {call_details_label.clone()}
                                    </button>
                                    {details_open.get().then(|| {
                                        view! {
                                            <div class="space-y-3 pt-2">
                                                {description.clone().map(|description| {
                                                    view! {
                                                        <section class="space-y-1.5">
                                                            <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                                                                {description_label.clone()}
                                                            </h4>
                                                            <p class="whitespace-pre-wrap break-words text-ui-base text-foreground-subtle">
                                                                {description}
                                                            </p>
                                                        </section>
                                                    }
                                                })}
                                                {has_parameters.then(|| {
                                                    view! {
                                                        <section class="space-y-1.5">
                                                            <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                                                                {parameters_label.clone()}
                                                            </h4>
                                                            <div class="max-h-72 overflow-auto rounded-xl border border-border bg-card">
                                                                <RichCodeBlock
                                                                    code=parameters_json.clone().unwrap_or_default()
                                                                    label=None
                                                                    copy_label=None
                                                                    wrap_label=None
                                                                    language="json".to_string()
                                                                />
                                                            </div>
                                                        </section>
                                                    }
                                                })}
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        })}
                        <crate::ToolCallBlocks::ToolSnapshotFieldNotice::ToolSnapshotFieldNoticeComponent
                            props=ToolSnapshotFieldNoticeProps {
                                refs: tool_call.snapshot_refs.clone(),
                                tool_id: tool_call.tool_id.clone(),
                                on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                            }
                        />
                    </div>
                }
                    .into_any()
                })
                    as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>
            )
        />
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn legacy(tool_name: Option<&str>, raw: Value) -> LegacyToolCall {
        crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall {
            tool_id: "t1".into(),
            tool_name: tool_name.map(str::to_string),
            kind: "unknown".into(),
            title: None,
            input: None,
            status: "completed".into(),
            v4_status: "success".into(),
            output: None,
            content: None,
            error: None,
            raw,
            started_at: None,
            snapshot_refs: Vec::new(),
            thought: None,
        }
    }

    #[test]
    fn looks_like_json_requires_bracket_start_and_parse() {
        // 真源 :36-45 —— {/[ 开头且能整体解析；其余一律不算。
        assert!(looks_like_json(" {\"a\":1} "));
        assert!(looks_like_json("[1,2]"));
        assert!(!looks_like_json("{\"a\":1"));
        assert!(!looks_like_json("plain text"));
        assert!(!looks_like_json(""));
    }

    #[test]
    fn compact_result_caps_length_and_rejects_json_or_newlines() {
        // 真源 :47-55 —— 非空、≤160、无换行、非 JSON。
        assert!(is_compact_result("done"));
        // 空串 length > 0 不成立 → false。
        assert!(!is_compact_result(""));
        assert!(!is_compact_result("line1\nline2"));
        assert!(!is_compact_result("{\"a\":1}"));
        let long = "x".repeat(COMPACT_RESULT_MAX_LENGTH + 1);
        assert!(!is_compact_result(&long));
        let exact = "x".repeat(COMPACT_RESULT_MAX_LENGTH);
        assert!(is_compact_result(&exact));
    }

    #[test]
    fn stringify_result_passes_strings_and_pretty_prints_values() {
        // 真源 :57-65 —— 字符串原样；缺席 → None；其余两空格缩进 JSON。
        assert_eq!(
            stringify_mcp_result(&Value::String("raw".into())).as_deref(),
            Some("raw")
        );
        assert_eq!(stringify_mcp_result(&Value::Null), None);
        let pretty = stringify_mcp_result(&json!({ "a": 1 })).unwrap();
        assert_eq!(pretty, "{\n  \"a\": 1\n}");
    }

    #[test]
    fn presentation_guard_requires_non_empty_names() {
        // 真源 :67-77 —— kind 必须 mcp_tool，serverName/toolName 必须非空。
        assert!(parse_mcp_tool_presentation(Some(&json!({
            "kind": "mcp_tool", "serverName": "Srv", "toolName": "get_env"
        })))
        .is_some());
        // serverName 缺席 → 守卫拒绝（interface 标 optional 但守卫要求在场）。
        assert!(parse_mcp_tool_presentation(Some(&json!({
            "kind": "mcp_tool", "toolName": "get_env"
        })))
        .is_none());
        assert!(parse_mcp_tool_presentation(Some(&json!({
            "kind": "other", "serverName": "Srv", "toolName": "get_env"
        })))
        .is_none());
        // description 透传。
        let with_description = parse_mcp_tool_presentation(Some(&json!({
            "kind": "mcp_tool", "serverName": "Srv", "toolName": "get_env",
            "description": "Fetch env"
        })))
        .unwrap();
        assert_eq!(with_description.description.as_deref(), Some("Fetch env"));
    }

    #[test]
    fn legacy_presentation_splits_protocol_names() {
        // 真源 :79-115 —— `mcp__server__tool` 机械拆分 + 共享 token 去重。
        // 简单形态。
        let p = read_legacy_mcp_tool_presentation(Some("mcp__firebase__get_env")).unwrap();
        assert_eq!(p.server_name, "firebase");
        assert_eq!(p.tool_name, "get_env");

        // 共享 token：server 尾部与 tool 头部重合（firebase_get_env → 去 firebase 前缀）。
        let p = read_legacy_mcp_tool_presentation(Some("mcp__firebase__firebase_get_env")).unwrap();
        assert_eq!(p.server_name, "firebase");
        assert_eq!(p.tool_name, "get_env");

        // plugin 命名空间：无共享 token 时取末段。
        let p =
            read_legacy_mcp_tool_presentation(Some("mcp__plugin_zcode-cua__computer_use")).unwrap();
        assert_eq!(p.server_name, "zcode-cua");
        assert_eq!(p.tool_name, "computer_use");

        // 段数不是 3 / 非空段缺失 → None。
        assert_eq!(read_legacy_mcp_tool_presentation(Some("mcp__only_two")), None);
        assert_eq!(read_legacy_mcp_tool_presentation(Some("read")), None);
        assert_eq!(read_legacy_mcp_tool_presentation(Some("mcp__a__")), None);
        assert_eq!(read_legacy_mcp_tool_presentation(None), None);
    }

    #[test]
    fn display_payload_beats_legacy_rules() {
        // 真源 :117-126 —— display 是权威，legacy 只兜历史记录。
        let tc = legacy(
            Some("mcp__old__names"),
            json!({ "display": { "kind": "mcp_tool", "serverName": "New Srv", "toolName": "tool" } }),
        );
        let p = read_mcp_tool_presentation(&tc).unwrap();
        assert_eq!(p.server_name, "New Srv");
        assert_eq!(p.tool_name, "tool");

        // display 不合格 → legacy 兜底。
        let tc = legacy(
            Some("mcp__legacy__tool"),
            json!({ "display": { "kind": "mcp_tool" } }),
        );
        let p = read_mcp_tool_presentation(&tc).unwrap();
        assert_eq!(p.server_name, "legacy");
    }

    #[test]
    fn identifier_formatting_collapses_separators() {
        // 真源 :128-132 —— [-_]+ 折叠成单空格，首字母大写。
        assert_eq!(format_mcp_identifier("get-env"), "Get env");
        assert_eq!(format_mcp_identifier("get__ENV"), "Get ENV");
        assert_eq!(format_mcp_identifier("--a-_b--"), "A b");
        assert_eq!(format_mcp_identifier("x"), "X");
    }

    #[test]
    fn server_label_unwraps_plugin_namespace() {
        // 真源 :134-146 —— plugin:...:server 取末段；非 plugin 命名空间原样格式化
        //（':' 不在 [-_] 集合里，保留）。
        assert_eq!(format_mcp_server_label("plugin:my-plug:my-srv"), "My srv");
        assert_eq!(format_mcp_server_label("firebase"), "Firebase");
        assert_eq!(format_mcp_server_label("plain:srv"), "Plain:srv");
    }

    #[test]
    fn tool_label_dedupes_repeated_server_prefix() {
        // 真源 :148-156 —— 大小写无关的去重。
        assert_eq!(format_mcp_tool_label("get_env", "Firebase"), "Get env");
        assert_eq!(
            format_mcp_tool_label("firebase_get_env", "Firebase"),
            "Get env"
        );
        assert_eq!(
            // 真源只把首字母大写、其余原样保留——"GET ENV" 不会被压成 "Get env"。
            format_mcp_tool_label("FIREBASE_GET_ENV", "firebase"),
            "GET ENV"
        );
        assert_eq!(format_mcp_tool_label("other_tool", "Firebase"), "Other tool");
    }
}
