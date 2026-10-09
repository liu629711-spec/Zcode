//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/list-models.tsx`（430 行）。
//!
//! ListModels 的聊天卡（真源 :243-252）：这个工具名不在 shared 的已知工具表里，
//! 通用路径是 `FallbackToolCallBlock`，它会把模型面的 `<models>` 文本原样摊开——
//! 那段文本每行都以 providerId 开头（可能是 UUID 等长标识），「当前」藏在行尾方括号里。
//! 卡只回答三件事：**有哪些、来自哪里、哪个是当前**。每行的档位表与规范 id 归 tooltip；
//! **providerId 一个字符都不上屏**。
//!
//! ★真源 :359-361 的分支纪律：失败态**不**退回兜底卡（那张卡会摊开错误 JSON），
//! 而这里真正要说的是「这个会话读不到模型目录」——它与「一个模型也没有」是两回事，
//! 卡上既不画列表也不说那句空话。三条出口：
//! - `result === null && failed` → 扁平行（只有种类词 + 状态词，`primaryText` 是 null）；
//! - `result === null` 其余 → 交回 fallback 卡（保住自己的 Cpu 图标）；
//! - 有结果 → 正常卡，`hasDetails = modelCount > 0`（0 个时摘要行就是那句空话，
//!   没有可展开内容——空卡体比没有卡体更难读）。
//!
//! **裁剪注明**：
//! - `providerName` 查找函数（真源 :256 `useWorkflowSubagentModelProviderName`）由调用方注入，
//!   形态同 `subagent_model_label.rs` 的 `Option<&dyn Fn(&str) -> Option<String>>`。
//!   宿主通道缺席时传 `None`，组名退回 `providerLabel` → 「模型供应商」这个词，
//!   **仍绝不回 providerId**（真源那条 UUID 的理由照旧成立）。
//! - `kindLabelOverride`（真源 :371/:409）与 `canToggle/forceOpen` 走真源缺省。

use std::sync::Arc;

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::toolResultDisplay::{ListModelsDisplay, ToolResultDisplay, read_tool_result_display};
use super::super::i18n;
use super::fallback::{FallbackBlockProps, FallbackToolCallBlock};
use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;
use crate::ToolCallBlocks::toolDisplay::ToolDisplayModel;
use crate::components::subagent_model_label::{
    model_provider_family_label, resolve_model_provider_family_id_by_provider_id,
    thought_level_label_id,
};
use crate::components::workflowIcons::icon_cpu;

/// providerId → provider 名的查找函数（真源 :24 的 `ProviderNameLookup`）。
pub type ProviderNameLookup<'a> = Option<&'a dyn Fn(&str) -> Option<String>>;

/// 真源 :16 —— `Cpu className="size-4 shrink-0 text-foreground-subtle"`。
pub const LIST_MODELS_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 卡内的一行模型（真源 :26-35 的 `ListModelsEntryView`）。
#[derive(Debug, Clone, PartialEq)]
pub struct ListModelsRow {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub provider_label: Option<String>,
    pub reasoning_levels: Vec<String>,
    pub default_reasoning_level: Option<String>,
    pub context_window: Option<f64>,
    pub disabled_reason: Option<String>,
}

/// 真源 :37-41 的读取结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ListModelsResult {
    pub current: Option<String>,
    pub models: Vec<ListModelsRow>,
    pub truncated: bool,
}

