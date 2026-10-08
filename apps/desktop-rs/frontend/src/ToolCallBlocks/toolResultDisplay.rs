//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/toolResultDisplay.ts`（320 行）。
//!
//! 工具结果的**装饰载荷**解析器：从 raw 的五个候选位置（result.display /
//! raw.display / metadata.display / rawOutput.display / output.display）
//! 找出第一个合法 display。真源用 shared 侧 strict schema——解析失败只丢
//! 这张卡的装饰载荷，不拒整条 row。
//!
//! ## 裁剪注明
//!
//! - 六个工作流 display schema（get_workflow_run / list_workflow_runs /
//!   eval_workflow_snippet / saved_workflow_list / list_models /
//!   resume_workflow_run）在 Rust 侧暂**宽松解析**——只认 kind 并保留原始
//!   JSON（`WorkflowDisplay { kind, value }`），待工作流迁移时补 strict 字段表
//!   （真源注释：手写第二套结构校验是漂移温床，届时直接从 shared schema 生成）。

use serde_json::{Value, json};

/// 本地 agent 消息结果（真源 :16-21）。
#[derive(Debug, Clone, PartialEq)]
pub struct LocalAgentMessageDisplay {
    pub status: String,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// TaskStop 结果（真源 :23-30）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskStopDisplay {
    pub task_id: String,
    pub task_type: String,
    pub command: Option<String>,
    pub message: String,
    pub truncated: Option<bool>,
}

/// TaskOutput 结果（真源 :32-38）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutputDisplay {
    pub retrieval_status: String,
    pub task_status: Option<String>,
    pub output: Option<String>,
    pub truncated: Option<bool>,
}

/// RespondToCoordinator 结果（真源 :40-43）。
#[derive(Debug, Clone, PartialEq)]
pub struct RespondToCoordinatorDisplay {
    pub status: String,
}

/// CUA 结果（真源 :45-65）的宽松形态：校验主字段，media/targetApp 保留原始 JSON。
#[derive(Debug, Clone, PartialEq)]
pub struct CuaDisplay {
    pub tool_name: String,
    pub status: String,
    pub structured_content: Option<String>,
    pub text: Option<String>,
    pub error_code: Option<String>,
    pub suggested_action: Option<String>,
    pub media: Value,
    pub truncated: Option<bool>,
    pub target_app: Value,
}

/// 工作流 display（六个 kind 的宽松形态，见模块头裁剪注明）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowDisplay {
    pub kind: String,
    pub value: Value,
}

/// 解析结果（真源 `ToolResultDisplay`，:67-78）。
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResultDisplay {
    LocalAgentMessage(LocalAgentMessageDisplay),
    TaskStop(TaskStopDisplay),
    TaskOutput(TaskOutputDisplay),
    RespondToCoordinator(RespondToCoordinatorDisplay),
    Cua(CuaDisplay),
    Workflow(WorkflowDisplay),
}

fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// `readOptionalString`（真源 :112-121）：
/// 三态——None=字段缺失，Some(None)=存在但非法/空（null 哨兵），Some(Some(s))=合法。
fn read_optional_string(value: &Value, key: &str) -> Option<Option<String>> {
    let candidate = value.get(key)?;
    if candidate.is_null() {
        // TS: candidate === undefined 才算缺失；null 会走 typeof !== "string" → null 哨兵。
        return Some(None);
    }
    let Some(s) = candidate.as_str() else {
        return Some(None);
    };
    let normalized = s.trim();
    if normalized.is_empty() {
        Some(None)
    } else {
        // 真源返回未 trim 的原始值（:120）。
        Some(Some(s.to_string()))
    }
}

/// 六个工作流 kind（真源 `WORKFLOW_DISPLAY_PARSERS_BY_KIND` 的键，:92-101）。
const WORKFLOW_DISPLAY_KINDS: [&str; 6] = [
    "get_workflow_run",
    "list_workflow_runs",
    "eval_workflow_snippet",
    "saved_workflow_list",
    "list_models",
    "resume_workflow_run",
];

