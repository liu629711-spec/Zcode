//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/agentHelpers.ts`（246 行）。
//!
//! Agent 卡的取值层：代理名（kindLabel detail）、颜色、后台进程信息、
//! 活动内容、主文本、提示词。所有读取都做「字符串 → JSON.parse」宽容降级
//! （v4 行的 output 是散文，旧快照的 output 可能是 JSON 字符串）。
//!
//! ## 裁剪注明
//!
//! - 真源 `formatAgentMessage(intl, id, fallback)` 的 i18n 回退机制
//!   （:9-14，漏配 key 时回退稳定术语）：Rust 无 i18n 框架，直接按
//!   zh-CN 文案查表，未知 id 回退 fallback——语义等价。

use serde_json::Value;

use super::super::toolCallRowAdapter::LegacyToolCall;

/// 真源 `DEFAULT_AGENT_TYPE_LABEL`（:7）。
pub const DEFAULT_AGENT_TYPE_LABEL: &str = "general-purpose";

/// `formatAgentMessage`（真源 :9-14）——中文文案表（zh-CN.ts:5639-5654）。
pub fn agent_message(id: &str, fallback: &str) -> String {
    let table = match id {
        "chat.toolCall.agent.label" => "子智能体",
        "chat.toolCall.agent.prompt" => "提示词",
        "chat.toolCall.agent.fallback" => "子智能体",
        "chat.toolCall.agent.backgroundProcess" => "后台 Agent 过程",
        "chat.toolCall.agent.backgroundLaunch" => "启动",
        "chat.toolCall.agent.backgroundLaunchFailed" => "启动失败",
        "chat.toolCall.agent.backgroundLaunching" => "启动中",
        "chat.toolCall.agent.backgroundLaunched" => "已启动",
        "chat.toolCall.agent.backgroundActivity" => "活动",
        "chat.toolCall.agent.backgroundActivityStreaming" => "后台运行中，正在同步输出",
        "chat.toolCall.agent.backgroundActivityRunningWaiting" => "后台运行中，等待输出",
        "chat.toolCall.agent.backgroundActivityReceived" => "已收到子智能体回传",
        "chat.toolCall.agent.backgroundActivityWaiting" => "等待子智能体回传",
        "chat.toolCall.agent.outputFile" => "输出文件",
        "chat.toolCall.agent.thought" => "子智能体思考",
        "chat.toolCall.agent.output" => "子智能体输出",
        "chat.toolCall.agent.openInSidePane" => "在右侧打开",
        _ => fallback,
    };
    table.to_string()
}

fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// `readStringField`（真源 :20-37）——trim 后非空，返回 **trim 后的值**。
fn read_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(candidate) = value.get(*key).and_then(|v| v.as_str()) else {
            continue;
        };
        let trimmed = candidate.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// `readTextFromUnknown`（真源 :39-62）——string（非 trim 原值）/ array 取文本
/// join("\n") / record 取 text 或递归 content。
fn read_text_from_unknown(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        if !s.trim().is_empty() {
            return Some(s.to_string());
        }
    }
    if let Some(array) = value.as_array() {
        let text = array
            .iter()
            .filter_map(read_text_from_unknown)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            return Some(text);
        }
        return None;
    }
    if !is_plain_record(value) {
        return None;
    }
    if let Some(text) = read_string_field(value, &["text"]) {
        return Some(text);
    }
    read_text_from_unknown(value.get("content").unwrap_or(&Value::Null))
}

/// `readStringFromNestedRecord`（真源 :64-74）。
fn read_string_from_nested_record(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        if !is_plain_record(current) {
            return None;
        }
        current = current.get(*key).unwrap_or(&Value::Null);
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// `parseJsonObject`（真源 :76-83）。
fn parse_json_object(value: &str) -> Option<Value> {
    serde_json::from_str::<Value>(value)
        .ok()
        .filter(|v| is_plain_record(v))
}

/// `readRecordFromUnknown`（真源 :85-92）。
fn read_record_from_unknown(value: &Value) -> Option<Value> {
    if is_plain_record(value) {
        return Some(value.clone());
    }
    let text = read_text_from_unknown(value)?;
    parse_json_object(&text)
}

/// `isImplementationToolTitle`（真源 :94-100）：title 归一后是 agent/task。
fn is_implementation_tool_title(title: &str) -> bool {
    let normalized = title.trim().to_lowercase().replace([' ', '-'], "_");
    normalized == "agent" || normalized == "task"
}

/// `readAgentNameFromRecord`（真源 :102-113）。
fn read_agent_name_from_record(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| {
        read_string_field(
            v,
            &[
                "agentType",
                "agent_type",
                "subagentType",
                "subagent_type",
                "name",
                "nickname",
            ],
        )
    })
}

