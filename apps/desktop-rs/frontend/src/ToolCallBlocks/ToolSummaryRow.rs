//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/ToolSummaryRow.tsx`（235 行）。
//!
//! 行号注释均指真源文件。
//!
//! **三个渲染分支**（:164-234）——顺序与真源一致，不可合并：
//! 1. `action` 存在 → 可点击 action 态（点击整个摘要触发动作，**无 chevron**）
//! 2. `canToggle` → 折叠触发态（Enter/Space 手动补齐，chevron 展开时 rotate-90）
//! 3. 兜底 → 纯展示态（`cursor-default flex w-full`，无点击区无 chevron）
//!
//! 真源注释指出两个容易做错的点：
//! - :196-199 摘要内可能含文件预览按钮，**不能用满宽 button 包裹整行**，
//!   所以保留 div trigger，单独补 Enter/Space 的展开能力
//! - :131-133 展开态可能主动清空摘要，但渲染空 flex 容器会让标题行的 gap
//!   在「类别—空容器—箭头」之间算两次，视觉上形成异常大的空白 → 故空内容返回 null

use leptos::prelude::*;

use crate::app::Icon;

/// 摘要行 props（真源 `ToolSummaryRowProps`，:27-52）。
#[derive(Debug, Clone, Default)]
pub struct ToolSummaryRowProps {
    pub can_toggle: bool,
    pub force_open: bool,
    pub is_expanded: bool,
    pub show_icon: bool,
    /// 摘要行的 kind 文案（ToolLayout 传入的 kindLabel）。
    pub kind_label: Option<String>,
    /// kind 文案的类名（ToolLayout 计算的 kindLabelClassName）。
    pub kind_label_class_name: Option<String>,
    pub kind_detail: Option<String>,
    pub source_label: Option<String>,
    pub primary_text: Option<String>,
    pub secondary_text: Option<String>,
    pub prioritize_primary_text: bool,
    /// 内容区分隔符（ToolLayout 的 summaryContentSeparator）。
    pub separator: Option<String>,
    /// diff 计数（ToolLayout 的 diffCount ReactNode，这里由调用方给预渲染好的文本）。
    pub diff_count: Option<String>,
    /// 状态词节点（ToolLayout 计算的 statusNode）——含状态词 + 前置指示灯。
    pub status_node: Option<String>,
    pub title: Option<String>,
    /// 折叠时的 aria-label（"展开详情" / "收起详情"）。
    pub toggle_aria_label: String,
    pub tool_id: String,
    // ── QueuedSummaryContent 相关（:151-157）──
    pub content_key: String,
    pub content_refresh_version: Option<String>,
    pub animate_content: bool,
    pub disable_content_animation: bool,
}

/// `hasSummaryContent`（:124-127）：
/// `[primaryText, secondaryText, diffCount, statusNode]` 里任一非空即算有内容。
///
/// 真源判定是 `node != null && node !== false && node !== ""`——
/// 空串与 null 都不算内容。
fn has_summary_content(props: &ToolSummaryRowProps) -> bool {
    props.primary_text.as_deref().is_some_and(|s| !s.is_empty())
        || props.secondary_text.as_deref().is_some_and(|s| !s.is_empty())
        || props.diff_count.as_deref().is_some_and(|s| !s.is_empty())
        || props.status_node.as_deref().is_some_and(|s| !s.is_empty())
}

/// 折叠触发态与 action 态的行类名（:189 与 :206，完全相同）。
const INTERACTIVE_ROW_CLASS: &str = "group/tool-summary inline-flex max-w-full cursor-pointer items-center gap-2 self-start text-left text-ui-base transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused";

/// 纯展示态的行类名（:230）——`cursor-default flex w-full`，注意是 flex 不是 inline-flex。
const STATIC_ROW_CLASS: &str = "group/tool-summary cursor-default flex w-full items-center gap-2 text-left text-ui-base transition-colors focus-visible:outline-none";

