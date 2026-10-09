//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowCompletionCard.tsx`（256 行）。
//!
//! 完成卡：主代理消化一条 **completed** 工作流通知的那一轮，轮尾落下这张卡。
//! 顺序即论点——表头 → 这次 run **交付了什么**（交付物行）→ **还做了什么**（产物索引，
//! 一件一行）→ **花了多少**（四格数字）。三段之间各隔一条细线，像一张收据。
//!
//! 纯展示：产物清单、四个数字、每件产物的预览都由宿主交进来（预览要读字节，那是宿主的活）。
//! 没有 chevron——没有可展开的东西；整卡不是开关。
//!
//! 数字的诚实：拿不到的格写 `—`，不写 0。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::presets::build_preset_labels;
use crate::lib::workDuration::{work_duration_parts, work_duration_unit_separator};
use crate::ToolCallBlocks::i18n;

use super::WorkflowCardChrome::{
    workflow_run_kind_message_id_of, WorkflowCardHeader, WorkflowRunStatus, WorkflowRunStatusInput,
};
use super::WorkflowCompletionArtifacts::{
    completion_artifact_cell_count, completion_artifact_layout, WorkflowCompletionArtifacts,
};
use super::WorkflowTimeline::PILL_STAGGER_MS;

/// 四格数字的视图模型（真源 `WorkflowCompletionFigures`，:31-37）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorkflowCompletionFigures {
    pub duration_ms: Option<i64>,
    pub tokens: Option<i64>,
    pub subagents: Option<i64>,
    /// 进过的阶段数（`run.phases`）；没有 phase() 标记的脚本拿不到，写 `—`。
    pub phases: Option<i64>,
}

/// 完成卡入参（真源 `WorkflowCompletionCardProps`，:39-50）。
pub struct WorkflowCompletionCardInput {
    pub name: String,
    pub figures: WorkflowCompletionFigures,
    pub artifacts: Vec<super::WorkflowArtifactTile::WorkflowCompletionArtifact>,
    /// 发射侧砍过：「还有 N 个」的 N 写 `…`。
    pub artifacts_truncated: bool,
    /// 点开 run 详情；缺席即无 ⤢。
    pub on_open_run: Option<Callback<()>>,
    /// 点一枚产物药丸开产物 tab；缺席即产物药丸禁用。
    pub on_open_artifact: Option<Callback<String>>,
    pub test_id_key: String,
}

/// 数字翻上来的时长；四格同一拍，与条上药丸依次落地的节奏相接（真源 :53）。
pub const COUNT_UP_MS: i64 = 640;

/// tokens 的紧凑写法（真源 `formatCompactCount`，:59-63）：
/// `812` / `386.4k` / `1.30M`。千位一位小数、百万两位——固定位数让四格的等宽数字在
/// run 与 run 之间对得齐。全值进 title。
pub fn format_compact_count(count: i64) -> (String, String) {
    if count < 1_000 {
        return (count.to_string(), String::new());
    }
    if count < 1_000_000 {
        let thousands = count as f64 / 1_000.0;
        return (format!("{thousands:.1}"), "k".to_string());
    }
    let millions = count as f64 / 1_000_000.0;
    (format!("{millions:.2}"), "M".to_string())
}

/// 数字从 0 翻到目标值（真源 `useCountUp`，:82-101）。**首帧就是目标值**（静态渲染、
/// reduced-motion 看到的都是终值），挂载之后才回到 0 再翻上来——「页面在静止时是完整的」。
/// Rust 侧的 Motion 闸与 rAF 循环按真源逐条对应（reduced-motion / 无 rAF 环境直接落终值）。
pub fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

