//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowArtifactStrip.tsx`（96 行）。
//!
//! 产物条：一行产物药丸，至多三枚，其余折成等宽的 `+N`。时间线下（工具卡、轮尾摘要）、
//! 完成通知的折叠头部、中枢的运行历史行与「最近产物」条都是它——同一个产物在四处
//! 必须长得一样。全量清单在 run 侧板。
//!
//! ⚠ 术语：artifact = 脚本经 `artifact.*` 发布给用户看的产出。
//!
//! 事件只替**药丸**止步：条的空白与 `+N` 则**是**那块区域的一部分（点它们冒泡给宿主）；
//! 药丸是一颗按钮而不是那块区域的一部分——**包括禁用的**。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::artifactPresentation::ARTIFACT_CHIP_MAX_VISIBLE;
use crate::ToolCallBlocks::i18n;

use super::WorkflowArtifactPill::{
    ArtifactPillData, ArtifactPillSize, ArtifactPillVariant, WorkflowArtifactPill,
};
use super::WorkflowTimeline::PILL_STAGGER_MS;

/// 条上的药丸命中（真源 `hitsPill`，:24-26）：点的是按钮（含禁用的）就拦下冒泡。
/// Rust 侧点击冒泡由药丸自己的 `on:click` 上 `stop_propagation()` 承担；条身不拦
/// （空白与 `+N` 属于外层区域）。
pub fn hits_pill() -> bool {
    // 保留谓词名以对齐真源；判定逻辑由 DOM 层的 stop_propagation 分派。
    false
}

/// 产物条（真源 `WorkflowArtifactStrip`，:28-95）。
#[component]
pub fn WorkflowArtifactStrip(
    artifacts: Vec<ArtifactPillData>,
    #[prop(optional, into)] class: String,
    more_test_id: Option<String>,
    on_open_artifact: Option<Callback<String>>,
    pill_test_id: Option<String>,
    #[prop(default = ArtifactPillSize::Md)] size: ArtifactPillSize,
    test_id: Option<String>,
    /// 发射侧砍过（超上界或被过滤）——`+N` 因此可能少报，用 `…` 而不是数字。
    #[prop(default = false)] truncated: bool,
    #[prop(default = ArtifactPillVariant::Pill)] variant: ArtifactPillVariant,
) -> impl IntoView {
    if artifacts.is_empty() {
        return view! { <span class="hidden" /> }.into_any();
    }
    let visible: Vec<ArtifactPillData> = artifacts
        .iter()
        .take(ARTIFACT_CHIP_MAX_VISIBLE)
        .cloned()
        .collect();
    let overflow = artifacts.len().saturating_sub(visible.len());
    view! {
        <span
            class=format!(
                "wf-motion flex min-w-0 shrink flex-wrap items-center {} {}",
                if size == ArtifactPillSize::Md { "gap-1.5" } else { "gap-1" },
                class,
            )
            data-testid=test_id
        >
            {visible
                .iter()
                .enumerate()
                .map(|(i, artifact)| {
                    view! {
                        <WorkflowArtifactPill
                            artifact=artifact.clone()
                            detail=None
                            enter_delay_ms=Some(PILL_STAGGER_MS * i as i64)
                            on_open=on_open_artifact.clone()
                            size=size
                            title=None
                            truncate_title=true
                            variant=variant
                            test_id=pill_test_id.clone().unwrap_or_else(|| "workflow-artifact-pill".to_string())
                        />
                    }
                    .into_any()
                })
                .collect_view()}
            {(overflow > 0 || truncated).then(|| {
                let count_text = if truncated && overflow == 0 {
                    "…".to_string()
                } else {
                    overflow.to_string()
                };
                view! {
                    <span
                        class="shrink-0 px-0.5 font-mono text-ui-xs tabular-nums text-foreground-subtlest"
                        data-testid=more_test_id
                    >
                        {i18n::format(
                            "chat.backgroundResult.workflow.artifacts.more",
                            &[("count".to_string(), count_text)],
                        )}
                    </span>
                }
            })}
        </span>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_shows_at_most_three_pills() {
        // 真源 :53 —— 至多三枚，其余折 +N。
        let artifacts: Vec<ArtifactPillData> = (0..5)
            .map(|i| ArtifactPillData {
                id: format!("a{i}"),
                ..Default::default()
            })
            .collect();
        let visible = artifacts.iter().take(ARTIFACT_CHIP_MAX_VISIBLE).count();
        assert_eq!(visible, 3);
        assert_eq!(artifacts.len() - visible, 2);
    }
}
