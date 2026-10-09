//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cua.tsx`（504 行）。
//!
//! CUA（Computer Use）主卡。逻辑层拆在同目录 8 个已迁模块里
//! （`cuaResultState` / `cuaActionDetail` / `cuaListDetails` / `cuaAccessDetails` /
//! `cuaScreenshotDetails` / `cuaErrorDetails` / `cuaIcon` / `cuaSummaryMessages`），
//! 本文件承载真源本体：工具名识别、详情模型、摘要呈现、组件编排。
//!
//! 真源头部有oxlint 豁免注释（:1）：summary 与同源 detail 投影集中维护，
//! 本次 review 不扩大重构范围 —— Rust 侧保持同样边界，不额外拆文件。

use leptos::prelude::*;
use serde_json::Value;
use std::sync::Arc;

use super::super::toolCallRowAdapter::LegacyToolCall;
use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};
use super::super::ToolLayout::ToolLayoutComponent;
use super::cuaAccessDetails::{CuaAccessDetails, build_cua_access_details};
use super::cuaActionDetail::read_cua_action_detail;
use super::cuaListDetails::{CuaDetailList, build_cua_detail_list};
use super::cuaResultState::{
    read_cua_action_target_name, read_cua_app_name, read_cua_result_bundle_id,
    read_cua_result_list_count, read_cua_result_state,
};
use super::cuaScreenshotDetails::{CuaScreenshotDetails, build_cua_screenshot_details};
use super::cuaSummaryMessages::cua_tool_summary_id;
use crate::ToolCallBlocks::i18n;

// ---------------------------------------------------------------------------
// 基础读取工具（真源 :34-48）
// ---------------------------------------------------------------------------

/// `asRecord`（真源 :34-38）：对象且非数组才算 record。
///
/// 用 `Option<Value>` 作参数（而非 `impl Borrow`）：真源的 record 判定
/// 本身就把「可能没有这层对象」当常态，签名直接吃 Option 比让每个调用点
/// 先unwrap 更贴近语义。
fn as_record(value: Option<&Value>) -> Option<Value> {
    value?.as_object().map(|o| Value::Object(o.clone()))
}

/// `readText`（真源 :40-44）：非空字符串（trim 后）。
fn read_text(record: Option<Value>, key: &str) -> Option<String> {
    record
        .as_ref()?
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `readNumber`（真源 :46-48）：有限数值。
fn read_number(record: Option<Value>, key: &str) -> Option<f64> {
    let value = record.as_ref()?.get(key)?.as_f64()?;
    value.is_finite().then_some(value)
}

/// `output` 字段是 `Option<String>`（散文文本），而部分真源 helper 要 `Value`。
/// 这里包成 `Value::String`，保持"文本形态"这一语义（真源也是字符串）。
fn output_value(output: Option<&str>) -> Value {
    output
        .map(|s| Value::String(s.to_string()))
        .unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// 工具名识别（真源 :50-83）
// ---------------------------------------------------------------------------

/// `normalizeCuaToolName`（真源 :50-52）：trim + 小写 + 连字符转下划线。
pub fn normalize_cua_tool_name(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        .replace('-', "_")
}

/// `readCuaToolName`（真源 :54-63）。
///
/// 命名空间两种（真源注释：feat 用 computer_use，main v3.5.3 在 plugin namespace 里）：
/// - feat:        `mcp__computer_use__<action>`
/// - main namesp: `mcp__plugin_zcode_cua_computer_use__<action>`
///
/// 都取尾部 `<action>`（最后一个 `__` 之后），且必须是纯 `[a-z0-9_]`。
pub fn read_cua_tool_name(value: Option<&str>) -> Option<String> {
    let normalized = normalize_cua_tool_name(value);
    if !normalized.contains("computer_use") {
        return None;
    }
    let last_sep = normalized.rfind("__");
    let short_name = match last_sep {
        Some(idx) => &normalized[idx + 2..],
        None => &normalized[..],
    };
    if short_name.is_empty()
        || !short_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }
    Some(short_name.to_string())
}

/// `readRawToolName`（真源 :65-72）：三候选键 `toolName` / `tool_name` / `name`。
pub fn read_raw_tool_name(raw: &Value) -> Option<String> {
    let record = as_record(Some(raw));
    for key in ["toolName", "tool_name", "name"] {
        if let Some(value) = read_text(record.clone(), key) {
            return Some(value);
        }
    }
    None
}

/// `readCuaUserTitle`（真源 :74-82）：三个位置找用户写的 title。
fn read_cua_user_title(tool_call: &LegacyToolCall) -> Option<String> {
    let raw = as_record(Some(&tool_call.raw));
    let raw_input = raw.as_ref().and_then(|r| r.get("rawInput")).cloned();
    let raw_embedded_input = raw.as_ref().and_then(|r| r.get("input")).cloned();
    let candidates = [tool_call.input.clone(), raw_input, raw_embedded_input];
    for input in candidates.iter().filter_map(|v| as_record(v.as_ref())) {
        if let Some(title) = read_text(Some(input), "title") {
            return Some(title);
        }
    }
    None
}

/// `collectToolNames`（真源 :84-88）：四个位置。
fn collect_tool_names(tool_call: &LegacyToolCall) -> Vec<Option<String>> {
    vec![
        tool_call.tool_name.clone(),
        Some(tool_call.kind.clone()),
        tool_call.title.clone(),
        read_raw_tool_name(&tool_call.raw),
    ]
}

/// `readBundleId`（真源 :90-98）：三个容器任一命中。
fn read_bundle_id(input: Option<&Value>) -> Option<String> {
    let record = as_record(input)?;
    let containers = [
        record.get("app").cloned().and_then(|v| as_record(Some(&v))),
        record.get("app_ref").cloned().and_then(|v| as_record(Some(&v))),
        Some(record.clone()),
    ];
    for container in containers.into_iter().flatten() {
        for key in ["bundle_id", "bundleId"] {
            if let Some(id) = read_text(Some(container.clone()), key) {
                return Some(id);
            }
        }
    }
    None
}

/// `readCuaApplicationIconRequest`（真源 :100-113）。
///
/// ★真源注释：历史记录没有 authority metadata 时兼容旧 `bundle_id`；
/// 新记录即使 locator 为空也**不回退模型 input** —— 避免身份冲突后
/// 重新显示未经 Helper 校验的图标。
fn read_cua_application_icon_request(tool_call: &LegacyToolCall) -> Vec<String> {
    let display = read_tool_result_display(&tool_call.raw);
    if let Some(ToolResultDisplay::Cua(cua)) = &display {
        if let Some(target_app) = cua.target_app.as_object() {
            if let Some(locators) = target_app.get("iconLocators").and_then(|v| v.as_array()) {
                return locators
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
            }
        }
        // 有 display 但 locator 为空 → 不回退，返回空。
        return Vec::new();
    }
    let mut out = read_bundle_id(tool_call.input.as_ref())
        .into_iter()
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.extend(read_cua_result_bundle_id(tool_call.output.as_deref(), &tool_call.raw));
    }
    out
}

