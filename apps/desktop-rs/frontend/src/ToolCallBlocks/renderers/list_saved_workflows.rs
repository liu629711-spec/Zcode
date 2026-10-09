//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/list-saved-workflows.tsx`（304 行）。
//!
//! ListSavedWorkflows 的聊天卡。为什么值得一个专用 renderer（真源 :139-148）：
//! 这个工具名没登记在 shared 的已知工具表里，通用路径是 `FallbackToolCallBlock`，
//! 而它的默认 display model 没有 inlinePreview，于是会把 `JSON.stringify(toolCall)`
//! 整包摊进聊天区——对这个工具正好是最坏情况，因为那包 JSON 就是全部工作流的
//! description / whenToUse / args 声明。
//!
//! 卡片只回答「有哪些、干什么用」；脚本本体本来就不在结果里（一次列举不该把 20 段脚本
//! 灌进上下文）。坏文件单独一行——它们是**刻意可见**的，不静默跳过。
//!
//! 结果读取顺序（真源 :102-107）：**display 通道优先**——v4 wire 上 output.text
//! 是 formatModelContent 的 XML 风格投影，下面的 JSON 探针对它永不命中；legacy JSON 探针
//! 保留，兜老会话与非 v4 宿主。一个都读不出来就交回 fallback——
//! **空列表与「读不懂」必须可分辨**，不能把解析失败画成「这个项目里没有工作流」。
//!
//! **裁剪注明**：`kindLabelOverride`（真源 :283）与 `canToggle/forceOpen` 走真源缺省。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::toolResultDisplay::{SavedWorkflowListDisplay, ToolResultDisplay, read_tool_result_display};
use super::super::i18n;
use super::fallback::{FallbackBlockProps, FallbackToolCallBlock};
use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;
use crate::ToolCallBlocks::toolDisplay::ToolDisplayModel;
use crate::components::workflowIcons::icon_library;

/// 真源 :10-12 —— `Library className="size-4 shrink-0 text-foreground-subtle"`。
pub const LIST_SAVED_WORKFLOWS_TOOL_ICON_CLASS: &str =
    "size-4 flex-none text-foreground-subtle";

/// `SavedWorkflowEntry`（真源 :26-33）——卡内的行模型。
///
/// display 通道的 `scope` / `path` 是协议必填值，legacy JSON 探针读出来的是可选值，
/// 两者都归一到这个 `Option` 形态（真源的本地 interface 同样是 optional）。
#[derive(Debug, Clone, PartialEq)]
pub struct SavedWorkflowRow {
    pub name: String,
    pub description: Option<String>,
    pub when_to_use: Option<String>,
    pub scope: Option<String>,
    pub path: Option<String>,
    pub arg_names: Vec<String>,
}

/// `InvalidSavedWorkflowEntry`（真源 :35-38）。
#[derive(Debug, Clone, PartialEq)]
pub struct InvalidSavedWorkflowRow {
    pub path: String,
    pub reason: Option<String>,
}

/// `ListSavedWorkflowsResult`（真源 :40-43）。
#[derive(Debug, Clone, PartialEq)]
pub struct ListSavedWorkflowsResult {
    pub workflows: Vec<SavedWorkflowRow>,
    pub invalid: Vec<InvalidSavedWorkflowRow>,
}

