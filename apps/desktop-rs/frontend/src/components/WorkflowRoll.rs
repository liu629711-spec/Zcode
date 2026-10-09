//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowRoll.tsx`（109 行）。
//!
//! 门后的名单：侧板名册站里没被钉住的人，每人一次，按状态分组、组序即注意力序
//! （failed → running → pending → done），组内参与者序。组头是计数行的那一项——
//! 图标、人数、状态词、语义色，右边一条细线拉到边——所以门开着时计数行不必再出现一次。
//! 行由调用方渲染（各面自己的接线），这里只排两列、发入场延迟。
//!
//! 表外的那些（`unlisted`）没有行可落：门与计数行算了它们，名单只能在末尾用一行淡字
//! 交代这个差额，而不是假装那些行在（追记「表外的那些」）。

use leptos::prelude::*;

use crate::components::roster_model::RollGroup;
use crate::components::workflow_graph::types::StepRunStatus;
use crate::components::workflowIcons;
use crate::ToolCallBlocks::i18n;

/// 行依次落地：每行 8 ms、封顶 400 ms（沿用格子的节奏；30 ms 的药丸节奏对一卷名单太慢）。
pub const ROW_STAGGER_MS: i64 = 8;
pub const ROW_STAGGER_CAP_MS: i64 = 400;

/// 组头的语义色（真源 `GROUP_TONE`，:22-27）。
fn group_tone(status: StepRunStatus) -> &'static str {
    match status {
        StepRunStatus::Done => "text-success",
        StepRunStatus::Failed => "text-destructive",
        StepRunStatus::Pending => "text-foreground-subtlest",
        StepRunStatus::Running => "text-warning",
    }
}

/// 组头图标（真源 `GroupIcon`，:29-38）。
fn group_icon(status: StepRunStatus) -> AnyView {
    match status {
        StepRunStatus::Done => view! {
            <span class="inline-block size-2.5">{workflowIcons::icon_circle_check()}</span>
        }
        .into_any(),
        StepRunStatus::Running => view! {
            <span class="inline-block size-2.5 animate-spin motion-reduce:animate-none">
                {workflowIcons::icon_loader_circle()}
            </span>
        }
        .into_any(),
        StepRunStatus::Failed => view! {
            <span class="inline-block size-2.5">{workflowIcons::icon_circle_x()}</span>
        }
        .into_any(),
        StepRunStatus::Pending => view! {
            <span aria-hidden="true" class="inline-block size-2 rounded-full border-[1.5px] border-current" />
        }
        .into_any(),
    }
}

/// 行的入场延迟：真源 :86 `Math.min(ROW_STAGGER_MS * index++, ROW_STAGGER_CAP_MS)`
/// —— index 是**跨组连续**的（这里返回下一个 index，调用方接力）。
pub fn row_delay(index: &mut usize) -> i64 {
    let delay = (ROW_STAGGER_MS * *index as i64).min(ROW_STAGGER_CAP_MS);
    *index += 1;
    delay
}

/// 组头标签的拆分（真源 :60-61）：两种语言的词条都以人数开头（`{count} done` /
/// `{count} 个已完成`），人数加粗、其余照常。
pub fn split_count_prefix(label: &str) -> Option<(String, String)> {
    // /^(\d+)(.*)$/ —— 首段连续数字与其余。
    let digits_end = label.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_end == 0 {
        return None;
    }
    let head: String = label.chars().take(digits_end).collect();
    let tail: String = label.chars().skip(digits_end).collect();
    Some((head, tail))
}

