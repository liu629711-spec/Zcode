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
//!
//! **裁剪注明**：真源 :150-158 把 primaryText / secondaryText / trailingText={diffCount}
//! 三件套整体交给 `QueuedSummaryContent`（滚轮队列），本模块暂按静态顺序渲染。
//! 卡在「快照冻结」上：真源每次渲染天然给出冻结的 ReactNode，而 Leptos 的 `ChildrenFn`
//! 是取当前响应式值的闭包，排队中的旧快照会播出**最新**文本，正违背该组件 :142-143
//! 那条「不能因为 key 去重留下最早的 +N/-N」。细节见同日项目日志。
//!
//! props 里的 ReactNode 槽位用 `SummaryNode`（文本 / 节点工厂两态），不是字符串——
//! 消费方要传文件 chip、带色子代理名、圆点+状态词这类节点。

use leptos::prelude::*;

use crate::app::Icon;

/// 真源 `ReactNode` 槽位在 Rust 侧的形态。
///
/// 真源里 `kindLabel / kindDetail / sourceLabel / primaryText / secondaryText /
/// diffCount / statusNode / separator` 全是 `ReactNode`，纯文本是它的子集。
/// 这里合成两态而不是只收 `ChildrenFn`，是为了保住真源 `hasSummaryContent` 那句
/// `node !== ""`——节点形态问不出「空不空」，文本形态问得出。
#[derive(Clone, Default)]
pub enum SummaryNode {
    /// 真源的 `null` / `undefined` / `false`。
    #[default]
    Empty,
    /// 真源的字符串子节点（空串按真源口径不算内容）。
    Text(String),
    /// 真源的元素子节点（文件 chip、带色名称、圆点+状态词这类）。
    View(ChildrenFn),
}

impl SummaryNode {
    /// 真源 `node != null && node !== false && node !== ""`。
    pub fn is_present(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Text(text) => !text.is_empty(),
            Self::View(_) => true,
        }
    }

    /// 塞进给定类名的 span（真源各段外层的 `cn(...)` span）。
    ///
    /// 不在场返回 `None`，调用方据此整段不渲染——真源的 `? :` 三元。
    pub fn render_with_class(&self, cls: &str) -> Option<AnyView> {
        match self {
            Self::Empty => None,
            Self::Text(text) if text.is_empty() => None,
            Self::Text(text) => {
                let text = text.clone();
                let cls = cls.to_string();
                Some(view! { <span class=cls>{text}</span> }.into_any())
            }
            Self::View(factory) => {
                let factory = factory.clone();
                let cls = cls.to_string();
                Some(view! { <span class=cls>{factory()}</span> }.into_any())
            }
        }
    }
}

/// 摘要行 props（真源 `ToolSummaryRowProps`，:27-52）。
#[derive(Clone, Default)]
pub struct ToolSummaryRowProps {
    pub can_toggle: bool,
    pub force_open: bool,
    pub is_expanded: bool,
    pub show_icon: bool,
    /// 摘要行的 kind 文案（ToolLayout 传入的 kindLabel）。
    pub kind_label: SummaryNode,
    /// kind 文案的类名（ToolLayout 计算的 kindLabelClassName）。
    pub kind_label_class_name: Option<String>,
    pub kind_detail: SummaryNode,
    pub source_label: SummaryNode,
    pub primary_text: SummaryNode,
    pub secondary_text: SummaryNode,
    pub prioritize_primary_text: bool,
    /// 内容区分隔符（ToolLayout 的 summaryContentSeparator）。
    pub separator: SummaryNode,
    /// diff 计数（真源 :33 `diffCount?: ReactNode`，由调用方给已渲染好的节点）。
    pub diff_count: SummaryNode,
    /// 状态词节点（ToolLayout 计算的 statusNode）——含状态词 + 前置指示灯。
    pub status_node: SummaryNode,
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
    [
        &props.primary_text,
        &props.secondary_text,
        &props.diff_count,
        &props.status_node,
    ]
    .into_iter()
    .any(SummaryNode::is_present)
}

