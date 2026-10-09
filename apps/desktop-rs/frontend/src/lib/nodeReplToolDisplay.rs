//! 1:1 翻译 `packages/ui/src/lib/nodeReplToolDisplay.ts`（371 行）。
//!
//! node_repl 工具卡的展示模型：从 legacy toolCall 的 input / raw / output 里
//! 机械抽取用户面信息（标题、代码、结果文本、图片、错误、产物引用、目标应用）。
//! 纯函数层——不碰 DOM、不碰信号，被 renderers/node_repl.rs 消费。
//!
//! 与真源的偏离（Rust 侧）：
//! - 真源用对象**引用**做 `records.includes(record)` 与 `visited` 去重；
//!   Rust 的 serde_json::Value 是值语义（树、无环），结构相等即可去重
//!   （影响面：两份内容相同的对象在 JS 里会被走两遍，Rust 走一遍——
//!   读到的候选按序取首个，结果一致）；visited 循环防护随之不需要。
//! - 正则用 regex-lite（workspace 已有依赖；draft_scan 的手写扫描器
//!   是因为当时没有正则依赖，此处不重复造轮子）。
//! - 字符长度按 Unicode 标量数（chars().count()）近似 JS 的 UTF-16 长度；
//!   增补平面字符（emoji 等）两边会差 1，Compact 判定阈值 160 内的实际影响可忽略。

use regex_lite::Regex;
use serde_json::Value;

use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;

/// node_repl 操作类型（真源 `NodeReplOperation`，:3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeReplOperation {
    Run,
    Reset,
    AddModuleDir,
}

impl NodeReplOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Reset => "reset",
            Self::AddModuleDir => "add-module-dir",
        }
    }
}

/// `NodeReplDisplayImage`（真源 :5-8）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeReplDisplayImage {
    pub base64: String,
    pub mime_type: String,
}

/// `NodeReplDisplayError`（真源 :10-13）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeReplDisplayError {
    pub summary: String,
    pub stack: Option<String>,
}

/// `NodeReplPersistedResult`（真源 :15-18）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeReplPersistedResult {
    pub artifact_path: String,
    pub size_label: String,
}

/// 本次 cell 操作的目标应用（Computer Use）；由 CLI 的 node_repl display 携带
/// （真源 `NodeReplCuaApp`，:20-24）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeReplCuaApp {
    pub app_key: String,
    pub display_name: Option<String>,
}

/// 展示源（真源 :35 `displaySource?: "browser_turn_end"`——闭集字面量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeReplDisplaySource {
    BrowserTurnEnd,
}

/// `NodeReplDisplayModel`（真源 :26-37）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodeReplDisplayModel {
    pub operation: Option<NodeReplOperation>,
    pub user_title: Option<String>,
    pub code: Option<String>,
    pub module_directory: Option<String>,
    pub result_text: Option<String>,
    pub error: Option<NodeReplDisplayError>,
    pub images: Vec<NodeReplDisplayImage>,
    pub persisted_result: Option<NodeReplPersistedResult>,
    pub display_source: Option<NodeReplDisplaySource>,
    pub app: Option<NodeReplCuaApp>,
}

// 真源 :39-45 的六个模式。
//
// IMPLEMENTATION_TITLE_PATTERN（:39）：实现性标题（js / javascript / node repl）
// ——用户没给 cell 起名时 CLI 会拿代码首行或操作词当 title，这些不配当用户标题。
const IMPLEMENTATION_TITLE_PATTERN: &str = r"(?:\bjs\b|\bjavascript\b|node[\s_-]*repl)";
// LEADING_BLANK_LINES_PATTERN（:40）：手写实现（只剥首个有效行前的空白行）。
// PROJECTED_COMPLETION_MARKER_PATTERN（:41）：手写实现（逐行剥 "=> " 前缀）。
// PROJECTED_IMAGE_PLACEHOLDER_PATTERN（:42）。
const PROJECTED_IMAGE_PLACEHOLDER_PATTERN: &str = r"^\[Attached image/[^\]]+\]$";
// IMAGE_MIME_TYPE_PATTERN（:43）。
const IMAGE_MIME_TYPE_PATTERN: &str = r"^image/[a-z0-9.+-]+$";
// PERSISTED_OUTPUT_PATTERN（:44-45）：超长结果的落盘信封。
const PERSISTED_OUTPUT_PATTERN: &str = concat!(
    r"^<persisted-output>\s*\nOutput too large \(([^)]+)\)\. Full output saved to: ([^\n]+)\n\n",
    r"Preview \([^)]+\):\n([\s\S]*?)\n</persisted-output>\s*$"
);