/// `readTrimmedString`（真源 :18-24）：trim 后非空才要，返回 trim 后的值。
fn read_trimmed_string(value: &Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// `parseJsonCandidate`（真源 :45-58）：非字符串原样返回；
/// trim 后不以 `{` / `[` 开头的字符串直接判「不是 JSON」（undefined）；
/// parse 失败也是 undefined。
pub fn parse_json_candidate(value: &Value) -> Option<Value> {
    let Some(s) = value.as_str() else {
        return Some(value.clone());
    };
    let trimmed = s.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    serde_json::from_str::<Value>(trimmed).ok()
}

/// `readResultRecord`（真源 :60-100）：JSON 探针 —— 必须是带 `workflows` 数组的对象，
/// 否则整份候选作废（返回 None，继续试下一个候选）。
pub fn read_result_record(value: &Value) -> Option<ListSavedWorkflowsResult> {
    let normalized = parse_json_candidate(value)?;
    let workflows = normalized.get("workflows")?.as_array()?;

    let rows: Vec<SavedWorkflowRow> = workflows
        .iter()
        .filter(|entry| entry.is_object())
        .filter_map(|entry| {
            let name = read_trimmed_string(entry.get("name").unwrap_or(&Value::Null))?;
            Some(SavedWorkflowRow {
                name,
                description: read_trimmed_string(entry.get("description").unwrap_or(&Value::Null)),
                when_to_use: read_trimmed_string(entry.get("whenToUse").unwrap_or(&Value::Null)),
                scope: read_trimmed_string(entry.get("scope").unwrap_or(&Value::Null)),
                path: read_trimmed_string(entry.get("path").unwrap_or(&Value::Null)),
                // 真源 :81 —— args 是对象时取其键名；否则空表（不是「读不动就丢这条」）。
                arg_names: entry
                    .get("args")
                    .and_then(|args| args.as_object())
                    .map(|map| map.keys().cloned().collect())
                    .unwrap_or_default(),
            })
        })
        .collect();

    // 真源 :85-97 —— `invalid` 缺席是空表；条目缺 path 的跳过。
    let invalid = normalized
        .get("invalid")
        .and_then(|list| list.as_array())
        .map(|list| {
            list.iter()
                .filter(|entry| entry.is_object())
                .filter_map(|entry| {
                    let path = read_trimmed_string(entry.get("path").unwrap_or(&Value::Null))?;
                    Some(InvalidSavedWorkflowRow {
                        path,
                        reason: read_trimmed_string(entry.get("reason").unwrap_or(&Value::Null)),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(ListSavedWorkflowsResult { workflows: rows, invalid })
}

/// display 通道 → 卡内行模型（真源 :112-127）。
fn rows_from_display(display: &SavedWorkflowListDisplay) -> ListSavedWorkflowsResult {
    ListSavedWorkflowsResult {
        workflows: display
            .workflows
            .iter()
            .map(|entry| SavedWorkflowRow {
                name: entry.name.clone(),
                description: entry.description.clone(),
                when_to_use: entry.when_to_use.clone(),
                scope: Some(entry.scope.clone()),
                path: Some(entry.path.clone()),
                arg_names: entry.arg_names.clone(),
            })
            .collect(),
        invalid: display
            .invalid
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|entry| InvalidSavedWorkflowRow {
                path: entry.path.clone(),
                reason: entry.reason.clone(),
            })
            .collect(),
    }
}

/// `readListSavedWorkflowsResult`（真源 :108-137）。
///
/// `output_text` 是 legacy 行的散文输出（真源 `toolCall.output`）。
pub fn read_list_saved_workflows_result(
    raw: &Value,
    output_text: Option<&str>,
) -> Option<ListSavedWorkflowsResult> {
    if let Some(ToolResultDisplay::SavedWorkflowList(display)) = read_tool_result_display(raw) {
        return Some(rows_from_display(&display));
    }

    // 真源 :129-135 —— 四个候选按序试：output / raw.rawOutput / raw.output / raw.result。
    // ★空表也是「读出来了」：`workflows: []` 是「这个项目里没有工作流」这件事实，
    // 与「读不懂」（None → 交回 fallback）必须可分辨。
    let raw_record = raw.is_object();
    let pick = |key: &str| -> Option<Value> {
        if raw_record {
            raw.get(key).cloned()
        } else {
            None
        }
    };
    let candidates: Vec<Value> = [
        output_text.map(|text| Value::String(text.to_string())),
        pick("rawOutput"),
        pick("output"),
        pick("result"),
    ]
    .into_iter()
    .flatten()
    .collect();

    candidates.iter().find_map(read_result_record)
}

/// 一条工作流行的 scope 标签（真源 :215-226）：
/// `global` 走镌刻小标；`project` 走词表；其余作用域值原样写出来（不是空、不是「未知」）。
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeTag {
    /// 边框小标「全局」。
    GlobalBadge,
    /// 弱文字：project 走词表，其余原样。
    Plain(String),
}

pub fn scope_tag(scope: Option<&str>) -> ScopeTag {
    match scope {
        Some("global") => ScopeTag::GlobalBadge,
        Some("project") => ScopeTag::Plain(i18n::text("chat.permission.workflow.saved.scope.project")),
        Some(other) => ScopeTag::Plain(other.to_string()),
        None => ScopeTag::Plain(String::new()),
    }
}

/// 单复数各一条 key（真源 :169-187，轻量 intl 没有 ICU 复数）。
pub fn list_saved_count_labels(result: Option<&ListSavedWorkflowsResult>) -> (String, String) {
    let workflow_count = result.map(|r| r.workflows.len()).unwrap_or(0);
    let invalid_count = result.map(|r| r.invalid.len()).unwrap_or(0);
    let count_id = if workflow_count == 1 {
        "chat.toolCall.workflow.list.countOne"
    } else {
        "chat.toolCall.workflow.list.count"
    };
    let invalid_id = if invalid_count == 1 {
        "chat.toolCall.workflow.list.invalidOne"
    } else {
        "chat.toolCall.workflow.list.invalid"
    };
    (
        i18n::format(count_id, &[("count".to_string(), workflow_count.to_string())]),
        i18n::format(invalid_id, &[("count".to_string(), invalid_count.to_string())]),
    )
}

/// 折叠行主文本（真源 :190-197）：0 条说「还没有」，否则计数。
pub fn list_saved_primary_text(result: Option<&ListSavedWorkflowsResult>) -> String {
    let (count_label, _) = list_saved_count_labels(result);
    if result.map(|r| r.workflows.is_empty()).unwrap_or(true) {
        i18n::text("chat.toolCall.workflow.list.empty")
    } else {
        count_label
    }
}

/// `ListSavedWorkflowsToolCallBlock` 的 props。
#[derive(Clone)]
pub struct ListSavedWorkflowsBlockProps {
    pub tool_id: String,
    pub kind: String,
    pub raw: Value,
    pub output_text: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    /// 读不出结构化结果时交回通用卡所需的原样载荷与 display model。
    pub legacy: LegacyToolCall,
    pub display_model: ToolDisplayModel,
    pub workspace_path: String,
}

/// `ListSavedWorkflowsToolCallBlock`（真源 :150-303）。
#[component]
pub fn ListSavedWorkflowsToolCallBlock(props: ListSavedWorkflowsBlockProps) -> impl IntoView {
    let result = read_list_saved_workflows_result(&props.raw, props.output_text.as_deref());

    // 真源 :270-273 —— 读不出结构化结果（老会话、失败、降级路径）就交回通用卡，
    // 而不是画一张空列表；但图标保住自己的 Library。
    let Some(result) = result else {
        return view! {
            <FallbackToolCallBlock
                props=FallbackBlockProps {
                    tool_id: props.tool_id.clone(),
                    kind: props.kind.clone(),
                    title: props.title.clone(),
                    status: props.status.clone(),
                    raw_tool_call: props.raw.clone(),
                    is_running: props.is_running,
                    status_label: props.status_label.clone(),
                    error_text: props.error_text.clone(),
                    source_label: props.source_label.clone(),
                    show_icon: props.show_icon,
                    icon_override: Some(std::sync::Arc::new(|| {
                        view! {
                            <span class=LIST_SAVED_WORKFLOWS_TOOL_ICON_CLASS>{icon_library()}</span>
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

    let kind_label = i18n::text(if props.is_running {
        "chat.toolCall.workflow.list.listing"
    } else {
        "chat.toolCall.workflow.list.listed"
    });
    let scope_global_label = i18n::text("chat.toolCall.workflow.scope.global");
    let empty_label = i18n::text("chat.toolCall.workflow.list.empty");
    // 计数词只在 invalid 那一行用；折叠行的主文本由 `list_saved_primary_text` 统一决定
    // （0 条走「还没有」，否则走计数）——两条路都从同一条 `list_saved_count_labels` 出。
    let (_, invalid_label) = list_saved_count_labels(Some(&result));
    let primary_text = list_saved_primary_text(Some(&result));

    let workflows = result.workflows.clone();
    let invalid = result.invalid.clone();
    let has_workflows = !workflows.is_empty();
    let has_invalid = !invalid.is_empty();
    let render_content = move || {
        let rows = workflows
            .iter()
            .map(|workflow| {
                // ★全部先在 view! 之外算好（children 先于属性求值）。
                let title = workflow
                    .path
                    .clone()
                    .unwrap_or_else(|| workflow.name.clone());
                let name = workflow.name.clone();
                let description = workflow.description.clone();
                let tag = scope_tag(workflow.scope.as_deref());
                let global_badge_label = scope_global_label.clone();
                let arg_chips = workflow
                    .arg_names
                    .iter()
                    .map(|arg_name| view! {
                        <span class="rounded-xs border border-border px-1.5 py-0.5 font-mono text-ui-xs leading-none text-foreground-subtlest">
                            {arg_name.clone()}
                        </span>
                    })
                    .collect_view();
                let has_args = !workflow.arg_names.is_empty();
                view! {
                    <div class="min-w-0 space-y-0.5">
                        <div class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
                            <span
                                class="min-w-0 truncate font-mono text-ui-base text-foreground-subtle"
                                title=title
                            >
                                {name}
                            </span>
                            {match tag {
                                ScopeTag::GlobalBadge => view! {
                                    <span
                                        data-workflow-scope-tag="global"
                                        class="shrink-0 rounded-xs border border-border px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest"
                                    >
                                        {global_badge_label}
                                    </span>
                                }
                                .into_any(),
                                ScopeTag::Plain(text) => view! {
                                    <span class="shrink-0 text-ui-xs text-foreground-subtlest">{text}</span>
                                }
                                .into_any(),
                            }}
                        </div>
                        {description.map(|text| view! {
                            <p class="min-w-0 whitespace-pre-wrap break-words text-ui-sm text-foreground-subtlest">
                                {text}
                            </p>
                        })}
                        {has_args.then(|| view! {
                            <div class="flex min-w-0 flex-wrap gap-1 pt-0.5">{arg_chips}</div>
                        })}
                    </div>
                }
            })
            .collect_view();

        let invalid_rows = invalid
            .iter()
            .map(|entry| {
                let title = entry
                    .reason
                    .clone()
                    .unwrap_or_else(|| entry.path.clone());
                let path = entry.path.clone();
                view! {
                    <p
                        class="min-w-0 truncate font-mono text-ui-xs text-foreground-subtlest"
                        title=title
                    >
                        {path}
                    </p>
                }
            })
            .collect_view();

        view! {
            <div class="mb-2 space-y-2" data-saved-workflow-list="true">
                {rows}
                {(!has_workflows).then(|| view! {
                    <p class="text-ui-sm text-foreground-subtlest">{empty_label.clone()}</p>
                })}
                {has_invalid.then(|| view! {
                    // 坏文件是刻意可见的：一圈警示描边 + 一句计数，不静默跳过。
                    <div class="space-y-0.5 rounded-lg border border-warning/40 px-2.5 py-2">
                        <p class="text-ui-sm text-warning">{invalid_label.clone()}</p>
                        {invalid_rows.clone()}
                    </div>
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
                // 真源 :281-282 —— 与 save-workflow 同样不以 hasDetails 门控。
                can_toggle: Some(true),
                force_open: Some(false),
                kind_label: Some(kind_label),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                status_label: props.status_label.clone(),
                status_tooltip: props.error_text.clone(),
                show_failure_status: Some(props.status.as_deref() == Some("failed")),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=LIST_SAVED_WORKFLOWS_TOOL_ICON_CLASS>{icon_library()}</span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content) as ChildrenFn)
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

    #[test]
    fn json_candidate_only_for_object_or_array_shapes() {
        // 真源 :45-58 —— 不以 `{` / `[` 开头的字符串一律不是候选（含 `"42"` 这类字面量）。
        assert_eq!(parse_json_candidate(&json!("{\"a\":1}")), Some(json!({"a": 1})));
        assert_eq!(parse_json_candidate(&json!("  [1,2]  ")), Some(json!([1, 2])));
        assert_eq!(parse_json_candidate(&json!("plain text")), None);
        assert_eq!(parse_json_candidate(&json!("42")), None);
        assert_eq!(parse_json_candidate(&json!("{broken")), None, "parse 失败不抛错");
        // 非字符串原样通过（再由 readResultRecord 判形状）。
        assert_eq!(parse_json_candidate(&json!({"a": 1})), Some(json!({"a": 1})));
        assert_eq!(parse_json_candidate(&Value::Null), Some(Value::Null));
    }

    #[test]
    fn record_requires_a_workflows_array() {
        assert!(read_result_record(&json!({"workflows": []})).is_some());
        assert!(read_result_record(&json!({})).is_none(), "缺 workflows 键整份候选作废");
        assert!(read_result_record(&json!({"workflows": {}})).is_none(), "不是数组");
        assert!(read_result_record(&json!("not json")).is_none());
    }

    #[test]
    fn nameless_and_malformed_entries_are_dropped_not_the_whole_list() {
        // 真源 :67-74 —— 一条读不动的条目跳过，其余照常；缺 name 也算读不动。
        let result = read_result_record(&json!({
            "workflows": [
                {"name": "部署检查", "description": "看一眼再发"},
                {"name": "   "},
                {"description": "没有名字"},
                "not a record",
                {"name": "回归"},
            ]
        }))
        .expect("合法载荷");
        let names: Vec<&str> = result.workflows.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, vec!["部署检查", "回归"]);
        assert_eq!(result.workflows[0].description.as_deref(), Some("看一眼再发"));
        assert_eq!(result.workflows[0].when_to_use, None);
        assert_eq!(result.workflows[0].arg_names, Vec::<String>::new());
    }

    #[test]
    fn arg_names_come_from_the_args_object_keys() {
        // 真源 :81 —— `isPlainRecord(entry.args) ? Object.keys(entry.args) : []`。
        let result = read_result_record(&json!({
            "workflows": [
                {"name": "w", "args": {"target": {"type": "string"}, "count": {"type": "number"}}},
                {"name": "v", "args": ["target"]},
                {"name": "u"},
            ]
        }))
        .expect("载荷");
        let w = result.workflows.iter().find(|x| x.name == "w").unwrap();
        // HashSet 无序 → 断言按集合（工程纪律 #18）。
        let mut names = w.arg_names.clone();
        names.sort();
        assert_eq!(names, vec!["count".to_string(), "target".to_string()]);
        // 数组形态的 args 不是 record → 空表。
        assert!(result.workflows.iter().find(|x| x.name == "v").unwrap().arg_names.is_empty());
        assert!(result.workflows.iter().find(|x| x.name == "u").unwrap().arg_names.is_empty());
    }

    #[test]
    fn invalid_list_defaults_empty_and_needs_a_path() {
        let result = read_result_record(&json!({
            "workflows": [],
            "invalid": [{"path": " a/.js ", "reason": "语法错"}, {"reason": "缺路径"}, "not a record"]
        }))
        .expect("载荷");
        assert!(result.workflows.is_empty());
        assert_eq!(result.invalid.len(), 1);
        assert_eq!(result.invalid[0].path, "a/.js", "path 同样 trim");
        assert_eq!(result.invalid[0].reason.as_deref(), Some("语法错"));
        // `invalid` 键缺席 → 空表（不是 None）。
        assert!(read_result_record(&json!({"workflows": []})).unwrap().invalid.is_empty());
    }

    #[test]
    fn display_channel_wins_and_carries_required_scope_path() {
        let raw = json!({ "result": { "display": {
            "kind": "saved_workflow_list",
            "workflows": [{"name": "review", "scope": "project", "path": ".zcode/review.js", "argNames": ["target"]}],
            "invalid": [{"path": ".zcode/broken.js", "reason": "缺 meta"}],
            "truncated": true,
        } } });
        let result = read_list_saved_workflows_result(&raw, Some("{\"workflows\":[]}"))
            .expect("display 通道命中");
        assert_eq!(result.workflows.len(), 1);
        assert_eq!(result.workflows[0].scope.as_deref(), Some("project"));
        assert_eq!(result.workflows[0].arg_names, vec!["target".to_string()]);
        assert_eq!(result.invalid[0].reason.as_deref(), Some("缺 meta"));
    }

    #[test]
    fn empty_list_is_a_fact_not_a_parse_failure() {
        // ★真源 :105-107 —— 空列表与「读不懂」必须可分辨。
        let raw = json!({ "result": { "display": {
            "kind": "saved_workflow_list", "workflows": [],
        } } });
        let result = read_list_saved_workflows_result(&raw, None).expect("空列表也是读出来了");
        assert!(result.workflows.is_empty());
        assert_eq!(
            list_saved_primary_text(Some(&result)),
            "这个项目里还没有保存过工作流"
        );
        // 一个候选都读不出来 → None（交回 fallback 卡，不画空列表）。
        assert_eq!(read_list_saved_workflows_result(&json!({}), Some("散文输出")), None);
    }

    #[test]
    fn legacy_probes_try_four_positions_in_order() {
        // 真源 :130 —— [output, raw.rawOutput, raw.output, raw.result]。
        let from_raw_output = json!({ "rawOutput": { "workflows": [{ "name": "只在 rawOutput" }] } });
        let result = read_list_saved_workflows_result(&from_raw_output, None).expect("命中 rawOutput");
        assert_eq!(result.workflows[0].name, "只在 rawOutput");
        // output 优先于 raw.result。
        let raw = json!({ "result": "{\"workflows\":[{\"name\":\"来自 result\"}]}" });
        let result = read_list_saved_workflows_result(&raw, Some("{\"workflows\":[{\"name\":\"来自 output\"}]}"))
            .expect("命中 output");
        assert_eq!(result.workflows[0].name, "来自 output");
    }

    #[test]
    fn counts_have_their_own_singular_keys() {
        let one = ListSavedWorkflowsResult {
            workflows: vec![SavedWorkflowRow {
                name: "w".into(),
                description: None,
                when_to_use: None,
                scope: None,
                path: None,
                arg_names: vec![],
            }],
            invalid: vec![InvalidSavedWorkflowRow { path: "p".into(), reason: None }],
        };
        assert_eq!(list_saved_count_labels(Some(&one)), ("1 个工作流".into(), "1 个文件无法解析".into()));
        let none = ListSavedWorkflowsResult { workflows: vec![], invalid: vec![] };
        let (count, invalid) = list_saved_count_labels(Some(&none));
        assert!(count.starts_with('0'), "{count}");
        assert!(invalid.starts_with('0'), "{invalid}");
        // result 缺席时两个计数都按 0 走。
        let (count, _) = list_saved_count_labels(None);
        assert!(count.starts_with('0'));
    }

    #[test]
    fn unknown_scope_is_printed_verbatim() {
        // 真源 :222-225 —— 不是 global 也不是 project 时把那个值写出来，不藏。
        assert!(matches!(scope_tag(Some("team")), ScopeTag::Plain(ref text) if text == "team"));
        assert!(matches!(scope_tag(Some("global")), ScopeTag::GlobalBadge));
        // ★词表里 project 那条文案就是字面 "project"（chat.permission.workflow.saved.scope.project），
        // 不是「项目」——这是文案表的既有取值，不是本卡的判定。
        assert!(matches!(
            scope_tag(Some("project")),
            ScopeTag::Plain(ref text) if text == "project"
        ));
        assert!(matches!(scope_tag(None), ScopeTag::Plain(ref text) if text.is_empty()));
    }

    #[test]
    fn icon_class_is_the_shared_library_slot() {
        assert_eq!(
            LIST_SAVED_WORKFLOWS_TOOL_ICON_CLASS,
            "size-4 flex-none text-foreground-subtle"
        );
    }
}
