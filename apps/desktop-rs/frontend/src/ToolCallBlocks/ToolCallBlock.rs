//! 1:1 翻译 `packages/ui/src/ToolCallBlocks.tsx`（393 行）。
//!
//! 顶层编排：legacy node → 共享上下文（状态词/运行态/失败正文/来源标签）
//! → resolveRenderer 分流 → 已迁 renderer 卡片或 fallback 兜底。
//!
//! **裁剪注明**（对齐真源 :73-152 的 props，未迁项给缺省）：
//! - 入场动画键表（真源 :26-67）：装饰性淡入，依赖模块级 Map + 定时清理，
//!   Leptos 侧 v1 跳过（`data-zcode-tool-stream-animate` 恒不设）；
//! - workflowRun / workflowDraft 联接（:91/:135）：依赖 workflowRuns 投影
//!   （宿主按 toolCallId 联接）与侧栏 run 视图，两者在 Rust 应用里都还没有宿主表面——
//!   上下文里留了字段位（恒 None），workflow 系 renderer 已不再走 fallback，
//!   但 run 态紧凑可点卡要等联接与侧板那批一起接上；
//! - theme / codePreviewSettings（:109-112）：桌面端单主题，v1 无代码预览设置；
//! - office 模式（:156）：桌面端恒 false；
//! - CUA 组（:378-384）：依赖 conversationCuaGroups 分组器，未迁，走 fallback。

use leptos::prelude::*;
use serde_json::Value;

use super::resolveRenderer::{Renderer, resolve_tool_call_renderer};
use super::toolCallRowAdapter::{LegacyToolCall, LegacyToolCallNode};
use super::toolStatus::{
    compact_tool_call_status_label, is_compact_tool_call_running_state, map_tool_status,
};

/// 编排层的宿主上下文（真源 `ConversationRowRenderContext` 中 ToolCallBlock
/// 实际消费的子集）。
#[derive(Debug, Clone)]
pub struct ToolCallBlockContext {
    /// 工作区路径（display model / 文件解析用；v4 行自包含，v1 透传给 fallback）。
    pub workspace_path: String,
    /// 是否渲染 Todo 工具卡（真源 :95 `showTodoToolCalls`；设置里可关）。
    pub show_todo_tool_calls: bool,
    /// 子代理来源标签抑制（嵌套卡沿用父级设置；v4 行无子树，v1 恒 false）。
    pub suppress_source_label: bool,
    /// 是否显示工具图标（真源 :79 `showIcon`；分组 children 传 false）。
    pub show_icon: bool,
    /// runtime 权威的子代理类型（真源 `authoritativeAgentType`，:99；
    /// AgentToolCall 配对时从 subagentRow 注入——已投影的类型优先于流式半截 input）。
    pub authoritative_agent_type: Option<String>,
    /// 「加载完整工具数据」请求通道（真源 `onLoadFullToolCallFields`，
    /// ToolCallBlocks.tsx:137）。真源返回 `Promise<boolean | void> | boolean | void`：
    /// resolve(false) / reject → 失败态。Rust 通道以同步
    /// `Callback<String, bool>` 承接（false = 失败）。宿主未接线时 None——
    /// 快照提示按真源语义（`!onLoadFullToolCallFields` → null）不渲染。
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    /// run 联接摘要（真源 `workflowRun`，fileSummaryTypes.ts:286-291）。
    ///
    /// ★宿主尚未接线：真源它由 `v4/workflowRunCardJoin.ts` 的 byToolCallId / byRunId 表
    /// 按行联接，而那张表的数据源 `workflowRuns` 投影与侧栏 run 视图在 Rust 应用里都还没有
    /// 宿主表面（同「node-repl 打开完整结果」那条 defer）。因此这里恒 None，
    /// create / resume 两张卡走各自的「未联接」分支——那是真源为
    /// 「display 缺席的老会话 / 失败路径 / 投影尚未就绪」写好的第三条路，不是兜底。
    pub workflow_run: Option<super::fileSummaryTypes::WorkflowRunCardSummary>,
    /// 打开侧栏 run 视图的通道（真源 `onOpenWorkflowRun`，fileSummaryTypes.ts:255）。
    /// 与 `workflow_run` 同样未接线，恒 None；卡片据此退回纯展示态（不加 role/tabIndex）。
    pub on_open_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
    /// 编译反馈的草稿位置（真源 `workflowDraft`，:295，宿主按 toolCallId 从行窗口联接）。
    /// 与 `workflow_run` 同一条联接通道，未接线时恒 None。
    pub workflow_draft: Option<super::fileSummaryTypes::WorkflowDraftPosition>,
    /// 页脚 Resume（真源 `onResumeWorkflowRun`，:260）。
    pub on_resume_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
    /// 点一枚药丸开那个子代理的 transcript（真源 `onOpenWorkflowActor`，:265-276）。
    pub on_open_workflow_actor: Option<Callback<WorkflowActorOpenRequest>>,
    /// 点脚本药丸开 run 的脚本 transcript（真源 `onOpenWorkflowWorkspace`，:277）。
    pub on_open_workflow_workspace: Option<Callback<WorkflowRunOpenRequest>>,
    /// 点一枚产物开产物（真源 `onOpenWorkflowArtifact`，:282，参数是 artifactId）。
    pub on_open_workflow_artifact: Option<Callback<String>>,
}

