//! 1:1 翻译 `packages/ui/src/lib/toolError.ts`（110 行）。
//!
//! 工具失败正文的提取阶梯：直接 error → output 结构化字段 → <tool_use_error>
//! 标签包裹的文本 → raw 层逐级兜底。失败态卡片能否读出原因，全靠这条链。

use serde_json::Value;

/// `readNonEmptyString`（:9-19）：trim 后非空才算。
fn read_non_empty_string(value: &Value) -> Option<&str> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty())
}

/// `readFirstStringField`（:21-31）：按候选键顺序取第一个非空字符串。
fn read_first_string_field<'a>(record: &'a Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(v) = record.get(*key) {
            if let Some(found) = read_non_empty_string(v) {
                return Some(found);
            }
        }
    }
    None
}

/// `stripMarkdownCodeFence`（:33-38）：剥一层 ``` 围栏。
fn strip_markdown_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    let after_fence = trimmed
        .strip_prefix("```")
        .map(|rest| {
            // 吃掉语言标记行（```rust → rest 去到换行前）。
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-');
            rest.strip_prefix('\n').unwrap_or(rest)
        })
        .unwrap_or(trimmed);
    // 吃掉尾部围栏。
    after_fence
        .strip_suffix("```")
        .map(|body| body.trim_end())
        .unwrap_or(after_fence)
        .trim()
}

/// `normalizeWrappedErrorText`（:40-48）：剥围栏 + 提取 <tool_use_error> 包裹体。
pub fn normalize_wrapped_error_text(text: &str) -> String {
    let unwrapped = strip_markdown_code_fence(text);
    let open = "<tool_use_error>";
    let close = "</tool_use_error>";
    let lower = unwrapped.to_lowercase();
    if let (Some(start), Some(end)) = (lower.find(open), lower.find(close)) {
        if end > start {
            let body = &unwrapped[start + open.len()..end];
            return body.trim().to_string();
        }
    }
    unwrapped.to_string()
}

/// `readTaggedToolErrorText`（:50-58）：仅当文本带 tool_use_error 标签才提取。
fn read_tagged_tool_error_text(value: &Value) -> Option<String> {
    let text = read_non_empty_string(value)?;
    let lower = text.to_lowercase();
    let open = "<tool_use_error>";
    let close = "</tool_use_error>";
    let has_pair = lower.contains(open) && lower.contains(close);
    if !has_pair {
        return None;
    }
    Some(normalize_wrapped_error_text(text))
}

