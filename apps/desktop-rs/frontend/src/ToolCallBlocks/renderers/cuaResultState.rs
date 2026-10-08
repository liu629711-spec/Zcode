//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaResultState.ts`
//! （219 行）。
//!
//! CUA 结果状态解析：应用身份（bundle_id / name）、列表计数、动作目标元素名。
//!
//! 真源注释（:43-46）：部分 MCP 结果在有效主 JSON 后**只追加空的**
//! `Structured content:` 标记——无条件解析标记后的空串会让应用身份丢失并降级
//! 成 Computer Use。

use std::sync::OnceLock;

use regex_lite::Regex;
use serde_json::Value;

use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};
use super::cuaErrorDetails::read_cua_error_details;

/// `asRecord`（真源的 `typeof value === "object" && !Array.isArray`）。
///
/// 签名用 `impl Borrow<Value>`：Rust 的 `Borrow<T> for T` blanket impl 让
/// `as_record(v)` 与 `as_record(&v)` 两种传法都成立（TS 的宽松 record 判定
/// 不区分引用形态）。
fn as_record(value: impl std::borrow::Borrow<Value>) -> Option<Value> {
    value.borrow().as_object().map(|o| Value::Object(o.clone()))
}

fn parse_record(value: &Value) -> Option<Value> {
    if let Some(record) = as_record(value) {
        return Some(record);
    }
    let text = value.as_str()?;
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| as_record(&v))
}

fn read_text(record: Option<&Value>, key: &str) -> Option<String> {
    record
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `parseCuaTextAppState`（真源 :26-33）：`app: <bundle> pid=N "<name>"` 文本头。
fn parse_cua_text_app_state(value: &str) -> Option<Value> {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源 :27 —— `/^app:\s+([A-Za-z0-9.-]+)\s+pid=\d+\s+"([^"\r\n]+)"\s*$/mu`
    // （Rust 正则默认 Unicode 语义，`u` flag 无对应）。
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?m)^app:\s+([A-Za-z0-9.-]+)\s+pid=\d+\s+"([^"\r\n]+)"\s*$"#)
            .expect("app 文本头正则")
    });
    let caps = re.captures(value)?;
    let bundle_id = caps.get(1)?.as_str();
    let name = caps.get(2)?.as_str();
    Some(serde_json::json!({
        "app": {"bundle_id": bundle_id, "name": name}
    }))
}

/// `parseCuaResultState`（真源 :35-76）：多候选 JSON 解析 + result 字符串解包
/// + 文本 app 头回退。
pub fn parse_cua_result_state(value: &Value) -> Option<Value> {
    let text = value.as_str()?;
    const STRUCTURED_MARKER: &str = "Structured content:";
    let structured_start = text.rfind(STRUCTURED_MARKER);
    let json_start = text.rfind("\n\n{");
    let candidates: Vec<&str> = match structured_start {
        Some(index) => vec![
            text[index + STRUCTURED_MARKER.len()..].trim(),
            text[..index].trim(),
        ],
        None => match json_start {
            Some(index) => vec![text[index + 2..].trim(), text.trim()],
            None => vec![text.trim()],
        },
    };

    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(candidate) {
            Ok(parsed) => {
                let Some(parsed) = as_record(&parsed) else {
                    continue;
                };
                // MCP structuredContent 会把 CUA JSON 再包成 result 字符串——
                // 不解包则名称等结构化字段丢失并错误回退到 bundle_id。
                if let Some(Value::String(wrapped)) = parsed.get("result") {
                    return match serde_json::from_str::<Value>(wrapped) {
                        Ok(inner) => as_record(&inner).or(Some(parsed)),
                        Err(_) => Some(parsed),
                    };
                }
                return Some(parsed);
            }
            Err(_) => {
                // get_app_state 的成功结果可能是面向模型的文本状态而非 JSON——
                // 其稳定 app 头已含唯一目标应用，忽略会让摘要错误降级。
                if let Some(text_state) = parse_cua_text_app_state(candidate) {
                    return Some(text_state);
                }
            }
        }
    }
    None
}

