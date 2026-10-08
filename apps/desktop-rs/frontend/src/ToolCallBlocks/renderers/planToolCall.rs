//! 1:1 翻译 `packages/ui/src/lib/planToolCall.ts`（103 行）+
//! `packages/ui/src/lib/path.ts` 的 `isAbsoluteFilePath` / `joinFilePath`
//! （后两个 Rust 侧此前未迁，本次随本文件补齐）。
//!
//! 计划类工具（switch_mode/exit_plan 等）的正文提取：input → inputText →
//! output → raw.rawInput/rawOutput → raw.content blocks 六路回落。
//! switch-mode 卡与计划目录都消费它。

use regex_lite::Regex;
use serde_json::Value;

/// `isAbsoluteFilePath`（path.ts:41-43）。
pub fn is_absolute_file_path(path: &str) -> bool {
    path.starts_with('/') || windows_absolute_re().is_match(path) || path.starts_with("\\\\")
}

fn windows_absolute_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z]:[\\/]").expect("windows 绝对路径正则"))
}

/// `joinFilePath`（path.ts:45-57）。
pub fn join_file_path(base_path: &str, child_path: &str) -> String {
    if child_path.is_empty() {
        return base_path.to_string();
    }
    if is_absolute_file_path(child_path) {
        return child_path.to_string();
    }

    // 分隔符：base 含 \ 且不含 / 时用 \，否则 /（真源同判）。
    let separator = if base_path.contains('\\') && !base_path.contains('/') {
        "\\"
    } else {
        "/"
    };
    let normalized_base = base_path.trim_end_matches(['\\', '/']);
    let normalized_child = child_path.trim_start_matches(['\\', '/']);
    format!("{normalized_base}{separator}{normalized_child}")
}

/// `PlanToolCallContent`（真源 :10-13）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlanToolCallContent {
    pub markdown: Option<String>,
    pub plan_file_path: Option<String>,
}

fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// `readStringField`（真源 :19-32）。
fn read_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(candidate) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = candidate.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// `resolvePlanFilePath`（真源 :34-37）。
fn resolve_plan_file_path(path: Option<String>, workspace_path: &str) -> Option<String> {
    let path = path?;
    if is_absolute_file_path(&path) {
        Some(path)
    } else {
        Some(join_file_path(workspace_path, &path))
    }
}

/// `extractPlanMarkdown`（真源 :39-53）。
fn extract_plan_markdown(source: &Value, workspace_path: &str) -> PlanToolCallContent {
    if let Some(s) = source.as_str() {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return PlanToolCallContent {
                markdown: Some(trimmed.to_string()),
                plan_file_path: None,
            };
        }
    }
    if !is_record(source) {
        return PlanToolCallContent::default();
    }
    let markdown = read_string_field(source, &["plan", "text", "content"]);
    // 真源注释（:50）：planFilePath 只在 markdown 命中时随行返回
    // （下方 `markdown ? { markdown, planFilePath } : {}`）。
    let plan_file_path =
        resolve_plan_file_path(read_string_field(source, &["planFilePath"]), workspace_path);
    if markdown.is_some() {
        PlanToolCallContent {
            markdown,
            plan_file_path,
        }
    } else {
        PlanToolCallContent::default()
    }
}

/// `extractPlanToolCallContent`（真源 :55-88）。
pub fn extract_plan_tool_call_content(
    input: &Value,
    input_text: &str,
    output: &Value,
    raw: &Value,
    workspace_path: &str,
) -> PlanToolCallContent {
    let input_content = extract_plan_markdown(input, workspace_path);
    if input_content.markdown.is_some() {
        return input_content;
    }

    if !input_text.trim().is_empty() {
        // 流式 inputText 可能暂时不是完整 JSON；解析失败继续走 legacy fallback。
        if let Ok(parsed) = serde_json::from_str::<Value>(input_text) {
            let content = extract_plan_markdown(&parsed, workspace_path);
            if content.markdown.is_some() {
                return content;
            }
        }
    }

    let output_content = extract_plan_markdown(output, workspace_path);
    if output_content.markdown.is_some() {
        return output_content;
    }

    if !is_record(raw) {
        return PlanToolCallContent::default();
    }
    for candidate in [raw.get("rawInput"), raw.get("rawOutput")] {
        if let Some(candidate) = candidate {
            let content = extract_plan_markdown(candidate, workspace_path);
            if content.markdown.is_some() {
                return content;
            }
        }
    }

    if let Some(entries) = raw.get("content").and_then(|c| c.as_array()) {
        for entry in entries {
            if !is_record(entry) {
                continue;
            }
            let nested = entry
                .get("content")
                .filter(|c| is_record(c))
                .unwrap_or(entry);
            let content = extract_plan_markdown(nested, workspace_path);
            if content.markdown.is_some() {
                return content;
            }
        }
    }
    PlanToolCallContent::default()
}

