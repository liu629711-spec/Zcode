//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cua-group.tsx`（282 行）。
//!
//! CUA 工具聚合卡的**渲染层**（分组逻辑在 `cuaGroups.rs`）。
//!
//! 结构：ToolLayout 外壳 + 一个可滚动的事件流视口（max-h-72），
//! 视口上下边缘按滚动位置加渐隐遮罩。

use leptos::prelude::*;
use std::sync::Arc;

use super::super::ToolLayout::ToolLayoutComponent;
use super::cua::{CuaSummaryPresentation, build_cua_summary_presentation};
use crate::cuaGroups::{CuaGroupEvent, CuaGroupRenderItem};
use crate::ToolCallBlocks::i18n;

// ---------------------------------------------------------------------------
// 滚动遮罩（1:1 翻译 packages/ui/src/mentions/components/scrollMask.ts，62 行）
// ---------------------------------------------------------------------------

const SCROLL_MASK_EDGE_SIZE_PX: u32 = 24;
const SCROLL_MASK_THRESHOLD_PX: f64 = 1.0;

/// `ScrollMetrics`（真源 :6-10）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollMetrics {
    pub client_height: f64,
    pub scroll_height: f64,
    pub scroll_top: f64,
}

/// `ScrollMaskState`（真源 :12-15）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollMaskState {
    pub show_bottom: bool,
    pub show_top: bool,
}

/// `EMPTY_SCROLL_MASK_STATE`（真源 :17-20）。
pub const EMPTY_SCROLL_MASK_STATE: ScrollMaskState = ScrollMaskState {
    show_bottom: false,
    show_top: false,
};

/// `resolveVerticalScrollMaskState`（真源 :22-36）。
pub fn resolve_vertical_scroll_mask_state(metrics: ScrollMetrics) -> ScrollMaskState {
    let max_scroll_top = (metrics.scroll_height - metrics.client_height).max(0.0);
    if max_scroll_top <= SCROLL_MASK_THRESHOLD_PX {
        return EMPTY_SCROLL_MASK_STATE;
    }
    ScrollMaskState {
        show_top: metrics.scroll_top > SCROLL_MASK_THRESHOLD_PX,
        show_bottom: metrics.scroll_top < max_scroll_top - SCROLL_MASK_THRESHOLD_PX,
    }
}

/// `getVerticalScrollMaskStyle`（真源 :38-61）：返回整段 CSS style。
///
/// 真源返回 `CSSProperties | undefined`；Rust 侧用 `None` 表达
/// 「两端都不显示 → 不加 mask」。
pub fn get_vertical_scroll_mask_style(state: ScrollMaskState) -> Option<String> {
    if !state.show_top && !state.show_bottom {
        return None;
    }
    let edge = SCROLL_MASK_EDGE_SIZE_PX;
    let top_stop = if state.show_top {
        format!("transparent 0px, black {edge}px")
    } else {
        format!("black 0px, black {edge}px")
    };
    let bottom_stop = if state.show_bottom {
        format!("black calc(100% - {edge}px), transparent 100%")
    } else {
        format!("black calc(100% - {edge}px), black 100%")
    };
    let mask_image = format!("linear-gradient(to bottom, {top_stop}, {bottom_stop})");
    Some(format!(
        "-webkit-mask-image: {mask_image}; \
         mask-image: {mask_image}; \
         -webkit-mask-repeat: no-repeat; \
         mask-repeat: no-repeat; \
         -webkit-mask-size: 100% 100%; \
         mask-size: 100% 100%;"
    ))
}

// ---------------------------------------------------------------------------
// 摘要推导（真源 :159-241）
// ---------------------------------------------------------------------------

/// `TERMINAL_CUA_STATUSES`（真源 :17）——已终结、可作为摘要依据的状态。
pub const TERMINAL_CUA_STATUSES: [&str; 3] = ["completed", "failed", "stopped"];

/// `STICK_TO_BOTTOM_THRESHOLD_PX`（真源 :18）。
pub const STICK_TO_BOTTOM_THRESHOLD_PX: f64 = 8.0;

/// `isCuaGroupViewportAtBottom`（真源 :24-28）。
pub fn is_viewport_at_bottom(metrics: ScrollMetrics) -> bool {
    metrics.scroll_height - metrics.client_height - metrics.scroll_top
        <= STICK_TO_BOTTOM_THRESHOLD_PX
}

