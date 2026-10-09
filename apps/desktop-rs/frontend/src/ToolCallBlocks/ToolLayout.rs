//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/ToolLayout.tsx`（363 行）。
//!
//! 行号注释均指真源文件。**所有 40+ 个 renderer 都复用这一个外壳**，
//! 差异只在 props——所以外壳对了，90% 的视觉就对了。
//!
//! 折叠状态机的四条规则（真源注释支撑，不能省）：
//! 1. 默认收起（:107-108 `toolLayoutOpenState.get(key) ?? false`）
//! 2. 模块级 Map 记忆（:16），按 `persistOpenKey ?? toolId`，跨卡片实例存活
//! 3. `autoOpen` 是**一次性**的（:154-166）——不能做成 forceOpen，那会锁死成不可收起
//! 4. 收起后**延迟 300ms 才卸载 children**（:19, :195-210）——立刻卸载会让
//!    高度动画读到继承的父容器高度，详情区瞬间撑成空白块，下面内容像全闪没了

// 目录名与文件名用 PascalCase 严格对齐真源（ToolCallBlocks/ToolLayout.tsx），
// 换来的是「一个真源文件 ↔ 一个 Rust 文件」的直��对应关系；偏离真源命名反而不利于对照。
#![allow(non_snake_case)]

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::app::Icon;
use super::ToolSummaryRow::{SummaryNode, ToolSummaryRowComponent, ToolSummaryRowProps};
use super::i18n;

/// `TOOL_CONTENT_COLLAPSE_UNMOUNT_DELAY_MS = 300`（:19）。
pub const TOOL_CONTENT_COLLAPSE_UNMOUNT_DELAY_MS: u64 = 300;

/// `TOOL_CONTENT_SHELL_CLASSNAME`（:20）。
pub const TOOL_CONTENT_SHELL_CLASSNAME: &str = "text-popover-foreground outline-none";

/// `TOOL_CONTENT_SPACING_CLASSNAME`（:21）。
pub const TOOL_CONTENT_SPACING_CLASSNAME: &str = "pt-2";

/// 失败详情复制后的图标复位延迟（真源 :237 的 `window.setTimeout(..., 1500)`）。
const FAILURE_TOOLTIP_COPY_RESET_MS: u64 = 1500;

/// 模块级折叠状态表（真源 :16 `const toolLayoutOpenState = new Map<string, boolean>()`）。
fn tool_layout_open_state() -> &'static Mutex<HashMap<String, bool>> {
    static STORE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn open_state_get(key: &str) -> Option<bool> {
    tool_layout_open_state()
        .lock()
        .ok()
        .and_then(|m| m.get(key).copied())
}

fn open_state_set(key: &str, open: bool) {
    if let Ok(mut m) = tool_layout_open_state().lock() {
        m.insert(key.to_string(), open);
    }
}

/// 文件 chip 的数据（对应真源 read 卡传给 primaryText 的 ReadFileChip 节点）。
#[derive(Clone, Default)]
pub struct FileChipData {
    pub path: String,
    pub file_name: String,
    pub icon_src: String,
    pub clickable: bool,
}

/// ToolLayout 的 props（真源 `ToolLayoutProps`，:22-65）。
///
/// 全部按真源字段名与默认值；`Option<T>` 对应真源的可选 props，
/// `#[serde(default)]` 语义在 Rust 侧用 `Default` trait 表达。
#[derive(Clone, Default)]
pub struct ToolLayoutProps {
    pub tool_id: String,
    pub persist_open_key: Option<String>,
    /// 图标（各 renderer 传入的 `*_TOOL_ICON`）。
    pub icon: Option<String>,
    // ── 以下是默认值非零的字段，用 Option 区分「未传」与「传了 false」──
    pub show_icon: Option<bool>,
    pub can_toggle: Option<bool>,
    pub force_open: Option<bool>,
    pub auto_open: Option<bool>,
    pub auto_collapse_on_complete: Option<bool>,
    pub prioritize_primary_text: Option<bool>,
    pub hide_secondary_text_when_open: Option<bool>,
    pub hide_diff_count_when_open: Option<bool>,
    pub show_status_label: Option<bool>,
    pub show_failure_status: Option<bool>,
    pub is_running: Option<bool>,