/// `readCuaResultBundleId`（真源 :78-86）。
pub fn read_cua_result_bundle_id(output: Option<&str>, raw: &Value) -> Option<String> {
    if let Some(bundle_id) = read_cua_error_details(output, raw).and_then(|d| d.target_bundle_id) {
        return Some(bundle_id);
    }
    let raw_output = read_text(as_record(raw).as_ref(), "rawOutput");
    let result = parse_cua_result_state(&Value::String(output.unwrap_or("").to_string()))
        .or_else(|| parse_cua_result_state(&Value::String(raw_output?)));
    let result_app = result
        .as_ref()
        .and_then(|r| as_record(r.get("app").cloned().unwrap_or(Value::Null)))
        .or_else(|| {
            result
                .as_ref()
                .and_then(|r| as_record(r.get("owner").cloned().unwrap_or(Value::Null)))
        })
        .or_else(|| result.clone());
    read_text(result_app.as_ref(), "bundle_id")
        .or_else(|| read_text(result_app.as_ref(), "bundleId"))
}

fn read_cua_input_app(input: &Value) -> Option<Value> {
    let input_record = as_record(input);
    parse_record(&input_record.as_ref()?.get("app").cloned()?)
        .or_else(|| parse_record(&input_record.as_ref()?.get("app_ref").cloned()?))
}

/// `parseCuaResultArrayLength`（真源 :98-110）。
fn parse_cua_result_array_length(value: &Value) -> Option<usize> {
    let text = value.as_str()?;
    let marker_start = text.rfind("\n\nStructured content:");
    let candidate = marker_start
        .map(|index| text[..index].trim())
        .unwrap_or(text)
        .trim();
    if candidate.is_empty() {
        return None;
    }
    let parsed: Value = serde_json::from_str(candidate).ok()?;
    parsed.as_array().map(|a| a.len())
}

/// `readCuaResultListCount`（真源 :112-124）。
pub fn read_cua_result_list_count(output: Option<&str>, raw: &Value) -> Option<usize> {
    if let Some(ToolResultDisplay::Cua(display)) = read_tool_result_display(raw) {
        if let Some(count) = display
            .text
            .as_deref()
            .and_then(|t| parse_cua_result_array_length(&Value::String(t.to_string())))
        {
            return Some(count);
        }
    }
    let raw_output = read_text(as_record(raw).as_ref(), "rawOutput");
    parse_cua_result_array_length(&Value::String(output.unwrap_or("").to_string()))
        .or_else(|| parse_cua_result_array_length(&Value::String(raw_output?)))
}

/// `readCuaOutputText`（真源 :126-132）。
fn read_cua_output_text(output: Option<&str>, raw: &Value) -> Option<String> {
    if let Some(ToolResultDisplay::Cua(display)) = read_tool_result_display(raw) {
        if let Some(text) = display.text.filter(|t| !t.is_empty()) {
            return Some(text);
        }
    }
    if let Some(text) = output.filter(|t| !t.trim().is_empty()) {
        return Some(text.to_string());
    }
    read_text(as_record(raw).as_ref(), "rawOutput")
}

/// `stripElementRole`（真源 :134-136）。
fn strip_element_role(value: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源 :134 —— `/^\S+\s+/u`
    let re = RE.get_or_init(|| Regex::new(r"^\S+\s+").expect("角色剥离正则"));
    re.replace(value, "").trim().to_string()
}

/// `readElementName`（真源 :138-152）。
fn read_element_name(element_line: &str) -> Option<String> {
    static PAREN_RE: OnceLock<Regex> = OnceLock::new();
    static TEXTAREA_RE: OnceLock<Regex> = OnceLock::new();
    static VALUE_RE: OnceLock<Regex> = OnceLock::new();
    // 真源 :139 —— `/\s+\([^)]*\)\s*$/u`
    let paren_re = PAREN_RE.get_or_init(|| Regex::new(r"\s+\([^)]*\)\s*$").expect("尾部括号正则"));
    let content = paren_re.replace(element_line, "").trim().to_string();
    let Some(separator) = content.rfind(" = ") else {
        return (!strip_element_role(&content).is_empty()).then(|| strip_element_role(&content));
    };
    let left = content[..separator].trim().to_string();
    let right = content[separator + 3..].trim().to_string();
    // textarea 右侧是跨行正文预览——当目标会让摘要泄露大段文档内容。
    let textarea_re =
        TEXTAREA_RE.get_or_init(|| Regex::new(r"(?i)^textarea\s+").expect("textarea 正则"));
    if textarea_re.is_match(&left) {
        let stripped = strip_element_role(&left);
        return (!stripped.is_empty()).then_some(stripped);
    }
    // 值型控件右侧是 0/1 等状态，不是目标名；文本节点等描述型元素以右侧为名。
    let value_re = VALUE_RE
        .get_or_init(|| Regex::new(r"(?i)^(?:-?\d+(?:\.\d+)?|true|false|null)$").expect("值正则"));
    if value_re.is_match(&right) {
        let stripped = strip_element_role(&left);
        return (!stripped.is_empty()).then_some(stripped);
    }
    (!right.is_empty()).then_some(right)
}