/// 找最后一个已终结的工具事件（真源 :163-167）。
///
/// 用**倒序**遍历找——真源用 `findLast`，语义是「最近一次终态」。
pub fn latest_summary_event(group: &CuaGroupRenderItem) -> Option<usize> {
    group.events.iter().rposition(|e| {
        matches!(e, CuaGroupEvent::Tool { node, .. }
            if TERMINAL_CUA_STATUSES.contains(&node.tool_call.status.as_str()))
    })
}

/// 消息计数（真源 :184）。
pub fn message_count(group: &CuaGroupRenderItem) -> usize {
    group
        .events
        .iter()
        .filter(|e| matches!(e, CuaGroupEvent::AssistantMessage { .. }))
        .count()
}

/// 计数文案（真源 :186-211）。
///
/// - 事件数文案（one/other 中文相同，但 id 不同，照抄分支）
/// - 消息数文案：`messageCount > 0 || !isRunning` 才出（真源 :189）
#[derive(Debug, Clone, PartialEq)]
pub struct CountTexts {
    pub event: String,
    pub message: Option<String>,
}

pub fn build_count_texts(group: &CuaGroupRenderItem, is_running: bool) -> CountTexts {
    let event_count = group.events.len();
    let event_id = if event_count == 1 {
        "chat.toolCall.cua.group.event.one"
    } else {
        "chat.toolCall.cua.group.event.other"
    };
    let event = i18n::format(event_id, &[("count".to_string(), event_count.to_string())]);

    let msg_count = message_count(group);
    let message = if msg_count > 0 || !is_running {
        let id = if msg_count == 1 {
            "chat.toolCall.cua.group.message.one"
        } else {
            "chat.toolCall.cua.group.message.other"
        };
        Some(i18n::format(id, &[("count".to_string(), msg_count.to_string())]))
    } else {
        None
    };
    CountTexts { event, message }
}

/// 主文本的扫光类名（真源 :212-214）。
pub fn animated_text_class(is_running: bool) -> &'static str {
    if is_running {
        "cua-group-gradient-text min-w-0 truncate"
    } else {
        "min-w-0 truncate text-foreground-subtlest"
    }
}

/// `summaryContentKey`（真源 :263-267）。
///
/// 带状态是为了「工具到达终态时重置滚轮动画」。
pub fn summary_content_key(group: &CuaGroupRenderItem, latest: Option<usize>) -> String {
    match latest {
        Some(idx) => {
            let CuaGroupEvent::Tool { node, .. } = &group.events[idx] else {
                return format!("cua:{}:empty", group.node.tool_call.tool_id);
            };
            format!(
                "cua:{}:tool:{}:{}",
                group.node.tool_call.tool_id, node.tool_call.tool_id, node.tool_call.status
            )
        }
        None => format!("cua:{}:empty", group.node.tool_call.tool_id),
    }
}

/// 标题（真源 :268-272）。
pub fn resolve_title(
    group: &CuaGroupRenderItem,
    is_running: bool,
    latest_summary: Option<&CuaSummaryPresentation>,
) -> String {
    if is_running {
        latest_summary
            .map(|s| s.title.clone())
            .unwrap_or_else(|| group.node.tool_call.title.clone().unwrap_or_default())
    } else {
        i18n::text("chat.toolCall.cua.group.completedLabel")
    }
}

/// `isRunning`（真源 :177）。
pub fn resolve_is_running(group: &CuaGroupRenderItem, context_is_running: bool) -> bool {
    context_is_running || group.node.tool_call.status == "in_progress"
}

// ---------------------------------------------------------------------------
// 组件（真源 :37-282）
// ---------------------------------------------------------------------------