// ---------------------------------------------------------------------------
// 详情模型（真源 :115-330）
// ---------------------------------------------------------------------------

/// `CuaDetailRow`（真源 :115-120）。
#[derive(Debug, Clone, PartialEq)]
pub struct CuaDetailRow {
    pub label_id: String,
    pub value: String,
    pub code: bool,
    pub status: bool,
}

impl CuaDetailRow {
    fn new(label_id: &str, value: impl Into<String>) -> Self {
        Self {
            label_id: label_id.to_string(),
            value: value.into(),
            code: false,
            status: false,
        }
    }

    fn code(label_id: &str, value: impl Into<String>) -> Self {
        Self {
            code: true,
            ..Self::new(label_id, value)
        }
    }
}

/// `CuaDetailsModel`（真源 :122-141）。
#[derive(Debug, Clone, PartialEq)]
pub struct CuaDetailsModel {
    pub action_rows: Vec<CuaDetailRow>,
    pub result_id: String,
    pub result_values: Vec<(String, String)>,
    pub state_rows: Vec<CuaDetailRow>,
    pub success: bool,
    pub list: Option<CuaDetailList>,
    pub permission_rows: Option<Vec<CuaDetailRow>>,
    pub environment_rows: Option<Vec<CuaDetailRow>>,
    pub screenshot: Option<CuaScreenshotDetails>,
    pub failure_reason_id: Option<String>,
    pub failure_reason: Option<String>,
    pub suggested_action_id: Option<String>,
    pub suggested_action: Option<String>,
}

/// `readTargetDescription`（真源 :143-165）。
fn read_target_description(input: Option<&Value>) -> Option<(String, Vec<(String, String)>)> {
    let target = as_record(input)?.get("target").cloned().and_then(|v| as_record(Some(&v)))?;
    match target.get("type").and_then(|v| v.as_str()) {
        Some("element") => {
            let index = target.get("index").and_then(|v| v.as_f64())?;
            Some((
                "chat.toolCall.cua.details.elementTarget".to_string(),
                vec![("index".to_string(), format_number(index))],
            ))
        }
        Some("coordinate") => {
            let x = target.get("x").and_then(|v| v.as_f64())?;
            let y = target.get("y").and_then(|v| v.as_f64())?;
            Some((
                "chat.toolCall.cua.details.coordinateTarget".to_string(),
                vec![
                    ("x".to_string(), format_number(x)),
                    ("y".to_string(), format_number(y)),
                ],
            ))
        }
        _ => None,
    }
}

/// JS 数字 → 字符串（`String(57)` = `"57"`，`String(57.5)` = `"57.5"`）。
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e21 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// 格式化函数（真源传入的 `formatTarget`）。
pub type FormatTarget = dyn Fn(&str, &[(String, String)]) -> String;

