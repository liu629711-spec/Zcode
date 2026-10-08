//! 1:1 翻译 `packages/ui/src/v4/toolCallRowAdapter.ts`（130 行）。
//!
//! v4 ToolCallRow → 旧 ToolCallBlocks 输入形态（LegacyToolCallNode）适配。
//! 纯函数：ToolCallBlock 及其 renderers（execute/read/edit/...）吃的是旧
//! ZCode Agent 的 TaskChatToolCall 形态；v4 row 自包含，字段一一映射即可。
//!
//! **裁剪注明**：真源 `resolveToolInputPreview` 在 input 缺席时用
//! `buildZCodeStreamingToolInputPreview` 解析流式半截 JSON（含部分键提取）。
//! Rust 侧 v1 先做严格 `serde_json` 解析——半截 JSON 解析失败时 input 为
//! None，流式中的 edit/write 卡片在 input 收齐前不显示文件预览。
//! 补齐半截解析后此注释即失效。

use serde_json::{Value, json};

/// 旧形态工具调用节点（真源 `TaskChatToolCallTreeNode`，toolCallTree.js）。
///
/// 字段只保留 ToolCallBlocks 链路实际消费的；v4 行没有子树，
/// `child_tool_calls` 恒空（真源 :126-128 同注释）。
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyToolCallNode {
    pub tool_call: LegacyToolCall,
    pub child_tool_calls: Vec<LegacyToolCallNode>,
}

/// 旧形态工具调用（真源 `TaskChatToolCall` 的子集）。
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyToolCall {
    pub tool_id: String,
    pub tool_name: Option<String>,
    /// kind 兼容旧聚合分类：v4 下没有旧快照形态，直接用固定工具名（真源 :96-97）。
    pub kind: String,
    pub title: Option<String>,
    pub input: Option<Value>,
    /// 旧词表：pending / in_progress / completed / failed / stopped。
    pub status: String,
    /// v4 原始状态（inputStreaming / pendingApproval / running / success / error / cancelled）。
    pub v4_status: String,
    /// `output.text`（散文结果）。
    pub output: Option<String>,
    /// Agent/Task 行的活动结果桥接（真源 :101-106）。
    pub content: Option<String>,
    /// 解析后的失败正文（真源 :107-109）。
    pub error: Option<String>,
    /// 组装的 raw 载荷（真源 :110-123）。
    pub raw: Value,
    pub started_at: Option<f64>,
    /// 快照字段引用（真源 `TaskChatToolCall.snapshotRefs`，
    /// taskChatMessageTypes.ts:35）。真源 v4 adapter 不产该字段
    /// （生产者在 legacy 服务层 zcodeTaskServiceAdapter.ts:852），
    /// 当前恒空；前向兼容读取见 ToolSnapshotFieldNotice::read_snapshot_refs。
    pub snapshot_refs: Vec<super::ToolSnapshotFieldNotice::SnapshotFieldRef>,
    /// 子代理思考输出（真源 `TaskChatToolCall.thought`，:31）。
    /// 真源 v4 adapter 不桥接（legacy 路径由服务层写入），恒 None。
    pub thought: Option<String>,
}

/// v4 status → 旧 ChatToolCall.status（真源 STATUS_MAP，:12-19）。
///
/// pendingApproval 视为 pending：审批中输入已定，展示为待执行。
pub fn map_v4_status_to_legacy(status: &str) -> &'static str {
    match status {
        "inputStreaming" | "pendingApproval" => "pending",
        "running" => "in_progress",
        "success" => "completed",
        "error" => "failed",
        "cancelled" => "stopped",
        _ => "pending",
    }
}

/// `readNonEmptyString`（真源 :56-62）。
fn read_non_empty_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// `resolveV4ToolErrorText`（真源 :64-80）：失败正文提取。
///
/// 阶梯：error.message → output.text（剥 <tool_use_error> 标签）→ error.code。
/// 仅在 v4 status == "error" 时有值。
pub fn resolve_v4_tool_error_text(
    status: &str,
    error: Option<&Value>,
    output: Option<&Value>,
) -> Option<String> {
    if status != "error" {
        return None;
    }
    let error = error.unwrap_or(&Value::Null);
    if let Some(direct) = error.get("message").and_then(read_non_empty_string) {
        return Some(direct);
    }
    if let Some(text) = output
        .and_then(|o| o.get("text"))
        .and_then(read_non_empty_string)
    {
        return Some(crate::ToolCallBlocks::toolError::normalize_wrapped_error_text(&text));
    }
    error.get("code").and_then(read_non_empty_string)
}

