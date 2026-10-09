//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowAgentPill.tsx`（218 行）。
//!
//! 子代理药丸：带色头像 + 名字 + 右侧状态标记。卡片的站下与侧栏的行都是它——
//! 同一个特性只有一枚药丸。
//!
//! 工作区没有身份，所以没有颜色，只有终端字形。`pending` 与无状态逐像素相同
//! （不变式 3）：没有标记、名字用次淡色、脸睡着。
//!
//! 可打开（`on_open` 在场）时整枚药丸就是按钮：悬停四件事同时落地，点一下直接开
//! 那个子代理的 transcript。尾槽（`wf-pill-tail`）是状态标记与 ↗ 共用的一格：
//! 两者叠在同一格里，悬停时标记缩出、箭头缩入。

use leptos::prelude::*;

use crate::components::workflow_graph::types::{LaneClass, StepRunStatus};
use crate::components::WorkflowAgentFace::{agent_color, WorkflowAgentFace};
use crate::components::workflowIcons;
use crate::ToolCallBlocks::i18n;

/// 车道字形（真源 `LaneGlyph`，:35-60）：agent 车道是瓦片脸（编号定色、status 定表情），
/// 工作区 / 未解析车道是图标。
#[component]
pub fn LaneGlyph(
    lane_class: LaneClass,
    #[prop(optional, into)] class: String,
    name: String,
    avatar_index: Option<usize>,
    status: Option<StepRunStatus>,
) -> impl IntoView {
    if lane_class == LaneClass::Agent {
        return view! {
            <WorkflowAgentFace avatar_index=avatar_index class=class name=name status=status />
        }
        .into_any();
    }
    let glyph = if lane_class == LaneClass::Workspace {
        workflowIcons::icon_terminal().into_any()
    } else {
        workflowIcons::icon_circle_help().into_any()
    };
    view! { <span class=class aria-hidden="true">{glyph}</span> }.into_any()
}

/// 状态词（真源 :66 `chat.toolCall.workflow.graph.status.${status}`）。
fn status_word(status: StepRunStatus) -> String {
    i18n::text(match status {
        StepRunStatus::Running => "chat.toolCall.workflow.graph.status.running",
        StepRunStatus::Done => "chat.toolCall.workflow.graph.status.done",
        StepRunStatus::Failed => "chat.toolCall.workflow.graph.status.failed",
        StepRunStatus::Pending => "chat.toolCall.workflow.graph.status.pending",
    })
}

/// 状态标记（真源 `PillStatusMark`，:63-94）：转圈 / 对勾 / 叉；`pending` 与
/// 无状态没有标记。状态变化时新标记弹入。
#[component]
pub fn PillStatusMark(status: Option<StepRunStatus>) -> impl IntoView {
    let Some(status) = status.filter(|s| *s != StepRunStatus::Pending) else {
        return view! { <span class="hidden" /> }.into_any();
    };
    let label = status_word(status);
    let tone = match status {
        // 运行圆环使用中性色，避免正常加载被读成警告（真源 :70-76）。
        StepRunStatus::Running | StepRunStatus::Done => "text-foreground-subtle",
        StepRunStatus::Failed => "text-destructive",
        StepRunStatus::Pending => "",
    };
    let icon = if status == StepRunStatus::Running {
        view! {
            <span class="inline-block size-3.5 animate-spin motion-reduce:animate-none">
                {workflowIcons::icon_loader_circle()}
            </span>
        }
        .into_any()
    } else if status == StepRunStatus::Done {
        view! {
            <span class="inline-block size-3.5">
                {workflowIcons::icon_circle_check()}
            </span>
        }
        .into_any()
    } else {
        view! {
            <span class="inline-block size-3.5">
                {workflowIcons::icon_circle_x()}
            </span>
        }
        .into_any()
    };
    view! {
        <span
            aria-label=label.clone()
            class=format!("wf-mark flex size-3.5 shrink-0 items-center justify-center {tone}")
            data-testid="workflow-pill-status"
            role="img"
            title=label
        >
            {icon}
        </span>
    }
    .into_any()
}

