//! 1:1 翻译 `packages/ui/src/components/workflow-graph/name-pattern.ts`（30 行）。

use super::types::NamePattern;

/// 省略号（真源 :14）。
///
/// ★真源 :12-13 注释——省略号（而不是 `*` 或 `${…}`）是刻意的：
/// 它读起来仍然是一个名字，不引入新的视觉词汇。
const ELLIPSIS: &str = "…";

/// `formatNamePattern`（真源 :20-29）。
///
/// ★真源 :16-19 注释——「名字只在运行时才成形」这件事的唯一渲染点。
/// 分析器给的是形状而不是名字：投影只搬两个 affix，省略号在**这里**才补上
/// ——与匿名兜底文案同一道理：投影是被 memo 的纯函数，文案与字形都该在渲染时成型，
/// 投影里只存数据。
///
/// 两个 affix 都缺席时返回 `None` 而不是孤零零一个 `…`：分析器不会发这种 pattern，
/// 但契约上 `{}` 是能通过 `.strict()` 的——这里不替它兜着就会在画面上留一个
/// 没有意义的省略号。
pub fn format_name_pattern(pattern: Option<&NamePattern>) -> Option<String> {
    let pattern = pattern?;
    let head = pattern.head.as_deref().unwrap_or("");
    let tail = pattern.tail.as_deref().unwrap_or("");
    if head.is_empty() && tail.is_empty() {
        return None;
    }
    Some(format!("{head}{ELLIPSIS}{tail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(head: Option<&str>, tail: Option<&str>) -> NamePattern {
        NamePattern {
            head: head.map(str::to_string),
            tail: tail.map(str::to_string),
        }
    }

    #[test]
    fn head_only_appends_ellipsis() {
        // 真源 :20 —— {head: "研究员"} → 研究员…
        assert_eq!(
            format_name_pattern(Some(&p(Some("研究员"), None))).as_deref(),
            Some("研究员…")
        );
    }

    #[test]
    fn tail_only_prefixes_ellipsis() {
        // 真源 :21 —— {tail: "-worker"} → …-worker
        assert_eq!(
            format_name_pattern(Some(&p(None, Some("-worker")))).as_deref(),
            Some("…-worker")
        );
    }

    #[test]
    fn both_ends_join() {
        // 真源 :21 —— 两端都有 → a…b
        assert_eq!(
            format_name_pattern(Some(&p(Some("a"), Some("b")))).as_deref(),
            Some("a…b")
        );
    }

    #[test]
    fn empty_pattern_returns_none() {
        // ★真源 :25-27 —— 两端都缺席返回 undefined，
        // 不留一个没有意义的省略号。
        assert_eq!(format_name_pattern(Some(&p(None, None))), None);
        assert_eq!(format_name_pattern(Some(&p(Some(""), Some("")))), None);
    }

    #[test]
    fn absent_pattern_returns_none() {
        assert_eq!(format_name_pattern(None), None);
    }

    #[test]
    fn ellipsis_is_single_char_not_asterisk() {
        // ★真源 :12-14 —— 省略号（不是 `*` 或 `${…}`）读起来仍是一个名字。
        let out = format_name_pattern(Some(&p(Some("研究员"), None))).unwrap();
        assert_eq!(out.chars().count(), "研究员".chars().count() + 1);
        assert!(out.ends_with('…'));
        assert!(!out.contains('*'));
        assert!(!out.contains('$'));
    }
}