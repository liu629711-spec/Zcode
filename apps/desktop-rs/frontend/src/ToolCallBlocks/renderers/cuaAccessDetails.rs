//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaAccessDetails.ts`
//! （120 行）。
//!
//! CUA `request_access` 权限详情：辅助功能 / 屏幕录制两行 + 平台环境行。
//!
//! 真源注释（:85-90）：旧展示把 runtime 的兼容字段（自动化 / 输入控制）
//! 也当成产品所需权限——Computer Use 的权限契约**只有**辅助功能与屏幕录制，
//! 原始兼容字段仍留在折叠数据里供排障，但不进面向用户的列表。

use serde_json::Value;

/// `CuaAccessRow`（真源 :3-7）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuaAccessRow {
    pub label_id: &'static str,
    pub value: String,
    /// `true` 已授权 / `false` 未授权 / `None` 未知（决定行首图标）。
    pub status: Option<bool>,
}

/// `CuaAccessDetails`（真源 :9-13）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuaAccessDetails {
    pub ready: bool,
    pub permission_rows: Vec<CuaAccessRow>,
    pub environment_rows: Vec<CuaAccessRow>,
}

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

/// `parsePrimaryObject`（真源 :21-31）：字符串取第一段（`Structured content:`
/// 之前）再 parse。
fn parse_primary_object(value: &Value) -> Option<Value> {
    if let Some(record) = as_record(value) {
        return Some(record);
    }
    let text = value.as_str()?;
    let candidate = text
        .split_once("\n\nStructured content:")
        .map(|(head, _)| head)
        .unwrap_or(text)
        .trim();
    if !candidate.starts_with('{') {
        return None;
    }
    serde_json::from_str::<Value>(candidate)
        .ok()
        .and_then(|v| as_record(&v))
}

/// `statusAfterToBoolean`（真源 :33-38）。
fn status_after_to_boolean(status: Option<&str>) -> Option<bool> {
    match status {
        Some("granted") | Some("not_required") => Some(true),
        Some("denied") | Some("not_granted") | Some("blocked") => Some(false),
        _ => None,
    }
}

/// `readPermissionStatus`（真源 :40-54）：runtime 把状态对象放在**顶层**
/// （`result.accessibility`），旧形态嵌在 `permission_request` 下——两处都读。
fn read_permission_status(value: Option<&Value>, fallback: Option<&Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(true)) => return Some(true),
        Some(Value::String(s)) if s == "granted" => return Some(true),
        Some(Value::Bool(false)) => return Some(false),
        Some(Value::String(s)) if s == "denied" => return Some(false),
        _ => {}
    }
    if let Some(from_value) = status_after_to_boolean(
        read_text(
            as_record(value.unwrap_or(&Value::Null)).as_ref(),
            "status_after",
        )
        .as_deref(),
    ) {
        return Some(from_value);
    }
    status_after_to_boolean(
        read_text(
            as_record(fallback.unwrap_or(&Value::Null)).as_ref(),
            "status_after",
        )
        .as_deref(),
    )
}

/// `statusValue`（真源 :68-76）：状态词中文。
fn status_value(status: Option<bool>) -> String {
    match status {
        Some(true) => "已授权".to_string(),
        Some(false) => "未授权".to_string(),
        None => "未知".to_string(),
    }
}

