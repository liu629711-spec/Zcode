//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowArtifactIndex.tsx`（258 行）。
//!
//! 产物索引（侧板）：交付物行之后的**其余产物**，一件一行。
//!
//! 规则只有一条：**预览要么读得清，要么不画**。其余产物把框丢掉，只剩瓦片说明行那
//! 一套语法——kind 图标、完整标题、等宽细节、尾槽（v{n}，悬停让位给 ↗）——独立成行。
//! 一行 26px。卡上按 `repeat(auto-fit, minmax(220px, 1fr))` 流成两列（窄卡一列）；
//! 侧板恒为一列。与交付物行之间隔一条细线（`--color-workflow-rule`）。
//!
//! 「还有 N 个」是一扇门不是一件产物：↗ 不等悬停就在。每一行永远是一颗 `<button>`。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::artifactPresentation::{
    artifact_detail_text, artifact_display_title, artifact_kind_message_id,
    artifact_kind_icon_view, ArtifactDetailInput,
};
use crate::app_shell::workflow_artifacts::presets::PresetLabels;
use crate::ToolCallBlocks::i18n;
use crate::components::workflowIcons;

use super::WorkflowArtifactTile::WorkflowCompletionArtifact;
use super::WorkflowTimeline::PILL_STAGGER_MS;

/// 入场样式（真源 `enterStyle`，:36-40）。
fn enter_style(enter_delay_ms: Option<i64>) -> String {
    match enter_delay_ms {
        Some(delay) if delay > 0 => {
            format!("animation-delay: {delay}ms; animation-fill-mode: backwards;")
        }
        _ => String::new(),
    }
}

const LINE_CLASS: &str = "wf-line wf-arrive -mx-1.5 flex h-[26px] min-w-0 items-center gap-2 rounded-md bg-transparent px-1.5 text-left text-ui-sm outline-none";

/// 索引列数（真源 `WorkflowArtifactIndexColumns`，:187）：`auto` 是卡上的两列流，
/// `one` 是侧板的单列。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArtifactIndexColumns {
    #[default]
    Auto,
    One,
}

/// 索引行（真源 `WorkflowArtifactLine`，:45-134）。
#[component]
fn WorkflowArtifactLine(
    artifact: WorkflowCompletionArtifact,
    labels: PresetLabels,
    on_open: Option<Callback<String>>,
    enter_delay_ms: Option<i64>,
    #[prop(default = "workflow-artifact-line".to_string())] test_id: String,
    title: Option<String>,
) -> impl IntoView {
    let row_title = artifact_display_title(artifact.pill.id.as_str(), artifact.pill.title.as_deref());
    let kind_label = i18n::text(artifact_kind_message_id(artifact.pill.kind()));
    let detail_input = ArtifactDetailInput {
        kind: artifact.pill.kind(),
        bytes: artifact.bytes,
        item_count: artifact.item_count,
        source_path: artifact.source_path.as_deref(),
        content_type: artifact.content_type.as_deref(),
    };
    let has_detail = artifact_detail_text(&detail_input, &labels).is_some();
    let openable = on_open.is_some();
    let version = artifact.version();
    let show_version = version >= 2;
    let tooltip = title.unwrap_or_else(|| format!("{kind_label} · {row_title}"));
    let artifact_id = artifact.pill.id.clone();
    // aria-label 先算（view! 的 children 先于属性求值，行内 clone 会撞上正文的 move）。
    let aria_label = openable.then(|| {
        format!(
            "{}: {}",
            i18n::text("chat.toolCall.workflow.run.artifacts.open"),
            row_title.clone()
        )
    });

    view! {
        <button
            aria-label=aria_label
            class=format!("{LINE_CLASS} {}", if openable { "wf-line-open cursor-pointer" } else { "cursor-default" })
            data-artifact-id=artifact.pill.id.clone()
            data-artifact-kind=artifact.pill.kind().as_str()
            data-artifact-open=openable.then_some("true")
            data-artifact-version=version.to_string()
            data-testid=test_id
            data-variant="line"
            disabled=!openable
            on:click=move |_| {
                if let Some(cb) = on_open.clone() {
                    cb.run(artifact_id.clone());
                }
            }
            style=enter_style(enter_delay_ms)
            title=tooltip
            type="button"
        >
            <span class="wf-line-icon shrink-0 inline-flex text-foreground-subtle" style="width: 14px; height: 14px;">
                {artifact_kind_icon_view(artifact.pill.kind())}
            </span>
            <span class="min-w-0 flex-1 truncate text-foreground">{row_title}</span>
            {has_detail.then(|| {
                let detail_text = artifact_detail_text(&detail_input, &labels).unwrap_or_default();
                view! {
                    <span
                        class="flex shrink-0 items-center gap-1 font-mono text-ui-xs tabular-nums text-foreground-subtlest"
                        data-testid="workflow-artifact-line-detail"
                    >
                        {detail_text}
                    </span>
                }
            })}
            {(show_version || openable).then(|| {
                view! {
                    <span class="grid size-3 shrink-0 place-items-center [&>*]:col-start-1 [&>*]:row-start-1">
                        {show_version.then(|| {
                            view! {
                                <span
                                    class="wf-mark font-mono text-ui-xs leading-none tabular-nums text-foreground-subtlest"
                                    data-testid="workflow-artifact-tile-version"
                                    title=i18n::format(
                                        "chat.toolCall.workflow.run.artifacts.version",
                                        &[("version".to_string(), version.to_string())],
                                    )
                                >
                                    {i18n::format(
                                        "chat.toolCall.workflow.run.artifacts.versionTail",
                                        &[("version".to_string(), version.to_string())],
                                    )}
                                </span>
                            }
                        })}
                        {openable.then(|| {
                            view! {
                                <span
                                    aria-hidden="true"
                                    class="wf-pill-go flex items-center justify-center text-foreground-subtlest"
                                    data-testid="workflow-artifact-tile-open"
                                >
                                    {workflowIcons::icon_arrow_up_right()}
                                </span>
                            }
                        })}
                    </span>
                }
            })}
        </button>
    }
}

