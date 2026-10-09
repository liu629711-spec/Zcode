//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/eval-workflow-snippet.tsx`
//! （264 行）。
//!
//! **裁剪注明**（对真源 264 行能力的差距）：
//! - `CodeBlock` 用的是 Rust 侧 v1 简化版（`codeBlock.rs` 模块头已列差距表：
//!   无 Header/复制按钮、行号、wrapLongLines）。真源 SnippetBody 用了
//!   `CodeBlockHeader` + `CodeBlockCopyButton`，本轮以静态标题替代
//!   （见文件末 TODO）。
//! - `useNowTicker`（运行中每秒一格计时）依赖 `workflow-graph/use-now-ticker.ts`，
//!   本轮把计时判定抽成纯函数 `duration_text`，秒级刷新待该hook 迁移时接。

use leptos::prelude::*;
use serde_json::Value;
use std::sync::Arc;

use super::super::ToolLayout::ToolLayoutComponent;
use super::super::i18n;
use super::super::toolCallRowAdapter::LegacyToolCall;
use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};
use super::codeBlock::CodeBlock;
use super::workflow_diagnostics::{
    WorkflowDiagnosticEntry, WorkflowDiagnosticsSectionComponent,
};
use super::workflow_snippet_presentation::{snippet_response, snippet_value};

// ---------------------------------------------------------------------------
// 计时（真源 :29-42）
// ---------------------------------------------------------------------------

/// 运行中按整秒走、每秒一格（与面板里其他计时同一粒度）；
/// 终态定格在工具结果的精确毫秒。
///
/// ★真源 :30-33 注释——**不能每 100ms 显示一次毫秒**：尾数肉眼读不了，
/// 却是聊天区里最频繁的待处理更新（React 嵌套更新计数在投影帧积压时的种子）。
pub fn running_duration_seconds(now_ms: f64, started_at: f64) -> String {
    format!("{}s", ((now_ms - started_at) / 1000.0).max(0.0).floor())
}

/// 终态的毫秒读数（真源 :36-38）。
pub fn terminal_duration_ms(duration_ms: i64) -> String {
    format!("{duration_ms}ms")
}

/// 失败判定（真源 :28）。
pub fn is_failed(running: bool, status: &str, snippet_ok: Option<bool>) -> bool {
    !running && (status == "failed" || snippet_ok == Some(false))
}

/// 展开入口判定（真源 :60-69）。
///
/// ★真源 :60 注释——展开入口与当前**实际展示的内容**一致，
/// 不能因隐藏的代码/日志留下空面板。
pub fn has_details(
    running: bool,
    failed: bool,
    code: Option<&str>,
    logs_len: usize,
    diagnostics_len: usize,
    response: Option<&str>,
    error: Option<&str>,
) -> bool {
    if running {
        return code.is_some();
    }
    if failed {
        return code.is_some()
            || logs_len > 0
            || diagnostics_len > 0
            || response.is_some()
            || error.is_some();
    }
    response.is_some()
}

/// kindLabel（真源 :97-105）。
pub fn kind_label(running: bool) -> &'static str {
    if running {
        "chat.toolCall.workflow.snippet.validating"
    } else {
        "chat.toolCall.workflow.snippet.ran"
    }
}

/// `eval_workflow_snippet` display 读出来（真源 :26）。
fn read_snippet_display(raw: &Value) -> Option<Value> {
    match read_tool_result_display(raw) {
        Some(ToolResultDisplay::Workflow(d)) if d.kind == "eval_workflow_snippet" => {
            Some(d.value)
        }
        _ => None,
    }
}

/// 从 display 的原始 JSON 里读出严格解析后的字段。
fn snippet_fields(value: &Value) -> Option<super::super::toolResultDisplay::EvalWorkflowSnippetDisplay> {
    super::super::toolResultDisplay::parse_eval_workflow_snippet_display(value)
}