fn read_trimmed_string(value: &Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// `readStringArray`（真源 :55-60）：非数组给空表，数组里的非字符串项丢掉。
fn read_string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// `parseJsonCandidate`（真源 :62-75）：与 list-saved-workflows 同款单发探针。
fn parse_json_candidate(value: &Value) -> Option<Value> {
    let Some(s) = value.as_str() else {
        return Some(value.clone());
    };
    let trimmed = s.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    serde_json::from_str::<Value>(trimmed).ok()
}

/// `readResultRecord`（真源 :77-111）：JSON 探针。必须是带 `models` 数组的对象；
/// 一条 id/providerId/modelId 三者任一为空的行被跳过（真源 :91-93），不打挂整张目录。
pub fn read_result_record(value: &Value) -> Option<ListModelsResult> {
    let normalized = parse_json_candidate(value)?;
    let models = normalized.get("models")?.as_array()?;

    let rows: Vec<ListModelsRow> = models
        .iter()
        .filter(|entry| entry.is_object())
        .filter_map(|entry| {
            let id = read_trimmed_string(entry.get("id").unwrap_or(&Value::Null))?;
            let provider_id =
                read_trimmed_string(entry.get("providerId").unwrap_or(&Value::Null))?;
            let model_id = read_trimmed_string(entry.get("modelId").unwrap_or(&Value::Null))?;
            Some(ListModelsRow {
                id,
                provider_id,
                model_id,
                provider_label: read_trimmed_string(
                    entry.get("providerLabel").unwrap_or(&Value::Null),
                ),
                reasoning_levels: read_string_array(
                    entry.get("reasoningLevels").unwrap_or(&Value::Null),
                ),
                default_reasoning_level: read_trimmed_string(
                    entry.get("defaultReasoningLevel").unwrap_or(&Value::Null),
                ),
                // 真源 :101 —— `typeof entry.contextWindow === "number"`，字符串形态不算。
                context_window: entry
                    .get("contextWindow")
                    .filter(|v| v.is_number())
                    .and_then(Value::as_f64),
                disabled_reason: read_trimmed_string(
                    entry.get("disabledReason").unwrap_or(&Value::Null),
                ),
            })
        })
        .collect();

    Some(ListModelsResult {
        current: read_trimmed_string(normalized.get("current").unwrap_or(&Value::Null)),
        models: rows,
        // 真源 :109 —— 只有显式 true 才算被裁过。
        truncated: normalized.get("truncated").and_then(Value::as_bool) == Some(true),
    })
}

/// `readListModelsResult`（真源 :119-148）：display 通道优先，legacy JSON 探针兜老会话。
///
/// 一个都读不出来回 `None`——「这台机器没有模型」与「读不懂这次结果」必须可分辨
/// （真源 :113-118）。
pub fn read_list_models_result(
    raw: &Value,
    output: Option<&Value>,
) -> Option<ListModelsResult> {
    if let Some(ToolResultDisplay::ListModels(display)) = read_tool_result_display(raw) {
        return Some(rows_from_display(&display));
    }
    let raw_record = raw.is_object();
    let pick = |key: &str| -> Option<Value> {
        if raw_record {
            raw.get(key).cloned()
        } else {
            None
        }
    };
    [
        output.cloned(),
        pick("rawOutput"),
        pick("output"),
        pick("result"),
    ]
    .into_iter()
    .flatten()
    .find_map(|candidate| read_result_record(&candidate))
}

fn rows_from_display(display: &ListModelsDisplay) -> ListModelsResult {
    ListModelsResult {
        current: display.current.clone(),
        models: display
            .models
            .iter()
            .map(|model| ListModelsRow {
                id: model.id.clone(),
                provider_id: model.provider_id.clone(),
                model_id: model.model_id.clone(),
                provider_label: model.provider_label.clone(),
                reasoning_levels: model.reasoning_levels.clone(),
                default_reasoning_level: model.default_reasoning_level.clone(),
                context_window: model.context_window,
                disabled_reason: model.disabled_reason.clone(),
            })
            .collect(),
        truncated: display.truncated == Some(true),
    }
}

/// `listModelsGroupName`（真源 :150-176）：组名 = provider 的**名字**，与模型菜单同一条规则。
///
/// 内置家族取家族名 → 载荷的 providerLabel → 会话清单里的名字 → 「模型供应商」这个词本身。
/// ★**永远不回 providerId**：团队套餐的它是一个 UUID，摆上屏幕等于让用户先跳过 36 个字符
/// 才看见模型名。两处「名字等于 id」的退回（providerLabel 与清单查询）都按没查到处理。
pub fn list_models_group_name(
    provider_id: &str,
    provider_label: Option<&str>,
    provider_name: ProviderNameLookup<'_>,
) -> String {
    if let Some(family_id) = resolve_model_provider_family_id_by_provider_id(provider_id) {
        if let Some(label) = model_provider_family_label(family_id) {
            return label.to_string();
        }
    }
    if let Some(label) = provider_label.map(str::trim).filter(|l| !l.is_empty()) {
        if label != provider_id {
            return label.to_string();
        }
    }
    if let Some(resolved) = provider_name.and_then(|lookup| lookup(provider_id)) {
        let resolved = resolved.trim();
        if !resolved.is_empty() && resolved != provider_id {
            return resolved.to_string();
        }
    }
    i18n::text("chat.toolCall.workflow.models.provider")
}

/// `formatContextWindow`（真源 :178-191）：千位以下原样，百万以下取整到 K，
/// 再往上到 M 且**只在有小数时**留一位。这一列是给人扫一眼比大小的，
/// 不是给人核对精确 token 数的。
pub fn format_context_window(context_window: f64) -> String {
    if context_window < 1_000.0 {
        // JS `String(250)`；整数值不带小数点。
        return js_number_string(context_window);
    }
    if context_window < 1_000_000.0 {
        // 真源 `Math.round`：平局朝 +∞。
        let k = (context_window / 1_000.0 + 0.5).floor();
        return format!("{}K", k as i64);
    }
    let millions = context_window / 1_000_000.0;
    if millions.fract() == 0.0 {
        format!("{}M", millions as i64)
    } else {
        format!("{:.1}M", millions)
    }
}

fn js_number_string(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// `levelWord`（真源 :193-197）：档位词与思考控件同一张表；
/// 表里没有的值原样显示（那是 provider 自定义的档位名）。
pub fn level_word(level: &str) -> String {
    match thought_level_label_id(level) {
        None => level.to_string(),
        Some(id) => i18n::text(id),
    }
}

/// `listModelsRowTooltip`（真源 :200-220）：第一行档位表，换行后是规范 id。
/// 规范 id 是给机器回填 `subagent_model` 用的，它只该住在这里。
pub fn list_models_row_tooltip(model: &ListModelsRow) -> String {
    let levels_line = if model.reasoning_levels.is_empty() {
        i18n::text("chat.toolCall.workflow.models.noLevels")
    } else {
        let levels = model
            .reasoning_levels
            .iter()
            .map(|level| level_word(level))
            .collect::<Vec<_>>()
            .join(" · ");
        match &model.default_reasoning_level {
            None => i18n::format(
                "chat.toolCall.workflow.models.levelsNoDefault",
                &[("levels".to_string(), levels)],
            ),
            Some(default) => i18n::format(
                "chat.toolCall.workflow.models.levels",
                &[
                    ("default".to_string(), level_word(default)),
                    ("levels".to_string(), levels),
                ],
            ),
        }
    };
    format!("{levels_line}\n{}", model.id)
}

/// 一行的组归属（真源 :227-241）：按 providerId 分组，**保持首次出现的顺序**——
/// 目录的顺序是宿主注册表的顺序，卡不重排。
pub fn group_models_by_provider(models: &[ListModelsRow]) -> Vec<(String, Vec<usize>)> {
    let mut order: Vec<String> = Vec::new();
    let mut by_id: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, model) in models.iter().enumerate() {
        match order.iter().position(|id| *id == model.provider_id) {
            Some(slot) => by_id[slot].1.push(index),
            None => {
                order.push(model.provider_id.clone());
                by_id.push((model.provider_id.clone(), vec![index]));
            }
        }
    }
    by_id
}

/// 折叠行主文本（真源 :287-294）：0 个说那句空话，否则计数。
pub fn list_models_primary_text(result: Option<&ListModelsResult>) -> String {
    let count = result.map(|r| r.models.len()).unwrap_or(0);
    if count == 0 {
        return i18n::text("chat.toolCall.workflow.models.empty");
    }
    let id = if count == 1 {
        "chat.toolCall.workflow.models.countOne"
    } else {
        "chat.toolCall.workflow.models.count"
    };
    i18n::format(id, &[("count".to_string(), count.to_string())])
}

/// 行内模型名的类名（真源 :320-325）：禁用行更弱一档。
pub fn model_name_class(disabled: bool) -> &'static str {
    if disabled {
        "min-w-0 truncate font-mono text-ui-base text-foreground-subtlest"
    } else {
        "min-w-0 truncate font-mono text-ui-base text-foreground-subtle"
    }
}

