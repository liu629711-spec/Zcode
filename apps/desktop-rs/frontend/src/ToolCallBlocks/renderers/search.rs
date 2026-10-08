//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/search.tsx`（134 行）。
//!
//! search / grep / glob / WebFetch 卡。真源要点：
//! - **`canToggle={false}`**（:104）—— 与 read 一样不可折叠
//! - **主文本直接拼成一句完整摘要**，不拆 secondaryText（真源 :100-102 注释）：
//!   「search 的主文本直接拼成一句完整摘要，不再拆 secondaryText；
//!   这样能避免 title 干扰，也更适合列表/目录查询这类操作」
//! - `content={null}`（:113）—— 无折叠主体
//! - `primaryText` 包一层 `<span className="truncate">`（:86）
//!
//! 查询词来源有多级（真源 :29-73），其中 url/prompt 的顺序有讲究：
//! 真源注释（:61-62）说「WebFetch 同时带 url 和 prompt 时，权限/工具摘要展示
//! prompt 会遮住真正需要用户确认的目标地址」——所以 url 排在 prompt **之前**。
//!
//! 行号注释均指真源文件。

use leptos::prelude::*;
use serde_json::Value;

use crate::ToolCallBlocks::fileSummaryTypes::is_plain_record;

/// `SEARCH_TOOL_ICON`（真源 :9）——SearchIcon。
pub const SEARCH_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// i18n 文案（zh-CN.ts:4839, 5614-5618）。
const FIND: &str = "查找";
const LIST: &str = "列出";
const SEARCH: &str = "搜索";
const SEARCHING: &str = "正在搜索";

/// `getSearchPrimaryText`（真源 :21-81）——摘要主文本。
///
/// 优先级：
/// - 字符串入参 → 「查找 {query}」或「查找」
/// - `parsed_cmd[]` 里 `list_files` → 「列出 {cwd}」/「列出」
/// - `parsed_cmd[]` 里 `search|grep|glob` → 「查找 {pattern|query|path}」
/// - 十个候选键兜底（:61-69，**url 在 prompt 之前**）
pub fn get_search_primary_text(input: &Value) -> String {
    // ① 字符串入参（真源 :22-27）
    if let Some(s) = input.as_str() {
        let t = s.trim();
        return if t.is_empty() {
            FIND.to_string()
        } else {
            format!("查找 {t}")
        };
    }

    if !is_plain_record(input) {
        return FIND.to_string();
    }

    // ② parsed_cmd（真源 :33-60）
    if let Some(parsed) = input.get("parsed_cmd").and_then(|v| v.as_array()) {
        for item in parsed {
            if !is_plain_record(item) {
                continue;
            }
            let Some(item_type) = item.get("type").and_then(|v| v.as_str()) else {
                continue;
            };

            // list_files → 列目录（真源 :38-45）
            if item_type == "list_files" {
                let cwd = input
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim_end_matches('/').to_string());
                return match cwd {
                    Some(cwd) => format!("列出 {cwd}"),
                    None => LIST.to_string(),
                };
            }

            // search / grep / glob → 查关键词（真源 :47-59）
            if item_type == "search" || item_type == "grep" || item_type == "glob" {
                let candidate = ["pattern", "query", "path"]
                    .iter()
                    .find_map(|k| item.get(k).and_then(|v| v.as_str()).map(str::trim))
                    .unwrap_or("");
                return if candidate.is_empty() {
                    FIND.to_string()
                } else {
                    format!("查找 {candidate}")
                };
            }
        }
    }

    // ③ 十个候选键兜底（真源 :61-73）
    // **顺序有意义**：url 在 prompt 之前——真源注释说 WebFetch 同时带两者时
    // 展示 prompt 会遮住真正需要用户确认的目标地址。
    for key in [
        "search_query",
        "searchQuery",
        "query",
        "pattern",
        "path",
        "url",
        "prompt",
        "target",
        "name",
    ] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return format!("查找 {t}");
            }
        }
    }

    FIND.to_string()
}

/// SearchToolCallBlock 的 props。
#[derive(Debug, Clone)]
pub struct SearchBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub kind_label_override: Option<String>,
}

