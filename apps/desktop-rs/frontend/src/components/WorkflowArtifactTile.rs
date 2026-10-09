//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowArtifactTile.tsx`（177 行）。
//!
//! 产物瓦片：一件产物 = 预览区 + 说明行。说明行是产物药丸的语法原样；预览区是产物
//! **本身**的缩略，由调用方按 kind 交进来（Rust 侧交「预览 view」，交不出来即纸页字形）。
//!
//! 整张瓦片永远是一颗 `<button>`：宿主没给回调时是**禁用**的按钮，与药丸同一条门。
//! 预览框是按钮的**兄弟**节点并标 `inert`（真源 :27-31——避免 button 嵌 button），
//! 按钮只包说明行，用铺满整张瓦片的 `::after`（`.wf-tile-hit`）接住整块的点击。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::artifactPresentation::{
    artifact_display_title, artifact_kind_message_id,
};
use crate::ToolCallBlocks::i18n;

use super::WorkflowArtifactPill::ArtifactPillData;

/// 完成卡的产物视图（真源 `WorkflowCompletionArtifact`，:33-45）：
/// 药丸数据 + 预览用的补充字段。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkflowCompletionArtifact {
    pub pill: ArtifactPillData,
    pub content_type: Option<String>,
    pub bytes: Option<i64>,
    pub source_path: Option<String>,
    /// 预置看板喂进来的条数；通知载荷上没有，只有活投影 / journal 补过之后才有。
    pub item_count: Option<i64>,
    /// 作者写的一两句说明；只有交付物行念它（瓦片放不下）。
    pub description: Option<String>,
    /// run 的交付物。
    pub primary: bool,
}

impl WorkflowCompletionArtifact {
    pub fn id(&self) -> &str {
        &self.pill.id
    }
    pub fn version(&self) -> i64 {
        self.pill.version.unwrap_or(1)
    }
}

/// 内容拿不到时的纸页字形（真源 `ArtifactSheetGlyph`，:48-67）：
/// 一张小纸 + 右下角的扩展名徽字。诚实，不装饰。
#[component]
pub fn ArtifactSheetGlyph(badge: Option<String>) -> impl IntoView {
    view! {
        <div class="absolute inset-0 grid place-items-center" data-testid="workflow-artifact-sheet">
            <div class="flex aspect-[3/4] w-[38%] max-w-[72px] flex-col gap-1.5 rounded-[4px] border border-border bg-card p-2.5 shadow-[0_1px_0_var(--color-workflow-rule)]">
                <i class="block h-[5px] w-[55%] rounded-sm bg-surface-hover" />
                <i class="block h-[3px] rounded-sm bg-surface-hover" />
                <i class="block h-[3px] rounded-sm bg-surface-hover" />
                <i class="block h-[3px] w-[70%] rounded-sm bg-surface-hover" />
            </div>
            {badge.map(|badge| {
                view! {
                    <span
                        class="absolute bottom-2 right-2 rounded-[4px] bg-surface-hover px-1.5 py-0.5 font-mono text-ui-xs text-foreground-subtle"
                        data-testid="workflow-artifact-sheet-badge"
                    >
                        {badge}
                    </span>
                }
            })}
        </div>
    }
}

/// 产物瓦片（真源 `WorkflowArtifactTile`，:69-176）。
#[component]
pub fn WorkflowArtifactTile(
    artifact: WorkflowCompletionArtifact,
    /// 预览区的内容；缺席即纸页字形。
    preview: Option<AnyView>,
    /// 名字之后、尾槽之前的等宽附属信息（`CSV · 6 KB` / `4 items`）。
    detail: Option<AnyView>,
    on_open: Option<Callback<String>>,
    enter_delay_ms: Option<i64>,
    #[prop(default = "workflow-artifact-tile".to_string())] test_id: String,
    /// tooltip 覆盖（侧板把工作区出处放进来）；缺席时是「种类词 · 标题」。
    title: Option<String>,
) -> impl IntoView {
    let tooltip_title = artifact_display_title(artifact.pill.id.as_str(), artifact.pill.title.as_deref());
    let kind_label = i18n::text(artifact_kind_message_id(artifact.pill.kind()));
    let openable = on_open.is_some();
    let version = artifact.version();
    let show_version = version >= 2;
    let style = enter_delay_ms
        .filter(|delay| *delay > 0)
        .map(|delay| format!("animation-delay: {delay}ms; animation-fill-mode: backwards"));
    let tooltip = title.unwrap_or_else(|| format!("{kind_label} · {tooltip_title}"));
    let artifact_id = artifact.pill.id.clone();
    let on_open_click = on_open.clone();

    view! {
        <div class="wf-tile relative flex min-w-0 flex-col gap-1.5" data-variant="tile">
            // 预览框：底部渐隐由预览内容自己决定（真源 :102-111）。inert 是按钮的兄弟节点。
            <div
                aria-hidden="true"
                class="wf-tile-frame wf-arrive relative aspect-[16/10] w-full overflow-hidden rounded-lg border border-border bg-panel"
                data-testid="workflow-artifact-tile-frame"
                inert="true"
                style=style.clone().unwrap_or_default()
            >
                {preview.unwrap_or_else(|| view! { <ArtifactSheetGlyph badge=None /> }.into_any())}
            </div>
            <button
                aria-label=if openable {
                    Some(format!(
                        "{}: {}",
                        i18n::text("chat.toolCall.workflow.run.artifacts.open"),
                        tooltip_title
                    ))
                } else {
                    None
                }
                class=format!(
                    "wf-tile-hit wf-arrive flex min-w-0 items-center gap-1.5 rounded-lg bg-transparent p-0 px-0.5 text-left text-ui-sm outline-none {}",
                    if openable { "wf-tile-open cursor-pointer" } else { "cursor-default" },
                )
                data-artifact-id=artifact.pill.id.clone()
                data-artifact-kind=artifact.pill.kind().as_str()
                data-artifact-open=openable.then_some("true")
                data-artifact-version=version.to_string()
                data-testid=test_id
                data-variant="tile"
                disabled=!openable
                on:click=move |_| {
                    if let Some(cb) = on_open_click.clone() {
                        cb.run(artifact_id.clone());
                    }
                }
                style=style.unwrap_or_default()
                title=tooltip
                type="button"
            >
                <span class="shrink-0 inline-flex text-foreground-subtle">
                    {crate::app_shell::workflow_artifacts::artifactPresentation::artifact_kind_icon_view(artifact.pill.kind())}
                </span>
                <span class="wf-pill-name min-w-0 flex-1 truncate text-foreground">{tooltip_title.clone()}</span>
                {detail.map(|detail| {
                    view! {
                        <span
                            class="flex shrink-0 items-center gap-1 font-mono text-ui-xs tabular-nums text-foreground-subtlest"
                            data-testid="workflow-artifact-tile-detail"
                        >
                            {detail}
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
                                        {crate::components::workflowIcons::icon_arrow_up_right()}
                                    </span>
                                }
                            })}
                        </span>
                    }
                })}
            </button>
        </div>
    }
}
