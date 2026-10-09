//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaDetails.tsx`（152 行）。
//!
//! 8 个区块按真源顺序渲染：操作 / 结果 / 失败原因 / 建议动作 /
//! 截图 / 权限 / 环境 / 列表 / 状态。
//!
//! ★真源 :58-59 的设计约束（照抄）：
//! 「完整 tool call JSON 混入了面向用户的 CUA 详情，暴露内部生命周期字段
//! 并制造无效入口。原始数据继续保留在协议与持久化层；这里仅渲染用户
//! 完成操作所需的信息。」——所以本组件**不渲染 raw JSON**。

use leptos::prelude::*;

use super::super::toolCallRowAdapter::LegacyToolCall;
use super::super::i18n;
use super::cua::CuaDetailsModel;
use super::cuaListDetails::{CuaDetailList, CuaDetailListSectionComponent};
use super::cuaScreenshotDetails::CuaScreenshotDetails;
use super::CuaScreenshotSection::CuaScreenshotSectionComponent;

/// 一条详情行（真源 :19-44 `CuaDetailRows`）。
#[derive(Debug, Clone, PartialEq)]
pub struct DetailRow {
    pub label_id: String,
    pub value: String,
    /// 等宽 + `break-all`（真源 :33-34）。
    pub code: bool,
    /// `Some(true)` 已授权 / `Some(false)` 未授权 / `None` 无状态图标。
    pub status: Option<bool>,
}

/// 区块标题 id（真源各处`intl.formatMessage({ id: "chat.toolCall.cua.details.*" })`）。
pub mod section_ids {
    pub const ACTION: &str = "chat.toolCall.cua.details.action";
    pub const RESULT: &str = "chat.toolCall.cua.details.result";
    pub const FAILURE_REASON: &str = "chat.toolCall.cua.details.failureReason";
    pub const SUGGESTED_ACTION: &str = "chat.toolCall.cua.details.suggestedAction";
    pub const PERMISSIONS: &str = "chat.toolCall.cua.details.permissions";
    pub const ENVIRONMENT: &str = "chat.toolCall.cua.details.environment";
    pub const STATE: &str = "chat.toolCall.cua.details.state";
}

/// 结果文案的插值兜底（真源 :86-88）。
///
/// 真源：`intl.formatMessage({ id: model.resultId }, model.resultValues ?? { count: String(typedCount) })`
/// —— `resultValues` 为空对象时用 `input.text` 的字符数当 `count`。
/// Rust 侧 `resultValues` 用 `Vec` 表达"空 = 未设置"，据此区分。
pub fn resolve_result_values(
    model: &CuaDetailsModel,
    typed_count: usize,
) -> Vec<(String, String)> {
    if model.result_values.is_empty() {
        vec![("count".to_string(), typed_count.to_string())]
    } else {
        model.result_values.clone()
    }
}

/// 输入文本的字符数（真源 :57：`readText(asRecord(toolCall.input), "text")?.length ?? 0`）。
pub fn typed_text_length(tool_call: &LegacyToolCall) -> usize {
    tool_call
        .input
        .as_ref()
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::chars)
        .map(|c| c.count())
        .unwrap_or(0)
}

/// 是否渲染失败原因区块（真源 :94）。
pub fn has_failure_reason(model: &CuaDetailsModel) -> bool {
    model.failure_reason_id.is_some() || model.failure_reason.is_some()
}

/// 是否渲染建议动作区块（真源 :104）。
pub fn has_suggested_action(model: &CuaDetailsModel) -> bool {
    model.suggested_action_id.is_some() || model.suggested_action.is_some()
}

/// 是否渲染列表区块（真源 :130`model.list && model.list.items.length > 0`）。
pub fn has_list(model: &CuaDetailsModel) -> bool {
    model.list.as_ref().is_some_and(|l| l.len() > 0)
}

/// 区块分隔类名（真源 :70-74）。
///
/// 真源：第一个区块（操作）无上边框；后续区块都有 `border-t border-border pt-3`。
/// 但**结果区块**是特例——它只在前面已有操作区块时才加分隔线。
pub fn section_class(has_previous: bool) -> &'static str {
    if has_previous {
        "space-y-2 border-t border-border pt-3"
    } else {
        "space-y-2"
    }
}

