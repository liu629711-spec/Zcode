//! 1:1 翻译 `packages/ui/src/lib/codePreviewSettings.ts`（24 行）。
//!
//! 代码预览设置的类型与默认值（中立模块，不依赖 store）。
//! `BundledTheme`（shiki 类型）以 `String` 承接——高亮器不在本层。

/// `CodePreviewSettings`（真源 :9-15）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePreviewSettings {
    pub light_theme: String,
    pub dark_theme: String,
    pub show_line_numbers: bool,
    pub wrap_long_lines: bool,
    pub font_size_px: u32,
}

/// `DEFAULT_CODE_PREVIEW_SETTINGS`（真源 :17-23）。
pub fn default_code_preview_settings() -> CodePreviewSettings {
    CodePreviewSettings {
        light_theme: "github-light".to_string(),
        dark_theme: "github-dark".to_string(),
        show_line_numbers: true,
        wrap_long_lines: false,
        font_size_px: 12,
    }
}

impl Default for CodePreviewSettings {
    fn default() -> Self {
        default_code_preview_settings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_source() {
        let settings = default_code_preview_settings();
        assert_eq!(settings.light_theme, "github-light");
        assert_eq!(settings.dark_theme, "github-dark");
        assert!(settings.show_line_numbers);
        assert!(!settings.wrap_long_lines);
        assert_eq!(settings.font_size_px, 12);
    }
}
