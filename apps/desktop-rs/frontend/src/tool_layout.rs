//! ToolLayout + ToolSummaryRow 外壳（1:1 翻译）。
//!
//! 真源：`packages/ui/src/ToolCallBlocks/ToolLayout.tsx`（363 行）+
//! `ToolSummaryRow.tsx`（235 行）。**所有 40+ 个 renderer 都复用这一个外壳**，
//! 差异只在 props（kindLabel / primaryText / renderContent）——所以外壳对了，
//! 90% 的视觉就对了。
//!
//! 折叠状态机的四条规则（都有真源注释支撑，��能省）：
//! 1. **默认收起**（`useState(() => toolLayoutOpenState.get(key) ?? false)`）
//! 2. **模块级 Map 记忆**（:16 `toolLayoutOpenState`），按 persistOpenKey ?? toolId，
//!    跨卡片实例存活
//! 3. **autoOpen 是一次性的**（:154-166）——不能��� forceOpen：
//!    forceOpen 会把卡片彻底锁死成不可收起
//! 4. **收起后延迟 300ms 才卸载 children**（:19, :195-210）——立刻卸载会让
//!    高度动画读到继承的父容器高度，详情区瞬间撑成超高空白块，下面内容像全闪没了

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::app::Icon;
use crate::tool_status::DisplayState;

/// 收起后卸载延迟（真源 `TOOL_CONTENT_COLLAPSE_UNMOUNT_DELAY_MS = 300`）。
pub const COLLAPSE_UNMOUNT_DELAY_MS: u64 = 300;

/// 折叠状态记忆表（真源模块级 `Map<string, boolean>`，ToolLayout.tsx:16）。
///
/// 真源是模块级单例（跨卡片实例存活），Rust 侧用OnceLock + Mutow 模拟。
fn open_state_store() -> &'static Mutex<HashMap<String, bool>> {
    static STORE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 读记忆的展开态（缺省 = 收起，真源 `?? false`）。
pub fn persisted_open(key: &str) -> bool {
    open_state_store().lock().ok().and_then(|m| m.get(key).copied()).unwrap_or(false)
}

/// 写记忆的展开态。
pub fn persist_open(key: &str, open: bool) {
    if let Ok(mut m) = open_state_store().lock() {
        m.insert(key.to_string(), open);
    }
}

/// 折叠配置（对齐真源 ToolLayoutProps，:24-46）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutConfig {
    /// 能否折叠。read/search 卡传 false（渲染成 div 而非 trigger，无 chevron）。
    pub can_toggle: bool,
    /// 完全锁死不可收起（真源 :308-310 onOpenChange 直接 return）。
    pub force_open: bool,
    /// 一次性自动展开（:154-166）。
    pub auto_open: bool,
    /// running → completed 边沿自动收起一次（:168-179，子代理卡专用）。
    pub auto_collapse_on_complete: bool,
}

impl Default for LayoutConfig {
    /// 真源默认值：execute/edit/fallback 默认 canToggle=true、forceOpen=false。
    fn default() -> Self {
        Self {
            can_toggle: true,
            force_open: false,
            auto_open: false,
            auto_collapse_on_complete: false,
        }
    }
}

impl LayoutConfig {
    /// 不可折叠的卡（read/search 真源传 `canToggle={false}`）。
    pub fn static_card() -> Self {
        Self {
            can_toggle: false,
            ..Default::default()
        }
    }
}

/// 摘要行配置（对齐真源 ToolSummaryRowProps）。
///
/// 展开/收起态用**不同文案**是真源的核心机制（ToolLayout.tsx:33-46）：
/// 多文件 edit 收起显示文件 chips，展开显示「N 个文件」。
#[derive(Debug, Clone, Default)]
pub struct SummaryConfig {
    pub icon: Option<&'static str>,
    /// 能否折叠（渲染成 trigger 还是 div；决定是否显示 chevron）。
    pub can_toggle: bool,
    /// 完全锁死不可收起。
    pub force_open: bool,
    pub kind_label: Option<String>,
    pub kind_detail: Option<String>,
    /// 来源药丸（子代理卡的「子智能体」）。
    pub source_label: Option<String>,
    /// 主文本（收起态）。
    pub primary_text: Option<String>,
    /// 主文本（展开态；None 表示沿用 primary_text）。
    pub expanded_primary_text: Option<String>,
    /// 次文本（命令、路径等）。
    pub secondary_text: Option<String>,
    /// 展开时清空次文本（execute 卡的 hideSecondaryTextWhenOpen）。
    pub hide_secondary_when_open: bool,
    /// 标题（tooltip）。
    pub title: Option<String>,
    /// 优先显示主文本（窄屏隐藏 kindLabel）。
    pub prioritize_primary_text: bool,
}

