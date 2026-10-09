//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowMoreRow.tsx`（142 行）。
//!
//! 「还有 n 个」那一行：卡上名册站钉住五枚药丸之后的第六枚——同高、同底色、同内距、
//! 同圆角、同一套悬停语法。内容是三张脸一叠（其余里最要紧的三个，表情自带状态）、
//! `还有 n 个`、藏着失败时一枚红色 `✕ n`、尾槽里常驻的 ↗。
//!
//! 它是一扇门不是状态：点一下开运行侧栏、落到这一站、把清单展开（`onOpen` 由卡接到
//! `onSelectStation`）。没有回调时是静态的 span。
//! 侧板上同一具身体是**门**（`door` 在场）：尾槽里换成下箭头，原地开合。

use leptos::prelude::*;

use crate::components::roster_model::{RosterCounts, RosterMore};
use crate::components::workflowIcons;
use crate::components::WorkflowAgentPill::LaneGlyph;
use crate::components::WorkflowRosterParts::RosterTally;
use crate::ToolCallBlocks::i18n;

fn pill_name_of(
    runtime_name: Option<&str>,
    lane: &crate::components::workflow_graph::lane_name::LaneRef,
) -> String {
    // 真源 :39 —— 运行时名 > 车道显示名。
    if let Some(name) = runtime_name {
        return name.to_string();
    }
    crate::components::workflow_graph::lane_name::lane_display_name(
        &lane.naming,
        &|id: &str| i18n::text(id),
    )
}

#[component]
pub fn WorkflowMoreRow(
    more: RosterMore,
    /// 入场延迟（跟在钉住的药丸之后落地）；缺席即立刻。
    enter_delay_ms: Option<i64>,
    /// 在场即整行是按钮（回调的存在即门控）。
    on_open: Option<Callback<()>>,
    /// 门的形态（侧板）：开合状态与其余人的计数。缺席即卡上那一行（↗ 常驻）。
    door: Option<(bool, RosterCounts)>,
) -> impl IntoView {
    let count = more.count;
    // 真源 :40-54 —— 文案与 title 按「卡上」还是「门」取词。
    let label = i18n::format(
        "chat.toolCall.workflow.timeline.roster.more",
        &[("count".to_string(), count.to_string())],
    );
    let door_open = door.as_ref().map(|(open, _)| *open);
    let title = i18n::format(
        match door_open {
            None => "chat.toolCall.workflow.timeline.roster.moreTitle",
            Some(true) => "chat.toolCall.workflow.timeline.roster.door.fold",
            Some(false) => "chat.toolCall.workflow.timeline.roster.door.list",
        },
        &[("count".to_string(), count.to_string())],
    );
    let style = enter_delay_ms
        .filter(|delay| *delay > 0)
        .map(|delay| format!("animation-delay: {delay}ms; animation-fill-mode: backwards"));

    let openable = on_open.is_some();
    let root_class = format!(
        "wf-pill wf-agent-pill wf-more-row wf-arrive flex h-8 min-w-0 items-center gap-2 rounded-full pl-2 pr-2.5 text-ui-sm {} {}",
        if door_open == Some(true) { "bg-surface-hover" } else { "bg-surface" },
        if openable {
            "wf-pill-open cursor-pointer text-left outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
        } else {
            ""
        },
    );

    // 行身两支共用一份构造：AnyView 不可克隆，改为每次现建（数据捕获后 clone 进闭包）。
    let body_for_button = {
        let label = label.clone();
        let door = door.clone();
        let more = more.clone();
        move || more_row_body(&more, label.clone(), door.clone())
    };
    let body_for_span = {
        let label = label.clone();
        let door = door.clone();
        let more = more.clone();
        move || more_row_body(&more, label.clone(), door.clone())
    };

    view! {
        {if openable {
            view! {
                <button
                    aria-label=title.clone()
                    aria-expanded=door_open
                    class=root_class
                    data-door=door_open.map(|open| if open { "open" } else { "closed" })
                    data-more-count=count
                    data-testid="workflow-roster-more-row"
                    on:click=move |_| {
                        if let Some(cb) = on_open.clone() {
                            cb.run(());
                        }
                    }
                    style=style.clone()
                    title=title.clone()
                    type="button"
                >
                    {body_for_button()}
                </button>
            }
            .into_any()
        } else {
            view! {
                <span
                    aria-label=title.clone()
                    aria-expanded=door_open
                    class=root_class
                    data-door=door_open.map(|open| if open { "open" } else { "closed" })
                    data-more-count=count
                    data-testid="workflow-roster-more-row"
                    style=style.clone()
                    title=title.clone()
                >
                    {body_for_span()}
                </span>
            }
            .into_any()
        }}
    }
}

