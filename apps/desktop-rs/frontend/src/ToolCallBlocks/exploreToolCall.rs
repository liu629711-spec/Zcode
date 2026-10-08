//! 1:1 翻译 `packages/ui/src/lib/exploreToolCall.ts`（175 行）。
//!
//! 「只读探查」与「写执行」的判定——决定工具行进 explore 分组还是 execute 分组。
//! 三个出口：
//! - `is_shell_tool_call_awaiting_command`：shell 工具但命令还没流出来
//!   （分组的可见性门控：这种行暂不参与分组与阶段判定）；
//! - `is_explore_tool_call`：读类（file-read/search/explore family）+ 只读 shell 命令；
//! - `is_execute_tool_call`：shell 且非等待、非只读。
//!
//! **正则照抄真源**（regex-lite，wasm 兼容）：`\\b`/交替/`(?i)` 语义一致，
//! 命令 token 全是 ASCII，无 Unicode 边界差异。

use regex_lite::Regex;
use std::sync::OnceLock;

use super::resolveRenderer::{ToolFamily, family_by_lower};

fn is_plain_record(value: &serde_json::Value) -> bool {
    value.is_object()
}

/// `splitCommandSegments`（:7-12）：按 && || ; 拆段。
fn split_command_segments(command: &str) -> Vec<String> {
    // 真源按 /&&|\|\||;/ 全局拆分。
    let re = split_re();
    re.split(command)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn split_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"&&|\|\||;").expect("拆分正则"))
}

/// `unwrapShellCommand`（:14-39）：剥掉 `bash -lc "..."` / `powershell -Command "..."` 外壳。
fn unwrap_shell_command(command: &str) -> String {
    let trimmed = command.trim();
    let strip_wrapping_quotes = |value: &str| -> String {
        let normalized = value.trim();
        let bytes = normalized.as_bytes();
        if bytes.len() >= 2
            && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
                || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
        {
            return normalized[1..normalized.len() - 1].trim().to_string();
        }
        normalized.to_string()
    };

    if let Some(caps) = shell_lc_re().captures(trimmed) {
        if let Some(inner) = caps.get(1) {
            return strip_wrapping_quotes(inner.as_str());
        }
    }
    if let Some(caps) = powershell_re().captures(trimmed) {
        if let Some(inner) = caps.get(1) {
            return strip_wrapping_quotes(inner.as_str());
        }
    }
    trimmed.to_string()
}

fn shell_lc_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源：/^(?:\/bin\/)?(?:zsh|bash|sh)\s+-lc\s+([\s\S]+)$/i
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(?:/bin/)?(?:zsh|bash|sh)\s+-lc\s+(.+)$").expect("shell -lc 正则")
    })
}

fn powershell_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源：/^(?:powershell(?:\.exe)?|pwsh(?:\.exe)?)\b[\s\S]*?\s-(?:command|c)\s+([\s\S]+)$/i
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(?:powershell(?:\.exe)?|pwsh(?:\.exe)?)\b.*?\s-(?:command|c)\s+(.+)$")
            .expect("powershell 正则")
    })
}

/// `normalizeCommandCandidate`（:41-48）。
fn normalize_command_candidate(candidate: &str) -> Vec<String> {
    let unwrapped = unwrap_shell_command(candidate);
    if unwrapped.is_empty() {
        return Vec::new();
    }
    split_command_segments(&unwrapped)
}

/// `extractToolCommands`（:50-111）：从 input 各形状提取命令段（去重、保序）。
///
/// 形状：字符串 / 字符串数组（找 `-lc` 取后一项，否则 join）/ 对象数组（取 `cmd`）
/// / 对象 input 的 command、cmd、script、parsed_cmd 四键。
pub fn extract_tool_commands(input: &serde_json::Value) -> Vec<String> {
    use serde_json::Value;
    let mut candidates: Vec<String> = Vec::new();

    fn collect_from_value(value: &Value, candidates: &mut Vec<String>) {
        if let Some(s) = value.as_str() {
            let command = s.trim();
            if !command.is_empty() {
                candidates.push(command.to_string());
            }
            return;
        }
        let Some(array) = value.as_array() else {
            return;
        };
        if array.iter().all(|item| item.is_string()) {
            let parts: Vec<&str> = array.iter().filter_map(|i| i.as_str()).collect();
            if let Some(index) = parts.iter().position(|p| *p == "-lc") {
                if let Some(shell_command) = parts.get(index + 1) {
                    let command = shell_command.trim();
                    if !command.is_empty() {
                        candidates.push(command.to_string());
                        return;
                    }
                }
            }
            let joined = parts.join(" ").trim().to_string();
            if !joined.is_empty() {
                candidates.push(joined);
            }
            return;
        }
        for item in array {
            if !is_plain_record(item) {
                continue;
            }
            if let Some(cmd) = item.get("cmd").and_then(|c| c.as_str()) {
                let cmd = cmd.trim();
                if !cmd.is_empty() {
                    candidates.push(cmd.to_string());
                }
            }
        }
    }

    collect_from_value(input, &mut candidates);

    if !is_plain_record(input) {
        return dedup_normalized(&candidates);
    }

    for key in ["command", "cmd", "script", "parsed_cmd"] {
        if let Some(value) = input.get(key) {
            collect_from_value(value, &mut candidates);
        }
    }
    dedup_normalized(&candidates)
}