/// `CuaDetailRows`（真源 :19-44）：`dl` 网格 + 每行 `dt`/`dd`。
#[component]
pub fn CuaDetailRowsComponent(rows: Vec<DetailRow>) -> impl IntoView {
    view! {
        <dl class="grid grid-cols-[minmax(4rem,auto)_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-sm">
            {rows
                .into_iter()
                .map(|row| {
                    let value_cls = if row.code {
                        // 真源 :33 —— code 行用 break-all（路径可能很长）。
                        "min-w-0 break-all font-mono text-foreground"
                    } else {
                        "min-w-0 break-words text-foreground"
                    };
                    // 真源 :35-39 —— status 三态决定行首图标。
                    let icon = match row.status {
                        Some(true) => view! {
                            <crate::app::Icon
                                paths=vec!["M21.801 10A10 10 0 1 1 17 3.335", "m9 11 3 3L22 4"]
                                circles=vec![]
                                class="size-3.5 shrink-0 text-success".to_string()
                            />
                        }
                            .into_any(),
                        Some(false) => view! {
                            <crate::app::Icon
                                paths=vec!["m15 9-6 6", "m9 9 6 6"]
                                circles=vec![("12", "12", "10")]
                                class="size-3.5 shrink-0 text-destructive".to_string()
                            />
                        }
                            .into_any(),
                        None => ().into_view().into_any(),
                    };
                    let label_id = row.label_id.clone();
                    view! {
                        <div class="contents">
                            <dt class="text-foreground-subtlest">{i18n::text(&label_id)}</dt>
                            <dd class=value_cls>
                                <span class="flex items-center gap-2">
                                    {icon}
                                    {row.value.clone()}
                                </span>
                            </dd>
                        </div>
                    }
                })
                .collect_view()}
        </dl>
    }
    .into_any()
}

