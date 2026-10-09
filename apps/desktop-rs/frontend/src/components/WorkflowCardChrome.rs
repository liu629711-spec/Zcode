//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowCardChrome.tsx`（285 行）。
//!
//! 工作流卡的表头与页脚。
//!
//! 不走 `ToolLayout`：它的摘要行是 `inline-flex self-start`，右对齐的状态簇放不进去。
//! 表头一行：图标 + 种类词 + 名字，右侧灯 + 状态词 + 等宽细节 + [⤢] + chevron。
//! 站数超过一列时不再有秩带：看不见的站在时间线自己的边檐上。
//!
//! Rust 侧 Button 是内联的类名组合（`ghost icon-md` 等，值照抄真源 Button 的变体类）。

use leptos::prelude::*;

use crate::components::workflow_graph::run_state::WorkflowRunStatus;
use crate::components::workflow_graph::run_status_presentation::{
    read_workflow_run_stop_reason, run_status_dot_class, run_status_text_class,
    status_dot_class, workflow_run_stop_reason_message_id,
};
use crate::components::workflow_graph::types::StepRunStatus;
use crate::components::workflowIcons;
use crate::ToolCallBlocks::i18n;

use crate::ControlHintTooltip::{ControlHintTooltip, ControlHintTooltipData};

/// 卡图标（真源 `WORKFLOW_CARD_ICON`，:24）：`<Workflow class="size-4 shrink-0 text-foreground-subtle" />`。
pub fn workflow_card_icon() -> impl IntoView {
    view! {
        <span class="inline-flex size-4 shrink-0 text-foreground-subtle">
            {workflowIcons::icon_workflow()}
        </span>
    }
}

/// 联接到 run 之后的种类词 message id（真源 `WORKFLOW_RUN_KIND_ID`，:27-33）：
/// 同一个 run 在卡上、轮尾摘要里、通知里、详情页里必须叫同一个名字。
pub fn workflow_run_kind_message_id_of(status: WorkflowRunStatus) -> &'static str {
    match status {
        WorkflowRunStatus::Pending => "chat.toolCall.workflow.card.started",
        WorkflowRunStatus::Running => "chat.toolCall.workflow.card.running",
        WorkflowRunStatus::Completed => "chat.toolCall.workflow.card.completed",
        WorkflowRunStatus::Errored => "chat.toolCall.workflow.card.errored",
        WorkflowRunStatus::Stopped => "chat.toolCall.workflow.card.stopped",
    }
}

/// run 不在活投影里时的中性种类词（真源 `WORKFLOW_RUN_ENDED_KIND_ID`，:38）：
/// 卡只说「这里曾有一条 run」，不冒充某个终态。
pub const WORKFLOW_RUN_ENDED_KIND_ID: &str = "chat.toolCall.workflow.card.ended";

/// 被修订替代的 run 的种类词（真源 :41）。
pub const WORKFLOW_RUN_SUPERSEDED_KIND_ID: &str = "chat.toolCall.workflow.card.superseded";

/// 种类词的唯一入口（真源 `workflowRunKindMessageId`，:47-54）：stopped ∧ superseded
/// 说「已被替代」，其余按状态查表。三处消费都走这里，否则同一个被替代的 run 会在
/// 一处叫「已停止」、另一处叫「已被替代」。
pub fn workflow_run_kind_message_id(
    status: WorkflowRunStatus,
    stop_reason: Option<&str>,
) -> &'static str {
    if crate::components::workflow_graph::run_status_presentation::is_workflow_run_superseded(
        status,
        stop_reason,
    ) {
        WORKFLOW_RUN_SUPERSEDED_KIND_ID
    } else {
        workflow_run_kind_message_id_of(status)
    }
}

/// run 级状态（真源 `WorkflowRunStatus`，:57-116）：灯 + 词，永远成对出现（不变式 3）。
#[component]
pub fn WorkflowRunStatus(
    /// 缺席时取 `run` 的 status（两者至少给一个）。
    status: Option<WorkflowRunStatus>,
    /// 来源 run：`stopped` 时原因词跟在状态词后；`resumable` 为真时再加一个词。
    #[prop(default = WorkflowRunStatusInput::default())] run: WorkflowRunStatusInput,
    #[prop(optional, into)] class: String,
    test_id: Option<String>,
) -> impl IntoView {
    let effective_status = status
        .or(run.status)
        .unwrap_or(WorkflowRunStatus::Pending);
    let reason = if run.has_run {
        read_workflow_run_stop_reason(effective_status, run.stop_reason.as_deref())
    } else {
        None
    };
    let dot = run_status_dot_class(effective_status);
    let text = run_status_text_class(effective_status);
    let status_word = i18n::text(&format!(
        "chat.toolCall.workflow.run.status.{}",
        effective_status.as_str()
    ));
    view! {
        <span class=format!("flex shrink-0 items-center gap-1.5 {class}")>
            <span aria-hidden="true" class=format!("wf-lamp size-2 shrink-0 rounded-full {dot}") />
            // 状态词按值换：旧词退场新词进场（真源 :88-95，key=effectiveStatus）。
            <span
                class=format!("wf-swap text-ui-sm {text}")
                data-testid=test_id.clone()
            >
                {status_word}
            </span>
            {reason.map(|reason| {
                let reason_word = i18n::text(&workflow_run_stop_reason_message_id(reason));
                view! {
                    <span
                        class="text-ui-sm text-foreground-subtlest"
                        data-testid=test_id.as_ref().map(|test| format!("{test}-reason"))
                    >
                        {format!("· {reason_word}")}
                    </span>
                }
            })}
            // 第三个词：这条 run 还能接着跑（真源 :104-113）。「停止」这个动词只说了动作，
            // 说不了后果——把后果放回状态行，用户就不必先点开详情页才知道自己没有丢掉什么。
            {(run.has_run && run.resumable == Some(true)).then(|| {
                view! {
                    <span
                        class="text-ui-sm text-foreground-subtlest"
                        data-testid=test_id.as_ref().map(|test| format!("{test}-resumable"))
                    >
                        {format!("· {}", i18n::text("chat.toolCall.workflow.run.resumable"))}
                    </span>
                }
            })}
        </span>
    }
}