/// SearchToolCallBlock 的主体（真源 :83-134）。
#[component]
pub fn SearchToolCallBlock(props: SearchBlockProps) -> impl IntoView {
    // 真源 :100-102 —— 主文本是拼好的完整摘要，不拆 secondaryText。
    let search_primary = get_search_primary_text(&props.input);

    // 真源 :105-109 —— 运行态「正在搜索」，否则「搜索」。
    let kind_label = props.kind_label_override.clone().unwrap_or_else(|| {
        if props.is_running {
            SEARCHING.to_string()
        } else {
            SEARCH.to_string()
        }
    });

    let is_failed = props.status.as_deref() == Some("failed");
    let status_tooltip = if is_failed {
        props.error_text.clone()
    } else {
        None
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                // 真源 :104 —— search 卡不可折叠。
                can_toggle: Some(false),
                kind_label: Some(kind_label.clone()),
                source_label: props.source_label.clone(),
                primary_text: Some(search_primary.clone()),
                status_label: props.status_label.clone(),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                // 真源 :113 —— 无折叠主体。
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=SEARCH_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec!["m21 21-4.34-4.34"]
                            circles=vec![("11", "11", "8")]
                        />
                    </span>
                }
                .into_any()
            }))
            render_content=None
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_string_input_uses_find_with_query() {
        // 真源 :22-27
        assert_eq!(get_search_primary_text(&json!("error")), "查找 error");
        assert_eq!(get_search_primary_text(&json!("  ")), "查找", "空串 → 查找");
    }

    #[test]
    fn non_record_input_falls_back_to_find() {
        // 真源 :30-32
        assert_eq!(get_search_primary_text(&json!(null)), "查找");
        assert_eq!(get_search_primary_text(&json!([])), "查找");
        assert_eq!(get_search_primary_text(&json!(42)), "查找");
    }

    #[test]
    fn list_files_uses_cwd() {
        // 真源 :38-45 —— cwd 尾部斜杠要裁掉。
        let input = json!({ "cwd": "/work/dir/", "parsed_cmd": [{ "type": "list_files" }] });
        assert_eq!(get_search_primary_text(&input), "列出 /work/dir");
        // 无 cwd → 列出
        let input2 = json!({ "parsed_cmd": [{ "type": "list_files" }] });
        assert_eq!(get_search_primary_text(&input2), "列出");
    }

    #[test]
    fn search_grep_glob_use_pattern_query_path_in_order() {
        // 真源 :47-59 —— pattern > query > path
        for t in ["search", "grep", "glob"] {
            let input = json!({ "parsed_cmd": [{ "type": t, "pattern": "TODO" }] });
            assert_eq!(
                get_search_primary_text(&input),
                "查找 TODO",
                "{t} 应取 pattern"
            );
            let input2 = json!({ "parsed_cmd": [{ "type": t, "query": "FIXME" }] });
            assert_eq!(
                get_search_primary_text(&input2),
                "查找 FIXME",
                "{t} 应取 query"
            );
            let input3 = json!({ "parsed_cmd": [{ "type": t, "path": "src" }] });
            assert_eq!(
                get_search_primary_text(&input3),
                "查找 src",
                "{t} 应取 path"
            );
            // pattern 优先于 query
            let input4 = json!({ "parsed_cmd": [{ "type": t, "pattern": "A", "query": "B" }] });
            assert_eq!(get_search_primary_text(&input4), "查找 A");
        }
    }

    #[test]
    fn url_wins_over_prompt() {
        // ★真源 :61-62 注释：WebFetch 同时带 url 和 prompt 时，
        // 展示 prompt 会遮住真正需要用户确认的目标地址 → url 必须在前。
        let input = json!({ "url": "https://example.com", "prompt": "总结这个页面" });
        assert_eq!(
            get_search_primary_text(&input),
            "查找 https://example.com",
            "url 应优先于 prompt"
        );
    }

    #[test]
    fn candidate_keys_fallback_order() {
        // 真源 :61-69 —— 十个候选键按序取第一个非空。
        assert_eq!(
            get_search_primary_text(&json!({ "search_query": "q1" })),
            "查找 q1"
        );
        assert_eq!(
            get_search_primary_text(&json!({ "query": "q2", "pattern": "p2" })),
            "查找 q2",
            "query 在 pattern 前"
        );
        assert_eq!(
            get_search_primary_text(&json!({ "name": "n" })),
            "查找 n",
            "name 是最后一个候选"
        );
    }

    #[test]
    fn unknown_tool_falls_back_to_find() {
        // 真源 :75 —— 全都没有 → 查找
        assert_eq!(get_search_primary_text(&json!({ "foo": "bar" })), "查找");
        assert_eq!(get_search_primary_text(&json!({})), "查找");
    }

    #[test]
    fn empty_candidate_values_are_skipped() {
        // 空串要跳过，继续找下一个键。
        let input = json!({ "query": "  ", "pattern": "real" });
        assert_eq!(get_search_primary_text(&input), "查找 real");
    }
}