    // ── 文本类 props ──
    pub kind_label: Option<String>,
    pub expanded_kind_label: Option<String>,
    pub kind_detail: Option<String>,
    pub expanded_kind_detail: Option<String>,
    /// kindDetail 的自定义节点（真源 ReactNode）。优先于 `kind_detail` 文本。
    pub kind_detail_view: Option<ChildrenFn>,
    pub source_label: Option<String>,
    pub primary_text: Option<String>,
    /// 主文本的「文件 chip」形态（真源 primaryText 是 ReactNode，read 卡传
    /// ReadFileChip 节点）。**不能直接传 view**——Leptos 的 View 类型非 Send，
    /// 放进 Arc 跨闭包传递会编译失败。故传数据（路径+ 文件名 + 图标 + 是否可点），
    /// 由 ToolLayout 内部渲染 chip。
    pub primary_file_chip: Option<FileChipData>,
    /// 主文本的自定义节点（真源 primaryText 是 ReactNode）。优先于 primary_text。
    pub primary_text_view: Option<ChildrenFn>,
    /// 次文本的自定义节点（真源 secondaryText 是 ReactNode）。优先于 secondary_text。
    pub secondary_text_view: Option<ChildrenFn>,
    pub expanded_primary_text: Option<String>,
    pub secondary_text: Option<String>,
    pub expanded_secondary_text: Option<String>,
    pub title: Option<String>,
    pub expanded_title: Option<String>,
    pub status_label: Option<String>,
    /// 状态词之前的灯（真源 :50 邻位的 `statusIndicator?: ReactNode`）。
    /// 编译反馈行的空环灯走这里：形状说「什么都没跑」，颜色说注意力还悬不悬着。
    pub status_indicator_view: Option<ChildrenFn>,
    /// 状态位的自定义节点（真源 :50 `statusLabel?: ReactNode`）。优先于 `status_label`。
    /// 观察/恢复一类的工作流卡在状态位里画「圆点 + 词」（状态绝不只靠颜色，DESIGN.md），
    /// 那是节点不是文本，故开这个槽位（同 `kind_detail_view` 的先例）。
    pub status_label_view: Option<ChildrenFn>,
    /// 失败详情（挂状态词 tooltip）。
    pub status_tooltip: Option<String>,
    /// diff 计数（增, 删）。
    pub diff_count: Option<(u32, u32)>,
    /// 展开后隐藏次文本（:44）。
    pub summary_content_separator: Option<String>,
    /// 摘要内容的 key（真源 :45summaryContentKey）——变化时重置滚轮动画。
    pub summary_content_key: Option<String>,
    /// 摘要内容是否启用滚轮式轮播动画（真源 :46 animateSummaryContent）。
    pub animate_summary_content: Option<bool>,
    /// 禁用摘要动画（真源 :47 disableSummaryContentAnimation）。
    /// 与 `animate_summary_content` 同为 API 兼容声明——Rust 侧滚轮动画未实现。
    pub disable_summary_content_animation: Option<bool>,
}

impl ToolLayoutProps {
    // ── 真源 props 解构处的默认值（:67-99）──
    pub fn show_icon(&self) -> bool {
        self.show_icon.unwrap_or(true)
    }
    pub fn can_toggle(&self) -> bool {
        self.can_toggle.unwrap_or(true)
    }
    pub fn force_open(&self) -> bool {
        self.force_open.unwrap_or(false)
    }
    pub fn auto_open(&self) -> bool {
        self.auto_open.unwrap_or(false)
    }
    pub fn auto_collapse_on_complete(&self) -> bool {
        self.auto_collapse_on_complete.unwrap_or(false)
    }
    pub fn prioritize_primary_text(&self) -> bool {
        self.prioritize_primary_text.unwrap_or(false)
    }
    pub fn hide_secondary_text_when_open(&self) -> bool {
        self.hide_secondary_text_when_open.unwrap_or(false)
    }
    pub fn hide_diff_count_when_open(&self) -> bool {
        self.hide_diff_count_when_open.unwrap_or(false)
    }
    pub fn show_status_label(&self) -> bool {
        self.show_status_label.unwrap_or(false)
    }
    pub fn show_failure_status(&self) -> bool {
        self.show_failure_status.unwrap_or(false)
    }
    pub fn is_running(&self) -> bool {
        self.is_running.unwrap_or(false)
    }

    /// `resolvedPersistOpenKey = persistOpenKey ?? toolId`（:105）。
    pub fn resolved_persist_open_key(&self) -> String {
        self.persist_open_key
            .clone()
            .unwrap_or_else(|| self.tool_id.clone())
    }

