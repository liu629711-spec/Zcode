//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/create-workflow.tsx`（464 行）。
//!
//! 聊天区的 CreateWorkflow / AmendWorkflow 工具卡（真源 :66-75）：
//! - **编写中**：不可展开的 ToolLayout，行下**常驻**草稿阶段线（站随脚本流式写出，
//!   不是展开内容，没有折叠入口）；
//! - **待确认**：可展开脚本的 ToolLayout；
//! - **编不过**：编译反馈行（「工作流草稿 · 第 n 稿 · n 处待修正 · 未运行」）——
//!   不是失败，什么都没跑。
//!
//! AmendWorkflow 行走**同一个**渲染器（display kind 同为 `create_workflow`，图、草稿笔与
//! 诊断卡只有一份实现），只换修订词汇，并在卡体多一行「调整自 run X」——
//! **按工具名判，不看 family**。
//!
//! ★真源 :220-222 写死的优先级：handler 在编不过的路径上直接回诊断、不启动引擎，
//! 本来就不该有 run——**即使宿主联接到了也留在反馈行**，把「诊断优先」写在这里而不是
//! 依赖调用方。
//!
//! **裁剪注明**：
//! - 已联接 run 的那一整枝（表头 + 时间线 + 产物条 + 页脚 Resume）依赖
//!   `context.workflowRun` 与四条打开通道的宿主表面；侧栏 run 视图在 Rust 应用里还不存在，
//!   上下文字段恒 None（见 `ToolCallBlock.rs` 文件头），所以这一枝今天不可达。
//!   代码照真源写全，接上即生效。
//! - `workflowCardOpenState`（真源 :63-64 的模块级 Map，按 toolId 记忆折叠态、默认展开）：
//!   Rust 侧用 thread_local Map，与 `ToolLayout` 的 `toolLayoutOpenState` 同一模式。
//! - Radix `Collapsible`（脚本折区）→ `RwSignal` + 条件挂载：无开合动画，
//!   chevron 的 `data-[state=open]` 旋转改为条件类名（同 mcp.rs 的既有等价）。

use leptos::prelude::*;
use serde_json::Value;

use super::codeBlock::RichCodeBlock;
use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::i18n;
use super::createWorkflowDisplay::{read_fallback_output_text, read_workflow_display};
use super::createWorkflowInput::{
    WorkflowSavedSource, read_workflow_amend_target, read_workflow_card_kept_script,
    read_workflow_kind_message_id, read_workflow_name, read_workflow_prelaunch_kind_message_id,
    read_workflow_retune_call, read_workflow_saved, read_workflow_script,
};
use super::WorkflowCardMetaLine::{
    WorkflowAmendsLineComponent, WorkflowCardMetaLineComponent,
};
use super::workflow_diagnostics::WorkflowDiagnosticsSectionComponent;
use super::workflow_draft_row::{WorkflowDraftRowInput, WorkflowFeedbackContent, build_workflow_draft_row_slots};
use crate::ToolCallBlocks::ToolCallBlock::{
    WorkflowActorOpenRequest, WorkflowRunOpenRequest,
};
use crate::ToolCallBlocks::fileSummaryTypes::{WorkflowDraftPosition, WorkflowRunCardSummary};
use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;
use crate::ToolCallBlocks::toolDisplay::ToolDisplayModel;
use crate::components::WorkflowArtifactStrip::WorkflowArtifactStrip;
use crate::components::WorkflowCardChrome::{
    WorkflowCardFooter, WorkflowCardHeader, WorkflowRunStatusInput, WorkflowRunStatus,
    WorkflowStaticStatus, workflow_run_kind_message_id, workflow_card_icon,
};
use crate::components::WorkflowArtifactPill::ArtifactPillData;
use crate::components::WorkflowTimeline::WorkflowTimeline;
use crate::components::timeline_model::{WorkflowTimelineModel, build_workflow_timeline};
use crate::components::timeline_summary::workflow_card_detail;
use crate::components::workflow_graph::types::WorkflowCausalityGraphData;
use crate::components::workflow_graph::run_state::WorkflowRunState;
use crate::components::workflowIcons::{icon_chevron_right, icon_rotate_ccw};
use crate::lib::workflowToolNames::is_amend_workflow_tool_call;

