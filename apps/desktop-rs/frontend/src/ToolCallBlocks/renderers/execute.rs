//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/execute.tsx`（380 行）。
//!
//! execute / bash 卡片。真源要点：
//! - **唯一有「终端盒子」主体的卡片**（:299 `rounded-xl border border-border
//!   bg-panel px-4 py-3`）
//! - `hideSecondaryTextWhenOpen`（:338）—— 展开后隐藏命令摘要
//! - `secondaryText` 收起态用 **font-sans**（:290），展开后的完整命令仍用
//!   font-mono 便于阅读复制技术内容（真源 :291-293 注释）
//! - 输出优先级（:274-283）：`raw.display.output` > `output` > `raw.rawOutput`
//! - **已接线**：快照提示（:368-377，Fragment 尾部；办公模式不渲染）
//!
//! 入参形态有 **5 种**（`getExecuteContentParts`，:98-190），都要兼容：
//! 字符串（可能带 `zsh -lc ` 前缀）、字符串数组、{command}/{cmd}/{script} 递归、
//! `parsed_cmd[]` 的字符串项或对象项（cmd/command/script）。
//!
//! 行号注释均指真源文件。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

use crate::ToolCallBlocks::fileSummaryTypes::is_plain_record;

/// `EXECUTE_TOOL_ICON`（真源 :14）——SquareTerminalIcon。
pub const EXECUTE_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 终端盒子的类名（真源 :299）。
pub const TERMINAL_BOX_CLASS: &str =
    "space-y-3 mb-2 rounded-xl border border-border bg-panel px-4 py-3";

