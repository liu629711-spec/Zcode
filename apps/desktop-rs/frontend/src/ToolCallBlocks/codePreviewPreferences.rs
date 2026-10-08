//! 1:1 翻译 `packages/ui/src/lib/codePreviewPreferences.ts`（113 行）。
//!
//! 代码高亮主题的选项表、明暗归一与解析。真源注释（:70-77）：八个渲染面
//! （聊天代码块/文件查看器/富 diff/轻量 diff/内联 diff/手机页…）此前各写
//! 一段三元表达式，任一处漏 system 或写反明暗就是「深底深字」——统一走这里。
//!
//! **化简注明**：`resolveCodePreviewTheme` 的 system 探测（真源
//! `window.matchMedia`）v1 走 `prefers_dark` 参数显式传入，缺省 false
//! （等同亮色）；宿主接线时传入真实偏好。

use super::codePreviewSettings::{CodePreviewSettings, default_code_preview_settings};

/// `CodePreviewThemeOption`（真源 :7-12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodePreviewThemeOption {
    pub value: &'static str,
    pub label: &'static str,
    /// 明暗归属：设置页两个下拉按它分组。
    pub mode: ThemeMode,
}

/// `"light" | "dark"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

/// `CODE_PREVIEW_THEME_OPTIONS`（真源 :14-25，10 项）。
pub const CODE_PREVIEW_THEME_OPTIONS: [CodePreviewThemeOption; 10] = [
    CodePreviewThemeOption {
        value: "github-light",
        label: "GitHub Light",
        mode: ThemeMode::Light,
    },
    CodePreviewThemeOption {
        value: "github-dark",
        label: "GitHub Dark",
        mode: ThemeMode::Dark,
    },
    CodePreviewThemeOption {
        value: "vitesse-light",
        label: "Vitesse Light",
        mode: ThemeMode::Light,
    },
    CodePreviewThemeOption {
        value: "vitesse-dark",
        label: "Vitesse Dark",
        mode: ThemeMode::Dark,
    },
    CodePreviewThemeOption {
        value: "min-light",
        label: "Minimal Light",
        mode: ThemeMode::Light,
    },
    CodePreviewThemeOption {
        value: "min-dark",
        label: "Minimal Dark",
        mode: ThemeMode::Dark,
    },
    CodePreviewThemeOption {
        value: "github-light-high-contrast",
        label: "GitHub HC Light",
        mode: ThemeMode::Light,
    },
    CodePreviewThemeOption {
        value: "github-dark-high-contrast",
        label: "GitHub HC Dark",
        mode: ThemeMode::Dark,
    },
    CodePreviewThemeOption {
        value: "catppuccin-latte",
        label: "Catppuccin Latte",
        mode: ThemeMode::Light,
    },
    CodePreviewThemeOption {
        value: "catppuccin-mocha",
        label: "Catppuccin Mocha",
        mode: ThemeMode::Dark,
    },
];

/// `getCodePreviewThemeOptions`（真源 :28-30）。
pub fn get_code_preview_theme_options(mode: ThemeMode) -> Vec<CodePreviewThemeOption> {
    CODE_PREVIEW_THEME_OPTIONS
        .iter()
        .filter(|option| option.mode == mode)
        .copied()
        .collect()
}

/// `DARK_CODE_PREVIEW_THEMES`（真源 :32-38）。
const DARK_CODE_PREVIEW_THEMES: [&str; 5] = [
    "github-dark",
    "vitesse-dark",
    "min-dark",
    "github-dark-high-contrast",
    "catppuccin-mocha",
];

/// `isDarkCodePreviewTheme`（真源 :40-44）：明暗语义必须跟随显式选项表，
/// 不能用主题名正则猜测。
pub fn is_dark_code_preview_theme(theme: &str) -> bool {
    DARK_CODE_PREVIEW_THEMES.contains(&theme)
}

/// `isLightCodePreviewTheme`（真源 :46-48）。
pub fn is_light_code_preview_theme(theme: &str) -> bool {
    !is_dark_code_preview_theme(theme)
}

/// `normalizeCodePreviewThemeSettings`（真源 :53-72）：错配的槽位拉回默认
/// （浅色主题画在深色卡片上就是深底深字）。
pub fn normalize_code_preview_theme_settings(
    settings: &CodePreviewSettings,
) -> CodePreviewSettings {
    let default = default_code_preview_settings();
    let light_theme = if is_light_code_preview_theme(&settings.light_theme) {
        settings.light_theme.clone()
    } else {
        default.light_theme
    };
    let dark_theme = if is_dark_code_preview_theme(&settings.dark_theme) {
        settings.dark_theme.clone()
    } else {
        default.dark_theme
    };
    if light_theme == settings.light_theme && dark_theme == settings.dark_theme {
        return settings.clone();
    }
    CodePreviewSettings {
        light_theme,
        dark_theme,
        ..settings.clone()
    }
}