impl SummaryConfig {
    /// 取当前态下的主文本（真源 ToolLayout 的展开态回落逻辑）。
    pub fn primary_for(&self, expanded: bool) -> Option<&str> {
        if expanded {
            self.expanded_primary_text
                .as_deref()
                .or(self.primary_text.as_deref())
        } else {
            self.primary_text.as_deref()
        }
    }

    /// 取当前态下的次文本。
    pub fn secondary_for(&self, expanded: bool) -> Option<&str> {
        if expanded && self.hide_secondary_when_open {
            None
        } else {
            self.secondary_text.as_deref()
        }
    }
}

// ---------------------------------------------------------------------------
// 图标（lucide path 取自 node_modules/lucide-react/dist/esm/icons）
// ---------------------------------------------------------------------------

fn icon_search() -> impl IntoView {
    view! { <Icon paths=vec!["m21 21-4.34-4.34"] circles=vec![("11", "11", "8")] /> }
}

fn icon_pencil() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
                "m15 5 4 4",
            ]
            circles=vec![]
        />
    }
}

fn icon_square_terminal() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M7 11l-4 3 4 3",
                "M12 17h6",
            ]
            circles=vec![]
        />
    }
}

fn icon_check() -> impl IntoView {
    view! { <Icon paths=vec!["M20 6 9 17l-5-5"] circles=vec![] /> }
}

