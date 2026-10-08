//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaActionDetail.ts`
//! （52 行）。
//!
//! CUA 动作摘要：`open_application` → 目标应用（URL 路径叶子）；
//! `key`/`hold_key` → macOS 快捷键符号串。

use serde_json::Value;

/// `MAC_KEY_SYMBOLS`（真源 :21-30）。
const MAC_KEY_SYMBOLS: [(&str, &str); 8] = [
    ("cmd", "⌘"),
    ("command", "⌘"),
    ("meta", "⌘"),
    ("shift", "⇧"),
    ("alt", "⌥"),
    ("option", "⌥"),
    ("ctrl", "⌃"),
    ("control", "⌃"),
];

/// `asRecord`（真源的 `typeof value === "object" && !Array.isArray`）。
///
/// 签名用 `impl Borrow<Value>`：Rust 的 `Borrow<T> for T` blanket impl 让
/// `as_record(v)` 与 `as_record(&v)` 两种传法都成立（TS 的宽松 record 判定
/// 不区分引用形态）。
fn as_record(value: impl std::borrow::Borrow<Value>) -> Option<Value> {
    value.borrow().as_object().map(|o| Value::Object(o.clone()))
}

fn read_text(record: Option<&Value>, key: &str) -> Option<String> {
    record
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `readOpenTarget`（真源 :11-20）：从 `input.app.url` 取路径末段。
///
/// **化简注明**：真源用 `new URL(url)` + `decodeURIComponent`；Rust 侧手写
/// 提取 `scheme://host` 之后到 `?`/`#` 之前的路径并做 percent decode
/// （decodeURIComponent 语义：保留字符也解码）。
fn read_open_target(input: &Value) -> Option<String> {
    let url = read_text(
        as_record(input)
            .and_then(|i| i.get("app").cloned())
            .as_ref(),
        "url",
    )?;
    // 去 scheme://host 前缀。
    let after_scheme = if let Some(rest) = url.split_once("://") {
        rest.1
    } else {
        url.as_str()
    };
    let path = after_scheme
        .split_once('/')
        .map(|(_, rest)| rest)
        .unwrap_or("");
    let path = path.split(['?', '#']).next().unwrap_or("");
    let leaf = decode_uri_component(path)
        .split('/')
        .filter(|s| !s.is_empty())
        .next_back()
        .unwrap_or("")
        .trim()
        .to_string();
    (!leaf.is_empty()).then_some(leaf)
}

/// `decodeURIComponent` 的等价（百分号编码全解码）。
fn decode_uri_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

/// `formatShortcut`（真源 :32-40）。
fn format_shortcut(value: &str) -> String {
    let parts: Vec<&str> = value
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return value.to_string();
    }
    parts
        .iter()
        .map(|part| {
            let lower = part.to_lowercase();
            MAC_KEY_SYMBOLS
                .iter()
                .find(|(key, _)| *key == lower)
                .map(|(_, symbol)| (*symbol).to_string())
                .unwrap_or_else(|| part.to_uppercase())
        })
        .collect::<Vec<_>>()
        .join("")
}

/// `readCuaActionDetail`（真源 :42-51）。
pub fn read_cua_action_detail(tool_name: Option<&str>, input: &Value) -> Option<String> {
    match tool_name {
        Some("open_application") => read_open_target(input),
        Some("key") | Some("hold_key") => {
            let record = as_record(input);
            let shortcut =
                read_text(record.as_ref(), "key").or_else(|| read_text(record.as_ref(), "text"))?;
            Some(format_shortcut(&shortcut))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn open_application_reads_url_leaf() {
        let input = json!({"app": {"url": "file:///Applications/Safari.app/"}});
        assert_eq!(
            read_cua_action_detail(Some("open_application"), &input).as_deref(),
            Some("Safari.app")
        );
    }

    #[test]
    fn key_formats_mac_symbols() {
        let input = json!({"key": "cmd+shift+a"});
        assert_eq!(
            read_cua_action_detail(Some("key"), &input).as_deref(),
            Some("⌘⇧A")
        );
        // text 字段回退。
        let input = json!({"text": "ctrl+c"});
        assert_eq!(
            read_cua_action_detail(Some("hold_key"), &input).as_deref(),
            Some("⌃C")
        );
    }

    #[test]
    fn other_tools_have_no_detail() {
        assert_eq!(
            read_cua_action_detail(Some("left_click"), &json!({"target": {"index": 1}})),
            None
        );
        assert_eq!(read_cua_action_detail(None, &json!({})), None);
    }
}
