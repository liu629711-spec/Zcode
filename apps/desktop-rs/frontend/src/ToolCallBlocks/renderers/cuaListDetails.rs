//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaListDetails.tsx`
//! （230 行）。
//!
//! `list_apps` / `list_windows` 的详情列表。
//!
//! 真源注释（:81-83）：v4/replayable 的稳定事实源是 **display**；legacy 文本
//! 可能已被 head/tail 裁剪。
//!
//! **裁剪注明**：应用图标走 `platform.getApplicationIcon`（IPC 未迁）——
//! 一律回退 AppWindow 图标（与真源的「取不到图标」分支同形态）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::toolResultDisplay::{ToolResultDisplay, read_tool_result_display};

/// `CuaAppItem`（真源 :9-13）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuaAppItem {
    pub name: String,
    pub bundle_id: Option<String>,
    pub active: bool,
}

/// `CuaWindowItem`（真源 :15-21）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuaWindowItem {
    pub title: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub main: bool,
    pub focused: bool,
}

/// `CuaDetailList`（真源 :23-26）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CuaDetailList {
    Apps(Vec<CuaAppItem>),
    Windows(Vec<CuaWindowItem>),
}

impl CuaDetailList {
    /// 列表项数（真源 `list.items.length` 的两分支共用）。
    pub fn len(&self) -> usize {
        match self {
            Self::Apps(items) => items.len(),
            Self::Windows(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

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

/// `parseCuaResultList`（真源 :42-54）：数组直取；字符串取 structured
/// marker 之前的段再 parse（必须以 `[` 开头）。
fn parse_cua_result_list(value: &Value) -> Option<Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items.clone());
    }
    let text = value.as_str()?;
    let candidate = text
        .split_once("\n\nStructured content:")
        .map(|(head, _)| head)
        .unwrap_or(text)
        .trim();
    if !candidate.starts_with('[') {
        return None;
    }
    serde_json::from_str::<Value>(candidate)
        .ok()
        .and_then(|v| v.as_array().cloned())
}

/// `parseStructuredCuaResultList`（真源 :56-67）：递归解 MCP 的
/// `{ result: "json" }` 包装。
fn parse_structured_cua_result_list(value: &Value) -> Option<Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items.clone());
    }
    if let Some(text) = value.as_str() {
        return serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|v| parse_structured_cua_result_list(&v));
    }
    let result = as_record(value)?.get("result").cloned()?;
    parse_structured_cua_result_list(&result)
}

/// `readCuaResultList`（真源 :69-80）。
fn read_cua_result_list(output: Option<&str>, raw: &Value) -> Vec<Value> {
    let display_rows = match read_tool_result_display(raw) {
        Some(ToolResultDisplay::Cua(display)) => display
            .structured_content
            .as_ref()
            // structured_content 是 Option<String>（真源可传字符串或对象）。
            .and_then(|v: &String| parse_structured_cua_result_list(&Value::String(v.clone()))),
        _ => None,
    };
    display_rows
        .or_else(|| parse_cua_result_list(&Value::String(output.unwrap_or("").to_string())))
        .or_else(|| {
            read_text(as_record(raw).as_ref(), "rawOutput")
                .and_then(|t| parse_cua_result_list(&Value::String(t)))
        })
        .unwrap_or_default()
}

/// `buildCuaDetailList`（真源 :82-124）。
pub fn build_cua_detail_list(
    tool_name: &str,
    output: Option<&str>,
    raw: &Value,
) -> Option<CuaDetailList> {
    let rows = read_cua_result_list(output, raw);
    match tool_name {
        "list_apps" => {
            let mut items: Vec<CuaAppItem> = rows
                .iter()
                .filter_map(|value| {
                    let record = as_record(value);
                    let name = read_text(record.as_ref(), "name")?;
                    Some(CuaAppItem {
                        name,
                        bundle_id: read_text(record.as_ref(), "bundle_id"),
                        // 真源 :95 —— 严格 `=== true`。
                        active: record.as_ref().and_then(|r| r.get("active"))
                            == Some(&Value::Bool(true)),
                    })
                })
                .collect();
            // 真源 :102 —— 活跃项排前（稳定排序：JS sort 对同值保持原序）。
            items.sort_by(|left, right| right.active.cmp(&left.active));
            Some(CuaDetailList::Apps(items))
        }
        "list_windows" => {
            let items: Vec<CuaWindowItem> = rows
                .iter()
                .filter_map(|value| {
                    let record = as_record(value)?;
                    let bounds = record
                        .get("bounds")
                        .and_then(|b| b.as_array().cloned())
                        .unwrap_or_default();
                    Some(CuaWindowItem {
                        title: read_text(Some(&record), "title"),
                        width: bounds.get(2).and_then(|v| v.as_i64()),
                        height: bounds.get(3).and_then(|v| v.as_i64()),
                        main: record.get("main") == Some(&Value::Bool(true)),
                        focused: record.get("focused") == Some(&Value::Bool(true)),
                    })
                })
                .collect();
            Some(CuaDetailList::Windows(items))
        }
        _ => None,
    }
}

/// AppWindowIcon（lucide）三段 path（真源 :135/:169 的图标）。
const APP_WINDOW_PATHS: [&str; 3] = ["M10 4v4", "M2 8h20", "M6 4v4"];