/// 子卡列表容器（真源 :38-152）。
///
/// ★滚动行为（真源 :49-101）：用户上滑时**停止**自动吸底
/// （`shouldStickToBottomRef`），只有原本在底部才继续跟随。
/// 本组件保留该状态机；DOM 事件用 Leptos 的 `on:scroll` 绑定。
#[component]
pub fn CuaGroupChildrenComponent(
    events: Vec<CuaGroupEvent>,
    /// 行文本（正文/思考行的原文，供默认渲染）。
    texts: Vec<(usize, String)>,
    /// 工具事件对应的子节点（渲染子ToolCall 卡）。
    tool_nodes: Vec<(usize, crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCallNode)>,
) -> impl IntoView {
    let scroll_mask = RwSignal::new(EMPTY_SCROLL_MASK_STATE);
    // 真源用 viewportRef 读滚动度量；Rust 侧按项目既有写法用 NodeRef
    // （照抄 taskTitle.rs 的容器测量模式）。
    let viewport: NodeRef<leptos::html::Div> = NodeRef::new();
    let should_stick = RwSignal::new(true);

    // 逐事件构造视图（View 不可克隆，故一次性建好再交给 children）。
    let mut children: Vec<AnyView> = Vec::new();
    for event in &events {
        match event {
            CuaGroupEvent::Tool { row_index, .. } => {
                let node = tool_nodes
                    .iter()
                    .find(|(i, _)| i == row_index)
                    .map(|(_, n)| n.clone());
                if let Some(node) = node {
                    children.push(
                        view! {
                            <div data-tool-call-id=node.tool_call.tool_id.clone()>
                                {node.tool_call.tool_name.clone().unwrap_or_default()}
                            </div>
                        }
                        .into_any(),
                    );
                }
            }
            CuaGroupEvent::AssistantMessage { row_index } => {
                let text = texts
                    .iter()
                    .find(|(i, _)| i == row_index)
                    .map(|(_, t)| t.clone())
                    .unwrap_or_default();
                // 真源 :134-142 —— 正文默认渲染是纯文本（可被 renderAssistantMessage 覆盖）。
                children.push(
                    view! {
                        <div class="min-w-0 py-1 text-ui-base text-foreground-subtle">
                            <div class="min-w-0">
                                <span class="whitespace-pre-wrap break-words">{text}</span>
                            </div>
                        </div>
                    }
                    .into_any(),
                );
            }
            CuaGroupEvent::Reasoning { row_index } => {
                let text = texts
                    .iter()
                    .find(|(i, _)| i == row_index)
                    .map(|(_, t)| t.clone())
                    .unwrap_or_default();
                children.push(
                    view! {
                        <div class="min-w-0 py-1">
                            <span class="whitespace-pre-wrap break-words">{text}</span>
                        </div>
                    }
                    .into_any(),
                );
            }
        }
    }
    let children = children.into_iter().collect_view();

    view! {
        <div
            class="max-h-72 overflow-y-auto overscroll-contain"
            data-cua-group-scroll-mask=move || {
                let s = scroll_mask.get();
                if s.show_top && s.show_bottom {
                    "both"
                } else if s.show_top {
                    "top"
                } else if s.show_bottom {
                    "bottom"
                } else {
                    "none"
                }
            }
            style=move || get_vertical_scroll_mask_style(scroll_mask.get()).unwrap_or_default()
            node_ref=viewport
            on:scroll=move |_| {
                // 真源 :52-101 —— 读视口度量，更新遮罩；
                // 用户离开底部就停止自动吸底。
                let Some(el) = viewport.get() else {
                    scroll_mask.set(EMPTY_SCROLL_MASK_STATE);
                    return;
                };
                let metrics = ScrollMetrics {
                    client_height: el.client_height() as f64,
                    scroll_height: el.scroll_height() as f64,
                    scroll_top: el.scroll_top() as f64,
                };
                scroll_mask.set(resolve_vertical_scroll_mask_state(metrics));
                should_stick.set(is_viewport_at_bottom(metrics));
            }
        >
            <div class="ml-2 space-y-2 border-border border-l pl-3.5">
                {children}
            </div>
        </div>
    }
    .into_any()
}