/// `getToolCallErrorText`（:60-109）：失败正文提取主入口。
///
/// 阶梯（真源同序）：
/// 1. `toolCall.error` 直接字符串；
/// 2. `output` 记录的 `error` / `message` 字段；
/// 3. `output` 的 <tool_use_error> 包裹文本；
/// 4. `raw.rawOutput` 记录的 error/message；
/// 5. `raw.rawOutput` 的包裹文本；
/// 6. status=failed 时 `raw.content` 块内的 error/message/text。
pub fn get_tool_call_error_text(
    error: Option<&str>,
    output: &Value,
    raw: &Value,
    status: &str,
) -> Option<String> {
    // 1. 直接 error。
    if let Some(e) = error.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(e.to_string());
    }

    // 2. output 记录的 error/message。
    // 注意：3（标签提取）在真源里位于 isRecord 判断**之外**（:69-73）——
    // output 是裸字符串（<tool_use_error> 包裹体）也要提取。
    if output.is_object() {
        if let Some(found) = read_first_string_field(output, &["error", "message"]) {
            return Some(found.to_string());
        }
    }
    if let Some(tagged) = read_tagged_tool_error_text(output) {
        return Some(tagged);
    }

    if !raw.is_object() {
        return None;
    }

    // 4. raw.rawOutput 记录。
    let raw_output = raw.get("rawOutput").filter(|v| v.is_object());
    if let Some(found) = raw_output.and_then(|r| read_first_string_field(r, &["error", "message"]))
    {
        return Some(found.to_string());
    }

    // 5. raw.rawOutput 的包裹文本。
    let raw_status = raw.get("status").and_then(read_non_empty_string);
    if let Some(tagged) = raw.get("rawOutput").and_then(read_tagged_tool_error_text) {
        return Some(tagged);
    }

    // 6. failed 时 content 块。
    if status == "failed" || raw_status == Some("failed") {
        if let Some(blocks) = raw.get("content").and_then(|c| c.as_array()) {
            for block in blocks {
                if !block.is_object() {
                    continue;
                }
                let nested_content = block.get("content").filter(|c| c.is_object());
                let block_error = nested_content
                    .and_then(|c| read_first_string_field(c, &["error", "message", "text"]))
                    .or_else(|| read_first_string_field(block, &["error", "message", "text"]));
                if let Some(found) = block_error {
                    return Some(normalize_wrapped_error_text(found));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_unwraps_tool_use_error_tag() {
        // 真源 :40-48 —— 提取标签体。
        assert_eq!(
            normalize_wrapped_error_text("<tool_use_error>\nboom\n</tool_use_error>"),
            "boom"
        );
        // 大小写不敏感（真源 /i）。
        assert_eq!(
            normalize_wrapped_error_text("<TOOL_USE_ERROR>x</TOOL_USE_ERROR>"),
            "x"
        );
        // 无标签原样返回（剥围栏后 trim）。
        assert_eq!(normalize_wrapped_error_text("  plain  "), "plain");
    }

    #[test]
    fn strip_markdown_code_fence_takes_inner_text() {
        // 真源 :33-38 —— 只剥一层围栏，吃语言标记。
        assert_eq!(strip_markdown_code_fence("```text\nhello\n```"), "hello");
        assert_eq!(strip_markdown_code_fence("```\nhello\n```"), "hello");
        assert_eq!(strip_markdown_code_fence("no fence"), "no fence");
    }

    #[test]
    fn error_text_ladder_order() {
        // 1. 直接 error 优先。
        let out = json!({"error": "structured"});
        let raw = json!({});
        assert_eq!(
            get_tool_call_error_text(Some("direct"), &out, &raw, "failed").as_deref(),
            Some("direct")
        );
        // 2. output.message 次之。
        assert_eq!(
            get_tool_call_error_text(None, &out, &raw, "failed").as_deref(),
            Some("structured")
        );
        // 3. output 的包裹文本。
        let wrapped = json!("<tool_use_error>wrapped</tool_use_error>");
        assert_eq!(
            get_tool_call_error_text(None, &wrapped, &raw, "failed").as_deref(),
            Some("wrapped")
        );
    }

    #[test]
    fn error_text_raw_content_fallback_needs_failed_status() {
        // 真源 :86-105 —— content 块兜底只在 failed 态走。
        let raw = json!({"content": [{"type": "text", "text": "块内错误"}]});
        let empty = json!({});
        assert_eq!(
            get_tool_call_error_text(None, &empty, &raw, "failed").as_deref(),
            Some("块内错误")
        );
        // 非 failed：不读 content。
        assert_eq!(
            get_tool_call_error_text(None, &empty, &raw, "completed"),
            None
        );
        // raw.status == failed 也能触发。
        let raw_status = json!({"status": "failed", "content": [{"text": "raw失败"}]});
        assert_eq!(
            get_tool_call_error_text(None, &empty, &raw_status, "completed").as_deref(),
            Some("raw失败")
        );
    }

    #[test]
    fn empty_and_non_object_shapes_are_safe() {
        let raw = json!({"rawOutput": "不是对象"});
        let out = json!("字符串output");
        assert_eq!(
            get_tool_call_error_text(Some("  "), &out, &raw, "failed"),
            None
        );
        assert_eq!(get_tool_call_error_text(None, &out, &raw, "failed"), None);
    }
}
