//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/agent.tsx`（413 行）。
//!
//! Agent/子智能体卡：父对话里**只保留单行摘要**（真源 :375-376 产品边界——
//! 完整 child timeline 统一从右侧 tab 查看，不自动展开、无手动展开入口），
//! 展开区仍保留（内容为后台过程 / 提示词 / 活动 / 递归子卡）。
//!
//! ## 裁剪注明
//!
//! - `useSubagentsContextStore` / `useSubagentsStore`（配置子代理的颜色查找）：
//!   Rust 侧无该 store，`configuredAgentColor` 恒 None——回退链
//!   `getAgentColor(toolCall) ?? resolveSubagentColorFromName(name)` 保留；
//! - `summaryAction`（摘要行点击打开右侧 tab）：依赖 onOpenSubagentSession
//!   与侧栏会话视图（未迁），v1 不传——摘要行不可点；
//! - `ToolSnapshotFieldNotice`：同 explore，未迁不渲染；
//! - `toolCall.thought`：v4 行 schema 无该字段（adapter 未映射），
//!   activityThought 恒 None——真源在 v4 路径下同为 undefined；
//! - theme / codePreviewSettings / onOpenCodeViewer 等渲染上下文回调：
//!   未迁，markdown 区块用 pulldown-cmark 静态渲染。

use leptos::prelude::*;
use serde_json::Value;

use super::super::toolCallRowAdapter::{LegacyToolCall, LegacyToolCallNode};
use super::agentHelpers::{
    BackgroundAgentInfo, agent_message, get_agent_activity_content, get_agent_color,
    get_agent_kind_label, get_agent_primary_text, get_agent_prompt, read_background_agent_info,
};

/// 真源 :26 —— `BotIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const AGENT_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// AgentNameText 的类名（真源 :131-137）——inline-flex 垂直居中 +
/// 1.5 倍行高 + mono + 颜色类。
pub fn agent_name_text_class(agent_color: &str) -> String {
    format!(
        "inline-flex max-w-36 items-center truncate font-mono text-ui-base font-medium leading-[1.5] {}",
        super::subagentColors::subagent_text_color_class(agent_color)
    )
}

/// `AgentNameText`（真源 :129-143）——带色子代理名。
pub fn agent_name_text(agent_name: &str, agent_color: &str) -> AnyView {
    let class = agent_name_text_class(agent_color);
    let name = agent_name.to_string();
    view! {
        <span class=class title=name.clone()>{name.clone()}</span>
    }
    .into_any()
}

/// `AgentActivitySection`（真源 :76-118）——「子智能体思考/输出」区块。
#[component]
pub fn AgentActivitySection(label: String, content: String) -> impl IntoView {
    let html = crate::app::render_markdown(&content);
    view! {
        <section class="space-y-2">
            <div class="flex flex-col rounded-lg border border-border bg-background-alt/40">
                <h4 class="p-3 text-ui-base font-medium tracking-wide text-foreground-subtlest uppercase">
                    {label}
                </h4>
                <div class="max-h-64 overflow-auto" data-markdown-table-sticky-scrollbar="disabled">
                    // 真源注释 :102 —— 长代码/路径允许横向滚动，避免窄屏截断。
                    <div class="min-w-0 px-3 py-2 text-ui-base break-words" inner_html=html></div>
                </div>
            </div>
        </section>
    }
}

/// `BackgroundAgentProcessRow`（真源 :120-127）。
#[component]
fn BackgroundAgentProcessRow(label: String, children: Children) -> impl IntoView {
    view! {
        <div class="grid grid-cols-[5rem_minmax(0,1fr)] items-start gap-2">
            <div class="text-foreground-subtlest">{label}</div>
            <div class="min-w-0 text-foreground-subtle">{children()}</div>
        </div>
    }
}

