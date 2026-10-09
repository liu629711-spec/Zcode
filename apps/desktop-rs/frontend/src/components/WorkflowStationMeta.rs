//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowStationMeta.tsx`（33 行）。
//!
//! 站头元数据（从 `WorkflowTimeline.tsx` 拆出以守 400 行）：`⟳ n` 轮次（只在循环上的站）
//! 与 `a/b` 步数分数（只在观察到节点后）。

use leptos::prelude::*;

use crate::components::timeline_model::TimelineStation;
use crate::ToolCallBlocks::i18n;

#[component]
pub fn StationMeta(station: TimelineStation) -> impl IntoView {
    let show_rounds = station.on_loop && station.rounds > 0;
    if !show_rounds && station.fraction.is_none() {
        return view! { <span class="hidden" /> }.into_any();
    }
    let rounds_title = i18n::format(
        "chat.toolCall.workflow.timeline.rounds",
        &[("count".to_string(), station.rounds.to_string())],
    );
    view! {
        <span class="flex shrink-0 items-center gap-1.5 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
            {show_rounds.then(|| {
                view! {
                    <span
                        class="flex items-center gap-1"
                        data-testid="workflow-timeline-rounds"
                        title=rounds_title.clone()
                    >
                        <span aria-hidden="true" class="inline-block size-2.5">
                            {crate::components::workflowIcons::icon_repeat2()}
                        </span>
                        {station.rounds}
                    </span>
                }
            })}
            {(show_rounds && station.fraction.is_some()).then(|| {
                view! { <span aria-hidden="true">{"\u{b7}"}</span> }
            })}
            {station.fraction.as_ref().map(|fraction| {
                view! {
                    <span class="text-ui-sm" data-testid="workflow-timeline-fraction">
                        {format!("{}/{}", fraction.settled, fraction.observed)}
                    </span>
                }
            })}
        </span>
    }
    .into_any()
}
