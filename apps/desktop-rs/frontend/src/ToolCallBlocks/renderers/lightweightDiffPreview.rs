//! 1:1 翻译 `packages/ui/src/components/ui/lightweight-diff-preview.tsx`
//! （187 行）。
//!
//! 轻量 diff 预览：行号 gutter + 行背景/状态条表达增删（不渲染 `+/-`
//! 协议 marker）。被 EditInlineDiffContent（内联 diff）等消费。
//!
//! 行样式用内联 style（color-mix 的 CSS 变量）——Rust 侧原样下发。

use leptos::prelude::*;

use super::super::codePreviewSettings::CodePreviewSettings;
use super::super::patchDiffPreview::{
    get_patch_preview_line_content, parse_truncated_marker_omitted_line_count,
};

/// `LightweightDiffLineKind`（真源 :9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightweightDiffLineKind {
    Added,
    Removed,
    Context,
}

/// `LightweightDiffLineParts`（真源 :12-17）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightweightDiffLineParts {
    pub content: String,
    pub kind: LightweightDiffLineKind,
    pub marker: String,
    pub raw: String,
}

/// `getLightweightDiffLineParts`（真源 :37-70）。
pub fn get_lightweight_diff_line_parts(line: &str) -> LightweightDiffLineParts {
    let content = get_patch_preview_line_content(line);
    if line.starts_with('+') {
        return LightweightDiffLineParts {
            content,
            kind: LightweightDiffLineKind::Added,
            marker: "+".to_string(),
            raw: line.to_string(),
        };
    }
    if line.starts_with('-') {
        return LightweightDiffLineParts {
            content,
            kind: LightweightDiffLineKind::Removed,
            marker: "-".to_string(),
            raw: line.to_string(),
        };
    }
    if line.starts_with(' ') {
        return LightweightDiffLineParts {
            content,
            kind: LightweightDiffLineKind::Context,
            marker: " ".to_string(),
            raw: line.to_string(),
        };
    }
    LightweightDiffLineParts {
        content,
        kind: LightweightDiffLineKind::Context,
        marker: String::new(),
        raw: line.to_string(),
    }
}

/// `getLightweightDiffLineStyles` 的结果（真源 :72-115）。
struct LightweightDiffLineStyles {
    line_number_class: &'static str,
    row_style: Option<&'static str>,
    gutter_style: Option<&'static str>,
}

fn get_lightweight_diff_line_styles(kind: LightweightDiffLineKind) -> LightweightDiffLineStyles {
    match kind {
        LightweightDiffLineKind::Added => LightweightDiffLineStyles {
            line_number_class: "text-diff-added",
            row_style: Some(
                "background-color:color-mix(in srgb, var(--color-diff-added) 14%, transparent);box-shadow:inset 3px 0 0 var(--color-diff-added)",
            ),
            gutter_style: Some(
                "background-color:color-mix(in srgb, var(--color-diff-added) 18%, var(--color-background));box-shadow:inset 3px 0 0 var(--color-diff-added);color:var(--color-diff-added)",
            ),
        },
        LightweightDiffLineKind::Removed => LightweightDiffLineStyles {
            line_number_class: "text-diff-removed",
            row_style: Some(
                "background-color:color-mix(in srgb, var(--color-diff-removed) 14%, transparent);box-shadow:inset 3px 0 0 var(--color-diff-removed)",
            ),
            gutter_style: Some(
                "background-color:color-mix(in srgb, var(--color-diff-removed) 18%, var(--color-background));box-shadow:inset 3px 0 0 var(--color-diff-removed);color:var(--color-diff-removed)",
            ),
        },
        LightweightDiffLineKind::Context => LightweightDiffLineStyles {
            line_number_class: "text-foreground-subtlest",
            row_style: None,
            gutter_style: Some("background-color:var(--color-background)"),
        },
    }
}

/// `LightweightDiffPreview` 的 props。
#[derive(Debug, Clone)]
pub struct LightweightDiffPreviewProps {
    /// 外层额外类名（真源 `className` 透传——cn 合并）。
    pub class: String,
    /// `data-*` 透传（真源 `...props` spread 的等价）。
    pub data_lightweight_diff_highlight_language: Option<String>,
    pub data_lightweight_diff_highlight_theme: Option<String>,
    pub data_lightweight_diff_highlighted: Option<String>,
    pub code_preview_settings: CodePreviewSettings,
    pub lines: Vec<String>,
}