/// `readAgentPrimaryDescription`（真源 :115-117）。
fn read_agent_primary_description(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| read_string_field(v, &["description", "summary", "message"]))
}

/// `getAgentKindLabel`（真源 :119-145）。
///
/// 关键注释（:132-134）：流式 input 的半截 JSON 暂时读不到 subagent_type，
/// 一读不到就回退 general-purpose 会导致首帧误报、之后跳变；只有
/// `inputPreviewComplete` 明确为 true 才确认字段省略。
pub fn get_agent_kind_label(
    tool_call: &LegacyToolCall,
    fallback_label: &str,
    authoritative_agent_type: Option<&str>,
) -> String {
    let output_record = read_record_from_unknown(
        tool_call
            .output
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .as_ref()
            .unwrap_or(&Value::Null),
    );
    let input_record = tool_call.input.as_ref().filter(|v| is_plain_record(v));
    let raw_record = (&tool_call.raw).is_object().then_some(&tool_call.raw);
    let raw_name = read_string_from_nested_record(&tool_call.raw, &["_meta", "zcode", "agentType"])
        .or_else(|| {
            read_string_from_nested_record(&tool_call.raw, &["_meta", "zcode", "agent_type"])
        })
        .or_else(|| {
            read_string_from_nested_record(&tool_call.raw, &["_meta", "zcode", "subagent_type"])
        });

    let explicit_name = authoritative_agent_type
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| read_agent_name_from_record(output_record.as_ref()))
        .or_else(|| read_agent_name_from_record(input_record))
        .or_else(|| raw_name);
    if let Some(name) = explicit_name {
        return name;
    }

    let preview_complete = raw_record
        .and_then(|r| r.get("inputPreviewComplete"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if preview_complete {
        if fallback_label.is_empty() {
            DEFAULT_AGENT_TYPE_LABEL.to_string()
        } else {
            fallback_label.to_string()
        }
    } else {
        String::new()
    }
}

/// `readAgentColorFromRecord`（真源 :159-161）。
fn read_agent_color_from_record(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| read_string_field(v, &["color", "agentColor", "agent_color"]))
}

/// `getAgentColor`（真源 :147-157）——8 色词表校验。
pub fn get_agent_color(tool_call: &LegacyToolCall) -> Option<String> {
    let output_record = read_record_from_unknown(
        tool_call
            .output
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .as_ref()
            .unwrap_or(&Value::Null),
    );
    let input_record = tool_call.input.as_ref().filter(|v| is_plain_record(v));
    let raw_color = read_string_from_nested_record(&tool_call.raw, &["_meta", "zcode", "color"])
        .or_else(|| read_string_from_nested_record(&tool_call.raw, &["color"]))
        .or_else(|| read_agent_color_from_record(input_record))
        .or_else(|| read_agent_color_from_record(output_record.as_ref()));

    raw_color.filter(|c| super::subagentColors::is_subagent_color(c))
}

/// 后台 Agent 信息（真源 `readBackgroundAgentInfo` 返回，:163-184）。
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundAgentInfo {
    pub output_file: Option<String>,
}

