//! 1:1 翻译 `packages/ui/src/lib/fileDisplay.tsx` 的**路径展示链**
//! （`normalizePath`/`trimTrailingSeparator` 来自 fileDisplayHelpers.ts:129-135，
//! `stripBasePath` :82-99，`getFileDisplayPath` :276-289）。
//!
//! `getFileDisplayPath`：把文件路径转成 UI 展示形态——去掉 basePath 前缀
//! （工作区相对化）；仅当「无 basePath 且是绝对路径」时保留全路径。
//! fileDisplay 全量（图标/素材）待后续批次，本文件先承接 edit 卡用到的部分。

use super::renderers::get_path_leaf;
use super::renderers::planToolCall::is_absolute_file_path;

/// `normalizePath`（fileDisplayHelpers.ts:129-131）：反斜杠统一成正斜杠。
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// `trimTrailingSeparator`（fileDisplayHelpers.ts:133-135）。
pub fn trim_trailing_separator(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}

/// `stripBasePath`（fileDisplay.tsx:82-99）：剥掉 basePath 前缀；
/// 全等 → 空串；不带前缀 → **原始 path**（真源未 normalize 的回退）。
pub fn strip_base_path(path: &str, base_path: Option<&str>) -> String {
    let Some(base) = base_path else {
        return path.to_string();
    };
    let normalized = trim_trailing_separator(&normalize_path(path));
    let normalized_base = trim_trailing_separator(&normalize_path(base));
    if normalized == normalized_base {
        return String::new();
    }
    if normalized.starts_with(&format!("{normalized_base}/")) {
        return normalized[normalized_base.len() + 1..].to_string();
    }
    path.to_string()
}

/// `getFileDisplayPath`（fileDisplay.tsx:276-289）。
pub fn get_file_display_path(file_path: &str, base_path: Option<&str>) -> String {
    let normalized = normalize_path(file_path);
    let path_without_base = strip_base_path(&normalized, base_path);

    if path_without_base.is_empty() {
        return get_path_leaf(&normalized).to_string();
    }

    if base_path.is_some() || !is_absolute_file_path(&path_without_base) {
        return path_without_base;
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_strips_workspace_base() {
        assert_eq!(
            get_file_display_path("/ws/src/main.rs", Some("/ws")),
            "src/main.rs"
        );
        // base 尾斜杠容错。
        assert_eq!(
            get_file_display_path("/ws/src/main.rs", Some("/ws/")),
            "src/main.rs"
        );
        // 等于 base → 叶子（空串回退）。
        assert_eq!(get_file_display_path("/ws", Some("/ws")), "ws");
        // 无 base：相对路径原样。
        assert_eq!(get_file_display_path("src/main.rs", None), "src/main.rs");
        // 无 base：绝对路径保留全路径。
        assert_eq!(
            get_file_display_path("C:\\ws\\src\\a.rs", None),
            "C:/ws/src/a.rs",
            "反斜杠归一后绝对路径保留"
        );
    }
}