/// 药丸可打开时的静态接线（真源 `WorkflowAgentPillOpen`，:97-102）：无障碍标签、
/// ↗ 的 testid 与数据属性（侧栏行用它们钉住实例）。真源的动态 `data-*` 展开与
/// 点击回调在 Rust 侧分别由 `data_pairs` / `on_open` 参数承担。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkflowAgentPillOpenData {
    pub label: String,
    pub test_id: Option<String>,
    pub data_pairs: Vec<(String, String)>,
}

/// 药丸尺寸（真源 `size`，:135-136）：`md` 32px；`row`（24px）给侧板名单的两列。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PillSize {
    #[default]
    Md,
    Row,
}

/// 药丸的静态外观（真源 props 的数据部分）。
#[derive(Debug, Clone, Default)]
pub struct WorkflowAgentPillVisual {
    pub avatar_index: Option<usize>,
    /// 入场延迟（一列药丸依次落地，每枚错 30 ms）；`None` 即立刻。
    pub enter_delay_ms: Option<i64>,
    /// 已本地化的显示名（运行时名 > 车道显示名）。
    pub name: String,
    pub lane_class: LaneClass,
    pub status: Option<StepRunStatus>,
    pub title: Option<String>,
    pub class: String,
    /// 不可打开时的提示（「子代理启动后才有会话记录」）；缺席时退回 title / name。
    pub inert_title: Option<String>,
    pub size: PillSize,
}

/// 子代理药丸（真源 `WorkflowAgentPill`，:104-218）。
#[component]
pub fn WorkflowAgentPill(
    visual: WorkflowAgentPillVisual,
    /// 名字之后、状态标记之前的附属信息（真源 children，侧栏行的活动与计数）。
    children: Option<ChildrenFn>,
    /// 状态标记之后的控件（真源 trailing）。
    trailing: Option<ChildrenFn>,
    /// 在场即整枚药丸是按钮（回调的存在即门控）。
    on_open: Option<Callback<()>>,
    /// open 的静态接线。
    open_data: Option<WorkflowAgentPillOpenData>,
) -> impl IntoView {
    let tinted = visual.lane_class == LaneClass::Agent;
    // 有延迟的入场要 backwards 填充：等待期间保持起始帧，否则药丸先满显再闪一下
    // 重新进场。不用 both：forwards 会把 transform 留在元素上（真源 :139-146）。
    let mut style_parts: Vec<String> = Vec::new();
    if tinted {
        style_parts.push(format!(
            "--wf-avatar: {}",
            agent_color(visual.avatar_index, &visual.name)
        ));
    }
    if let Some(delay) = visual.enter_delay_ms {
        if delay > 0 {
            style_parts.push(format!("animation-delay: {delay}ms"));
            style_parts.push("animation-fill-mode: backwards".to_string());
        }
    }
    let style = style_parts.join("; ");

    let settled = visual.status.is_some_and(|s| s != StepRunStatus::Pending);
    let has_tail = settled || on_open.is_some();
    let openable = on_open.is_some();

    // 真源 :212 —— 可打开用 title / name；不可打开 inertTitle 优先。
    let title = if openable {
        visual.title.clone().unwrap_or_else(|| visual.name.clone())
    } else {
        visual
            .inert_title
            .clone()
            .or_else(|| visual.title.clone())
            .unwrap_or_else(|| visual.name.clone())
    };

    let glyph_class = format!(
        "shrink-0 {}",
        if visual.size == PillSize::Row { "size-3.5" } else { "size-4" }
    );
    let glyph_class = if tinted {
        format!("{glyph_class} text-[var(--wf-avatar)]")
    } else {
        format!("{glyph_class} text-foreground-subtle")
    };

    let root_class = format!(
        "wf-pill wf-agent-pill wf-arrive flex rounded-full min-w-0 items-center {} {} {}",
        if visual.size == PillSize::Row {
            "h-6 gap-1.5 pl-1 pr-1.5 text-ui-sm"
        } else {
            "h-8 gap-2 bg-surface pl-2 pr-2.5 text-ui-sm"
        },
        if openable {
            "wf-pill-open cursor-pointer text-left outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
        } else {
            ""
        },
        visual.class,
    );

    let name_class = format!(
        "wf-pill-name min-w-0 flex-1 truncate {}",
        if settled { "text-foreground" } else { "text-foreground-subtle" }
    );
    let status = visual.status;
    let aria_label = open_data.as_ref().map(|o| o.label.clone());
    let test_id = open_data
        .as_ref()
        .and_then(|o| o.test_id.clone())
        .unwrap_or_else(|| "workflow-pill-open".to_string());

    view! {
        {if openable {
            view! {
                <button
                    aria-label=aria_label.clone()
                    class=root_class.clone()
                    data-agent-open="true"
                    data-agent-status=status.map(|s| s.as_str()).unwrap_or("pending")
                    data-pill-size=if visual.size == PillSize::Row { "row" } else { "md" }
                    data-testid="workflow-agent-pill"
                    on:click=move |_| {
                        if let Some(cb) = on_open.clone() {
                            cb.run(());
                        }
                    }
                    style=style.clone()
                    title=title.clone()
                    type="button"
                >
                    <LaneGlyph lane_class=visual.lane_class class=glyph_class.clone() name=visual.name.clone() avatar_index=visual.avatar_index status=status />
                    <span class=name_class.clone()>{visual.name.clone()}</span>
                    {children.clone().map(|c| view! { <>{c()}</> })}
                    {has_tail.then(|| pill_tail(status, Some(test_id.clone())))}
                    {trailing.clone().map(|t| view! { <>{t()}</> })}
                </button>
            }
            .into_any()
        } else {
            view! {
                <span
                    class=root_class.clone()
                    data-agent-status=status.map(|s| s.as_str()).unwrap_or("pending")
                    data-pill-size=if visual.size == PillSize::Row { "row" } else { "md" }
                    data-testid="workflow-agent-pill"
                    style=style.clone()
                    title=title.clone()
                >
                    <LaneGlyph lane_class=visual.lane_class class=glyph_class.clone() name=visual.name.clone() avatar_index=visual.avatar_index status=status />
                    <span class=name_class.clone()>{visual.name.clone()}</span>
                    {children.clone().map(|c| view! { <>{c()}</> })}
                    {has_tail.then(|| pill_tail(status, None))}
                    {trailing.clone().map(|t| view! { <>{t()}</> })}
                </span>
            }
            .into_any()
        }}
    }
}

