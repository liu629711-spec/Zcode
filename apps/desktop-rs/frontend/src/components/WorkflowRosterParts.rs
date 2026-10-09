//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowRosterParts.tsx`（115 行）。
//!
//! 阶段名册的两件小零件：计数行与量条。
//!
//! 量条的段序是 done · failed · running · pending（从左到右随阶段结算填满）；
//! 计数行的段序是注意力序 done · running · failed · pending。两者不同是**故意的**：
//! 计数行按「先说要紧的」读，量条按「结算进度」读。

use leptos::prelude::*;

use crate::components::roster_model::RosterCounts;
use crate::components::workflow_graph::types::StepRunStatus;
use crate::components::workflowIcons;
use crate::ToolCallBlocks::i18n;

/// 计数行的段序（真源 `COUNT_ORDER`，:10）：注意力序。
pub const COUNT_ORDER: [StepRunStatus; 4] = [
    StepRunStatus::Done,
    StepRunStatus::Running,
    StepRunStatus::Failed,
    StepRunStatus::Pending,
];

/// 计数词（真源 `useCountLabels`，:12-25）。
fn count_label(status: StepRunStatus, count: i64) -> String {
    let id = format!("chat.toolCall.workflow.timeline.roster.{}", status.as_str());
    i18n::format(&id, &[("count".to_string(), count.to_string())])
}

fn status_count(counts: &RosterCounts, status: StepRunStatus) -> i64 {
    match status {
        StepRunStatus::Done => counts.done,
        StepRunStatus::Running => counts.running,
        StepRunStatus::Failed => counts.failed,
        StepRunStatus::Pending => counts.pending,
    }
}

/// 计数行（真源 `RosterTally`，:28-68）：`✓ n · ◌ n · ✕ n · ○ n`，为零的项缺席；
/// 每项的 title 是整句。
#[component]
pub fn RosterTally(counts: RosterCounts, #[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <div
            class=format!(
                "flex h-3.5 items-center gap-2.5 font-mono text-ui-xs leading-none tabular-nums {class}",
            )
            data-testid="workflow-roster-tally"
        >
            {COUNT_ORDER
                .iter()
                .copied()
                .filter(|status| status_count(&counts, *status) > 0)
                .map(|status| {
                    let count = status_count(&counts, status);
                    let title = count_label(status, count);
                    let tone = match status {
                        StepRunStatus::Done => "text-success",
                        StepRunStatus::Running => "text-warning",
                        StepRunStatus::Failed => "text-destructive",
                        StepRunStatus::Pending => "text-foreground-subtlest",
                    };
                    let icon = match status {
                        StepRunStatus::Done => view! {
                            <span class="inline-block size-2.5">
                                {workflowIcons::icon_circle_check()}
                            </span>
                        }
                        .into_any(),
                        StepRunStatus::Running => view! {
                            <span class="inline-block size-2.5 animate-spin motion-reduce:animate-none">
                                {workflowIcons::icon_loader_circle()}
                            </span>
                        }
                        .into_any(),
                        StepRunStatus::Failed => view! {
                            <span class="inline-block size-2.5">
                                {workflowIcons::icon_circle_x()}
                            </span>
                        }
                        .into_any(),
                        StepRunStatus::Pending => view! {
                            <span
                                aria-hidden="true"
                                class="inline-block size-2 rounded-full border-[1.5px] border-current"
                            />
                        }
                        .into_any(),
                    };
                    view! {
                        <span class=format!("flex items-center gap-[3px] {tone}") data-roster-count=status.as_str() title=title>
                            {icon}
                            <span class="font-medium">{count}</span>
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// 量条（真源 `RosterMeter`，:74-115）：段序 done · failed · running · pending，
/// 从左到右随阶段结算填满；pending 段用 `--color-border` 作轨道。段宽随计数过渡
/// （`.wf-meter-seg`）。`mini` 是折叠节头上的 44 px 版本。
#[component]
pub fn RosterMeter(
    counts: RosterCounts,
    #[prop(optional, into)] class: String,
    #[prop(default = false)] mini: bool,
) -> impl IntoView {
    // 段序与计数行不同：这是结算序（真源 :84）。
    let order = [
        StepRunStatus::Done,
        StepRunStatus::Failed,
        StepRunStatus::Running,
        StepRunStatus::Pending,
    ];
    let aria_label = COUNT_ORDER
        .iter()
        .map(|status| count_label(*status, status_count(&counts, *status)))
        .collect::<Vec<_>>()
        .join(", ");
    view! {
        <div
            aria-label=aria_label
            class=format!(
                "flex h-[3px] gap-px overflow-hidden rounded-xs {} {class}",
                if mini { "w-11 shrink-0" } else { "" },
            )
            data-testid=if mini { "workflow-roster-meter-mini" } else { "workflow-roster-meter" }
            role="img"
        >
            {order
                .iter()
                .copied()
                .filter(|status| status_count(&counts, *status) > 0)
                .map(|status| {
                    let count = status_count(&counts, status);
                    let title = count_label(status, count);
                    let tone = match status {
                        StepRunStatus::Done => "bg-success",
                        StepRunStatus::Failed => "bg-destructive",
                        StepRunStatus::Running => "bg-warning",
                        StepRunStatus::Pending => "bg-border",
                    };
                    view! {
                        <span
                            class=format!("wf-meter-seg min-w-0.5 basis-0 {tone}")
                            data-meter-segment=status.as_str()
                            style=format!("flex-grow: {count}")
                            title=title
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_order_is_attention_order() {
        // 真源 :10 —— done → running → failed → pending。
        assert_eq!(
            COUNT_ORDER,
            [
                StepRunStatus::Done,
                StepRunStatus::Running,
                StepRunStatus::Failed,
                StepRunStatus::Pending
            ]
        );
    }

    #[test]
    fn count_labels_carry_the_number() {
        // 计数词把人数插进 {count}。
        let label = count_label(StepRunStatus::Done, 3);
        assert!(label.contains('3'), "计数词应插值人数：{label}");
    }

    #[test]
    fn meter_segments_skip_zero_counts() {
        // 零计数的段不画（真源 :97 filter）。
        let counts = RosterCounts {
            done: 2,
            running: 0,
            failed: 1,
            pending: 0,
        };
        let visible: Vec<StepRunStatus> = order_filter(&counts);
        assert_eq!(visible, vec![StepRunStatus::Done, StepRunStatus::Failed]);
    }

    fn order_filter(counts: &RosterCounts) -> Vec<StepRunStatus> {
        [
            StepRunStatus::Done,
            StepRunStatus::Failed,
            StepRunStatus::Running,
            StepRunStatus::Pending,
        ]
        .iter()
        .copied()
        .filter(|status| status_count(counts, *status) > 0)
        .collect()
    }
}
