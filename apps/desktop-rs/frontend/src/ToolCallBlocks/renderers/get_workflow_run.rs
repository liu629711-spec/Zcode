//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/get-workflow-run.tsx`（274 行）。
//!
//! GetWorkflowRun 工具卡：一次「这个 run 现在怎么样了」的查询结果。
//! 情势截面（阶段轨 / 健康行）在 `get_workflow_run_situation.rs`，
//! 子代理花名册在 `get_workflow_run_roster.rs`。
//!
//! ★真源 :29-31 的两层状态：**查询错误与 run 执行失败是两个状态层级**，
//! 错误查询不能继续展示旧 display——所以 `failed` 时 `run` 直接被置为 undefined，
//! 正文只剩错误串，不再画阶段/花名册/健康。
//!
//! ★真源 :44-48 的取舍：折叠行有那句摘要就让摘要占住——一句「在第 2 / 4 个阶段、5 步已结算、
//! 2 个在跑」比「5/7 步」回答了更多问题。截断交给 CSS 的 `truncate`，不在这里切字符
//! （切出来的半句话在窄屏和宽屏上都是错的长度）。
//!
//! **裁剪注明**：`context.theme`（真源 :79/:80）——桌面端单主题，代码块不按主题取色；
//! `kindLabelOverride`（真源 :88）与 `canToggle/forceOpen` 走真源缺省（同其余已迁卡）；
//! 代码块头部的文件图标（真源 `CodeBlockHeader` 默认的 `FileDisplayIcon`）在
//! `RichCodeBlock` 的 header 模型里没有对应槽位，这里只给语言名——同 mcp.rs 的既有处理。

use leptos::prelude::*;
use serde_json::Value;

use super::get_workflow_run_roster::WorkflowRunSubagentRoster;
use super::get_workflow_run_situation::{WorkflowRunHealthLine, WorkflowRunPhaseTrack};
use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::toolResultDisplay::{
    GetWorkflowRunDisplay, ToolResultDisplay, read_tool_result_display,
};
use super::super::i18n;
use super::codeBlock::RichCodeBlock;
use crate::components::workflow_graph::run_state::WorkflowRunStatus;
use crate::components::workflow_graph::run_status_presentation::{
    read_workflow_run_stop_reason, run_status_text_class, workflow_run_stop_reason_message_id,
};
use crate::components::workflowIcons::icon_gauge;
use crate::lib::workflowObservationFormat::{format_workflow_age, format_workflow_token_count};

/// 真源 :22 —— `Gauge className="size-4 shrink-0 text-foreground-subtle"`。
pub const GET_RUN_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 真源 :32 —— 散文兜底（`toolCall.output` trim 后）。
pub fn read_fallback(output: Option<&str>) -> Option<String> {
    output.map(|text| text.trim().to_string())
}

/// 真源 :31 —— 只有「不在跑、没失败、display 是 get_workflow_run」三者同时成立才用 display。
pub fn read_run(
    raw: &Value,
    running: bool,
    failed: bool,
) -> Option<GetWorkflowRunDisplay> {
    if running || failed {
        return None;
    }
    match read_tool_result_display(raw) {
        Some(ToolResultDisplay::GetWorkflowRun(d)) => Some(d),
        _ => None,
    }
}

/// 真源 :126-127 —— 终态 = completed / errored / stopped。
pub fn is_terminal_status(status: WorkflowRunStatus) -> bool {
    matches!(
        status,
        WorkflowRunStatus::Completed | WorkflowRunStatus::Errored | WorkflowRunStatus::Stopped
    )
}

/// 真源 :34 —— `context.errorText || fallback || failureLabel`。
///
/// ★用的是 `||` 不是 `??`：空串的 fallback 也会被跳过。
pub fn failure_text_chain(
    error_text: Option<&str>,
    fallback: Option<&str>,
    failure_label: &str,
) -> String {
    if error_text.is_some_and(|s| !s.is_empty()) {
        return error_text.unwrap().to_string();
    }
    if fallback.is_some_and(|s| !s.is_empty()) {
        return fallback.unwrap().to_string();
    }
    failure_label.to_string()
}

/// 真源 :68 —— 展开门。
pub fn has_details(
    running: bool,
    failed: bool,
    run_present: bool,
    fallback: Option<&str>,
) -> bool {
    !running && (failed || run_present || fallback.is_some_and(|text| !text.is_empty()))
}