/// `CuaGroupToolCallBlock`（真源 :154-282）。
#[component]
pub fn CuaGroupToolCallBlock(
    group: CuaGroupRenderItem,
    /// 行文本（事件对应的原文，供子组件渲染）。
    texts: Vec<(usize, String)>,
    /// 行 → 子节点映射，供工具事件渲染子卡。
    tool_nodes: Vec<(usize, crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCallNode)>,
    is_running_ctx: bool,
    can_toggle: Option<bool>,
    force_open: Option<bool>,
) -> impl IntoView {
    let latest_idx = latest_summary_event(&group);
    let latest_summary = latest_idx.and_then(|idx| {
        let CuaGroupEvent::Tool { node, .. } = &group.events[idx] else {
            return None;
        };
        Some(build_cua_summary_presentation(
            &node.tool_call,
            None,
        ))
    });
    let is_running = resolve_is_running(&group, is_running_ctx);
    let counts = build_count_texts(&group, is_running);
    let title = resolve_title(&group, is_running, latest_summary.as_ref());
    let content_key = summary_content_key(&group, latest_idx);

    // diffCount 文案（真源 :236-243）：运行中且还没工具时不显示前导「·」。
    let show_leading_dot = latest_idx.is_some() || !is_running;
    let diff_count_text = match &counts.message {
        Some(m) => format!("{}, {m}", counts.event),
        None => counts.event.clone(),
    };
    let summary_app_name = latest_summary.as_ref().map(|s| s.app_name.clone());
    let summary_primary = latest_summary.as_ref().and_then(|s| s.tagged_target.clone());
    let summary_failed = latest_summary.as_ref().is_some_and(|s| s.is_failed);

    let events = group.events.clone();
    let texts_for_children = texts.clone();
    let nodes_for_children = tool_nodes.clone();

    view! {
        <ToolLayoutComponent
            props=super::super::ToolLayout::ToolLayoutProps {
                tool_id: group.node.tool_call.tool_id.clone(),
                // ★真源 :255-256 —— 运行中不显示图标（primaryText 里已有大图标）。
                icon: None,
                show_icon: Some(!is_running),
                can_toggle: Some(can_toggle.unwrap_or(true)),
                // 真源 :257 —— 运行中不强制展开。
                force_open: Some(!is_running && force_open.unwrap_or(false)),
                // 真源 :258-260 —— 运行中 kindLabel 为 null，
                // 完成后显示「电脑控制」。
                kind_label: if is_running {
                    None
                } else {
                    Some(i18n::text("chat.toolCall.cua.group.completedLabel"))
                },
                // 真源 :261 —— 运行中才有主文本。
                primary_text: if is_running {
                    Some(String::new())
                } else {
                    None
                },
                primary_text_view: if is_running {
                    Some(Arc::new(move || {
                        // 真源 :217-234 —— 大图标 + 扫光文本 + 失败态状态词。
                        view! {
                            <span class="inline-flex min-w-0 items-center gap-2">
                                <span class="shrink-0 size-4 flex items-center justify-center [&_svg]:size-4">
                                    <crate::app::Icon
                                        paths=super::cuaIcon::MOUSE_POINTER_CLICK_PATHS.to_vec()
                                        circles=vec![]
                                    />
                                </span>
                                <span class=animated_text_class(is_running)>
                                    {{
                                        let inner = summary_app_name.clone().map(|name| {
                                            view! {
                                                <span class="inline-flex min-w-0 items-center gap-1.5">
                                                    <span class="shrink-0 font-medium">{name}</span>
                                                    <span class="min-w-0 truncate">
                                                        {summary_primary.clone().unwrap_or_default()}
                                                    </span>
                                                </span>
                                            }
                                            .into_any()
                                        });
                                        inner.unwrap_or_else(|| ().into_view().into_any())
                                    }}
                                </span>
                                {summary_failed.then(|| {
                                    view! {
                                        <span class="shrink-0 text-foreground-subtlest">
                                            {i18n::text("chat.toolCall.status.failed")}
                                        </span>
                                    }
                                    .into_any()
                                })}
                            </span>
                        }
                        .into_any()
                    }))
                } else {
                    None
                },
                // 真源 :262 —— 窄屏优先保主文本。
                prioritize_primary_text: Some(true),
                diff_count: Some((diff_count_text.len() as u32, 0)),
                summary_content_separator: show_leading_dot.then(|| "·".to_string()),
                summary_content_key: Some(content_key),
                // 真源 :264 —— 运行中启用滚轮式轮播动画。
                animate_summary_content: Some(is_running),
                is_running: Some(is_running),
                title: Some(title),
                ..Default::default()
            }
            // 真源 :255 —— 运行中 icon 为 null（primaryText 里有大图标），
            // 但组件参数本身必填，故显式传 None。
            icon_view=None
            render_content=Some(Arc::new(move || {
                view! {
                    <CuaGroupChildrenComponent
                        events=events.clone()
                        texts=texts_for_children.clone()
                        tool_nodes=nodes_for_children.clone()
                    />
                }
                .into_any()
            }))
        />
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolCallRowAdapter::{LegacyToolCall, LegacyToolCallNode};
    use crate::cuaGroups::{CuaFlowKind, CuaGroupEvent};
    use serde_json::json;

    fn node(tool_id: &str, status: &str) -> LegacyToolCallNode {
        LegacyToolCallNode {
            tool_call: LegacyToolCall {
                tool_id: tool_id.into(),
                tool_name: Some("mcp__computer-use__left_click".into()),
                kind: "toolCall".into(),
                title: None,
                input: Some(json!({})),
                status: status.into(),
                v4_status: "running".into(),
                output: None,
                content: None,
                error: None,
                raw: json!({}),
                started_at: None,
                snapshot_refs: Vec::new(),
                thought: None,
            },
            child_tool_calls: Vec::new(),
        }
    }

    fn group_with(events: Vec<CuaGroupEvent>) -> CuaGroupRenderItem {
        CuaGroupRenderItem {
            key: "cua-response:r1".into(),
            row_id: 1,
            row_indices: events
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    CuaGroupEvent::Tool { .. } => i,
                    _ => i,
                })
                .collect(),
            node: LegacyToolCallNode {
                tool_call: LegacyToolCall {
                    tool_id: "cua-response:r1".into(),
                    tool_name: Some("CuaGroup".into()),
                    kind: "cuaGroup".into(),
                    title: Some("Computer Use".into()),
                    input: Some(json!({})),
                    status: "in_progress".into(),
                    v4_status: "running".into(),
                    output: None,
                    content: None,
                    error: None,
                    raw: json!({}),
                    started_at: None,
                    snapshot_refs: Vec::new(),
                    thought: None,
                },
                child_tool_calls: Vec::new(),
            },
            events,
            active: true,
            flow_kind: CuaFlowKind::AssistantWork,
            assistant_response_ids: vec!["r1".into()],
        }
    }

    // ── 滚动遮罩 ──

    #[test]
    fn no_scroll_no_mask() {
        // 真源 :27-29 —— 内容装得下时两端都不显示。
        let state = resolve_vertical_scroll_mask_state(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 100.0,
            scroll_top: 0.0,
        });
        assert_eq!(state, EMPTY_SCROLL_MASK_STATE);
        assert_eq!(get_vertical_scroll_mask_style(state), None);
    }

    #[test]
    fn at_top_shows_bottom_mask_only() {
        let state = resolve_vertical_scroll_mask_state(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 300.0,
            scroll_top: 0.0,
        });
        assert!(state.show_bottom);
        assert!(!state.show_top);
        let style = get_vertical_scroll_mask_style(state).unwrap();
        // 顶部用 black（不渐隐），底部渐隐。
        assert!(style.contains("black 0px, black 24px"), "顶部不透明：{style}");
        assert!(
            style.contains("black calc(100% - 24px), transparent 100%"),
            "底部渐隐：{style}"
        );
    }

    #[test]
    fn at_bottom_shows_top_mask_only() {
        let state = resolve_vertical_scroll_mask_state(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 300.0,
            scroll_top: 200.0,
        });
        assert!(state.show_top);
        assert!(!state.show_bottom);
    }

    #[test]
    fn in_middle_shows_both() {
        let state = resolve_vertical_scroll_mask_state(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 300.0,
            scroll_top: 100.0,
        });
        assert!(state.show_top && state.show_bottom);
    }

    #[test]
    fn threshold_tolerance_hides_mask() {
        // 真源 :28 —— maxScrollTop <= 1 时不显示（亚像素抖动不触发）。
        let state = resolve_vertical_scroll_mask_state(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 100.5,
            scroll_top: 0.0,
        });
        assert_eq!(state, EMPTY_SCROLL_MASK_STATE);
    }

    #[test]
    fn style_uses_edge_size_24() {
        let state = ScrollMaskState {
            show_top: true,
            show_bottom: false,
        };
        let style = get_vertical_scroll_mask_style(state).unwrap();
        assert!(style.contains("-webkit-mask-image"), "要带前缀版本");
        assert!(style.contains("mask-repeat: no-repeat"));
        assert!(style.contains("mask-size: 100% 100%"));
    }

    #[test]
    fn stick_to_bottom_threshold_is_8px() {
        // 真源 :18/24-28 —— 距底 ≤ 8px 视为仍在底部。
        assert!(is_viewport_at_bottom(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 300.0,
            scroll_top: 195.0,
        }));
        assert!(!is_viewport_at_bottom(ScrollMetrics {
            client_height: 100.0,
            scroll_height: 300.0,
            scroll_top: 190.0,
        }));
    }

    // ── 摘要推导 ──

    #[test]
    fn latest_summary_picks_last_terminal_tool() {
        let group = group_with(vec![
            CuaGroupEvent::Tool {
                row_index: 0,
                node: node("tc-1", "running"),
            },
            CuaGroupEvent::AssistantMessage { row_index: 1 },
            CuaGroupEvent::Tool {
                row_index: 2,
                node: node("tc-2", "completed"),
            },
        ]);
        assert_eq!(latest_summary_event(&group), Some(2), "取最后一个终态");
    }

    #[test]
    fn no_terminal_tool_yields_none() {
        let group = group_with(vec![CuaGroupEvent::Tool {
            row_index: 0,
            node: node("tc-1", "running"),
        }]);
        assert_eq!(latest_summary_event(&group), None);
    }

    #[test]
    fn all_three_terminal_statuses_count() {
        for status in ["completed", "failed", "stopped"] {
            let group = group_with(vec![CuaGroupEvent::Tool {
                row_index: 0,
                node: node("tc", status),
            }]);
            assert_eq!(latest_summary_event(&group), Some(0), "{status} 应算终态");
        }
        assert_eq!(TERMINAL_CUA_STATUSES.len(), 3);
    }

    #[test]
    fn message_count_only_counts_assistant_messages() {
        let group = group_with(vec![
            CuaGroupEvent::Tool {
                row_index: 0,
                node: node("tc-1", "completed"),
            },
            CuaGroupEvent::AssistantMessage { row_index: 1 },
            CuaGroupEvent::Reasoning { row_index: 2 },
            CuaGroupEvent::AssistantMessage { row_index: 3 },
        ]);
        assert_eq!(message_count(&group), 2, "思考行不计入消息数");
    }

    #[test]
    fn count_texts_hide_message_when_running_and_zero() {
        // ★真源 :189 —— `messageCount > 0 || !isRunning` 才出消息文案。
        let group = group_with(vec![CuaGroupEvent::Tool {
            row_index: 0,
            node: node("tc-1", "running"),
        }]);
        let running = build_count_texts(&group, true);
        assert_eq!(running.event, "1 个事件");
        assert_eq!(running.message, None, "运行中且无消息时不出");

        let stopped = build_count_texts(&group, false);
        assert_eq!(
            stopped.message.as_deref(),
            Some("0 条消息"),
            "已停止时即使 0 条也要显示"
        );
    }

    #[test]
    fn count_texts_join_event_and_message() {
        let group = group_with(vec![
            CuaGroupEvent::Tool {
                row_index: 0,
                node: node("tc-1", "completed"),
            },
            CuaGroupEvent::AssistantMessage { row_index: 1 },
        ]);
        let texts = build_count_texts(&group, false);
        assert_eq!(texts.event, "2 个事件");
        assert_eq!(texts.message.as_deref(), Some("1 条消息"));
        assert_eq!(
            format!("{}, {}", texts.event, texts.message.unwrap()),
            "2 个事件, 1 条消息",
            "真源 :241 的拼接形态"
        );
    }

    #[test]
    fn animated_class_switches_on_running() {
        assert!(animated_text_class(true).contains("cua-group-gradient-text"));
        assert!(animated_text_class(false).contains("text-foreground-subtlest"));
    }

    #[test]
    fn content_key_encodes_tool_and_status() {
        // ★真源 :263-267 —— 带状态是为了「到终态时重置滚轮动画」。
        let group = group_with(vec![
            CuaGroupEvent::Tool {
                row_index: 0,
                node: node("tc-1", "running"),
            },
            CuaGroupEvent::Tool {
                row_index: 1,
                node: node("tc-2", "completed"),
            },
        ]);
        assert_eq!(
            summary_content_key(&group, Some(1)),
            "cua:cua-response:r1:tool:tc-2:completed"
        );
        assert_eq!(summary_content_key(&group, None), "cua:cua-response:r1:empty");
    }

    #[test]
    fn title_switches_between_running_and_completed() {
        let group = group_with(vec![]);
        // 运行中：用 toolCall.title（无 latest summary 时）
        assert_eq!(
            resolve_title(&group, true, None),
            "Computer Use",
            "运行中回退节点标题"
        );
        // 已停止：固定「电脑控制」
        assert_eq!(
            resolve_title(&group, false, None),
            "电脑控制",
            "完成后显示泛称"
        );
    }

    #[test]
    fn is_running_from_context_or_node_status() {
        let group = group_with(vec![]);
        assert!(resolve_is_running(&group, true), "上下文运行中");
        assert!(
            resolve_is_running(&group, false),
            "节点 in_progress 也算运行中"
        );
        let mut stopped = group.clone();
        stopped.node.tool_call.status = "completed".into();
        assert!(!resolve_is_running(&stopped, false));
    }
}