/// `BackgroundAgentProcessSection`（真源 :149-211）。
#[component]
pub fn BackgroundAgentProcessSection(
    output_file: Option<String>,
    has_activity: bool,
    is_running: bool,
    status: Option<String>,
) -> impl IntoView {
    let msg = |id: &str, fallback: &str| agent_message(id, fallback);
    let status = status.as_deref();
    let launch_status = if status == Some("failed") {
        msg("chat.toolCall.agent.backgroundLaunchFailed", "启动失败")
    } else if status == Some("pending") {
        msg("chat.toolCall.agent.backgroundLaunching", "启动中")
    } else {
        msg("chat.toolCall.agent.backgroundLaunched", "已启动")
    };
    let launch_status_kind = if status == Some("failed") {
        "failed"
    } else if status == Some("pending") {
        "pending"
    } else {
        "launched"
    };
    let activity_status = if is_running {
        if has_activity {
            msg(
                "chat.toolCall.agent.backgroundActivityStreaming",
                "后台运行中，正在同步输出",
            )
        } else {
            msg(
                "chat.toolCall.agent.backgroundActivityRunningWaiting",
                "后台运行中，等待输出",
            )
        }
    } else if has_activity {
        msg(
            "chat.toolCall.agent.backgroundActivityReceived",
            "已收到子智能体回传",
        )
    } else {
        msg(
            "chat.toolCall.agent.backgroundActivityWaiting",
            "等待子智能体回传",
        )
    };
    let activity_status_kind = if is_running {
        if has_activity {
            "streaming"
        } else {
            "running_waiting"
        }
    } else if has_activity {
        "received"
    } else {
        "waiting"
    };

    view! {
        <section
            data-background-agent-activity-status=activity_status_kind
            data-background-agent-launch-status=launch_status_kind
            class="rounded-lg border border-border bg-background-alt/40 p-3 text-ui-base"
        >
            <div class="font-medium text-foreground-subtle">
                {msg("chat.toolCall.agent.backgroundProcess", "后台 Agent 过程")}
            </div>
            <div class="mt-2 space-y-2">
                <BackgroundAgentProcessRow label=msg("chat.toolCall.agent.backgroundLaunch", "启动")>
                    {launch_status}
                </BackgroundAgentProcessRow>
                <BackgroundAgentProcessRow label=msg("chat.toolCall.agent.backgroundActivity", "活动")>
                    {activity_status}
                </BackgroundAgentProcessRow>
                {output_file.map(|file| view! {
                    <BackgroundAgentProcessRow label=msg("chat.toolCall.agent.outputFile", "输出文件")>
                        <span class="block rounded-md bg-background px-2 py-1 font-mono text-foreground-subtle break-all">
                            {file}
                        </span>
                    </BackgroundAgentProcessRow>
                })}
            </div>
        </section>
    }
}

/// AgentToolCallBlock 的 props（真源从 context 解构的字段）。
#[derive(Debug, Clone)]
pub struct AgentBlockProps {
    pub tool_id: String,
    /// 完整 legacy 工具调用（helpers 取值用）。
    pub legacy: LegacyToolCall,
    pub child_tool_calls: Vec<LegacyToolCallNode>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub is_failed: bool,
    /// runtime 权威的子代理类型（subagentRow.subagentType，配对时注入）。
    pub authoritative_agent_type: Option<String>,
    pub show_icon: bool,
}