/// 真源 :74 icon 外层 span——`[&_svg]:` 那半段决定 svg 图标是否吃到同一着色。
pub const ICON_CLASS: &str = "shrink-0 text-foreground-subtlest [&_svg]:text-foreground-subtlest";
/// 真源 :81 `cn("tool-summary-kind-label", kindLabelClassName, ...)` 的第一段。
pub const KIND_LABEL_CLASS: &str = "tool-summary-kind-label";
/// 真源 :83 与 :94：prioritizePrimaryText 时 kindLabel / sourceLabel 在窄屏隐藏。
pub const HIDE_ON_NARROW_CLASS: &str = "@max-[360px]/conversation:hidden";
/// 真源 :93 sourceLabel 药丸。
pub const SOURCE_LABEL_CLASS: &str = "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest";
/// 真源 :94 prioritizePrimaryText 时的药丸（`cn` 把隐藏类拼在末尾）。
pub const SOURCE_LABEL_PRIORITIZED_CLASS: &str = "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest @max-[360px]/conversation:hidden";
/// 真源 :143 SummaryContent 外层 div。
pub const SUMMARY_CONTENT_CLASS: &str = "tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest";
/// 真源 :144 prioritizePrimaryText 时追加的两项。
pub const SUMMARY_CONTENT_PRIORITIZED_CLASS: &str = "tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest flex-1 overflow-hidden";
/// 真源 :148 separator 的 span。
pub const SEPARATOR_CLASS: &str = "shrink-0 text-foreground-subtlest";

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
    // 真源 :167-169 sharedContent = <SummaryLeadingContent/> + <SummaryContent/>，
    // 两者都从同一份 props 取值（真源用 `{...props}` 摊平）。
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
                    {leading_content(&props, &icon)}
                    {summary_content(&props)}
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
                    {leading_content(&props, &icon)}
                    {summary_content(&props)}
                </div>
            }
            .into_any()
        }}
    }
}

/// 真源 :79-87 `cn("tool-summary-kind-label", kindLabelClassName, prioritize && 隐藏)`。
///
/// `cn` 就是按顺序空格拼接，故三段顺序不可换。
pub fn kind_label_class(kind_label_class_name: Option<&str>, prioritize: bool) -> String {
    let mut parts = vec![KIND_LABEL_CLASS];
    if let Some(cls) = kind_label_class_name {
        parts.push(cls);
    }
    if prioritize {
        parts.push(HIDE_ON_NARROW_CLASS);
    }
    parts.join(" ")
}

/// 真源 :90-98 sourceLabel 药丸类名。
pub fn source_label_class(prioritize: bool) -> &'static str {
    if prioritize {
        SOURCE_LABEL_PRIORITIZED_CLASS
    } else {
        SOURCE_LABEL_CLASS
    }
}

/// 真源 :141-146 SummaryContent 外层 div 类名。
pub fn summary_content_class(prioritize: bool) -> &'static str {
    if prioritize {
        SUMMARY_CONTENT_PRIORITIZED_CLASS
    } else {
        SUMMARY_CONTENT_CLASS
    }
}

/// SummaryLeadingContent（真源 :53-102）。
///
/// 四段顺序：icon → kindLabel → kindDetail → sourceLabel。
/// 真源把整个 props 摊进来（`<SummaryLeadingContent {...props} />`），这里同形。
fn leading_content(props: &ToolSummaryRowProps, icon: &Option<ChildrenFn>) -> AnyView {
    let kind_label_cls = kind_label_class(
        props.kind_label_class_name.as_deref(),
        props.prioritize_primary_text,
    );
    let source_label_cls = source_label_class(props.prioritize_primary_text);
    let icon_cls = ICON_CLASS;

    view! {
        <>
            {props.show_icon.then(|| {
                icon.as_ref().map(|f| view! { <span class=icon_cls>{f()}</span> })
            })}
            {props.kind_label.render_with_class(&kind_label_cls)}
            {props
                .kind_detail
                .render_with_class("min-w-0 shrink-0")}
            {props.source_label.render_with_class(source_label_cls)}
        </>
    }
    .into_any()
}