/// `buildCuaDetailsModel`（真源 :167-330）。
pub fn build_cua_details_model(
    tool_name: &str,
    tool_call: &LegacyToolCall,
    format_target: &FormatTarget,
) -> CuaDetailsModel {
    let input = tool_call.input.clone().unwrap_or(Value::Null);
    let input_record = as_record(Some(&input));
    let result = read_cua_result_state(tool_call.output.as_deref(), &tool_call.raw);
    let app_name = read_cua_app_name(&input, result.as_ref());
    let mut state_rows: Vec<CuaDetailRow> = Vec::new();
    let mut action_rows: Vec<CuaDetailRow> = Vec::new();

    let action_detail = read_cua_action_detail(Some(tool_name), &input);
    let typed_text = if tool_name == "type" {
        read_text(input_record.clone(), "text")
    } else {
        None
    };
    let open_url = read_text(
        input_record.as_ref().and_then(|r| r.get("app")).cloned(),
        "url",
    );
    let target = read_target_description(Some(&input));
    let list = build_cua_detail_list(tool_name, tool_call.output.as_deref(), &tool_call.raw);
    let access = if tool_name == "request_access" {
        Some(build_cua_access_details(
            &output_value(tool_call.output.as_deref()),
            &tool_call.raw,
        ))
    } else {
        None
    };
    let screenshot_candidate = if tool_name == "screenshot" || tool_name == "zoom" {
        Some(build_cua_screenshot_details(
            Some(&output_value(tool_call.output.as_deref())),
            &input,
            &tool_call.raw,
        ))
    } else {
        None
    };
    let error = super::cuaErrorDetails::read_cua_error_details(
        tool_call.output.as_deref(),
        &tool_call.raw,
    );
    let display = read_tool_result_display(&tool_call.raw);
    let cua_display = match &display {
        Some(ToolResultDisplay::Cua(d)) => Some(d),
        _ => None,
    };
    let wait_duration = if tool_name == "wait" {
        read_number(input_record.clone(), "duration")
    } else {
        None
    };

    // ── actionRows（真源 :189-227）──
    if app_name.is_none() {
        if let Some(err) = &error {
            if let Some(name) = &err.target_app_name {
                action_rows.push(CuaDetailRow::new("chat.toolCall.cua.details.app", name.clone()));
            }
        }
    }
    let access_is_some = access.is_some();
    if !access_is_some {
        if let Some(name) = &app_name {
            action_rows.push(CuaDetailRow::new("chat.toolCall.cua.details.app", name.clone()));
        }
        if tool_name != "list_apps" && tool_name != "screenshot" {
            let id = cua_tool_summary_id(tool_name).unwrap_or("chat.toolCall.cua.default");
            action_rows.push(CuaDetailRow::new(
                "chat.toolCall.cua.details.operation",
                format_target(id, &[]),
            ));
        }
    }
    if let Some(value) = action_detail.clone().or(typed_text.clone()).or(open_url.clone()) {
        action_rows.push(CuaDetailRow::code("chat.toolCall.cua.details.value", value));
    }
    if let Some(duration) = wait_duration {
        action_rows.push(CuaDetailRow::new(
            "chat.toolCall.cua.details.duration",
            format_target(
                "chat.toolCall.cua.seconds",
                &[("duration".to_string(), format_number(duration))],
            ),
        ));
    }
    if let Some((id, values)) = &target {
        action_rows.push(CuaDetailRow::new(
            "chat.toolCall.cua.details.target",
            format_target(id, values),
        ));
    }

    // ── stateRows（真源 :229-264）──
    let state_id = read_text(result.clone(), "state_id");
    let window_title = read_text(
        result.as_ref().and_then(|r| r.get("window")).cloned(),
        "title",
    );
    let focused_element = result
        .as_ref()
        .and_then(|r| r.get("focused_element"))
        .and_then(|v| v.as_f64());
    let changes = result
        .as_ref()
        .and_then(|r| r.get("changes"))
        .cloned()
        .and_then(|v| as_record(Some(&v)));
    let added_count = changes
        .as_ref()
        .and_then(|c| c.get("added_count"))
        .and_then(|v| v.as_f64());
    let removed_count = changes
        .as_ref()
        .and_then(|c| c.get("removed_count"))
        .and_then(|v| v.as_f64());

    if state_id.is_none() {
        if let Some(err) = &error {
            if let Some(id) = &err.state_id {
                state_rows.push(CuaDetailRow::code(
                    "chat.toolCall.cua.details.stateId",
                    id.clone(),
                ));
            }
        }
    }
    if let Some(id) = state_id {
        state_rows.push(CuaDetailRow::code(
            "chat.toolCall.cua.details.stateId",
            id,
        ));
    }
    if let Some(title) = window_title {
        state_rows.push(CuaDetailRow::new("chat.toolCall.cua.details.window", title));
    }
    if let Some(index) = focused_element {
        state_rows.push(CuaDetailRow::new(
            "chat.toolCall.cua.details.focus",
            format_target(
                "chat.toolCall.cua.details.elementTarget",
                &[("index".to_string(), format_number(index))],
            ),
        ));
    }
    if added_count.is_some() || removed_count.is_some() {
        state_rows.push(CuaDetailRow::new(
            "chat.toolCall.cua.details.changes",
            format_target(
                "chat.toolCall.cua.details.changeCounts",
                &[
                    (
                        "added".to_string(),
                        format_number(added_count.unwrap_or_default()),
                    ),
                    (
                        "removed".to_string(),
                        format_number(removed_count.unwrap_or_default()),
                    ),
                ],
            ),
        ));
    }

    // ── success 判定（真源 :266-271）──
    // ★真源注释：MCP transport completed **不代表** CUA 动作成功；
    // 新 session 优先采用 display 状态。
    let error_code = cua_display
        .and_then(|d| d.error_code.clone())
        .or_else(|| error.as_ref().map(|e| e.code.clone()));
    // 真源 :270 `(access?.ready ?? true)` —— access 为 null 时取 true，
// 只有真的读到 access 且 ready=false 才算未就绪。
// （写成 `is_some_and` 会让「无 access」变成 false，把所有非
//  request_access 的工具全判为失败——测试直接抓到了这个。）
let access_ready = access.as_ref().map(|a| a.ready).unwrap_or(true);
    let success = cua_display.map(|d| d.status.as_str()) != Some("failed")
        && tool_call.status != "failed"
        && error.is_none()
        && access_ready;
    // ★真源 :273 —— 失败截图也建详情模型时会渲染无效截图占位；失败原因区已足够。
    let screenshot = if success { screenshot_candidate } else { None };

    let result_values: Vec<(String, String)> = if let Some(l) = &list {
        vec![("count".to_string(), l.len().to_string())]
    } else if let Some(d) = wait_duration {
        vec![("duration".to_string(), format_number(d))]
    } else {
        Vec::new()
    };

    let result_id = resolve_result_id(
        success,
        access.as_ref(),
        error_code.as_deref(),
        tool_name,
        typed_text.as_deref(),
        wait_duration,
        list.as_ref(),
    );

    let failure_reason_id = if error_code.as_deref() == Some("element_stale") {
        Some("chat.toolCall.cua.details.elementStaleReason".to_string())
    } else {
        None
    };
    let failure_reason = if error_code.as_deref() != Some("element_stale") {
        cua_display
            .filter(|d| d.status == "failed")
            .and_then(|d| d.text.clone().or_else(|| d.error_code.clone()))
    } else {
        None
    };
    let suggested_action_id = if error_code.as_deref() == Some("element_stale") {
        Some("chat.toolCall.cua.details.elementStaleAction".to_string())
    } else {
        None
    };
    let suggested_action = if error_code.as_deref() != Some("element_stale") {
        cua_display.and_then(|d| d.suggested_action.clone())
    } else {
        None
    };

    CuaDetailsModel {
        action_rows,
        result_id,
        result_values,
        state_rows,
        success,
        list,
        permission_rows: access.as_ref().map(|a| {
            a.permission_rows
                .iter()
                .map(|r| CuaDetailRow {
                    label_id: r.label_id.to_string(),
                    value: r.value.clone(),
                    code: false,
                    // 真源 :11 —— `status` 三态（已授权/未授权/未知）决定行首图标。
                    status: r.status.unwrap_or(false),
                })
                .collect()
        }),
        environment_rows: access.as_ref().map(|a| {
            a.environment_rows
                .iter()
                .map(|r| CuaDetailRow {
                    label_id: r.label_id.to_string(),
                    value: r.value.clone(),
                    code: false,
                    status: r.status.unwrap_or(false),
                })
                .collect()
        }),
        screenshot,
        failure_reason_id,
        failure_reason,
        suggested_action_id,
        suggested_action,
    }
}