/// `readBackgroundAgentInfo`（真源 :163-184）。
pub fn read_background_agent_info(tool_call: &LegacyToolCall) -> Option<BackgroundAgentInfo> {
    let raw = (&tool_call.raw).is_object().then_some(&tool_call.raw);
    let meta = raw
        .and_then(|r| r.get("_meta"))
        .filter(|v| is_plain_record(v));
    let zcode = meta
        .and_then(|m| m.get("zcode"))
        .filter(|v| is_plain_record(v));
    let zcode_background_agent = zcode
        .and_then(|z| z.get("backgroundAgent"))
        .filter(|v| is_plain_record(v));
    let task_notification = zcode
        .and_then(|z| z.get("taskNotification"))
        .filter(|v| is_plain_record(v));
    let input = tool_call.input.as_ref().filter(|v| is_plain_record(v));
    let output_text = read_text_from_unknown(
        tool_call
            .output
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .as_ref()
            .unwrap_or(&Value::Null),
    );
    let output_file = task_notification
        .and_then(|t| read_string_field(t, &["outputFile", "output_file"]))
        .or_else(|| {
            zcode_background_agent
                .and_then(|b| read_string_field(b, &["outputFile", "output_file"]))
        })
        .or_else(|| {
            output_text
                .as_deref()
                .and_then(extract_output_file_from_text)
        });

    let input_bg = input
        .and_then(|i| i.get("run_in_background"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || input
            .and_then(|i| i.get("runInBackground"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    if !input_bg && output_file.is_none() {
        return None;
    }

    Some(BackgroundAgentInfo { output_file })
}

/// `outputText?.match(/output_file:\s*([^\s]+)/i)?.[1]`（真源 :177）。
fn extract_output_file_from_text(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let index = lower.find("output_file:")?;
    let rest = text[index + "output_file:".len()..].trim_start();
    let token: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
    (!token.is_empty()).then_some(token)
}

/// `getAgentActivityContent`（真源 :186-197）。
pub fn get_agent_activity_content(tool_call: &LegacyToolCall) -> Option<String> {
    let task_notification_result = read_string_from_nested_record(
        &tool_call.raw,
        &["_meta", "zcode", "taskNotification", "result"],
    )
    .or_else(|| {
        read_string_from_nested_record(
            &tool_call.raw,
            &["_meta", "zcode", "taskNotification", "summary"],
        )
    });
    if task_notification_result.is_some() {
        // 真源注释（:191-193）：background Agent 的 output_file 是完整 sidechain
        // transcript，task-notification result 才是适合阅读的摘要——优先摘要。
        return task_notification_result;
    }
    tool_call
        .content
        .as_ref()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
}

/// `getAgentPrimaryText`（真源 :199-231）。
pub fn get_agent_primary_text(tool_call: &LegacyToolCall, fallback_label: &str) -> String {
    if let Some(title) = tool_call.title.as_ref() {
        let trimmed = title.trim();
        if !trimmed.is_empty() && !is_implementation_tool_title(trimmed) {
            return trimmed.to_string();
        }
    }

    if let Some(input) = tool_call.input.as_ref().filter(|v| is_plain_record(v)) {
        if let Some(description) = read_string_field(input, &["description"]) {
            return description;
        }
        if let Some(subagent_type) = read_string_field(input, &["subagent_type"]) {
            return subagent_type;
        }
    }

    let output_record = read_record_from_unknown(
        tool_call
            .output
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .as_ref()
            .unwrap_or(&Value::Null),
    );
    if let Some(desc) = read_agent_primary_description(output_record.as_ref()) {
        return desc;
    }
    if let Some(name) = read_agent_name_from_record(output_record.as_ref()) {
        return name;
    }

    fallback_label.to_string()
}

/// `getAgentPrompt`（真源 :233-246）。
pub fn get_agent_prompt(tool_call: &LegacyToolCall) -> Option<String> {
    if let Some(input) = tool_call.input.as_ref().filter(|v| is_plain_record(v)) {
        if let Some(prompt) = read_string_field(input, &["prompt", "message", "description"]) {
            return Some(prompt);
        }
    }
    if let Some(s) = tool_call.input.as_ref().and_then(|v| v.as_str()) {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolCallRowAdapter::tool_call_row_to_legacy_node;
    use serde_json::json;

    fn node(row: serde_json::Value) -> LegacyToolCall {
        tool_call_row_to_legacy_node(&row).tool_call
    }

    /// 构造 Agent 行；`output_text` 进 `output.text`（adapter 从这读散文）。
    fn agent_row(input: Value, output_text: Option<&str>) -> LegacyToolCall {
        node(json!({
            "kind": "toolCall", "rowId": 1, "toolCallId": "tc-1",
            "toolName": "Agent", "status": "success", "inputText": "",
            "input": input,
            "output": output_text.map(|t| json!({"text": t})),
        }))
    }

    #[test]
    fn kind_label_reads_authoritative_then_input_then_output() {
        // authoritative 优先（真源 :135-139）。
        let tc = agent_row(json!({"subagent_type": "explorer"}), None);
        assert_eq!(
            get_agent_kind_label(&tc, "", Some("code-reviewer")),
            "code-reviewer"
        );
        // input 的 subagent_type。
        assert_eq!(get_agent_kind_label(&tc, "", None), "explorer");
        // output 文本里的 JSON（readRecordFromUnknown 的字符串解析路径）。
        let tc = agent_row(json!({}), Some("{\"agentType\":\"planner\"}"));
        assert_eq!(get_agent_kind_label(&tc, "", None), "planner");
    }

    #[test]
    fn kind_label_streaming_preview_returns_empty() {
        // 真源 :132-144 —— inputPreviewComplete 非 true 时不回退默认类型（防首帧误报）。
        // 半截 JSON → input 解析失败 → adapter 标 previewComplete=false。
        let tc = node(json!({
            "kind": "toolCall", "rowId": 2, "toolCallId": "tc-2",
            "toolName": "Agent", "status": "inputStreaming",
            "inputText": "{\"prompt\":\"half",
        }));
        assert_eq!(get_agent_kind_label(&tc, "", None), "");
        // inputPreviewComplete=true 且无显式名 → DEFAULT（fallbackLabel 空时）。
        let mut tc = agent_row(json!({}), None);
        tc.raw["inputPreviewComplete"] = json!(true);
        assert_eq!(
            get_agent_kind_label(&tc, "", None),
            DEFAULT_AGENT_TYPE_LABEL
        );
        assert_eq!(get_agent_kind_label(&tc, "SubAgent", None), "SubAgent");
    }

    #[test]
    fn agent_color_from_input_and_validation() {
        // input.color（真源 :159-161）。
        let tc = agent_row(json!({"color": "purple"}), None);
        assert_eq!(get_agent_color(&tc).as_deref(), Some("purple"));
        // 非法色被拒。
        let tc = agent_row(json!({"color": "neon"}), None);
        assert_eq!(get_agent_color(&tc), None);
        // raw._meta.zcode.color 优先。
        let mut tc = agent_row(json!({"color": "purple"}), None);
        tc.raw["_meta"] = json!({"zcode": {"color": "cyan"}});
        assert_eq!(get_agent_color(&tc).as_deref(), Some("cyan"));
    }

    #[test]
    fn background_info_gate_and_output_file() {
        // 未开后台且无 outputFile → null（真源 :179-181）。
        let tc = agent_row(json!({"prompt": "x"}), None);
        assert_eq!(read_background_agent_info(&tc), None);
        // run_in_background=true → 有信息。
        let tc = agent_row(json!({"run_in_background": true}), None);
        assert_eq!(
            read_background_agent_info(&tc),
            Some(BackgroundAgentInfo { output_file: None })
        );
        // output 文本里的 output_file: 提取（真源 :177）。
        let tc = agent_row(json!({}), Some("done\noutput_file: /tmp/t.jsonl"));
        assert_eq!(
            read_background_agent_info(&tc)
                .unwrap()
                .output_file
                .as_deref(),
            Some("/tmp/t.jsonl")
        );
        // taskNotification.outputFile 优先。
        let mut tc = agent_row(json!({}), None);
        tc.raw["_meta"] = json!({"zcode": {"taskNotification": {"outputFile": "/a.jsonl"}}});
        assert_eq!(
            read_background_agent_info(&tc)
                .unwrap()
                .output_file
                .as_deref(),
            Some("/a.jsonl")
        );
    }

    #[test]
    fn activity_content_prefers_task_notification() {
        // 真源 :186-197 —— result > summary > content。
        let mut tc = agent_row(json!({}), None);
        tc.content = Some("内容".into());
        assert_eq!(get_agent_activity_content(&tc).as_deref(), Some("内容"));
        tc.raw["_meta"] = json!({"zcode": {"taskNotification": {"summary": "摘要"}}});
        assert_eq!(get_agent_activity_content(&tc).as_deref(), Some("摘要"));
        tc.raw["_meta"] =
            json!({"zcode": {"taskNotification": {"result": "结果", "summary": "摘要"}}});
        assert_eq!(get_agent_activity_content(&tc).as_deref(), Some("结果"));
    }

    #[test]
    fn primary_text_ladder() {
        // title 非实现名优先。
        let mut tc = agent_row(json!({"description": "扫描风险"}), None);
        tc.title = Some("安全审计".into());
        assert_eq!(get_agent_primary_text(&tc, "子智能体"), "安全审计");
        // title 是实现名（agent/task）→ 跳过（真源 :199-205）。
        tc.title = Some("Task".into());
        assert_eq!(get_agent_primary_text(&tc, "子智能体"), "扫描风险");
        // 无 description → subagent_type。
        let tc = agent_row(json!({"subagent_type": "explorer"}), None);
        assert_eq!(get_agent_primary_text(&tc, "子智能体"), "explorer");
        // 全无 → fallback。
        let tc = agent_row(json!({}), None);
        assert_eq!(get_agent_primary_text(&tc, "子智能体"), "子智能体");
    }

    #[test]
    fn prompt_reads_input_shapes() {
        // record 的 prompt/message/description（真源 :233-239）。
        let tc = agent_row(json!({"prompt": "做这件事"}), None);
        assert_eq!(get_agent_prompt(&tc).as_deref(), Some("做这件事"));
        // 字符串 input。
        let mut tc = agent_row(json!({}), None);
        tc.input = Some(json!("直接文本"));
        assert_eq!(get_agent_prompt(&tc).as_deref(), Some("直接文本"));
        // 都没有。
        let tc = agent_row(json!({}), None);
        assert_eq!(get_agent_prompt(&tc), None);
    }

    #[test]
    fn implementation_title_normalization() {
        // 真源 :94-100 —— agent / Agent / TASK / "a-g-e-n-t"? 只归空白与连字符。
        assert!(is_implementation_tool_title("agent"));
        assert!(is_implementation_tool_title(" Agent "));
        assert!(is_implementation_tool_title("TASK"));
        assert!(!is_implementation_tool_title("Agents"));
        assert!(!is_implementation_tool_title("my task"));
    }
}