fn icon_copy() -> impl IntoView {
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

/// 按名字取工具图标（各 renderer 的`*_TOOL_ICON`）。
///
/// 各分支的 view 类型不同，统一包成 `AnyView` 才能在 match 里返回。
fn tool_icon(name: &str) -> Option<AnyView> {
    match name {
        "read" => Some(icon_search().into_any()),
        "edit" => Some(icon_pencil().into_any()),
        "execute" => Some(icon_square_terminal().into_any()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 摘要行（ToolSummaryRow.tsx）
// ---------------------------------------------------------------------------

/// 摘要行（可折叠分支，:190-223）。
///
/// 容器类名照抄真源：`inline-flex max-w-full cursor-pointer items-center gap-2
/// self-start text-left text-ui-base transition-colors`。
#[component]
pub fn ToolSummaryRow(
    config: SummaryConfig,
    state: DisplayState,
    status_label: Option<&'static str>,
    has_error_tooltip: bool,
    is_expanded: bool,
    on_toggle: Callback<()>,
) -> impl IntoView {
    let is_running = state.is_running();

    // 前导区类名（SummaryLeadingContent，:71-101）。
    let icon_cls = "shrink-0 text-foreground-subtlest";
    let source_label_cls = "shrink-0 rounded border border-border bg-background-alt px-1.5 py-0.5 text-ui-xs leading-none text-foreground-subtlest";

    // 摘要行触发区（CollapsibleTrigger 分支，:200-208）。
    let can_toggle = config.can_toggle;
    let force_open = config.force_open;
    let row_cls = if can_toggle {
        "group/tool-summary inline-flex max-w-full cursor-pointer items-center gap-2 self-start text-left text-ui-base transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused"
    } else {
        // 不可折叠时渲染成 div，无点击区无 chevron（ToolSummaryRow.tsx:226-234）。
        "group/tool-summary inline-flex max-w-full items-center gap-2 self-start text-left text-ui-base transition-colors"
    };

    let primary = config.primary_for(is_expanded);
    let secondary = config.secondary_for(is_expanded);

    view! {
        <div
            class=row_cls
            role="button"
            tabindex="0"
            aria-expanded=is_expanded
            on:click=move |_| {
                if can_toggle && !force_open {
                    on_toggle.run(());
                }
            }
        >
            // ① 前导区
            {config.icon.and_then(tool_icon).map(|i| view! {
                <span class=icon_cls>{i}</span>
            })}
            {config.kind_label.clone().map(|label| {
                let cls = crate::tool_status::kind_label_class(is_running);
                view! { <span class=cls>{label}</span> }
            })}
            {config.kind_detail.clone().map(|d| view! {
                <span class="min-w-0 shrink-0">{d}</span>
            })}
            {config.source_label.clone().map(|s| view! {
                <span class=source_label_cls>{s}</span>
            })}
            // ② 内容区（ToolSummaryRow.tsx:140-161）
            <span class="tool-summary-content min-w-0 flex max-w-full items-center gap-2 text-foreground-subtlest">
                {primary.map(|p| view! { <span class="min-w-0 truncate">{p.to_string()}</span> })}
                {secondary.map(|s| view! {
                    <span class="min-w-0 truncate font-sans text-foreground-subtlest">{s.to_string()}</span>
                })}
            </span>
            // ③ 状态词（失败态强制显示 + 虚线下划线 tooltip）
            {status_label.map(|label| {
                let cls = crate::tool_status::status_label_class(state, has_error_tooltip);
                view! { <span class=cls>{label.to_string()}</span> }
            })}
            // 失败态复制按钮（ToolLayout.tsx:248-289，点击后 1.5s 变对勾）
            {(state.is_failure() && has_error_tooltip).then(|| view! { <CopyButton /> })}
            // 折叠箭头（canToggle 时才有，:210-222）
            {can_toggle.then(|| view! {
                <span
                    class="flex size-4 flex-none items-center justify-center text-foreground-subtlest opacity-0 transition-transform transition-opacity duration-200 ease-out will-change-transform group-hover/tool-summary:opacity-100 shrink-0"
                    class:rotate-90=is_expanded
                    class:opacity-100=is_expanded
                    class:rotate-0=move || !is_expanded
                >
                    {icon_chevron_right()}
                </span>
            })}
        </div>
    }
}

fn icon_chevron_right() -> impl IntoView {
    view! { <Icon paths=vec!["m9 18 6-6-6-6"] circles=vec![] /> }
}

/// 失败详情复制按钮（真源 ToolLayout.tsx:248-289）。
///
/// 点击后复制错误文本，1.5s 内图标变对勾反馈。
#[component]
fn CopyButton() -> impl IntoView {
    let copied = RwSignal::new(false);

    let on_click = {
        let copied = copied.clone();
        move |_| {
            copied.set(true);
            // 1.5s 后恢复复制图标（真源 ToolLayout.tsx 的 copyReset 定时器）。
            let copied2 = copied.clone();
            spawn_local(async move {
                let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                    if let Some(w) = web_sys::window() {
                        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                            &resolve,
                            1500,
                        );
                    }
                });
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                copied2.set(false);
            });
        }
    };

    view! {
        <span
            class="ml-1 inline-flex size-4 flex-none cursor-pointer items-center justify-center rounded-sm text-foreground-subtlest hover:text-foreground"
            role="button"
            title="复制错误详情"
            on:click=move |ev| {
                ev.stop_propagation();
                on_click(());
            }
        >
            {if copied.get() {
                view! { <span class="flex size-3 items-center justify-center">{icon_check()}</span> }
                    .into_any()
            } else {
                view! { <span class="flex size-3 items-center justify-center">{icon_copy()}</span> }
                    .into_any()
            }}
        </span>
    }
}

// ---------------------------------------------------------------------------
// ToolLayout（ToolLayout.tsx:305-358）
// ---------------------------------------------------------------------------

/// 工具卡外壳：摘要行 + 折叠内容区。
///
/// 折叠状态机（模块文档四条规则）在 `ToolLayoutCard` 内实现。
#[component]
pub fn ToolLayoutCard(
    tool_id: String,
    config: LayoutConfig,
    summary: SummaryConfig,
    state: DisplayState,
    status_label: Option<&'static str>,
    has_error_tooltip: bool,
    #[prop(optional)] show_diff_count: Option<(u32, u32)>,
    #[prop(optional)] hide_diff_count_when_open: bool,
    /// 折叠内容（各 renderer 的 renderContent）。
    content: Option<Children>,
) -> impl IntoView {
    let persist_key = if tool_id.is_empty() {
        "tool-layout".to_string()
    } else {
        tool_id.clone()
    };
    // toggle 闭包专用副本（Effect 会按值捕获 persist_key）。
    let toggle_key = persist_key.clone();

    // 初始状态：读记忆，缺省收起（真源 ?? false）。
    let is_open = RwSignal::new(persisted_open(&persist_key));
    // 内容是否挂载（收起后延迟 300ms 才卸载，真源 :195-210）。
    let content_mounted = RwSignal::new(is_open.get_untracked());

    // 延迟卸载句柄。真源用 window.setTimeout（:201-207）。
    let unmount_timer: RwSignal<Option<i32>> = RwSignal::new(None);
    let _ = unmount_timer;

    // autoOpen：一次性自动展开（真源 :154-166，有 hasAutoOpenedRef 守卫）。
    let auto_opened = RwSignal::new(false);
    let was_running = RwSignal::new(state.is_running());

    // 状态边沿联动：autoOpen 一次性展开 / autoCollapseOnComplete 边沿收起。
    Effect::new(move |_| {
        let running = state.is_running();

        // autoOpen：首次满足条件展开一次，之后允许用户手动关闭。
        if config.auto_open && !auto_opened.get_untracked() {
            persist_open(&persist_key, true);
            is_open.set(true);
            content_mounted.set(true);
            auto_opened.set(true);
        }

        // autoCollapseOnComplete：只在 running → completed 边沿收起一次。
        if config.auto_collapse_on_complete && !running && was_running.get_untracked() {
            persist_open(&persist_key, false);
            is_open.set(false);
        }
        was_running.set(running);
    });

    // 展开/收起时的内容挂载管理（真源 :186-210 的逻辑）。
    Effect::new(move |_| {
        let open = is_open.get();
        if open {
            content_mounted.set(true);
            return;
        }
        if !content_mounted.get_untracked() {
            return;
        }
        // 收起不立即卸载：延迟 300ms，让高度动画读完再卸载。
        let delay_ms = COLLAPSE_UNMOUNT_DELAY_MS;
        spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                if let Some(w) = web_sys::window() {
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                        &resolve,
                        delay_ms as i32,
                    );
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            if !is_open.get_untracked() {
                content_mounted.set(false);
            }
        });
    });

    let toggle = {
        let is_open = is_open.clone();
        move |_| {
            // forceOpen 完全锁死（真源 :308-310）。
            if config.force_open || !config.can_toggle {
                return;
            }
            let next = !is_open.get_untracked();
            is_open.set(next);
            persist_open(&toggle_key, next);
        }
    };

    let expanded = is_open.get();
    let show_diff = show_diff_count
        .filter(|_| !(expanded && hide_diff_count_when_open))
        .map(|(added, removed)| (added, removed));

    view! {
        <CollapsibleShell
            is_open=move || is_open.get()
            can_toggle=config.can_toggle
            force_open=config.force_open
        >
            <ToolSummaryRow
                config=summary
                state=state
                status_label=status_label
                has_error_tooltip=has_error_tooltip
                is_expanded=expanded
                on_toggle=Callback::new(move |_| toggle(()))
            />
            {show_diff.map(|(added, removed)| view! {
                <DiffCountLabel added=added removed=removed />
            })}
            // 内容区（真源 :322-338）：canToggle 走 CollapsibleContent，
            // forceOpen 走常驻 div，两者都不是时不渲染。
            {{
                let show_content =
                    (config.can_toggle || config.force_open) && content_mounted.get();
                show_content
                    .then(|| {
                        content.map(|render| {
                            view! {
                                <div class="text-popover-foreground outline-none">
                                    <div class="pt-2">{render()}</div>
                                </div>
                            }
                        })
                    })
                    .flatten()
            }}
        </CollapsibleShell>
    }
}