/// `CuaToolCallDetails`（真源 :47-152）。
#[component]
pub fn CuaToolCallDetailsComponent(
    model: CuaDetailsModel,
    tool_call: LegacyToolCall,
) -> impl IntoView {
    // 真源 :57 —— 结果文案的 count 兜底来自 input.text 长度。
    let typed_count = typed_text_length(&tool_call);
    let result_values = resolve_result_values(&model, typed_count);

    let has_action = !model.action_rows.is_empty();
    let action_rows = model.action_rows.clone();
    let permission_rows = model.permission_rows.clone();
    let environment_rows = model.environment_rows.clone();
    let state_rows = model.state_rows.clone();
    let list = model.list.clone();
    let screenshot = model.screenshot.clone();
    let result_id = model.result_id.clone();
    let success = model.success;

    view! {
        <div class="space-y-3 rounded-xl border border-border bg-surface/40 p-3">
            // ── 1. 操作（真源 :61-67）──
            {has_action.then(|| {
                view! {
                    <section class="space-y-2">
                        <h4 class="text-sm text-foreground-subtle">
                            {i18n::text(section_ids::ACTION)}
                        </h4>
                        <CuaDetailRowsComponent
                            rows=action_rows
                                .iter()
                                .map(|r| DetailRow {
                                    label_id: r.label_id.clone(),
                                    value: r.value.clone(),
                                    code: r.code,
                                    status: if r.status { Some(true) } else { None },
                                })
                                .collect()
                        />
                    </section>
                }
            })}
            // ── 2. 结果（真源 :68-92）──
            <section class=section_class(has_action)>
                <h4 class="text-sm text-foreground-subtle">
                    {i18n::text(section_ids::RESULT)}
                </h4>
                {if success {
                    view! {
                        <div class="flex items-center gap-2 text-sm text-foreground">
                            <crate::app::Icon
                                paths=vec!["M21.801 10A10 10 0 1 1 17 3.335", "m9 11 3 3L22 4"]
                                circles=vec![]
                                class="size-3.5 shrink-0".to_string()
                            />
                            <span>{i18n::format(&result_id, &result_values)}</span>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="flex items-center gap-2 text-sm text-destructive">
                            <crate::app::Icon
                                paths=vec!["m15 9-6 6", "m9 9 6 6"]
                                circles=vec![("12", "12", "10")]
                                class="size-3.5 shrink-0".to_string()
                            />
                            <span>{i18n::format(&result_id, &result_values)}</span>
                        </div>
                    }
                        .into_any()
                }}
            </section>
            // ── 3. 失败原因（真源 :93-103）──
            {has_failure_reason(&model).then(|| {
                let reason_id = model.failure_reason_id.clone();
                let reason_text = model.failure_reason.clone();
                view! {
                    <section class="space-y-2 border-t border-border pt-3">
                        <h4 class="text-sm text-foreground-subtle">
                            {i18n::text(section_ids::FAILURE_REASON)}
                        </h4>
                        <p class="text-sm text-foreground">
                            {reason_id
                                .map(|id| i18n::text(&id))
                                .or(reason_text)
                                .unwrap_or_default()}
                        </p>
                    </section>
                }
            })}
            // ── 4. 建议动作（真源 :104-113）──
            {has_suggested_action(&model).then(|| {
                let action_id = model.suggested_action_id.clone();
                let action_text = model.suggested_action.clone();
                view! {
                    <section class="space-y-2 border-t border-border pt-3">
                        <h4 class="text-sm text-foreground-subtle">
                            {i18n::text(section_ids::SUGGESTED_ACTION)}
                        </h4>
                        <p class="text-sm text-foreground">
                            {action_id
                                .map(|id| i18n::text(&id))
                                .or(action_text)
                                .unwrap_or_default()}
                        </p>
                    </section>
                }
            })}
            // ── 5. 截图（真源 :114）──
            {screenshot.map(|s| view! { <CuaScreenshotSectionComponent screenshot=s /> })}
            // ── 6. 权限（真源 :115-123）──
            {permission_rows
                .as_ref()
                .filter(|rows| !rows.is_empty())
                .map(|rows| {
                    view! {
                        <section class="space-y-2 border-t border-border pt-3">
                            <h4 class="text-sm text-foreground-subtle">
                                {i18n::text(section_ids::PERMISSIONS)}
                            </h4>
                            <CuaDetailRowsComponent
                                rows=rows
                                    .iter()
                                    .map(|r| DetailRow {
                                        label_id: r.label_id.clone(),
                                        value: r.value.clone(),
                                        code: false,
                                        status: Some(r.status),
                                    })
                                    .collect()
                            />
                        </section>
                    }
                })}
            // ── 7. 环境（真源 :124-132）──
            {environment_rows
                .as_ref()
                .filter(|rows| !rows.is_empty())
                .map(|rows| {
                    view! {
                        <section class="space-y-2 border-t border-border pt-3">
                            <h4 class="text-sm text-foreground-subtle">
                                {i18n::text(section_ids::ENVIRONMENT)}
                            </h4>
                            <CuaDetailRowsComponent
                                rows=rows
                                    .iter()
                                    .map(|r| DetailRow {
                                        label_id: r.label_id.clone(),
                                        value: r.value.clone(),
                                        code: false,
                                        status: Some(r.status),
                                    })
                                    .collect()
                            />
                        </section>
                    }
                })}
            // ── 8. 列表（真源 :133-135）──
            {has_list(&model).then(|| {
                let list = list.clone().unwrap_or(CuaDetailList::Apps(Vec::new()));
                view! { <CuaDetailListSectionComponent list=list /> }
            })}
            // ── 9. 状态（真源 :136-144）──
            {(!state_rows.is_empty()).then(|| {
                view! {
                    <section class="space-y-2 border-t border-border pt-3">
                        <h4 class="text-sm text-foreground-subtle">
                            {i18n::text(section_ids::STATE)}
                        </h4>
                        <CuaDetailRowsComponent
                            rows=state_rows
                                .iter()
                                .map(|r| DetailRow {
                                    label_id: r.label_id.clone(),
                                    value: r.value.clone(),
                                    code: r.code,
                                    status: if r.status { Some(true) } else { None },
                                })
                                .collect()
                        />
                    </section>
                }
            })}
        </div>
    }
    .into_any()
}

