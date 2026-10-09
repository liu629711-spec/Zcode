//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/createWorkflowDisplay.ts`（71 行）。
//!
//! CreateWorkflow 工具输出侧的读取规则（display 载荷 + 纯文本兜底）。
//! 真源从 `create-workflow.tsx` 拆出（oxlint max-lines 400 门）：**纯函数、无 JSX**。

use serde_json::Value;

use super::super::toolResultDisplay::WorkflowDiagnostic;

/// `ToolCallCreateWorkflowDisplay`（真源 `create-workflow-display.ts:135-155`）。
#[derive(Debug, Clone, PartialEq)]
pub struct CreateWorkflowDisplay {
    pub ok: bool,
    /// 截断前的诊断总数（`diagnostics` 可能已被截断，故另存）。
    pub error_count: i64,
    pub diagnostics: Vec<WorkflowDiagnostic>,
    /// 因果图（真源 `toolCallCreateWorkflowCausalityGraphSchema`，本轮保留原始 JSON）。
    pub causality_graph: Option<Value>,
    pub truncated: Option<bool>,
}

/// `isPlainRecord`（真源 `createWorkflowInput.ts:12-14`）。
pub fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// zod 的 `.int().nonnegative()`。
fn nonnegative_int(value: Option<&Value>) -> Option<i64> {
    let n = value?.as_i64()?;
    (n >= 0).then_some(n)
}

/// `readWorkflowDisplay`（真源 :15-22）。
///
/// 真源注释：结构化诊断只走 display 通道；安全解析 `raw.display`，
/// 缺失或形态不符时**退回纯文本兜底，绝不 JSON dump，绝不崩溃**。
pub fn read_workflow_display(raw: &Value) -> Option<CreateWorkflowDisplay> {
    if !is_plain_record(raw) {
        return None;
    }
    parse_create_workflow_display(raw.get("display")?)
}

/// `toolCallCreateWorkflowDisplaySchema` 的 strict 解析
/// （真源 `create-workflow-display.ts:135-155`）。
pub fn parse_create_workflow_display(value: &Value) -> Option<CreateWorkflowDisplay> {
    if !is_plain_record(value) {
        return None;
    }
    // kind: z.literal("create_workflow")
    if value.get("kind").and_then(|k| k.as_str()) != Some("create_workflow") {
        return None;
    }
    let ok = value.get("ok")?.as_bool()?;
    let error_count = nonnegative_int(value.get("errorCount"))?;

    let raw_diagnostics = value.get("diagnostics")?.as_array()?;
    // z.array(...).max(100)
    if raw_diagnostics.len() > 100 {
        return None;
    }
    let mut diagnostics = Vec::with_capacity(raw_diagnostics.len());
    for item in raw_diagnostics {
        if !is_plain_record(item) {
            return None;
        }
        // inner .strict()：只认四个键
        if item.as_object().is_some_and(|o| o.len() != 4) {
            return None;
        }
        let line = nonnegative_int(item.get("line"))?;
        let column = nonnegative_int(item.get("column"))?;
        let code = nonnegative_int(item.get("code"))?;
        let message = item.get("message")?.as_str()?;
        if message.is_empty() || message.chars().count() > 2048 {
            return None;
        }
        diagnostics.push(WorkflowDiagnostic {
            line,
            column,
            code,
            message: message.to_string(),
        });
    }

    // causalityGraph: 复杂嵌套 schema，本轮保留原始 JSON（Rust 侧宽松）。
    let causality_graph = value.get("causalityGraph").cloned();
    // truncated: z.boolean().optional() —— zod optional 不接受 null
    let truncated = match value.get("truncated") {
        None => None,
        Some(v) => Some(v.as_bool()?),
    };

    Some(CreateWorkflowDisplay {
        ok,
        error_count,
        diagnostics,
        causality_graph,
        truncated,
    })
}