/// `resolveToolInputPreview`（真源 :37-54）的 v1 简化版。
///
/// input 已在 → 直接用（complete）；否则严格解析 inputText，失败给 None。
/// `streamingRawInputLength` 语义保留：inputText 非空即记录。
fn resolve_tool_input_preview(
    input: Option<&Value>,
    input_text: &str,
) -> (Option<Value>, bool, Option<usize>) {
    if let Some(parsed) = input {
        let streaming_len = (!input_text.is_empty()).then(|| input_text.chars().count());
        return (Some(parsed.clone()), true, streaming_len);
    }
    if input_text.is_empty() {
        return (None, false, None);
    }
    match serde_json::from_str::<Value>(input_text) {
        Ok(v) => {
            // 空对象视为无 input（真源 isEmptyPlainRecord → undefined，:50）。
            let is_empty_obj = v.as_object().is_some_and(|o| o.is_empty());
            let streaming_len = input_text.chars().count().into();
            (
                Some(if is_empty_obj { Value::Null } else { v }),
                true,
                streaming_len,
            )
        }
        Err(_) => {
            // 半截 JSON：v1 裁剪，见模块头注释。
            (None, false, Some(input_text.chars().count()))
        }
    }
}

/// `toolCallRowToLegacyNode`（真源 :82-130）：v4 行 → 旧节点。
///
/// `row` 是 v4 ToolCallRow 的 JSON（rowsRange 原始投影，宽容读取）。
pub fn tool_call_row_to_legacy_node(row: &Value) -> LegacyToolCallNode {
    let tool_call_id = row
        .get("toolCallId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let tool_name = row
        .get("toolName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let v4_status = row
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("inputStreaming")
        .to_string();
    let legacy_status = map_v4_status_to_legacy(&v4_status).to_string();
    let input_text = row
        .get("inputText")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let input_field = row.get("input");

    // 失败正文（真源 :84）。
    let error_text = resolve_v4_tool_error_text(&v4_status, row.get("error"), row.get("output"));

    // input 预览（真源 :85）。
    let (input, input_preview_complete, streaming_len) =
        resolve_tool_input_preview(input_field, input_text);

    // output 的散文文本。
    let output_text = row
        .get("output")
        .and_then(|o| o.get("text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // CUA 等结构化展示事实位于 output.display；顶层 display 仅是旧 Node REPL
    // 图片通道（真源 :86-91）。cua display 会重复保存 input，桥接时丢弃。
    let display = row
        .get("output")
        .and_then(|o| o.get("display"))
        .filter(|v| !v.is_null())
        .or_else(|| row.get("display").filter(|v| !v.is_null()));
    let legacy_display = display.map(|d| {
        if d.get("kind").and_then(|k| k.as_str()) == Some("cua") {
            let mut stripped = d.clone();
            if let Some(obj) = stripped.as_object_mut() {
                obj.remove("input");
            }
            stripped
        } else {
            d.clone()
        }
    });

    // Agent/Task 的活动结果桥接（真源 :101-106）。
    let is_agent_tool = matches!(tool_name.as_deref(), Some("Agent") | Some("Task"));
    let content = if is_agent_tool {
        output_text.clone()
    } else {
        None
    };

    // 组装 raw（真源 :110-123，字段名对齐旧形态的 raw 约定）。
    let mut raw = json!({
        "error": row.get("error").cloned().unwrap_or(Value::Null),
        "rawOutput": output_text.clone().map(Value::String).unwrap_or(Value::Null),
        "outputPreview": row.get("outputPreview").cloned().unwrap_or(Value::Null),
        "outputTruncated": row
            .get("output")
            .and_then(|o| o.get("truncated"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        "status": legacy_status,
        "toolCallId": tool_call_id,
        "toolName": tool_name.clone(),
        "v4Status": v4_status,
        "inputPreviewComplete": input_preview_complete,
    });
    if let Some(obj) = raw.as_object_mut() {
        if let Some(len) = streaming_len {
            obj.insert("streamingRawInputLength".into(), json!(len));
        }
        if let Some(cua_app) = row.get("cuaApp").filter(|v| !v.is_null()) {
            obj.insert("cuaApp".into(), cua_app.clone());
        }
        if let Some(d) = legacy_display {
            obj.insert("display".into(), d);
        }
    }

    let started_at = row
        .get("startedAt")
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite());

    LegacyToolCallNode {
        tool_call: LegacyToolCall {
            tool_id: tool_call_id,
            kind: tool_name.clone().unwrap_or_default(),
            tool_name,
            // 真源 :96-97 —— kind 直接用工具名。
            title: None,
            input,
            status: legacy_status,
            v4_status,
            output: output_text,
            content,
            error: error_text,
            raw,
            started_at,
            // 前向兼容：真源 v4 adapter 不传播 snapshotRefs（见字段注释）。
            snapshot_refs: super::ToolSnapshotFieldNotice::read_snapshot_refs(row),
            thought: None,
        },
        // 真源 :126-128 —— v4 行没有子树。
        child_tool_calls: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_map_matches_source_wordlist() {
        // 真源 STATUS_MAP（:12-19）。
        assert_eq!(map_v4_status_to_legacy("inputStreaming"), "pending");
        assert_eq!(map_v4_status_to_legacy("pendingApproval"), "pending");
        assert_eq!(map_v4_status_to_legacy("running"), "in_progress");
        assert_eq!(map_v4_status_to_legacy("success"), "completed");
        assert_eq!(map_v4_status_to_legacy("error"), "failed");
        assert_eq!(map_v4_status_to_legacy("cancelled"), "stopped");
        assert_eq!(map_v4_status_to_legacy("bogus"), "pending");
    }

    #[test]
    fn success_row_maps_to_completed() {
        let row = json!({
            "kind": "toolCall",
            "rowId": 1,
            "toolCallId": "tc-1",
            "toolName": "Read",
            "status": "success",
            "inputText": "{\"path\":\"a.rs\"}",
            "input": { "path": "a.rs" },
            "output": { "text": "文件内容" },
        });
        let node = tool_call_row_to_legacy_node(&row);
        let tc = &node.tool_call;
        assert_eq!(tc.tool_id, "tc-1");
        assert_eq!(tc.status, "completed");
        assert_eq!(tc.kind, "Read");
        assert_eq!(
            tc.input
                .as_ref()
                .and_then(|i| i.get("path"))
                .and_then(|p| p.as_str()),
            Some("a.rs")
        );
        assert_eq!(tc.output.as_deref(), Some("文件内容"));
        assert_eq!(tc.error, None);
        // raw 载荷字段名对齐旧形态。
        assert_eq!(tc.raw["status"], "completed");
        assert_eq!(tc.raw["v4Status"], "success");
        assert_eq!(tc.raw["rawOutput"], "文件内容");
        assert_eq!(tc.raw["inputPreviewComplete"], true);
        // Agent 桥接只对 Agent/Task 生效。
        assert_eq!(tc.content, None);
        // v4 无子树。
        assert!(node.child_tool_calls.is_empty());
    }

    #[test]
    fn error_row_resolves_message_first() {
        let row = json!({
            "kind": "toolCall",
            "rowId": 2,
            "toolCallId": "tc-2",
            "toolName": "Bash",
            "status": "error",
            "inputText": "",
            "error": { "code": "E1", "message": "命令失败" },
            "output": { "text": "<tool_use_error>标签错误</tool_use_error>" },
        });
        let node = tool_call_row_to_legacy_node(&row);
        // error.message 优先于 output 包裹文本（真源 :69-77）。
        assert_eq!(node.tool_call.error.as_deref(), Some("命令失败"));
        assert_eq!(node.tool_call.status, "failed");
    }

    #[test]
    fn error_row_falls_back_to_tagged_output_then_code() {
        // 无 message：output 的 tool_use_error 标签体。
        let row = json!({
            "kind": "toolCall", "rowId": 3, "toolCallId": "tc-3",
            "toolName": "Bash", "status": "error", "inputText": "",
            "error": { "code": "E2" },
            "output": { "text": "```text\n<tool_use_error>包裹的</tool_use_error>\n```" },
        });
        let node = tool_call_row_to_legacy_node(&row);
        assert_eq!(node.tool_call.error.as_deref(), Some("包裹的"));

        // output 也没有：error.code 兜底。
        let row2 = json!({
            "kind": "toolCall", "rowId": 4, "toolCallId": "tc-4",
            "toolName": "Bash", "status": "error", "inputText": "",
            "error": { "code": "E3" },
        });
        let node2 = tool_call_row_to_legacy_node(&row2);
        assert_eq!(node2.tool_call.error.as_deref(), Some("E3"));
    }

    #[test]
    fn non_error_status_has_no_error_text() {
        // 真源 :65-67 —— 非 error 状态直接 None。
        let row = json!({
            "kind": "toolCall", "rowId": 5, "toolCallId": "tc-5",
            "toolName": "Bash", "status": "success", "inputText": "",
            "error": { "code": "x", "message": "不该出现" },
        });
        let node = tool_call_row_to_legacy_node(&row);
        assert_eq!(node.tool_call.error, None);
    }

    #[test]
    fn agent_task_rows_bridge_content() {
        // 真源 :101-106 —— Agent/Task 的 output 桥接到 content。
        for name in ["Agent", "Task"] {
            let row = json!({
                "kind": "toolCall", "rowId": 6, "toolCallId": "tc-6",
                "toolName": name, "status": "success", "inputText": "",
                "output": { "text": "活动结果" },
            });
            let node = tool_call_row_to_legacy_node(&row);
            assert_eq!(
                node.tool_call.content.as_deref(),
                Some("活动结果"),
                "{name} 应桥接"
            );
        }
        // 其他工具不桥接。
        let row = json!({
            "kind": "toolCall", "rowId": 7, "toolCallId": "tc-7",
            "toolName": "Read", "status": "success", "inputText": "",
            "output": { "text": "结果" },
        });
        let node = tool_call_row_to_legacy_node(&row);
        assert_eq!(node.tool_call.content, None);
    }

    #[test]
    fn streaming_row_without_complete_input_is_lenient() {
        // input 缺席 + inputText 是完整 JSON → 解析成功。
        let row = json!({
            "kind": "toolCall", "rowId": 8, "toolCallId": "tc-8",
            "toolName": "Edit", "status": "running",
            "inputText": "{\"path\":\"b.rs\"}",
        });
        let node = tool_call_row_to_legacy_node(&row);
        assert_eq!(
            node.tool_call
                .input
                .as_ref()
                .and_then(|i| i.get("path"))
                .and_then(|p| p.as_str()),
            Some("b.rs")
        );
        // inputText 是半截 JSON → v1 裁剪为 None（见模块头注释）。
        let row2 = json!({
            "kind": "toolCall", "rowId": 9, "toolCallId": "tc-9",
            "toolName": "Edit", "status": "inputStreaming",
            "inputText": "{\"path\":\"c.rs",
        });
        let node2 = tool_call_row_to_legacy_node(&row2);
        assert_eq!(node2.tool_call.input, None);
        assert_eq!(node2.tool_call.status, "pending");
    }

    #[test]
    fn cua_display_strips_legacy_input() {
        // 真源 :88-91 —— cua display 丢弃重复保存的 input。
        let row = json!({
            "kind": "toolCall", "rowId": 10, "toolCallId": "tc-10",
            "toolName": "Cua", "status": "success", "inputText": "",
            "output": { "text": "ok", "display": { "kind": "cua", "input": {"x":1}, "screenshot": "data:" } },
        });
        let node = tool_call_row_to_legacy_node(&row);
        let display = node.tool_call.raw.get("display").expect("display 应进 raw");
        assert!(
            display.get("input").is_none(),
            "cua display 的 input 副本应被丢弃"
        );
        assert_eq!(display.get("kind").and_then(|k| k.as_str()), Some("cua"));
        // 非 cua display 原样保留。
        let row2 = json!({
            "kind": "toolCall", "rowId": 11, "toolCallId": "tc-11",
            "toolName": "X", "status": "success", "inputText": "",
            "output": { "text": "ok", "display": { "kind": "file_diff", "filePath": "a" } },
        });
        let node2 = tool_call_row_to_legacy_node(&row2);
        let display2 = node2.tool_call.raw.get("display").unwrap();
        assert!(display2.get("filePath").is_some());
    }

    #[test]
    fn malformed_row_does_not_panic() {
        for bad in [
            json!({}),
            json!({"kind": "toolCall"}),
            json!({"kind": "toolCall", "status": 123}),
        ] {
            let node = tool_call_row_to_legacy_node(&bad);
            assert!(node.child_tool_calls.is_empty());
        }
    }
}