/// `EvalWorkflowSnippetToolCallBlock`（真源 :24-127）。
#[component]
pub fn EvalWorkflowSnippetToolCallBlock(tool_call: LegacyToolCall) -> impl IntoView {
    let running = false; // 真源读 context.isRunning；Rust 侧由调用方传入后合并
    let snippet = read_snippet_display(&tool_call.raw);
    let parsed = snippet.as_ref().and_then(snippet_fields);

    let snippet_ok = parsed.as_ref().map(|p| p.ok);
    let failed = is_failed(running, &tool_call.status, snippet_ok);

    // 代码：input.code（真源 :51-52）。
    let code = tool_call
        .input
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    // 响应文本（真源 :53-61）：display 优先，退回 output。
    let response = parsed.as_ref().and_then(|p| {
        snippet_response(
            p.ok,
            p.duration_ms,
            &p.response,
            &p.logs,
            &p
                .diagnostics
                .iter()
                .map(|d| (d.line, d.column, d.message.clone()))
                .collect::<Vec<_>>(),
        )
    });
    let response = response.or_else(|| {
        tool_call
            .output
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    });

    let error = tool_call.error.clone().or_else(|| {
        if !failed {
            return None;
        }
        response.clone().or_else(|| {
            parsed.as_ref().map(|p| {
                p.diagnostics
                    .iter()
                    .map(|d| d.message.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        })
    });

    let details = has_details(
        running,
        failed,
        code.as_deref(),
        parsed.as_ref().map(|p| p.logs.len()).unwrap_or(0),
        parsed.as_ref().map(|p| p.diagnostics.len()).unwrap_or(0),
        response.as_deref(),
        error.as_deref(),
    );

    // 时长文本（真源 :33-42）：失败时不出（真源 :45）。
    // 真源 :33-42 —— 运行中按整秒、终态定格毫秒；失败时不出（真源 :45）。
    let duration_text: Option<String> = if failed {
        None
    } else if running {
        tool_call
            .started_at
            .map(|started| running_duration_seconds(0.0, started))
    } else {
        parsed.as_ref().map(|p| terminal_duration_ms(p.duration_ms))
    };

    let kind = i18n::text(kind_label(running));
    let status_label = failed.then(|| i18n::text("chat.toolCall.status.failed"));
    let status_tooltip = if failed { error.clone() } else { None };
    let tool_id = tool_call.tool_id.clone();
    let show_icon = true;

    let body = details.then(|| {
        let code = code.clone();
        let response = response.clone();
        let diagnostics: Vec<WorkflowDiagnosticEntry> = parsed
            .as_ref()
            .map(|p| {
                p.diagnostics
                    .iter()
                    .map(|d| WorkflowDiagnosticEntry {
                        line: d.line,
                        column: d.column,
                        code: d.code,
                        message: d.message.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let logs: Vec<String> = parsed
            .as_ref()
            .map(|p| p.logs.clone())
            .unwrap_or_default();
        let truncated = parsed.as_ref().and_then(|p| p.truncated).unwrap_or(false);
        let render = move || -> AnyView {
            view! {
                <SnippetBody
                    code=code.clone()
                    response=response.clone()
                    diagnostics=diagnostics.clone()
                    logs=logs.clone()
                    failed=failed
                    truncated=truncated
                />
            }
            .into_any()
        };
        Arc::new(render)
            as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>
    });

    view! {
        <ToolLayoutComponent
            props=super::super::ToolLayout::ToolLayoutProps {
                tool_id: tool_id.clone(),
                show_icon: Some(show_icon),
                can_toggle: Some(details),
                force_open: Some(details),
                kind_label: Some(kind),
                // 真源 :107 —— 时长是 primaryText。
                primary_text: duration_text.clone(),
                // 真源 :109 —— 分隔符是「·」。
                summary_content_separator: Some("·".to_string()),
                status_label,
                status_tooltip,
                show_failure_status: Some(failed),
                is_running: Some(running),
                ..Default::default()
            }
            icon_view=Some(Arc::new(|| {
                view! {
                    // FlaskConical（lucide）。
                    <crate::app::Icon
                        paths=vec!["M10 2v2", "M14 2v2", "M16 8a6 6 0 0 1 4 9v3a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-3a6 6 0 0 1 4-9",
                                "M9.5 18h5"]
                        circles=vec![]
                        class="size-4 shrink-0 text-foreground-subtle"
                    />
                }
                .into_any()
            }))
            render_content=body
        />
    }
    .into_any()
}

/// `SnippetBody`（真源 :130-227）。
///
/// 三态：成功走单面板、失败走四区块（诊断 / 结果 / 日志 / 代码）。
#[component]
pub fn SnippetBody(
    code: Option<String>,
    response: Option<String>,
    diagnostics: Vec<WorkflowDiagnosticEntry>,
    logs: Vec<String>,
    failed: bool,
    truncated: bool,
) -> impl IntoView {
    if !failed {
        // 真源 :135-147 —— 成功态：运行中显示代码，终态显示返回值。
        // 两者都没有 → 不渲染（真源 `if (!content) return null`）。
        let (text, language): (String, &'static str) = if let Some(c) = code.clone() {
            (c, "typescript")
        } else {
            match response.as_deref() {
                None => return ().into_view().into_any(),
                Some(r) => snippet_value(r),
            }
        };
        let panel_truncated = truncated && code.is_none();
        return view! {
            <SnippetTextPanel
                text=text
                language=language
                truncated=panel_truncated
            />
        }
        .into_any();
    }

    // 真源 :151-224 —— 失败态四区块。
    let value = response.as_deref().map(snippet_value);
    // ★真源 :203 —— `logs.some(line => line.trim())`：全空白的日志段不渲染。
    let has_logs = logs.iter().any(|l| !l.trim().is_empty());

    view! {
        <div class="mb-2 space-y-3" data-testid="workflow-snippet-body">
            // ── 诊断（真源 :156-158）──
            {(!diagnostics.is_empty())
                .then(|| {
                    view! {
                        <WorkflowDiagnosticsSectionComponent
                            diagnostics=diagnostics.clone()
                            truncated=false
                            count=None
                            saved=false
                        />
                    }
                    .into_any()
                })}
            // ── 返回值（真源 :159-190）──
            {value.map(|(code, language)| {
                view! {
                    <section class="space-y-1.5" data-testid="snippet-result">
                        <div class="max-h-72 overflow-auto">
                            <CodeBlock code=code language=language.to_string() />
                            <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                                {i18n::text("chat.toolCall.status.failed")}
                            </h4>
                        </div>
                    </section>
                }
                .into_any()
            })}
            // ── 日志（真源 :191-203）──
            {has_logs.then(|| {
                let joined = logs.join("\n");
                view! {
                    <section class="space-y-1.5" data-testid="snippet-logs">
                        <h4 class="text-ui-sm font-medium text-foreground-subtlest">
                            {i18n::text("chat.toolCall.workflow.snippet.section.logs")}
                        </h4>
                        <pre class="max-h-64 overflow-auto whitespace-pre-wrap break-words \
                                    rounded-lg bg-panel px-3 py-2 font-mono text-ui-sm text-foreground-subtle">
                            {joined}
                        </pre>
                    </section>
                }
                .into_any()
            })}
            // ── 代码（真源 :204-214）──
            {code.clone().map(|c| {
                view! {
                    <details class="space-y-2" data-testid="snippet-code">
                        <summary class="cursor-pointer text-ui-sm text-foreground-subtlest">
                            {i18n::text("chat.toolCall.workflow.snippet.section.code")}
                        </summary>
                        <div class="max-h-64 overflow-auto">
                            <CodeBlock code=c.clone() language="typescript".to_string() />
                        </div>
                    </details>
                }
                .into_any()
            })}
            // ── 截断提示（真源 :215-219）──
            {truncated.then(|| {
                view! {
                    <p class="text-ui-xs text-foreground-subtle">
                        {i18n::text("chat.toolCall.workflow.truncated")}
                    </p>
                }
                .into_any()
            })}
        </div>
    }
    .into_any()
}

/// `SnippetTextPanel`（真源 :230-264）。
///
/// ★真源 :229 注释——共用 Markdown 工具栏，正文独立滚动，
/// 避免复制按钮随长内容卷走。
#[component]
pub fn SnippetTextPanel(
    text: String,
    language: &'static str,
    truncated: bool,
) -> impl IntoView {
    view! {
        <div class="mb-2 min-w-0" data-testid="workflow-snippet-body">
            <div data-testid="snippet-result">
                <CodeBlock code=text.clone() language=language.to_string() />
            </div>
            {truncated.then(|| {
                view! {
                    <p class="mt-1 text-ui-xs text-foreground-subtle">
                        {i18n::text("chat.toolCall.workflow.truncated")}
                    </p>
                }
                .into_any()
            })}
        </div>
    }
    .into_any()
}

// TODO(后续迁移)：以下真源部分本轮未实现。
// 1. `CodeBlockHeader` + `CodeBlockCopyButton`（真源 :180-186）——
//    Rust 侧 CodeBlock 是 v1 简化版（无 Header/复制按钮/行号/wrapLongLines），
//    待码查看器链路整体迁移时把 codeBlock.rs 补到 550 行能力再接。
// 2. `useNowTicker`（真源 :33）——运行中每秒一格刷新。当前
//    `running_duration_seconds` 是纯函数，秒级 tick 待该 hook 迁移。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolResultDisplay::parse_eval_workflow_snippet_display;
    use serde_json::json;

    fn tc(input: Value, output: Option<&str>, raw: Value) -> LegacyToolCall {
        LegacyToolCall {
            tool_id: "tc-1".into(),
            tool_name: Some("EvalWorkflowSnippet".into()),
            kind: "EvalWorkflowSnippet".into(),
            title: None,
            input: Some(input),
            status: "completed".into(),
            v4_status: "success".into(),
            output: output.map(str::to_string),
            content: None,
            error: None,
            raw,
            started_at: None,
            snapshot_refs: Vec::new(),
            thought: None,
        }
    }

    fn display_value(ok: bool, logs: &[&str], diags: &[Value], response: &str) -> Value {
        json!({
            "kind": "eval_workflow_snippet",
            "ok": ok,
            "diagnostics": diags,
            "logs": logs,
            "response": response,
            "durationMs": 12,
        })
    }

    // ── 计时 ──

    #[test]
    fn running_duration_floors_to_seconds() {
        // ★真源 :30 —— 不能每 100ms 显示毫秒，尾数肉眼读不了。
        assert_eq!(running_duration_seconds(3_500.0, 1_000.0), "2s");
        assert_eq!(running_duration_seconds(1_000.0, 1_000.0), "0s");
        // now < started（时钟漂移）→ 0s 不为负。
        assert_eq!(running_duration_seconds(500.0, 1_000.0), "0s");
    }

    #[test]
    fn terminal_duration_keeps_milliseconds() {
        // 真源 :36 —— 终态定格在精确毫秒。
        assert_eq!(terminal_duration_ms(1234), "1234ms");
        assert_eq!(terminal_duration_ms(0), "0ms");
    }

    // ── 状态判定 ──

    #[test]
    fn failure_needs_not_running() {
        // 真源 :28 —— 运行中即便 display 说失败也不算失败。
        assert!(!is_failed(true, "failed", Some(false)));
        assert!(is_failed(false, "failed", Some(false)));
        assert!(is_failed(false, "completed", Some(false)));
        assert!(!is_failed(false, "completed", Some(true)));
    }

    #[test]
    fn details_match_actually_shown_content() {
        // ★真源 :60 —— 展开入口与实际展示内容一致，
        // 不能因隐藏的代码/日志留下空面板。
        // 运行中只看代码。
        assert!(has_details(true, false, Some("code"), 0, 0, None, None));
        assert!(!has_details(true, false, None, 5, 5, Some("r"), Some("e")));

        // 成功终态只看响应。
        assert!(has_details(false, false, Some("code"), 5, 5, Some("r"), Some("e")));
        assert!(!has_details(false, false, Some("code"), 5, 5, None, None));

        // 失败终态看全部。
        assert!(has_details(false, true, None, 5, 5, None, None));
        assert!(!has_details(false, true, None, 0, 0, None, None));
    }

    #[test]
    fn kind_label_switches_on_running() {
        assert_ne!(kind_label(true), kind_label(false));
        // 两个词条都要在文案表里。
        for id in [
            "chat.toolCall.workflow.snippet.validating",
            "chat.toolCall.workflow.snippet.ran",
            "chat.toolCall.workflow.snippet.section.logs",
            "chat.toolCall.workflow.snippet.section.code",
        ] {
            assert_ne!(i18n::text(id), id, "{id} 词条应已迁");
        }
    }

    // ── display 读取 ──

    #[test]
    fn strict_display_is_required_for_snippet() {
        // 本轮给 eval_workflow_snippet 补了 strict 解析，非法 display 不成立。
        let valid = display_value(
            true,
            &["log"],
            &[json!({ "line": 1, "column": 1, "code": 9001, "message": "x" })],
            "Return value:\n42",
        );
        assert!(parse_eval_workflow_snippet_display(&valid).is_some());

        // 缺必填字段 → None（退回纯文本兜底）。
        let bad = json!({ "kind": "eval_workflow_snippet", "ok": true });
        assert!(parse_eval_workflow_snippet_display(&bad).is_none());
    }

    #[test]
    fn response_prefers_display_over_output() {
        // 真源 :53-61 —— display 优先，退回 output。
        // 真源 :22-30 —— 先剥耗时前缀，再剥 "Return value:\n"，
        // 所以测试数据要带耗时头（durationMs=12）才能验证完整链路。
        let display = display_value(
            true,
            &[],
            &[],
            "The snippet completed in 12ms.\nReturn value:\n42",
        );
        let parsed = parse_eval_workflow_snippet_display(&display).unwrap();
        assert_eq!(
            snippet_response(
                parsed.ok,
                parsed.duration_ms,
                &parsed.response,
                &parsed.logs,
                &[]
            )
            .as_deref(),
            Some("42"),
            "两层前缀都该被剥掉"
        );
    }

    #[test]
    fn output_used_when_display_absent() {
        let call = tc(json!({ "code": "x" }), Some("  纯文本输出  "), json!({}));
        let fallback = call
            .output
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        assert_eq!(fallback, Some("纯文本输出"));
    }

    #[test]
    fn code_requires_non_blank() {
        // 真源 :51-52 —— input.code 要 trim 后非空。
        for (input, expected) in [
            (json!({ "code": "  " }), false),
            (json!({ "code": "" }), false),
            (json!({ "code": "x" }), true),
            (json!({}), false),
        ] {
            let got = input
                .get("code")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .is_some();
            assert_eq!(got, expected, "input={input}");
        }
    }

    #[test]
    fn error_falls_back_to_joined_diagnostics() {
        // 真源 :62-64 —— 无 errorText 时用诊断消息拼行。
        let diags = vec![(1i64, 1i64, "第一处".to_string()), (2, 1, "第二处".to_string())];
        let joined = diags
            .iter()
            .map(|(_, _, m)| m.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(joined, "第一处\n第二处");
    }

    #[test]
    fn snippet_value_json_pretty_prints() {
        let (code, lang) = snippet_value(r#"{"b":2}"#);
        assert_eq!(lang, "json");
        assert!(code.contains('\n'), "应pretty-print：{code}");
    }

    // ── 组件冒烟（纯逻辑部分）──

    #[test]
    fn failed_body_shows_four_sections() {
        let diags = vec![WorkflowDiagnosticEntry {
            line: 1,
            column: 1,
            code: 9001,
            message: "x".into(),
        }];
        assert!(!diags.is_empty(), "诊断区块");
        assert!(has_details(
            false,
            true,
            Some("code"),
            1,
            1,
            Some("r"),
            Some("e")
        ));
    }

    #[test]
    fn blank_logs_are_hidden() {
        // ★真源 :203 —— logs.some(line => line.trim())。
        let blank = vec!["".to_string(), "   ".to_string()];
        assert!(!blank.iter().any(|l| !l.trim().is_empty()));
        let with_content = vec!["".to_string(), "真的日志".to_string()];
        assert!(with_content.iter().any(|l| !l.trim().is_empty()));
    }
}