/// `CuaAppList`（真源 :128-176）。
#[component]
pub fn CuaAppListComponent(items: Vec<CuaAppItem>) -> impl IntoView {
    view! {
        <div class="max-h-64 space-y-1 overflow-y-auto pr-1">
            {items
                .into_iter()
                .map(|item| {
                    let active_label = item
                        .active
                        .then(|| view! {
                            <span class="shrink-0 text-sm text-foreground-subtle">"当前活跃"</span>
                        });
                    view! {
                        <div class="flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 hover:bg-surface-hover">
                            // 真源 :154-161 —— 图标取不到时回退 AppWindow。
                            <span class="size-4 flex-none text-foreground-subtle">
                                <crate::app::Icon
                                    paths=APP_WINDOW_PATHS.to_vec()
                                    circles=vec![]
                                />
                            </span>
                            <span class="min-w-0 flex-1 truncate text-sm text-foreground">
                                {item.name.clone()}
                            </span>
                            {active_label}
                        </div>
                    }
                    .into_any()
                })
                .collect_view()}
        </div>
    }
}

/// `CuaWindowList`（真源 :178-222）。
#[component]
pub fn CuaWindowListComponent(items: Vec<CuaWindowItem>) -> impl IntoView {
    view! {
        <div class="max-h-64 space-y-1 overflow-y-auto pr-1">
            {items
                .into_iter()
                .map(|item| {
                    let title = item
                        .title
                        .clone()
                        .unwrap_or_else(|| "未命名窗口".to_string());
                    let tags = (item.main || item.focused).then(|| {
                        let main_tag = item
                            .main
                            .then(|| view! { <span>"主窗口"</span> });
                        let focused_tag = item
                            .focused
                            .then(|| view! { <span>"已聚焦"</span> });
                        view! {
                            <div class="flex flex-wrap gap-x-2 text-sm text-foreground-subtle">
                                {main_tag}
                                {focused_tag}
                            </div>
                        }
                    });
                    let size = match (item.width, item.height) {
                        (Some(w), Some(h)) => view! {
                            <span class="shrink-0 font-mono text-sm text-foreground-subtle">
                                {format!("{w} × {h}")}
                            </span>
                        }
                        .into_any(),
                        _ => ().into_any(),
                    };
                    view! {
                        <div class="flex min-w-0 items-center gap-3 rounded-md px-2 py-1.5 hover:bg-surface-hover">
                            <span class="size-4 flex-none text-foreground-subtle">
                                <crate::app::Icon
                                    paths=APP_WINDOW_PATHS.to_vec()
                                    circles=vec![]
                                />
                            </span>
                            <div class="min-w-0 flex-1">
                                <div class="truncate text-sm text-foreground">{title}</div>
                                {tags}
                            </div>
                            {size}
                        </div>
                    }
                    .into_any()
                })
                .collect_view()}
        </div>
    }
}

/// `CuaDetailListSection`（真源 :224-247）。
#[component]
pub fn CuaDetailListSectionComponent(list: CuaDetailList) -> impl IntoView {
    view! {
        <section class="space-y-2 border-t border-border pt-3">
            <h4 class="text-sm text-foreground-subtle">
                {match &list {
                    CuaDetailList::Apps(_) => "应用列表".to_string(),
                    CuaDetailList::Windows(_) => "窗口列表".to_string(),
                }}
            </h4>
            {match list {
                CuaDetailList::Apps(items) => {
                    view! { <CuaAppListComponent items=items /> }.into_any()
                }
                CuaDetailList::Windows(items) => {
                    view! { <CuaWindowListComponent items=items /> }.into_any()
                }
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_app_list_sorted_active_first() {
        let output = json!([
            {"name": "Safari", "bundle_id": "com.apple.Safari", "active": false},
            {"name": "Finder", "bundle_id": "com.apple.finder", "active": true},
            {"name": ""}
        ]);
        let list =
            build_cua_detail_list("list_apps", Some(output.to_string().as_str()), &json!({}))
                .unwrap();
        let CuaDetailList::Apps(items) = list else {
            panic!("期望 apps 列表");
        };
        assert_eq!(items.len(), 2, "空名丢弃");
        assert_eq!(items[0].name, "Finder", "活跃项排前");
        assert!(items[0].active);
        assert_eq!(items[1].bundle_id.as_deref(), Some("com.apple.Safari"));
    }

    #[test]
    fn builds_window_list_from_bounds() {
        let output = json!([
            {"title": "文档", "bounds": [0, 0, 800, 600], "main": true, "focused": false},
            {"bounds": [0, 0, 100, 100]}
        ]);
        let list = build_cua_detail_list(
            "list_windows",
            Some(output.to_string().as_str()),
            &json!({}),
        )
        .unwrap();
        let CuaDetailList::Windows(items) = list else {
            panic!("期望 windows 列表");
        };
        assert_eq!(items[0].width, Some(800));
        assert_eq!(items[0].height, Some(600));
        assert!(items[0].main);
        assert_eq!(items[1].title, None, "无标题 → 未命名窗口");
    }

    #[test]
    fn display_structured_content_wins() {
        // 真源注释：display 是稳定事实源；legacy 文本可能已裁剪。
        let raw = json!({"display": {"kind": "cua", "schemaVersion": 1, "toolName": "list_apps", "status": "success", "structuredContent": "[{\"name\":\"来自 display\"}]"}});
        let list =
            build_cua_detail_list("list_apps", Some("[{\"name\":\"来自输出\"}]"), &raw).unwrap();
        let CuaDetailList::Apps(items) = list else {
            panic!("期望 apps 列表");
        };
        assert_eq!(items[0].name, "来自 display");
    }

    #[test]
    fn other_tools_have_no_list() {
        assert_eq!(build_cua_detail_list("screenshot", None, &json!({})), None);
    }
}