/// `onOpenWorkflowRun` / `onResumeWorkflowRun` / `onOpenWorkflowWorkspace` 共用的请求体。
///
/// 真源是三个各自的内联形状（fileSummaryTypes.ts:255/:260/:277）：
/// `{workflowName?, phaseId?}` —— `phaseId` 只有 run 视图与脚本 transcript 用，
/// Resume 只带 `workflowName`。Rust 侧收成一个结构，缺的字段就是真源里的「不带」。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowRunOpenRequest {
    pub workflow_name: Option<String>,
    pub phase_id: Option<String>,
}

/// `onOpenWorkflowActor` 的请求体（真源 fileSummaryTypes.ts:265-276）。
///
/// 卡片只交出**槽位身份**（还没启动的药丸也可开，会话 id 有则随行），
/// 会话与 workspace 身份由宿主补齐。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowActorOpenRequest {
    pub ordinal: i64,
    pub run_id: String,
    pub site_id: String,
    pub actor_session_id: Option<String>,
    pub actor_name: Option<String>,
}

impl Default for ToolCallBlockContext {
    fn default() -> Self {
        Self {
            workspace_path: String::new(),
            show_todo_tool_calls: true,
            suppress_source_label: false,
            // 真源默认 showIcon = true（:79）。
            show_icon: true,
            authoritative_agent_type: None,
            on_load_full_tool_call_fields: None,
            workflow_run: None,
            on_open_workflow_run: None,
            workflow_draft: None,
            on_resume_workflow_run: None,
            on_open_workflow_actor: None,
            on_open_workflow_workspace: None,
            on_open_workflow_artifact: None,
        }
    }
}

/// 「todo family 且宿主不显示」的早退判定（真源 :366-368）。
///
/// 注意真源注释（:191-196）：early return 必须在所有 hook 之后——React 的
/// hook 数量一致性约束。Leptos 无此约束，但保留同序：判定放渲染前最后一步。
pub fn should_hide_todo(tool_call: &LegacyToolCall, ctx: &ToolCallBlockContext) -> bool {
    !ctx.show_todo_tool_calls
        && super::resolveRenderer::family_by_lower(
            tool_call.tool_name.as_deref().unwrap_or_default(),
        ) == super::resolveRenderer::ToolFamily::Todo
}