/// 行身（真源 :79-139）：三张脸一叠 + 计数词 + （门）其余人计数行 + （卡）✕ n + 尾槽。
fn more_row_body(more: &RosterMore, label: String, door: Option<(bool, RosterCounts)>) -> AnyView {
    let failed = more.failed;
    let door_open = door.as_ref().map(|(open, _)| *open);
    // 三张脸一叠（真源 :79-90）。
    let deck: Vec<AnyView> = more
        .deck
        .iter()
        .map(|pill| {
            let name = pill_name_of(pill.runtime_name.as_deref(), &pill.lane);
            view! {
                <LaneGlyph
                    class="wf-more-face size-4 shrink-0 text-foreground-subtle"
                    avatar_index=pill.avatar_index
                    lane_class=pill.lane_class
                    name=name
                    status=pill.status
                />
            }
            .into_any()
        })
        .collect();
    view! {
        <>
            <span class="wf-more-deck flex shrink-0 items-center" data-testid="workflow-more-deck">
                {deck.into_iter().collect_view()}
            </span>
            <span class=format!(
                "wf-pill-name min-w-0 flex-1 truncate {}",
                if door_open == Some(true) { "text-foreground" } else { "text-foreground-subtle" },
            )>
                {label}
            </span>
            {match &door {
                Some((false, tally)) => {
                    view! { <RosterTally class="mr-0.5 shrink-0" counts=tally.clone() /> }.into_any()
                }
                _ => view! { <span class="hidden" /> }.into_any(),
            }}
            {if door.is_none() && failed > 0 {
                let title = i18n::format(
                    "chat.toolCall.workflow.timeline.roster.failed",
                    &[("count".to_string(), failed.to_string())],
                );
                view! {
                    <span
                        class="flex shrink-0 items-center gap-[3px] font-mono text-ui-xs tabular-nums text-destructive"
                        data-testid="workflow-more-failed"
                        title=title
                    >
                        <span class="inline-block size-2.5">{workflowIcons::icon_circle_x()}</span>
                        <span class="font-medium">{failed}</span>
                    </span>
                }
                .into_any()
            } else {
                view! { <span class="hidden" /> }.into_any()
            }}
            <span
                class="wf-pill-tail grid size-3.5 shrink-0 place-items-center"
                data-testid="workflow-pill-tail"
            >
                {if door.is_none() {
                    // 卡上：↗ 常驻（空尾槽读作「这里没东西」）。
                    view! {
                        <span
                            aria-hidden="true"
                            class="wf-pill-go wf-pill-go-rest flex size-3.5 items-center justify-center text-foreground-subtlest"
                            data-testid="workflow-more-open"
                        >
                            {workflowIcons::icon_arrow_up_right()}
                        </span>
                    }
                    .into_any()
                } else {
                    // 侧板：下箭头，开着时翻转（原地开合）。
                    let rotated = door_open == Some(true);
                    view! {
                        <span
                            aria-hidden="true"
                            class=format!(
                                "wf-pill-go wf-pill-go-rest flex size-3.5 items-center justify-center text-foreground-subtlest transition-transform {}",
                                if rotated { "rotate-180" } else { "" },
                            )
                            data-testid="workflow-more-chevron"
                        >
                            {workflowIcons::icon_chevron_down()}
                        </span>
                    }
                    .into_any()
                }}
            </span>
        </>
    }
    .into_any()
}