/// `SETTINGS_PREVIEW_CODE`（真源 :74-78）——设置页的主题预览代码样本。
pub const SETTINGS_PREVIEW_CODE: &str = "const themePreview: ThemeConfig = {\n  surface: \"sidebar\",\n  accent: \"#339CFF\",\n  contrast: 45,\n};";

/// `getCodePreviewTheme`（真源 :80-86）。
pub fn get_code_preview_theme(mode: ThemeMode, settings: &CodePreviewSettings) -> String {
    let normalized = normalize_code_preview_theme_settings(settings);
    match mode {
        ThemeMode::Dark => normalized.dark_theme,
        ThemeMode::Light => normalized.light_theme,
    }
}

/// `resolveCodePreviewTheme`（真源 :94-113）。
///
/// `theme`：应用主题（`"system"` / `"dark"` / `"light"` / `"zai-dark"`…）。
/// `prefers_dark`：system 模式下的 OS 偏好（真源 `window.matchMedia` 探测；
/// v1 由宿主传入，缺省 false）。
pub fn resolve_code_preview_theme(
    theme: Option<&str>,
    settings: &CodePreviewSettings,
    prefers_dark: bool,
) -> String {
    let normalized = normalize_code_preview_theme_settings(settings);
    let mode = match theme {
        None | Some("system") => {
            if prefers_dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            }
        }
        Some(value) => {
            if value == "dark" || value == "zai-dark" {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            }
        }
    };
    match mode {
        ThemeMode::Dark => normalized.dark_theme,
        ThemeMode::Light => normalized.light_theme,
    }
}

/// 别名的占位（与真源 `Pick<CodePreviewSettings, "lightTheme" | "darkTheme">`
/// 的消费方兼容——Rust 直接复用完整 struct）。
pub type CodePreviewThemePair = CodePreviewSettings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_options_split_by_mode() {
        assert_eq!(get_code_preview_theme_options(ThemeMode::Light).len(), 5);
        assert_eq!(get_code_preview_theme_options(ThemeMode::Dark).len(), 5);
    }

    #[test]
    fn dark_light_classification_follows_table() {
        assert!(is_dark_code_preview_theme("github-dark"));
        assert!(!is_dark_code_preview_theme("github-light"));
        assert!(is_light_code_preview_theme("vitesse-light"));
        // 未登记的主题按亮色（真源 :47）。
        assert!(!is_dark_code_preview_theme("unknown-theme"));
    }

    #[test]
    fn normalize_pulls_mismatched_slots_back_to_default() {
        // darkTheme 错配成浅色 → 拉回默认 dark。
        let settings = CodePreviewSettings {
            light_theme: "vitesse-light".into(),
            dark_theme: "github-light".into(),
            ..default_code_preview_settings()
        };
        let normalized = normalize_code_preview_theme_settings(&settings);
        assert_eq!(normalized.light_theme, "vitesse-light", "合规槽位保留");
        assert_eq!(normalized.dark_theme, "github-dark", "错配槽位拉回默认");
    }

    #[test]
    fn resolve_follows_theme_and_prefers() {
        let settings = default_code_preview_settings();
        // system + prefers_dark → dark。
        assert_eq!(
            resolve_code_preview_theme(Some("system"), &settings, true),
            "github-dark"
        );
        // system + !prefers_dark → light。
        assert_eq!(
            resolve_code_preview_theme(Some("system"), &settings, false),
            "github-light"
        );
        // 显式 dark / zai-dark → dark。
        assert_eq!(
            resolve_code_preview_theme(Some("dark"), &settings, false),
            "github-dark"
        );
        assert_eq!(
            resolve_code_preview_theme(Some("zai-dark"), &settings, false),
            "github-dark"
        );
        // 其他 → light。
        assert_eq!(
            resolve_code_preview_theme(Some("light"), &settings, true),
            "github-light"
        );
        assert_eq!(
            resolve_code_preview_theme(None, &settings, false),
            "github-light"
        );
    }

    #[test]
    fn get_theme_reads_normalized_slot() {
        let mut settings = default_code_preview_settings();
        settings.dark_theme = "vitesse-dark".into();
        assert_eq!(
            get_code_preview_theme(ThemeMode::Dark, &settings),
            "vitesse-dark"
        );
        assert_eq!(
            get_code_preview_theme(ThemeMode::Light, &settings),
            "github-light"
        );
    }
}