/// `resultId` 的深层三元链（真源 :283-317），拆成函数以便测试。
fn resolve_result_id(
    success: bool,
    access: Option<&CuaAccessDetails>,
    error_code: Option<&str>,
    tool_name: &str,
    typed_text: Option<&str>,
    wait_duration: Option<f64>,
    list: Option<&CuaDetailList>,
) -> String {
    if !success {
        return if access.is_some() {
            "chat.toolCall.cua.details.accessIncomplete".to_string()
        } else if error_code == Some("element_stale") {
            "chat.toolCall.cua.details.elementStale".to_string()
        } else {
            "chat.toolCall.cua.details.failed".to_string()
        };
    }
    if access.is_some() {
        return "chat.toolCall.cua.details.accessReady".to_string();
    }
    let list_len = list.as_ref().map(|l| l.len()).unwrap_or(0);
    match tool_name {
        "type" if typed_text.is_some() => "chat.toolCall.cua.details.typed",
        "key" => "chat.toolCall.cua.details.keyPressed",
        "open_application" => "chat.toolCall.cua.details.opened",
        "get_app_state" => "chat.toolCall.cua.details.observed",
        "screenshot" => "chat.toolCall.cua.details.screenshotCaptured",
        "wait" if wait_duration.is_some() => "chat.toolCall.cua.details.waited",
        "zoom" => "chat.toolCall.cua.details.zoomed",
        "list_apps" => {
            if list_len > 0 {
                "chat.toolCall.cua.details.appsFound"
            } else {
                "chat.toolCall.cua.details.noApps"
            }
        }
        "list_windows" => {
            if list_len > 0 {
                "chat.toolCall.cua.details.windowsFound"
            } else {
                "chat.toolCall.cua.details.noWindows"
            }
        }
        _ => "chat.toolCall.cua.details.completed",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// CUA 判定与摘要呈现（真源 :332-444）
// ---------------------------------------------------------------------------

/// `isCuaToolCall`（真源 :332-337）。
pub fn is_cua_tool_call(tool_call: &LegacyToolCall) -> bool {
    collect_tool_names(tool_call)
        .iter()
        .any(|name| {
            name.as_deref()
                .is_some_and(|n| super::super::resolveRenderer::is_cua_tool_name(n))
        })
}

/// `CuaSummaryPresentation`（真源 :339-348）。`primaryText` 含节点，
/// 拆成「文案 + 目标药丸」两个字段由渲染侧组装（View 非 Send，见模块说明）。
#[derive(Debug, Clone, PartialEq)]
pub struct CuaSummaryPresentation {
    pub tool_name: Option<String>,
    pub app_name: String,
    pub description: String,
    /// 药丸文本（真源 `taggedTarget`）；`None` 表示纯文本形态。
    pub tagged_target: Option<String>,
    pub title: String,
    pub is_failed: bool,
    pub failure_text: Option<String>,
    pub icon_locators: Vec<String>,
    pub uses_fallback_icon: bool,
}

/// `buildCuaSummaryPresentation`（真源 :353-444）。
pub fn build_cua_summary_presentation(
    tool_call: &LegacyToolCall,
    fallback_error_text: Option<&str>,
) -> CuaSummaryPresentation {
    let tool_name = collect_tool_names(tool_call)
        .iter()
        .filter_map(|name| read_cua_tool_name(name.as_deref()))
        .next();
    let display = read_tool_result_display(&tool_call.raw);
    let cua_display = match &display {
        Some(ToolResultDisplay::Cua(d)) => Some(d),
        _ => None,
    };
    let is_failed = match cua_display {
        Some(d) => d.status == "failed",
        None => tool_call.status == "failed",
    };
    let failure_text = match cua_display {
        Some(d) if d.status == "failed" => d.text.clone().or_else(|| fallback_error_text.map(str::to_string)),
        _ => fallback_error_text.map(str::to_string),
    };

    let raw_cua_app = as_record(Some(&tool_call.raw))
        .as_ref()
        .and_then(|r| r.get("cuaApp"))
        .cloned()
        .and_then(|v| as_record(Some(&v)));
    let result = read_cua_result_state(tool_call.output.as_deref(), &tool_call.raw);
    let result_app = result
        .as_ref()
        .and_then(|r| r.get("app"))
        .cloned()
        .and_then(|v| as_record(Some(&v)));
    let generic_app_name = i18n::text("chat.toolCall.cua.appName");

    let display_name = cua_display.and_then(|d| {
        d.target_app
            .get("displayName")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    let app_name = display_name
        .or_else(|| read_text(result_app.clone(), "name"))
        .or_else(|| read_text(raw_cua_app.clone(), "name"))
        .unwrap_or_else(|| generic_app_name.clone());

    let raw_action_target = match tool_name.as_deref() {
        Some("left_click") | Some("right_click") | Some("type") => {
            let input = tool_call.input.clone().unwrap_or(Value::Null);
            read_cua_action_target_name(&input, tool_call.output.as_deref(), &tool_call.raw)
        }
        _ => None,
    };
    // ★真源 :386-389 —— CUA 无法解析元素可读名称时返回纯数字 index；
    // 直接把 `57` 放进 tag 看起来像无上下文的值，故明确标注为元素编号。
    let action_target = raw_action_target.map(|t| {
        if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
            i18n::format(
                "chat.toolCall.cua.elementTarget",
                &[("index".to_string(), t)],
            )
        } else {
            t
        }
    });
    let key_name = match tool_name.as_deref() {
        Some("key") | Some("hold_key") => {
            read_cua_action_detail(tool_name.as_deref(), &tool_call.input.clone().unwrap_or(Value::Null))
        }
        _ => None,
    };
    let authored_description = if tool_name.as_deref() == Some("get_app_state") {
        read_cua_user_title(tool_call)
    } else {
        None
    };
    let list_count = if tool_name.as_deref() == Some("list_windows") {
        read_cua_result_list_count(tool_call.output.as_deref(), &tool_call.raw)
    } else {
        None
    };

    let description = authored_description.clone().unwrap_or_else(|| {
        if action_target.is_some() {
            let id = match tool_name.as_deref() {
                Some("type") => "chat.toolCall.cua.type",
                Some("right_click") => "chat.toolCall.cua.rightClick",
                _ => "chat.toolCall.cua.leftClick",
            };
            i18n::text(id)
        } else if let Some(_key) = &key_name {
            // 真源 :404-411 —— key_name 非空即走此支，不区分是否 authored。
            let id = if tool_name.as_deref() == Some("hold_key") {
                "chat.toolCall.cua.holdKey"
            } else {
                "chat.toolCall.cua.pressKeyAction"
            };
            i18n::text(id)
        } else if let Some(count) = list_count {
            i18n::format(
                "chat.toolCall.cua.listWindowsCount",
                &[("count".to_string(), count.to_string())],
            )
        } else {
            i18n::text(
                cua_tool_summary_id(tool_name.as_deref().unwrap_or(""))
                    .unwrap_or("chat.toolCall.cua.default"),
            )
        }
    });

    let tagged_target = action_target.or(key_name);
    let tagged_target = if authored_description.is_some() {
        None
    } else {
        tagged_target
    };

    let icon_locators = read_cua_application_icon_request(tool_call);
    let uses_fallback_icon = app_name == generic_app_name || icon_locators.is_empty();

    CuaSummaryPresentation {
        tool_name,
        title: format!("{app_name} {description}"),
        app_name,
        description,
        tagged_target,
        is_failed,
        failure_text,
        icon_locators,
        uses_fallback_icon,
    }
}

// ---------------------------------------------------------------------------
// 组件（真源 :447-504）
// ---------------------------------------------------------------------------

/// 展开/折叠决策（真源 :484-488）。
///
/// 真源三行判定：
/// - `isActive = context.isRunning || status === "pending" || status === "in_progress"`
/// - `canToggle = !isActive && (context.canToggle ?? true)`
/// - `forceOpen = !isActive && !(summary.isFailed && toolName === "screenshot") && (context.forceOpen ?? false)`
///
/// 抽成独立函数：组件里调它、单测直接测它（组件内部的 Effect 需要
/// Leptos executor，单测环境没初始化，不能靠渲染来验证）。
pub fn resolve_active_flags(
    is_running: bool,
    status: &str,
    is_failed: bool,
    tool_name: Option<&str>,
    can_toggle_ctx: Option<bool>,
    force_open_ctx: Option<bool>,
) -> (bool, bool) {
    let is_active = is_running || status == "pending" || status == "in_progress";
    let is_failed_screenshot = is_failed && tool_name == Some("screenshot");
    let can_toggle = can_toggle_ctx.unwrap_or(true) && !is_active;
    let force_open = force_open_ctx.unwrap_or(false) && !is_active && !is_failed_screenshot;
    (can_toggle, force_open)
}

/// `CuaToolCallBlock`（真源 :447-504）。
#[component]
pub fn CuaToolCallBlock(
    tool_call: LegacyToolCall,
    is_running: bool,
    show_icon: bool,
    can_toggle: Option<bool>,
    force_open: Option<bool>,
    error_text: Option<String>,
) -> impl IntoView {
    let fallback_error = error_text.clone();
    let summary = build_cua_summary_presentation(&tool_call, fallback_error.as_deref());
    let tool_name = summary.tool_name.clone();

    let is_failed = summary.is_failed;
    let (resolved_can_toggle, resolved_force_open) = resolve_active_flags(
        is_running,
        &tool_call.status,
        is_failed,
        tool_name.as_deref(),
        can_toggle,
        force_open,
    );

    let status_label = is_failed.then(|| i18n::text("chat.toolCall.status.failed"));
    let status_tooltip = if is_failed {
        summary.failure_text.clone()
    } else {
        None
    };
    let summary_title = summary.title.clone();
    // app_name / description 各留两份：props 结构体按值取一份，
    // 图标闭包与药丸闭包各再取一份（Rust 的 move 语义，闭包捕获即独占）。
    let app_name = summary.app_name.clone();
    let app_name_for_icon = app_name.clone();
    let description = summary.description.clone();
    let tagged_target = summary.tagged_target.clone();
    let uses_fallback_icon = summary.uses_fallback_icon;

    // 真源 :472-479 —— detailsModel 只在能解析出 toolName 时才建。
    let details = tool_name.as_deref().map(|name| {
        build_cua_details_model(name, &tool_call, &|id, values| i18n::format(id, values))
    });

    view! {
        <ToolLayoutComponent
            props=super::super::ToolLayout::ToolLayoutProps {
                tool_id: tool_call.tool_id.clone(),
                show_icon: Some(show_icon),
                can_toggle: Some(resolved_can_toggle),
                force_open: Some(resolved_force_open),
                kind_label: Some(app_name),
                primary_text: Some(description),
                // 真源 :421-427 —— 带药丸时用「描述 + 圆角边框标签」两段。
                secondary_text_view: tagged_target.map(|target| {
                    // 显式标注返回 AnyView：`.into_any()` 的类型推断在闭包里
                    // 会被收窄成 View，导致 Arc 类型与 ChildrenFn 不匹配。
                    let render = move || -> AnyView {
                        view! {
                            <span class="cua-action-target min-w-0 truncate rounded-full border \
                                        border-border px-1.5 text-ui-sm text-foreground-subtlest">
                                {target.clone()}
                            </span>
                        }
                        .into_any()
                    };
                    Arc::new(render)
                        as std::sync::Arc<
                            dyn Fn() -> AnyView + Send + Sync + 'static,
                        >
                }),
                status_label,
                status_tooltip,
                show_failure_status: Some(is_failed),
                is_running: Some(is_running),
                title: Some(summary_title),
                ..Default::default()
            }
            icon_view=Some(Arc::new(move || {
                view! {
                    <span class="shrink-0 size-4 flex items-center justify-center">
                        <CuaSummaryIcon
                            uses_fallback=uses_fallback_icon
                            app_name=app_name_for_icon.clone()
                        />
                    </span>
                }
                .into_any()
            }))
            render_content=details.map(|model| {
                let snapshot = tool_call.clone();
                Arc::new(move || {
                    view! {
                        <super::cuaDetails::CuaToolCallDetailsComponent
                            model=model.clone()
                            tool_call=snapshot.clone()
                        />
                    }
                    .into_any()
                })
                    as std::sync::Arc<
                        dyn Fn() -> leptos::prelude::AnyView + Send + Sync + 'static,
                    >
            })
        />
    }
}

/// 摘要图标（真源 :424-434 的 `CUA_FALLBACK_ICON` / `CuaAppSummaryIcon` 分支）。
///
/// ★`CuaAppSummaryIcon` 走的是 Helper 权威元数据通道（iconLocators，
/// Rust 侧对应字段已由 `cuaIcon.rs` 迁入），本轮只还原兜底分支，
/// 权威图标请求需要主进程侧的 Helper 协议，本文件末尾标注。
#[component]
fn CuaSummaryIcon(uses_fallback: bool, app_name: String) -> impl IntoView {
    if uses_fallback {
        view! {
            <span class=super::cuaIcon::CUA_FALLBACK_ICON_CLASS>
                <crate::app::Icon
                    paths=super::cuaIcon::MOUSE_POINTER_CLICK_PATHS.to_vec()
                    circles=vec![]
                />
            </span>
        }
        .into_any()
    } else {
        // 真源 :430-433 —— 有权威元数据时渲染目标应用图标。
        // Rust 侧暂以首字母药丸占位，形态与类名对齐（详见文件末 TODO）。
        view! {
            <span
                class="flex size-4 flex-none items-center justify-center rounded-[20%] \
                       bg-tag text-ui-xs text-foreground-subtle"
                title=app_name.clone()
            >
                {app_name.chars().next().unwrap_or('?')}
            </span>
        }
        .into_any()
    }
}

// TODO(后续迁移)：以下真源部分本轮未实现，逐项列出避免遗漏。
// 1. `CuaAppSummaryIcon` 的权威图标渲染（cuaAppSummaryIcon.tsx）——
//    需要主进程提供 Helper 协议（bundleId → iconLocators 的解析），
//    当前用首字母药丸占位，类名与尺寸已对齐。
// （`CuaToolCallDetails` 的九个区块已由 `cuaDetails.rs` 完整迁入。）

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;
    use serde_json::json;

    fn tc(tool_name: &str, input: Value, output: Option<&str>, raw: Value) -> LegacyToolCall {
        LegacyToolCall {
            tool_id: "tc-1".into(),
            tool_name: Some(tool_name.into()),
            kind: tool_name.into(),
            title: None,
            input: Some(input),
            status: "completed".into(),
            v4_status: "success".into(),
            output: output.map(str::to_string),
            content: None,
            error: None,
            raw,
            started_at: None,
            snapshot_refs: Vec::new(),
            thought: None,
        }
    }

    fn no_format(id: &str, _values: &[(String, String)]) -> String {
        id.to_string()
    }

    // ── 工具名识别 ──

    #[test]
    fn tool_name_reads_both_namespaces() {
        // 真源 :56-62 注释：feat 与 main 两种命名空间都取尾部 action。
        assert_eq!(
            read_cua_tool_name(Some("mcp__computer_use__left_click")).as_deref(),
            Some("left_click")
        );
        assert_eq!(
            read_cua_tool_name(Some("mcp__plugin_zcode_cua_computer_use__screenshot")).as_deref(),
            Some("screenshot")
        );
    }

    #[test]
    fn tool_name_normalizes_hyphen_and_case() {
        // 真源 :50-52 —— trim + 小写 + 连字符转下划线。
        assert_eq!(normalize_cua_tool_name(Some("  GET-APP-STATE ")), "get_app_state");
        assert_eq!(
            read_cua_tool_name(Some("MCP__COMPUTER-USE__ZOOM")).as_deref(),
            Some("zoom")
        );
    }

    #[test]
    fn tool_name_rejects_non_cua_and_dirty_short_names() {
        // 非 computer_use 命名空间一律拒绝。
        assert_eq!(read_cua_tool_name(Some("mcp__other__left_click")), None);
        // 尾部含非法字符（非 [a-z0-9_]）拒绝。
        assert_eq!(read_cua_tool_name(Some("mcp__computer_use__left click")), None);
        assert_eq!(read_cua_tool_name(Some("mcp__computer_use__")), None);
        assert_eq!(read_cua_tool_name(None), None);
    }

    #[test]
    fn is_cua_tool_call_scans_four_positions() {
        // 真源 :84-88 —— toolName / kind / title / raw.toolName 四处任一命中。
        let mut call = tc("mcp__computer_use__type", json!({}), None, json!({}));
        assert!(is_cua_tool_call(&call));

        call.tool_name = None;
        assert!(is_cua_tool_call(&call), "kind 位仍应命中");

        call.kind = "other".into();
        call.title = Some("mcp__computer_use__type".into());
        assert!(is_cua_tool_call(&call), "title 位仍应命中");

        call.title = None;
        call.raw = json!({ "toolName": "mcp__computer_use__type" });
        assert!(is_cua_tool_call(&call), "raw.toolName 位仍应命中");

        call.raw = json!({});
        assert!(!is_cua_tool_call(&call), "四处都不命中则不是 CUA");
    }

    // ── 详情模型 ──

    #[test]
    fn details_model_action_rows_for_typical_click() {
        let call = tc(
            "mcp__computer_use__left_click",
            json!({
                "app": { "name": "Finder" },
                "target": { "type": "element", "index": 3 },
            }),
            None,
            json!({}),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        let labels: Vec<&str> = model
            .action_rows
            .iter()
            .map(|r| r.label_id.as_str())
            .collect();
        assert!(labels.contains(&"chat.toolCall.cua.details.app"), "应用名");
        assert!(labels.contains(&"chat.toolCall.cua.details.operation"), "操作");
        assert!(labels.contains(&"chat.toolCall.cua.details.target"), "目标");
        assert_eq!(model.result_id, "chat.toolCall.cua.details.completed");
        assert!(model.success);
    }

    #[test]
    fn details_model_coordinate_target() {
        let call = tc(
            "mcp__computer_use__left_click",
            json!({ "target": { "type": "coordinate", "x": 10, "y": 20 } }),
            None,
            json!({}),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        let target_row = model
            .action_rows
            .iter()
            .find(|r| r.label_id == "chat.toolCall.cua.details.target")
            .expect("应有目标行");
        // 真源 :222-227 —— labelId 恒为 details.target，
        // 元素/坐标的具体类型体现在 value 里（value 由 target.id 格式化而来）。
        assert_eq!(target_row.label_id, "chat.toolCall.cua.details.target");
        assert_eq!(
            target_row.value, "chat.toolCall.cua.details.coordinateTarget",
            "no_format 直接回显 id，真实场景由 intl.formatMessage 格式化"
        );
    }

    #[test]
    fn details_model_coordinate_target_formats_with_values() {
        // 用真源语义（带插值）跑一遍，验证 x/y 拼成 "坐标 10, 20"。
        let call = tc(
            "mcp__computer_use__left_click",
            json!({ "target": { "type": "coordinate", "x": 10, "y": 20 } }),
            None,
            json!({}),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &|id, values| {
            i18n::format(id, values)
        });
        let target_row = model
            .action_rows
            .iter()
            .find(|r| r.label_id == "chat.toolCall.cua.details.target")
            .expect("应有目标行");
        assert_eq!(target_row.value, "坐标 10, 20");
    }

    #[test]
    fn details_model_element_target_formats_with_index() {
        let call = tc(
            "mcp__computer_use__left_click",
            json!({ "target": { "type": "element", "index": 57 } }),
            None,
            json!({}),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &|id, values| {
            i18n::format(id, values)
        });
        let target_row = model
            .action_rows
            .iter()
            .find(|r| r.label_id == "chat.toolCall.cua.details.target")
            .expect("应有目标行");
        assert_eq!(target_row.value, "界面元素 #57");
    }

    #[test]
    fn details_model_skips_operation_row_for_list_and_screenshot() {
        // 真源 :203 —— list_apps / screenshot 不出「操作」行。
        for tool in ["list_apps", "screenshot"] {
            let call = tc(
                &format!("mcp__computer_use__{tool}"),
                json!({ "app": { "name": "Finder" } }),
                None,
                json!({}),
            );
            let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
            let model = build_cua_details_model(&tool_name, &call, &no_format);
            assert!(
                !model
                    .action_rows
                    .iter()
                    .any(|r| r.label_id == "chat.toolCall.cua.details.operation"),
                "{tool} 不应有操作行"
            );
        }
    }

    #[test]
    fn details_model_wait_duration_row_and_result() {
        let call = tc(
            "mcp__computer_use__wait",
            json!({ "duration": 3 }),
            None,
            json!({}),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        assert!(model
            .action_rows
            .iter()
            .any(|r| r.label_id == "chat.toolCall.cua.details.duration"));
        assert_eq!(model.result_id, "chat.toolCall.cua.details.waited");
        assert_eq!(
            model.result_values,
            vec![("duration".to_string(), "3".to_string())]
        );
    }

    #[test]
    fn details_model_transport_completed_is_not_success() {
        // ★真源 :266-271 —— MCP transport completed 不代表 CUA 动作成功。
        let call = tc(
            "mcp__computer_use__left_click",
            json!({ "app": { "name": "Finder" } }),
            None,
            json!({
                "display": {
                    "kind": "cua", "schemaVersion": 1, "status": "failed",
                    "toolName": "left_click", "errorCode": "boom"
                }
            }),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        assert!(!model.success, "display 失败时即便 status=completed 也算失败");
        assert_eq!(model.result_id, "chat.toolCall.cua.details.failed");
        assert_eq!(model.failure_reason.as_deref(), Some("boom"));
    }

    #[test]
    fn details_model_element_stale_has_reason_and_action() {
        let call = tc(
            "mcp__computer_use__left_click",
            json!({}),
            None,
            json!({
                "display": {
                    "kind": "cua", "schemaVersion": 1, "status": "failed",
                    "toolName": "left_click", "errorCode": "element_stale",
                    "suggestedAction": "重新读取界面"
                }
            }),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        assert_eq!(model.result_id, "chat.toolCall.cua.details.elementStale");
        assert_eq!(
            model.failure_reason_id.as_deref(),
            Some("chat.toolCall.cua.details.elementStaleReason")
        );
        assert_eq!(
            model.suggested_action_id.as_deref(),
            Some("chat.toolCall.cua.details.elementStaleAction")
        );
        // element_stale 走专属文案，不重复用 display.suggestedAction。
        assert_eq!(model.suggested_action, None);
    }

    #[test]
    fn details_model_failed_screenshot_is_dropped() {
        // ★真源 :273 —— 失败截图不渲染（避免无效占位）。
        let call = tc(
            "mcp__computer_use__screenshot",
            json!({}),
            Some("iVBORw0KGgo="),
            json!({
                "display": { "kind": "cua", "schemaVersion": 1, "status": "failed", "toolName": "screenshot" }
            }),
        );
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        assert!(!model.success);
        assert!(model.screenshot.is_none(), "失败时截图应被丢弃");
    }

    #[test]
    fn details_model_type_without_text_is_not_typed() {
        // 真源 :288 —— type 分支要求 typedText 非空。
        let call = tc("mcp__computer_use__type", json!({}), None, json!({}));
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        assert_eq!(model.result_id, "chat.toolCall.cua.details.completed");
    }

    // ── 摘要呈现 ──

    #[test]
    fn summary_uses_generic_app_name_and_fallback_icon() {
        let call = tc("mcp__computer_use__left_click", json!({}), None, json!({}));
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(summary.app_name, "电脑控制", "无应用名时用泛称");
        assert!(summary.uses_fallback_icon, "泛称时用兜底图标");
        // description 走 CUA_TOOL_SUMMARY_IDS[left_click]（cua.tsx:414-418），
        // 不是默认文案——默认文案只在工具名未登记时兜底。
        assert_eq!(summary.description, "单击");

        // 未登记的工具名才落到 cua.default。
        let unknown = tc("mcp__computer_use__未登记动作", json!({}), None, json!({}));
        assert_eq!(
            build_cua_summary_presentation(&unknown, None).description,
            "使用 Computer Use"
        );
    }

    #[test]
    fn summary_app_name_priority_display_result_raw_generic() {
        // 真源 :370-375 —— display > result.app.name > raw.cuaApp.name > 泛称。
        let call = tc(
            "mcp__computer_use__left_click",
            json!({}),
            None,
            json!({
                "display": {
                    "kind": "cua", "schemaVersion": 1, "status": "success", "toolName": "left_click",
                    "targetApp": { "displayName": "Safari" }
                },
                "cuaApp": { "name": "Chrome" }
            }),
        );
        // display 优先于 raw.cuaApp。
        assert_eq!(build_cua_summary_presentation(&call, None).app_name, "Safari");

        let call2 = tc(
            "mcp__computer_use__left_click",
            json!({}),
            None,
            json!({ "cuaApp": { "name": "Chrome" } }),
        );
        assert_eq!(build_cua_summary_presentation(&call2, None).app_name, "Chrome");
    }

    #[test]
    fn summary_numeric_target_is_labeled() {
        // ★真源 :386-389 —— 纯数字 index 必须标注为元素编号，
        // 否则 tag 里一个 `57` 看起来像无上下文的值。
        // 元素名要从 output 的 `[57] <name>` 行解析（cuaResultState.rs:249-252），
        // 所以这里要给出 output。
        let call = tc(
            "mcp__computer_use__left_click",
            json!({ "target": { "type": "element", "index": 57 } }),
            Some("[57] 57
"),
            json!({}),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(
            summary.tagged_target.as_deref(),
            Some("元素 #57"),
            "纯数字目标应被标注"
        );
        assert_eq!(summary.description, "单击");

        // 元素有可读名时直接用名字，不加编号。
        let named = tc(
            "mcp__computer_use__left_click",
            json!({ "target": { "type": "element", "index": 2 } }),
            Some("[2] 提交按钮
"),
            json!({}),
        );
        let summary_named = build_cua_summary_presentation(&named, None);
        assert_eq!(summary_named.tagged_target.as_deref(), Some("提交按钮"));
    }

    #[test]
    fn summary_click_type_differentiated() {
        // 真源 :395-399 —— type / right_click / left_click 三种 actionId。
        for (tool, expected) in [
            ("type", "输入文本"),
            ("right_click", "右键单击"),
            ("left_click", "单击"),
        ] {
            let call = tc(
                &format!("mcp__computer_use__{tool}"),
                json!({ "target": { "type": "element", "index": 1 } }),
                None,
                json!({}),
            );
            let summary = build_cua_summary_presentation(&call, None);
            assert_eq!(summary.description, expected, "{tool} 的描述");
        }
    }

    #[test]
    fn summary_authored_description_suppresses_target_tag() {
        // 真源 :412-427 —— authoredDescription 存在时不带药丸。
        let call = tc(
            "mcp__computer_use__get_app_state",
            json!({ "title": "看看邮件" }),
            None,
            json!({}),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(summary.description, "看看邮件");
        assert_eq!(summary.tagged_target, None, "有自定义描述时不挂药丸");
    }

    #[test]
    fn summary_title_is_app_plus_description() {
        // ★摘要的 appName 链（真源 :368-372）是
        // display.targetApp.displayName > result.app.name > raw.cuaApp.name > 泛称，
        // **不含 input.app.name**（那只有详情模型用）。故这里给raw.cuaApp。
        let call = tc(
            "mcp__computer_use__left_click",
            json!({ "target": { "type": "element", "index": 2 } }),
            None,
            json!({ "cuaApp": { "name": "Finder" } }),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(summary.app_name, "Finder");
        assert_eq!(summary.title, "Finder 单击");
    }

    #[test]
    fn summary_failure_text_falls_back_to_context_error() {
        let call = tc(
            "mcp__computer_use__left_click",
            json!({}),
            None,
            json!({ "display": { "kind": "cua", "schemaVersion": 1, "status": "failed", "toolName": "left_click" } }),
        );
        let summary = build_cua_summary_presentation(&call, Some("上下文错误"));
        assert!(summary.is_failed);
        assert_eq!(summary.failure_text.as_deref(), Some("上下文错误"));
    }

    #[test]
    fn summary_key_action_uses_key_wording() {
        // 真源 :402-404 —— keyName 非空才走 pressKeyAction 分支；
        // keyName 来自 read_cua_action_detail（读 input.key / input.text）。
        let call = tc(
            "mcp__computer_use__key",
            json!({ "key": "CMD+C" }),
            None,
            json!({}),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(summary.description, "按下", "key 走 pressKeyAction");
        // 真源走 format_shortcut，CMD 会被转成 ⌘ 符号（cuaActionDetail.rs）。
        assert_eq!(summary.tagged_target.as_deref(), Some("⌘C"), "药丸带格式化后的按键名");

        // keyName 为空（无 key/text 字段）→ 不走该分支，
        // 落回 CUA_TOOL_SUMMARY_IDS[key]（真源 :414-418）。
        let bare = tc("mcp__computer_use__key", json!({}), None, json!({}));
        assert_eq!(build_cua_summary_presentation(&bare, None).description, "按键");
    }

    #[test]
    fn summary_hold_key_uses_hold_wording() {
        let call = tc(
            "mcp__computer_use__hold_key",
            json!({ "key": "SHIFT" }),
            None,
            json!({}),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(summary.description, "长按按键");
    }

    #[test]
    fn summary_list_windows_count() {
        let call = tc(
            "mcp__computer_use__list_windows",
            json!({}),
            None,
            json!({
                "display": {
                    "kind": "cua", "schemaVersion": 1, "status": "success", "toolName": "list_windows",
                    // 真源 :110-113 —— 列表计数从 display.text 解析数组长度。
                    "text": "[{\"title\":\"A\"},{\"title\":\"B\"}]"
                }
            }),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert_eq!(summary.description, "2 个窗口");
    }

    // ── 组件的展开/折叠决策（真源 :485-488）──
    //
    // 这里**不渲染组件**：ToolLayout 内部的 Effect 需要 Leptos executor，
    // 单元测试环境没初始化。改为直接验证组件里那三行判定逻辑——
    // 折叠/展开行为本身由 ToolLayout 的测试覆盖。

    #[test]
    fn active_state_suppresses_toggle_and_force_open() {
        // 真源 :485 —— 运行中不能折叠。
        let (can_toggle, force_open) = resolve_active_flags(true, "completed", false, None, None, None);
        assert!(!can_toggle, "运行中不可折叠");
        assert!(!force_open, "运行中不 forceOpen");
    }

    #[test]
    fn pending_and_in_progress_also_count_as_active() {
        // 真源 :484 —— pending / in_progress 与 isRunning 同等对待。
        for status in ["pending", "in_progress"] {
            let (can_toggle, force_open) =
                resolve_active_flags(false, status, false, None, Some(true), Some(true));
            assert!(!can_toggle, "{status} 不可折叠");
            assert!(!force_open, "{status} 不 forceOpen");
        }
    }

    #[test]
    fn failed_screenshot_suppresses_force_open() {
        // 真源 :487 —— 失败截图不强制展开（失败原因区已足够）。
        let (_, force_open) =
            resolve_active_flags(false, "failed", true, Some("screenshot"), None, Some(true));
        assert!(!force_open, "失败截图不 forceOpen");
        // 其他工具失败时仍可 forceOpen。
        let (_, force_open) =
            resolve_active_flags(false, "failed", true, Some("left_click"), None, Some(true));
        assert!(force_open, "非截图失败仍可 forceOpen");
    }

    #[test]
    fn idle_state_respects_context_defaults() {
        // 真源 :485 —— canToggle ?? true / forceOpen ?? false。
        let (can_toggle, force_open) = resolve_active_flags(false, "completed", false, None, None, None);
        assert!(can_toggle, "默认允许折叠");
        assert!(!force_open, "默认不强制展开");
        // 上下文显式给false 时不折叠。
        let (can_toggle, _) =
            resolve_active_flags(false, "completed", false, None, Some(false), None);
        assert!(!can_toggle, "上下文 canToggle=false 应生效");
    }

    #[test]
    fn details_model_none_when_tool_name_unresolvable() {
        // 真源 :472-479 —— detailsModel 只在 toolName 可解析时创建。
        // 非 CUA 工具名解析不出短名→ 详情模型为 None（组件据此返回 null）。
        let call = tc("SomeOtherTool", json!({}), None, json!({}));
        assert_eq!(read_cua_tool_name(call.tool_name.as_deref()), None);
        assert!(!is_cua_tool_call(&call), "非 CUA 工具不应进CUA 渲染路径");
    }

    #[test]
    fn failed_screenshot_summary_keeps_failure_text() {
        let call = tc(
            "mcp__computer_use__screenshot",
            json!({}),
            None,
            json!({
                "display": {
                    "kind": "cua", "schemaVersion": 1, "status": "failed",
                    "toolName": "screenshot", "errorCode": "capture_denied"
                }
            }),
        );
        let summary = build_cua_summary_presentation(&call, None);
        assert!(summary.is_failed);
        // ★真源 :361-363 —— failureText 只有 `text ?? fallbackErrorText` 两级，
        // **不回落到 errorCode**（errorCode 只进详情模型的 failureReason，:305-309）。
        // 这里两者都没有 → None。
        assert_eq!(summary.failure_text, None, "无 text 且无兜底时为 None");

        // 给了上下文错误文本才回落上去。
        let with_fallback = build_cua_summary_presentation(&call, Some("截图权限不足"));
        assert_eq!(
            with_fallback.failure_text.as_deref(),
            Some("截图权限不足"),
            "display 无 text 时回落上下文错误"
        );

        // errorCode 走详情模型那条路。
        let tool_name = read_cua_tool_name(call.tool_name.as_deref()).unwrap();
        let model = build_cua_details_model(&tool_name, &call, &no_format);
        assert_eq!(model.failure_reason.as_deref(), Some("capture_denied"));
    }
}