/// `WorkflowRunStatus` 的 run 入参（真源 :63-76 的结构读法：按结构读，不绑死协议类型）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorkflowRunStatusInput {
    pub has_run: bool,
    pub status: Option<WorkflowRunStatus>,
    pub stop_reason: Option<&'static str>,
    pub resumable: Option<bool>,
}

/// 静态（尚未联接 run）的状态（真源 `WorkflowStaticStatus`，:119-128）：
/// 空环灯 + 一个词（「compiled」）。
#[component]
pub fn WorkflowStaticStatus(word: String) -> impl IntoView {
    view! {
        <span class="flex shrink-0 items-center gap-1.5">
            <span
                aria-hidden="true"
                class=format!(
                    "size-2 shrink-0 rounded-full {}",
                    status_dot_class(StepRunStatus::Pending),
                )
            />
            <span class="text-ui-sm text-foreground-subtle" data-testid="workflow-card-static-status">
                {word}
            </span>
        </span>
    }
}

/// 表头的插槽：真实子 view 由调用方构造（真源 leading / trailing 是 ReactNode）。
pub type Slot = Option<ChildrenFn>;

/// 表头一行（真源 `WorkflowCardHeader`，:130-246）。
///
/// Rust 侧的 leading / trailing / status 是「已构造好的 view」：真源把它们当 ReactNode
/// 插槽，Rust 侧用 `Option<AnyView>` 表达同一件事（不跨组件传 View 的规则不适用于
/// 这三个纯插槽——它们没有自己的状态，只是渲染结果）。
#[component]
pub fn WorkflowCardHeader(
    /// 种类词（已本地化）。
    kind: String,
    name: String,
    /// 运行中的种类词扫光（与 ToolLayout 的 isRunning 同一表达）。
    #[prop(default = false)] live: bool,
    status: Option<AnyView>,
    detail: Option<String>,
    /// 细节串的 tooltip；只有子代理模型在场时才给（强度与规范串住在这里）。
    detail_title: Option<String>,
    /// 状态之前的插槽（轮尾摘要的待答问题芯片）。
    leading: Option<AnyView>,
    /// 细节之后、⤢ 之前的插槽（轮尾摘要把 Resume 放进表头）。
    trailing: Option<AnyView>,
    on_open_details: Option<Callback<()>>,
    #[prop(default = false)] expanded: bool,
    /// 缺席即不可折叠（forceOpen / canToggle=false）。
    on_toggle: Option<Callback<()>>,
    /// chevron 的无障碍名；缺席时是工具卡的「展开 / 收起工具详情」。
    toggle_label: Option<String>,
) -> impl IntoView {
    let open_label = i18n::text("chat.toolCall.workflow.openRunDetails");
    let toggle_label = toggle_label.unwrap_or_else(|| {
        i18n::text(if expanded {
            "chat.toolCall.collapseDetails"
        } else {
            "chat.toolCall.expandDetails"
        })
    });
    // 种类词按文案换（key=文案，旧词退场新词进场）。换词的 wf-swap 必须包在扫光的
    // animated-gradient-text **外面**（真源 :177-183 —— background-clip 的裁切陷阱）。
    view! {
        <div class="flex min-w-0 items-center gap-2 text-ui-base" data-testid="workflow-card-header">
            {workflow_card_icon()}
            <span class="shrink-0 whitespace-nowrap font-medium" data-testid="workflow-card-kind">
                <span class="wf-swap">
                    <span class=if live { "animated-gradient-text" } else { "text-foreground" }>
                        {kind.clone()}
                    </span>
                </span>
            </span>
            <span
                class="min-w-0 flex-1 truncate text-foreground-subtle"
                data-testid="workflow-card-name"
                title=name.clone()
            >
                {name.clone()}
            </span>
            // 右簇可压缩，簇里只有细节串会让位（真源 :193-200 的 min-w-0 修复——别当成冗余删掉）。
            <span class="flex min-w-0 items-center gap-2">
                {leading}
                {status}
                {detail.filter(|text| !text.is_empty()).map(|text| {
                    view! {
                        <span
                            class="min-w-0 truncate text-ui-base tabular-nums text-foreground-subtlest"
                            data-testid="workflow-card-detail"
                            title=detail_title.clone()
                        >
                            {text}
                        </span>
                    }
                })}
                {trailing}
                {on_open_details.map(|on_open| {
                    // Callback 不满足 Send+Sync；ChildrenFn 是 Arc<dyn Fn + Send + Sync>，
                    // 经 StoredValue 中转（不可 Clone 的借道值语义）。
                    let children_label = open_label.clone();
                    let on_open = StoredValue::new(on_open);
                    view! {
                        <ControlHintTooltip
                            children=std::sync::Arc::new(move || {
                                let on_open = on_open.get_value();
                                view! {
                                    <button
                                        aria-label=children_label.clone()
                                        data-testid="workflow-card-open-details"
                                        on:click=move |_| on_open.run(())
                                        class="inline-flex h-6 w-6 shrink-0 items-center justify-center gap-1.5 rounded-md text-foreground-subtle hover:bg-surface-hover hover:text-foreground"
                                        type="button"
                                    >
                                        <span class="inline-flex size-3.5">
                                            {workflowIcons::icon_maximize2()}
                                        </span>
                                    </button>
                                }
                                .into_any()
                            })
                            data=ControlHintTooltipData { title: open_label.clone(), ..Default::default() }
                        />
                    }
                })}
                {on_toggle.map(|on_toggle| {
                    view! {
                        <button
                            aria-expanded=expanded
                            aria-label=toggle_label.clone()
                            data-testid="workflow-card-toggle"
                            on:click=move |_| on_toggle.run(())
                            class="inline-flex h-6 w-6 shrink-0 items-center justify-center gap-1.5 rounded-md text-foreground-subtle hover:bg-surface-hover hover:text-foreground"
                            type="button"
                        >
                            <span class=format!(
                                "inline-flex size-4 transition-transform {}",
                                if expanded { "rotate-90" } else { "" },
                            )>
                                {workflowIcons::icon_chevron_right()}
                            </span>
                        </button>
                    }
                })}
            </span>
        </div>
    }
}

