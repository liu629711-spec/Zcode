//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaErrorDetails.ts`
//! （51 行）。
//!
//! CUA 工具失败详情：从 output 的 `"Error executing tool"` 标记之后截 JSON 解析，
//! 取错误码 / 状态 id / 元素索引 / 目标应用（名称 + bundle id）。

use serde_json::Value;

/// `CuaErrorDetails`（真源 :3-9）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuaErrorDetails {
    pub code: String,
    pub state_id: Option<String>,
    pub element_index: Option<i64>,
    pub target_app_name: Option<String>,
    pub target_bundle_id: Option<String>,
}

/// `asRecord`（真源 :11-15）。
/// `asRecord`（真源的 `typeof value === "object" && !Array.isArray`）。
///
/// 签名用 `impl Borrow<Value>`：Rust 的 `Borrow<T> for T` blanket impl 让
/// `as_record(v)` 与 `as_record(&v)` 两种传法都成立（TS 的宽松 record 判定
/// 不区分引用形态）。
fn as_record(value: impl std::borrow::Borrow<Value>) -> Option<Value> {
    value.borrow().as_object().map(|o| Value::Object(o.clone()))
}

/// `readText`（真源 :17-20）：非空白字符串（trim 后）。
fn read_text(record: Option<&Value>, key: &str) -> Option<String> {
    record
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `readCuaErrorDetails`（真源 :22-48）。
///
/// `output`：工具输出字符串（v4 投影下 `LegacyToolCall.output`）；
/// `raw`：原始载荷（取 `rawOutput` 回退）。
pub fn read_cua_error_details(output: Option<&str>, raw: &Value) -> Option<CuaErrorDetails> {
    let raw_record = as_record(raw);
    let text = match output {
        Some(o) => o,
        None => raw_record
            .as_ref()
            .and_then(|r| r.get("rawOutput"))
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    };
    // 真源 :29-31 —— marker 之后第一个 `{` 起截 JSON。
    const MARKER: &str = "Error executing tool";
    let marker_index = text.find(MARKER);
    let json_start = marker_index.and_then(|index| text[index..].find('{').map(|p| index + p));
    let json_start = json_start?;
    let parsed: Value = serde_json::from_str(&text[json_start..]).ok()?;
    let error = as_record(&parsed)?;
    let code = read_text(Some(&error), "error")?;
    let target_app = error.get("target_app").cloned().and_then(|v| as_record(&v));
    let element_index = error.get("index").and_then(|v| v.as_i64());
    Some(CuaErrorDetails {
        code,
        state_id: read_text(Some(&error), "state_id"),
        element_index,
        target_app_name: read_text(target_app.as_ref(), "name"),
        target_bundle_id: read_text(target_app.as_ref(), "bundle_id"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_error_json_after_marker() {
        let output = "Error executing tool: {\"error\":\"accessibility_denied\",\"state_id\":\"s1\",\"index\":3,\"target_app\":{\"name\":\"Finder\",\"bundle_id\":\"com.apple.finder\"}}";
        let details = read_cua_error_details(Some(output), &json!({})).unwrap();
        assert_eq!(details.code, "accessibility_denied");
        assert_eq!(details.state_id.as_deref(), Some("s1"));
        assert_eq!(details.element_index, Some(3));
        assert_eq!(details.target_app_name.as_deref(), Some("Finder"));
        assert_eq!(
            details.target_bundle_id.as_deref(),
            Some("com.apple.finder")
        );
    }

    #[test]
    fn falls_back_to_raw_output() {
        let raw = json!({"rawOutput": "Error executing tool {\"error\":\"boom\"}"});
        let details = read_cua_error_details(None, &raw).unwrap();
        assert_eq!(details.code, "boom");
        assert_eq!(details.element_index, None);
    }

    #[test]
    fn rejects_without_marker_or_code() {
        assert_eq!(read_cua_error_details(Some("普通输出"), &json!({})), None);
        // 有 marker 但无 error 字段 → None。
        assert_eq!(
            read_cua_error_details(Some("Error executing tool {\"a\":1}"), &json!({})),
            None
        );
    }
}