    /// 摘要行的 kindLabel 类名（:143-147）。
    ///
    /// 运行态加 `animated-gradient-text`（4s 扫光，styles.css:1068-1087）；
    /// 非运行态最浅文本色。**图标保持静态**——真源注释说流式期间旋转图标
    /// 会长期占用渲染资源，改由文案扫光表达。
    pub fn kind_label_class(&self) -> &'static str {
        if self.is_running() {
            "font-medium whitespace-nowrap shrink-0 animated-gradient-text"
        } else {
            "font-medium whitespace-nowrap shrink-0 text-foreground-subtlest"
        }
    }

    /// `summaryPrimaryText`（:132-133）。
    pub fn summary_primary_text(&self, is_expanded: bool) -> Option<&str> {
        if is_expanded && self.expanded_primary_text.is_some() {
            self.expanded_primary_text.as_deref()
        } else {
            self.primary_text.as_deref()
        }
    }

    /// `summaryKindLabel`（:133）。
    pub fn summary_kind_label(&self, is_expanded: bool) -> Option<&str> {
        if is_expanded && self.expanded_kind_label.is_some() {
            self.expanded_kind_label.as_deref()
        } else {
            self.kind_label.as_deref()
        }
    }

    /// `summaryKindDetail`（:134）。
    pub fn summary_kind_detail(&self, is_expanded: bool) -> Option<&str> {
        if is_expanded && self.expanded_kind_detail.is_some() {
            self.expanded_kind_detail.as_deref()
        } else {
            self.kind_detail.as_deref()
        }
    }

    /// `summarySecondaryText`（:135-140）——注意三层回落：展开态文案 →
    /// hideSecondaryTextWhenOpen 时 null → 原始 secondaryText。
    pub fn summary_secondary_text(&self, is_expanded: bool) -> Option<&str> {
        if is_expanded && self.expanded_secondary_text.is_some() {
            self.expanded_secondary_text.as_deref()
        } else if is_expanded && self.hide_secondary_text_when_open() {
            None
        } else {
            self.secondary_text.as_deref()
        }
    }

    /// `summaryTitle`（:141）。
    pub fn summary_title(&self, is_expanded: bool) -> Option<&str> {
        if is_expanded && self.expanded_title.is_some() {
            self.expanded_title.as_deref()
        } else {
            self.title.as_deref()
        }
    }

    /// `shouldShowDiffCount`（:144）。
    pub fn should_show_diff_count(&self, is_expanded: bool) -> bool {
        self.diff_count.is_some() && !(is_expanded && self.hide_diff_count_when_open())
    }

    /// `shouldShowStatusLabel`（:114）：
    /// `(showStatusLabel || showFailureStatus) && statusLabel != null`。
    ///
    /// 真源的 `statusLabel` 是 ReactNode，节点形态在 Rust 侧走 `status_label_view`；
    /// 两者任一在场即算「statusLabel != null」。
    pub fn should_show_status_label(&self) -> bool {
        (self.show_status_label() || self.show_failure_status())
            && (self.status_label.is_some() || self.status_label_view.is_some())
    }

    /// `resolvedSummaryContentKey`（:142-143）。
    pub fn resolved_summary_content_key(&self, is_expanded: bool) -> String {
        let title = self.summary_title(is_expanded).unwrap_or_default();
        let status = self.status_label.as_deref().unwrap_or_default();
        format!("{title}:{status}")
    }
}

// ---------------------------------------------------------------------------
// 图标（lucide path 取自 node_modules/lucide-react/dist/esm/icons，与真源同源）
// ---------------------------------------------------------------------------

fn icon_check() -> impl IntoView {
    // 真源 :277 CheckIcon
    view! { <Icon paths=vec!["M20 6 9 17l-5-5"] circles=vec![] /> }
}