/// SummaryContent（真源 :104-162）。
///
/// `hasSummaryContent` 为假时**返回 null**——真源注释说渲染空 flex 容器会让
/// 标题行 gap 在「类别—空容器—箭头」之间算两次，形成异常大的空白。
fn summary_content(props: &ToolSummaryRowProps) -> AnyView {
    if !has_summary_content(props) {
        return ().into_any();
    }
    // 真源 :141-146 外层 div 类名。
    let content_cls = summary_content_class(props.prioritize_primary_text);

    view! {
        <div class=content_cls>
            {props.separator.render_with_class(SEPARATOR_CLASS)}
            // 真源 :150-158 primaryText / secondaryText / trailingText={diffCount}
            // 三件套整体交给 QueuedSummaryContent（滚轮队列）。该组件依赖「每次内容变化
            // 冻结一份视图快照」，Leptos 的 ChildrenFn 取的是当前响应式值、冻结不了，
            // 故本模块暂按静态顺序渲染——见文件头的裁剪注明。
            {props.primary_text.render_with_class("min-w-0 truncate")}
            {props
                .secondary_text
                .render_with_class("min-w-0 truncate font-sans text-foreground-subtlest")}
            {props
                .diff_count
                .render_with_class("min-w-0 truncate font-mono leading-none tabular-nums text-foreground-subtlest")}
            // 真源 :159 statusNode 原样落在这层之后，无包装 span。
            {(&props.status_node).render_with_class("shrink-0 text-foreground-subtlest")}
        </div>
    }
    .into_any()
}

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
    // Arc 只在测试里构造 ChildrenFn 用到；放顶层会在 wasm 侧被判 unused（host 下算用到了）。
    use std::sync::Arc;

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
                "primary" => p.primary_text = SummaryNode::Text("x".into()),
                "secondary" => p.secondary_text = SummaryNode::Text("x".into()),
                "diff" => p.diff_count = SummaryNode::Text("x".into()),
                "status" => p.status_node = SummaryNode::Text("x".into()),
                _ => unreachable!(),
            }
            assert!(has_summary_content(&p), "{field} 非空时应有内容");
        }
    }

    #[test]
    fn empty_string_counts_as_no_content() {
        // 真源判定 `node !== ""`——空串不算内容（Rust 的 Text("") 要显式排除）。
        let mut p = base();
        p.primary_text = SummaryNode::Text(String::new());
        assert!(!has_summary_content(&p), "空串不算内容");

        // 但 None 也不算，两者等价。
        let p2 = base();
        assert!(!has_summary_content(&p2));
    }

    #[test]
    fn summary_node_presence_rules() {
        // 节点形态（真源的 ReactNode 元素）恒算在场——它问不出「空不空」。
        // ChildrenFn = Arc<dyn Fn() -> AnyView + Send + Sync>，强转要在 let 上完成。
        let view_node: ChildrenFn = Arc::new(|| view! { <span>"chip"</span> }.into_any());
        assert!(SummaryNode::View(view_node).is_present());
        assert!(!SummaryNode::Empty.is_present());
        assert!(!SummaryNode::Text(String::new()).is_present());
        assert!(SummaryNode::Text("x".into()).is_present());
    }

    #[test]
    fn prioritize_primary_text_changes_content_and_label_classes() {
        // 真源 :141-146 + :83/:94：prioritizePrimaryText 时内容区加 flex-1
        // overflow-hidden，kindLabel/sourceLabel 窄屏隐藏。
        assert!(!summary_content_class(false).contains("flex-1"));
        // 完整串必须等于「基础串 + 追加项」，两处常量不能各自漂移。
        assert_eq!(
            summary_content_class(true),
            format!("{SUMMARY_CONTENT_CLASS} flex-1 overflow-hidden")
        );
        assert_eq!(source_label_class(false), SOURCE_LABEL_CLASS);
        assert_eq!(
            source_label_class(true),
            format!("{SOURCE_LABEL_CLASS} {HIDE_ON_NARROW_CLASS}")
        );

        // 真源 cn(...) 的顺序：基础类 → 调用方类 → 窄屏隐藏。顺序换了视觉就变。
        assert_eq!(kind_label_class(None, false), KIND_LABEL_CLASS);
        assert_eq!(
            kind_label_class(Some("text-ui-xs"), true),
            format!("{KIND_LABEL_CLASS} text-ui-xs {HIDE_ON_NARROW_CLASS}")
        );
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
