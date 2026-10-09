//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/resume-workflow-run.tsx`（175 行）。
//!
//! ResumeWorkflowRun 的聊天卡。两态（真源 :26-37）：
//! - **run 已联接**（宿主按 display.runId 联接 workflowRuns 投影，`workflowRunCardJoin` 的
//!   byRunId 表）→ 复用 CreateWorkflow 的紧凑可点卡（`WorkflowRunCompactCard`），标签换
//!   「工作流实例已恢复」，状态点词/步数实时驱动，整卡点击打开侧栏 run 视图——tab 身份
//!   runId 键，与原始 create 卡打开的是同一个 tab。
//! - **未联接**（display 缺席的老会话 / 失败路径 / 投影尚未就绪）→ ToolLayout 折叠卡：
//!   runId（mono）+「后台运行中」状态点词 + 展开的本地化续跑说明；无 display 时有界纯文本
//!   面板，绝不 raw JSON dump。
//!
//! ★Rust 侧宿主还没给出第一态的数据源：`workflowRuns` 投影与侧栏 run 视图都还没有宿主表面
//! （见 `ToolCallBlock.rs` 文件头的裁剪注明），`context.workflow_run` 恒 None，实际渲染走
//! 第二态。第一态的代码照真源写全并留单测，接上联接即生效——**不是删掉的分支**。
//!
//! **裁剪注明**：`canToggle/forceOpen ?? 默认` 走真源默认（同 escalate / plan_guidance）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::toolResultDisplay::{ResumeWorkflowRunDisplay, ToolResultDisplay, read_tool_result_display};
use super::super::i18n;
use super::workflow_run_compact_card::WorkflowRunCompactCardComponent;
use crate::ToolCallBlocks::ToolCallBlock::WorkflowRunOpenRequest;
use crate::ToolCallBlocks::fileSummaryTypes::WorkflowRunCardSummary;
use crate::components::workflowIcons::{icon_rotate_ccw, icon_workflow};

/// 真源 :16-18 —— 紧凑 run 态卡的图标与 CreateWorkflow 同一枚（族内一致）；
/// 折叠态保留 RotateCcw 讲「恢复」（:19-21）。
pub const RESUME_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 无 display 时的纯文本面板高度（照 get-workflow-run 的输出面板量级，真源 :24）。
pub const FALLBACK_OUTPUT_MAX_HEIGHT_CLASS: &str = "max-h-60";

/// 真源 :42-43 —— display 只有是 `resume_workflow_run` 时才算数。
pub fn read_resume_display(raw: &Value) -> Option<ResumeWorkflowRunDisplay> {
    match read_tool_result_display(raw) {
        Some(ToolResultDisplay::ResumeWorkflowRun(d)) => Some(d),
        _ => None,
    }
}

/// 真源 :46-49 的种类词二选一。
pub fn resume_kind_label_id(is_running: bool) -> &'static str {
    if is_running {
        "chat.toolCall.workflow.resumeRun.resuming"
    } else {
        "chat.toolCall.workflow.resumeRun.label"
    }
}

/// 真源 :82 —— 紧凑态的主文本：display 带的 runId 优先，退回联接摘要的 runId。
pub fn resume_primary_run_id(
    display: Option<&ResumeWorkflowRunDisplay>,
    workflow_run: Option<&WorkflowRunCardSummary>,
) -> Option<String> {
    display
        .map(|d| d.run_id.clone())
        .or_else(|| workflow_run.map(|r| r.run_id.clone()))
}

/// 真源 :127-149 —— 折叠态有没有展开内容：有 display，或 output 是可读的有界散文。
pub fn resume_has_details(display_present: bool, output: Option<&str>) -> bool {
    display_present || output.is_some_and(|text| !text.trim().is_empty())
}

/// `ResumeWorkflowRunToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct ResumeBlockProps {
    pub tool_id: String,
    pub raw: Value,
    pub status: String,
    pub is_running: bool,
    /// legacy 行的散文输出（真源 `toolCall.output`）——无 display 时的有界文本面板用它。
    pub output_text: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    /// run 联接摘要（真源 `context.workflowRun`）。
    pub workflow_run: Option<WorkflowRunCardSummary>,
    /// 打开侧栏 run 视图的通道（真源 `context.onOpenWorkflowRun`）。
    pub on_open_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
}