fn implementation_title_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!("(?i){IMPLEMENTATION_TITLE_PATTERN}")).unwrap())
}

fn projected_image_placeholder_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!("(?u){PROJECTED_IMAGE_PLACEHOLDER_PATTERN}")).unwrap())
}

fn image_mime_type_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!("(?iu){IMAGE_MIME_TYPE_PATTERN}")).unwrap())
}

fn persisted_output_re() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(PERSISTED_OUTPUT_PATTERN).unwrap())
}

/// `isRecord`（真源 :47-49）：对象且非数组。
fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// `readNonEmptyString`（真源 :51-58）：字符串且 trim 后非空（返回**原值**不 trim）。
fn read_non_empty_string(value: Option<&Value>) -> Option<String> {
    value?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

/// `readFirstStringField`（真源 :60-72）：按候选键顺序取第一个非空字符串。
fn read_first_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(candidate) = read_non_empty_string(value.get(key)) {
            return Some(candidate);
        }
    }
    None
}

/// `readRawInput`（真源 :74-80）：`raw.rawInput ?? raw.input`。
fn read_raw_input(raw: &Value) -> Option<&Value> {
    raw.get("rawInput").or_else(|| raw.get("input"))
}

/// `readRawOutput`（真源 :82-88）：`raw.rawOutput ?? raw.output ?? raw.result`。
fn read_raw_output(raw: &Value) -> Option<&Value> {
    raw.get("rawOutput")
        .or_else(|| raw.get("output"))
        .or_else(|| raw.get("result"))
}

/// `resolveOperation`（真源 :90-104）。
///
/// ★真实 MCP 工具会带 `mcp__node_repl__` 前缀；仅匹配旧 built-in 名称
/// 会把 reset/configure 错误展示成执行 JavaScript。
fn resolve_operation(tool_call: &LegacyToolCall) -> NodeReplOperation {
    let tool_name = tool_call
        .tool_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&tool_call.kind)
        .to_lowercase();
    if tool_name == "js_reset" || tool_name == "mcp__node_repl__js_reset" {
        return NodeReplOperation::Reset;
    }
    if tool_name == "js_add_node_module_dir"
        || tool_name == "mcp__node_repl__js_add_node_module_dir"
    {
        return NodeReplOperation::AddModuleDir;
    }
    NodeReplOperation::Run
}

/// `parseInputRecord`（真源 :106-120）：对象直取；字符串按 JSON 解析（解析结果须是对象）。
fn parse_input_record(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    if is_record(value) {
        return Some(value.clone());
    }
    let text = value.as_str()?;
    let parsed: Value = serde_json::from_str(text).ok()?;
    is_record(&parsed).then_some(parsed)
}

/// `readInputRecords`（真源 :122-132）。
///
/// 真源 `!records.includes(record)` 是**引用**去重；Rust 值语义下用结构相等
/// （见文件头偏离注：候选按序读首个命中，结果一致）。
fn read_input_records(tool_call: &LegacyToolCall) -> Vec<Value> {
    let mut records: Vec<Value> = Vec::new();
    let raw = parse_input_record(read_raw_input(&tool_call.raw));
    for candidate in [
        parse_input_record(tool_call.input.as_ref()),
        raw,
    ]
    .into_iter()
    .flatten()
    {
        if !records.contains(&candidate) {
            records.push(candidate);
        }
    }
    records
}