/// `buildCuaAccessDetails`（真源 :57-118）。
///
/// `output` / `raw`：工具输出与原始载荷（`rawOutput` 回退）。
pub fn build_cua_access_details(output: &Value, raw: &Value) -> CuaAccessDetails {
    let raw_output = read_text(as_record(raw).as_ref(), "rawOutput");
    let empty = Value::Object(serde_json::Map::new());
    let result = parse_primary_object(output)
        .or_else(|| {
            raw_output
                .as_deref()
                .and_then(|t| parse_primary_object(&Value::String(t.to_string())))
        })
        .unwrap_or(empty);
    let permission_request = as_record(
        result
            .get("permission_request")
            .cloned()
            .unwrap_or(Value::Null),
    );

    let accessibility = read_permission_status(
        result.get("accessibility"),
        permission_request
            .as_ref()
            .and_then(|p| p.get("accessibility")),
    );
    let screen_recording = read_permission_status(
        result.get("screen_recording"),
        permission_request
            .as_ref()
            .and_then(|p| p.get("screen_recording")),
    );
    // permission_guide 顶层（runtime）或 permission_request 下（旧形态）都认。
    let permission_guide = permission_request
        .as_ref()
        .and_then(|p| p.get("permission_guide"))
        .cloned()
        .and_then(|v| as_record(&v))
        .or_else(|| {
            result
                .get("permission_guide")
                .cloned()
                .and_then(|v| as_record(&v))
        });
    let ready = permission_guide
        .as_ref()
        .and_then(|g| g.get("all_required_granted"))
        .and_then(|v| v.as_bool())
        .unwrap_or(accessibility == Some(true) && screen_recording == Some(true));

    let permission_rows = vec![
        CuaAccessRow {
            label_id: "chat.toolCall.cua.details.accessibility",
            value: status_value(accessibility),
            status: accessibility,
        },
        CuaAccessRow {
            label_id: "chat.toolCall.cua.details.screenRecording",
            value: status_value(screen_recording),
            status: screen_recording,
        },
    ];

    let mut environment_rows: Vec<CuaAccessRow> = Vec::new();
    if let Some(platform) = read_text(Some(&result), "platform") {
        environment_rows.push(CuaAccessRow {
            label_id: "chat.toolCall.cua.details.platform",
            value: if platform == "macos" {
                "macOS".to_string()
            } else {
                platform
            },
            status: None,
        });
    }
    if let Some(backend) = read_text(Some(&result), "backend") {
        environment_rows.push(CuaAccessRow {
            label_id: "chat.toolCall.cua.details.backend",
            value: backend,
            status: None,
        });
    }
    let subject = as_record(
        result
            .get("authorization_subject")
            .cloned()
            .unwrap_or(Value::Null),
    );
    if let Some(helper) = read_text(subject.as_ref(), "display_name") {
        environment_rows.push(CuaAccessRow {
            label_id: "chat.toolCall.cua.details.permissionOwner",
            value: helper,
            status: None,
        });
    }

    CuaAccessDetails {
        ready,
        permission_rows,
        environment_rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_top_level_runtime_status_objects() {
        let output = json!({
            "accessibility": {"status_after": "granted"},
            "screen_recording": {"status_after": "denied"},
            "platform": "macos",
            "backend": "core",
            "authorization_subject": {"display_name": "终端用户"}
        });
        let details = build_cua_access_details(&output, &json!({}));
        assert_eq!(details.permission_rows[0].status, Some(true));
        assert_eq!(details.permission_rows[0].value, "已授权");
        assert_eq!(details.permission_rows[1].status, Some(false));
        assert!(!details.ready, "未全授权");
        assert_eq!(details.environment_rows.len(), 3);
        assert_eq!(details.environment_rows[0].value, "macOS");
        assert_eq!(details.environment_rows[2].value, "终端用户");
    }

    #[test]
    fn reads_legacy_nested_permission_request() {
        let output = json!({
            "permission_request": {
                "accessibility": {"status_after": "not_required"},
                "screen_recording": {"status_after": "granted"},
                "permission_guide": {"all_required_granted": true}
            }
        });
        let details = build_cua_access_details(&output, &json!({}));
        assert_eq!(details.permission_rows[0].status, Some(true));
        assert!(details.ready);
    }

    #[test]
    fn unknown_status_renders_unknown() {
        let details = build_cua_access_details(&json!({}), &json!({}));
        assert_eq!(details.permission_rows[0].status, None);
        assert_eq!(details.permission_rows[0].value, "未知");
        assert!(!details.ready);
        assert!(details.environment_rows.is_empty());
    }
}