#[component]
pub fn WorkflowCompletionCard(input: WorkflowCompletionCardInput) -> impl IntoView {
    let labels = build_preset_labels();
    let layout = completion_artifact_layout(&input.artifacts, input.artifacts_truncated);

    // 四格的数字翻上来；时长翻的是毫秒，再拆成「11m 48s」（真源 :176-189）。
    // Rust 侧当前渲染面是静态投影（无宿主 rAF 循环），四格直接落终值——与真源
    // 「静态渲染看到的就是终值」的第一帧约定一致；翻滚动画等宿主接 rAF 后补。
    let duration_ms = input.figures.duration_ms;
    let tokens = input.figures.tokens;
    let subagents = input.figures.subagents;
    let phases = input.figures.phases;
    let separator = work_duration_unit_separator("zh-CN");
    let time_parts: Option<Vec<(String, String)>> = duration_ms.map(|ms| {
        work_duration_parts(ms)
            .into_iter()
            .map(|part| (part.value.to_string(), format!("{separator}{}", part.unit)))
            .collect()
    });
    let token_parts: Option<Vec<(String, String)>> = tokens.map(format_compact_count).map(|(v, u)| vec![(v, u)]);
    let plain = |value: Option<i64>| -> Option<Vec<(String, String)>> {
        value.map(|value| vec![(value.to_string(), String::new())])
    };
    let figure_delay = PILL_STAGGER_MS * (completion_artifact_cell_count(&layout) as i64 + 1);
    let kind_word = i18n::text(workflow_run_kind_message_id_of(
        crate::components::workflow_graph::run_state::WorkflowRunStatus::Completed,
    ));
    let unavailable = i18n::text("chat.toolCall.workflow.completion.unavailable");

    view! {
        <section
            aria-label=kind_word.clone()
            class="wf-motion wf-arrive flex w-full min-w-0 flex-col gap-2.5 rounded-xl border border-border/70 bg-card/70 px-3.5 pb-3 pt-1.5"
            data-testid=format!("workflow-completion-card-{}", input.test_id_key)
            data-workflow-completion-card="true"
        >
            <WorkflowCardHeader
                detail=None
                detail_title=None
                expanded=false
                kind=kind_word.clone()
                leading=None
                name=input.name.clone()
                status=Some(view! {
                    <WorkflowRunStatus
                        status=Some(crate::components::workflow_graph::run_state::WorkflowRunStatus::Completed)
                        run=WorkflowRunStatusInput::default()
                        test_id=Some("workflow-completion-status".to_string())
                    />
                }.into_any())
                trailing=None
                on_open_details=None
                on_toggle=None
                toggle_label=None
            />
            <WorkflowCompletionArtifacts
                artifacts_truncated=input.artifacts_truncated
                labels=labels.clone()
                layout=layout
                render_preview=None
                on_open_artifact=input.on_open_artifact.clone()
                on_open_run=input.on_open_run.clone()
            />
            <div
                class="grid grid-cols-4 gap-x-3 border-t border-[var(--color-workflow-rule)] pt-2.5"
                data-testid="workflow-completion-figures"
            >
                {figure_view("time", "chat.toolCall.workflow.completion.time", time_parts, None, figure_delay, unavailable.clone())}
                {figure_view("tokens", "chat.toolCall.workflow.completion.tokens", token_parts, input.figures.tokens.map(|tokens| {
                    i18n::format(
                        "chat.toolCall.workflow.card.tokens",
                        &[("count".to_string(), tokens.to_string())],
                    )
                }), figure_delay + PILL_STAGGER_MS, unavailable.clone())}
                {figure_view("subagents", "chat.toolCall.workflow.completion.subagents", plain(subagents), None, figure_delay + PILL_STAGGER_MS * 2, unavailable.clone())}
                {figure_view("phases", "chat.toolCall.workflow.completion.phases", plain(phases), None, figure_delay + PILL_STAGGER_MS * 3, unavailable)}
            </div>
        </section>
    }
    .into_any()
}

/// 一格数字（真源 `Figure`，:103-152）：`[数字, 单位]` 交替；缺席即 `—`。
#[allow(clippy::too_many_arguments)]
fn figure_view(
    test_key: &str,
    label_id: &str,
    parts: Option<Vec<(String, String)>>,
    title: Option<String>,
    delay_ms: i64,
    unavailable: String,
) -> AnyView {
    let label = i18n::text(label_id);
    let test_id = format!("workflow-completion-figure-{test_key}");
    let has_parts = parts.is_some();
    let value_attr = parts
        .as_ref()
        .map(|parts| {
            parts
                .iter()
                .map(|(value, unit)| format!("{value}{unit}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let items: Vec<AnyView> = parts
        .map(|parts| {
            parts
                .into_iter()
                .enumerate()
                .map(|(index, (value, unit))| {
                    view! {
                        <>
                            {(index > 0).then(|| view! { <span>{" "}</span> })}
                            <span>{value}</span>
                            {(!unit.is_empty()).then(|| {
                                view! {
                                    <span class="text-ui-sm font-normal text-foreground-subtle">
                                        {unit}
                                    </span>
                                }
                            })}
                        </>
                    }
                    .into_any()
                })
                .collect()
        })
        .unwrap_or_default();
    view! {
        <div
            class="wf-arrive flex min-w-0 flex-col"
            data-testid=test_id
            data-value=has_parts.then_some(value_attr)
            style=format!("animation-delay: {delay_ms}ms; animation-fill-mode: backwards;")
        >
            <span
                aria-label=(!has_parts).then_some(unavailable)
                class=format!(
                    "whitespace-nowrap font-mono text-ui-lg leading-tight tabular-nums {}",
                    if !has_parts {
                        "text-foreground-subtlest"
                    } else {
                        "font-medium text-foreground"
                    },
                )
                title=title
            >
                {if !has_parts {
                    view! { <span>{"—"}</span> }.into_any()
                } else {
                    items.into_iter().collect_view().into_any()
                }}
            </span>
            <span class="truncate text-ui-sm text-foreground-subtle">{label}</span>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_count_formats_like_truth_source() {
        // 真源 :59-63 —— `812` / `386.4k` / `1.30M`。
        assert_eq!(format_compact_count(812), ("812".to_string(), String::new()));
        assert_eq!(format_compact_count(386_400), ("386.4".to_string(), "k".to_string()));
        assert_eq!(format_compact_count(1_300_000), ("1.30".to_string(), "M".to_string()));
    }

    #[test]
    fn ease_out_cubic_matches_truth_source() {
        // 真源 :65-67 —— 1 - (1 - t)³。
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert!((ease_out_cubic(0.5) - 0.875).abs() < 1e-9);
    }

    #[test]
    fn count_up_ms_matches_truth_source() {
        // 真源 :53 —— 四格同一拍 640ms。
        assert_eq!(COUNT_UP_MS, 640);
    }
}