/// `readUserTitle`（真源 :134-147）。
///
/// 完成态快照可能只在 raw input 或顶层 title 保留用户标题，不能因主 input 只有 code 就丢失。
fn read_user_title(tool_call: &LegacyToolCall, inputs: &[Value]) -> Option<String> {
    let mut candidates: Vec<Option<String>> = inputs
        .iter()
        .map(|input| read_non_empty_string(input.get("title")))
        .collect();
    candidates.push(tool_call.title.clone());
    for candidate in candidates.into_iter().flatten() {
        let title = candidate.trim();
        if !title.is_empty() && !implementation_title_re().is_match(title) {
            return Some(title.to_string());
        }
    }
    None
}

/// `readInputString`（真源 :149-160）：逐份 input 按候选键取第一个非空字符串。
fn read_input_string(inputs: &[Value], keys: &[&str]) -> Option<String> {
    for input in inputs {
        if let Some(value) = read_first_string_field(input, keys) {
            return Some(value);
        }
    }
    None
}

/// `extractText`（真源 :162-208）：从任意载荷里抽人类可读文本。
fn extract_text(value: &Value, depth: usize) -> Option<String> {
    if depth > 4 {
        return None;
    }

    if let Some(direct) = read_non_empty_string(Some(value)) {
        return Some(direct);
    }

    if let Some(n) = value.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = value.as_u64() {
        return Some(n.to_string());
    }
    if let Some(f) = value.as_f64() {
        return Some(format!("{f}"));
    }
    if let Some(b) = value.as_bool() {
        return Some(b.to_string());
    }

    if let Some(items) = value.as_array() {
        let parts: Vec<String> = items
            .iter()
            .filter_map(|item| extract_text(item, depth + 1))
            .collect();
        return (!parts.is_empty()).then(|| parts.join("\n"));
    }

    if !is_record(value) {
        return None;
    }

    if value.get("type").and_then(|t| t.as_str()) == Some("text") {
        if let Some(text_value) = read_first_string_field(value, &["value", "text", "content"]) {
            return Some(text_value);
        }
    }

    let logs = read_non_empty_string(value.get("logs"));
    let result = extract_text(value.get("result").unwrap_or(&Value::Null), depth + 1);
    if logs.is_some() || result.is_some() {
        let mut parts: Vec<String> = Vec::new();
        if let Some(logs) = logs {
            parts.push(logs);
        }
        if let Some(result) = result {
            parts.push(result);
        }
        return Some(parts.join("\n"));
    }

    for key in ["value", "output", "text", "content", "stdout", "message"] {
        if let Some(text) = extract_text(value.get(key).unwrap_or(&Value::Null), depth + 1) {
            return Some(text);
        }
    }

    None
}

