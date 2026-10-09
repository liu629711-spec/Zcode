//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowCompletionArtifacts.tsx`
//! （117 行）+ 完成卡的产物区。
//!
//! - **有交付物**（打了 primary 旗子的那件，或清单只有一件）：交付物行，然后其余产物作
//!   **索引**——一件一行，卡宽时两列；至多六行，从第七件起五行 + 一行「还有 N 个」。
//! - **没有交付物**：没有哪一件配得上一张预览，就一张也不画——索引独占产物区。
//!
//! 只有交付物的框读字节（预览要么读得清，要么不画）；索引行与「还有 N 个」从不读。

use leptos::prelude::*;

use crate::app_shell::workflow_artifacts::artifactPresentation::resolve_primary_index;
use crate::app_shell::workflow_artifacts::presets::PresetLabels;

use super::WorkflowArtifactTile::{
    ArtifactSheetGlyph, WorkflowCompletionArtifact,
};
use super::WorkflowTimeline::PILL_STAGGER_MS;

/// 卡上索引的行数上限，也是不折叠时的上限（真源 `COMPLETION_INDEX_MAX`，:23）。
pub const COMPLETION_INDEX_MAX: usize = 6;
/// 需要「还有 N 个」时，与它同在的行数（真源 :25）。
const COMPLETION_INDEX_WITH_MORE: usize = COMPLETION_INDEX_MAX - 1;

/// 布局（真源 `CompletionArtifactLayout`，:27-36）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompletionArtifactLayout {
    /// 交付物；缺席即索引独占。
    pub primary: Option<WorkflowCompletionArtifact>,
    /// 画成行的那些件（交付物之外）。
    pub lines: Vec<WorkflowCompletionArtifact>,
    /// 没画出来的件数（「还有 N 个」的 N）。
    pub folded: i64,
    /// 画「还有 N 个」：有折叠的件，或清单被发射侧砍过（此时 N 不可知，写 `…`）。
    pub more: bool,
}

/// 布局的**唯一**判定（真源 `completionArtifactLayout`，:39-55）；
/// 卡（节奏）、取数门（哪些件读字节）与渲染都从这里读。
pub fn completion_artifact_layout(
    artifacts: &[WorkflowCompletionArtifact],
    truncated: bool,
) -> CompletionArtifactLayout {
    if artifacts.is_empty() {
        return CompletionArtifactLayout::default();
    }
    let primaries: Vec<Option<bool>> = artifacts
        .iter()
        .map(|artifact| Some(artifact.primary))
        .collect();
    let primary_index = resolve_primary_index(&primaries);
    let rest: Vec<WorkflowCompletionArtifact> = match primary_index {
        Some(index) => artifacts
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, artifact)| artifact.clone())
            .collect(),
        None => artifacts.to_vec(),
    };
    // 砍过的清单（超 8）不知道真实件数：仍给一扇门，N 写 `…`——与产物条的省略号同一个诚实。
    let more = rest.len() > COMPLETION_INDEX_MAX || truncated;
    let lines: Vec<WorkflowCompletionArtifact> = rest
        .iter()
        .take(if more { COMPLETION_INDEX_WITH_MORE } else { COMPLETION_INDEX_MAX })
        .cloned()
        .collect();
    let folded = (rest.len() - lines.len()) as i64;
    CompletionArtifactLayout {
        primary: primary_index.map(|index| artifacts[index].clone()),
        lines,
        folded,
        more,
    }
}

/// 卡上画出来的格数（真源 `completionArtifactCellCount`，:58-60）：行算一格，
/// 每条索引行一格，「还有 N 个」一格——四格数字的落地节拍接在它们之后。
pub fn completion_artifact_cell_count(layout: &CompletionArtifactLayout) -> usize {
    (layout.primary.is_some() as usize) + layout.lines.len() + (layout.more as usize)
}

/// 会读字节的那些件（真源 `completionPreviewIds`，:63-65）：只有交付物的框读。
pub fn completion_preview_ids(layout: &CompletionArtifactLayout) -> Vec<String> {
    layout
        .primary
        .as_ref()
        .map(|primary| vec![primary.id().to_string()])
        .unwrap_or_default()
}