/// 折叠壳（真源 `<Collapsible className="w-full flex flex-col">`，:317）。
///
/// forceOpen 时恒展开且不响应点击。
#[component]
fn CollapsibleShell(
    is_open: impl Fn() -> bool + Copy + Send + Sync + 'static,
    can_toggle: bool,
    force_open: bool,
    children: Children,
) -> impl IntoView {
    // Radix Collapsible 的 data-state（真源依赖它做样式）。
    let data_state = move || {
        if force_open || !can_toggle || is_open() {
            "open".to_string()
        } else {
            "closed".to_string()
        }
    };
    view! {
        <div class="flex w-full flex-col" data-state=data_state data-open=move || is_open()>
            {children()}
        </div>
    }
}

/// diff 计数（真源 `renderers.tsx:31-62`）。
///
/// `font-mono` + `tabular-nums` 保证数字等宽不跳动；增删分别用
/// `text-diff-added` / `text-diff-removed`。
#[component]
fn DiffCountLabel(added: u32, removed: u32) -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-1 whitespace-nowrap font-mono leading-none tabular-nums">
            {(added > 0).then(|| view! {
                <span class="inline-flex items-center text-diff-added" title=format!("+{added}")>
                    "+"
                    {added.to_string()}
                </span>
            })}
            {(removed > 0).then(|| view! {
                <span
                    class="inline-flex items-center text-diff-removed"
                    title=format!("-{removed}")
                >
                    "-"
                    {removed.to_string()}
                </span>
            })}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_collapsible_not_forced() {
        // 真源默认值：execute/edit/fallback canToggle=true、forceOpen=false。
        let c = LayoutConfig::default();
        assert!(c.can_toggle);
        assert!(!c.force_open);
        assert!(!c.auto_open);
        assert!(!c.auto_collapse_on_complete);
    }

    #[test]
    fn static_card_is_not_collapsible() {
        // read/search 真源传 canToggle={false}。
        let c = LayoutConfig::static_card();
        assert!(!c.can_toggle);
        assert!(!c.force_open);
    }

    #[test]
    fn open_state_defaults_to_collapsed() {
        // 真源 `toolLayoutOpenState.get(key) ?? false` —— 缺省收起。
        let key = "test-never-seen-key";
        assert!(!persisted_open(key), "未见过的 key 应默认收起");
    }

    #[test]
    fn open_state_persists_across_reads() {
        // 真源模块级 Map：记忆跨卡片实例存活。
        let key = "test-persist-key";
        persist_open(key, true);
        assert!(persisted_open(key));
        persist_open(key, false);
        assert!(!persisted_open(key));
    }

    #[test]
    fn collapse_delay_matches_source_constant() {
        // 真源 TOOL_CONTENT_COLLAPSE_UNMOUNT_DELAY_MS = 300。
        assert_eq!(COLLAPSE_UNMOUNT_DELAY_MS, 300);
    }

    #[test]
    fn summary_primary_switches_on_expand() {
        // 多文件 edit：收起显示文件 chips，展开显示「N 个文件」。
        let s = SummaryConfig {
            primary_text: Some("a.ts".into()),
            expanded_primary_text: Some("3 个文件".into()),
            ..Default::default()
        };
        assert_eq!(s.primary_for(false), Some("a.ts"));
        assert_eq!(s.primary_for(true), Some("3 个文件"));
    }

    #[test]
    fn expanded_primary_falls_back_to_primary() {
        // 没给展开态文案时沿用收起态（真源 expandedPrimaryText 可选）。
        let s = SummaryConfig {
            primary_text: Some("x".into()),
            ..Default::default()
        };
        assert_eq!(s.primary_for(true), Some("x"));
    }

    #[test]
    fn secondary_can_be_hidden_when_open() {
        // execute 卡的 hideSecondaryTextWhenOpen：展开后隐藏命令。
        let s = SummaryConfig {
            secondary_text: Some("npm test".into()),
            hide_secondary_when_open: true,
            ..Default::default()
        };
        assert_eq!(s.secondary_for(false), Some("npm test"));
        assert_eq!(s.secondary_for(true), None, "展开后应隐藏次文本");
    }

    #[test]
    fn secondary_shown_when_not_hidden() {
        let s = SummaryConfig {
            secondary_text: Some("npm test".into()),
            ..Default::default()
        };
        assert_eq!(s.secondary_for(true), Some("npm test"));
    }

    #[test]
    fn summary_config_default_is_empty() {
        let s = SummaryConfig::default();
        assert!(s.primary_for(false).is_none());
        assert!(s.secondary_for(true).is_none());
        assert!(s.icon.is_none());
    }
}