/// `removeProjectedCompletionMarkers`（真源 :210-217）。
///
/// 结果投影会用“=> ”区分完成值与日志，但这是内部协议标记，不应展示给用户。
/// 真源 `(^|\n)=> ` → `$1` 等价于：每一行剥掉行首的 "=> " 前缀。
fn remove_projected_completion_markers(text: Option<String>) -> Option<String> {
    let text = text?;
    Some(
        text.split('\n')
            .map(|line| line.strip_prefix("=> ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// `removeProjectedImagePlaceholders`（真源 :219-234）。
///
/// Agent/Provider 需要图片的文本占位，但工具卡片已持有真实 display 图片；
/// 若继续渲染投影文本，用户会同时看到图片和“Attached MCP image”内部协议描述。
fn remove_projected_image_placeholders(
    text: Option<String>,
    has_images: bool,
) -> Option<String> {
    let text = text?;
    if !has_images {
        return Some(text);
    }

    let visible_lines: Vec<&str> = text
        .split('\n')
        .filter(|line| !projected_image_placeholder_re().is_match(line.trim()))
        .collect();
    let without_placeholder = visible_lines.join("\n").trim().to_string();
    (without_placeholder != "(no output)" && !without_placeholder.is_empty())
        .then_some(without_placeholder)
}

/// `removeLeadingBlankLines`（真源 :236-243）。
///
/// 模型生成的执行内容经常在首个有效行前带换行；只移除空白行，避免破坏代码缩进。
/// 手写等价于 `^(?:[ \t]*\r?\n)+`：反复剥掉「只有空白 + 换行」的首行；
/// 没有换行的前导空白**不算空行**（真源模式要求 `\r?\n` 收尾）。
fn remove_leading_blank_lines(code: Option<String>) -> Option<String> {
    let code = code?;
    let mut rest: &str = &code;
    loop {
        let after_spaces = rest.trim_start_matches([' ', '\t']);
        if after_spaces.starts_with("\r\n") {
            rest = &after_spaces[2..];
        } else if after_spaces.starts_with('\n') {
            rest = &after_spaces[1..];
        } else {
            break;
        }
    }
    Some(rest.to_string())
}

/// `extractError`（真源 :245-281）。
fn extract_error(value: &Value, depth: usize) -> Option<NodeReplDisplayError> {
    if depth > 4 {
        return None;
    }

    if let Some(text) = value.as_str() {
        let summary = text.trim();
        return (!summary.is_empty()).then(|| NodeReplDisplayError {
            summary: summary.to_string(),
            stack: None,
        });
    }

    if !is_record(value) {
        return None;
    }

    if let Some(nested) = value.get("error") {
        if let Some(nested) = extract_error(nested, depth + 1) {
            return Some(nested);
        }
    }

    let message = read_first_string_field(value, &["message", "errorText"]);
    let name = read_non_empty_string(value.get("name")).map(|n| n.trim().to_string());
    let stack = read_non_empty_string(value.get("stack"));
    if let Some(message) = message {
        let normalized_message = message.trim().to_string();
        let summary = match &name {
            Some(name) if !normalized_message.starts_with(&format!("{name}:")) => {
                format!("{name}: {normalized_message}")
            }
            _ => normalized_message,
        };
        return Some(NodeReplDisplayError { summary, stack });
    }

    None
}

/// `extractImages`（真源 :283-319）。
///
/// ★旧 built-in result 使用 `{images:[{base64,mimeType}]}`，真实 MCP
/// 使用 content 里的 `{type:"image",data,mimeType}`。专用 renderer 必须兼容两种历史形态。
fn extract_images(values: &[Value]) -> Vec<NodeReplDisplayImage> {
    let mut images: Vec<NodeReplDisplayImage> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn add_image(
        value: &Value,
        seen: &mut std::collections::HashSet<String>,
        images: &mut Vec<NodeReplDisplayImage>,
    ) {
        // base64: 旧形态 base64 / 新形态 data；mimeType 任一缺席或非图片即弃。
        let base64 = read_non_empty_string(value.get("base64"))
            .or_else(|| read_non_empty_string(value.get("data")));
        let mime_type = read_non_empty_string(value.get("mimeType")).map(|m| m.trim().to_string());
        let (Some(base64), Some(mime_type)) = (base64, mime_type) else {
            return;
        };
        if !image_mime_type_re().is_match(&mime_type) {
            return;
        }
        // 真源 key：`{mimeType}:{base64.length}:{base64.slice(0,24)}`——
        // length 是 UTF-16 长度，Rust 侧按字节长度同语义去重（同一编码下两者恒一致比例，碰撞面相同）。
        let key = format!("{}:{}:{}", mime_type, base64.len(), &base64[..base64.len().min(24)]);
        if !seen.insert(key) {
            return;
        }
        images.push(NodeReplDisplayImage { base64, mime_type });
    }

    fn visit(value: &Value, seen: &mut std::collections::HashSet<String>, images: &mut Vec<NodeReplDisplayImage>) {
        match value {
            Value::Array(items) => {
                for item in items {
                    visit(item, seen, images);
                }
            }
            Value::Object(_) => {
                let is_image = value.get("type").and_then(|t| t.as_str()) == Some("image")
                    || value.get("base64").is_some();
                if is_image {
                    add_image(value, seen, images);
                }
                for child in value.as_object().unwrap().values() {
                    visit(child, seen, images);
                }
            }
            _ => {}
        }
    }

    for value in values {
        visit(value, &mut seen, &mut images);
    }

    images
}

/// 从 raw 里找 node_repl display 携带的 App 身份（真源 `findCuaApp`，:321-351）。
///
/// 与 `extractImages` / `hasBrowserTurnEndDisplay` 同款递归：实时 tool.updated 把 display 放在
/// raw.result 内，终态 snapshot 则把 completed part 的 metadata 直接当作 raw，只扫一个固定位置
/// 会让对话结束后图标消失。
fn find_cua_app(value: &Value) -> Option<NodeReplCuaApp> {
    fn visit(value: &Value) -> Option<NodeReplCuaApp> {
        match value {
            Value::Array(items) => items.iter().find_map(visit),
            Value::Object(_) => {
                if value.get("kind").and_then(|k| k.as_str()) == Some("node_repl_images") {
                    if let Some(app) = value.get("app").filter(|a| a.is_object()) {
                        let app_key = read_non_empty_string(app.get("appKey"))
                            .map(|k| k.trim().to_string());
                        if let Some(app_key) = app_key {
                            let display_name = read_non_empty_string(app.get("displayName"))
                                .map(|d| d.trim().to_string());
                            return Some(NodeReplCuaApp {
                                app_key,
                                display_name,
                            });
                        }
                    }
                }
                value
                    .as_object()
                    .unwrap()
                    .values()
                    .find_map(visit)
            }
            _ => None,
        }
    }
    visit(value)
}

/// `hasBrowserTurnEndDisplay`（真源 :353-362）。
fn has_browser_turn_end_display(value: &Value) -> bool {
    fn visit(value: &Value) -> bool {
        match value {
            Value::Array(items) => items.iter().any(visit),
            Value::Object(_) => {
                if value.get("kind").and_then(|k| k.as_str()) == Some("node_repl_images")
                    && value.get("source").and_then(|s| s.as_str()) == Some("browser_turn_end")
                {
                    return true;
                }
                value.as_object().unwrap().values().any(visit)
            }
            _ => false,
        }
    }
    visit(value)
}

/// `parsePersistedResult`（真源 :364-389）：超长结果的落盘信封 → 预览 + 产物引用。
fn parse_persisted_result(text: Option<String>) -> (Option<String>, Option<NodeReplPersistedResult>) {
    let Some(text) = text else {
        return (None, None);
    };

    let Some(caps) = persisted_output_re().captures(&text) else {
        return (Some(text), None);
    };

    let size_label = caps.get(1).map(|m| m.as_str().to_string());
    let artifact_path = caps.get(2).map(|m| m.as_str().trim().to_string());
    let preview = caps.get(3).map(|m| m.as_str().to_string());
    let (Some(size_label), Some(artifact_path)) = (size_label, artifact_path) else {
        return (Some(text), None);
    };

    let result_text = preview
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().to_string());
    (
        result_text,
        Some(NodeReplPersistedResult {
            artifact_path,
            size_label: size_label.trim().to_string(),
        }),
    )
}

/// `buildNodeReplDisplayModel`（真源 :391-427）。
pub fn build_node_repl_display_model(tool_call: &LegacyToolCall) -> NodeReplDisplayModel {
    let inputs = read_input_records(tool_call);
    // 真源 :393-395 —— 输出候选：顶层 output 优先，raw 兜底；缺席的候选跳过。
    let mut output_candidates: Vec<Value> = Vec::new();
    if let Some(output) = &tool_call.output {
        output_candidates.push(Value::String(output.clone()));
    }
    if let Some(raw_output) = read_raw_output(&tool_call.raw) {
        output_candidates.push(raw_output.clone());
    }
    let projected_text = output_candidates
        .iter()
        .find_map(|candidate| extract_text(candidate, 0));
    // 实时 tool.updated 把 display 放在 raw.result 内，终态 snapshot 则把
    // completed part 的 metadata 直接作为 raw。只扫描 raw.result 会让对话结束后的图片消失。
    let mut scan_values = output_candidates.clone();
    scan_values.push(tool_call.raw.clone());
    let images = extract_images(&scan_values);
    let app = find_cua_app(&tool_call.raw);
    let (result_text, persisted_result) = parse_persisted_result(remove_projected_image_placeholders(
        remove_projected_completion_markers(projected_text),
        !images.is_empty(),
    ));

    let error = extract_error(
        &tool_call
            .error
            .as_deref()
            .map(|e| Value::String(e.to_string()))
            .unwrap_or(Value::Null),
        0,
    )
    .or_else(|| {
        (tool_call.status == "failed").then(|| {
            output_candidates
                .iter()
                .find_map(|candidate| extract_error(candidate, 0))
        })?
    });

    NodeReplDisplayModel {
        operation: Some(resolve_operation(tool_call)),
        user_title: read_user_title(tool_call, &inputs),
        code: remove_leading_blank_lines(read_input_string(&inputs, &["code"])),
        module_directory: read_input_string(&inputs, &["dir", "path"]).map(|d| d.trim().to_string()),
        result_text,
        error,
        images,
        persisted_result,
        display_source: has_browser_turn_end_display(&tool_call.raw)
            .then_some(NodeReplDisplaySource::BrowserTurnEnd),
        app,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool_call(input: Value, output: Option<&str>, raw: Value) -> LegacyToolCall {
        crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall {
            tool_id: "t1".into(),
            tool_name: Some("js".into()),
            kind: "unknown".into(),
            title: None,
            input: Some(input),
            status: "completed".into(),
            v4_status: "success".into(),
            output: output.map(str::to_string),
            content: None,
            error: None,
            raw,
            started_at: None,
            snapshot_refs: Vec::new(),
            thought: None,
        }
    }

    #[test]
    fn operation_resolves_by_prefixed_names() {
        // 真源 :90-104 —— 带前缀的 MCP 名与旧 built-in 名都能识别。
        let mut tc = tool_call(json!({ "code": "1" }), None, json!({}));
        assert_eq!(resolve_operation(&tc), NodeReplOperation::Run);
        tc.tool_name = Some("mcp__node_repl__js_reset".into());
        assert_eq!(resolve_operation(&tc), NodeReplOperation::Reset);
        tc.tool_name = Some("mcp__node_repl__js_add_node_module_dir".into());
        assert_eq!(resolve_operation(&tc), NodeReplOperation::AddModuleDir);
        tc.tool_name = Some("js_reset".into());
        assert_eq!(resolve_operation(&tc), NodeReplOperation::Reset);
        // 名字缺席退 kind。
        tc.tool_name = None;
        tc.kind = "js_add_node_module_dir".into();
        assert_eq!(resolve_operation(&tc), NodeReplOperation::AddModuleDir);
    }

    #[test]
    fn user_title_rejects_implementation_words() {
        // 真源 :134-147 —— 「js」/「node repl」类实现词不配当用户标题。
        let tc = tool_call(
            json!({ "code": "1+1", "title": "js snippet" }),
            None,
            json!({}),
        );
        let model = build_node_repl_display_model(&tc);
        assert_eq!(model.user_title, None);

        let tc = tool_call(
            json!({ "code": "1+1", "title": "计算器" }),
            None,
            json!({}),
        );
        let model = build_node_repl_display_model(&tc);
        assert_eq!(model.user_title.as_deref(), Some("计算器"));

        // 完成态快照只在 raw input 里保留用户标题。
        let mut tc = tool_call(json!({ "code": "1+1" }), None, json!({}));
        tc.raw = json!({ "rawInput": { "code": "1+1", "title": "node repl 计数器" } });
        let model = build_node_repl_display_model(&tc);
        // 「node repl 计数器」命中 node[\s_-]*repl —— 照真源被拒。
        assert_eq!(model.user_title, None);

        let mut tc = tool_call(json!({ "code": "1+1" }), None, json!({}));
        tc.raw = json!({ "rawInput": { "code": "1+1", "title": "斐波那契" } });
        tc.title = Some("js".into());
        let model = build_node_repl_display_model(&tc);
        assert_eq!(model.user_title.as_deref(), Some("斐波那契"));
    }

    #[test]
    fn extract_text_unwraps_projection_shapes() {
        // 真源 :162-208 —— {logs,result} / {type:"text"} / 数组拼行。
        let tc = tool_call(
            json!({ "code": "1" }),
            Some("=> 42"),
            json!({}),
        );
        let model = build_node_repl_display_model(&tc);
        // "=> " 是内部完成标记，剥掉。
        assert_eq!(model.result_text.as_deref(), Some("42"));

        let value = json!({ "logs": "step1\nstep2", "result": "=> done" });
        // extract_text 原样拼行；"=> " 的剥除在 removeProjectedCompletionMarkers（组装层）。
        assert_eq!(
            extract_text(&value, 0).as_deref(),
            Some("step1\nstep2\n=> done")
        );

        let value = json!([{ "type": "text", "value": "第一段" }, "=> 第二段"]);
        assert_eq!(
            extract_text(&value, 0).as_deref(),
            Some("第一段\n=> 第二段")
        );
    }

    #[test]
    fn image_placeholders_removed_only_with_images() {
        // 真源 :219-234 —— 有真实图片才清占位；清空后整个文本消失。
        let text = "[Attached image/1]\n结果如下".to_string();
        assert_eq!(
            remove_projected_image_placeholders(Some(text.clone()), false).as_deref(),
            Some(text.as_str())
        );
        assert_eq!(
            remove_projected_image_placeholders(Some(text), true).as_deref(),
            Some("结果如下")
        );
        assert_eq!(
            remove_projected_image_placeholders(Some("[Attached image/1]".to_string()), true),
            None
        );
        assert_eq!(
            remove_projected_image_placeholders(Some("(no output)".to_string()), true),
            None
        );
    }

    #[test]
    fn images_accept_both_legacy_and_mcp_shapes() {
        // 真源 :283-319 —— 旧 {base64,mimeType} 与新 {type:"image",data,mimeType}。
        let output = json!({
            "images": [{ "base64": "AAAA", "mimeType": "image/png" }],
            "content": [{ "type": "image", "data": "BBBB", "mimeType": "image/jpeg" }]
        });
        let images = extract_images(&[output]);
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].base64, "AAAA");
        assert_eq!(images[1].base64, "BBBB");

        // 非图片 MIME 与重复项被拒。
        let output = json!({
            "a": { "base64": "AAAA", "mimeType": "text/plain" },
            "b": { "base64": "CCCC", "mimeType": "image/png" },
            "c": { "base64": "CCCC", "mimeType": "image/png" }
        });
        let images = extract_images(&[output]);
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn cua_app_found_anywhere_in_raw() {
        // 真源 :321-351 —— 实时形态（raw.result 内）与终态形态（raw 顶上）都要扫到。
        let mut tc = tool_call(json!({}), None, json!({
            "result": { "kind": "node_repl_images", "app": { "appKey": "com.apple.Safari" } }
        }));
        let model = build_node_repl_display_model(&tc);
        assert_eq!(model.app.as_ref().unwrap().app_key, "com.apple.Safari");

        tc.raw = json!({ "kind": "node_repl_images", "app": { "appKey": "Finder", "displayName": "访达" } });
        let model = build_node_repl_display_model(&tc);
        let app = model.app.unwrap();
        assert_eq!(app.app_key, "Finder");
        assert_eq!(app.display_name.as_deref(), Some("访达"));

        // 空 appKey 不算找到。
        tc.raw = json!({ "kind": "node_repl_images", "app": { "appKey": "  " } });
        assert_eq!(build_node_repl_display_model(&tc).app, None);
    }

    #[test]
    fn browser_turn_end_display_switches_card_body() {
        // 真源 :353-362 / node-repl.tsx :373-375 —— browser_turn_end 的卡只剩图片网格。
        let tc = tool_call(json!({}), None, json!({
            "result": { "kind": "node_repl_images", "source": "browser_turn_end" }
        }));
        let model = build_node_repl_display_model(&tc);
        assert_eq!(
            model.display_source,
            Some(NodeReplDisplaySource::BrowserTurnEnd)
        );

        let tc = tool_call(json!({}), None, json!({
            "result": { "kind": "node_repl_images", "source": "tool_result" }
        }));
        assert_eq!(build_node_repl_display_model(&tc).display_source, None);
    }

    #[test]
    fn persisted_output_envelope_is_parsed() {
        // 真源 :364-389 —— <persisted-output> 信封拆出预览与产物引用。
        let text = "<persisted-output>\nOutput too large (12.3 KB). Full output saved to: /tmp/a.txt\n\nPreview (first 1 KB):\nline1\nline2\n</persisted-output>\n".to_string();
        let (result_text, persisted) = parse_persisted_result(Some(text));
        assert_eq!(result_text.as_deref(), Some("line1\nline2"));
        let persisted = persisted.unwrap();
        assert_eq!(persisted.artifact_path, "/tmp/a.txt");
        assert_eq!(persisted.size_label, "12.3 KB");

        // 普通文本原样返回。
        let (result_text, persisted) = parse_persisted_result(Some("plain".to_string()));
        assert_eq!(result_text.as_deref(), Some("plain"));
        assert!(persisted.is_none());
    }

    #[test]
    fn error_summary_prefixes_name_and_keeps_stack() {
        // 真源 :245-281 —— name 前缀补齐（message 未带 name: 时）。
        let value = json!({ "name": "TypeError", "message": "x is not defined", "stack": "at f()" });
        let err = extract_error(&value, 0).unwrap();
        assert_eq!(err.summary, "TypeError: x is not defined");
        assert_eq!(err.stack.as_deref(), Some("at f()"));

        // message 已带 name: 前缀则不重复。
        let value = json!({ "name": "TypeError", "message": "TypeError: dup" });
        let err = extract_error(&value, 0).unwrap();
        assert_eq!(err.summary, "TypeError: dup");

        // 嵌套 error 键优先。
        let value = json!({ "message": "outer", "error": { "message": "inner" } });
        let err = extract_error(&value, 0).unwrap();
        assert_eq!(err.summary, "inner");

        // failed 状态才从 output 候选里抽错误。
        let mut tc = tool_call(json!({}), None, json!({ "rawOutput": { "message": "boom" } }));
        tc.status = "failed".into();
        let model = build_node_repl_display_model(&tc);
        assert_eq!(model.error.unwrap().summary, "boom");

        tc.status = "completed".into();
        assert_eq!(build_node_repl_display_model(&tc).error, None);
    }

    #[test]
    fn leading_blank_lines_and_module_dir() {
        // 真源 :236-243 —— 只剥空白行；moduleDirectory 取 dir/path 并 trim。
        assert_eq!(
            remove_leading_blank_lines(Some("\n\n  \ncode here".to_string())).as_deref(),
            Some("code here")
        );
        assert_eq!(
            remove_leading_blank_lines(Some("  indented".to_string())).as_deref(),
            Some("  indented")
        );

        let tc = tool_call(
            json!({ "dir": " /tmp/modules " }),
            None,
            json!({}),
        );
        let model = build_node_repl_display_model(&tc);
        assert_eq!(model.module_directory.as_deref(), Some("/tmp/modules"));
    }

    #[test]
    fn completion_markers_stripped_line_wise() {
        // 真源 :210-217 —— (^|\n)=> 逐行剥前缀；行中的 => 保留。
        let text = "=> a\nb => c\n=> d".to_string();
        assert_eq!(
            remove_projected_completion_markers(Some(text)).as_deref(),
            Some("a\nb => c\nd")
        );
        assert_eq!(remove_projected_completion_markers(None), None);
    }
}