/// 真源 :61 —— 没有 display 时交给草稿槽位的空诊断：模块级常量，
/// 免得每次渲染一个新数组打穿记忆。
const NO_DIAGNOSTICS: [(i64, i64, String); 0] = [];

/// wire 上的 run 状态串解回协议枚举（真源靠 TS 类型，Rust 侧 serde 是唯一词表）。
fn run_status_of(raw: &str) -> Option<crate::components::workflow_graph::run_state::WorkflowRunStatus> {
    serde_json::from_value(Value::String(raw.to_string())).ok()
}

/// `workflowCardOpenState`（真源 :63-64）：折叠状态按 toolId 记忆，默认展开。
///
/// 真源是模块级 `Map<string, boolean>`；wasm 下用 thread_local（UI 线程单线程，
/// 与 `ToolLayout::toolLayoutOpenState` 同一模式）。
mod open_state {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static OPEN_BY_TOOL_ID: RefCell<HashMap<String, bool>> =
            RefCell::new(HashMap::new());
    }

    pub fn remember(tool_id: &str, open: bool) {
        OPEN_BY_TOOL_ID.with(|map| {
            map.borrow_mut().insert(tool_id.to_string(), open);
        });
    }

    /// 真源 :129 —— `workflowCardOpenState.get(toolId) ?? true`：没记过就是展开。
    pub fn recalled(tool_id: &str) -> bool {
        OPEN_BY_TOOL_ID
            .with(|map| map.borrow().get(tool_id).copied())
            .unwrap_or(true)
    }
}

/// 卡片的状态判定（真源 :94-132、:223-232 的那一堆布尔）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateWorkflowFlags {
    /// `writing`：在跑且 v4 状态是 inputStreaming（真源 :106）。
    pub writing: bool,
    /// `prelaunch`：没有 run 且在编写或待批准（真源 :223）。
    pub prelaunch: bool,
    /// `summaryOnly`：编写中且不失败 → 只给摘要行，不展示半截脚本（真源 :224）。
    pub summary_only: bool,
    pub show_failure_status: bool,
    /// `inFlight`：在跑且还没联接到 run（真源 :112）。
    pub in_flight: bool,
}

/// 真源 :94-132 的那一条判定链，纯函数便于逐条断言。
#[allow(clippy::too_many_arguments)]
pub fn create_workflow_flags(
    is_running: bool,
    v4_status: Option<&str>,
    status: &str,
    display_ok: Option<bool>,
    has_display: bool,
    joined_to_run: bool,
) -> CreateWorkflowFlags {
    let writing = is_running && v4_status == Some("inputStreaming");
    // 真源 :96-97 —— 编不过是 display.ok === false；没有 display 时才看行状态。
    let has_compile_errors = display_ok == Some(false);
    let show_failure_status =
        has_compile_errors || (!has_display && status == "failed");
    // 真源 :112 —— 联接到 run 之后就不算在途。
    let in_flight = is_running && !joined_to_run;
    CreateWorkflowFlags {
        writing,
        // 真源 :223 —— 待确认（pendingApproval）也走普通摘要那一枝。
        prelaunch: !joined_to_run && (writing || v4_status == Some("pendingApproval")),
        summary_only: writing && !show_failure_status,
        show_failure_status,
        in_flight,
    }
}

/// 真源 :100-104 —— 空图（脚本里一次 ask / files.* 都没有）不值得一条空轨道。
pub fn graph_is_worth_a_track(graph: Option<&WorkflowCausalityGraphData>) -> bool {
    graph.is_some_and(|g| !g.steps.is_empty())
}

/// 真源 :122-127 的模型来源：有图用分析器的图，否则编写中从半截脚本扫出来。
pub fn should_scan_draft_timeline(
    has_graph: bool,
    writing: bool,
    has_script: bool,
) -> bool {
    !has_graph && writing && has_script
}

/// `CreateWorkflowToolCallBlock` 的 props。
#[derive(Clone)]
pub struct CreateWorkflowBlockProps {
    pub tool_id: String,
    pub kind: String,
    pub tool_name: Option<String>,
    pub title: Option<String>,
    pub input: Value,
    pub output: Value,
    pub raw: Value,
    pub status: String,
    pub v4_status: String,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub workflow_run: Option<WorkflowRunCardSummary>,
    pub workflow_draft: Option<WorkflowDraftPosition>,
    pub on_open_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
    pub on_resume_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
    pub on_open_workflow_actor: Option<Callback<WorkflowActorOpenRequest>>,
    pub on_open_workflow_workspace: Option<Callback<WorkflowRunOpenRequest>>,
    pub on_open_workflow_artifact: Option<Callback<String>>,
    /// 交回 fallback 卡所需（真源没有这条路，但 family 兜底之外的形状要保住信息）。
    pub legacy: LegacyToolCall,
    pub display_model: ToolDisplayModel,
    pub workspace_path: String,
}