/// `getPlanDirectoryTitle`（真源 :93-102）：首个 H1，否则首个非空文本行。
pub fn get_plan_directory_title(markdown: &str) -> Option<String> {
    if let Some(caps) = h1_re().captures(markdown) {
        if let Some(title) = caps.get(1) {
            let title = title.as_str().trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    for line in markdown.split('\n') {
        let title = leading_decoration_re().replace(line, "").trim().to_string();
        if !title.is_empty() {
            return Some(title);
        }
    }
    None
}

fn h1_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // 真源：/^\s{0,3}#(?!#)\s+(.+?)\s*#*\s*$/m。regex-lite 无 look-ahead，
    // 但 `#(?!#)\s+` 与 `#\s+` 等价——`#` 必须紧跟空白，而空白不是 `#`，
    // 负向断言恒真。多行锚点用 (?m)。
    RE.get_or_init(|| Regex::new(r"(?m)^\s{0,3}#\s+(.+?)\s*#*\s*$").expect("H1 正则"))
}

fn leading_decoration_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s{0,3}(?:#{1,6}\s+|>\s*|[-*+]\s+)").expect("行首装饰正则"))
}

/// `getPlanFileLabel`（真源 :104-106）。
pub fn get_plan_file_label(plan_file_path: Option<&str>) -> Option<String> {
    plan_file_path.map(|p| super::get_path_leaf(p).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn absolute_path_detection() {
        // path.ts:41-43 —— / 开头、盘符、UNC。
        assert!(is_absolute_file_path("/home/x"));
        assert!(is_absolute_file_path("C:\\Users\\x"));
        assert!(is_absolute_file_path("d:/work"));
        assert!(is_absolute_file_path("\\\\server\\share"));
        assert!(!is_absolute_file_path("src/main.rs"));
        assert!(!is_absolute_file_path("./a"));
    }

    #[test]
    fn join_uses_backslash_only_for_pure_windows_base() {
        assert_eq!(join_file_path("C:\\work", "a.rs"), "C:\\work\\a.rs");
        assert_eq!(join_file_path("/home/u/", "a.rs"), "/home/u/a.rs");
        // 绝对 child 直接返回。
        assert_eq!(join_file_path("/base", "/abs/a.rs"), "/abs/a.rs");
        // 空 child 返回 base。
        assert_eq!(join_file_path("/base", ""), "/base");
    }

    #[test]
    fn input_string_short_circuits() {
        // 真源 :40-42 —— 字符串 source 即 markdown。
        let content =
            extract_plan_tool_call_content(&json!("# 计划正文"), "", &json!({}), &json!({}), "/ws");
        assert_eq!(content.markdown.as_deref(), Some("# 计划正文"));
    }

    #[test]
    fn ladder_input_then_input_text_then_output_then_raw() {
        let ws = "/ws";
        // input 记录体的 plan 字段。
        let content = extract_plan_tool_call_content(
            &json!({"plan": "输入计划", "planFilePath": "docs/p.md"}),
            "",
            &json!({}),
            &json!({}),
            ws,
        );
        assert_eq!(content.markdown.as_deref(), Some("输入计划"));
        // planFilePath 相对路径拼 workspace。
        assert_eq!(content.plan_file_path.as_deref(), Some("/ws/docs/p.md"));

        // inputText 是 JSON 字符串。
        let content = extract_plan_tool_call_content(
            &json!({}),
            "{\"text\":\"流式计划\"}",
            &json!({}),
            &json!({}),
            ws,
        );
        assert_eq!(content.markdown.as_deref(), Some("流式计划"));

        // output 字符串。
        let content =
            extract_plan_tool_call_content(&json!({}), "", &json!("输出计划"), &json!({}), ws);
        assert_eq!(content.markdown.as_deref(), Some("输出计划"));

        // raw.rawOutput。
        let content = extract_plan_tool_call_content(
            &json!({}),
            "",
            &json!({}),
            &json!({"rawOutput": {"content": "raw 计划"}}),
            ws,
        );
        assert_eq!(content.markdown.as_deref(), Some("raw 计划"));

        // raw.content 块（嵌套 content 优先）。
        let content = extract_plan_tool_call_content(
            &json!({}),
            "",
            &json!({}),
            &json!({"content": [{"type": "text", "content": {"text": "块内计划"}}]}),
            ws,
        );
        assert_eq!(content.markdown.as_deref(), Some("块内计划"));

        // 全落空。
        let content = extract_plan_tool_call_content(&json!({}), "", &json!({}), &json!({}), ws);
        assert_eq!(content, PlanToolCallContent::default());
    }

    #[test]
    fn markdown_hit_carries_plan_file_path_only_when_present() {
        // 真源 :51 —— 无 markdown 时整体空（path 不单独返回）。
        let content = extract_plan_tool_call_content(
            &json!({"planFilePath": "docs/p.md"}),
            "",
            &json!({}),
            &json!({}),
            "/ws",
        );
        assert_eq!(content, PlanToolCallContent::default());
    }

    #[test]
    fn directory_title_prefers_first_h1() {
        // 真源 :93-102。
        assert_eq!(
            get_plan_directory_title("intro\n# 计划标题 #\n## 小节").as_deref(),
            Some("计划标题")
        );
        // 无 H1 → 首个非空行剥装饰。
        assert_eq!(
            get_plan_directory_title("- 第一行\n更多").as_deref(),
            Some("第一行")
        );
        // 全空。
        assert_eq!(get_plan_directory_title("   \n\n"), None);
    }

    #[test]
    fn file_label_is_leaf() {
        assert_eq!(
            get_plan_file_label(Some("/ws/docs/plan.md")).as_deref(),
            Some("plan.md")
        );
        assert_eq!(get_plan_file_label(None), None);
    }
}
