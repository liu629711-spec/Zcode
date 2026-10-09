//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowArtifactPill.tsx`（181 行）。
//!
//! 产物药丸：与子代理药丸同一套语法——同样的高度、圆角、底色、悬停抬起、尾槽里 ↗
//! 顶替原有内容——只换两样：**方**的墨灰瓦片代替**圆**的带色头像（颜色说「谁」，
//! 形状说「什么」），尾槽里放版本号而不是状态标记。
//!
//! ⚠ 术语：artifact = 脚本经 `artifact.*` 发布给用户看的产出。
//!
//! 永远是 `<button>`：产物是一个「可以打开的东西」，宿主没给回调时它是**禁用**的按钮
//! ——「交付了什么」是事实，「能不能打开」是能力。版本号从 v2 起才出现
//! （v1 是常态，写出来是噪音）。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::artifactPresentation::{
    artifact_display_title, artifact_kind_message_id, truncate_artifact_chip_title,
};
use crate::ToolCallBlocks::i18n;
use crate::components::workflow_graph::run_state::WorkflowRunArtifactKind;
use crate::components::workflowIcons;

/// 产物药丸的数据（真源 `ArtifactPillData`，:28-33）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArtifactPillData {
    pub id: String,
    pub kind: Option<WorkflowRunArtifactKind>,
    pub title: Option<String>,
    pub version: Option<i64>,
}

impl ArtifactPillData {
    /// 真源 kind 无缺席；Rust 侧由宿主保证非空，这里兜 file。
    pub fn kind(&self) -> WorkflowRunArtifactKind {
        self.kind.unwrap_or(WorkflowRunArtifactKind::File)
    }
}

/// 药丸尺寸（真源 `ArtifactPillSize`，:35）：md 32px / sm 24px。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArtifactPillSize {
    #[default]
    Md,
    Sm,
}

/// 变体（真源 :39 `variant?: "pill" | "link"`）：`link` 复用 Read 链接语法，
/// 避免工具行中出现厚重的产物药丸。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArtifactPillVariant {
    #[default]
    Pill,
    Link,
}

fn kind_icon(kind: WorkflowRunArtifactKind) -> AnyView {
    match kind {
        WorkflowRunArtifactKind::File => workflowIcons::icon_file().into_any(),
        WorkflowRunArtifactKind::Markdown => workflowIcons::icon_file().into_any(),
        WorkflowRunArtifactKind::Chart => workflowIcons::icon_chart_line().into_any(),
        WorkflowRunArtifactKind::Table => workflowIcons::icon_table().into_any(),
        WorkflowRunArtifactKind::Metrics => workflowIcons::icon_gauge().into_any(),
        WorkflowRunArtifactKind::Board => workflowIcons::icon_square_kanban().into_any(),
    }
}