/// `readFallbackOutputText`（真源 :24-42）。
///
/// 字符串 → trim 后非空才返回；对象 → 四候选键 `response`/`output`/`text`/`content` 依次找。
pub fn read_fallback_output_text(output: &Value) -> Option<String> {
    if let Some(s) = output.as_str() {
        let trimmed = s.trim();
        return if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
    }
    if is_plain_record(output) {
        for key in ["response", "output", "text", "content"] {
            if let Some(candidate) = output.get(key).and_then(|v| v.as_str()) {
                let trimmed = candidate.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// `formatWorkflowFeedbackTooltip`（真源 :52-63）。
///
/// 真源注释：先是那一句话（什么没发生、谁接着动），再逐条
/// `L{line}:C{col} message` —— 与模型收到的行同形，复制出来可以直接对照。
pub fn format_workflow_feedback_tooltip(
    lede: &str,
    diagnostics: &[(i64, i64, String)],
) -> String {
    let mut parts = vec![lede.to_string()];
    parts.extend(
        diagnostics
            .iter()
            .map(|(line, column, message)| format!("L{}:C{} {}", line, column, message)),
    );
    parts.join("\n")
}

/// `workflowDiagnosticLines`（真源 :68-71）。
///
/// 被诊断点名的脚本行（首次出现序、去重、只要正行号）。
pub fn workflow_diagnostic_lines(diagnostics: &[(i64, i64, String)]) -> Vec<i64> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (line, _, _) in diagnostics {
        if *line > 0 && seen.insert(*line) {
            out.push(*line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "kind": "create_workflow",
            "ok": false,
            "errorCount": 2,
            "diagnostics": [
                { "line": 3, "column": 7, "code": 9001, "message": "Expected ';'" },
                { "line": 3, "column": 1, "code": 9002, "message": "Another" },
            ],
        })
    }

    #[test]
    fn parses_valid_display() {
        let d = parse_create_workflow_display(&valid()).unwrap();
        assert!(!d.ok);
        assert_eq!(d.error_count, 2);
        assert_eq!(d.diagnostics.len(), 2);
        assert_eq!(d.truncated, None);
        assert_eq!(d.causality_graph, None);
    }

    #[test]
    fn rejects_wrong_kind() {
        let mut v = valid();
        v["kind"] = json!("something_else");
        assert!(parse_create_workflow_display(&v).is_none());
    }

    #[test]
    fn rejects_missing_required_fields() {
        for missing in ["kind", "ok", "errorCount", "diagnostics"] {
            let mut v = valid();
            v.as_object_mut().unwrap().remove(missing);
            assert!(
                parse_create_workflow_display(&v).is_none(),
                "缺 {missing} 应非法"
            );
        }
    }

    #[test]
    fn rejects_negative_error_count() {
        let mut v = valid();
        v["errorCount"] = json!(-1);
        assert!(parse_create_workflow_display(&v).is_none());
    }

    #[test]
    fn rejects_diagnostics_over_100() {
        let mut v = valid();
        let diag = json!({ "line": 1, "column": 1, "code": 9001, "message": "x" });
        v["diagnostics"] = json!(vec![diag; 101]);
        assert!(parse_create_workflow_display(&v).is_none());
    }

    #[test]
    fn rejects_diagnostic_with_extra_key() {
        // 真源 inner schema `.strict()`。
        let mut v = valid();
        v["diagnostics"] = json!([{
            "line": 1, "column": 1, "code": 9001, "message": "x", "extra": true
        }]);
        assert!(parse_create_workflow_display(&v).is_none());
    }

    #[test]
    fn rejects_null_truncated() {
        // zod 的 optional() 不接受 null。
        let mut v = valid();
        v["truncated"] = json!(null);
        assert!(parse_create_workflow_display(&v).is_none());
    }

    #[test]
    fn accepts_causality_graph_as_raw_json() {
        let mut v = valid();
        v["causalityGraph"] = json!({ "nodes": [1, 2], "edges": [] });
        let d = parse_create_workflow_display(&v).unwrap();
        assert!(d.causality_graph.is_some(), "因果图保留原始 JSON");
    }

    #[test]
    fn read_workflow_display_reads_raw_display_field() {
        let raw = json!({ "display": valid() });
        assert!(read_workflow_display(&raw).is_some());
        // 非record 或无 display → None（退回纯文本兜底，不崩）。
        assert!(read_workflow_display(&json!(1)).is_none());
        assert!(read_workflow_display(&json!({})).is_none());
    }

    #[test]
    fn fallback_output_text_from_string() {
        assert_eq!(
            read_fallback_output_text(&json!("  hello  ")).as_deref(),
            Some("hello")
        );
        assert_eq!(read_fallback_output_text(&json!("   ")), None);
    }

    #[test]
    fn fallback_output_text_candidate_order() {
        // 真源 :32 —— 顺序 response → output → text → content。
        let obj = json!({ "content": "c", "text": "t", "output": "o", "response": "r" });
        assert_eq!(read_fallback_output_text(&obj).as_deref(), Some("r"));

        // 前面的键空/非串时继续往后找。
        let obj2 = json!({ "response": "  ", "text": "t" });
        assert_eq!(read_fallback_output_text(&obj2).as_deref(), Some("t"));

        let obj3 = json!({ "response": 1, "output": "o" });
        assert_eq!(read_fallback_output_text(&obj3).as_deref(), Some("o"));
    }

    #[test]
    fn fallback_output_text_none_when_all_blank() {
        assert_eq!(read_fallback_output_text(&json!({})), None);
        assert_eq!(
            read_fallback_output_text(&json!({ "response": " ", "text": "" })),
            None
        );
    }

    #[test]
    fn tooltip_joins_lede_and_diagnostics() {
        let diags = vec![
            (3i64, 7i64, "Expected ';'".to_string()),
            (4, 1, "Unexpected".to_string()),
        ];
        let tip = format_workflow_feedback_tooltip("编不过", &diags);
        assert_eq!(
            tip,
            "编不过\nL3:C7 Expected ';'\nL4:C1 Unexpected",
            "首行是那句话，后逐条「行:列 消息」"
        );
    }

    #[test]
    fn tooltip_without_diagnostics_is_just_lede() {
        assert_eq!(
            format_workflow_feedback_tooltip("只有那句话", &[]),
            "只有那句话"
        );
    }

    #[test]
    fn diagnostic_lines_dedupes_and_filters_non_positive() {
        // 真源 :68-71 —— 首次出现序、去重、只要正行号。
        let diags = vec![
            (3i64, 1i64, "a".to_string()),
            (3, 2, "b".to_string()),
            (0, 1, "c".to_string()),
            (-1, 1, "d".to_string()),
            (5, 1, "e".to_string()),
        ];
        assert_eq!(workflow_diagnostic_lines(&diags), vec![3, 5]);
    }

    #[test]
    fn diagnostic_lines_empty_when_all_non_positive() {
        let diags = vec![(0i64, 1i64, "a".to_string())];
        assert!(workflow_diagnostic_lines(&diags).is_empty());
    }
}