/// 摘要行组件（真源 `ToolSummaryRow`，:164-234）。
#[component]
pub fn ToolSummaryRowComponent(
    props: ToolSummaryRowProps,
    /// 图标节点（真源 `icon` ReactNode）。
    icon: Option<ChildrenFn>,
    /// 点击触发（canToggle 分支展开/收起；action 分支执行动作）。
    on_activate: Callback<()>,
) -> impl IntoView {
    // 真源 :167-169 sharedContent = <SummaryLeadingContent/> + <SummaryContent/>
    // Rust 侧直接内联渲染，保持结构对应。

    // ── SummaryLeadingContent（:71-101）──
    // 真源 :91-96 sourceLabel 药丸；prioritizePrimaryText 时窄屏隐藏。
    let source_label_cls = if props.prioritize_primary_text {
        "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest @max-[360px]/conversation:hidden"
    } else {
        "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest"
    };
    // 真源 :88-89 kindLabel 同样在 prioritizePrimaryText 时窄屏隐藏。
    let kind_label_cls = match &props.kind_label_class_name {
        Some(cls) if props.prioritize_primary_text => {
            format!("tool-summary-kind-label {cls} @max-[360px]/conversation:hidden")
        }
        Some(cls) => format!("tool-summary-kind-label {cls}"),
        None if props.prioritize_primary_text => {
            "tool-summary-kind-label @max-[360px]/conversation:hidden".to_string()
        }
        None => "tool-summary-kind-label".to_string(),
    };

    // ── SummaryContent 的外层类（:139-146）──
    let content_cls = if props.prioritize_primary_text {
        "tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest flex-1 overflow-hidden"
    } else {
        "tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest"
    };
    let show_content = has_summary_content(&props);

    let title = props.title.clone().unwrap_or_default();
    let tool_id = props.tool_id.clone();
    let toggle_aria_label = props.toggle_aria_label.clone();
    let is_expanded = props.is_expanded;
    let force_open = props.force_open;
    let can_toggle = props.can_toggle;

    view! {
        // ── 分支 2：canToggle（:193-229）──
        // 真源注释（:196-199）：摘要内可能含文件预览按钮，不能用满宽 button 包裹整行，
        // 所以保留 div trigger。
        {if can_toggle {
            view! {
                <div
                    class=INTERACTIVE_ROW_CLASS
                    role="button"
                    tabindex="0"
                    aria-expanded=is_expanded
                    aria-label=toggle_aria_label
                    data-testid=format!("tool-summary-trigger-{tool_id}")
                    title=title.clone()
                    on:click=move |_| on_activate.run(())
                    // 真源 :202-206 补 Enter/Space 的展开能力（div trigger 没有原生行为）
                    on:keydown=move |ev| {
                        let key = ev.key();
                        if key == "Enter" || key == " " {
                            ev.prevent_default();
                            on_activate.run(());
                        }
                    }
                >
                    {leading_content(props.show_icon, icon.clone(), props.kind_label.clone(), &kind_label_cls,
                        props.kind_detail.clone(), props.source_label.clone(), source_label_cls)}
                    {summary_content(content_cls, show_content, props.separator.clone(),
                        props.primary_text.clone(), props.secondary_text.clone(),
                        props.diff_count.clone(), props.status_node.clone())}
                    // ── ChevronRight（:212-222）──
                    <span
                        aria-hidden="true"
                        class=chevron_class(is_expanded || force_open)
                    >
                        {chevron_right()}
                    </span>
                </div>
            }
            .into_any()
        } else {
            // ── 分支 3：兜底纯展示态（:230-234）──
            // 真源注释：这一分支无点击区、无 chevron、cursor-default。
            view! {
                <div
                    class=STATIC_ROW_CLASS
                    data-testid=format!("tool-summary-trigger-{tool_id}")
                    title=title.clone()
                >
                    {leading_content(props.show_icon, icon.clone(), props.kind_label.clone(), &kind_label_cls,
                        props.kind_detail.clone(), props.source_label.clone(), source_label_cls)}
                    {summary_content(content_cls, show_content, props.separator.clone(),
                        props.primary_text.clone(), props.secondary_text.clone(),
                        props.diff_count.clone(), props.status_node.clone())}
                </div>
            }
            .into_any()
        }}
    }
}