/// 渲染一个工具调用节点（真源 `ToolCallBlockComponent`）。
///
/// 从 v1 起：已迁 renderer（execute/read/edit/search/todo）出专属卡；
/// 其余工具出 fallback 兜底卡（标题 + 状态 + 失败正文 + raw 折叠），
/// 与真源 fallback 的职责一致——**未迁不隐藏**。
#[component]
pub fn ToolCallBlock(node: LegacyToolCallNode, context: ToolCallBlockContext) -> impl IntoView {
    let tc = &node.tool_call;
    let tool_id = tc.tool_id.clone();
    let tool_name = tc.tool_name.clone();
    let kind = tc.kind.clone();
    let title = tc.title.clone();
    let input = tc.input.clone().unwrap_or(Value::Null);
    let output = tc.output.clone().map(Value::String).unwrap_or(Value::Null);
    let raw = tc.raw.clone();
    let status_legacy = tc.status.clone();

    // ── 共享上下文（真源 :197-232）──
    let tool_state = map_tool_status(&status_legacy);
    let is_running = is_compact_tool_call_running_state(tool_state.as_str());
    let status_label = compact_tool_call_status_label(tool_state.as_str(), Some(&status_legacy));
    let error_text = super::toolError::get_tool_call_error_text(
        tc.error.as_deref(),
        &output,
        &raw,
        &status_legacy,
    );

    // 来源标签（真源 :227-232）：v4 行无 parentToolUseId 且 depth=0，
    // 子代理判定不触发 → source_label 恒 None（isSubAgentToolCall 的 v4 化简）。
    let source_label: Option<String> = None;

    let renderer = resolve_tool_call_renderer(
        tool_name.as_deref().unwrap_or_default(),
        &kind,
        title.as_deref().unwrap_or_default(),
        &raw,
        // 真源 :124 `readMcpToolPresentation(context)`——display 载荷或 legacy
        // `mcp__server__tool` 命名任一命中即通用 MCP 卡（node-repl 先于它分流）。
        super::renderers::mcp::read_mcp_tool_presentation(tc).is_some(),
    );

    // todo 隐藏判定（真源 :366-368）。
    if should_hide_todo(tc, &context) {
        return ().into_any();
    }

    let show_icon = context.show_icon;
    let is_office_mode = false;
    let children = node.child_tool_calls.clone();

    view! {
        <div
            class="w-full"
            data-tool-call-id=tool_id.clone()
            data-tool-name=tool_name.clone().unwrap_or_else(|| kind.clone())
            data-status=status_legacy.clone()
        >
            {render_dispatch(
                renderer,
                &tool_id,
                tool_name.clone(),
                &kind,
                title.clone(),
                input.clone(),
                output.clone(),
                raw.clone(),
                status_legacy.clone(),
                is_running,
                status_label.to_string(),
                error_text.clone(),
                source_label,
                show_icon,
                is_office_mode,
                children,
                tc.clone(),
                context.authoritative_agent_type.clone(),
                context.workspace_path.clone(),
                context.on_load_full_tool_call_fields.clone(),
                context.workflow_run.clone(),
                context.on_open_workflow_run.clone(),
                context.workflow_draft,
                context.on_resume_workflow_run.clone(),
                context.on_open_workflow_actor.clone(),
                context.on_open_workflow_workspace.clone(),
                context.on_open_workflow_artifact.clone(),
            )}
        </div>
    }
    .into_any()
}

