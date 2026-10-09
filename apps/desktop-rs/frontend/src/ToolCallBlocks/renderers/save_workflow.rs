//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/save-workflow.tsx`（270 行）。
//!
//! SaveWorkflow 的聊天卡（真源 :129-134）：**刻意紧凑**——名字、覆盖标记、说明。
//! 完整内容（落点、args 声明表、脚本）归确认窗：那才是用户做决定的地方，
//! 而这张卡是决定之后的一行记录。
//!
//! ★真源 :244-245 的别处没有的处理：`canToggle` / `forceOpen` **不以 hasDetails 门控**
//! （本卡没有「读不出内容就别给展开」的门，展开恒可用），`renderContent` 也恒传。
//!
//! 归一化入参是可复用工作流 spec 的「SaveWorkflow 的归一化形状」
//! `{name, description, whenToUse?, args?, script, path, overwrite, scope}`
//! （真源 :42-48）：`path` / `overwrite` / `scope` / `shadowing` 是**解析阶段算出来的事实**，
//! 不是模型说的——确认窗展示的是「将要发生的事实」。这个 gate 没有 display 载荷，
//! 入参就是全部内容。
//!
//! **裁剪注明**：`kindLabelOverride`（真源 :246）与 `canToggle/forceOpen` 走真源缺省。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolLayout::{ToolLayoutComponent, ToolLayoutProps};
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::i18n;
use crate::components::workflowIcons::icon_save;

/// 真源 :8 —— `Save className="size-4 shrink-0 text-foreground-subtle"`。
pub const SAVE_WORKFLOW_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `OVERWRITE_BADGE_CLASSNAME`（真源 :16-17）。
///
/// 覆盖徽标与 CreateWorkflow 结果卡的 compiled 小签同族（镌刻小签——rounded-xs +
/// 大写等宽微标签），只把语义色换成 warning。用 warning 而不是 destructive 是刻意的
/// （真源 :11-15）：文件是被**替换**不是被删除，destructive 会过度表达；
/// 而覆盖是真实的语义状态，不属于 DESIGN.md 禁止的「借语义色让区块更响」。
pub const OVERWRITE_BADGE_CLASSNAME: &str =
    "shrink-0 rounded-xs border border-warning/40 px-1.5 py-0.5 font-mono text-ui-xs uppercase tracking-wf-label leading-none text-warning";

/// `SCOPE_BADGE_CLASSNAME`（真源 :118-119）：全局作用域折叠行小标。
///
/// 与 overwrite 徽标同一族，语义色改为 foreground-subtle——它只是一个作用域说明，
/// 不是警示状态，不借 warning / destructive。
pub const SCOPE_BADGE_CLASSNAME: &str =
    "shrink-0 rounded-xs border border-border px-1.5 py-0.5 font-mono text-ui-xs uppercase tracking-wf-label leading-none text-foreground-subtle";

/// `WorkflowArgDeclaration`（真源 :31-40）：归一化入参里的一条 args 声明。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowArgDeclaration {
    pub name: String,
    pub arg_type: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    /// `default` 是否在场——`default: false` 与「没有默认值」必须可分辨（真源 :37）。
    pub has_default: bool,
    pub default_value: Value,
}

/// `SaveWorkflowInput`（真源 :49-60）。
#[derive(Debug, Clone, PartialEq)]
pub struct SaveWorkflowInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub when_to_use: Option<String>,
    pub path: Option<String>,
    pub scope: Option<String>,
    /// 另一档已有同名时的遮蔽事实（真源 :55-56）。
    pub shadowing: Option<SaveWorkflowShadowing>,
    pub overwrite: bool,
    pub script: Option<String>,
    pub args: Vec<WorkflowArgDeclaration>,
}

/// `shadowing` 的两个值（真源 :56）；词表外的值算「没有遮蔽事实」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWorkflowShadowing {
    HidesGlobal,
    HiddenByProject,
}

