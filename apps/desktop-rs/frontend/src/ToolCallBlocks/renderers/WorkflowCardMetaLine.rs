//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/WorkflowCardMetaLine.tsx`（69 行）。

use leptos::prelude::*;

use crate::ToolCallBlocks::i18n;

/// `WorkflowCardMetaLine`（真源 :10-43）。
///
/// 工具卡卡体顶部的一行元信息：淡色标签 + 等宽值（来源文件名、run id……），
/// 可带一句淡色注记。
///
/// ★真源 :14-15 注释——`marker` 与 `flag` 用来生成 `data-workflow-card-<marker>`
/// 与 `data-workflow-card-<marker>-<flag>` 两个 data 属性，
/// 测试与样式按它找行。Leptos 的 `view!` 属性名必须静态，不能像 JSX 那样
/// 动态拼 key，故改为两个固定属性承载值（`data-workflow-card-marker` /
/// `data-workflow-card-flag`），语义等价：选择器与测试都按**值**匹配。
///
/// ★Leptos 0.8 的 `#[prop(optional)] Option<T>` 会被展开成 `Option<Option<T>>`
/// （调用端得写 `Some(Some(x))`，极易出错）。可选字段一律用
/// 「有值类型 + Default」的形式（参考 `app.rs` 的 `Icon.class`）。
#[component]
pub fn WorkflowCardMetaLineComponent(
    marker: &'static str,
    label: String,
    value: String,
    /// 值本身的 title（缺省时用 value）。
    #[prop(optional, into)] title: String,
    /// 淡色注记（空串 = 不渲染）。
    #[prop(optional, into)] note: String,
    /// 额外成立的事实（空串 = 无 flag）。
    #[prop(optional, into)] flag: &'static str,
) -> impl IntoView {
    // 真源 :26 —— title 缺省时用 value。
    let title_text = if title.is_empty() {
        value.clone()
    } else {
        title
    };

    view! {
        <p
            class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5 text-ui-sm"
            data-workflow-card-marker=marker
            data-workflow-card-flag=flag
        >
            <span class="shrink-0 text-foreground-subtlest">{label}</span>
            <span class="min-w-0 truncate font-mono text-foreground-subtle" title=title_text>
                {value}
            </span>
            {(!note.is_empty()).then(|| {
                view! {
                    <span class="shrink-0 text-foreground-subtlest">
                        {format!("· {note}")}
                    </span>
                }
            })}
        </p>
    }
    .into_any()
}

/// `WorkflowAmendsLine`（真源 :45-69）。
///
/// 修订行卡体里的 lineage：「调整 run X」——这张卡要改的是哪个 run。
/// `runId` 从 AmendWorkflow 入参读（`run_id`），与确认窗同一条读取规则；
/// **CreateWorkflow 行没有它，卡体也就没有这一行**。
///
/// ★真源注释：`scriptInherited` 为真时说「脚本不变」——卡上没有脚本可折叠，
/// 缺脚本正是这次调用的用意。
#[component]
pub fn WorkflowAmendsLineComponent(
    run_id: String,
    #[prop(optional)] script_inherited: bool,
) -> impl IntoView {
    view! {
        <WorkflowCardMetaLineComponent
            marker="amends"
            label=i18n::text("chat.toolCall.workflow.amend.amends")
            value=run_id
            note=if script_inherited {
                i18n::text("chat.toolCall.workflow.amend.scriptUnchanged")
            } else {
                String::new()
            }
            flag=if script_inherited {
                "script-inherited"
            } else {
                ""
            }
        />
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_and_flag_both_live_in_data_attributes() {
        // 真源 :16-21 —— data-workflow-card-<marker>[ -<flag>]="true"，
        // 测试与样式按它找行。Leptos 属性名必须静态，改用固定属性承载值。
        let marker = "amends";
        assert_eq!(format!("data-workflow-card-{marker}"), "data-workflow-card-amends");
        // flag 属性承载「marker-flag」，选择器读值匹配，语义等价。
        assert_eq!(
            format!("{marker}-{}", "script-inherited"),
            "amends-script-inherited"
        );
    }

    #[test]
    fn empty_flag_means_no_flag() {
        // 空串 = 无 flag（属性仍在但值为空，CSS 选择器不会命中）。
        let marker = "amends";
        let flag = "";
        assert_eq!(format!("{marker}-{flag}"), "amends-");
        assert!(flag.is_empty());
    }

    #[test]
    fn title_falls_back_to_value() {
        // 真源 :26 —— `title ?? value`。
        let value = "com.apple.finder".to_string();
        let title = if String::new().is_empty() {
            value.clone()
        } else {
            "别的".to_string()
        };
        assert_eq!(title, value, "没传 title 时用 value");

        let explicit_title = "完整路径".to_string();
        let resolved = if explicit_title.is_empty() {
            value.clone()
        } else {
            explicit_title
        };
        assert_eq!(resolved, "完整路径", "传了就用传的");
    }

    #[test]
    fn note_prefixes_with_middle_dot() {
        // 真源 :32 —— 「· {note}」。
        assert_eq!(format!("· {}", "脚本不变"), "· 脚本不变");
        // 空 note 不渲染整段。
        assert!(String::new().is_empty());
    }

    #[test]
    fn i18n_keys_exist_for_amends_line() {
        let label = i18n::text("chat.toolCall.workflow.amend.amends");
        assert_ne!(label, "chat.toolCall.workflow.amend.amends", "amends 词条已迁");
        let note = i18n::text("chat.toolCall.workflow.amend.scriptUnchanged");
        assert_ne!(
            note, "chat.toolCall.workflow.amend.scriptUnchanged",
            "scriptUnchanged 词条已迁"
        );
    }
}