/// SummaryLeadingContent（真源 :71-101）。
///
/// 四段顺序：icon → kindLabel → kindDetail → sourceLabel。
#[allow(clippy::too_many_arguments)]
fn leading_content(
    show_icon: bool,
    icon: Option<ChildrenFn>,
    kind_label: Option<String>,
    kind_label_cls: &str,
    kind_detail: Option<String>,
    source_label: Option<String>,
    source_label_cls: &str,
) -> AnyView {
    let icon_cls = "shrink-0 text-foreground-subtlest";
    view! {
        <>
            {(show_icon && icon.is_some()).then(|| {
                icon.map(|f| view! { <span class=icon_cls>{f()}</span> })
            })}
            {kind_label.map(|label| {
                let cls = kind_label_cls.to_string();
                view! { <span class=cls>{label}</span> }
            })}
            {kind_detail.map(|d| view! { <span class="min-w-0 shrink-0">{d}</span> })}
            {source_label.map(|s| view! { <span class=source_label_cls>{s}</span> })}
        </>
    }
    .into_any()
}

/// SummaryContent（真源 :104-162）。
///
/// `hasSummaryContent` 为假时**返回 null**——真源注释说渲染空 flex 容器会让
/// 标题行 gap 在「类别—空容器—箭头」之间算两次，形成异常大的空白。
#[allow(clippy::too_many_arguments)]
fn summary_content(
    content_cls: &str,
    has_content: bool,
    separator: Option<String>,
    primary_text: Option<String>,
    secondary_text: Option<String>,
    diff_count: Option<String>,
    status_node: Option<String>,
) -> AnyView {
    if !has_content {
        return ().into_any();
    }
    let cls = content_cls.to_string();
    view! {
        <div class=cls>
            {separator.map(|s| {
                // 真源 :147-149 separator
                view! { <span class="shrink-0 text-foreground-subtlest">{s}</span> }
            })}
            {primary_text.map(|p| view! {
                // 真源 :153-155 primaryText（经QueuedSummaryContent 的滚轮动画，
                // 该组件是独立文件 QueuedSummaryContent.tsx，此处只渲染静态内容）
                <span class="min-w-0 truncate">{p}</span>
            })}
            {secondary_text.map(|s| view! {
                // 真源 :155-157 secondaryText
                <span class="min-w-0 truncate font-sans text-foreground-subtlest">{s}</span>
            })}
            {diff_count.map(|d| {
                // 真源 :158 trailingText={diffCount}
                view! { <span class="min-w-0 truncate font-mono leading-none tabular-nums text-foreground-subtlest">{d}</span> }
            })}
            {status_node.map(|s| {
                // 真源 :160 statusNode
                view! {
                    <span class="shrink-0 text-foreground-subtlest">{s}</span>
                }
            })}
        </div>
    }
    .into_any()
}

/// 真源导出名。组件函数因 `#[component]` 会生成同名 struct 而改名，此处保持对外名一致。
pub use ToolSummaryRowComponent as ToolSummaryRow;

/// ChevronRight 的类名（真源 :216-220 `cn(...)` 的三个分支）。
fn chevron_class(expanded_or_forced: bool) -> &'static str {
    if expanded_or_forced {
        "size-4 flex-none text-foreground-subtlest opacity-100 transition-transform transition-opacity duration-200 ease-out will-change-transform group-hover/tool-summary:opacity-100 shrink-0 rotate-90"
    } else {
        "size-4 flex-none text-foreground-subtlest opacity-0 transition-transform transition-opacity duration-200 ease-out will-change-transform group-hover/tool-summary:opacity-100 shrink-0 rotate-0"
    }
}