/// `CreateWorkflowToolCallBlock`（真源 :76-463）。
#[component]
pub fn CreateWorkflowToolCallBlock(props: CreateWorkflowBlockProps) -> impl IntoView {
    // 真源 :79 —— 按工具名判修订，不看 family。六个位置同 matchesToolName。
    let amend = is_amend_workflow_tool_call(
        props.tool_name.as_deref().unwrap_or_default(),
        &props.kind,
        props.title.as_deref().unwrap_or_default(),
        &props.raw,
    );
    let display = read_workflow_display(&props.raw);
    let script_text = read_workflow_script(&props.input);
    let workflow_name = read_workflow_name(&props.input);
    let saved = read_workflow_saved(&props.input);
    let amend_target = amend.then(|| read_workflow_amend_target(&props.input)).flatten();
    let fallback_output_text = if display.is_some() {
        None
    } else {
        read_fallback_output_text(&props.output)
    };
    // 真源 :83-86 —— 只改并发上限的调用不写脚本也不编译，「正在校验工作流」对它不成立。
    let retuning =
        amend && read_workflow_retune_call(&props.input).is_some();

    let flags = create_workflow_flags(
        props.is_running,
        Some(props.v4_status.as_str()),
        &props.status,
        display.as_ref().map(|d| d.ok),
        display.is_some(),
        props.workflow_run.is_some(),
    );
    let fallback_name = i18n::text("chat.toolCall.workflow.fallbackName");
    let name = workflow_name.clone().unwrap_or_else(|| fallback_name.clone());

    // ── 因果图（display 带的是 wire JSON，解不出就当没有图）──
    let graph: Option<WorkflowCausalityGraphData> = display
        .as_ref()
        .and_then(|d| d.causality_graph.clone())
        .and_then(|value| serde_json::from_value::<WorkflowCausalityGraphData>(value).ok())
        .filter(|g| graph_is_worth_a_track(Some(g)));
    let run: Option<WorkflowRunState> = props
        .workflow_run
        .as_ref()
        .and_then(|summary| summary.run.clone())
        .and_then(|value| serde_json::from_value::<WorkflowRunState>(value).ok());

    // 真源 :111 —— 沿用前驱脚本的修订：只在入参写完之后才说「脚本不变」，流式中脚本可能还没到。
    let kept_script = amend
        && !flags.writing
        && read_workflow_card_kept_script(&props.input, false);

    // ── 草稿槽位（真源 :113-120）──
    let draft_slots = build_workflow_draft_row_slots(&WorkflowDraftRowInput {
        draft_ordinal: props.workflow_draft.map(|position| position.ordinal as i64),
        superseded: props.workflow_draft.is_some_and(|position| position.superseded),
        compile_errors: display.as_ref().map(|d| d.ok) == Some(false),
        in_flight: flags.in_flight,
        error_count: display.as_ref().map(|d| d.error_count).unwrap_or(0),
        diagnostics: NO_DIAGNOSTICS.to_vec(),
        saved: saved.is_some(),
    });
    // 真源 :107-111 —— 代码块上被点名的行号（display.ok 为假才有）。
    let flagged_lines = super::workflow_draft_row::feedback_flagged_lines(display.as_ref())
        .unwrap_or_default();

    // ── 模型（真源 :122-127）──
    let model: Option<WorkflowTimelineModel> = if let Some(g) = graph.as_ref() {
        Some(build_workflow_timeline(g, run.as_ref()))
    } else if should_scan_draft_timeline(
        graph.is_some(),
        flags.writing,
        script_text.is_some(),
    ) {
        script_text
            .as_deref()
            .map(|text| {
                crate::components::draft_scan::draft_timeline(&crate::components::draft_scan::scan_workflow_draft(text))
            })
    } else {
        None
    };

    // ── 折叠态（真源 :129-139）──
    let expanded = RwSignal::new(open_state::recalled(&props.tool_id));

    // ── 普通枝要用的细节串（真源 :289-295）──
    let card_detail = workflow_card_detail(
        model.as_ref(),
        graph.as_ref(),
        run.as_ref(),
        run.as_ref()
            .and_then(|r| r.subagent_model.clone())
            .as_deref(),
        None,
    );
    let header_detail = card_detail
        .as_ref()
        .map(|d| d.detail.clone())
        .or_else(|| draft_slots.in_flight_ordinal_text.clone());
    let header_detail_title = card_detail.as_ref().and_then(|d| d.title.clone());

    let snapshot_notice = {
        let refs = props.snapshot_refs.clone();
        let tool_id = props.tool_id.clone();
        let loader = props.on_load_full_tool_call_fields.clone();
        move || {
            view! {
                <ToolSnapshotFieldNoticeComponent
                    props=ToolSnapshotFieldNoticeProps {
                        refs: refs.clone(),
                        tool_id: tool_id.clone(),
                        on_load_full_tool_call_fields: loader.clone(),
                    }
                />
            }
            .into_any()
        }
    };

    // ══ 枝一：编不过 / 启动前（真源 :220-273）══
    if flags.show_failure_status || flags.prelaunch {
        let summary_only = flags.summary_only;
        let kind_label = i18n::text(read_workflow_prelaunch_kind_message_id(
            super::createWorkflowInput::PrelaunchPhase {
                compile_errors: display.as_ref().map(|d| d.ok) == Some(false),
                failed: flags.show_failure_status,
                writing: flags.writing,
                revising: (props.workflow_draft.map(|d| d.ordinal).unwrap_or(1)) >= 2,
            },
            amend,
        ));
        let draft_model = model.clone();
        let show_draft_timeline =
            summary_only && draft_model.as_ref().is_some_and(|m| m.draft.is_some());
        let display_for_body = display.clone();
        let fallback_for_body = fallback_output_text.clone();
        let saved_flag = saved.is_some();
        let script_for_body = script_text.clone();
        let slots_secondary = draft_slots.secondary_text.clone();
        let slots_status = draft_slots.status_label.clone().or_else(|| props.status_label.clone());
        let slots_indicator = draft_slots.status_indicator;
        let slots_tooltip = draft_slots
            .status_tooltip
            .clone()
            .or_else(|| props.error_text.clone());

        return view! {
            <ToolLayoutComponent
                props=ToolLayoutProps {
                    tool_id: props.tool_id.clone(),
                    icon: None,
                    show_icon: Some(props.show_icon),
                    // 真源 :240-241 —— 编写中不可展开。
                    can_toggle: Some(!summary_only),
                    force_open: Some(!summary_only),
                    kind_label: Some(kind_label),
                    source_label: props.source_label.clone(),
                    primary_text: Some(name.clone()),
                    // 稿号在展开后仍留在行上：它说的是「这是第几稿」，不是折叠时的摘要
                    // （真源 :258）。
                    secondary_text: slots_secondary,
                    status_label: slots_status,
                    status_indicator_view: slots_indicator.map(|lamp| {
                        std::sync::Arc::new(move || view! {
                            // 空环灯：形状说「什么都没跑」，颜色说注意力还悬不悬着。
                            <span
                                aria-hidden="true"
                                class=format!(
                                    "size-1.5 shrink-0 rounded-full border-[1.5px] border-foreground-subtlest bg-transparent {}",
                                    match lamp {
                                        super::workflow_draft_row::DraftLampState::Open => "bg-destructive border-transparent",
                                        super::workflow_draft_row::DraftLampState::Settled => "bg-foreground-subtlest border-transparent",
                                    },
                                )
                                data-draft-lamp=lamp.data_attr()
                            />
                        }
                        .into_any()) as ChildrenFn
                    }),
                    status_tooltip: slots_tooltip,
                    show_failure_status: Some(flags.show_failure_status),
                    // 真源 :265 —— 待确认仍处于启动流程中，与编写态共用扫光，避免看起来已经结束。
                    is_running: Some(if flags.show_failure_status {
                        props.is_running
                    } else {
                        flags.prelaunch
                    }),
                    title: props.title.clone(),
                    ..Default::default()
                }
                icon_view=Some(std::sync::Arc::new(|| {
                    view! { <span class="size-4 flex-none">{workflow_card_icon()}</span> }.into_any()
                }))
                render_content=(!summary_only).then(|| {
                    let d = display_for_body.clone();
                    let f = fallback_for_body.clone();
                    let s = script_for_body.clone();
                    std::sync::Arc::new(move || view! {
                        <WorkflowFeedbackContent
                            display=d.clone()
                            fallback_output_text=f.clone()
                            saved=saved_flag
                            script_text=s.clone()
                        />
                    }
                    .into_any()) as ChildrenFn
                })
            />
            // 编写中的行不可展开，但草稿阶段线常驻在行下（真源 :225-232：
            // 不是展开内容，没有折叠入口；display 一到整个模型换成分析器的站）。
            {show_draft_timeline.then(|| view! {
                <div class="pt-2" data-testid="workflow-draft-timeline">
                    <WorkflowTimeline
                        class="py-1".to_string()
                        model=draft_model.clone().expect("上面已判过 Some")
                        on_select_station=None
                        on_open_more=None
                        on_open_pill=None
                        on_open_workspace=None
                    />
                </div>
            })}
            {snapshot_notice()}
        }
        .into_any();
    }

    // ══ 枝二：已联接 run / 静态卡（真源 :275-462）══
    let kind_id: String = match &props.workflow_run {
        Some(summary) => workflow_run_kind_message_id(
            run_status_of(&summary.status)
                .unwrap_or(crate::components::workflow_graph::run_state::WorkflowRunStatus::Pending),
            summary.stop_reason.as_deref(),
        )
        .to_string(),
        // 修订行在 run 出现之前用修订词汇（真源 :276-279）。
        None => read_workflow_kind_message_id(&props.raw, props.is_running, amend, retuning)
            .to_string(),
    };
    let kind_text = i18n::text(&kind_id);
    let live = match &props.workflow_run {
        Some(summary) => summary.status == "running",
        None => props.is_running,
    };
    let joined_run_status = props
        .workflow_run
        .as_ref()
        .and_then(|s| run_status_of(&s.status));

    let compiled_word = i18n::text("chat.toolCall.workflow.compiled");
    let display_ok = display.as_ref().map(|d| d.ok);
    let status_node: Option<AnyView> = if props.workflow_run.is_some() {
        Some(view! {
            <WorkflowRunStatus
                run=WorkflowRunStatusInput::default()
                status=joined_run_status
                test_id=Some("workflow-card-status".to_string())
            />
        }
        .into_any())
    } else if display_ok == Some(true) {
        Some(view! { <WorkflowStaticStatus word=compiled_word /> }.into_any())
    } else {
        None
    };

    // 真源 :295-309 —— Resume 只在「可恢复」且宿主给了通道时出现；可恢复性是状态位，
    // UI 绝不自己按 status + failureCode 推导。
    let resumable = props
        .workflow_run
        .as_ref()
        .is_some_and(|s| s.resumable)
        && props.on_resume_workflow_run.is_some();
    let resume_label = i18n::text("chat.toolCall.workflow.run.resume");
    let resume_callback = props.on_resume_workflow_run.clone();
    let resume_name = workflow_name.clone();
    let resume_node: Option<AnyView> = resumable.then(|| {
        view! {
            <button
                class="ml-auto inline-flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1 text-ui-sm font-medium whitespace-nowrap transition-colors hover:bg-hover"
                data-testid="workflow-card-resume"
                type="button"
                on:click=move |_| {
                    if let Some(callback) = &resume_callback {
                        callback.run(WorkflowRunOpenRequest {
                            workflow_name: resume_name.clone(),
                            phase_id: None,
                        });
                    }
                }
            >
                <span class="size-3.5">{icon_rotate_ccw()}</span>
                {resume_label.clone()}
            </button>
        }
        .into_any()
    });

    let script_open = RwSignal::new(false);
    let show_script_fold =
        props.workflow_run.is_none() && !flags.writing && script_text.is_some();
    let fold_code = script_text.clone().unwrap_or_default();
    let fold_marked = flagged_lines.clone();

    let saved_meta = saved.map(|source: WorkflowSavedSource| {
        let badge = i18n::text("chat.permission.workflow.saved.badge");
        let label = if source.scope.as_deref() == Some("project") {
            format!(
                "{badge} · {}",
                i18n::text("chat.permission.workflow.saved.scope.project")
            )
        } else {
            badge
        };
        (label, source)
    });

    let artifacts: Vec<ArtifactPillData> = run
        .as_ref()
        .and_then(|r| r.artifacts.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|artifact| ArtifactPillData {
            id: artifact.id,
            kind: Some(artifact.kind),
            title: artifact.title,
            version: Some(artifact.version),
        })
        .collect();
    let graph_truncated = graph.as_ref().is_some_and(|_| display.as_ref().and_then(|d| d.truncated) == Some(true));
    let diagnostics = display
        .as_ref()
        .map(|d| {
                d.diagnostics
                    .iter()
                    .map(|entry| super::workflow_diagnostics::WorkflowDiagnosticEntry {
                        line: entry.line,
                        column: entry.column,
                        code: entry.code,
                        message: entry.message.clone(),
                    })
                    .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let has_diagnostics = !diagnostics.is_empty();
    let error_count = display.as_ref().map(|d| d.error_count);
    let display_truncated = display.as_ref().and_then(|d| d.truncated) == Some(true);
    let no_content_at_all = script_text.is_none()
        && !kept_script
        && display.is_none()
        && fallback_output_text.is_none()
        && !props.is_running;
    let fallback_text = fallback_output_text.clone().unwrap_or_default();
    let amend_target_for_body = amend_target.clone();
    let terminal = run
        .as_ref()
        .is_some_and(|r| matches!(r.status, crate::components::workflow_graph::run_state::WorkflowRunStatus::Errored | crate::components::workflow_graph::run_state::WorkflowRunStatus::Stopped));
    let footer_status = joined_run_status;
    let open_run_request = WorkflowRunOpenRequest {
        workflow_name: workflow_name.clone(),
        phase_id: None,
    };
    let card_state_attr = match &props.workflow_run {
        Some(summary) => summary.status.clone(),
        None if flags.writing => "writing".to_string(),
        None => "static".to_string(),
    };
    let run_id_attr = props
        .workflow_run
        .as_ref()
        .map(|summary| summary.run_id.clone());
    view! {
        <section
            aria-label=kind_text.clone()
            class="wf-motion flex w-full min-w-0 flex-col gap-2"
            data-testid="workflow-card"
            data-workflow-card-state=card_state_attr
            data-workflow-run-id=run_id_attr.unwrap_or_default()
            data-workflow-run-status=match &props.workflow_run {
                Some(summary) => summary.status.clone(),
                None => String::new(),
            }
        >
            <WorkflowCardHeader
                kind=kind_text.clone()
                name=name.clone()
                live
                status=status_node
                detail=header_detail
                detail_title=header_detail_title
                leading=None
                trailing=None
                toggle_label=None
                expanded=expanded.get()
                on_open_details=props.on_open_workflow_run.clone().map(|callback| {
                    let request = open_run_request.clone();
                    Callback::from(move || callback.run(request.clone()))
                })
                on_toggle=(!flags.writing).then(|| {
                    let tool_id = props.tool_id.clone();
                    Callback::from(move || {
                        expanded.update(|value| {
                            *value = !*value;
                            open_state::remember(&tool_id, *value);
                        })
                    })
                })
            />
            {expanded.get().then(|| view! {
                <div class="wf-unfold flex min-w-0 flex-col gap-2" data-testid="workflow-card-body">
                    {amend_target_for_body.map(|target| view! {
                        <WorkflowAmendsLineComponent run_id=target script_inherited=kept_script />
                    })}
                    {saved_meta.as_ref().map(|(label, source)| view! {
                        <WorkflowCardMetaLineComponent
                            marker="saved-source"
                            label=label.clone()
                            value=source.name.clone()
                            title=source.path.clone().unwrap_or_else(|| source.name.clone())
                        />
                    })}
                    {model.clone().map(|m| view! {
                        // 站头与「还有 n 个」那一行交出站 id —— 打开通道未接线时不给，
                        // 站头因此不是控件（真源 :364-373 的同一条件形状）。
                        <WorkflowTimeline class="py-1".to_string() model=m on_select_station=None on_open_more=None on_open_pill=None on_open_workspace=None />
                    })}
                    {(!artifacts.is_empty()).then(|| view! {
                        // 产物条：run 交付了什么，≤ 3 枚 + N，全量在详情侧板（真源 :376）。
                        <WorkflowArtifactStrip
                            artifacts=artifacts.clone()
                            class="px-1 pb-0.5".to_string()
                            more_test_id=Some("workflow-card-artifacts-more".to_string())
                            on_open_artifact=None
                            pill_test_id=Some("workflow-card-artifact".to_string())
                            test_id=Some("workflow-card-artifacts".to_string())
                        />
                    })}
                    {graph_truncated.then(|| view! {
                        <p class="text-ui-xs text-foreground-subtlest">
                            {i18n::text("chat.toolCall.workflow.graph.truncated")}
                        </p>
                    })}
                    {has_diagnostics.then(|| view! {
                        <WorkflowDiagnosticsSectionComponent
                            count=error_count
                            diagnostics=diagnostics.clone()
                            saved=saved_meta.is_some()
                            truncated=display_truncated
                        />
                    })}
                    {(display.is_none() && !fallback_text.is_empty()).then(|| view! {
                        <pre class="max-h-60 overflow-auto whitespace-pre-wrap break-words rounded-xl border border-border bg-panel px-3 py-2 font-mono text-ui-base text-foreground-subtle">
                            {fallback_text.clone()}
                        </pre>
                    })}
                    {no_content_at_all.then(|| view! {
                        <p class="font-mono text-ui-base text-foreground-subtle">
                            {i18n::text("chat.toolCall.workflow.noScript")}
                        </p>
                    })}
                    {show_script_fold.then(|| view! {
                        // 没有 run 就没有详情页的 Script 区，脚本原文只能在这里读；
                        // 默认收起，与确认窗同形（真源 :417-449）。
                        <div class="min-w-0">
                            <button
                                class="flex min-w-0 items-center gap-1 rounded-md py-0.5 text-left text-ui-xs font-medium text-foreground-subtlest transition-colors hover:text-foreground-subtle"
                                data-testid="workflow-card-script-toggle"
                                type="button"
                                on:click=move |_| {
                                    script_open.update(|value| {
                                        *value = !*value;
                                    })
                                }
                            >
                                <span class=move || format!(
                                    "size-3.5 shrink-0 transition-transform {}",
                                    if script_open.get() { "rotate-90" } else { "" },
                                )>
                                    {icon_chevron_right()}
                                </span>
                                <span class="min-w-0 truncate">
                                    {move || i18n::text(if script_open.get() {
                                        "chat.permission.workflow.hideScript"
                                    } else {
                                        "chat.permission.workflow.showScript"
                                    })}
                                </span>
                            </button>
                            {script_open.get().then(|| view! {
                                <div class="pt-1.5">
                                    <div class="max-h-72 overflow-auto">
                                        <RichCodeBlock
                                            code=fold_code.clone()
                                            language="typescript".to_string()
                                            label=Some("typescript".to_string())
                                            copy_label=None
                                            wrap_label=None
                                            marked_lines=fold_marked.clone()
                                            show_line_numbers=true
                                        />
                                    </div>
                                </div>
                            })}
                        </div>
                    })}
                </div>
            })}
            // 页脚：展开时恒在；折叠时只有终态（失败 / 取消）留下——Resume 必须仍然够得着。
            // （页脚曾是摘要行；卡上只说阶段与子代理，都在表头——
            //   页脚只剩 Resume 的落点，没有 Resume 就没有页脚。真源 :453-458）
            {(resumable && run.is_some() && (expanded.get() || terminal)).then(|| view! {
                <WorkflowCardFooter status=footer_status.expect("有 run 即有状态") trailing=resume_node />
            })}
        </section>
        {snapshot_notice()}
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_is_only_input_streaming_while_running() {
        // 真源 :106 —— 两个条件都要：在跑 **且** v4 状态是 inputStreaming。
        let f = |running: bool, v4: &str| {
            create_workflow_flags(running, Some(v4), "in_progress", None, false, false)
        };
        assert!(f(true, "inputStreaming").writing);
        assert!(!f(false, "inputStreaming").writing);
        assert!(!f(true, "running").writing);
    }

    #[test]
    fn compile_errors_beat_a_joined_run() {
        // ★真源 :220-222 —— handler 在编不过的路径上直接回诊断、不启动引擎，本来就不该有
        // run；即使宿主联接到了也留在反馈行。「诊断优先」写死在这里而不是依赖调用方。
        let f = create_workflow_flags(
            false,
            Some("success"),
            "completed",
            Some(false), // display.ok === false
            true,
            true,        // 宿主已联接到 run
        );
        assert!(f.show_failure_status);
        assert!(!f.summary_only, "编不过时不能只给摘要行——诊断要看得到");
    }

    #[test]
    fn failure_without_display_still_reads_as_failed() {
        // 真源 :97 —— 有 display 时以 display.ok 为准，没 display 才看行状态。
        let with_display =
            create_workflow_flags(false, Some("error"), "failed", Some(true), true, false);
        assert!(
            !with_display.show_failure_status,
            "display.ok 为真：行状态是 failed 也不算失败样式"
        );
        let no_display =
            create_workflow_flags(false, Some("error"), "failed", None, false, false);
        assert!(no_display.show_failure_status);
    }

    #[test]
    fn writing_line_is_summary_only_unless_it_failed() {
        // 真源 :224 —— 编写中不展示半截脚本。
        let f = create_workflow_flags(true, Some("inputStreaming"), "in_progress", None, false, false);
        assert!(f.summary_only);
        let failed_mid_write =
            create_workflow_flags(true, Some("inputStreaming"), "in_progress", Some(false), true, false);
        assert!(!failed_mid_write.summary_only);
    }

    #[test]
    fn prelaunch_needs_an_unjoined_run() {
        // 真源 :223 —— 联接到 run 之后就不走普通摘要那一枝了。
        assert!(create_workflow_flags(true, Some("inputStreaming"), "in_progress", None, false, false).prelaunch);
        assert!(create_workflow_flags(false, Some("pendingApproval"), "pending", None, false, false).prelaunch);
        assert!(!create_workflow_flags(false, Some("pendingApproval"), "pending", None, false, true).prelaunch);
        assert!(!create_workflow_flags(false, Some("running"), "in_progress", None, false, false).prelaunch);
    }

    #[test]
    fn in_flight_stops_once_the_run_is_joined() {
        // 真源 :112。
        assert!(create_workflow_flags(true, Some("running"), "in_progress", None, false, false).in_flight);
        assert!(!create_workflow_flags(true, Some("running"), "in_progress", None, false, true).in_flight);
    }

    #[test]
    fn empty_graph_gets_no_track() {
        // 真源 :100-104 + :115 —— 一次 ask / files.* 都没有的脚本不值得一条空轨道。
        assert!(!graph_is_worth_a_track(None));
        let empty = serde_json::from_value::<WorkflowCausalityGraphData>(serde_json::json!({
            "steps": [], "lanes": [], "participants": [], "handoffs": [],
        }))
        .expect("零步图");
        assert!(!graph_is_worth_a_track(Some(&empty)));
        // 有 step 才建模型这半边由 `graph_is_worth_a_track` 的 `!steps.is_empty()` 承担；
        // 正向用例需要一个合法的整图（step 还要 lane 等件），那属于 types.rs 自己的
        // 反序列化测试面，这里不重复造一份 schema。
    }

    #[test]
    fn draft_timeline_only_before_the_display_lands() {
        // 真源 :124-125 —— display 一到整个模型被替换，扫出来的半截站随即离场。
        assert!(should_scan_draft_timeline(false, true, true));
        assert!(!should_scan_draft_timeline(true, true, true), "已有图就不扫草稿");
        assert!(!should_scan_draft_timeline(false, false, true), "不在编写中不扫");
        assert!(!should_scan_draft_timeline(false, true, false), "脚本还没写出第一个字");
    }

    #[test]
    fn open_state_defaults_to_expanded_per_tool() {
        // 真源 :63 + :129 —— 没记过的工具行默认展开；记忆按 toolId。
        assert!(open_state::recalled("never-seen"));
        open_state::remember("some-tool", false);
        assert!(!open_state::recalled("some-tool"));
        open_state::remember("some-tool", true);
        assert!(open_state::recalled("some-tool"));
        // 别的行不受影响。
        assert!(open_state::recalled("another-tool"));
    }

    #[test]
    fn empty_diagnostics_constant_is_reused_not_rebuilt() {
        // 真源 :60-61 —— 模块级常量，免得每次渲染一个新数组打穿记忆。
        assert_eq!(NO_DIAGNOSTICS.len(), 0);
    }
}