/// 页脚摘要行（真源 `WorkflowCardFooter`，:249-285）：灯 + 状态词 + 清单图标 +
/// 各段用 `·` 隔开 + 末尾控件（失败态的 Resume）。
#[component]
pub fn WorkflowCardFooter(
    status: WorkflowRunStatus,
    /// 摘要各段；缺席或为空时只画灯、状态词与 trailing。
    #[prop(default = Vec::new())] parts: Vec<String>,
    trailing: Option<AnyView>,
) -> impl IntoView {
    view! {
        <div
            class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-ui-sm text-foreground-subtle"
            data-testid="workflow-card-footer"
        >
            <WorkflowRunStatus status=Some(status) run=WorkflowRunStatusInput::default() test_id=None />
            {(!parts.is_empty()).then(|| {
                view! {
                    <>
                        <span class="shrink-0 inline-flex size-3.5 text-foreground-subtlest">
                            {workflowIcons::icon_list()}
                        </span>
                        <span class="flex min-w-0 flex-wrap items-center gap-x-2 tabular-nums">
                            {parts
                                .iter()
                                .enumerate()
                                .map(|(i, part)| {
                                    view! {
                                        <span class="flex items-center gap-x-2">
                                            {(i > 0).then(|| {
                                                view! {
                                                    <span aria-hidden="true" class="text-foreground-subtlest">
                                                        {"\u{b7}"}
                                                    </span>
                                                }
                                            })}
                                            <span>{part.clone()}</span>
                                        </span>
                                    }
                                })
                                .collect_view()}
                        </span>
                    </>
                }
            })}
            {trailing}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_word_is_superseded_aware() {
        // 真源 :47-54 —— stopped ∧ superseded 说「已被替代」；其余按状态。
        assert_eq!(
            workflow_run_kind_message_id(WorkflowRunStatus::Stopped, Some("superseded")),
            WORKFLOW_RUN_SUPERSEDED_KIND_ID
        );
        assert_eq!(
            workflow_run_kind_message_id(WorkflowRunStatus::Stopped, Some("user")),
            "chat.toolCall.workflow.card.stopped"
        );
        assert_eq!(
            workflow_run_kind_message_id(WorkflowRunStatus::Completed, None),
            "chat.toolCall.workflow.card.completed"
        );
    }

    #[test]
    fn ended_kind_id_is_neutral() {
        // 真源 :38 —— 不在活投影里：中性词，不冒充终态。
        assert_eq!(WORKFLOW_RUN_ENDED_KIND_ID, "chat.toolCall.workflow.card.ended");
    }

    #[test]
    fn kind_ids_cover_all_five_statuses() {
        for status in [
            WorkflowRunStatus::Pending,
            WorkflowRunStatus::Running,
            WorkflowRunStatus::Completed,
            WorkflowRunStatus::Errored,
            WorkflowRunStatus::Stopped,
        ] {
            assert!(workflow_run_kind_message_id_of(status).starts_with("chat.toolCall.workflow.card."));
        }
    }
}