/// 「还有 N 个」——门，不是产物（真源 `WorkflowArtifactMoreLine`，:137-185）。
/// 砍过的清单不知道 N，写 `…`。
#[component]
fn WorkflowArtifactMoreLine(
    count: i64,
    #[prop(optional)] truncated: bool,
    on_open: Option<Callback<()>>,
    enter_delay_ms: Option<i64>,
    #[prop(default = "workflow-artifact-more-line".to_string())] test_id: String,
) -> impl IntoView {
    let openable = on_open.is_some();
    let count_text = if truncated {
        "…".to_string()
    } else {
        count.to_string()
    };
    view! {
        <button
            class=format!("{LINE_CLASS} {}", if openable { "wf-line-open cursor-pointer" } else { "cursor-default" })
            data-testid=test_id
            data-variant="more"
            disabled=!openable
            on:click=move |_| {
                if let Some(cb) = on_open.clone() {
                    cb.run(());
                }
            }
            style=enter_style(enter_delay_ms)
            title=i18n::text("chat.toolCall.workflow.openRunDetails")
            type="button"
        >
            <span class="shrink-0 inline-flex text-foreground-subtlest" style="width: 14px; height: 14px;">
                {workflowIcons::icon_ellipsis()}
            </span>
            <span class="min-w-0 flex-1 truncate text-foreground-subtle">
                {i18n::format(
                    "chat.toolCall.workflow.completion.moreArtifacts",
                    &[("count".to_string(), count_text)],
                )}
            </span>
            {openable.then(|| {
                // 一扇门没有状态标记可让位：↗ 在场即在（wf-pill-go-rest）。
                view! {
                    <span class="grid size-3 shrink-0 place-items-center">
                        <span
                            aria-hidden="true"
                            class="wf-pill-go wf-pill-go-rest flex items-center justify-center text-foreground-subtlest"
                            data-testid="workflow-artifact-tile-open"
                        >
                            {workflowIcons::icon_arrow_up_right()}
                        </span>
                    </span>
                }
            })}
        </button>
    }
}

/// 索引本身（真源 `WorkflowArtifactIndex`，:194-258）：行 + 可选的门。
/// `rule` 在上方画那条细线（跟在交付物行之后时要；索引独占卡时不要）。
/// 行依次落地，从 `firstDelayMs` 起每行错 30ms，门排在最后一行之后。
#[component]
pub fn WorkflowArtifactIndex(
    artifacts: Vec<WorkflowCompletionArtifact>,
    labels: PresetLabels,
    #[prop(default = ArtifactIndexColumns::Auto)] columns: ArtifactIndexColumns,
    #[prop(default = false)] rule: bool,
    /// 画「还有 N 个」；`folded` 是 N。
    #[prop(default = false)] more: bool,
    #[prop(default = 0)] folded: i64,
    #[prop(default = false)] truncated: bool,
    #[prop(default = 0)] first_delay_ms: i64,
    on_open_artifact: Option<Callback<String>>,
    on_open_run: Option<Callback<()>>,
    #[prop(default = "workflow-artifact-index".to_string())] test_id: String,
    line_test_id: Option<String>,
    more_test_id: Option<String>,
    tooltip_of: Option<Callback<usize, String>>,
) -> impl IntoView {
    let line_default = "workflow-artifact-line".to_string();
    let more_default = "workflow-artifact-more-line".to_string();
    view! {
        <div
            class=format!(
                "grid gap-x-5 {} {}",
                match columns {
                    ArtifactIndexColumns::Auto => "grid-cols-[repeat(auto-fit,minmax(220px,1fr))]",
                    ArtifactIndexColumns::One => "grid-cols-1",
                },
                if rule {
                    "border-t border-[var(--color-workflow-rule)] pt-1.5"
                } else {
                    ""
                },
            )
            data-columns=match columns {
                ArtifactIndexColumns::Auto => "auto",
                ArtifactIndexColumns::One => "one",
            }
            data-testid=test_id
        >
            {artifacts
                .iter()
                .enumerate()
                .map(|(index, artifact)| {
                    let on_open = on_open_artifact.clone();
                    let line_test_id = line_test_id.clone().unwrap_or_else(|| line_default.clone());
                    let tooltip = tooltip_of.as_ref().map(|cb| cb.run(index));
                    view! {
                        <WorkflowArtifactLine
                            artifact=artifact.clone()
                            enter_delay_ms=Some(first_delay_ms + PILL_STAGGER_MS * index as i64)
                            labels=labels.clone()
                            on_open=on_open
                            test_id=line_test_id
                            title=tooltip
                        />
                    }
                    .into_any()
                })
                .collect_view()}
            {more.then(|| {
                let on_open_run = on_open_run.clone();
                view! {
                    <WorkflowArtifactMoreLine
                        count=folded
                        enter_delay_ms=Some(first_delay_ms + PILL_STAGGER_MS * artifacts.len() as i64)
                        on_open=on_open_run
                        test_id=more_test_id.clone().unwrap_or_else(|| more_default.clone())
                        truncated=truncated
                    />
                }
                .into_any()
            })}
        </div>
    }
}