/// `readFirstStringField`（真源 :81-96）：按候选键顺序取第一个非空字符串。
fn read_first_string_field(record: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = record.get(key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// `getExecuteSecondaryText`（真源 :21-78）——摘要行里的命令文本。
///
/// 数组形态要特别处理（:37-43）：找到 `-lc` 就取它**下一项**，
/// 那才是真正的命令；否则把所有项拼起来。
pub fn get_execute_secondary_text(input: &Value) -> Option<String> {
    // ① 字符串
    if let Some(s) = input.as_str() {
        let t = s.trim();
        return (!t.is_empty()).then(|| t.to_string());
    }

    // ② 全字符串数组
    if let Some(items) = input.as_array() {
        if items.iter().all(|i| i.is_string()) {
            let parts: Vec<String> = items
                .iter()
                .filter_map(|i| i.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if parts.is_empty() {
                return None;
            }
            // 找 "-lc"，取下一项（真源 :39-41）。
            if let Some(idx) = parts.iter().position(|p| p == "-lc") {
                if let Some(next) = parts.get(idx + 1) {
                    return Some(next.clone());
                }
            }
            return Some(parts.join(" "));
        }
        return None;
    }

    // ③ 对象
    if !is_plain_record(input) {
        return None;
    }

    // parsed_cmd 数组：逐项找字符串或 { cmd } （真源 :47-64）。
    if let Some(parsed) = input.get("parsed_cmd").and_then(|v| v.as_array()) {
        for item in parsed {
            if let Some(s) = item.as_str() {
                let t = s.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
                continue;
            }
            if !is_plain_record(item) {
                continue;
            }
            if let Some(cmd) = item.get("cmd").and_then(|v| v.as_str()) {
                let t = cmd.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }
    }

    // 四个候选键（真源 :67-77）。
    read_first_string_field(input, &["command", "cmd", "script", "parsed_cmd"])
}

/// `getExecuteContentParts`（真源 :98-190）的结果。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecuteContentParts {
    pub execution_command: Option<String>,
}

/// `getExecuteContentParts`（真源 :98-190）——递归解析五种入参形态。
///
/// 字符串形态要剥掉 `zsh -lc ` / `bash -lc ` / `sh -lc ` / `/bin/zsh -lc ` 前缀
/// （真源 :110-112 正则 `^(?:\/bin\/)?(zsh|bash|sh)\s+-lc\s+([\s\S]+)$`）。
pub fn get_execute_content_parts(input: &Value) -> ExecuteContentParts {
    // ① 字符串（可能带 shell 前缀）
    if let Some(s) = input.as_str() {
        let t = s.trim();
        if t.is_empty() {
            return ExecuteContentParts::default();
        }
        if let Some(cmd) = strip_shell_prefix(t) {
            return ExecuteContentParts {
                execution_command: Some(cmd),
            };
        }
        return ExecuteContentParts {
            execution_command: Some(t.to_string()),
        };
    }

    // ② 全字符串数组
    if let Some(items) = input.as_array() {
        if items.iter().all(|i| i.is_string()) {
            let parts: Vec<String> = items
                .iter()
                .filter_map(|i| i.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if parts.is_empty() {
                return ExecuteContentParts::default();
            }
            if let Some(idx) = parts.iter().position(|p| p == "-lc") {
                return ExecuteContentParts {
                    execution_command: parts.get(idx + 1).cloned(),
                };
            }
            // 真源 :135-137 —— 无 -lc 时从第二项开始拼（首项通常是 shell 名）。
            let joined = parts[1..].join(" ");
            return ExecuteContentParts {
                execution_command: (!joined.is_empty()).then_some(joined),
            };
        }
        return ExecuteContentParts::default();
    }

    if !is_plain_record(input) {
        return ExecuteContentParts::default();
    }

    // ③④⑤ command/cmd/script 递归（真源 :147-160）
    for key in ["command", "cmd", "script"] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return get_execute_content_parts(&Value::String(s.to_string()));
            }
        }
    }

    // ⑥ parsed_cmd 数组（真源 :162-187）
    if let Some(parsed) = input.get("parsed_cmd").and_then(|v| v.as_array()) {
        for item in parsed {
            if let Some(s) = item.as_str() {
                let parts = get_execute_content_parts(&Value::String(s.to_string()));
                if parts.execution_command.is_some() {
                    return parts;
                }
                continue;
            }
            if !is_plain_record(item) {
                continue;
            }
            // 优先级 cmd > command > script（真源 :172-181）。
            let cmd = ["cmd", "command", "script"]
                .iter()
                .find_map(|k| item.get(k).and_then(|v| v.as_str()).map(str::trim))
                .filter(|s| !s.is_empty());
            if let Some(cmd) = cmd {
                return ExecuteContentParts {
                    execution_command: Some(cmd.to_string()),
                };
            }
        }
    }

    ExecuteContentParts::default()
}

/// 剥掉 `zsh -lc ` 等 shell 前缀（真源 :110-112 的正则）。
///
/// 匹配 `^(/bin/)?(zsh|bash|sh) -lc <命令>`，返回命令部分。
pub fn strip_shell_prefix(text: &str) -> Option<String> {
    let rest = text.strip_prefix("/bin/").unwrap_or(text).trim_start();
    let rest = ["zsh", "bash", "sh"]
        .iter()
        .find_map(|shell| rest.strip_prefix(shell))?
        .trim_start();
    let rest = rest.strip_prefix("-lc")?.trim_start();
    (!rest.is_empty()).then(|| rest.to_string())
}

/// `extractExecuteResultText`（真源 :192-263）——递归提取输出文本。
///
/// 优先级（:210-218）：output / text / content / result / stdout / message，
/// 再递归 `rawOutput`，再拼 `content[]` 数组。数字/布尔转字符串（:257-259）。
pub fn extract_execute_result_text(output: &Value) -> Option<String> {
    if output.is_null() {
        return None;
    }
    if let Some(s) = output.as_str() {
        return (!s.trim().is_empty()).then(|| s.to_string());
    }
    if let Some(items) = output.as_array() {
        let pieces: Vec<String> = items
            .iter()
            .filter_map(extract_execute_result_text)
            .collect();
        return (!pieces.is_empty()).then(|| pieces.join("\n"));
    }
    if is_plain_record(output) {
        if let Some(direct) = read_first_string_field(
            output,
            &["output", "text", "content", "result", "stdout", "message"],
        ) {
            return Some(direct);
        }
        if let Some(raw_output) = output.get("rawOutput") {
            if let Some(nested) = extract_execute_result_text(raw_output) {
                return Some(nested);
            }
        }
        if let Some(items) = output.get("content").and_then(|v| v.as_array()) {
            let text = items
                .iter()
                .map(|item| {
                    if let Some(s) = item.as_str() {
                        return s.trim().to_string();
                    }
                    if is_plain_record(item) {
                        return read_first_string_field(
                            item,
                            &["output", "text", "content", "result", "stdout", "message"],
                        )
                        .unwrap_or_default();
                    }
                    String::new()
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    // 数字/布尔转字符串（真源 :257-259）
    if output.is_number() || output.is_boolean() {
        return Some(output.to_string());
    }
    None
}

/// `resultText` 的解析优先级（真源 :274-283）：
/// `raw.display.output` > `output` > `raw.rawOutput`。
pub fn resolve_result_text(
    raw_display_output: Option<String>,
    output: Option<String>,
    raw_output_text: Option<String>,
) -> Option<String> {
    raw_display_output.or(output).or(raw_output_text)
}

/// ExecuteToolCallBlock 的 props（对应真源从 context 解构的字段）。
#[derive(Debug, Clone)]
pub struct ExecuteBlockProps {
    pub tool_id: String,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub input: Value,
    pub output: Value,
    pub raw: Value,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub status: Option<String>,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub is_office_mode: bool,
}

/// 终端盒子主体（真源 :297-328 的 JSX）。
///
/// 结构：命令行（`$` 前缀 + 预格式文本，max-h-15 截断）+ 输出区。
#[component]
pub fn ExecuteTerminalBox(command: Option<String>, output: Option<String>) -> impl IntoView {
    view! {
        <div class=TERMINAL_BOX_CLASS>
            <div class="space-y-1">
                <div class="flex items-start gap-2 font-sans text-ui-base text-foreground">
                    <span class="flex-none text-foreground-subtle">{"$"}</span>
                    <pre class="block min-w-0 max-h-15 flex-1 overflow-auto truncate whitespace-pre-wrap break-words">
                        {command.unwrap_or_default()}
                    </pre>
                </div>
            </div>
            // 真源 :314-318 —— 无输出且非运行中时显「没有输出。」
            {output.map(|o| view! {
                <pre class="max-h-[5lh] flex-none overflow-auto whitespace-pre-wrap break-words font-mono text-ui-base leading-5 text-foreground-subtle">
                    {o}
                </pre>
            })}
        </div>
    }
}

/// ExecuteToolCallBlock 的主体（真源 :265-380）。
#[component]
pub fn ExecuteToolCallBlock(props: ExecuteBlockProps) -> impl IntoView {
    let secondary_text = get_execute_secondary_text(&props.input);
    let content_parts = get_execute_content_parts(&props.input);

    // 真源 :283 —— failure 时用 errorText 优先，其次 resultText。
    let result_text = extract_execute_result_text(&props.output);
    let raw_output_text = if is_plain_record(&props.raw) {
        props
            .raw
            .get("rawOutput")
            .and_then(extract_execute_result_text)
    } else {
        None
    };
    let display_output = props
        .raw
        .get("display")
        .and_then(|d| d.get("output"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let final_result = resolve_result_text(display_output, result_text, raw_output_text);

    let failure_visible_text = if props.status.as_deref() == Some("failed") {
        props.error_text.clone().or_else(|| final_result.clone())
    } else {
        None
    };

    // kindLabel：运行态「正在执行」，否则「终端」（真源 :339-343）。
    let kind_label = if props.is_running {
        "正在执行"
    } else {
        "终端"
    };

    let status_tooltip = if props.is_office_mode {
        None
    } else {
        failure_visible_text.clone()
    };

    // 快照提示（真源 :368-377 —— Fragment 尾部；办公模式不渲染）。
    let notice_refs = props.snapshot_refs.clone();
    let notice_tool_id = props.tool_id.clone();
    let notice_cb = props.on_load_full_tool_call_fields.clone();
    let show_snapshot_notice = !props.is_office_mode;

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                kind_label: Some(kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: props
                    .title
                    .clone()
                    .or_else(|| props.kind.clone())
                    .or_else(|| Some("执行".to_string())),
                secondary_text: secondary_text.clone(),
                // 真源 :338 —— 展开后隐藏命令摘要。
                hide_secondary_text_when_open: Some(true),
                status_label: props.status_label.clone(),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(props.status.as_deref() == Some("failed")),
                is_running: Some(props.is_running),
                title: if props.is_office_mode { None } else { props.title.clone() },
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=EXECUTE_TOOL_ICON_CLASS>
                        <crate::app::Icon paths=vec!["M7 11l-4 3 4 3", "M12 17h6"] circles=vec![] />
                    </span>
                }
                .into_any()
            }))
            // 真源 :297-328 —— 终端盒子主体（命令 + 输出）。
            render_content=Some(std::sync::Arc::new(move || {
                let output_text = failure_visible_text
                    .clone()
                    .or_else(|| final_result.clone())
                    .unwrap_or_default();
                view! {
                    <ExecuteTerminalBox
                        command=content_parts.execution_command.clone()
                        output=(!output_text.is_empty()).then_some(output_text)
                    />
                }
                .into_any()
            }))
        />
        {show_snapshot_notice.then(|| view! {
            <ToolSnapshotFieldNoticeComponent
                props=ToolSnapshotFieldNoticeProps {
                    refs: notice_refs.clone(),
                    tool_id: notice_tool_id.clone(),
                    on_load_full_tool_call_fields: notice_cb.clone(),
                }
            />
        })}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shell_prefix_is_stripped() {
        // 真源 :110-112 正则 `^(?:\/bin\/)?(zsh|bash|sh)\s+-lc\s+...`
        for prefix in [
            "zsh -lc ",
            "bash -lc ",
            "sh -lc ",
            "/bin/zsh -lc ",
            "/bin/bash -lc ",
        ] {
            let text = format!("{prefix}echo hello");
            assert_eq!(
                strip_shell_prefix(&text).as_deref(),
                Some("echo hello"),
                "{prefix} 前缀应被剥掉"
            );
        }
        // 非 shell 前缀不剥。
        assert_eq!(strip_shell_prefix("echo hello"), None);
        assert_eq!(strip_shell_prefix("zsh -c echo"), None, "-c 不是 -lc");
    }

    #[test]
    fn secondary_text_from_plain_string() {
        assert_eq!(
            get_execute_secondary_text(&json!("npm test")).as_deref(),
            Some("npm test")
        );
        // 空串 → None
        assert_eq!(get_execute_secondary_text(&json!("  ")), None);
    }

    #[test]
    fn secondary_text_from_array_takes_item_after_dash_lc() {
        // 真源 :39-41 —— 找 -lc 取下一项。
        let input = json!(["zsh", "-lc", "npm run build"]);
        assert_eq!(
            get_execute_secondary_text(&input).as_deref(),
            Some("npm run build")
        );
        // 无 -lc → 全部拼接。
        let input2 = json!(["ls", "-la"]);
        assert_eq!(
            get_execute_secondary_text(&input2).as_deref(),
            Some("ls -la")
        );
        // 空数组 → None。
        assert_eq!(get_execute_secondary_text(&json!([])), None);
    }

    #[test]
    fn content_parts_from_string_with_prefix() {
        // 真源 :105-118
        let parts = get_execute_content_parts(&json!("bash -lc npm test"));
        assert_eq!(parts.execution_command.as_deref(), Some("npm test"));
        // 无前缀时整串就是命令。
        let parts2 = get_execute_content_parts(&json!("npm test"));
        assert_eq!(parts2.execution_command.as_deref(), Some("npm test"));
    }

    #[test]
    fn content_parts_from_array() {
        // 真源 :124-138
        let parts = get_execute_content_parts(&json!(["zsh", "-lc", "ls -la"]));
        assert_eq!(parts.execution_command.as_deref(), Some("ls -la"));
        // 无 -lc → 从第二项开始拼。
        let parts2 = get_execute_content_parts(&json!(["bash", "echo hi"]));
        assert_eq!(parts2.execution_command.as_deref(), Some("echo hi"));
    }

    #[test]
    fn content_parts_recurses_into_command_field() {
        // 真源 :147-160 —— command/cmd/script 递归。
        let parts = get_execute_content_parts(&json!({ "command": "bash -lc make" }));
        assert_eq!(parts.execution_command.as_deref(), Some("make"));
        let parts2 = get_execute_content_parts(&json!({ "cmd": "ls" }));
        assert_eq!(parts2.execution_command.as_deref(), Some("ls"));
        let parts3 = get_execute_content_parts(&json!({ "script": "python x.py" }));
        assert_eq!(parts3.execution_command.as_deref(), Some("python x.py"));
    }

    #[test]
    fn content_parts_from_parsed_cmd() {
        // 真源 :162-187 —— parsed_cmd 里的 cmd/command/script。
        let input = json!({ "parsed_cmd": [{ "cmd": "git status" }] });
        assert_eq!(
            get_execute_content_parts(&input)
                .execution_command
                .as_deref(),
            Some("git status")
        );
    }

    #[test]
    fn result_text_extraction_priority() {
        // 真源 :210-218
        assert_eq!(
            extract_execute_result_text(&json!({ "output": "A" })).as_deref(),
            Some("A")
        );
        assert_eq!(
            extract_execute_result_text(&json!({ "stdout": "B" })).as_deref(),
            Some("B")
        );
        // output 优先于 text。
        assert_eq!(
            extract_execute_result_text(&json!({ "output": "A", "text": "B" })).as_deref(),
            Some("A")
        );
        // 递归 rawOutput。
        assert_eq!(
            extract_execute_result_text(&json!({ "rawOutput": { "text": "C" } })).as_deref(),
            Some("C")
        );
        // 数组拼接。
        assert_eq!(
            extract_execute_result_text(&json!(["A", "B"])).as_deref(),
            Some("A\nB")
        );
        // 数字/布尔转字符串。
        assert_eq!(
            extract_execute_result_text(&json!(42)).as_deref(),
            Some("42")
        );
        assert_eq!(
            extract_execute_result_text(&json!(true)).as_deref(),
            Some("true")
        );
        // null → None。
        assert_eq!(extract_execute_result_text(&json!(null)), None);
    }

    #[test]
    fn resolve_result_text_priority_order() {
        // 真源 :274-283 —— display.output > output > raw.rawOutput
        assert_eq!(
            resolve_result_text(Some("D".into()), Some("O".into()), Some("R".into())).as_deref(),
            Some("D")
        );
        assert_eq!(
            resolve_result_text(None, Some("O".into()), Some("R".into())).as_deref(),
            Some("O")
        );
        assert_eq!(
            resolve_result_text(None, None, Some("R".into())).as_deref(),
            Some("R")
        );
        assert_eq!(resolve_result_text(None, None, None), None);
    }

    #[test]
    fn terminal_box_class_matches_source() {
        // 真源 :299
        assert!(TERMINAL_BOX_CLASS.contains("rounded-xl"));
        assert!(TERMINAL_BOX_CLASS.contains("border border-border"));
        assert!(TERMINAL_BOX_CLASS.contains("bg-panel"));
        assert!(TERMINAL_BOX_CLASS.contains("px-4 py-3"));
    }
}
