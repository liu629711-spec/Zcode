//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowArtifactRow.tsx`（167 行）。
//!
//! 交付物行（侧板）：run 的 primary 产物在完成卡与 run 侧板上的样子。
//!
//! 它是**侧躺的瓦片**：同一只 16:10 预览框（160 × 100），框旁是 kind 图标 +
//! 比说明行大一级的标题、作者写的 description（三行截断）、一行等宽的 `kind · size`，
//! 尾槽是瓦片的（v{n}，悬停让位给 ↗）。强调来自位置、形态与文字，从不来自任何标签。
//!
//! 面板窄于 380px 时（容器查询 `wf-artifacts`）框收成 136 × 85；完成卡不声明容器，恒 160 × 100。
//! 与瓦片同一个结构：预览框是按钮的兄弟节点并标 `inert`，按钮只包文字列。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::artifactPresentation::{
    artifact_detail_text, artifact_display_title, artifact_kind_message_id,
    artifact_kind_icon_view, ArtifactDetailInput,
};
use crate::ToolCallBlocks::i18n;

use super::WorkflowArtifactTile::{ArtifactSheetGlyph, WorkflowCompletionArtifact};

/// 交付物行（真源 `WorkflowArtifactRow`，:32-166）。
#[component]
pub fn WorkflowArtifactRow(
    artifact: WorkflowCompletionArtifact,
    /// 三句预置译文（`artifactDetailText` 要用）。
    labels: crate::app_shell::workflow_artifacts::presets::PresetLabels,
    /// 预览框的内容；缺席即纸页字形（由调用方交 `ArtifactSheetGlyph`，与瓦片一致）。
    preview: Option<AnyView>,
    on_open: Option<Callback<String>>,
    enter_delay_ms: Option<i64>,
    #[prop(default = "workflow-artifact-row".to_string())] test_id: String,
    /// tooltip 覆盖（侧板把工作区出处放进来）；缺席时是「种类词 · 标题」。
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
    let description = artifact
        .description
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let openable = on_open.is_some();
    let version = artifact.version();
    let show_version = version >= 2;
    let style = enter_delay_ms
        .filter(|delay| *delay > 0)
        .map(|delay| format!("animation-delay: {delay}ms; animation-fill-mode: backwards"));
    let tooltip = title.unwrap_or_else(|| format!("{kind_label} · {row_title}"));
    let artifact_id = artifact.pill.id.clone();

    view! {
        <div
            class="wf-tile relative grid w-full min-w-0 grid-cols-[160px_minmax(0,1fr)] items-start gap-3 @max-[380px]/wf-artifacts:grid-cols-[136px_minmax(0,1fr)]"
            data-variant="row"
        >
            <div
                aria-hidden="true"
                class="wf-tile-frame wf-arrive relative h-[100px] w-[160px] overflow-hidden rounded-lg border border-border bg-panel @max-[380px]/wf-artifacts:h-[85px] @max-[380px]/wf-artifacts:w-[136px]"
                data-testid="workflow-artifact-row-frame"
                inert="true"
                style=style.clone().unwrap_or_default()
            >
                {preview.unwrap_or_else(|| {
                    view! { <ArtifactSheetGlyph badge=None /> }.into_any()
                })}
            </div>
            <button
                aria-label=if openable {
                    Some(format!(
                        "{}: {}",
                        i18n::text("chat.toolCall.workflow.run.artifacts.open"),
                        row_title.clone()
                    ))
                } else {
                    None
                }
                class=format!(
                    "wf-tile-hit wf-arrive flex min-w-0 flex-col gap-0.5 rounded-lg bg-transparent p-0 pt-px text-left outline-none {}",
                    if openable { "wf-tile-open cursor-pointer" } else { "cursor-default" },
                )
                data-artifact-id=artifact.pill.id.clone()
                data-artifact-kind=artifact.pill.kind().as_str()
                data-artifact-open=openable.then_some("true")
                data-artifact-version=version.to_string()
                data-testid=test_id
                data-variant="row"
                disabled=!openable
                on:click=move |_| {
                    if let Some(cb) = on_open.clone() {
                        cb.run(artifact_id.clone());
                    }
                }
                style=style.unwrap_or_default()
                title=tooltip
                type="button"
            >
                <span class="flex h-5 min-w-0 items-center gap-1.5">
                    <span class="shrink-0 inline-flex text-foreground-subtle">
                        {artifact_kind_icon_view(artifact.pill.kind())}
                    </span>
                    <span
                        class="wf-pill-name min-w-0 flex-1 truncate text-ui-base font-medium text-foreground"
                        data-testid="workflow-artifact-row-title"
                    >
                        {row_title.clone()}
                    </span>
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
                                            {crate::components::workflowIcons::icon_arrow_up_right()}
                                        </span>
                                    }
                                })}
                            </span>
                        }
                    })}
                </span>
                {description.map(|description_text| {
                    // 作者没写 description 时这一行消失、行仍在（真源 :141-149）。
                    view! {
                        <span
                            class="line-clamp-3 text-ui-sm text-foreground-subtle [text-wrap:pretty]"
                            data-testid="workflow-artifact-row-description"
                        >
                            {description_text}
                        </span>
                    }
                })}
                <span
                    class="mt-0.5 flex h-4 min-w-0 items-center gap-1.5 font-mono text-ui-xs tabular-nums text-foreground-subtlest"
                    data-testid="workflow-artifact-row-detail"
                >
                    <span>{kind_label}</span>
                    {has_detail.then(|| {
                        let detail_text = artifact_detail_text(&detail_input, &labels).unwrap_or_default();
                        view! {
                            <>
                                <span aria-hidden="true">{"\u{b7}"}</span>
                                <span class="flex min-w-0 items-center gap-1 truncate">
                                    {detail_text}
                                </span>
                            </>
                        }
                    })}
                </span>
            </button>
        </div>
    }
}