fn icon_copy() -> impl IntoView {
    // 真源 :280 CopyIcon
    view! {
        <Icon
            paths=vec![
                "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2",
                "M9 2h6a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z",
            ]
            circles=vec![]
        />
    }
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/// ToolLayout 组件（真源 `ToolLayoutComponent`，:67-363）。
///
/// 折叠内容由调用方通过 `content` 传入（对应真源的 `content` / `renderContent`）。
#[component]
pub fn ToolLayoutComponent(
    props: ToolLayoutProps,
    /// 图标节点（对应真源 `icon` prop 是 ReactNode；这里由调用方直接给 view）。
    icon_view: Option<ChildrenFn>,
    /// 对应真源 `renderContent`（惰性内容）。
    render_content: Option<ChildrenFn>,
) -> impl IntoView {
    // props 的所有取值在这里一次性取出：Rust 侧 props 会被后面的 Effect 闭包按值捕获，
    // 之后再读就报borrow of moved value。真源不存在这个问题（props 是解构后的常量）。
    let props_kind_label = props.summary_kind_label(props.force_open());
    let props_kind_detail = props.summary_kind_detail(props.force_open());
    // kind_detail 的自定义节点（真源 kindDetail 是 ReactNode，Agent 卡传带色
    // 子代理名 span）。与 primary_text_view 同模式：ChildrenFn 可克隆复用。
    let props_kind_detail_view = props.kind_detail_view.clone();
    // 状态位的节点形态（真源 statusLabel 是 ReactNode）——同 kind_detail_view 的可克隆模式。
    let props_status_label_view = props.status_label_view.clone();
    let props_primary_text = props.summary_primary_text(props.force_open());
    let props_secondary_text = props.summary_secondary_text(props.force_open());
    let props_title = props.summary_title(props.force_open());
    let props_kind_label_cls = props.kind_label_class();
    let props_status_label = props.status_label.clone();
    let props_status_indicator_view = props.status_indicator_view.clone();
    let props_status_tooltip = props.status_tooltip.clone();
    let props_source_label = props.source_label.clone();
    let props_diff_count = props.diff_count;
    let props_primary_file_chip = props.primary_file_chip.clone();
    let props_primary_text_view = props.primary_text_view.clone();
    let props_secondary_text_view = props.secondary_text_view.clone();

    // ── :105-114 状态初始化 ──
    // 真源这些值都来自 props 解构（:67-99 已是常量），Effect 闭包里直接用不会有问题；
    // Rust 侧 props 会被闭包按值捕获，所以在开头一次性算成标量，避免各处borrow 冲突。
    let force_open = props.force_open();
    let can_toggle = props.can_toggle();
    let auto_open = props.auto_open();
    let auto_collapse_on_complete = props.auto_collapse_on_complete();
    let show_icon = props.show_icon();
    let is_running = props.is_running();
    let resolved_persist_open_key = props.resolved_persist_open_key();
    // :107-108 默认收起
    let is_open = RwSignal::new(open_state_get(&resolved_persist_open_key).unwrap_or(false));
    // :110 hasSummaryAction（Rust 侧暂无 summaryAction，等 ToolSummaryRow 迁完再接）
    let has_summary_action = false;
    // :111 isExpanded = !hasSummaryAction && (forceOpen || (canToggle && isOpen))
    // Signal::derive：is_expanded 会被多处闭包捕获，派生 Signal 才能满足 Send + Sync。
    let is_expanded = Signal::derive(move || {
        !has_summary_action && (force_open || (can_toggle && is_open.get()))
    });
    // :112 shouldRenderContent 初值 = isExpanded
    let should_render_content = RwSignal::new(is_expanded.get_untracked());
    // :113 isFailureTooltipCopied
    let is_failure_tooltip_copied = RwSignal::new(false);
    // :117 hasAutoOpenedRef
    let has_auto_opened = RwSignal::new(false);
    // :118 previousIsRunningRef
    let previous_is_running = RwSignal::new(is_running);
    // :120 shouldRenderResolvedContent = !hasSummaryAction && (isExpanded || shouldRenderContent)
    let should_render_resolved_content = Signal::derive(move || {
        !has_summary_action && (is_expanded.get() || should_render_content.get())
    });

    // ── :154-166 autoOpen：一次性自动展开 ──
    // 真源注释：edit/read 这类工具有"完成后默认自动展开"的需求，但 forceOpen 会把
    // 卡片彻底锁死成不可收起。这里改成一次性的 autoOpen。
    Effect::new({
        let has_auto_opened = has_auto_opened.clone();
        let is_open = is_open.clone();
        let should_render_content = should_render_content.clone();
        let key = resolved_persist_open_key.clone();
        move |_| {
            if !auto_open || has_auto_opened.get_untracked() {
                return;
            }
            open_state_set(&key, true);
            should_render_content.set(true);
            is_open.set(true);
            has_auto_opened.set(true);
        }
    });

    // ── :168-179 autoCollapseOnComplete：running → completed 边沿收起一次 ──
    // 真源注释：子智能体完成后继续展开会把一长串子工具明细永久摊开。
    Effect::new({
        let previous_is_running = previous_is_running.clone();
        let is_open = is_open.clone();
        let key = resolved_persist_open_key.clone();
        move |_| {
            let was_running = previous_is_running.get_untracked();
            previous_is_running.set(is_running);
            if auto_collapse_on_complete && !is_running && was_running {
                open_state_set(&key, false);
                is_open.set(false);
            }
        }
    });

    // ── :186-210 展开/收起时的内容挂载管理 ──
    Effect::new({
        let is_open = is_open.clone();
        let should_render_content = should_render_content.clone();
        move |_| {
            if is_open.get() {
                should_render_content.set(true);
                return;
            }
            if !should_render_content.get_untracked() {
                return;
            }
            // 收起不能立刻卸载：真源注释说 Radix 会在 closed 动画里读
            // --radix-collapsible-content-height，子内容先卸载会让高度变量消失
            // 并继承外层历史消息的高度，详情区瞬间撑成超高空白块。
            spawn_local(async move {
                sleep_js(TOOL_CONTENT_COLLAPSE_UNMOUNT_DELAY_MS).await;
                if !is_open.get_untracked() {
                    should_render_content.set(false);
                }
            });
        }
    });

    // ── :308-313 onOpenChange：forceOpen 完全锁死 ──
    let toggle = {
        let is_open = is_open.clone();
        let key = resolved_persist_open_key.clone();
        move |open: bool| {
            if force_open || !can_toggle {
                return;
            }
            open_state_set(&key, open);
            is_open.set(open);
        }
    };

    // ── :132-144 摘要行取值 ──
    let expanded = is_expanded.get();
    // 展开态文案（:132-141）。props_* 是开头一次性取出的副本，这里转成 owned便于 view 用。
    let kind_label = props_kind_label.map(str::to_string);
    let kind_detail = props_kind_detail.map(str::to_string);
    let primary_text = props_primary_text.map(str::to_string);
    let secondary_text = props_secondary_text.map(str::to_string);
    let summary_title = props_title.map(str::to_string);
    let show_diff_count = props.should_show_diff_count(expanded);
    let show_status_label = props.should_show_status_label();

    // ── :140 ToolSummaryRow 的节点槽（真源传 ReactNode，见 ToolSummaryRow::SummaryNode）──
    // primaryText 的三档优先级与真源一致：文件 chip > 自定义节点 > 纯文本。
    let primary_node = match (
        props_primary_file_chip.clone(),
        props_primary_text_view.clone(),
    ) {
        (Some(chip), _) => {
            let cls_static = "inline-flex min-w-0 max-w-full items-center gap-1.5 text-foreground-subtle";
            let cls_clickable =
                "inline-flex min-w-0 max-w-full cursor-pointer items-center gap-1.5 text-foreground-subtle hover:underline";
            let f: ChildrenFn = Arc::new(move || {
                let path = chip.path.clone();
                let icon_src = chip.icon_src.clone();
                let file_name = chip.file_name.clone();
                view! {
                    <span class=if chip.clickable { cls_clickable } else { cls_static } title=path>
                        <img src=icon_src class="size-4 flex-none" alt="" />
                        <span class="min-w-0 truncate">{file_name}</span>
                    </span>
                }
                .into_any()
            });
            SummaryNode::View(f)
        }
        (None, Some(f)) => SummaryNode::View(f),
        (None, None) => primary_text.map(SummaryNode::Text).unwrap_or_default(),
    };
    let secondary_node = match props_secondary_text_view.clone() {
        Some(f) => SummaryNode::View(f),
        None => secondary_text.map(SummaryNode::Text).unwrap_or_default(),
    };
    // kindDetail 同理：节点优先于文本（真源 Agent 卡带色子代理名）。
    let kind_detail_node = match props_kind_detail_view.clone() {
        Some(f) => SummaryNode::View(f),
        None => kind_detail.map(SummaryNode::Text).unwrap_or_default(),
    };

    // statusNode（真源 :294-302）整段在这里造完，组件侧按 :159 裸插、不再包 span。
    let status_node = show_status_label.then(|| {
        // 真源 :250-254：有 statusTooltip 时挂虚线下划线 + tooltip，
        // 失败态不再强制展开内容，错误详情改挂状态词 tooltip。
        let cls = if props_status_tooltip.is_some() {
            "whitespace-nowrap underline decoration-dotted underline-offset-2 cursor-help"
        } else {
            "whitespace-nowrap"
        };
        let label_view = props_status_label_view.clone();
        let label_text = props_status_label.clone();
        let tooltip = props_status_tooltip.clone();
        let indicator = props_status_indicator_view.clone();
        let copied = is_failure_tooltip_copied;
        let has_tooltip = props_status_tooltip.is_some();
        let f: ChildrenFn = Arc::new(move || {
            // 节点优先于文本（真源把 statusLabel 原样塞进这层 span，
            // 圆点 + 词那类组合由调用方给节点，包装层与 tooltip 都归本组件）。
            let inner = label_view
                .clone()
                .map(|g| g())
                .or_else(|| {
                    label_text
                        .clone()
                        .map(|label| view! { <>{label}</> }.into_any())
                });
            let word = inner.map(|node| view! {
                <span class=cls title=tooltip.clone().unwrap_or_default()>{node}</span>
            });
            // 真源 :294-302 —— 有 statusIndicator 时把灯与状态词包进一个
            // inline-flex 小组（gap-1.5），否则状态词自己站。
            let word = match (word, indicator.clone()) {
                (None, None) => ().into_any(),
                (Some(word_node), None) => word_node.into_any(),
                (word_node, Some(ind)) => view! {
                    <span class="inline-flex shrink-0 items-center gap-1.5">
                        {ind()}
                        {word_node}
                    </span>
                }
                .into_any(),
            };
            // 复制按钮（真源 :257-285，在 tooltip 内容里）。Rust 侧没做 Radix
            // tooltip，按钮仍跟在状态词后面，位置与改造前的行内标记一致。
            view! {
                <>
                    {word}
                    {has_tooltip.then(|| view! { <FailureCopyButton copied=copied /> })}
                </>
            }
            .into_any()
        });
        f
    });

    // diffCount（真源 :147 是当作 prop 传进摘要行的，不是画在行外）。
    let diff_count_node = show_diff_count
        .then(|| props_diff_count)
        .flatten()
        .map(|(added, removed)| {
            let f: ChildrenFn = Arc::new(move || view! {
                // renderers.tsx:31-62
                <span class="inline-flex items-center gap-1 whitespace-nowrap font-mono leading-none tabular-nums">
                    {(added > 0).then(|| view! {
                        <span class="inline-flex items-center text-diff-added">
                            {format!("+{added}")}
                        </span>
                    })}
                    {(removed > 0).then(|| view! {
                        <span class="inline-flex items-center text-diff-removed">
                            {format!("-{removed}")}
                        </span>
                    })}
                </span>
            }
            .into_any());
            f
        });

    // 真源 :341-343 toggleAriaLabel 按展开态换两条文案。
    let toggle_aria_label = i18n::text(if expanded {
        "chat.toolCall.collapseDetails"
    } else {
        "chat.toolCall.expandDetails"
    });

    let row_props = ToolSummaryRowProps {
        can_toggle,
        force_open,
        is_expanded: expanded,
        show_icon,
        kind_label: kind_label.map(SummaryNode::Text).unwrap_or_default(),
        kind_label_class_name: Some(props_kind_label_cls.to_string()),
        kind_detail: kind_detail_node,
        source_label: props_source_label.clone().map(SummaryNode::Text).unwrap_or_default(),
        primary_text: primary_node,
        secondary_text: secondary_node,
        prioritize_primary_text: props.prioritize_primary_text(),
        separator: props
            .summary_content_separator
            .clone()
            .map(SummaryNode::Text)
            .unwrap_or_default(),
        diff_count: diff_count_node.map(SummaryNode::View).unwrap_or_default(),
        status_node: status_node.map(SummaryNode::View).unwrap_or_default(),
        title: summary_title.clone(),
        toggle_aria_label,
        tool_id: props.tool_id.clone(),
        content_key: props.resolved_summary_content_key(expanded),
        content_refresh_version: None,
        animate_content: props.animate_summary_content.unwrap_or(false),
        disable_content_animation: props
            .disable_summary_content_animation
            .unwrap_or(false),
    };
    // 组件的 canToggle 分支把点击与 Enter/Space 都收敛到这一个回调（真源 :194/:199-208）。
    let row_on_activate = {
        let is_open = is_open.clone();
        let toggle = toggle.clone();
        Callback::new(move |_| {
            if can_toggle {
                toggle(!is_open.get_untracked());
            }
        })
    };

    view! {
        // ── :305-358 Collapsible 外壳 ──
        <div
            class="flex w-full flex-col"
            data-state=move || if is_expanded.get() { "open" } else { "closed" }
            data-open=move || is_expanded.get()
        >
            // ── :319-345 ToolSummaryRow ──
            // 行结构、类名、aria、chevron 全归 ToolSummaryRow 组件（真源就是这么分的），
            // ToolLayout 只负责把上面算好的节点塞进 props。
            <ToolSummaryRowComponent
                props=row_props
                icon=icon_view.clone()
                on_activate=row_on_activate
            />
            // ── :322-338 内容区 ──
            {{
                let show = should_render_resolved_content.get();
                show
                    .then(|| render_content.clone())
                    .flatten()
                    .map(|f| view! {
                        <div class=TOOL_CONTENT_SHELL_CLASSNAME>
                            <div class=TOOL_CONTENT_SPACING_CLASSNAME>{f()}</div>
                        </div>
                    })
            }}
        </div>
    }
}

/// 失败详情复制按钮（真源 :262-275）。
///
/// 点击后 1.5s 内图标变对勾（`FAILURE_TOOLTIP_COPY_RESET_MS`）。
#[component]
fn FailureCopyButton(copied: RwSignal<bool>) -> impl IntoView {
    // 两个闭包各持一份副本：set 闭包与 async 块都会用到。
    let copied_click = copied.clone();
    let copied_reset = copied.clone();
    let on_click = Callback::new(move |_: ()| {
        copied_click.set(true);
        let copied_reset = copied_reset.clone();
        spawn_local(async move {
            sleep_js(FAILURE_TOOLTIP_COPY_RESET_MS).await;
            copied_reset.set(false);
        });
    });
    let is_copied = copied.get_untracked();
    view! {
        <span
            class="ml-1 inline-flex size-6 flex-none cursor-pointer items-center justify-center rounded-md text-foreground-subtle hover:bg-surface-hover"
            role="button"
            title=if is_copied { "已复制" } else { "复制错误详情" }
            on:click=move |ev| {
                ev.stop_propagation();
                on_click.run(());
            }
        >
            <span class="flex size-3 items-center justify-center">
                {if is_copied {
                    icon_check().into_any()
                } else {
                    icon_copy().into_any()
                }}
            </span>
        </span>
    }
}

/// wasm 侧的 setTimeout 包装（真源用 `window.setTimeout`）。
///
/// Leptos 没有 task::sleep，用 JsPromise 包 setTimeout 等价实现。
async fn sleep_js(ms: u64) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_props(tool_id: &str) -> ToolLayoutProps {
        ToolLayoutProps {
            tool_id: tool_id.into(),
            primary_text: Some("src/main.rs".into()),
            ..Default::default()
        }
    }

    #[test]
    fn defaults_match_source_destructuring() {
        // 真源 :67-99 的默认值逐条对齐。
        let p = base_props("t1");
        assert!(p.show_icon());
        assert!(p.can_toggle());
        assert!(!p.force_open());
        assert!(!p.auto_open());
        assert!(!p.auto_collapse_on_complete());
        assert!(!p.prioritize_primary_text());
        assert!(!p.hide_secondary_text_when_open());
        assert!(!p.hide_diff_count_when_open());
        assert!(!p.show_status_label());
        assert!(!p.show_failure_status());
        assert!(!p.is_running());
    }

    #[test]
    fn explicit_false_overrides_default() {
        // showIcon={false} 要能覆盖默认 true——所以用 Option<bool> 而非 bool。
        let p = ToolLayoutProps {
            show_icon: Some(false),
            can_toggle: Some(false),
            ..base_props("t1")
        };
        assert!(!p.show_icon());
        assert!(!p.can_toggle());
    }

    #[test]
    fn resolved_key_falls_back_to_tool_id() {
        // 真源 :105 resolvedPersistOpenKey = persistOpenKey ?? toolId
        let p = base_props("t1");
        assert_eq!(p.resolved_persist_open_key(), "t1");
        let p2 = ToolLayoutProps {
            persist_open_key: Some("custom".into()),
            ..base_props("t1")
        };
        assert_eq!(p2.resolved_persist_open_key(), "custom");
    }

    #[test]
    fn open_state_defaults_to_collapsed_and_persists() {
        // 真源 :107-108 toolLayoutOpenState.get(key) ?? false
        let key = "tool-layout-test-unique";
        assert_eq!(open_state_get(key), None, "新 key 应无记录");
        open_state_set(key, true);
        assert_eq!(open_state_get(key), Some(true));
        open_state_set(key, false);
        assert_eq!(open_state_get(key), Some(false));
    }

    #[test]
    fn kind_label_uses_gradient_when_running() {
        // 真源 :143-147
        let running = ToolLayoutProps {
            is_running: Some(true),
            ..base_props("t1")
        };
        assert!(
            running
                .kind_label_class()
                .contains("animated-gradient-text")
        );
        let done = base_props("t1");
        assert!(!done.kind_label_class().contains("animated-gradient-text"));
        assert!(done.kind_label_class().contains("text-foreground-subtlest"));
    }

    #[test]
    fn expanded_text_overrides_each_field() {
        // 真源 :132-141 五组展开态文案覆盖。
        let p = ToolLayoutProps {
            primary_text: Some("a.ts".into()),
            expanded_primary_text: Some("3 个文件".into()),
            kind_label: Some("读取".into()),
            expanded_kind_label: Some("正在读取".into()),
            kind_detail: Some("d1".into()),
            expanded_kind_detail: Some("d2".into()),
            title: Some("t1".into()),
            expanded_title: Some("t2".into()),
            ..base_props("t1")
        };
        assert_eq!(p.summary_primary_text(false), Some("a.ts"));
        assert_eq!(p.summary_primary_text(true), Some("3 个文件"));
        assert_eq!(p.summary_kind_label(false), Some("读取"));
        assert_eq!(p.summary_kind_label(true), Some("正在读取"));
        assert_eq!(p.summary_kind_detail(false), Some("d1"));
        assert_eq!(p.summary_kind_detail(true), Some("d2"));
        assert_eq!(p.summary_title(false), Some("t1"));
        assert_eq!(p.summary_title(true), Some("t2"));
    }

    #[test]
    fn secondary_text_has_three_level_fallback() {
        // 真源 :135-140 summarySecondaryText 三层回落。
        // ① expandedSecondaryText 优先
        let p1 = ToolLayoutProps {
            secondary_text: Some("npm test".into()),
            expanded_secondary_text: Some("npm run build".into()),
            ..base_props("t1")
        };
        assert_eq!(p1.summary_secondary_text(false), Some("npm test"));
        assert_eq!(p1.summary_secondary_text(true), Some("npm run build"));

        // ② hideSecondaryTextWhenOpen 时展开态为 null
        let p2 = ToolLayoutProps {
            secondary_text: Some("npm test".into()),
            hide_secondary_text_when_open: Some(true),
            ..base_props("t1")
        };
        assert_eq!(p2.summary_secondary_text(false), Some("npm test"));
        assert_eq!(p2.summary_secondary_text(true), None);

        // ③ 未设置时保持 None
        let p3 = base_props("t1");
        assert_eq!(p3.summary_secondary_text(true), None);
    }

    #[test]
    fn diff_count_hidden_when_open_and_flagged() {
        // 真源 :144 shouldShowDiffCount = diffCount != null && !(isExpanded && hideDiffCountWhenOpen)
        let p = ToolLayoutProps {
            diff_count: Some((3, 1)),
            hide_diff_count_when_open: Some(true),
            ..base_props("t1")
        };
        assert!(p.should_show_diff_count(false), "收起时应显示");
        assert!(!p.should_show_diff_count(true), "展开且要求隐藏时不应显示");

        // 没开 hide 时两种状态都显示。
        let p2 = ToolLayoutProps {
            diff_count: Some((3, 1)),
            ..base_props("t1")
        };
        assert!(p2.should_show_diff_count(true));
        // 没传 diffCount 时永不显示。
        assert!(!base_props("t1").should_show_diff_count(false));
    }

    #[test]
    fn status_label_requires_flag_and_non_null_label() {
        // 真源 :114 (showStatusLabel || showFailureStatus) && statusLabel != null
        let none_flag = ToolLayoutProps {
            status_label: Some("执行中".into()),
            ..base_props("t1")
        };
        assert!(
            !none_flag.should_show_status_label(),
            "两个 flag 都没开时不显示"
        );

        let show = ToolLayoutProps {
            status_label: Some("执行中".into()),
            show_status_label: Some(true),
            ..base_props("t1")
        };
        assert!(show.should_show_status_label());

        let failure = ToolLayoutProps {
            status_label: Some("执行失败".into()),
            show_failure_status: Some(true),
            ..base_props("t1")
        };
        assert!(failure.should_show_status_label(), "失败态强制显示");

        // ★真源的 statusLabel 是 ReactNode（观察/恢复一类卡在状态位里画「圆点 + 词」），
        // 节点形态走 status_label_view，同样算「statusLabel != null」。
        let view_only = ToolLayoutProps {
            status_label_view: Some(std::sync::Arc::new(|| {
                view! { <span class="flex shrink-0 items-center gap-1.5">"后台运行中"</span> }.into_any()
            })),
            show_status_label: Some(true),
            ..base_props("t1")
        };
        assert!(
            view_only.should_show_status_label(),
            "只有节点、没有文本时也要显示状态位"
        );
        let view_no_flag = ToolLayoutProps {
            status_label_view: view_only.status_label_view.clone(),
            ..base_props("t1")
        };
        assert!(
            !view_no_flag.should_show_status_label(),
            "两个 flag 都没开时节点也不显示（与文本同规则）"
        );

        let null_label = ToolLayoutProps {
            show_status_label: Some(true),
            ..base_props("t1")
        };
        assert!(
            !null_label.should_show_status_label(),
            "statusLabel 为 null 时不显示"
        );
    }

    #[test]
    fn summary_content_key_format() {
        // 真源 :142-143 `${String(summaryTitle ?? "")}:${String(statusLabel ?? "")}`
        let p = ToolLayoutProps {
            title: Some("读取文件".into()),
            status_label: Some("已完成".into()),
            ..base_props("t1")
        };
        assert_eq!(p.resolved_summary_content_key(false), "读取文件:已完成");
        // 两个都缺省时是 ":"。
        assert_eq!(base_props("t1").resolved_summary_content_key(false), ":");
    }

    #[test]
    fn collapse_unmount_delay_constant() {
        // 真源 :19
        assert_eq!(TOOL_CONTENT_COLLAPSE_UNMOUNT_DELAY_MS, 300);
    }
}