#[component]
pub fn WorkflowRoll(
    groups: Vec<RollGroup>,
    /// 渲染一行；`enter_delay_ms` 是这一行在整卷名单里的落地延迟。
    render_row: Callback<(crate::components::timeline_model::TimelinePill, i64), AnyView>,
    /// 这一站列不出行的子代理数（跑完的、还没跑的都算）；零即那一行淡字缺席。
    #[prop(default = 0)] unlisted: i64,
) -> impl IntoView {
    let mut index: usize = 0;
    view! {
        <div class="wf-unfold grid grid-cols-2 gap-x-2 pt-0.5" data-testid="workflow-roster-roll">
            {groups
                .iter()
                .map(|group| {
                    let label = i18n::format(
                        &format!(
                            "chat.toolCall.workflow.timeline.roster.{}",
                            group.status.as_str()
                        ),
                        &[("count".to_string(), (group.pills.len() as i64).to_string())],
                    );
                    let split = split_count_prefix(&label);
                    let status = group.status;
                    let rows: Vec<AnyView> = group
                        .pills
                        .iter()
                        .map(|pill| render_row.run((pill.clone(), row_delay(&mut index))))
                        .collect();
                    view! {
                        <>
                            <div
                                attr:aria-level="4"
                                class=format!(
                                    "col-span-2 mt-1 flex h-[22px] items-center gap-[5px] pl-1 font-mono text-ui-xs leading-none tabular-nums first:mt-0 {}",
                                    group_tone(status),
                                )
                                data-roll-group=status.as_str()
                                data-testid="workflow-roll-heading"
                                role="heading"
                            >
                                {group_icon(status)}
                                {match split {
                                    None => view! { <span>{label.clone()}</span> }.into_any(),
                                    Some((head, tail)) => view! {
                                        <>
                                            <span class="font-medium">{head}</span>
                                            <span>{tail.trim().to_string()}</span>
                                        </>
                                    }
                                        .into_any(),
                                }}
                                <span aria-hidden="true" class="ml-[3px] h-px flex-1 bg-border" />
                            </div>
                            {rows.into_iter().collect_view()}
                        </>
                    }
                    .into_any()
                })
                .collect_view()}
            {if unlisted <= 0 {
                view! { <span class="hidden" /> }.into_any()
            } else {
                let word = if unlisted == 1 {
                    i18n::format(
                        "chat.toolCall.workflow.timeline.roster.unlisted.one",
                        &[("count".to_string(), unlisted.to_string())],
                    )
                } else {
                    i18n::format(
                        "chat.toolCall.workflow.timeline.roster.unlisted.many",
                        &[("count".to_string(), unlisted.to_string())],
                    )
                };
                view! {
                    <p
                        class="col-span-2 mt-1.5 min-w-0 text-ui-xs text-foreground-subtlest"
                        data-testid="workflow-roll-unlisted"
                    >
                        {word}
                    </p>
                }
                .into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_stagger_is_8ms_capped_at_400() {
        // 真源 :19-20 —— 每行 8ms、封顶 400ms；index 跨组连续递增。
        let mut index = 0;
        assert_eq!(row_delay(&mut index), 0);
        assert_eq!(row_delay(&mut index), 8);
        assert_eq!(index, 2);
        let mut index = 60;
        assert_eq!(row_delay(&mut index), 400);
        assert_eq!(index, 61);
    }

    #[test]
    fn count_prefix_splits_label() {
        // 真源 :61 —— `3 done` / `3 个已完成`：人数加粗、其余照常。
        assert_eq!(
            split_count_prefix("3 个已完成"),
            Some(("3".to_string(), " 个已完成".to_string()))
        );
        assert_eq!(
            split_count_prefix("12 done"),
            Some(("12".to_string(), " done".to_string()))
        );
        assert_eq!(split_count_prefix("还有 n 个"), None);
    }

    #[test]
    fn group_tone_is_attention_palette() {
        assert_eq!(group_tone(StepRunStatus::Done), "text-success");
        assert_eq!(group_tone(StepRunStatus::Failed), "text-destructive");
        assert_eq!(group_tone(StepRunStatus::Running), "text-warning");
        assert_eq!(group_tone(StepRunStatus::Pending), "text-foreground-subtlest");
    }
}
