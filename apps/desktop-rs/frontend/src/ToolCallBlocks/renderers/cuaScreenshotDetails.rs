//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaScreenshotDetails.ts`
//! （87 行）。
//!
//! CUA 截图详情：图片 data URL、尺寸、格式、全屏/缩放/区域/钳制标记。

use std::sync::OnceLock;

use regex_lite::Regex;
use serde_json::Value;

/// `CuaScreenshotDetails`（真源 :3-12）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuaScreenshotDetails {
    pub data_url: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub mime_type: Option<String>,
    pub full_screen: bool,
    pub zoom: bool,
    pub region: Option<String>,
    pub clamped: bool,
}

/// `asRecord`（真源的 `typeof value === "object" && !Array.isArray`）。
///
/// 签名用 `impl Borrow<Value>`：Rust 的 `Borrow<T> for T` blanket impl 让
/// `as_record(v)` 与 `as_record(&v)` 两种传法都成立（TS 的宽松 record 判定
/// 不区分引用形态）。
fn as_record(value: impl std::borrow::Borrow<Value>) -> Option<Value> {
    value.borrow().as_object().map(|o| Value::Object(o.clone()))
}

/// `findImageDataUrl`（真源 :20-49）：递归（≤5 层）找 data URL 或
/// `{mimeType, data}` 组装。
fn find_image_data_url(value: &Value, depth: usize) -> Option<String> {
    static DATA_URL_RE: OnceLock<Regex> = OnceLock::new();
    let data_url_re = DATA_URL_RE.get_or_init(|| {
        Regex::new(r"(?i)^data:image/[a-z0-9.+-]+;base64,").expect("data URL 正则")
    });
    if depth > 5 {
        return None;
    }
    if let Some(text) = value.as_str() {
        return data_url_re.is_match(text).then(|| text.to_string());
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .find_map(|item| find_image_data_url(item, depth + 1));
    }
    let Some(record) = as_record(value) else {
        return None;
    };
    let mime_type = record
        .get("mimeType")
        .and_then(|v| v.as_str())
        .or_else(|| record.get("mime_type").and_then(|v| v.as_str()));
    let data = record.get("data").and_then(|v| v.as_str());
    if let (Some(mime), Some(data)) = (mime_type, data) {
        if mime.starts_with("image/") && !data.starts_with("data:") {
            return Some(format!("data:{mime};base64,{data}"));
        }
    }
    // 真源 :45-48 —— 遍历记录全部字段值递归下探。
    let Some(object) = record.as_object() else {
        return None;
    };
    let mut values = object.values();
    values.find_map(|child| find_image_data_url(child, depth + 1))
}

/// `collectText`（真源 :51-58）。
fn collect_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(other) => serde_json::to_string(other).unwrap_or_default(),
        None => String::new(),
    }
}