/// `AgentToolCallBlock`（真源 :213-413）。
#[component]
pub fn AgentToolCallBlock(props: AgentBlockProps) -> impl IntoView {
    let tc = &props.legacy;

    // ── 取值链（真源 :216-264）──
    let prompt = get_agent_prompt(tc);
    let fallback_label = agent_message("chat.toolCall.agent.fallback", "SubAgent");
    let primary_text = get_agent_primary_text(tc, &fallback_label);
    let agent_name = get_agent_kind_label(tc, "", props.authoritative_agent_type.as_deref());
    // configuredAgentColor 裁剪：无 subagents store（见模块头注释）。
    let agent_color = if agent_name.is_empty() {
        None
    } else {
        get_agent_color(tc)
            .unwrap_or_else(|| {
                super::subagentColors::resolve_subagent_color_from_name(&agent_name).to_string()
            })
            .into()
    };
    let kind_detail_view: Option<ChildrenFn> = match (&agent_name, &agent_color) {
        (name, Some(color)) if !name.is_empty() => {
            let name = name.clone();
            let color = color.clone();
            Some(std::sync::Arc::new(move || agent_name_text(&name, &color)) as ChildrenFn)
        }
        _ => None,
    };

    // 父块运行态只由父 Agent tool 决定（真源 :251-254 注释）：子工具不能反向续住。
    let is_agent_visually_running = props.is_running;
    let collapsed = is_agent_visually_running
        .then(|| {
            super::explore::get_latest_explore_child_summary_from_children(
                &props.child_tool_calls,
                true,
            )
        })
        .flatten();

    let background_agent_info = read_background_agent_info(tc);
    let activity_content = get_agent_activity_content(tc);
    // activityThought 恒 None（v4 行无 thought 字段，见模块头注释）。

    // 折叠态主文本（collapsed 摘要优先，否则纯文本 truncate）。
    let primary_view_source = collapsed.clone();
    let primary_view: Option<ChildrenFn> = primary_view_source.map(|c| {
        std::sync::Arc::new(move || super::explore::explore_summary_primary_view(&c)) as ChildrenFn
    });
    let secondary_view_source = collapsed.clone();
    let secondary_view: Option<ChildrenFn> = secondary_view_source.map(|c| {
        std::sync::Arc::new(move || super::explore::explore_summary_secondary_view(&c))
            as ChildrenFn
    });

    let summary_content_key = collapsed
        .as_ref()
        .map(|c| c.animation_key.clone())
        .unwrap_or_else(|| format!("agent:{}:{}", props.tool_id, primary_text));
    let title = collapsed
        .as_ref()
        .and_then(|c| c.title.clone())
        .unwrap_or_else(|| primary_text.clone());

    // ── 展开区（真源 :288-346）──
    let child_context = crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext {
        show_icon: false,
        ..Default::default()
    };
    let children_data: Vec<(String, LegacyToolCallNode)> = props
        .child_tool_calls
        .iter()
        .map(|child| (child.tool_call.tool_id.clone(), child.clone()))
        .collect();
    let bg_section = background_agent_info.clone();
    let has_activity = activity_content.is_some();
    let status_for_bg = tc.v4_status.clone();
    let activity_for_view = activity_content.clone();
    let prompt_for_view = prompt.clone();

    let render_content = move || {
        let bg = bg_section.clone();
        let prompt_inner = prompt_for_view.clone();
        let activity_inner = activity_for_view.clone();
        let children_inner = children_data.clone();
        let child_ctx = child_context.clone();
        let status_for_bg = status_for_bg.clone();
        view! {
            // 真源 :290 —— 展开容器。
            <div class="ml-2 space-y-3 border-l border-border pl-3.5">
                {bg.map(|info: BackgroundAgentInfo| view! {
                    <BackgroundAgentProcessSection
                        output_file=info.output_file
                        has_activity=has_activity
                        is_running=is_agent_visually_running
                        status=Some(status_for_bg.clone())
                    />
                })}
                {prompt_inner.map(|p| view! { <super::agentPromptSection::AgentPromptSection prompt=p /> })}
                {activity_inner.map(|content| view! {
                    <AgentActivitySection
                        label=agent_message("chat.toolCall.agent.output", "Agent output")
                        content=content
                    />
                })}
                {(!children_inner.is_empty()).then(|| {
                    // AgentChildToolList（真源 :28-74）：space-y-2 + 递归（showIcon=false）。
                    let rows = children_inner
                        .iter()
                        .map(|(tool_id, node)| {
                            view! {
                                <div class="min-w-0" data-tool-id=tool_id.clone()>
                                    <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                                        node=node.clone()
                                        context=child_ctx.clone()
                                    />
                                </div>
                            }
                        })
                        .collect_view();
                    view! { <div class="space-y-2">{rows}</div> }
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
                // 真源 :372-373 —— 摘要行不可折叠（右栏承担完整 timeline）。
                can_toggle: Some(false),
                force_open: Some(false),
                auto_collapse_on_complete: Some(true),
                kind_label: Some(fallback_label.clone()),
                expanded_kind_label: Some(fallback_label.clone()),
                kind_detail_view: kind_detail_view,
                primary_text: Some(primary_text.clone()),
                primary_text_view: primary_view,
                expanded_primary_text: Some(primary_text.clone()),
                secondary_text_view: secondary_view,
                // 真源 :388 —— 展开态显式清空次文本（父展开后由子 text 表达）。
                expanded_secondary_text: Some(String::new()),
                summary_content_separator: Some("·".to_string()),
                animate_summary_content: Some(true),
                summary_content_key: Some(summary_content_key),
                status_label: props.status_label.clone(),
                status_tooltip: props.is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(props.is_failed),
                is_running: Some(is_agent_visually_running),
                title: Some(title),
                expanded_title: Some(primary_text.clone()),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=AGENT_TOOL_ICON_CLASS>
                        // BotIcon（lucide）：头 + 双眼 + 天线。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 8V4H8"></path>
                            <rect width="16" height="12" x="4" y="8" rx="2"></rect>
                            <path d="M2 14h2"></path>
                            <path d="M20 14h2"></path>
                            <path d="M15 13v2"></path>
                            <path d="M9 13v2"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content))
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolCallRowAdapter::tool_call_row_to_legacy_node;
    use serde_json::json;

    fn tc(input: Value, output: Value) -> LegacyToolCall {
        tool_call_row_to_legacy_node(&json!({
            "kind": "toolCall", "rowId": 1, "toolCallId": "tc-1",
            "toolName": "Agent", "status": "success", "inputText": "",
            "input": input, "output": output,
        }))
        .tool_call
    }

    #[test]
    fn agent_name_text_has_mono_and_color_class() {
        // 真源 :129-143 —— inline-flex 垂直居中、1.5 行高、mono、带色。
        let class = agent_name_text_class("cyan");
        assert!(class.contains("font-mono"));
        assert!(class.contains("max-w-36"));
        assert!(class.contains("leading-[1.5]"));
        assert!(class.contains("text-cyan-700"), "应含颜色类: {class}");
    }

    #[test]
    fn agent_color_resolution_fallback_chain() {
        // 显式色 → 名字哈希兜底（真源 :246-248）。
        let tool = tc(json!({"color": "purple"}), json!({}));
        let color = get_agent_color(&tool).unwrap_or_else(|| {
            super::super::subagentColors::resolve_subagent_color_from_name("explorer").to_string()
        });
        assert_eq!(color, "purple");
        // 无显式色 → 哈希。
        let tool = tc(json!({}), json!({}));
        assert!(get_agent_color(&tool).is_none());
        assert!(super::super::subagentColors::is_subagent_color(
            super::super::subagentColors::resolve_subagent_color_from_name("explorer")
        ));
    }

    #[test]
    fn background_section_status_words() {
        // 状态词映射（真源 :162-176）。
        assert_eq!(
            agent_message("chat.toolCall.agent.backgroundLaunchFailed", "x"),
            "启动失败"
        );
        assert_eq!(
            agent_message("chat.toolCall.agent.backgroundActivityReceived", "x"),
            "已收到子智能体回传"
        );
        assert_eq!(
            agent_message("chat.toolCall.agent.backgroundProcess", "x"),
            "后台 Agent 过程"
        );
    }
}