/// `parseDisplay`（真源 :123-267）。
fn parse_display(value: &Value) -> Option<ToolResultDisplay> {
    if !is_record(value) {
        return None;
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("local_agent_message") {
        let status = value.get("status").and_then(|s| s.as_str());
        if status != Some("success") && status != Some("failed") {
            return None;
        }
        // error/message：null 哨兵（存在但非法）拒；缺失合法（真源 :128-130）。
        let error = match read_optional_string(value, "error") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        let message = match read_optional_string(value, "message") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        return Some(ToolResultDisplay::LocalAgentMessage(
            LocalAgentMessageDisplay {
                status: status.unwrap().to_string(),
                error,
                message,
            },
        ));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("task_stop") {
        let task_id = read_optional_string(value, "taskId");
        let task_type = read_optional_string(value, "taskType");
        let command = read_optional_string(value, "command");
        let message = read_optional_string(value, "message");
        let truncated = value.get("truncated");
        // taskId/taskType/message 必填且必须合法（Some(None) 是 null 哨兵）。
        let (Some(Some(task_id)), Some(Some(task_type)), Some(Some(message))) =
            (task_id, task_type, message)
        else {
            return None;
        };
        // command：缺失合法、null 哨兵拒（真源 :148 `command === null`）。
        let command = match command {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        if let Some(t) = truncated {
            if !t.is_null() && !t.is_boolean() {
                return None;
            }
        }
        return Some(ToolResultDisplay::TaskStop(TaskStopDisplay {
            task_id,
            task_type,
            command,
            message,
            truncated: truncated.and_then(|t| t.as_bool()),
        }));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("task_output") {
        let retrieval = value.get("retrievalStatus").and_then(|s| s.as_str());
        if !matches!(
            retrieval,
            Some("success") | Some("not_ready") | Some("timeout")
        ) {
            return None;
        }
        // null 哨兵拒；缺失合法（真源 :172-173）。
        let task_status = match read_optional_string(value, "taskStatus") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        let output = match read_optional_string(value, "output") {
            None => None,
            Some(None) => return None,
            Some(Some(s)) => Some(s),
        };
        let truncated = value.get("truncated");
        // 长度守卫（真源 :177-179）：taskStatus ≤ 64、output ≤ 2000。
        if task_status.as_ref().is_some_and(|s| s.chars().count() > 64) {
            return None;
        }
        if output.as_ref().is_some_and(|s| s.chars().count() > 2_000) {
            return None;
        }
        // truncated 只允许 true 或缺省（真源 :180）。
        if let Some(t) = truncated {
            if !t.is_null() && t.as_bool() != Some(true) {
                return None;
            }
        }
        return Some(ToolResultDisplay::TaskOutput(TaskOutputDisplay {
            retrieval_status: retrieval.unwrap().to_string(),
            task_status,
            output,
            truncated: truncated.and_then(|t| t.as_bool()),
        }));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("respond_to_coordinator") {
        let status = value.get("status").and_then(|s| s.as_str());
        if status != Some("success") && status != Some("failed") {
            return None;
        }
        return Some(ToolResultDisplay::RespondToCoordinator(
            RespondToCoordinatorDisplay {
                status: status.unwrap().to_string(),
            },
        ));
    }

    if value.get("kind").and_then(|k| k.as_str()) == Some("cua") {
        // 必填字段：缺失或 null 哨兵都拒（TS `!toolName` 语义）。
        let tool_name = match read_optional_string(value, "toolName") {
            Some(Some(s)) => s,
            _ => return None,
        };
        // 可缺省字段：缺失合法（缺省），null 哨兵拒（真源 :205/:233-236——
        // `legacyInput === null` 拒的是「存在但非法」，undefined 合法）。
        let optional_or_reject = |key: &str| -> Option<Option<String>> {
            match read_optional_string(value, key) {
                None => Some(None),
                Some(None) => None, // null 哨兵 → 整体拒
                Some(Some(s)) => Some(Some(s)),
            }
        };
        let Some(structured_content) = optional_or_reject("structuredContent") else {
            return None;
        };
        let Some(text) = optional_or_reject("text") else {
            return None;
        };
        let Some(error_code) = optional_or_reject("errorCode") else {
            return None;
        };
        let Some(suggested_action) = optional_or_reject("suggestedAction") else {
            return None;
        };
        // input 同可选语义（仅存在性校验，值不进模型）。
        if optional_or_reject("input").is_none() {
            return None;
        }
        let target_app = value.get("targetApp").cloned().unwrap_or(Value::Null);
        let media = value.get("media").cloned().unwrap_or(Value::Null);
        let status = value.get("status").and_then(|s| s.as_str());
        let schema_ok = value.get("schemaVersion").and_then(|v| v.as_i64()) == Some(1);
        if !schema_ok || !matches!(status, Some("success") | Some("failed")) {
            return None;
        }
        let truncated = value.get("truncated");
        if let Some(t) = truncated {
            if !t.is_null() && !t.is_boolean() {
                return None;
            }
        }
        return Some(ToolResultDisplay::Cua(CuaDisplay {
            tool_name,
            status: status.unwrap().to_string(),
            structured_content,
            text,
            error_code,
            suggested_action,
            media,
            truncated: truncated.and_then(|t| t.as_bool()),
            target_app,
        }));
    }

    // 工作流 kind（宽松形态，见模块头裁剪注明）。
    if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
        if WORKFLOW_DISPLAY_KINDS.contains(&kind) {
            return Some(ToolResultDisplay::Workflow(WorkflowDisplay {
                kind: kind.to_string(),
                value: value.clone(),
            }));
        }
    }

    None
}

/// `readToolResultDisplay`（真源 :299-320）：五候选位置按序取第一个合法者。
pub fn read_tool_result_display(raw: &Value) -> Option<ToolResultDisplay> {
    if !is_record(raw) {
        return None;
    }

    let result = raw.get("result").filter(|v| is_record(v));
    let metadata = raw.get("metadata").filter(|v| is_record(v));
    let raw_output = raw.get("rawOutput").filter(|v| is_record(v));
    let output = raw.get("output").filter(|v| is_record(v));
    let candidates = [
        result.and_then(|r| r.get("display")),
        raw.get("display"),
        metadata.and_then(|m| m.get("display")),
        raw_output.and_then(|r| r.get("display")),
        output.and_then(|o| o.get("display")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Some(display) = parse_display(candidate) {
            return Some(display);
        }
    }
    None
}

/// 便捷：把 display 以 JSON 形态取出（消费方按 kind 取字段时用）。
pub fn display_as_value(display: &ToolResultDisplay) -> Value {
    match display {
        ToolResultDisplay::TaskOutput(d) => json!({
            "kind": "task_output",
            "retrievalStatus": d.retrieval_status,
            "taskStatus": d.task_status,
            "output": d.output,
            "truncated": d.truncated,
        }),
        ToolResultDisplay::TaskStop(d) => json!({
            "kind": "task_stop",
            "taskId": d.task_id,
            "taskType": d.task_type,
            "command": d.command,
            "message": d.message,
            "truncated": d.truncated,
        }),
        ToolResultDisplay::LocalAgentMessage(d) => json!({
            "kind": "local_agent_message",
            "status": d.status,
            "error": d.error,
            "message": d.message,
        }),
        ToolResultDisplay::RespondToCoordinator(d) => json!({
            "kind": "respond_to_coordinator",
            "status": d.status,
        }),
        ToolResultDisplay::Cua(d) => json!({
            "kind": "cua",
            "toolName": d.tool_name,
            "status": d.status,
        }),
        ToolResultDisplay::Workflow(d) => d.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_from_five_candidate_positions_in_order() {
        // 真源 :306-312 —— result.display 优先。
        let raw = json!({
            "result": { "display": { "kind": "respond_to_coordinator", "status": "success" } },
            "display": { "kind": "respond_to_coordinator", "status": "failed" },
        });
        assert_eq!(
            read_tool_result_display(&raw),
            Some(ToolResultDisplay::RespondToCoordinator(
                RespondToCoordinatorDisplay {
                    status: "success".into()
                }
            ))
        );
        // 无 result 时取 raw.display。
        let raw = json!({
            "display": { "kind": "respond_to_coordinator", "status": "failed" },
        });
        assert_eq!(
            read_tool_result_display(&raw),
            Some(ToolResultDisplay::RespondToCoordinator(
                RespondToCoordinatorDisplay {
                    status: "failed".into()
                }
            ))
        );
        // metadata.display 第三候选。
        let raw = json!({
            "metadata": { "display": { "kind": "respond_to_coordinator", "status": "success" } },
        });
        assert!(read_tool_result_display(&raw).is_some());
    }

    #[test]
    fn invalid_first_candidate_falls_through() {
        // 真源 :314-317 —— 非法候选跳过继续找。
        let raw = json!({
            "result": { "display": { "kind": "task_output", "retrievalStatus": "bogus" } },
            "rawOutput": { "display": { "kind": "task_output", "retrievalStatus": "success" } },
        });
        match read_tool_result_display(&raw) {
            Some(ToolResultDisplay::TaskOutput(d)) => assert_eq!(d.retrieval_status, "success"),
            other => panic!("期望 TaskOutput，得 {other:?}"),
        }
    }

    #[test]
    fn task_stop_requires_core_fields() {
        // 真源 :139-161 —— taskId/taskType/message 必填。
        let ok = json!({
            "kind": "task_stop", "taskId": "t1", "taskType": "agent", "message": "已停止",
        });
        match parse_display(&ok) {
            Some(ToolResultDisplay::TaskStop(d)) => {
                assert_eq!(d.task_id, "t1");
                assert_eq!(d.command, None);
            }
            other => panic!("期望 TaskStop，得 {other:?}"),
        }
        // 缺 message → 拒。
        let bad = json!({ "kind": "task_stop", "taskId": "t1", "taskType": "agent" });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn task_output_length_guards() {
        // 真源 :177-179 —— output 超 2000 字符拒。
        let long_output = "x".repeat(2_001);
        let bad = json!({
            "kind": "task_output", "retrievalStatus": "success", "output": long_output,
        });
        assert_eq!(parse_display(&bad), None);
        // taskStatus 超 64 拒。
        let long_status = "x".repeat(65);
        let bad = json!({
            "kind": "task_output", "retrievalStatus": "success", "taskStatus": long_status,
        });
        assert_eq!(parse_display(&bad), None);
        // truncated 只允许 true。
        let bad = json!({
            "kind": "task_output", "retrievalStatus": "success", "truncated": false,
        });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn workflow_kinds_are_lenient_v1() {
        // 六个工作流 kind 宽松解析（裁剪注明：待 strict 字段表）。
        for kind in WORKFLOW_DISPLAY_KINDS {
            let value = json!({ "kind": kind, "extra": 1 });
            match parse_display(&value) {
                Some(ToolResultDisplay::Workflow(d)) => assert_eq!(d.kind, kind),
                other => panic!("{kind} 期望 Workflow 形态，得 {other:?}"),
            }
        }
        // 未知 kind → None。
        let unknown = json!({ "kind": "no_such_kind" });
        assert_eq!(parse_display(&unknown), None);
    }

    #[test]
    fn cua_display_core_validation() {
        let ok = json!({
            "kind": "cua", "schemaVersion": 1, "toolName": "screenshot",
            "status": "success", "text": "done",
        });
        assert!(matches!(
            parse_display(&ok),
            Some(ToolResultDisplay::Cua(_))
        ));
        // schemaVersion 必须 1。
        let bad = json!({
            "kind": "cua", "schemaVersion": 2, "toolName": "screenshot",
            "status": "success",
        });
        assert_eq!(parse_display(&bad), None);
        // toolName 非字符串 → 拒。
        let bad = json!({ "kind": "cua", "schemaVersion": 1, "toolName": 42, "status": "success" });
        assert_eq!(parse_display(&bad), None);
    }

    #[test]
    fn malformed_raw_is_safe() {
        for bad in [json!(null), json!("字符串"), json!(42), json!({})] {
            assert_eq!(read_tool_result_display(&bad), None);
        }
    }
}