/// `readCuaActionTargetName`（真源 :154-186）。
pub fn read_cua_action_target_name(
    input: &Value,
    output: Option<&str>,
    raw: &Value,
) -> Option<String> {
    let input_record = as_record(input);
    let target_value = input_record
        .as_ref()
        .and_then(|i| i.get("target").cloned())
        .unwrap_or(Value::Null);
    let target = parse_record(&target_value)?;
    if target.get("type").and_then(|v| v.as_str()) != Some("element") {
        return None;
    }
    let index = target.get("index").and_then(|v| v.as_i64())?;
    let output_text = read_cua_output_text(output, raw)?;
    let lines: Vec<&str> = output_text
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .collect();

    // 真源 :165 —— 按目标索引动态构造行正则。
    let pattern = format!(r"^\s*\[{index}\]\s+(.+)$");
    let element_re = Regex::new(&pattern).ok()?;
    let target_line_index = lines.iter().position(|line| element_re.is_match(line))?;
    let target_line = lines[target_line_index];
    let element_line = element_re
        .captures(target_line)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())?;
    let direct_name = read_element_name(element_line);
    let index_text = index.to_string();
    if direct_name.as_deref() != Some(index_text.as_str()) {
        return direct_name;
    }

    // CUA 会把按钮名拆成紧随其后的扁平文本节点，按钮自身只留数字索引。
    let any_re = Regex::new(r"^\s*\[\d+\]\s+(.+)$").ok()?;
    let text_re = Regex::new(r"(?i)^text\s+").ok()?;
    let mut child_names: Vec<String> = Vec::new();
    for line in lines.iter().skip(target_line_index + 1) {
        let Some(child_line) = any_re
            .captures(line)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
        else {
            break;
        };
        if !text_re.is_match(child_line) {
            break;
        }
        let Some(child_name) = read_element_name(child_line) else {
            break;
        };
        child_names.push(child_name);
    }

    if !child_names.is_empty() {
        Some(child_names.concat())
    } else {
        direct_name
    }
}

/// `readCuaResultState`（真源 :188-196）。
pub fn read_cua_result_state(output: Option<&str>, raw: &Value) -> Option<Value> {
    if let Some(ToolResultDisplay::Cua(display)) = read_tool_result_display(raw) {
        // 新 session 的 display 仍保留 MCP `{ result: "...json..." }` 包装——
        // 必须复用同一解包逻辑，否则 display 优先路径反而读不到 App 名称。
        if let Some(structured) = display
            .structured_content
            .as_deref()
            .and_then(|t| parse_cua_result_state(&Value::String(t.to_string())))
        {
            return Some(structured);
        }
    }
    let raw_output = read_text(as_record(raw).as_ref(), "rawOutput");
    parse_cua_result_state(&Value::String(output.unwrap_or("").to_string()))
        .or_else(|| parse_cua_result_state(&Value::String(raw_output?)))
}