/// 渲染前算好的一行（真源 :313-347 的每个判定）。
#[derive(Debug, Clone, PartialEq)]
struct ListModelsRowView {
    id: String,
    model_id: String,
    disabled: bool,
    is_current: bool,
    disabled_reason: Option<String>,
    /// 空串 = 这一列缺席。
    context_window_text: String,
    tooltip: String,
}

/// 渲染前算好的一组（真源 :303-350）。
#[derive(Debug, Clone, PartialEq)]
struct ListModelsGroupView {
    provider_id: String,
    group_name: String,
    rows: Vec<ListModelsRowView>,
}

/// `ListModelsToolCallBlock` 的 props。
#[derive(Clone)]
pub struct ListModelsBlockProps {
    pub tool_id: String,
    pub raw: Value,
    /// legacy 行的散文输出（真源 `toolCall.output` 是 unknown，v4 适配层压平成字符串）。
    pub output: Option<Value>,
    pub status: String,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    /// providerId → provider 名（真源 :256）；宿主通道未接线时 None。
    pub provider_name: Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
    /// 交回 fallback 卡所需（真源 :395）。
    pub legacy: LegacyToolCall,
    pub display_model: ToolDisplayModel,
    pub workspace_path: String,
}

/// `ListModelsToolCallBlock`（真源 :253-429）。
#[component]
pub fn ListModelsToolCallBlock(props: ListModelsBlockProps) -> impl IntoView {
    let result = read_list_models_result(&props.raw, props.output.as_ref());
    let is_failed = props.status == "failed";
    let icon_view: ChildrenFn = Arc::new(|| {
        view! { <span class=LIST_MODELS_TOOL_ICON_CLASS>{icon_cpu()}</span> }.into_any()
    });

    let kind_label = i18n::text(if props.is_running {
        "chat.toolCall.workflow.models.listing"
    } else {
        "chat.toolCall.workflow.models.listed"
    });

    // ── 出口一：失败且读不出目录 → 扁平行（真源 :359-391）──
    let no_body: Option<ChildrenFn> = None;
    if result.is_none() && is_failed {
        return view! {
            <ToolLayoutComponent
                props=ToolLayoutProps {
                    tool_id: props.tool_id.clone(),
                    icon: None,
                    show_icon: Some(props.show_icon),
                    // 真源 :369-370 —— 这里没有可展开内容，两个开关都关死。
                    can_toggle: Some(false),
                    force_open: Some(false),
                    kind_label: Some(kind_label),
                    source_label: props.source_label.clone(),
                    // 真源 :373 —— 失败时摘要行只有种类词与状态词：
                    // 计数与那句「没有配置模型」在这里都是谎话。
                    primary_text: None,
                    status_label: props.status_label.clone(),
                    status_tooltip: props.error_text.clone(),
                    show_failure_status: Some(true),
                    is_running: Some(props.is_running),
                    title: props.title.clone(),
                    ..Default::default()
                }
                icon_view=Some(icon_view)
                // 这一条出口没有卡体（真源 :365-380 不传 renderContent）。
                // view! 的属性位不收 turbofish，故先在外部把 None 的类型绑好。
                render_content=no_body
            />
            <ToolSnapshotFieldNoticeComponent
                props=ToolSnapshotFieldNoticeProps {
                    refs: props.snapshot_refs.clone(),
                    tool_id: props.tool_id.clone(),
                    on_load_full_tool_call_fields: props.on_load_full_tool_call_fields.clone(),
                }
            />
        }
        .into_any();
    }

    // ── 出口二：读不出结构化结果（老会话、降级路径）→ 交回通用卡（真源 :393-396）──
    let Some(result) = result else {
        return view! {
            <FallbackToolCallBlock
                props=FallbackBlockProps {
                    tool_id: props.tool_id.clone(),
                    kind: props.legacy.kind.clone(),
                    title: props.title.clone(),
                    status: Some(props.status.clone()),
                    raw_tool_call: props.raw.clone(),
                    is_running: props.is_running,
                    status_label: props.status_label.clone(),
                    error_text: props.error_text.clone(),
                    source_label: props.source_label.clone(),
                    show_icon: props.show_icon,
                    icon_override: Some(Arc::new(|| {
                        view! {
                            <span class=LIST_MODELS_TOOL_ICON_CLASS>{icon_cpu()}</span>
                        }
                        .into_any()
                    }) as ChildrenFn),
                    has_inline_preview: false,
                    hide_raw_fallback: false,
                    summary_only: false,
                    summary_text_override: None,
                    kind_label_override: None,
                    snapshot_refs: props.snapshot_refs.clone(),
                    on_load_full_tool_call_fields: props.on_load_full_tool_call_fields.clone(),
                    legacy: props.legacy.clone(),
                    display_model: props.display_model.clone(),
                    workspace_path: props.workspace_path.clone(),
                }
            />
        }
        .into_any();
    };

    // ── 出口三：正常目录卡（真源 :398-428）──
    let model_count = result.models.len();
    let has_details = model_count > 0;
    let primary_text = list_models_primary_text(Some(&result));
    let groups = group_models_by_provider(&result.models);
    let current = result.current.clone();
    let truncated = result.truncated;

    // 组名与每行的展示件先算好（children 先于属性求值）。
    // Arc<dyn Fn> 不能直接强转成 `&dyn Fn(&str)`（缺 HRTB），包一层本地闭包再取引用。
    let provider_lookup_fn = |provider_id: &str| -> Option<String> {
        props
            .provider_name
            .as_ref()
            .and_then(|lookup| lookup(provider_id))
    };
    let provider_lookup: ProviderNameLookup<'_> = props
        .provider_name
        .is_some()
        .then_some(&provider_lookup_fn as &dyn Fn(&str) -> Option<String>);
    let group_views: Vec<ListModelsGroupView> = groups
        .iter()
        .map(|(provider_id, indexes)| {
            // 真源 :308 —— 组名取这一组里**第一个带 providerLabel** 的模型。
            let first_label = indexes
                .iter()
                .filter_map(|i| result.models[*i].provider_label.clone())
                .next();
            let group_name =
                list_models_group_name(provider_id, first_label.as_deref(), provider_lookup);
            let rows = indexes
                .iter()
                .map(|i| {
                    let model = &result.models[*i];
                    ListModelsRowView {
                        id: model.id.clone(),
                        model_id: model.model_id.clone(),
                        disabled: model.disabled_reason.is_some(),
                        is_current: current.as_deref() == Some(model.id.as_str()),
                        disabled_reason: model.disabled_reason.clone(),
                        // 空串 = 这一列缺席（真源 :342 的 `=== undefined` 才不画）。
                        context_window_text: model.context_window.map(format_context_window)
                            .unwrap_or_default(),
                        tooltip: list_models_row_tooltip(model),
                    }
                })
                .collect();
            ListModelsGroupView {
                provider_id: provider_id.clone(),
                group_name,
                rows,
            }
        })
        .collect();
    let truncated_label = i18n::text("chat.toolCall.workflow.models.truncated");

    let render_content = move || {
        let current_label = i18n::text("chat.toolCall.workflow.models.current");
        let groups_for_body = group_views.clone();
        view! {
            <div class="mb-2 space-y-2" data-model-list="true">
                {groups_for_body
                    .into_iter()
                    .map(|group| {
                        // 真源 :304 的 `key={group.providerId}` 是 React 的列表身份，不落 DOM。
                        view! {
                            <div class="min-w-0 space-y-0.5">
                                <div class="text-ui-xs text-foreground-subtlest">{group.group_name}</div>
                                {group.rows
                                    .into_iter()
                                    .map(|row| view! {
                                        // 规范 id 与档位表都只住在这里（真源 :200-202 的分工）。
                                        <div
                                            class="flex min-w-0 items-baseline gap-x-2"
                                            title=row.tooltip
                                            data-model-id=row.id
                                        >
                                            <span class=model_name_class(row.disabled)>{row.model_id}</span>
                                            {row.is_current.then(|| view! {
                                                <span
                                                    class="shrink-0 text-ui-xs text-foreground-subtlest"
                                                    data-model-current="true"
                                                >
                                                    {current_label.clone()}
                                                </span>
                                            })}
                                            {row.disabled_reason.map(|reason| view! {
                                                <span class="min-w-0 truncate text-ui-xs text-warning">{reason}</span>
                                            })}
                                            {(!row.context_window_text.is_empty()).then(|| view! {
                                                <span class="ml-auto shrink-0 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
                                                    {row.context_window_text}
                                                </span>
                                            })}
                                        </div>
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })
                    .collect_view()}
                {truncated.then(|| view! {
                    <p class="text-ui-sm text-foreground-subtlest">{truncated_label.clone()}</p>
                })}
            </div>
        }
        .into_any()
    };

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(has_details),
                force_open: Some(false),
                kind_label: Some(kind_label),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                status_label: props.status_label.clone(),
                status_tooltip: props.error_text.clone(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(icon_view)
            render_content=has_details.then(|| {
                let body = render_content.clone();
                Arc::new(move || body()) as ChildrenFn
            })
        />
        <ToolSnapshotFieldNoticeComponent
            props=ToolSnapshotFieldNoticeProps {
                refs: props.snapshot_refs.clone(),
                tool_id: props.tool_id.clone(),
                on_load_full_tool_call_fields: props.on_load_full_tool_call_fields.clone(),
            }
        />
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(id: &str, provider_id: &str, model_id: &str) -> ListModelsRow {
        ListModelsRow {
            id: id.into(),
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            provider_label: None,
            reasoning_levels: vec![],
            default_reasoning_level: None,
            context_window: None,
            disabled_reason: None,
        }
    }

    #[test]
    fn provider_id_never_reaches_the_screen() {
        // ★真源 :150-156 的硬纪律：组名四跳之后落到「模型供应商」这个词，绝不回 id。
        let uuidish = "0f0e1c6a-0000-4000-8000-abcdefabcdef";
        assert_eq!(
            list_models_group_name(uuidish, None, None),
            i18n::text("chat.toolCall.workflow.models.provider")
        );
        // 内置家族取家族名。
        assert_eq!(
            list_models_group_name("account:zai-individual-coding-plan", None, None).as_str(),
            "Z.ai"
        );
        assert_eq!(
            list_models_group_name("account:bigmodel-start-plan", None, None).as_str(),
            "BigModel"
        );
        // providerLabel 等于 providerId 时按「没查到」处理（那是清单的退回值，不是名字）。
        assert_eq!(
            list_models_group_name("acme", Some("acme"), None),
            i18n::text("chat.toolCall.workflow.models.provider")
        );
        // 正常 label 优先于清单。
        assert_eq!(
            list_models_group_name("acme", Some(" Acme Lab "), None).as_str(),
            "Acme Lab"
        );
    }

    #[test]
    fn session_catalog_name_is_the_third_source() {
        // 真源 :171-174 —— 载荷没带 label 时才查会话清单。
        let lookup = |_: &str| Some("深度求索".to_string());
        let provider: &dyn Fn(&str) -> Option<String> = &lookup;
        assert_eq!(list_models_group_name("deepseek", None, Some(provider)).as_str(), "深度求索");
        // 清单查到的名字等于 id → 仍算没查到。
        let same_as_id = |_: &str| Some("deepseek".to_string());
        let provider2: &dyn Fn(&str) -> Option<String> = &same_as_id;
        assert_eq!(
            list_models_group_name("deepseek", None, Some(provider2)),
            i18n::text("chat.toolCall.workflow.models.provider")
        );
        // 查不到（None）→ 同一个词。
        let none_lookup = |_: &str| None;
        let provider3: &dyn Fn(&str) -> Option<String> = &none_lookup;
        assert_eq!(
            list_models_group_name("deepseek", None, Some(provider3)),
            i18n::text("chat.toolCall.workflow.models.provider")
        );
    }

    #[test]
    fn context_window_three_tiers() {
        // 真源 :178-191 —— 比大小用的列，不是核对精确值用的。
        assert_eq!(format_context_window(250.0), "250");
        assert_eq!(format_context_window(999.0), "999");
        assert_eq!(format_context_window(1_000.0), "1K");
        assert_eq!(format_context_window(128_000.0), "128K");
        // Math.round 平局朝 +∞：1500/1000 = 1.5 → 2K
        assert_eq!(format_context_window(1_500.0), "2K");
        assert_eq!(format_context_window(1_499.0), "1K");
        assert_eq!(format_context_window(1_000_000.0), "1M");
        assert_eq!(format_context_window(400_000.0), "400K");
        // 有小数才留一位
        assert_eq!(format_context_window(1_500_000.0), "1.5M");
        assert_eq!(format_context_window(2_000_000.0), "2M");
    }

    #[test]
    fn level_word_falls_through_for_unknown_levels() {
        // 真源 :193-197 —— 表里的走思考控件那张表，表外的是 provider 自定义档位名，原样显示。
        assert_eq!(level_word("unknown-tier-xyz"), "unknown-tier-xyz");
        assert_eq!(level_word("low"), "低");
        assert_eq!(level_word("high"), "高");
        // 查表大小写与首尾空白不敏感（thought_level_label_id 自己归一）。
        assert_eq!(level_word("  LOW "), "低");
    }

    #[test]
    fn tooltip_is_levels_then_canonical_id() {
        // 真源 :200-220 —— 第一行档位表（档位词与思考控件同一张表），换行后是规范 id。
        // 规范 id 是给机器回填 `subagent_model` 用的，它只该住在这里。
        let mut m = row("acme:model-x$high", "acme", "model-x");
        assert_eq!(list_models_row_tooltip(&m), "没有思考强度档位\nacme:model-x$high");

        m.reasoning_levels = vec!["low".into(), "high".into()];
        assert_eq!(list_models_row_tooltip(&m), "思考强度：低 · 高\nacme:model-x$high");

        m.default_reasoning_level = Some("high".into());
        assert_eq!(
            list_models_row_tooltip(&m),
            "思考强度：低 · 高（默认 高）\nacme:model-x$high"
        );
    }

    #[test]
    fn grouping_keeps_first_appearance_order() {
        // 真源 :227-241 —— 目录顺序是宿主注册表的顺序，卡不重排。
        let models = vec![
            row("b:1", "beta", "1"),
            row("a:1", "alpha", "1"),
            row("b:2", "beta", "2"),
        ];
        let groups = group_models_by_provider(&models);
        let ids: Vec<&str> = groups.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["beta", "alpha"]);
        assert_eq!(groups[0].1, vec![0, 2]);
        assert_eq!(groups[1].1, vec![1]);
    }

    #[test]
    fn json_probe_needs_a_models_array() {
        assert!(read_result_record(&json!({"models": []})).is_some());
        assert!(read_result_record(&json!({})).is_none());
        assert!(read_result_record(&json!({"models": "x"})).is_none());
        assert!(read_result_record(&json!("plain")).is_none());
        // 字符串形态的 JSON 走探针（老宿主）。
        let parsed = read_result_record(&json!("{\"models\":[{\"id\":\"i\",\"providerId\":\"p\",\"modelId\":\"m\"}]}"))
            .expect("字符串 JSON");
        assert_eq!(parsed.models.len(), 1);
    }

    #[test]
    fn rows_missing_identity_keys_are_dropped_not_the_catalog() {
        let parsed = read_result_record(&json!({
            "models": [
                {"id": "i1", "providerId": "p", "modelId": "m1"},
                {"id": "  ", "providerId": "p", "modelId": "m2"},
                {"providerId": "p", "modelId": "m3"},
                {"id": "i4", "modelId": "m4"},
                {"id": "i5", "providerId": "p"},
                "not a record",
            ]
        }))
        .expect("目录");
        let kept: Vec<&str> = parsed.models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(kept, vec!["i1"], "三条缺任一身份键的行都被跳过（真源 :91-93）");
    }

    #[test]
    fn truncated_only_for_explicit_true() {
        // 真源 :109。
        for (value, expected) in [
            (json!(true), true),
            (json!(false), false),
            (json!("true"), false),
            (json!(null), false),
        ] {
            let mut payload = json!({"models": []});
            payload["truncated"] = value;
            assert_eq!(read_result_record(&payload).unwrap().truncated, expected);
        }
        assert!(!read_result_record(&json!({"models": []})).unwrap().truncated);
    }

    #[test]
    fn context_window_only_accepts_numbers() {
        // 真源 :101 —— `typeof entry.contextWindow === "number"`，字符串形态不算。
        let text_form = json!({
            "models": [{"id": "i", "providerId": "p", "modelId": "m", "contextWindow": "400000"}]
        });
        assert_eq!(read_result_record(&text_form).unwrap().models[0].context_window, None);
        let num_form = json!({
            "models": [{"id": "i", "providerId": "p", "modelId": "m", "contextWindow": 400000}]
        });
        assert_eq!(
            read_result_record(&num_form).unwrap().models[0].context_window,
            Some(400_000.0)
        );
    }

    #[test]
    fn display_channel_wins_over_the_probe() {
        let raw = json!({ "result": { "display": {
            "kind": "list_models",
            "current": "acme:gpt",
            "models": [{
                "id": "acme:gpt", "providerId": "acme", "modelId": "gpt",
                "reasoningLevels": [], "contextWindow": 128000.0,
            }],
            "truncated": true,
        } } });
        let result = read_list_models_result(&raw, Some(&json!("{\"models\":[]}")))
            .expect("display 命中");
        assert_eq!(result.current.as_deref(), Some("acme:gpt"));
        assert_eq!(result.models[0].context_window, Some(128_000.0));
        assert!(result.truncated);
    }

    #[test]
    fn empty_catalog_and_unreadable_are_different_facts() {
        // ★空目录（读得懂，只是没有）vs 读不懂（None）——后者才交回 fallback。
        let empty = json!({ "result": { "display": { "kind": "list_models", "models": [] } } });
        let result = read_list_models_result(&empty, None).expect("空目录也是读出来了");
        assert!(result.models.is_empty());
        assert_eq!(
            list_models_primary_text(Some(&result)),
            "这台机器上没有配置模型"
        );
        assert_eq!(read_list_models_result(&json!({}), Some(&json!("散文"))), None);
    }

    #[test]
    fn count_label_switches_on_one() {
        let one = ListModelsResult {
            current: None,
            models: vec![row("i", "p", "m")],
            truncated: false,
        };
        assert_eq!(list_models_primary_text(Some(&one)), "1 个模型");
        let many = ListModelsResult {
            models: vec![row("i", "p", "m"), row("j", "p", "n")],
            ..one.clone()
        };
        assert_eq!(list_models_primary_text(Some(&many)), "2 个模型");
    }

    #[test]
    fn disabled_rows_are_weaker_and_carry_the_reason() {
        // 真源 :320-341 —— 禁用行的模型名降一档（subtle → subtlest），行尾挂警示色原因。
        assert_eq!(
            model_name_class(false),
            "min-w-0 truncate font-mono text-ui-base text-foreground-subtle"
        );
        assert_eq!(
            model_name_class(true),
            "min-w-0 truncate font-mono text-ui-base text-foreground-subtlest"
        );
        // 两者的差别只有那一个词。
        assert_ne!(model_name_class(false), model_name_class(true));
    }

    #[test]
    fn icon_class_matches_source() {
        assert_eq!(LIST_MODELS_TOOL_ICON_CLASS, "size-4 flex-none text-foreground-subtle");
    }
}