/// 按解析结果分发到 renderer 卡片（真源 :378-387 的 `ToolCallRenderer` 位）。
#[allow(clippy::too_many_arguments)]
fn render_dispatch(
    renderer: Renderer,
    tool_id: &str,
    tool_name: Option<String>,
    kind: &str,
    title: Option<String>,
    input: Value,
    output: Value,
    raw: Value,
    status: String,
    is_running: bool,
    status_label: String,
    error_text: Option<String>,
    source_label: Option<String>,
    show_icon: bool,
    is_office_mode: bool,
    // 子工具节点（分组聚合卡展开时递归渲染；叶子行恒空）。
    children: Vec<LegacyToolCallNode>,
    // 原始 legacy 载荷（Agent 卡 helpers 取值链用；重建会丢 content 桥接等字段）。
    legacy: LegacyToolCall,
    // runtime 权威的子代理类型（AgentToolCall 配对注入）。
    authoritative_agent_type: Option<String>,
    // 工作区路径（switch_mode 的 planFilePath 相对路径解析用）。
    workspace_path: String,
    // 「加载完整工具数据」通道（宿主未接线时为 None，快照提示按真源不渲染）。
    on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    // run 联接摘要（workflowRunCardJoin 的产物；宿主未接线上文恒 None）。
    workflow_run: Option<super::fileSummaryTypes::WorkflowRunCardSummary>,
    // 打开侧栏 run 视图的通道（同上，恒 None）。
    on_open_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
    // 编译反馈的稿号位置（同上，恒 None）。
    workflow_draft: Option<super::fileSummaryTypes::WorkflowDraftPosition>,
    on_resume_workflow_run: Option<Callback<WorkflowRunOpenRequest>>,
    on_open_workflow_actor: Option<Callback<WorkflowActorOpenRequest>>,
    on_open_workflow_workspace: Option<Callback<WorkflowRunOpenRequest>>,
    on_open_workflow_artifact: Option<Callback<String>>,
) -> AnyView {
    use super::renderers::*;

    match renderer {
        Renderer::Agent => view! {
            <agent::AgentToolCallBlock
                props=agent::AgentBlockProps {
                    tool_id: tool_id.to_string(),
                    legacy: legacy.clone(),
                    child_tool_calls: children,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    is_failed: status == "failed",
                    authoritative_agent_type: authoritative_agent_type.clone(),
                    show_icon,
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::Explore => view! {
            <explore::ExploreToolCallBlock
                props=explore::ExploreBlockProps {
                    tool_id: tool_id.to_string(),
                    child_tool_calls: children,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    is_failed: status == "failed",
                    title,
                    source_label,
                    show_icon,
                    can_toggle: None,
                    force_open: None,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::ExecuteGroup => {
            // kind 聚合类：children 逐项递归渲染（真源 execute-group.tsx:56-70）。
            let child_tool_calls: Vec<execute_group::ChildToolCall> = children
                .into_iter()
                .map(|child| execute_group::ChildToolCall {
                    tool_id: child.tool_call.tool_id.clone(),
                    input: child.tool_call.input.clone().unwrap_or(Value::Null),
                    status: child.tool_call.status.clone(),
                    node: child,
                })
                .collect();
            view! {
                <execute_group::ExecuteGroupToolCallBlock
                    props=execute_group::ExecuteGroupProps {
                        tool_id: tool_id.to_string(),
                        title,
                        child_tool_calls,
                        is_running,
                        status_label: Some(status_label),
                        is_office_mode,
                        can_toggle: None,
                        force_open: None,
                    }
                />
            }
            .into_any()
        }
        Renderer::Execute => view! {
            <execute::ExecuteToolCallBlock
                props=execute::ExecuteBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    raw,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    status: Some(status),
                    title,
                    kind: Some(kind.to_string()),
                    source_label,
                    show_icon,
                    is_office_mode,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::Read => {
            let summary = read::build_read_summary(&input, &raw);
            view! {
                <read::ReadToolCallBlock
                    props=read::ReadBlockProps {
                        tool_id: tool_id.to_string(),
                        input,
                        raw,
                        title,
                        kind: Some(kind.to_string()),
                        status: Some(status),
                        is_running,
                        status_label: Some(status_label),
                        error_text,
                        source_label,
                        show_icon,
                        summary,
                        snapshot_refs: legacy.snapshot_refs.clone(),
                        on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                    }
                />
            }
            .into_any()
        }
        Renderer::Escalate => view! {
            <escalate::EscalateToolCallBlock
                props=escalate::EscalateBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    status: Some(status),
                    is_running,
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // 主代答子代理阻塞问题的卡（真源 :114-116 —— 按名分流，与 escalate 是配对的两张卡）。
        Renderer::ResolveWorkflowQuestion => view! {
            <resolve_workflow_question::ResolveWorkflowQuestionToolCallBlock
                props=resolve_workflow_question::ResolveQuestionBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    error: legacy.error.clone(),
                    status: Some(status),
                    is_running,
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // 恢复入口卡（真源 :96-100 —— 按名分流，必须抢在 workflow family 之前认领自己的名字）。
        Renderer::ResumeWorkflowRun => view! {
            <resume_workflow_run::ResumeWorkflowRunToolCallBlock
                props=resume_workflow_run::ResumeBlockProps {
                    tool_id: tool_id.to_string(),
                    raw,
                    status,
                    is_running,
                    output_text: legacy.output.clone(),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                    workflow_run: workflow_run.clone(),
                    on_open_workflow_run: on_open_workflow_run.clone(),
                }
            />
        }
        .into_any(),
        // actor 提交结果的卡（真源 :148-156 —— workflow family 内按工具名分出的另一张卡面）。
        Renderer::SubmitResult => view! {
            <submit_result::SubmitResultToolCallBlock
                props=submit_result::SubmitResultBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    status: status.clone(),
                    is_running,
                    error: legacy.error.clone(),
                    error_text,
                    output_text: legacy.output.clone(),
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // 实例清单卡（真源 :90-92 —— 按名分流，与 GetWorkflowRun 同族）。
        Renderer::ListWorkflowRuns => view! {
            <list_workflow_runs::ListWorkflowRunsToolCallBlock
                props=list_workflow_runs::ListWorkflowRunsBlockProps {
                    tool_id: tool_id.to_string(),
                    raw,
                    status,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    output_text: legacy.output.clone(),
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // run 情势快照卡（真源 :87-89 —— 按名分流）。
        Renderer::GetWorkflowRun => view! {
            <get_workflow_run::GetWorkflowRunToolCallBlock
                props=get_workflow_run::GetWorkflowRunBlockProps {
                    tool_id: tool_id.to_string(),
                    raw,
                    status,
                    is_running,
                    error_text,
                    output_text: legacy.output.clone(),
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // 可复用工作流的落库记录卡（真源 :77-79 —— 按名分流，排在 family 之前）。
        Renderer::SaveWorkflow => view! {
            <save_workflow::SaveWorkflowToolCallBlock
                props=save_workflow::SaveWorkflowBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    status,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // 可复用清单卡（真源 :80-82 —— 按名分流）。读不出结构化结果时它自己交回 fallback，
        // 所以要把 display model 与 legacy 一并递给它。
        Renderer::ListSavedWorkflows => {
            let display_model = super::toolDisplay::build_tool_display_model(
                &super::codeViewer::CodeViewerToolCall::from_legacy(&legacy),
                legacy.error.as_deref(),
                &status,
                &workspace_path,
            );
            view! {
                <list_saved_workflows::ListSavedWorkflowsToolCallBlock
                    props=list_saved_workflows::ListSavedWorkflowsBlockProps {
                        tool_id: tool_id.to_string(),
                        kind: kind.to_string(),
                        raw,
                        output_text: legacy.output.clone(),
                        title,
                        status: Some(status.clone()),
                        is_running,
                        status_label: Some(status_label),
                        error_text,
                        source_label,
                        show_icon,
                        snapshot_refs: legacy.snapshot_refs.clone(),
                        on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                        legacy: legacy.clone(),
                        display_model,
                        workspace_path: workspace_path.clone(),
                    }
                />
            }
            .into_any()
        }
        Renderer::ReadSessionContext => view! {
            <read_session_context::ReadSessionContextToolCallBlock
                props=read_session_context::ReadSessionContextBlockProps {
                    tool_id: tool_id.to_string(),
                    input: Some(input),
                    // v4 适配层把输出压平成散文（toolCallRowAdapter），这里还原成 Value 形态
                    // 以走真源的 record 探测路径。
                    output: legacy.output.clone().map(Value::String),
                    raw,
                    status,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        // 模型目录卡（真源 :101-105 —— 不按名认领，兜底卡会把 `<models>` 文本原样摊开）。
        Renderer::ListModels => {
            let display_model = super::toolDisplay::build_tool_display_model(
                &super::codeViewer::CodeViewerToolCall::from_legacy(&legacy),
                legacy.error.as_deref(),
                &status,
                &workspace_path,
            );
            view! {
                <list_models::ListModelsToolCallBlock
                    props=list_models::ListModelsBlockProps {
                        tool_id: tool_id.to_string(),
                        raw,
                        output: legacy.output.clone().map(Value::String),
                        status,
                        is_running,
                        status_label: Some(status_label),
                        error_text,
                        title,
                        source_label,
                        show_icon,
                        snapshot_refs: legacy.snapshot_refs.clone(),
                        on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                        // provider 名来自会话 store 的 configOptions（workspace-config topic），
                        // 宿主通道未接线时 None → 组名退回 providerLabel → 「模型供应商」，
                        // 仍绝不回 providerId（见 list_models.rs 的裁剪注明）。
                        provider_name: None,
                        legacy: legacy.clone(),
                        display_model,
                        workspace_path: workspace_path.clone(),
                    }
                />
            }
            .into_any()
        }
        // CreateWorkflow / AmendWorkflow 共用的一张卡（真源 :148-156 —— workflow family 的兜底）。
        Renderer::CreateWorkflow => {
            let display_model = super::toolDisplay::build_tool_display_model(
                &super::codeViewer::CodeViewerToolCall::from_legacy(&legacy),
                legacy.error.as_deref(),
                &status,
                &workspace_path,
            );
            view! {
                <create_workflow::CreateWorkflowToolCallBlock
                    props=create_workflow::CreateWorkflowBlockProps {
                        tool_id: tool_id.to_string(),
                        kind: kind.to_string(),
                        tool_name,
                        title,
                        input,
                        output,
                        raw,
                        status: status.clone(),
                        v4_status: legacy.v4_status.clone(),
                        is_running,
                        status_label: Some(status_label),
                        error_text,
                        source_label,
                        show_icon,
                        snapshot_refs: legacy.snapshot_refs.clone(),
                        on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                        workflow_run,
                        workflow_draft,
                        on_open_workflow_run,
                        on_resume_workflow_run,
                        on_open_workflow_actor,
                        on_open_workflow_workspace,
                        on_open_workflow_artifact,
                        legacy: legacy.clone(),
                        display_model,
                        workspace_path: workspace_path.clone(),
                    }
                />
            }
            .into_any()
        }
        Renderer::Edit => {
            // kind_source 携带完整输入输出（edit.rs 的操作类型判定用）。
            let kind_source = super::fileSummaryTypes::EditKindSource {
                tool_name: tool_name.clone(),
                kind: Some(kind.to_string()),
                title: title.clone(),
                input: Some(input.clone()),
                output: Some(output.clone()),
                raw: Some(raw.clone()),
            };
            // 文件摘要：display 结构化事实 → content diff 块 → changes → 启发式兜底。
            let file_summaries =
                super::fileSummaries::read_raw_tool_call_file_summaries(&raw, Some(&kind_source));
            // 展示模型（真源 context.displayModel 由 ToolCallBlocks 组装）——
            // edit 展开区的 ToolCallBody 与每个文件块共用同一份。
            let code_viewer_tc = super::codeViewer::CodeViewerToolCall::from_legacy(&legacy);
            let display_model = super::toolDisplay::build_tool_display_model(
                &code_viewer_tc,
                legacy.error.as_deref(),
                &status,
                &workspace_path,
            );
            view! {
                <edit::EditToolCallBlock
                    props=edit::EditBlockProps {
                        tool_id: tool_id.to_string(),
                        raw_file_summaries: file_summaries,
                        // 失败判定（真源 :91-92）：failed 态或有失败正文。
                        is_failed: status == "failed" || error_text.is_some(),
                        is_running,
                        status_label: Some(status_label),
                        error_text,
                        title,
                        kind: Some(kind.to_string()),
                        kind_source,
                        source_label,
                        show_icon,
                        is_office_mode,
                        legacy: legacy.clone(),
                        display_model,
                        child_tool_calls: children,
                        workspace_path: workspace_path.clone(),
                        theme: None,
                        prefers_dark: false,
                        snapshot_refs: legacy.snapshot_refs.clone(),
                        on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                    }
                />
            }
            .into_any()
        }
        Renderer::Search => view! {
            <search::SearchToolCallBlock
                props=search::SearchBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    status: Some(status),
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    kind_label_override: None,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::Todo => {
            let display_model = super::toolDisplay::build_tool_display_model(
                &super::codeViewer::CodeViewerToolCall::from_legacy(&legacy),
                legacy.error.as_deref(),
                &status,
                &workspace_path,
            );
            view! {
            <todo::TodoToolCallBlock
                props=todo::TodoBlockProps {
                    tool_id: tool_id.to_string(),
                    title,
                    kind: Some(kind.to_string()),
                    input,
                    output,
                    status: Some(status),
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                    legacy: legacy.clone(),
                    display_model,
                    workspace_path: workspace_path.clone(),
                }
            />
            }
            .into_any()
        }
        Renderer::AskQuestion => view! {
            <ask_question::AskQuestionToolCallBlock
                props=ask_question::AskQuestionBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    raw,
                    status: Some(status),
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                }
            />
        }
        .into_any(),
        Renderer::Skill => view! {
            <skill::SkillToolCallBlock
                props=skill::SkillBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    raw,
                    status: Some(status),
                    is_running,
                    error_text,
                    title,
                    status_label: Some(status_label),
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::SwitchMode => view! {
            <switch_mode::SwitchModeToolCallBlock
                props=switch_mode::SwitchModeBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    // inputText 未随 legacy 载荷传递（adapter 只留长度提示）——
                    // 完整 plan 走 input/output/raw 路径覆盖；流式半截场景
                    // 由 raw.content 兜底，v1 传空串。
                    input_text: String::new(),
                    output,
                    raw,
                    error_text,
                    workspace_path,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::TaskStop => view! {
            <task_stop::TaskStopToolCallBlock
                props=task_stop::TaskStopBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    raw,
                    status: Some(status),
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::TaskOutput => view! {
            <task_output::TaskOutputToolCallBlock
                props=task_output::TaskOutputBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    raw,
                    status: Some(status),
                    is_running,
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::SendMessage => view! {
            <send_message::SendMessageToolCallBlock
                props=send_message::SendMessageBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    raw,
                    status: Some(status),
                    is_running,
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::PlanGuidance => view! {
            <plan_guidance::PlanGuidanceToolCallBlock
                props=plan_guidance::PlanGuidanceBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    output,
                    raw,
                    status: Some(status),
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::RespondToCoordinator => view! {
            <respond_to_coordinator::RespondToCoordinatorToolCallBlock
                props=respond_to_coordinator::RespondToCoordinatorBlockProps {
                    tool_id: tool_id.to_string(),
                    input,
                    raw,
                    status: Some(status),
                    is_running,
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::Goal => view! {
            <goal::GoalToolCallBlock
                props=goal::GoalBlockProps {
                    tool_id: tool_id.to_string(),
                    output,
                    // 真源 :92/:110 —— content/thought 从 legacy 载荷桥接
                    // （v4 adapter 下 thought 恒 None，content 仅 Agent/Task 行有）。
                    content: legacy.content.clone(),
                    thought: legacy.thought.clone(),
                    raw,
                    status: Some(status),
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    title,
                    source_label,
                    show_icon,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any(),
        Renderer::Cua => view! {
            <cua::CuaToolCallBlock
                tool_call=legacy.clone()
                is_running
                show_icon
                can_toggle=None
                force_open=None
                error_text=error_text.clone()
            />
        }
        .into_any(),
        // 通用 MCP 卡（真源 renderers.tsx :125-126 的 McpToolCallBlock 位）。
        Renderer::Mcp => view! {
            <mcp::McpToolCallBlock
                tool_call=legacy.clone()
                is_running
                show_icon
                can_toggle=None
                force_open=None
                error_text=error_text.clone()
                status_label=Some(status_label)
                on_load_full_tool_call_fields=on_load_full_tool_call_fields.clone()
            />
        }
        .into_any(),
        // node-repl 卡（真源 :121-123 —— 先于通用 MCP 分流的专用卡）。
        Renderer::NodeRepl => view! {
            <node_repl::NodeReplToolCallBlock
                tool_call=legacy.clone()
                is_running
                show_icon
                can_toggle=None
                force_open=None
                error_text=error_text.clone()
                source_label
                on_load_full_tool_call_fields=on_load_full_tool_call_fields.clone()
            />
        }
        .into_any(),
        // 未迁 renderer 走 fallback 兜底卡（Workflow 系 / submit-result / cron 等小卡）。
        // 行级上下文不会命中的聚合类（changesGroup/executeGroup/cuaGroup 走
        // conversationAssistantWorkItems 分组器，未迁）；其余未迁 renderer 统一
        // 走 fallback 兜底卡——**未迁不隐藏**，标题/状态/错误仍可见。
        _ => {
            let display_model = super::toolDisplay::build_tool_display_model(
                &super::codeViewer::CodeViewerToolCall::from_legacy(&legacy),
                legacy.error.as_deref(),
                &status,
                &workspace_path,
            );
            view! {
            <fallback::FallbackToolCallBlock
                props=fallback::FallbackBlockProps {
                    tool_id: tool_id.to_string(),
                    kind: kind.to_string(),
                    title,
                    status: Some(status),
                    raw_tool_call: raw,
                    is_running,
                    status_label: Some(status_label),
                    error_text,
                    source_label,
                    show_icon,
                    icon_override: None,
                    has_inline_preview: false,
                    hide_raw_fallback: false,
                    summary_only: false,
                    summary_text_override: None,
                    kind_label_override: None,
                    snapshot_refs: legacy.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                    legacy: legacy.clone(),
                    display_model,
                    workspace_path: workspace_path.clone(),
                }
            />
            }
            .into_any()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(row: Value) -> LegacyToolCallNode {
        super::super::toolCallRowAdapter::tool_call_row_to_legacy_node(&row)
    }

    fn ctx() -> ToolCallBlockContext {
        ToolCallBlockContext {
            workspace_path: String::new(),
            show_todo_tool_calls: true,
            suppress_source_label: false,
            show_icon: true,
            authoritative_agent_type: None,
            on_load_full_tool_call_fields: None,
            workflow_run: None,
            on_open_workflow_run: None,
            ..Default::default()
        }
    }

    #[test]
    fn execute_row_resolves_execute_renderer() {
        // 编排器消费的判定链：resolve_renderer 对 Bash → Execute。
        let n = node(json!({
            "kind": "toolCall", "rowId": 1, "toolCallId": "tc-1",
            "toolName": "Bash", "status": "success", "inputText": "{\"command\":\"ls\"}",
        }));
        let r = resolve_tool_call_renderer("Bash", &n.tool_call.kind, "", &n.tool_call.raw, false);
        assert!(matches!(r, Renderer::Execute));
    }

    #[test]
    fn todo_hidden_when_host_disables() {
        // 真源 :366-368 —— showTodoToolCalls=false 时 todo family 返回 null。
        let n = node(json!({
            "kind": "toolCall", "rowId": 2, "toolCallId": "tc-2",
            "toolName": "TodoWrite", "status": "success", "inputText": "{}",
        }));
        let mut c = ctx();
        c.show_todo_tool_calls = false;
        assert!(should_hide_todo(&n.tool_call, &c));
        c.show_todo_tool_calls = true;
        assert!(!should_hide_todo(&n.tool_call, &c));
    }

    #[test]
    fn status_label_follows_legacy_status() {
        // 状态词链：v4 status → legacy → 中文标签。
        let n = node(json!({
            "kind": "toolCall", "rowId": 3, "toolCallId": "tc-3",
            "toolName": "Bash", "status": "running", "inputText": "{}",
        }));
        let state = map_tool_status(&n.tool_call.status);
        assert_eq!(state.as_str(), "input-available");
        assert!(is_compact_tool_call_running_state(state.as_str()));
        assert_eq!(
            compact_tool_call_status_label(state.as_str(), Some(&n.tool_call.status)),
            "执行中"
        );
    }

    #[test]
    fn error_row_surfaces_error_text() {
        // 失败正文从 legacy node 的 error 字段取出。
        let n = node(json!({
            "kind": "toolCall", "rowId": 4, "toolCallId": "tc-4",
            "toolName": "Bash", "status": "error", "inputText": "{}",
            "error": { "code": "E", "message": "退出码 1" },
        }));
        let out = n
            .tool_call
            .output
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null);
        let err = super::super::toolError::get_tool_call_error_text(
            n.tool_call.error.as_deref(),
            &out,
            &n.tool_call.raw,
            &n.tool_call.status,
        );
        assert_eq!(err.as_deref(), Some("退出码 1"));
    }

    /// 冒烟：dispatch 的路由决策（真源 resolveRenderer → 卡片类型的映射）。
    ///
    /// 不构造视图——组件树含响应式 effect，单测环境无 executor；
    /// 各 renderer 的逻辑在各自模块的单测里覆盖。
    #[test]
    fn dispatch_routing_matches_renderer_resolution() {
        // 已迁 renderer 走专属卡；未迁的都落到 fallback 分支。
        let migrated = [
            Renderer::Execute,
            Renderer::Read,
            Renderer::Edit,
            Renderer::Search,
            Renderer::Todo,
        ];
        for r in migrated {
            assert!(
                matches!(
                    r,
                    Renderer::Execute
                        | Renderer::Read
                        | Renderer::Edit
                        | Renderer::Search
                        | Renderer::Todo
                ),
                "专属卡分支需覆盖 {r:?}"
            );
        }
        // 行级上下文不会命中的聚合类 + 未迁 renderer → fallback。
        for r in [
            Renderer::ChangesGroup,
            Renderer::ExecuteGroup,
            Renderer::CuaGroup,
            Renderer::Cua,
            Renderer::Mcp,
            Renderer::Agent,
            Renderer::AskQuestion,
            Renderer::Fallback,
        ] {
            assert!(
                !matches!(
                    r,
                    Renderer::Execute
                        | Renderer::Read
                        | Renderer::Edit
                        | Renderer::Search
                        | Renderer::Todo
                ),
                "{r:?} 应走 fallback 分支"
            );
        }
    }
}