/// 真源末尾的 `Array.from(new Set(flatMap(normalizeCommandCandidate)))`。
fn dedup_normalized(candidates: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for candidate in candidates {
        for normalized in normalize_command_candidate(candidate) {
            if seen.insert(normalized.clone()) {
                out.push(normalized);
            }
        }
    }
    out
}

/// `EXECUTE_READ_COMMAND_RE`（:113-114）。
fn execute_read_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(rg|grep|find|ls|cat|head|tail|wc|stat|pwd|which|readlink|tree|sed\s+-n|get-childitem|gci|dir|get-content|gc|type|select-string|sls|get-location|test-path|resolve-path)\b|^git\s+(status|log|show|diff)\b",
        )
        .expect("读命令正则")
    })
}

/// `EXECUTE_WRITE_COMMAND_RE`（:115-116）。
fn execute_write_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(sed\s+-i|perl\s+-pi|tee|mv|cp|rm|mkdir|rmdir|touch|truncate|chmod|chown|remove-item|del|erase|set-content|add-content|clear-content|out-file|new-item|move-item|copy-item|rename-item|set-item)\b|^git\s+(add|commit|rm|mv|checkout|switch|restore|reset|clean|revert|cherry-pick|merge|rebase)\b",
        )
        .expect("写命令正则")
    })
}

/// `SHELL_REDIRECT_WRITE_RE`（:117）。
fn shell_redirect_write_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)(^|[^\d<])>>?\s*\S|&>\s*\S").expect("重定向正则"))
}

/// `SHELL_LOOP_RE`（:118）。
fn shell_loop_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\b(for|while)\b").expect("循环正则"))
}

/// `isShellToolCallAwaitingCommand`（:120-123）。
pub fn is_shell_tool_call_awaiting_command(kind: &str, input: &serde_json::Value) -> bool {
    family_by_lower(kind) == ToolFamily::Shell && extract_tool_commands(input).is_empty()
}

/// `isExploreToolCall`（:125-166）。
pub fn is_explore_tool_call(kind: &str, input: &serde_json::Value) -> bool {
    let family = family_by_lower(kind);

    if family == ToolFamily::FileWrite {
        return false;
    }
    if family == ToolFamily::FileRead
        || family == ToolFamily::Search
        || family == ToolFamily::Explore
    {
        return true;
    }
    if family != ToolFamily::Shell {
        return false;
    }

    let commands = extract_tool_commands(input);
    if commands.is_empty() {
        return false;
    }
    if commands.iter().any(|c| execute_write_re().is_match(c)) {
        return false;
    }
    if commands
        .iter()
        .any(|c| shell_redirect_write_re().is_match(c))
    {
        return false;
    }
    if commands.iter().any(|c| execute_read_re().is_match(c)) {
        return true;
    }
    // 真源 :161-165：只读探查包在 for/while 循环里的情况。
    commands
        .iter()
        .any(|c| shell_loop_re().is_match(c) && execute_read_re().is_match(c))
}