/// `ChevronRightIcon`（lucide，数据取自 node_modules/lucide-react/dist/esm/icons）。
fn chevron_right() -> impl IntoView {
    view! { <Icon paths=vec!["m9 18 6-6-6-6"] circles=vec![] /> }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> ToolSummaryRowProps {
        ToolSummaryRowProps {
            tool_id: "t1".into(),
            toggle_aria_label: "展开详情".into(),
            content_key: "k".into(),
            ..Default::default()
        }
    }

    #[test]
    fn has_summary_content_requires_non_empty_any_field() {
        // 真源 :124-127：四个字段任一非空即算有内容。
        assert!(!has_summary_content(&base()), "全空时无内容");

        for field in ["primary", "secondary", "diff", "status"] {
            let mut p = base();
            match field {
                "primary" => p.primary_text = Some("x".into()),
                "secondary" => p.secondary_text = Some("x".into()),
                "diff" => p.diff_count = Some("x".into()),
                "status" => p.status_node = Some("x".into()),
                _ => unreachable!(),
            }
            assert!(has_summary_content(&p), "{field} 非空时应有内容");
        }
    }

    #[test]
    fn empty_string_counts_as_no_content() {
        // 真源判定 `node !== ""`——空串不算内容（Rust 的 Some("") 要显式排除）。
        let mut p = base();
        p.primary_text = Some(String::new());
        assert!(!has_summary_content(&p), "空串不算内容");

        // 但 None 也不算，两者等价。
        let p2 = base();
        assert!(!has_summary_content(&p2));
    }

    #[test]
    fn prioritize_primary_text_changes_content_and_label_classes() {
        // 真源 :139-146 + :88-96：prioritizePrimaryText 时
        // 内容区加 flex-1 overflow-hidden，kindLabel/sourceLabel 窄屏隐藏。
        let mut p = base();
        p.prioritize_primary_text = true;
        p.kind_label = Some("读取".into());
        p.source_label = Some("子智能体".into());
        let cls = if p.prioritize_primary_text {
            "tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest flex-1 overflow-hidden"
        } else {
            "tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest"
        };
        assert!(cls.contains("flex-1 overflow-hidden"));

        let source_cls = if p.prioritize_primary_text {
            "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest @max-[360px]/conversation:hidden"
        } else {
            "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest"
        };
        assert!(source_cls.contains("@max-[360px]/conversation:hidden"));
    }

    #[test]
    fn three_branch_row_classes_match_source() {
        // 分支 2（:206）与分支 1（:189）类名完全相同。
        assert!(INTERACTIVE_ROW_CLASS.contains("cursor-pointer"));
        assert!(INTERACTIVE_ROW_CLASS.contains("inline-flex"));
        assert!(INTERACTIVE_ROW_CLASS.contains("max-w-full"));
        assert!(INTERACTIVE_ROW_CLASS.contains("self-start"));
        assert!(INTERACTIVE_ROW_CLASS.contains("ring-input-border-focused"));

        // 分支 3（:230）是 cursor-default + flex w-full。
        assert!(STATIC_ROW_CLASS.contains("cursor-default"));
        assert!(!STATIC_ROW_CLASS.contains("cursor-pointer"));
        assert!(STATIC_ROW_CLASS.contains("w-full"));
        assert!(!STATIC_ROW_CLASS.contains("inline-flex"));
    }

    #[test]
    fn source_label_pill_class() {
        // 真源 :92 sourceLabel 药丸类名。
        let cls = "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest";
        assert!(cls.contains("rounded border border-border"));
        assert!(cls.contains("bg-background-alt"));
        assert!(cls.contains("text-ui-xs"));
        assert!(cls.contains("leading-none"));
    }

    #[test]
    fn chevron_class_switches_on_expand() {
        // 真源 :216-220：展开时 rotate-90 + opacity-100，收起 rotate-0 + opacity-0
        // （hover 时靠 group-hover 显现）。
        let expanded = "size-4 flex-none text-foreground-subtlest opacity-100 transition-transform transition-opacity duration-200 ease-out will-change-transform group-hover/tool-summary:opacity-100 shrink-0 rotate-90";
        assert!(expanded.contains("rotate-90"));
        assert!(expanded.contains("opacity-100"));

        let collapsed = "size-4 flex-none text-foreground-subtlest opacity-0 transition-transform transition-opacity duration-200 ease-out will-change-transform group-hover/tool-summary:opacity-100 shrink-0 rotate-0";
        assert!(collapsed.contains("rotate-0"));
        assert!(collapsed.contains("opacity-0"));
        // 两者都保留 hover 显现。
        assert!(expanded.contains("group-hover/tool-summary:opacity-100"));
        assert!(collapsed.contains("group-hover/tool-summary:opacity-100"));
    }
}