/// 产物药丸（真源 `WorkflowArtifactPill`，:37-181）。
#[component]
pub fn WorkflowArtifactPill(
    artifact: ArtifactPillData,
    #[prop(optional, into)] class: String,
    /// 名字之后、尾槽之前的等宽附属信息（`PDF · 1.2 MB` / `12 items`），已烹熟的纯文本。
    detail: Option<String>,
    /// 在场即可打开；缺席即禁用（回调的存在即门控）。
    on_open: Option<Callback<String>>,
    /// 名字占满剩余宽度（侧栏行）；缺席时药丸按内容收拢（时间线下的产物条）。
    #[prop(default = false)] fill: bool,
    /// 条里的标题截到 24 字（完整标题在 tooltip 里）；侧栏行靠 CSS truncate，不截字。
    #[prop(default = false)] truncate_title: bool,
    /// tooltip 覆盖；缺席时是「种类词 · 标题」。
    title: Option<String>,
    /// 入场延迟（产物条里依次落地，每枚错 30 ms）。
    enter_delay_ms: Option<i64>,
    /// 尺寸（真源 :42 `size`）：md 32px 用在药丸本来就在的地方；sm 24px 用在单行里。
    #[prop(default = ArtifactPillSize::Md)] size: ArtifactPillSize,
    #[prop(default = "workflow-artifact-pill".to_string())] test_id: String,
    #[prop(default = ArtifactPillVariant::Pill)] variant: ArtifactPillVariant,
) -> impl IntoView {
    let full_title = artifact_display_title(&artifact.id, artifact.title.as_deref());
    let label = if truncate_title {
        truncate_artifact_chip_title(&full_title)
    } else {
        full_title.clone()
    };
    let kind_label = i18n::text(artifact_kind_message_id(artifact.kind()));
    let openable = on_open.is_some();
    let version = artifact.version.unwrap_or(1);
    let show_version = version >= 2;
    let version_label = i18n::format(
        "chat.toolCall.workflow.run.artifacts.version",
        &[("version".to_string(), version.to_string())],
    );
    let md = size == ArtifactPillSize::Md;
    // 有延迟的入场要 backwards 填充（与子代理药丸同一条理由，真源 :80-84）。
    let style = enter_delay_ms
        .filter(|delay| *delay > 0)
        .map(|delay| format!("animation-delay: {delay}ms; animation-fill-mode: backwards"));
    let default_title = format!("{kind_label} · {full_title}");
    let tooltip = title.unwrap_or(default_title);
    let artifact_id = artifact.id.clone();

    // 通知摘要的 Read 链接形态（真源 :87-106）。
    if variant == ArtifactPillVariant::Link {
        let on_open_link = on_open.clone();
        let link_id = artifact.id.clone();
        return view! {
            <button
                type="button"
                disabled=!openable
                on:click=move |_| {
                    if let Some(cb) = on_open_link.clone() {
                        cb.run(link_id.clone());
                    }
                }
                data-testid=test_id
                data-artifact-id=artifact.id.clone()
                data-artifact-kind=artifact.kind().as_str()
                data-artifact-version=version.to_string()
                title=tooltip
                class="inline-flex min-w-0 max-w-full items-center gap-1.5 text-ui-base font-normal text-foreground-subtle enabled:cursor-pointer enabled:hover:underline"
            >
                <span class="shrink-0 inline-flex">{kind_icon(artifact.kind())}</span>
                <span class="min-w-0 truncate">{label}</span>
                {show_version.then(|| {
                    view! { <span data-testid="workflow-run-artifact-version">{version_label.clone()}</span> }
                })}
            </button>
        }
        .into_any();
    }

    // 药丸形态（真源 :107-180）。
    let on_open_pill = on_open.clone();
    let version_tail = show_version.then(|| {
        // 按版本重挂：同 id 再发布时尾槽弹入一次（wf-mark 的进场），悬停时让位给 ↗。
        view! {
            <span
                class="wf-mark font-mono text-ui-xs leading-none tabular-nums text-foreground-subtlest"
                data-testid="workflow-run-artifact-version"
                title=version_label.clone()
            >
                {i18n::format(
                    "chat.toolCall.workflow.run.artifacts.versionTail",
                    &[("version".to_string(), version.to_string())],
                )}
            </span>
        }
    });
    let open_tail = openable.then(|| {
        view! {
            <span
                aria-hidden="true"
                class="wf-pill-go flex items-center justify-center text-foreground-subtlest"
                data-testid="workflow-artifact-pill-open"
            >
                {if md {
                    workflowIcons::icon_arrow_up_right()
                } else {
                    workflowIcons::icon_arrow_up_right()
                }}
            </span>
        }
    });
    view! {
        <button
            aria-label=if openable {
                Some(format!(
                    "{}: {}",
                    i18n::text("chat.toolCall.workflow.run.artifacts.open"),
                    full_title.clone()
                ))
            } else {
                None
            }
            class=format!(
                "wf-pill wf-arrive flex min-w-0 max-w-full items-center bg-surface text-left {} {} {} {}",
                if md {
                    "h-8 gap-2 rounded-[9px] pl-2 pr-2.5 text-ui-sm"
                } else {
                    "h-6 gap-1.5 rounded-md pl-1.5 pr-2 text-ui-xs"
                },
                if openable {
                    "wf-pill-open cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
                } else {
                    "cursor-default"
                },
                if fill { "w-full" } else { "" },
                class,
            )
            data-artifact-id=artifact.id.clone()
            data-artifact-kind=artifact.kind().as_str()
            data-artifact-open=openable.then_some("true")
            data-artifact-version=version.to_string()
            data-pill-size=if md { "md" } else { "sm" }
            data-testid=test_id
            disabled=!openable
            on:click=move |_| {
                if let Some(cb) = on_open_pill.clone() {
                    cb.run(artifact_id.clone());
                }
            }
            style=style.unwrap_or_default()
            title=tooltip
            type="button"
        >
            <span class="shrink-0 inline-flex text-foreground-subtle">{kind_icon(artifact.kind())}</span>
            <span class=format!(
                "wf-pill-name min-w-0 truncate text-foreground {}",
                if fill { "flex-1" } else { "" },
            )>
                {label}
            </span>
            {detail.map(|detail_text| {
                view! {
                    <span class="flex shrink-0 items-center gap-1 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
                        {detail_text}
                    </span>
                }
            })}
            {(show_version || openable).then(|| {
                view! {
                    <span
                        class=format!(
                            "wf-pill-tail grid shrink-0 place-items-center [&>*]:col-start-1 [&>*]:row-start-1 {}",
                            if md { "size-3.5" } else { "size-3" },
                        )
                        data-testid="workflow-pill-tail"
                    >
                        {version_tail}
                        {open_tail}
                    </span>
                }
            })}
        </button>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_appears_from_v2_only() {
        // 真源 :73-74 —— v1 是常态；v2 起尾槽才有版本号。
        let artifact = ArtifactPillData {
            id: "a1".into(),
            kind: Some(WorkflowRunArtifactKind::File),
            title: Some("报告".into()),
            version: Some(1),
        };
        assert_eq!(artifact.version.unwrap_or(1), 1);
        assert!(!matches!(artifact.version, Some(v) if v >= 2));
        let updated = ArtifactPillData { version: Some(2), ..artifact };
        assert!(matches!(updated.version, Some(v) if v >= 2));
    }

    #[test]
    fn display_title_falls_back_to_id() {
        // 真源 :69 —— 没写 title 时用 id。
        assert_eq!(
            artifact_display_title("a1", Some("报告")),
            "报告"
        );
        assert_eq!(artifact_display_title("a1", None), "a1");
    }

    #[test]
    fn truncate_title_caps_at_24_chars() {
        // 真源 :70 —— 条里的标题截 24 字。
        let long = "标".repeat(30);
        assert_eq!(truncate_artifact_chip_title(&long).chars().count(), 25);
        assert_eq!(truncate_artifact_chip_title("短"), "短");
    }

    #[test]
    fn defaults_are_md_pill_variant() {
        assert_eq!(ArtifactPillSize::default(), ArtifactPillSize::Md);
        assert_eq!(ArtifactPillVariant::default(), ArtifactPillVariant::Pill);
    }
}