/// `isExecuteToolCall`（:168-175）。
pub fn is_execute_tool_call(kind: &str, input: &serde_json::Value) -> bool {
    family_by_lower(kind) == ToolFamily::Shell
        && !is_shell_tool_call_awaiting_command(kind, input)
        && !is_explore_tool_call(kind, input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn split_and_unwrap_shell_commands() {
        // 真源 :7-48。
        assert_eq!(
            split_command_segments("ls -la &&  cat a.txt ; pwd"),
            vec!["ls -la", "cat a.txt", "pwd"]
        );
        // bash -lc "..." 剥壳 + 去引号。
        assert_eq!(unwrap_shell_command("bash -lc \"rg foo\""), "rg foo");
        assert_eq!(unwrap_shell_command("/bin/zsh -lc 'ls -la'"), "ls -la");
        // powershell -Command。
        assert_eq!(
            unwrap_shell_command("powershell.exe -Command \"Get-ChildItem\""),
            "Get-ChildItem"
        );
    }

    #[test]
    fn extract_commands_from_string_array_object_shapes() {
        // 字符串数组含 -lc：取后一项（真源 :68-75）。
        assert_eq!(
            extract_tool_commands(&json!(["bash", "-lc", "rg -n foo"])),
            vec!["rg -n foo"]
        );
        // 对象数组取 cmd。
        assert_eq!(
            extract_tool_commands(&json!([{ "cmd": "ls -la" }, { "cmd": "pwd" }])),
            vec!["ls -la", "pwd"]
        );
        // 对象 input 的 command 键。
        assert_eq!(
            extract_tool_commands(&json!({ "command": "git status" })),
            vec!["git status"]
        );
        // 空输入。
        assert!(extract_tool_commands(&json!({})).is_empty());
    }

    #[test]
    fn read_only_shell_is_explore() {
        // 真源 :157-159：读命令 → explore。
        assert!(is_explore_tool_call(
            "Bash",
            &json!({ "command": "rg -n foo" })
        ));
        assert!(is_explore_tool_call(
            "Bash",
            &json!({ "command": "git status" })
        ));
        assert!(is_explore_tool_call(
            "Bash",
            &json!({ "command": "ls -la && cat a" })
        ));
    }

    #[test]
    fn write_shell_is_execute_not_explore() {
        // 写命令 → 不是 explore（真源 :149-151）。
        assert!(!is_explore_tool_call(
            "Bash",
            &json!({ "command": "rm -rf build" })
        ));
        assert!(is_execute_tool_call(
            "Bash",
            &json!({ "command": "rm -rf build" })
        ));
        // 重定向写（真源 :153-155）。
        assert!(!is_explore_tool_call(
            "Bash",
            &json!({ "command": "echo hi > out.txt" })
        ));
        assert!(is_execute_tool_call(
            "Bash",
            &json!({ "command": "echo hi > out.txt" })
        ));
        // git 写子命令。
        assert!(is_execute_tool_call(
            "Bash",
            &json!({ "command": "git commit -m x" })
        ));
    }

    #[test]
    fn redirect_regex_ignores_numeric_fd() {
        // 真源 :117：2> 这类数字 fd 重定向不算写（[^\d<] 负向）。
        assert!(!shell_redirect_write_re().is_match("cmd 2> /dev/null"));
        // 普通 > 算写。
        assert!(shell_redirect_write_re().is_match("echo x > f"));
        assert!(shell_redirect_write_re().is_match("echo x >> f"));
        assert!(shell_redirect_write_re().is_match("cmd &> f"));
    }

    #[test]
    fn loop_wrapped_read_is_explore() {
        // 真源 :161-165：for/while 包裹的只读命令属 explore。
        assert!(is_explore_tool_call(
            "Bash",
            &json!({ "command": "for f in *.md; do cat $f; done" })
        ));
    }

    #[test]
    fn awaiting_command_holds_shell_without_command() {
        // 真源 :120-123：shell 但还没有命令 → awaiting。
        assert!(is_shell_tool_call_awaiting_command("Bash", &json!({})));
        assert!(!is_shell_tool_call_awaiting_command(
            "Bash",
            &json!({ "command": "ls" })
        ));
        // 非 shell family 恒 false。
        assert!(!is_shell_tool_call_awaiting_command("Read", &json!({})));
    }

    #[test]
    fn file_family_predicates_short_circuit() {
        // 真源 :128-142：读类 family 直接 true，写类直接 false。
        assert!(is_explore_tool_call(
            "Read",
            &json!({ "file_path": "a.rs" })
        ));
        assert!(is_explore_tool_call("Grep", &json!({ "pattern": "x" })));
        assert!(!is_explore_tool_call(
            "Write",
            &json!({ "file_path": "a.rs" })
        ));
        assert!(!is_explore_tool_call(
            "Edit",
            &json!({ "file_path": "a.rs" })
        ));
        // 非以上 family：不是 explore 也不是 execute。
        assert!(!is_explore_tool_call("TodoWrite", &json!({})));
        assert!(!is_execute_tool_call("TodoWrite", &json!({})));
    }
}
