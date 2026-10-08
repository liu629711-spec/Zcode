//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/EditInlineDiffContent.tsx`
//! （218 行）的 **v1 简化版**。
//!
//! 内联 diff 预览：挂 LightweightDiffPreview 展示 patch 的轻量行。
//!
//! **裁剪注明**：真源用 `HighlightedLightweightDiffPreview`（Shiki 异步
//! token 补丁，首帧轻量文本 + effect 里补高亮——注释说明直接挂
//! @pierre/diffs 会把高亮压到主线程、点击展开掉帧）。Rust 侧无 Shiki
//! ——v1 恒为轻量文本（`data-lightweight-diff-highlighted="false"`），
//! 与真源**首帧**形态一致；语法高亮待 hljs/自研方案接续。
//! `codePreviewSettings` 的 font/行号/换行参与渲染（与真源同）；
//! `theme` 解析出的 highlightTheme 仅作为 data 属性输出（无消费者）。

use leptos::prelude::*;

use super::lightweightDiffPreview::{LightweightDiffPreviewComponent, LightweightDiffPreviewProps};
use crate::ToolCallBlocks::codePreviewPreferences::resolve_code_preview_theme;
use crate::ToolCallBlocks::codePreviewSettings::CodePreviewSettings;
use crate::ToolCallBlocks::codeViewer::{PatchCodeViewerSource, infer_code_language};
use crate::ToolCallBlocks::patchDiffPreview::get_plain_text_patch_preview_lines;

/// `EditInlineDiffContent` 的 props。
#[derive(Debug, Clone)]
pub struct EditInlineDiffContentProps {
    pub preview: PatchCodeViewerSource,
    /// 应用主题（`"system"` / `"dark"`…）。
    pub theme: Option<String>,
    /// system 模式下的 OS 偏好（宿主传入，缺省 false）。
    pub prefers_dark: bool,
    pub code_preview_settings: CodePreviewSettings,
}

/// `EditInlineDiffContent`（真源 :43-84）。
#[component]
pub fn EditInlineDiffContentComponent(props: EditInlineDiffContentProps) -> impl IntoView {
    let preview_lines = get_plain_text_patch_preview_lines(&props.preview.patch);
    let highlight_language = infer_code_language(
        Some(
            props
                .preview
                .path
                .as_deref()
                .unwrap_or(props.preview.title.as_str()),
        ),
        Some(&props.preview.patch),
    );
    let highlight_theme = resolve_code_preview_theme(
        props.theme.as_deref(),
        &props.code_preview_settings,
        props.prefers_dark,
    );

    view! {
        <div class="space-y-3">
            <div
                class="mb-2 max-h-60 overflow-auto rounded-xl border border-border bg-card"
                data-inline-diff-preview
            >
                <LightweightDiffPreviewComponent
                    props=LightweightDiffPreviewProps {
                        class: "h-full bg-card".to_string(),
                        data_lightweight_diff_highlight_language: Some(
                            highlight_language.to_string(),
                        ),
                        data_lightweight_diff_highlight_theme: Some(highlight_theme),
                        data_lightweight_diff_highlighted: Some("false".to_string()),
                        code_preview_settings: props.code_preview_settings.clone(),
                        lines: preview_lines,
                    }
                />
            </div>
        </div>
    }
}