/// 完成卡的产物区（真源 `WorkflowCompletionArtifacts`，:67-116）。
#[component]
pub fn WorkflowCompletionArtifacts(
    layout: CompletionArtifactLayout,
    artifacts_truncated: bool,
    labels: PresetLabels,
    /// 预览 view 工厂（真源 renderPreview：只有交付物行有框）；缺席即纸页字形。
    render_preview: Option<Callback<WorkflowCompletionArtifact, AnyView>>,
    on_open_artifact: Option<Callback<String>>,
    on_open_run: Option<Callback<()>>,
) -> impl IntoView {
    let has_primary = layout.primary.is_some();
    if !has_primary && layout.lines.is_empty() && !layout.more {
        return view! { <span class="hidden" /> }.into_any();
    }
    let primary_view = layout.primary.as_ref().map(|primary| {
        let preview = render_preview
            .as_ref()
            .map(|cb| cb.run(primary.clone()))
            .unwrap_or_else(|| view! { <ArtifactSheetGlyph badge=None /> }.into_any());
        view! {
            <super::WorkflowArtifactRow::WorkflowArtifactRow
                artifact=primary.clone()
                enter_delay_ms=Some(PILL_STAGGER_MS)
                labels=labels.clone()
                on_open=on_open_artifact.clone()
                preview=Some(preview)
                test_id="workflow-completion-row".to_string()
                title=None
            />
        }
        .into_any()
    });
    let show_index = !layout.lines.is_empty() || layout.more;
    view! {
        <>
            {primary_view}
            {show_index.then(|| {
                // 索引跟在交付物行之后时隔一条细线；独占产物区时不要（真源 :97-114）。
                view! {
                    <super::WorkflowArtifactIndex::WorkflowArtifactIndex
                        artifacts=layout.lines.clone()
                        columns=super::WorkflowArtifactIndex::ArtifactIndexColumns::Auto
                        first_delay_ms=PILL_STAGGER_MS * (has_primary as i64 + 1)
                        folded=layout.folded
                        labels=labels.clone()
                        line_test_id=Some("workflow-completion-line".to_string())
                        more=layout.more
                        more_test_id=Some("workflow-completion-more".to_string())
                        on_open_artifact=on_open_artifact.clone()
                        on_open_run=on_open_run.clone()
                        rule=has_primary
                        test_id="workflow-completion-index".to_string()
                        tooltip_of=None
                        truncated=artifacts_truncated
                    />
                }
                .into_any()
            })}
        </>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::run_state::WorkflowRunArtifactKind;

    fn artifact(id: &str, primary: bool) -> WorkflowCompletionArtifact {
        WorkflowCompletionArtifact {
            pill: super::super::WorkflowArtifactPill::ArtifactPillData {
                id: id.to_string(),
                kind: Some(WorkflowRunArtifactKind::File),
                title: None,
                version: None,
            },
            primary,
            ..Default::default()
        }
    }

    #[test]
    fn empty_list_has_no_layout() {
        // 真源 :43 —— 空清单：无主无行无门。
        let layout = completion_artifact_layout(&[], false);
        assert!(layout.primary.is_none());
        assert!(layout.lines.is_empty());
        assert!(!layout.more);
        assert_eq!(completion_artifact_cell_count(&layout), 0);
        assert!(completion_preview_ids(&layout).is_empty());
    }

    #[test]
    fn single_item_becomes_primary() {
        // 真源 :44-47 + resolvePrimaryArtifact 的单件规则。
        let artifacts = vec![artifact("only", false)];
        let layout = completion_artifact_layout(&artifacts, false);
        assert_eq!(layout.primary.as_ref().unwrap().id(), "only");
        assert!(layout.lines.is_empty());
        // 单件规则只在 UI 上成立：交付物之外的行是零行。
        assert_eq!(completion_artifact_cell_count(&layout), 1);
    }

    #[test]
    fn index_caps_at_six_and_folds_the_rest() {
        // 真源 :46-53 —— 8 件：交付物之外 7 件 → 门在场时行数收成 5（WITH_MORE），
        // folded = 7 - 5 = 2；5 行 + 门 + 交付物 = 7 格。
        let mut artifacts = vec![artifact("primary", true)];
        for i in 0..7 {
            artifacts.push(artifact(&format!("a{i}"), false));
        }
        let layout = completion_artifact_layout(&artifacts, false);
        assert!(layout.primary.is_some());
        assert_eq!(layout.lines.len(), 5);
        assert!(layout.more);
        assert_eq!(layout.folded, 2);
        assert_eq!(completion_artifact_cell_count(&layout), 7);
        // 不折叠的上限是 6 行（COMPLETION_INDEX_MAX）：6 件交付物之外 → 6 行、无门。
        let mut artifacts = vec![artifact("primary", true)];
        for i in 0..6 {
            artifacts.push(artifact(&format!("b{i}"), false));
        }
        let layout = completion_artifact_layout(&artifacts, false);
        assert_eq!(layout.lines.len(), 6);
        assert!(!layout.more);
        assert_eq!(layout.folded, 0);
    }

    #[test]
    fn truncated_list_still_gets_the_gate() {
        // 真源 :46 —— 发射侧砍过：N 不可知仍给门。
        let artifacts = vec![artifact("primary", true), artifact("a", false)];
        let layout = completion_artifact_layout(&artifacts, true);
        assert!(layout.more);
        assert_eq!(layout.folded, 0);
    }

    #[test]
    fn only_primary_reads_bytes() {
        // 真源 :63-65 —— 会读字节的只有交付物。
        let artifacts = vec![artifact("primary", true), artifact("a", false)];
        let layout = completion_artifact_layout(&artifacts, false);
        assert_eq!(completion_preview_ids(&layout), vec!["primary".to_string()]);
    }
}