/// 尾槽（真源 :173-190）：状态标记与 ↗ 叠在同一格 14px，悬停时标记缩出、箭头缩入。
///
/// 真源 `{...open.data}` 的动态 data-* 展开在 Rust 侧没有对应语法；
/// 当前消费方没有实际传入 data 键值，侧栏行迁移时改为逐属性接线。
fn pill_tail(status: Option<StepRunStatus>, open_test_id: Option<String>) -> impl IntoView {
    view! {
        <span
            class="wf-pill-tail grid size-3.5 shrink-0 place-items-center [&>*]:col-start-1 [&>*]:row-start-1"
            data-testid="workflow-pill-tail"
        >
            <PillStatusMark status=status />
            {open_test_id.map(|test_id| {
                view! {
                    <span
                        aria-hidden="true"
                        class="wf-pill-go flex size-3.5 items-center justify-center text-foreground-subtlest"
                        data-testid=test_id
                    >
                        {workflowIcons::icon_arrow_up_right()}
                    </span>
                }
            })}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_word_keys_cover_four_states() {
        // 真源 :66 —— 四态各一词，键前缀一致。
        for status in [
            StepRunStatus::Pending,
            StepRunStatus::Running,
            StepRunStatus::Done,
            StepRunStatus::Failed,
        ] {
            assert!(!status_word(status).is_empty());
            assert_ne!(status_word(status), format!("chat.toolCall.workflow.graph.status.{}", status.as_str()), "缺词条时应能看出来");
        }
    }

    #[test]
    fn visual_defaults_are_pending_md() {
        let visual = WorkflowAgentPillVisual::default();
        assert_eq!(visual.status, None);
        assert_eq!(visual.size, PillSize::Md);
        assert!(visual.class.is_empty());
    }
}