/// `JSON.parse` 出对象/数组才给 pretty 文本（真源 :133-141）；
/// 普通文本结果沿用正文排版，解析失败不抛错。
pub fn result_as_json(result: Option<&str>) -> Option<String> {
    let parsed: Value = serde_json::from_str(result?).ok()?;
    if !parsed.is_object() && !parsed.is_array() {
        return None;
    }
    serde_json::to_string_pretty(&parsed).ok()
}

/// 真源 :142-154 —— 日志行：有事件时刻就前缀年龄，没有就只剩正文。
///
/// 情势上线前的 journal 没有这一列，而**一个编出来的「刚刚」比没有年龄更糟**；
/// 年龄一律对快照时刻 `generatedAt` 算。
pub fn log_lines(display: &GetWorkflowRunDisplay) -> String {
    display
        .log_tail
        .iter()
        .map(|entry| {
            let age = format_workflow_age(display.generated_at, entry.at);
            let prefix = match age {
                None => String::new(),
                Some(age) => format!(
                    "{}  ",
                    i18n::format("chat.toolCall.workflow.getRun.age", &[("age".to_string(), age)])
                ),
            };
            format!("{prefix}{}", entry.message)
        })
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 真源 :51-57 + :233-241 —— 「已结算 = 完成 + 失败」「总数 = 已排程」。
pub fn steps_label(nodes_completed: f64, nodes_failed: f64, nodes_observed: f64) -> String {
    i18n::format(
        "chat.toolCall.workflow.card.steps",
        &[
            (
                "done".to_string(),
                js_number_string(nodes_completed + nodes_failed),
            ),
            ("total".to_string(), js_number_string(nodes_observed)),
        ],
    )
}

/// `z.number()` 是浮点：整数值要写成 `5` 而不是 `5.0`（JS 的字符串形态）。
fn js_number_string(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// 折叠行的第二段（真源 :49-63）：有摘要用摘要，没有摘要用步数。
/// 返回 `None` 表示该走 `fallbackName`（工作流脚本）那条兜底文案。
#[derive(Debug, Clone, PartialEq)]
pub enum FoldTrailing {
    /// 摘要原样（CSS 截断，不切字符）。
    Summary(String),
    /// 老载荷没有摘要 → 画步数。
    Steps(String),
}

pub fn fold_trailing(display: &GetWorkflowRunDisplay) -> FoldTrailing {
    match &display.summary {
        Some(summary) if !summary.is_empty() => FoldTrailing::Summary(summary.clone()),
        _ => FoldTrailing::Steps(steps_label(
            display.usage.nodes_completed,
            display.usage.nodes_failed,
            display.usage.nodes_observed,
        )),
    }
}

/// `GetWorkflowRunToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct GetWorkflowRunBlockProps {
    pub tool_id: String,
    pub raw: Value,
    pub status: String,
    pub is_running: bool,
    pub error_text: Option<String>,
    pub output_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `GetWorkflowRunBody`（真源 :118-273）。
#[component]
fn GetWorkflowRunBody(display: GetWorkflowRunDisplay) -> impl IntoView {
    let terminal = is_terminal_status(display.status);
    let stop_reason = read_workflow_run_stop_reason(display.status, display.stop_reason.as_deref());
    let tokens = i18n::format(
        "chat.toolCall.workflow.run.usage.tokens",
        &[("tokens".to_string(), format_workflow_token_count(display.usage.spent_tokens))],
    );
    // ★以下每一件都在 view! 之前算好（children 先于属性求值）。
    let json_result = result_as_json(display.result.as_deref());
    let plain_result = display.result.clone();
    let logs = log_lines(&display);
    let status_word = i18n::text(&format!(
        "chat.toolCall.workflow.run.status.{}",
        display.status.as_str()
    ));
    let status_class = run_status_text_class(display.status);
    let stop_reason_word = stop_reason
        .map(|reason| i18n::text(&workflow_run_stop_reason_message_id(reason)));
    let running_nodes = i18n::format(
        "chat.toolCall.workflow.getRun.runningNodes",
        &[("count".to_string(), js_number_string(display.usage.nodes_running))],
    );
    let summary_lede = display
        .summary
        .clone()
        .filter(|summary| !summary.is_empty());
    let questions_unknown = matches!(display.health.as_ref(), Some(health) if !health.pending_questions_known);
    let error_payload = display.error.clone();
    let phases = display.phases.clone();
    let subagents = display.subagents.clone();
    let health = display.health.clone();
    let generated_at = display.generated_at;
    let show_logs = display.status != WorkflowRunStatus::Completed && !logs.is_empty();
    let possibly_interrupted = display.possibly_interrupted.unwrap_or(false);
    let truncated = display.truncated.unwrap_or(false);
    let completed = display.status == WorkflowRunStatus::Completed;

    view! {
        <div class="mb-2 min-w-0 space-y-2" data-testid="workflow-status-body">
            // 那句由工具装配好的摘要放在最前：它是整张卡的导语，下面的阶段轨、花名册和
            // 健康行都是它的展开。老载荷没有它，卡就从第一件事（结果 / 错误）开始（真源 :156-160）。
            {summary_lede.map(|summary| view! {
                <p class="break-words text-ui-base text-foreground" data-testid="workflow-run-summary">
                    {summary}
                </p>
            })}
            // 两处「把不知道说出口」之一：这个会话不持有这个 run，停驻的问题只活在提问进程的
            // 内存里，看不见不等于没有——沉默会被读成「没人在等回答」（真源 :166-169）。
            {questions_unknown.then(|| view! {
                <p class="break-words text-ui-sm text-warning" data-testid="workflow-run-questions-unknown">
                    {i18n::text("chat.toolCall.workflow.getRun.questionsUnknown")}
                </p>
            })}
            {error_payload.map(|error| view! {
                <p class="max-h-80 overflow-auto whitespace-pre-wrap break-words text-ui-base text-destructive">
                    {error.code}
                    {": "}
                    {error.message}
                </p>
            })}
            {(completed && plain_result.is_some()).then(|| {
                // 真源 :183-200 —— 结构化结果进代码块，普通文本结果走正文。
                match json_result.clone() {
                    Some(code) => view! {
                        <div class="max-h-80 overflow-auto border border-border bg-card">
                            <RichCodeBlock
                                class="border-border bg-card".to_string()
                                code=code
                                language="json".to_string()
                                label=Some("json".to_string())
                                copy_label=None
                                wrap_label=None
                                wrap_long_lines=true
                            />
                        </div>
                    }
                    .into_any(),
                    None => view! {
                        <p class="max-h-80 overflow-auto whitespace-pre-wrap break-words text-ui-base text-foreground">
                            {plain_result.clone().unwrap_or_default()}
                        </p>
                    }
                    .into_any(),
                }
            })}
            {phases.clone().map(|items| view! {
                <WorkflowRunPhaseTrack
                    phases=items
                    generated_at=generated_at
                    terminal=terminal
                />
            })}
            {subagents.clone().map(|items| view! {
                <WorkflowRunSubagentRoster
                    subagents=items
                    generated_at=generated_at
                />
            })}
            {health.clone().map(|item| view! {
                <WorkflowRunHealthLine
                    health=item
                    generated_at=generated_at
                    terminal=terminal
                />
            })}
            <div class="flex flex-wrap items-center gap-x-2 text-ui-sm text-foreground-subtlest">
                <span class=status_class>{status_word}</span>
                {stop_reason_word.map(|word| view! {
                    <span data-testid="workflow-run-stop-reason">{word}</span>
                })}
                <span aria-hidden="true">"·"</span>
                // 活着的 run 才报「几个在跑」：终态 run 的这个数已经是历史。
                {(!terminal).then(|| view! {
                    <>
                        <span>{steps_label(
                            display.usage.nodes_completed,
                            display.usage.nodes_failed,
                            display.usage.nodes_observed,
                        )}</span>
                        <span aria-hidden="true">"·"</span>
                        <span>{running_nodes.clone()}</span>
                        <span aria-hidden="true">"·"</span>
                    </>
                })}
                <span>{tokens}</span>
            </div>
            {show_logs.then(|| view! {
                <pre class="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-3 py-2 font-mono text-ui-sm text-foreground-subtle">
                    {logs.clone()}
                </pre>
            })}
            {possibly_interrupted.then(|| view! {
                <p class="text-ui-sm text-warning">
                    {i18n::text("chat.toolCall.workflow.getRun.interruptedHint")}
                </p>
            })}
            {truncated.then(|| view! {
                // 这张卡被裁掉的是花名册 / 阶段 / 日志的行，不是诊断——共用 create_workflow 的
                // 「省略了部分诊断」会说错是什么被省略了（真源 :266-267）。
                <p class="text-ui-xs text-foreground-subtle">
                    {i18n::text("chat.toolCall.workflow.getRun.truncated")}
                </p>
            })}
        </div>
    }
}

/// `GetWorkflowRunToolCallBlock`（真源 :24-116）。
#[component]
pub fn GetWorkflowRunToolCallBlock(props: GetWorkflowRunBlockProps) -> impl IntoView {
    let running = props.is_running;
    // 真源 :30 —— 查询错误与 run 执行失败是两个状态层级。
    let failed = !running && props.status == "failed";
    let display = read_run(&props.raw, running, failed);
    let fallback = read_fallback(props.output_text.as_deref());
    let failure_label = i18n::text("chat.toolCall.status.failed");
    let error = failure_text_chain(
        props.error_text.as_deref(),
        fallback.as_deref(),
        &failure_label,
    );
    let run_present = display.is_some();
    let has_details = has_details(running, failed, run_present, fallback.as_deref());

    // 折叠行的主文本（真源 :35-67）——三件套先算好。
    let fold_label = display
        .as_ref()
        .map(|d| d.label.clone())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| i18n::text("chat.toolCall.workflow.fallbackName"));
    let fold_trailing = display.as_ref().map(fold_trailing);
    let primary_text_view = run_present.then(|| {
        let label = fold_label.clone();
        let trailing = fold_trailing.clone();
        std::sync::Arc::new(move || {
            let tail = match trailing.clone() {
                Some(FoldTrailing::Summary(summary)) => view! {
                    <span class="min-w-0 truncate" data-testid="workflow-run-summary-line">
                        {summary}
                    </span>
                }
                .into_any(),
                Some(FoldTrailing::Steps(steps)) => view! {
                    <span class="shrink-0 tabular-nums">{steps}</span>
                }
                .into_any(),
                None => ().into_any(),
            };
            view! {
                <span class="inline-flex min-w-0 items-center gap-2">
                    <span aria-hidden="true">"·"</span>
                    <span class="min-w-0 truncate">{label.clone()}</span>
                    <span aria-hidden="true">"·"</span>
                    {tail}
                </span>
            }
            .into_any()
        }) as ChildrenFn
    });

    let body_display = display.clone();
    let body_text = if failed {
        error.clone()
    } else {
        fallback.clone().unwrap_or_default()
    };
    let render_content = has_details.then(|| {
        let display = body_display.clone();
        let text = body_text.clone();
        std::sync::Arc::new(move || match display.clone() {
            // 真源 :69-80 —— failed 或没有 display 时只有一段正文。
            None => view! {
                <p
                    data-testid="workflow-status-body"
                    class="mb-2 max-h-80 overflow-auto whitespace-pre-wrap break-words text-ui-base text-foreground-subtle"
                >
                    {text.clone()}
                </p>
            }
            .into_any(),
            Some(item) => view! { <GetWorkflowRunBody display=item /> }.into_any(),
        }) as ChildrenFn
    });

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                kind_label: Some(i18n::text(if running {
                    "chat.toolCall.workflow.getRun.fetching"
                } else {
                    "chat.toolCall.workflow.getRun.fetched"
                })),
                source_label: props.source_label.clone(),
                is_running: Some(running),
                show_failure_status: Some(failed),
                status_label: failed.then(|| failure_label.clone()),
                status_tooltip: failed.then(|| error.clone()),
                can_toggle: Some(has_details),
                force_open: Some(false),
                title: props.title.clone(),
                primary_text_view,
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! { <span class=GET_RUN_ICON_CLASS>{icon_gauge()}</span> }.into_any()
            }))
            render_content=render_content
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
    use crate::ToolCallBlocks::toolResultDisplay::{
        WorkflowRunLogTailEntry, WorkflowRunUsage,
    };
    use serde_json::json;

    fn display(status: WorkflowRunStatus) -> GetWorkflowRunDisplay {
        GetWorkflowRunDisplay {
            run_id: "run-1".into(),
            label: "把改动跑一遍".into(),
            status,
            stop_reason: None,
            possibly_interrupted: None,
            summary: None,
            generated_at: None,
            usage: WorkflowRunUsage {
                spent_tokens: 1_000.0,
                nodes_observed: 7.0,
                nodes_running: 2.0,
                nodes_completed: 4.0,
                nodes_failed: 1.0,
            },
            phases: None,
            subagents: None,
            health: None,
            actors: vec![],
            log_tail: vec![],
            result: None,
            error: None,
            truncated: None,
        }
    }

    #[test]
    fn terminal_is_exactly_the_three_settled_states() {
        assert!(is_terminal_status(WorkflowRunStatus::Completed));
        assert!(is_terminal_status(WorkflowRunStatus::Errored));
        assert!(is_terminal_status(WorkflowRunStatus::Stopped));
        assert!(!is_terminal_status(WorkflowRunStatus::Pending));
        assert!(!is_terminal_status(WorkflowRunStatus::Running));
    }

    /// 一份最小合法的 GetWorkflowRun display JSON（写 JSON 而不是序列化 struct：
    /// struct 不带 Serialize，真源的 display 本就是 wire 形态）。
    fn get_run_raw() -> Value {
        json!({ "result": { "display": {
            "kind": "get_workflow_run",
            "runId": "run-1",
            "label": "把改动跑一遍",
            "status": "running",
            "usage": {
                "spentTokens": 1000.0, "nodesObserved": 7.0, "nodesRunning": 2.0,
                "nodesCompleted": 4.0, "nodesFailed": 1.0,
            },
            "actors": [],
            "logTail": [],
        } } })
    }

    #[test]
    fn running_or_failed_query_drops_the_display() {
        // ★真源 :29-31 —— 错误查询不能继续展示旧 display。
        let raw = get_run_raw();
        assert!(read_run(&raw, true, false).is_none(), "还在跑就不读快照");
        assert!(read_run(&raw, false, true).is_none(), "查询失败就不读快照");
        let loaded = read_run(&raw, false, false).expect("不跑不失败才读 display");
        assert_eq!(loaded.run_id, "run-1");
        assert_eq!(loaded.status, WorkflowRunStatus::Running);
        // display 是别的 kind 也不读。
        assert!(
            read_run(
                &json!({"result": {"display": {"kind": "resume_workflow_run", "runId": "r"}}}),
                false,
                false
            )
            .is_none()
        );
    }

    #[test]
    fn fallback_output_is_trimmed_at_read_time() {
        // 真源 :32 —— `toolCall.output.trim()`。链条下游拿到的永远是 trim 后的值。
        assert_eq!(read_fallback(Some("  散文  ")).as_deref(), Some("散文"));
        assert_eq!(read_fallback(Some("   ")).as_deref(), Some(""));
        assert_eq!(read_fallback(None), None);
    }

    #[test]
    fn failure_text_chain_skips_empty_strings() {
        // 真源 :34 用的是 `||`（空串也跳），不是 `??`。
        assert_eq!(failure_text_chain(Some("宿主给的"), Some("散文"), "执行失败"), "宿主给的");
        assert_eq!(failure_text_chain(None, Some("散文"), "执行失败"), "散文");
        assert_eq!(failure_text_chain(Some(""), Some("散文"), "执行失败"), "散文");
        // 纯空白 output 被 read_fallback trim 成 "" 后同样被跳过。
        assert_eq!(
            failure_text_chain(None, read_fallback(Some("   ")).as_deref(), "执行失败"),
            "执行失败"
        );
        assert_eq!(failure_text_chain(None, None, "执行失败"), "执行失败");
    }

    #[test]
    fn details_gate_waits_for_the_query_to_finish() {
        // 真源 :68 —— 仍在跑时一条都不给展开。
        assert!(!has_details(true, false, true, Some("有内容")));
        assert!(has_details(false, true, false, None), "失败本身就有正文可看");
        assert!(has_details(false, false, true, None));
        assert!(has_details(false, false, false, Some("散文兜底")));
        // 空白 output trim 成 "" → `Boolean(fallback)` 为假（真源 :68 + :32）。
        assert!(!has_details(false, false, false, read_fallback(Some("   ")).as_deref()));
        assert!(!has_details(false, false, false, None));
    }

    #[test]
    fn result_json_only_for_structured_payloads() {
        // 真源 :133-141 —— 解析出对象/数组才走代码块。
        assert_eq!(
            result_as_json(Some("{\"b\":2}")).as_deref(),
            Some("{\n  \"b\": 2\n}")
        );
        assert_eq!(result_as_json(Some("[1,2]")).as_deref(), Some("[\n  1,\n  2\n]"));
        assert_eq!(result_as_json(Some("\"just a sentence\"")), None, "字符串字面量不是结构化载荷");
        assert_eq!(result_as_json(Some("42")), None);
        assert_eq!(result_as_json(Some("普通文本")), None);
        assert_eq!(result_as_json(None), None);
    }

    #[test]
    fn steps_add_completed_and_failed_over_observed() {
        // 真源 :54-56 + :237-239 —— 已结算 = completed + failed，总数 = 已排程。
        assert_eq!(steps_label(4.0, 1.0, 7.0), "5/7 步");
        assert_eq!(steps_label(0.0, 0.0, 0.0), "0/0 步");
        // 整数形态：f64 的 4.0 要写成 4 而不是 4.0。
        assert!(!steps_label(4.0, 0.0, 7.0).contains(".0"));
    }

    #[test]
    fn fold_line_prefers_the_summary() {
        // ★真源 :44-48 —— 有摘要就让摘要占住折叠行，截断交给 CSS。
        let mut d = display(WorkflowRunStatus::Running);
        assert_eq!(
            fold_trailing(&d),
            FoldTrailing::Steps("5/7 步".to_string()),
            "老载荷没有摘要仍按步数画"
        );
        d.summary = Some("在第 2 / 4 个阶段、5 步已结算、2 个在跑".into());
        assert_eq!(
            fold_trailing(&d),
            FoldTrailing::Summary("在第 2 / 4 个阶段、5 步已结算、2 个在跑".into())
        );
        // 空串摘要不算「有摘要」（真源 :49 的 `.length === 0`）。
        d.summary = Some(String::new());
        assert!(matches!(fold_trailing(&d), FoldTrailing::Steps(_)));
    }

    #[test]
    fn log_lines_prefix_ages_against_the_snapshot() {
        let mut d = display(WorkflowRunStatus::Running);
        d.generated_at = Some(60_000.0);
        d.log_tail = vec![
            WorkflowRunLogTailEntry {
                sequence: 1.0,
                message: "node started".into(),
                at: Some(20_000.0),
            },
            WorkflowRunLogTailEntry {
                sequence: 2.0,
                message: "no clock".into(),
                at: None,
            },
            WorkflowRunLogTailEntry {
                sequence: 3.0,
                message: String::new(),
                at: Some(0.0),
            },
        ];
        let text = log_lines(&d);
        // ★没有时刻就只剩正文——一个编出来的「刚刚」比没有年龄更糟。
        // 第三条正文是空串，但它带着年龄前缀，`line.trim()` 仍非空 → 保留
        // （真源 :153 滤的是整行不是正文，这里逐字照做）。
        assert_eq!(text, "40s 前  node started\nno clock\n1m 00s 前  ");
        // 没有快照时刻时一行前缀都不加，此时空白正文的行才真的被滤掉。
        d.generated_at = None;
        assert_eq!(log_lines(&d), "node started\nno clock");
    }

    #[test]
    fn empty_run_id_usage_defaults_render_the_zero_steps() {
        // 缺 usage 的载荷根本进不来（strict 解析已拒），这里锁住「有 usage 时数字形态」。
        let d = display(WorkflowRunStatus::Pending);
        assert_eq!(
            i18n::format(
                "chat.toolCall.workflow.getRun.runningNodes",
                &[("count".to_string(), js_number_string(d.usage.nodes_running))]
            ),
            "2 个节点运行中"
        );
        assert_eq!(js_number_string(1_000.0), "1000");
        assert_eq!(js_number_string(1.5), "1.5");
    }

    #[test]
    fn icon_class_matches_source() {
        assert_eq!(GET_RUN_ICON_CLASS, "size-4 flex-none text-foreground-subtle");
    }
}