/// `ResumeWorkflowRunToolCallBlock`（真源 :38-174）。
#[component]
pub fn ResumeWorkflowRunToolCallBlock(props: ResumeBlockProps) -> impl IntoView {
    let display = read_resume_display(&props.raw);
    let kind_label = i18n::text(resume_kind_label_id(props.is_running));

    // 快照提示在两条分支里都要挂，故做成可重复调用的闭包而不是一个被 move 走的视图值
    // （AnyView 不可 Clone，见工程纪律）。
    let notice_refs = props.snapshot_refs.clone();
    let notice_tool_id = props.tool_id.clone();
    let notice_loader = props.on_load_full_tool_call_fields.clone();
    let snapshot_notice = move || {
        view! {
            <ToolSnapshotFieldNoticeComponent
                props=ToolSnapshotFieldNoticeProps {
                    refs: notice_refs.clone(),
                    tool_id: notice_tool_id.clone(),
                    on_load_full_tool_call_fields: notice_loader.clone(),
                }
            />
        }
        .into_any()
    };

    // ── ① run 态：紧凑可点卡（真源 :69-92）──
    if let Some(workflow_run) = props.workflow_run.clone() {
        // 真源 :70-76 —— 状态词按 run 状态词汇表取，步数是「已排程里结算了几个」。
        let status_label = i18n::text(&format!(
            "chat.toolCall.workflow.run.status.{}",
            workflow_run.status
        ));
        let steps_label = i18n::format(
            "chat.toolCall.workflow.card.steps",
            &[
                ("done".to_string(), workflow_run.nodes_settled.to_string()),
                ("total".to_string(), workflow_run.nodes_total.to_string()),
            ],
        );
        let primary_text =
            resume_primary_run_id(display.as_ref(), Some(&workflow_run)).unwrap_or_default();
        let open_callback = props
            .on_open_workflow_run
            .clone()
            .map(StoredValue::new);

        return view! {
            <WorkflowRunCompactCardComponent
                aria_label=kind_label.clone()
                icon_view=Some(std::sync::Arc::new(|| {
                    view! { <span class=RESUME_TOOL_ICON_CLASS>{icon_workflow()}</span> }.into_any()
                }) as ChildrenFn)
                label_text=kind_label
                primary_text=primary_text
                primary_title=None
                workflow_run=workflow_run
                status_label=status_label
                steps_label=steps_label
                // 真源 :86 —— 宿主没给通道时 onOpen 是 undefined，卡片退回纯展示态。
                // 真源的 `() => onOpenWorkflowRun({})`：请求体在调用点烤进闭包，
                // 恢复卡给的是空请求（workflowName / phaseId 都不带）。
                on_open=open_callback.map(|stored| {
                    Callback::from(move || {
                        stored.get_value().run(WorkflowRunOpenRequest::default())
                    })
                })
                show_icon=props.show_icon
            >
                {snapshot_notice()}
            </WorkflowRunCompactCardComponent>
        }
        .into_any();
    }

    // ── ② 折叠态：ToolLayout + 状态点词 + 展开说明（真源 :94-173）──
    let run_id = display.as_ref().map(|d| d.run_id.clone());
    let display_present = display.is_some();
    // 真源 :127-131 —— 无 display 的老会话 / 失败路径：formatModelContent 的文本投影
    // 也是有界信息，直接给面板，不做 JSON dump。
    let fallback_text = props
        .output_text
        .clone()
        .filter(|text| !text.trim().is_empty());
    let has_details = resume_has_details(display_present, props.output_text.as_deref());

    let background_label = i18n::text("chat.toolCall.workflow.resumeRun.inBackground");
    let hint_label = i18n::text("chat.toolCall.workflow.resumeRun.hint");

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(has_details),
                force_open: Some(false),
                kind_label: Some(kind_label),
                source_label: props.source_label.clone(),
                title: props.title.clone(),
                // ★真源 :107-121 —— 状态词永远在圆点旁边：状态绝不只靠颜色或动画表达。
                // display 只在成功输出上构造，失败路径走 showFailureStatus + 文本面板。
                show_status_label: Some(display_present),
                status_label_view: display_present.then(|| {
                    let text = background_label.clone();
                    std::sync::Arc::new(move || {
                        view! {
                            <span class="flex shrink-0 items-center gap-1.5">
                                <span
                                    aria-hidden="true"
                                    class="size-1.5 rounded-full animate-pulse bg-warning motion-reduce:animate-none"
                                />
                                <span class="text-ui-sm text-warning">{text.clone()}</span>
                            </span>
                        }
                        .into_any()
                    }) as ChildrenFn
                }),
                status_tooltip: props.error_text.clone(),
                show_failure_status: Some(props.status == "failed"),
                is_running: Some(props.is_running),
                // 真源 :97-105 —— primaryText 只在 runId 在场时存在，且是 mono 节点。
                primary_text_view: run_id.clone().map(|id| {
                    std::sync::Arc::new(move || {
                        view! {
                            <span class="min-w-0 truncate font-mono text-foreground-subtlest" title=id.clone()>
                                {id.clone()}
                            </span>
                        }
                        .into_any()
                    }) as ChildrenFn
                }),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! { <span class=RESUME_TOOL_ICON_CLASS>{icon_rotate_ccw()}</span> }.into_any()
            }))
            render_content=has_details.then(|| {
                let hint = hint_label.clone();
                let panel = fallback_text.clone();
                std::sync::Arc::new(move || {
                    // 真源 :124-144 —— 有 display 时给续跑说明；无 display 时给有界文本面板。
                    if display_present {
                        view! {
                            <p class="mb-2 rounded-lg border border-border bg-panel px-4 py-3 text-ui-sm leading-5 text-foreground-subtle">
                                {hint.clone()}
                            </p>
                        }
                        .into_any()
                    } else {
                        let text = panel.clone().unwrap_or_default();
                        view! {
                            <pre class=format!("{FALLBACK_OUTPUT_MAX_HEIGHT_CLASS} mb-2 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-panel px-4 py-3 font-mono text-ui-base text-foreground-subtle")>
                                {text}
                            </pre>
                        }
                        .into_any()
                    }
                }) as ChildrenFn
            })
        />
        {snapshot_notice()}
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn display_value(run_id: &str) -> Value {
        json!({ "result": { "display": {
            "kind": "resume_workflow_run", "runId": run_id,
        } } })
    }

    fn summary(run_id: &str, status: &str) -> WorkflowRunCardSummary {
        WorkflowRunCardSummary {
            run_id: run_id.into(),
            tool_call_id: None,
            status: status.into(),
            stop_reason: None,
            nodes_settled: 2,
            nodes_total: 5,
            agents: None,
            run: None,
            resumable: false,
        }
    }

    #[test]
    fn display_only_when_the_kind_matches() {
        assert_eq!(
            read_resume_display(&display_value("run-7")),
            Some(ResumeWorkflowRunDisplay { run_id: "run-7".into() })
        );
        // 别的 display kind 不算（真源 :43 的 `display?.kind === "resume_workflow_run"`）。
        let other = json!({ "result": { "display": { "kind": "get_workflow_run" } } });
        assert_eq!(read_resume_display(&other), None);
        // display 缺席（老会话 / 失败路径）→ None。
        assert_eq!(read_resume_display(&json!({})), None);
    }

    #[test]
    fn kind_label_switches_on_running() {
        // 真源 :46-49。
        assert_eq!(resume_kind_label_id(true), "chat.toolCall.workflow.resumeRun.resuming");
        assert_eq!(resume_kind_label_id(false), "chat.toolCall.workflow.resumeRun.label");
    }

    #[test]
    fn display_run_id_wins_over_joined_run_id() {
        // 真源 :82 —— `runDisplay?.runId ?? context.workflowRun.runId`。
        let display = read_resume_display(&display_value("run-from-display"));
        assert_eq!(
            resume_primary_run_id(display.as_ref(), Some(&summary("run-from-projection", "running")))
                .as_deref(),
            Some("run-from-display")
        );
        // display 缺席时退回联接摘要的 runId。
        assert_eq!(
            resume_primary_run_id(None, Some(&summary("run-from-projection", "running")))
                .as_deref(),
            Some("run-from-projection")
        );
        // 两者都没有 → 紧凑卡没有可读的主文本（真源这里也是空串）。
        assert_eq!(resume_primary_run_id(None, None), None);
    }

    #[test]
    fn details_gate_covers_both_branches() {
        // 真源 :147-149 —— 有 display，或 output 是**非空白**散文。
        assert!(resume_has_details(true, None));
        assert!(resume_has_details(false, Some("跑起来了，稍后回来看")));
        assert!(!resume_has_details(false, Some("   \n ")), "空白不算有内容");
        assert!(!resume_has_details(false, None));
    }

    #[test]
    fn joined_branch_reads_the_five_run_status_words() {
        // 真源 :70-72 —— 紧凑态的状态词按 `chat.toolCall.workflow.run.status.${status}` 取，
        // 五个态都在文案表里（漏一个界面会显示原始 id）。
        for status in ["pending", "running", "completed", "errored", "stopped"] {
            let id = format!("chat.toolCall.workflow.run.status.{status}");
            let text = i18n::text(&id);
            assert_ne!(text, id, "{id} 应在文案表里");
            assert!(!text.is_empty());
        }
        // 步数插值（真源 :73-76）：done = 已结算、total = 已排程。
        let steps = i18n::format(
            "chat.toolCall.workflow.card.steps",
            &[
                ("done".to_string(), "2".to_string()),
                ("total".to_string(), "5".to_string()),
            ],
        );
        assert_eq!(steps, "2/5 步");
    }

    #[test]
    fn constants_match_source() {
        // 真源 :24 —— 有界纯文本面板，绝不 raw JSON dump。
        assert_eq!(FALLBACK_OUTPUT_MAX_HEIGHT_CLASS, "max-h-60");
        assert_eq!(RESUME_TOOL_ICON_CLASS, "size-4 flex-none text-foreground-subtle");
    }
}