/// `LightweightDiffPreview`（真源 :113-187）。
#[component]
pub fn LightweightDiffPreviewComponent(props: LightweightDiffPreviewProps) -> impl IntoView {
    let wrap_long_lines = props.code_preview_settings.wrap_long_lines;
    let show_line_numbers = props.code_preview_settings.show_line_numbers;
    let font_size_style = format!("font-size:{}px", props.code_preview_settings.font_size_px);
    let content_class = if wrap_long_lines {
        "min-w-full font-mono leading-relaxed text-foreground"
    } else {
        "min-w-full w-max font-mono leading-relaxed text-foreground"
    };
    // cn("w-full min-w-0 overflow-auto bg-background", className)
    let outer_class = format!("w-full min-w-0 overflow-auto bg-background {}", props.class);
    let code_class = if wrap_long_lines {
        "block flex-1 px-3 whitespace-pre-wrap break-words"
    } else {
        "block flex-1 px-3 whitespace-pre"
    };

    let lines = props.lines.clone();
    view! {
        <div
            class=outer_class
            data-lightweight-diff-preview
            data-lightweight-diff-highlight-language=props.data_lightweight_diff_highlight_language.clone()
            data-lightweight-diff-highlight-theme=props.data_lightweight_diff_highlight_theme.clone()
            data-lightweight-diff-highlighted=props.data_lightweight_diff_highlighted.clone()
        >
            <div class=content_class data-lightweight-diff-scroll-content style=font_size_style>
                {lines
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| {
                        // 截断 marker 行：走 i18n 文案（真源 :135-144）。
                        if let Some(omitted_count) =
                            parse_truncated_marker_omitted_line_count(&line)
                        {
                            let text = format!(
                                "Diff 预览已截断：为保持界面流畅，省略了 {omitted_count} 行。"
                            );
                            return view! {
                                <div class="px-3 py-1 text-foreground-subtle">{text}</div>
                            }
                            .into_any();
                        }

                        let parts = get_lightweight_diff_line_parts(&line);
                        let styles = get_lightweight_diff_line_styles(parts.kind);
                        let gutter_class = if show_line_numbers {
                            format!(
                                "sticky left-0 z-[1] w-12 shrink-0 select-none border-r border-border px-2 text-right tabular-nums {}",
                                styles.line_number_class
                            )
                        } else {
                            String::new()
                        };
                        let row_style = styles.row_style.map(str::to_string);
                        let gutter_style = styles.gutter_style.map(str::to_string);
                        // 真源 :178 —— `renderLineContent?.(...) ?? (content || " ")`：
                        // EditInlineDiffContent 的 renderLineContent 只回 content，
                        // 空行回退一个空格。
                        let content = if parts.content.is_empty() {
                            " ".to_string()
                        } else {
                            parts.content.clone()
                        };
                        view! {
                            <div class="flex min-w-full w-full" style=row_style>
                                {show_line_numbers.then(|| view! {
                                    <span
                                        aria-hidden="true"
                                        class=gutter_class.clone()
                                        style=gutter_style.clone()
                                    >
                                        {index + 1}
                                    </span>
                                })}
                                <code class=code_class>{content}</code>
                            </div>
                        }
                            .into_any()
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_parts_classify_markers() {
        let added = get_lightweight_diff_line_parts("+new");
        assert_eq!(added.kind, LightweightDiffLineKind::Added);
        assert_eq!(added.content, "new");
        assert_eq!(added.marker, "+");
        let removed = get_lightweight_diff_line_parts("-old");
        assert_eq!(removed.kind, LightweightDiffLineKind::Removed);
        let context = get_lightweight_diff_line_parts(" ctx");
        assert_eq!(context.kind, LightweightDiffLineKind::Context);
        assert_eq!(context.marker, " ");
        // 无 marker 的行也是 context，marker 为空。
        let plain = get_lightweight_diff_line_parts("plain");
        assert_eq!(plain.kind, LightweightDiffLineKind::Context);
        assert_eq!(plain.marker, "");
        assert_eq!(plain.content, "plain");
    }
}