/// `readCuaAppName`（真源 :198-218）。
pub fn read_cua_app_name(input: &Value, result: Option<&Value>) -> Option<String> {
    let input_app = read_cua_input_app(input);
    let result_app = result
        .and_then(|r| as_record(r.get("app").cloned().unwrap_or(Value::Null)))
        .or_else(|| result.and_then(|r| as_record(r.get("owner").cloned().unwrap_or(Value::Null))))
        .or_else(|| result.cloned());
    if let Some(name) = read_text(result_app.as_ref(), "name")
        .or_else(|| read_text(result_app.as_ref(), "display_name"))
        .or_else(|| read_text(input_app.as_ref(), "name"))
    {
        return Some(name);
    }
    let bundle_id = read_text(result_app.as_ref(), "bundle_id")
        .or_else(|| read_text(input_app.as_ref(), "bundle_id"));
    // Finder 的 list_windows 等结果只返回系统 bundle ID——用标准名称，
    // 避免摘要与详情暴露 com.apple.finder。
    match bundle_id.as_deref() {
        Some("com.apple.finder") => Some("Finder".to_string()),
        other => other.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_structured_marker_priority() {
        // 主 JSON 与 structured content 同时在：有真实 structured 时优先。
        let text = "prefix\n\n{\"app\":{\"name\":\"主\"}}\n\nStructured content:\n{\"app\":{\"name\":\"结构化\"}}";
        let result = parse_cua_result_state(&json!(text)).unwrap();
        assert_eq!(
            result["app"]["name"].as_str(),
            Some("结构化"),
            "有真实 structured content 时优先"
        );
        // 空 structured（部分 MCP 结果只追加标记）→ 回退 marker 之前的
        // 主 JSON。真源在 structuredStart 存在时不走 `lastIndexOf("\n\n{")`
        // 分支——回退段必须整体是 JSON，散文前缀会让解析失败（真源同）。
        let text = "{\"app\":{\"name\":\"主\"}}\n\nStructured content:";
        let result = parse_cua_result_state(&json!(text)).unwrap();
        assert_eq!(result["app"]["name"].as_str(), Some("主"));
    }

    #[test]
    fn unwraps_mcp_result_string() {
        let inner = "{\"app\":{\"name\":\"Safari\",\"bundle_id\":\"com.apple.Safari\"}}";
        let wrapped = serde_json::json!({ "result": inner }).to_string();
        let result = parse_cua_result_state(&json!(wrapped)).unwrap();
        assert_eq!(result["app"]["name"].as_str(), Some("Safari"));
    }

    #[test]
    fn text_app_state_fallback() {
        let text = "app: com.apple.finder pid=123 \"Finder\"\n其他内容";
        let result = parse_cua_result_state(&json!(text)).unwrap();
        assert_eq!(result["app"]["name"].as_str(), Some("Finder"));
        assert_eq!(
            result["app"]["bundle_id"].as_str(),
            Some("com.apple.finder")
        );
    }

    #[test]
    fn app_name_prefers_result_then_maps_finder() {
        // 结果名称优先。
        let result = json!({"app": {"name": "Safari"}});
        assert_eq!(
            read_cua_app_name(&json!({"app": {"name": "输入侧"}}), Some(&result)).as_deref(),
            Some("Safari")
        );
        // 无名称只有 bundle：Finder 映射为标准名。
        let result = json!({"app": {"bundle_id": "com.apple.finder"}});
        assert_eq!(
            read_cua_app_name(&json!({}), Some(&result)).as_deref(),
            Some("Finder")
        );
        // 其他 bundle 原样。
        let result = json!({"app": {"bundle_id": "com.other.App"}});
        assert_eq!(
            read_cua_app_name(&json!({}), Some(&result)).as_deref(),
            Some("com.other.App")
        );
    }

    #[test]
    fn element_name_skips_textarea_body_and_values() {
        // textarea 右侧是正文 → 取左侧角色后的控件名（stripElementRole
        // 只剥「角色 + 空白」，名称自带的引号保留——真源同）。
        assert_eq!(
            read_element_name("textarea \"输入框\" = 这是跨行正文").as_deref(),
            Some("\"输入框\"")
        );
        // 数值状态 → 取左侧控件名（同样保留名称引号）。
        assert_eq!(
            read_element_name("checkbox \"同意\" = true").as_deref(),
            Some("\"同意\"")
        );
        // 描述型元素（text 节点）→ 取右侧。
        assert_eq!(
            read_element_name("StaticText \"提交\" = 提交").as_deref(),
            Some("提交")
        );
    }

    #[test]
    fn action_target_name_joins_flattened_text_children() {
        let input = json!({"target": {"type": "element", "index": 12}});
        let output = "[12] button 12\n[13] text 提交\n[14] button 确定\n[15] text 取消";
        let name = read_cua_action_target_name(&input, Some(output), &json!({}));
        // 按钮自身只有数字索引 → 拼接紧随的连续 text 节点。
        assert_eq!(name.as_deref(), Some("提交"));
    }

    #[test]
    fn list_count_reads_array_lengths() {
        // 真源：数组取 structured marker **之前**的段（marker 后是结构化补充）。
        let raw = json!({"display": {"kind": "cua", "schemaVersion": 1, "toolName": "list_windows", "status": "success", "text": "[1,2,3]\n\nStructured content:\n{}"}});
        assert_eq!(read_cua_result_list_count(Some("x"), &raw), Some(3));
        // 裸数组输出。
        assert_eq!(
            read_cua_result_list_count(Some("[1,2]"), &json!({})),
            Some(2)
        );
        assert_eq!(read_cua_result_list_count(Some("{}"), &json!({})), None);
    }
}