/// `buildCuaScreenshotDetails`（真源 :60-87）。
pub fn build_cua_screenshot_details(
    output: Option<&Value>,
    input: &Value,
    raw: &Value,
) -> CuaScreenshotDetails {
    static DIMENSIONS_RE: OnceLock<Regex> = OnceLock::new();
    static ATTACHED_MIME_RE: OnceLock<Regex> = OnceLock::new();
    static FULL_SCREEN_RE: OnceLock<Regex> = OnceLock::new();
    static ZOOM_RE: OnceLock<Regex> = OnceLock::new();
    static CLAMPED_RE: OnceLock<Regex> = OnceLock::new();
    static DATA_MIME_RE: OnceLock<Regex> = OnceLock::new();

    let dimensions_re = DIMENSIONS_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:(?:Full-screen\s+)?screenshot|Zoom image)\s+(\d+)x(\d+)px")
            .expect("尺寸正则")
    });
    let attached_mime_re = ATTACHED_MIME_RE.get_or_init(|| {
        Regex::new(r"(?i)\[Attached\s+(image/[a-z0-9.+-]+):").expect("附件 mime 正则")
    });
    let full_screen_re = FULL_SCREEN_RE
        .get_or_init(|| Regex::new(r"(?i)Full-screen screenshot\s+\d+x\d+px").expect("全屏正则"));
    let zoom_re = ZOOM_RE.get_or_init(|| Regex::new(r"(?i)Zoom image").expect("缩放正则"));
    let clamped_re =
        CLAMPED_RE.get_or_init(|| Regex::new(r"(?i)Region clamped").expect("钳制正则"));
    let data_mime_re = DATA_MIME_RE.get_or_init(|| {
        Regex::new(r"(?i)^data:(image/[a-z0-9.+-]+);base64,").expect("data mime 正则")
    });

    let raw_output = as_record(raw).and_then(|r| r.get("rawOutput").cloned());
    let text = format!(
        "{}\n{}",
        collect_text(output),
        collect_text(raw_output.as_ref())
    );
    let dimensions = dimensions_re.captures(&text);
    let attached_mime = attached_mime_re
        .captures(&text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());
    let data_url = find_image_data_url(output.unwrap_or(&Value::Null), 0)
        .or_else(|| find_image_data_url(raw, 0));
    let data_mime = data_url
        .as_deref()
        .and_then(|url| data_mime_re.captures(url))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    // 真源 :77-82 —— region 必须是 4 个数字的数组。
    let region = as_record(input)
        .and_then(|i| i.get("region").cloned())
        .and_then(|r| r.as_array().cloned())
        .filter(|items| items.len() == 4 && items.iter().all(|v| v.is_number()))
        .map(|items| {
            items
                .iter()
                .map(|v| v.as_f64().map(|f| f.to_string()).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(", ")
        });

    CuaScreenshotDetails {
        width: dimensions
            .as_ref()
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok()),
        height: dimensions
            .as_ref()
            .and_then(|c| c.get(2))
            .and_then(|m| m.as_str().parse().ok()),
        data_url,
        mime_type: data_mime.or(attached_mime),
        full_screen: full_screen_re.is_match(&text),
        zoom: zoom_re.is_match(&text),
        region,
        clamped: clamped_re.is_match(&text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_dimensions_and_flags() {
        let output = json!("Full-screen screenshot 1280x720px\nRegion clamped");
        let details = build_cua_screenshot_details(Some(&output), &json!({}), &json!({}));
        assert_eq!(details.width, Some(1280));
        assert_eq!(details.height, Some(720));
        assert!(details.full_screen);
        assert!(details.clamped);
        assert!(!details.zoom);
    }

    #[test]
    fn finds_nested_image_data_url() {
        let output = json!({"content": [{"type": "image", "data": "AAA"}]});
        let raw = json!({"mimeType": "image/png", "data": "BBB"});
        // output 无图 → 落到 raw 顶层的 {mimeType, data} 组装。
        let details = build_cua_screenshot_details(Some(&output), &json!({}), &raw);
        assert_eq!(
            details.data_url.as_deref(),
            Some("data:image/png;base64,BBB")
        );
        assert_eq!(details.mime_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn region_requires_four_numbers() {
        let input = json!({"region": [0, 0, 100, 200]});
        let details = build_cua_screenshot_details(None, &input, &json!({}));
        assert_eq!(details.region.as_deref(), Some("0, 0, 100, 200"));
        let input = json!({"region": [0, 0, 100]});
        let details = build_cua_screenshot_details(None, &input, &json!({}));
        assert_eq!(details.region, None);
    }

    #[test]
    fn zoom_flag_and_attached_mime() {
        let output = json!("Zoom image 800x600px\n[Attached image/webp: x]");
        let details = build_cua_screenshot_details(Some(&output), &json!({}), &json!({}));
        assert!(details.zoom);
        assert_eq!(details.mime_type.as_deref(), Some("image/webp"));
    }
}
