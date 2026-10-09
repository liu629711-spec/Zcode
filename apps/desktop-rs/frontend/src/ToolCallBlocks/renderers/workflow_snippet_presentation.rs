//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/workflow-snippet-presentation.ts`
//! （47 行）。
//!
//! handler 的 response 面向模型，含重复耗时与日志；本模块只解包已知格式，
//! 未知/截断内容原样保留。

/// `snippetResponse`（真源 :4-46）。
///
/// 参数按真源 `ToolCallEvalWorkflowSnippetDisplay` 的字段顺序用元组表达：
/// `(ok, duration_ms, response, logs, diagnostics)`，
/// 其中 `diagnostics` 是 `(line, column, message)` 序列。
pub fn snippet_response(
    ok: bool,
    duration_ms: i64,
    response: &str,
    logs: &[String],
    diagnostics: &[(i64, i64, String)],
) -> Option<String> {
    let mut text = response.to_string();

    // 真源 :6-8 —— logs 段落（空列表则无）。
    let logs_suffix = if logs.is_empty() {
        None
    } else {
        Some(format!(
            "\n\nLogs:\n{}",
            logs
                .iter()
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    };

    // 真源 :9-21 —— 若正文以耗时/失败开头，剥掉尾部重复的 logs 段落。
    if let Some(suffix) = &logs_suffix {
        let starts_with_timing = text.starts_with(&format!(
            "The snippet completed in {duration_ms}ms.\n"
        )) || text.starts_with("The snippet failed (");
        if starts_with_timing {
            for candidate in [
                suffix.clone(),
                format!("{suffix}\n- … (logs truncated)"),
            ] {
                if text.ends_with(&candidate) {
                    text = text[..text.len() - candidate.len()].to_string();
                    break;
                }
            }
        }
    }

    if ok {
        // 真源 :22-30 —— 剥掉耗时前缀与「无返回值」提示。
        let prefix = format!("The snippet completed in {duration_ms}ms.\n");
        if text.starts_with(&prefix) {
            text = text[prefix.len()..].to_string();
            if text == "It returned no value." {
                return None;
            }
            if text.starts_with("Return value:\n") {
                text = text["Return value:\n".len()..].to_string();
            }
        }
    } else if !diagnostics.is_empty() {
        // 真源 :31-44 —— 编译没过时，正文是「诊断清单 + 未执行提示」的固定串，
        // 整体返回 None（正文已被下方诊断列表替代）。
        let diag_text = diagnostics
            .iter()
            .map(|(line, column, message)| format!("L{line}:C{column} {message}"))
            .collect::<Vec<_>>()
            .join("\n");
        let expected = format!(
            "The snippet has errors:\n{diag_text}\n\n\
             NOTE: The snippet was NOT executed — fix the errors above and call the tool again."
        );
        if text == expected {
            return None;
        }
    }

    // 真源 :45 —— `return text.trim() ? text : undefined`：
    // 用trim 判断是否为空，但**返回原 text**（不裁掉首尾空白）。
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

/// `snippetValue`（真源 :48-55）：能解析成 JSON 就 pretty-print 成 json，
/// 否则当纯文本。
pub fn snippet_value(value: &str) -> (String, &'static str) {
    match serde_json::from_str::<serde_json::Value>(value) {
        Ok(parsed) => {
            // 真源 `JSON.stringify(value, null, 2)` —— 缩进 2 空格。
            let pretty = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| value.to_string());
            // serde_json 的 pretty 缩进也是 2 空格，与 JS 一致。
            (pretty, "json")
        }
        Err(_) => (value.to_string(), "text"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logs_block(logs: &[&str]) -> String {
        format!(
            "\n\nLogs:\n{}",
            logs.iter()
                .map(|l| format!("- {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    #[test]
    fn strips_completion_prefix_and_return_value() {
        // 真源 :22-26 —— 完成 + 有返回值。
        let out = snippet_response(
            true,
            12,
            "The snippet completed in 12ms.\nReturn value:\n42",
            &[],
            &[],
        );
        assert_eq!(out.as_deref(), Some("42"));
    }

    #[test]
    fn no_value_returns_none() {
        // 真源 :26 —— 「It returned no value.」直接回None。
        assert_eq!(
            snippet_response(true, 5, "The snippet completed in 5ms.\nIt returned no value.", &[], &[]),
            None
        );
    }

    #[test]
    fn returns_raw_text_when_no_prefix() {
        // 没有耗时前缀时原样保留。
        assert_eq!(
            snippet_response(true, 9, "  hello  ", &[], &[]).as_deref(),
            Some("  hello  "),
            "真源用 trim 判断空，返回原 text（不裁空白）"
        );
    }

    #[test]
    fn blank_response_is_none() {
        assert_eq!(snippet_response(true, 3, "   \n ", &[], &[]), None);
    }

    #[test]
    fn strips_duplicate_logs_suffix() {
        // 真源 :9-21 —— 正文以耗时开头且尾部重复 logs 段时剥掉。
        let logs = logs_block(&["read file", "compute"]);
        let response = format!("The snippet completed in 7ms.\nvalue here\n{logs}");
        let out = snippet_response(
            true,
            7,
            &response,
            &["read file".to_string(), "compute".to_string()],
            &[],
        );
        assert_eq!(
            out.as_deref(),
            Some("value here
"),
            "尾部 logs 被剥掉，但保留原换行（真源按后缀长度切）"
        );
    }

    #[test]
    fn strips_truncated_logs_suffix() {
        // 真源 :18 —— 截断标记形态也要剥。
        let logs = logs_block(&["a"]);
        let response =
            format!("The snippet failed (boom)\nkept\n{logs}\n- … (logs truncated)");
        let out = snippet_response(false, 0, &response, &["a".to_string()], &[]);
        assert_eq!(
            out.as_deref(),
            Some("The snippet failed (boom)
kept
"),
            "失败分支不剥耗时前缀（真源 :31 只在 ok 时剥）"
        );
    }

    #[test]
    fn keeps_logs_when_prefix_absent() {
        // 正文不以耗时/失败开头 → 不剥 logs（真源 :10-12 的条件）。
        let logs = logs_block(&["x"]);
        let response = format!("plain body{logs}");
        let out = snippet_response(true, 1, &response, &["x".to_string()], &[]);
        assert!(out.unwrap().contains("Logs:"), "无前缀时保留 logs");
    }

    #[test]
    fn failed_snippet_with_matching_diagnostics_returns_none() {
        // 真源 :31-44 —— 正文正是「诊断 + 未执行提示」固定串时回 None。
        let diags = vec![(3i64, 7i64, "Expected ;".to_string())];
        let expected = "The snippet has errors:\nL3:C7 Expected ;\n\n\
NOTE: The snippet was NOT executed — fix the errors above and call the tool again.";
        assert_eq!(
            snippet_response(false, 0, expected, &[], &diags),
            None,
            "诊断清单已单独渲染，正文回None"
        );
    }

    #[test]
    fn failed_snippet_with_other_text_keeps_it() {
        // 真源 :41-43 —— 只有精确匹配才回 None。
        assert_eq!(
            snippet_response(false, 0, "some other failure", &[], &[(1, 1, "x".into())]).as_deref(),
            Some("some other failure")
        );
    }

    #[test]
    fn failed_snippet_without_diagnostics_keeps_text() {
        assert_eq!(
            snippet_response(false, 0, "The snippet failed (null)", &[], &[]).as_deref(),
            Some("The snippet failed (null)")
        );
    }

    #[test]
    fn diagnostics_format_uses_line_col_message() {
        // 多条诊断按 L{line}:C{col} {msg} 换行拼接。
        let diags = vec![
            (3i64, 7i64, "Expected ;".to_string()),
            (4, 1, "Unexpected token".to_string()),
        ];
        let expected = "The snippet has errors:\nL3:C7 Expected ;\nL4:C1 Unexpected token\n\n\
NOTE: The snippet was NOT executed — fix the errors above and call the tool again.";
        assert_eq!(snippet_response(false, 0, expected, &[], &diags), None);
    }

    #[test]
    fn value_pretty_prints_json() {
        let (code, lang) = snippet_value(r#"{"b":1,"a":[2,3]}"#);
        assert_eq!(lang, "json");
        assert!(code.contains("\n  "), "应缩进 2 空格：{code}");
        // serde_json::Map 默认按 BTree 排序（无 preserve_order 时），JS 保序。
        // 不断言键序，只断言结构。
        assert!(code.contains("\"b\": 1") || code.contains("\"b\":1"));
    }

    #[test]
    fn value_falls_back_to_text() {
        let (code, lang) = snippet_value("not json at all");
        assert_eq!(lang, "text");
        assert_eq!(code, "not json at all", "非 JSON 原样返回");
    }

    #[test]
    fn value_of_plain_number_is_json() {
        let (code, lang) = snippet_value("42");
        assert_eq!(lang, "json");
        assert_eq!(code, "42");
    }
}