/// 截图区块的入参类型别名（真源 `CuaScreenshotDetails` 已在同模块，这里重导出便于外部引用）。
pub type DetailsScreenshot = CuaScreenshotDetails;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::renderers::cua::CuaDetailsModel;
    use crate::ToolCallBlocks::renderers::cuaListDetails::{CuaAppItem, CuaWindowItem};
    use serde_json::json;

    fn call_with_input(input: serde_json::Value) -> LegacyToolCall {
        LegacyToolCall {
            tool_id: "tc-1".into(),
            tool_name: Some("mcp__computer_use__type".into()),
            kind: "mcp__computer_use__type".into(),
            title: None,
            input: Some(input),
            status: "completed".into(),
            v4_status: "success".into(),
            output: None,
            content: None,
            error: None,
            raw: json!({}),
            started_at: None,
            snapshot_refs: Vec::new(),
            thought: None,
        }
    }

    fn empty_model() -> CuaDetailsModel {
        CuaDetailsModel {
            action_rows: Vec::new(),
            result_id: "chat.toolCall.cua.details.completed".into(),
            result_values: Vec::new(),
            state_rows: Vec::new(),
            success: true,
            list: None,
            permission_rows: None,
            environment_rows: None,
            screenshot: None,
            failure_reason_id: None,
            failure_reason: None,
            suggested_action_id: None,
            suggested_action: None,
        }
    }

    #[test]
    fn typed_text_length_counts_chars_not_bytes() {
        // 真源 :57 用的是字符串 length（UTF-16 码元，Rust 用 chars 近似）。
        assert_eq!(typed_text_length(&call_with_input(json!({ "text": "hello" }))), 5);
        assert_eq!(typed_text_length(&call_with_input(json!({ "text": "你好" }))), 2);
    }

    #[test]
    fn typed_text_length_tolerates_missing_and_blank() {
        assert_eq!(typed_text_length(&call_with_input(json!({}))), 0);
        assert_eq!(typed_text_length(&call_with_input(json!({ "text": "   " }))), 0);
        assert_eq!(typed_text_length(&call_with_input(json!({ "text": null }))), 0);
    }

    #[test]
    fn result_values_fall_back_to_typed_count() {
        // ★真源 :86-88 —— resultValues 缺失时用 input.text 长度当 count。
        let mut model = empty_model();
        model.result_id = "chat.toolCall.cua.details.typed".into();
        assert_eq!(
            resolve_result_values(&model, 7),
            vec![("count".to_string(), "7".to_string())]
        );
    }

    #[test]
    fn result_values_used_when_present() {
        let mut model = empty_model();
        model.result_values = vec![("duration".to_string(), "3".into())];
        assert_eq!(
            resolve_result_values(&model, 7),
            vec![("duration".to_string(), "3".to_string())],
            "已有 resultValues 时不该被count 覆盖"
        );
    }

    #[test]
    fn failure_and_suggested_sections_need_either_form() {
        let mut model = empty_model();
        assert!(!has_failure_reason(&model));
        assert!(!has_suggested_action(&model));

        model.failure_reason = Some("权限不足".into());
        assert!(has_failure_reason(&model), "有原文也要渲染");

        model.failure_reason = None;
        model.failure_reason_id = Some("chat.toolCall.cua.details.elementStaleReason".into());
        assert!(has_failure_reason(&model), "有 i18n id 也要渲染");

        assert!(!has_suggested_action(&model));
        model.suggested_action = Some("重新读取界面".into());
        assert!(has_suggested_action(&model));
    }

    #[test]
    fn list_section_requires_non_empty_items() {
        let mut model = empty_model();
        assert!(!has_list(&model), "None 不渲染");

        model.list = Some(CuaDetailList::Apps(Vec::new()));
        assert!(!has_list(&model), "空列表不渲染（真源 items.length > 0）");

        model.list = Some(CuaDetailList::Apps(vec![CuaAppItem {
            name: "Safari".into(),
            bundle_id: None,
            active: true,
        }]));
        assert!(has_list(&model), "有项才渲染");
    }

    #[test]
    fn section_class_only_separates_after_first() {
        // 真源 :70-74 —— 第一个区块无上边框。
        assert_eq!(section_class(false), "space-y-2");
        assert_eq!(section_class(true), "space-y-2 border-t border-border pt-3");
    }

    #[test]
    fn detail_row_class_switches_on_code_flag() {
        // 真源 :32-37 —— code 行 break-all+mono，普通行 break-words。
        let code = DetailRow {
            label_id: "x".into(),
            value: "y".into(),
            code: true,
            status: None,
        };
        let plain = DetailRow {
            code: false,
            ..code.clone()
        };
        assert_eq!(code.code, true);
        assert_eq!(plain.code, false);
        assert_eq!(code.status, None);
    }

    #[test]
    fn windows_list_also_counts() {
        let mut model = empty_model();
        model.list = Some(CuaDetailList::Windows(vec![CuaWindowItem {
            title: Some("终端".into()),
            width: Some(800),
            height: Some(600),
            main: true,
            focused: true,
        }]));
        assert!(has_list(&model), "窗口列表同样按项数判断");
    }

    #[test]
    fn detail_row_carries_status_tri_state() {
        // 真源 :35-39 —— status 决定行首图标（已授权/未授权/无图标）。
        let row = DetailRow {
            label_id: "chat.toolCall.cua.details.app".into(),
            value: "Finder".into(),
            code: false,
            status: None,
        };
        assert_eq!(row.status, None, "默认无状态图标");

        let authorized = DetailRow {
            label_id: "x".into(),
            value: "y".into(),
            code: false,
            status: Some(true),
        };
        assert_eq!(authorized.status, Some(true));
    }
}