/// `readTrimmedString`（真源 :23-29）：trim 后非空才要，且返回 **trim 后的值**
/// （与 resolve-workflow-question 的 `readText` 不同，那个返回原值）。
fn read_trimmed_string(value: &Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// `readArgDeclarations`（真源 :62-82）：对象逐键展开，非对象的条目跳过。
fn read_arg_declarations(value: &Value) -> Vec<WorkflowArgDeclaration> {
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    map.iter()
        .filter(|(_, declaration)| declaration.is_object())
        .map(|(name, declaration)| WorkflowArgDeclaration {
            name: name.clone(),
            arg_type: read_trimmed_string(declaration.get("type").unwrap_or(&Value::Null)),
            description: read_trimmed_string(declaration.get("description").unwrap_or(&Value::Null)),
            required: declaration.get("required").and_then(|v| v.as_bool()) == Some(true),
            has_default: declaration
                .as_object()
                .is_some_and(|map| map.contains_key("default")),
            default_value: declaration
                .get("default")
                .cloned()
                .unwrap_or(Value::Null),
        })
        .collect()
}

/// `readSaveWorkflowInput`（真源 :84-104）。
pub fn read_save_workflow_input(input: &Value) -> SaveWorkflowInput {
    let record = if input.is_object() { input.clone() } else { Value::Object(Default::default()) };
    let get = |key: &str| -> Value { record.get(key).cloned().unwrap_or(Value::Null) };

    let shadowing = match get("shadowing").as_str() {
        Some("hides_global") => Some(SaveWorkflowShadowing::HidesGlobal),
        Some("hidden_by_project") => Some(SaveWorkflowShadowing::HiddenByProject),
        _ => None,
    };
    // 真源 :100-101 —— script 只看「是字符串且非空」，不 trim（脚本的缩进是内容）。
    let script = get("script")
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    SaveWorkflowInput {
        name: read_trimmed_string(&get("name")),
        description: read_trimmed_string(&get("description")),
        when_to_use: read_trimmed_string(&get("whenToUse")),
        path: read_trimmed_string(&get("path")),
        scope: read_trimmed_string(&get("scope")),
        shadowing,
        // ★真源 :97-99 —— 只有显式 true 才是覆盖：字段缺席时说不出「已经有一个文件在那儿」，
        // 就不能让确认窗替用户断言这件事。
        overwrite: get("overwrite").as_bool() == Some(true),
        script,
        args: read_arg_declarations(&get("args")),
    }
}

/// 折叠行的 kindDetail（真源 :175-185）：覆盖徽标 + 全局小标同排，两者都不在场时不渲染。
#[derive(Debug, Clone, PartialEq)]
pub enum SaveWorkflowKindDetail {
    None,
    Overwrite,
    GlobalScope,
    Both,
}

pub fn save_workflow_kind_detail(overwrite: bool, is_global_scope: bool) -> SaveWorkflowKindDetail {
    match (overwrite, is_global_scope) {
        (true, true) => SaveWorkflowKindDetail::Both,
        (true, false) => SaveWorkflowKindDetail::Overwrite,
        (false, true) => SaveWorkflowKindDetail::GlobalScope,
        (false, false) => SaveWorkflowKindDetail::None,
    }
}

/// 作用域行（真源 :154-158）：global / project 两档有值，其余整行省略。
pub fn save_workflow_scope_value(scope: Option<&str>) -> Option<String> {
    match scope {
        Some("global") => Some(i18n::text("chat.toolCall.workflow.save.scope.global")),
        Some("project") => Some(i18n::text("chat.toolCall.workflow.save.scope.project")),
        _ => None,
    }
}

/// 遮蔽行（真源 :159-164）：两个值各有说法，其余整行省略。
pub fn save_workflow_shadowing_label(shadowing: Option<SaveWorkflowShadowing>) -> Option<String> {
    match shadowing? {
        SaveWorkflowShadowing::HidesGlobal => {
            Some(i18n::text("chat.toolCall.workflow.save.scope.hidesGlobal"))
        }
        SaveWorkflowShadowing::HiddenByProject => {
            Some(i18n::text("chat.toolCall.workflow.save.scope.hiddenByProject"))
        }
    }
}

/// `SaveWorkflowToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct SaveWorkflowBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub status: String,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `SaveWorkflowToolCallBlock`（真源 :135-269）。
#[component]
pub fn SaveWorkflowToolCallBlock(props: SaveWorkflowBlockProps) -> impl IntoView {
    let saved = read_save_workflow_input(&props.input);
    // ★以下文案全部在 view! 之前取好（children 先于属性求值）。
    let kind_label = i18n::text(if props.is_running {
        "chat.toolCall.workflow.save.saving"
    } else {
        "chat.toolCall.workflow.save.saved"
    });
    let fallback_name = i18n::text("chat.toolCall.workflow.fallbackName");
    let overwrite_label = i18n::text("chat.toolCall.workflow.save.overwrite");
    let global_badge_label = i18n::text("chat.toolCall.workflow.scope.global");
    let path_label = i18n::text("chat.permission.workflow.save.path");
    let when_to_use_label = i18n::text("chat.permission.workflow.save.whenToUse");
    let scope_label = i18n::text("chat.toolCall.workflow.save.scope.label");

    let is_global_scope = saved.scope.as_deref() == Some("global");
    let scope_value = save_workflow_scope_value(saved.scope.as_deref());
    let shadowing_label = save_workflow_shadowing_label(saved.shadowing);
    // 真源 :169 —— 折叠行主文本只有名字，等宽。
    let primary_text = saved.name.clone().unwrap_or_else(|| fallback_name.clone());
    let kind_detail = save_workflow_kind_detail(saved.overwrite, is_global_scope);
    // 真源 :256 —— title 优先用说明，没有说明才用工具标题。
    let title = saved.description.clone().or_else(|| props.title.clone());
    let description = saved.description.clone();
    let path = saved.path.clone();
    let when_to_use = saved.when_to_use.clone();
    let render_content = move || {
        let row_class = "flex min-w-0 items-baseline gap-2 text-ui-sm";
        view! {
            <div class="mb-2 space-y-1.5">
                {description.clone().map(|text| view! {
                    // 折叠行只留名字 + overwrite 徽标，说明移进这里（真源 :191）。
                    <p class="min-w-0 whitespace-pre-wrap break-words text-ui-base leading-5 text-foreground-subtle">
                        {text}
                    </p>
                })}
                {scope_value.clone().map(|value| view! {
                    // 作用域行在落点之上（真源 :197）。
                    <p class=row_class>
                        <span class="shrink-0 text-foreground-subtlest">{scope_label.clone()}</span>
                        <span class="min-w-0 text-foreground-subtle">{value}</span>
                    </p>
                })}
                {shadowing_label.clone().map(|text| view! {
                    <p class="min-w-0 whitespace-pre-wrap break-words text-ui-sm text-foreground-subtle">
                        {text}
                    </p>
                })}
                {path.clone().map(|path| {
                    // ★view! 的 children 先于属性求值（工程纪律 #5）：先把 title 拷出来，
                    // 再让 children 把 path move 掉。
                    let title = path.clone();
                    view! {
                        <p class=row_class>
                            <span class="shrink-0 text-foreground-subtlest">{path_label.clone()}</span>
                            <span
                                class="min-w-0 truncate font-mono text-foreground-subtle"
                                title=title
                            >
                                {path}
                            </span>
                        </p>
                    }
                })}
                {when_to_use.clone().map(|text| view! {
                    <p class=row_class>
                        <span class="shrink-0 text-foreground-subtlest">{when_to_use_label.clone()}</span>
                        <span class="min-w-0 whitespace-pre-wrap break-words text-foreground-subtle">
                            {text}
                        </span>
                    </p>
                })}
            </div>
        }
        .into_any()
    };
    let badge_overwrite = saved.overwrite;
    let badge_global = is_global_scope;
    // 折叠行的徽标（真源 :175-185）：先建好节点再进 view!（props 结构体字段用 `:`）。
    let kind_detail_view: Option<ChildrenFn> = match kind_detail {
        SaveWorkflowKindDetail::None => None,
        _ => Some(std::sync::Arc::new(move || {
            view! {
                <span class="flex items-center gap-1">
                    {badge_overwrite.then(|| view! {
                        <span class=OVERWRITE_BADGE_CLASSNAME data-workflow-overwrite-badge="true">
                            {overwrite_label.clone()}
                        </span>
                    })}
                    {badge_global.then(|| view! {
                        <span class=SCOPE_BADGE_CLASSNAME data-workflow-scope-badge="global">
                            {global_badge_label.clone()}
                        </span>
                    })}
                </span>
            }
            .into_any()
        }) as ChildrenFn),
    };

    view! {
        <ToolLayoutComponent
            props=ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                // 真源 :244-245 —— 本卡不以 hasDetails 门控展开。
                can_toggle: Some(true),
                force_open: Some(false),
                kind_label: Some(kind_label),
                kind_detail_view,
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text),
                // 刻意不传 secondaryText（真源 :250-251）：折叠行只有名字 + 徽标，
                // 说明/落点/whenToUse 全部移入展开卡体——「决定之后的一行记录」越短越好读。
                status_label: props.status_label.clone(),
                status_tooltip: props.error_text.clone(),
                show_failure_status: Some(props.status == "failed"),
                is_running: Some(props.is_running),
                title,
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! { <span class=SAVE_WORKFLOW_TOOL_ICON_CLASS>{icon_save()}</span> }.into_any()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn trimmed_strings_are_returned_trimmed() {
        // ★与 resolve-workflow-question 的 readText 不同：这里返回 trim 后的值（真源 :28）。
        let input = json!({"name": "  部署检查  ", "description": "   "});
        let saved = read_save_workflow_input(&input);
        assert_eq!(saved.name.as_deref(), Some("部署检查"));
        assert_eq!(saved.description, None, "纯空白不算说明");
    }

    #[test]
    fn non_object_input_degrades_to_everything_absent() {
        // 真源 :85 —— 不是 plain record 就当 `{}`，不抛错。
        for value in [json!("字符串"), json!([1, 2]), json!(null), json!(7)] {
            let saved = read_save_workflow_input(&value);
            assert_eq!(saved.name, None);
            assert!(!saved.overwrite);
            assert!(saved.args.is_empty());
        }
    }

    #[test]
    fn overwrite_requires_an_explicit_true() {
        // ★真源 :97-99 —— 缺席说不出「已有一个文件在那儿」，不能让窗替用户断言。
        for (value, expected) in [
            (json!(true), true),
            (json!(false), false),
            (json!("true"), false),
            (json!(1), false),
            (json!(null), false),
        ] {
            let saved = read_save_workflow_input(&json!({"overwrite": value}));
            assert_eq!(saved.overwrite, expected, "overwrite={value:?}");
        }
        // 字段缺席同样是 false。
        assert!(!read_save_workflow_input(&json!({})).overwrite);
    }

    #[test]
    fn shadowing_only_accepts_the_two_known_facts() {
        assert_eq!(
            read_save_workflow_input(&json!({"shadowing": "hides_global"})).shadowing,
            Some(SaveWorkflowShadowing::HidesGlobal)
        );
        assert_eq!(
            read_save_workflow_input(&json!({"shadowing": "hidden_by_project"})).shadowing,
            Some(SaveWorkflowShadowing::HiddenByProject)
        );
        // 陌生值/非字符串 → 没有遮蔽事实（不显示那条说明行）。
        assert_eq!(read_save_workflow_input(&json!({"shadowing": "whatever"})).shadowing, None);
        assert_eq!(read_save_workflow_input(&json!({})).shadowing, None);
    }

    #[test]
    fn script_keeps_its_indentation() {
        // 真源 :100-101 —— 只看「是字符串且非空」，不 trim：脚本的缩进是内容。
        let saved = read_save_workflow_input(&json!({"script": "  export const meta = {}\n"}));
        assert_eq!(saved.script.as_deref(), Some("  export const meta = {}\n"));
        assert_eq!(read_save_workflow_input(&json!({"script": ""})).script, None);
        // 纯空白仍算有脚本（长度非 0）——真源这里没 trim。
        assert_eq!(read_save_workflow_input(&json!({"script": "  "})).script.as_deref(), Some("  "));
    }

    #[test]
    fn arg_declarations_skip_non_object_entries() {
        // 真源 :68-70 —— 一条读不动的声明被跳过，不打挂整张表。
        let saved = read_save_workflow_input(&json!({
            "args": {
                "target": { "type": "string", "required": true, "default": false },
                "count": { "type": "number" },
                "broken": "not a record",
            }
        }));
        assert_eq!(saved.args.len(), 2, "broken 那条被跳过");
        // serde_json 的对象迭代按 BTreeMap 序（开了 preserve_order 才是插入序），
        // 断言按集合而非顺序。
        let target = saved.args.iter().find(|a| a.name == "target").expect("target");
        assert_eq!(target.arg_type.as_deref(), Some("string"));
        assert!(target.required);
        // ★`default: false` 与「没有默认值」必须可分辨（真源 :37）。
        assert!(target.has_default);
        assert_eq!(target.default_value, json!(false));
        let count = saved.args.iter().find(|a| a.name == "count").expect("count");
        assert!(!count.has_default);
        assert!(!count.required);
        // args 不是对象时是空表。
        assert!(read_save_workflow_input(&json!({"args": [1, 2]})).args.is_empty());
    }

    #[test]
    fn fold_badge_pairs_overwrite_with_global() {
        // 真源 :175-185 —— 只有覆盖、只有全局、两者都有、都不加四种。
        assert_eq!(save_workflow_kind_detail(false, false), SaveWorkflowKindDetail::None);
        assert_eq!(save_workflow_kind_detail(true, false), SaveWorkflowKindDetail::Overwrite);
        assert_eq!(save_workflow_kind_detail(false, true), SaveWorkflowKindDetail::GlobalScope);
        assert_eq!(save_workflow_kind_detail(true, true), SaveWorkflowKindDetail::Both);
        // 未知作用域不算全局（不加小标）。
        assert_eq!(
            save_workflow_kind_detail(false, false),
            SaveWorkflowKindDetail::None
        );
    }

    #[test]
    fn scope_and_shadowing_rows_are_omitted_when_unknown() {
        assert_eq!(
            save_workflow_scope_value(Some("global")).as_deref(),
            Some("全局 · 对所有项目可见")
        );
        assert_eq!(save_workflow_scope_value(Some("project")).as_deref(), Some("项目"));
        assert_eq!(save_workflow_scope_value(Some("weird")), None);
        assert_eq!(save_workflow_scope_value(None), None);

        assert_eq!(
            save_workflow_shadowing_label(Some(SaveWorkflowShadowing::HidesGlobal)).as_deref(),
            Some("同名的全局工作流将被这一份遮蔽")
        );
        assert_eq!(
            save_workflow_shadowing_label(Some(SaveWorkflowShadowing::HiddenByProject)).as_deref(),
            Some("本项目里已有同名工作流，在这里会遮蔽它")
        );
        assert_eq!(save_workflow_shadowing_label(None), None);
    }

    #[test]
    fn badges_are_the_engraved_family_not_a_loud_block() {
        // 真源 :10-19 + :114-119 —— 同族镌刻小签，只有语义色不同。
        for class in [OVERWRITE_BADGE_CLASSNAME, SCOPE_BADGE_CLASSNAME] {
            assert!(class.contains("rounded-xs"));
            assert!(class.contains("font-mono"));
            assert!(class.contains("uppercase"));
            assert!(class.contains("tracking-wf-label"));
            assert!(class.contains("leading-none"));
        }
        // 覆盖用 warning 不用 destructive；全局档不借警示色。
        assert!(OVERWRITE_BADGE_CLASSNAME.contains("text-warning"));
        assert!(!OVERWRITE_BADGE_CLASSNAME.contains("destructive"));
        assert!(SCOPE_BADGE_CLASSNAME.contains("border-border"));
        assert!(SCOPE_BADGE_CLASSNAME.contains("text-foreground-subtle"));
    }
}
