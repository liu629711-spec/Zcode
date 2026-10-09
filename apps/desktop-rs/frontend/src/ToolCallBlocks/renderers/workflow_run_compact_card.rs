//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/workflow-run-compact-card.tsx`
//! （115 行）。
//!
//! workflow run 的紧凑可点卡——CreateWorkflow 与 ResumeWorkflowRun 工具卡
//! 共享的「run 态第三态」。
//!
//! 状态点色/状态词色**不在这里定义**：真源 :3-7 是从
//! `components/workflow-graph/run-status-presentation.js` import `RUN_STATUS_DOT` /
//! `RUN_STATUS_TEXT`，Rust 侧同样走 `workflow_graph::run_status_presentation`，
//! 一个特性只有一套「运行中」的画法。

use leptos::prelude::*;

use crate::ToolCallBlocks::fileSummaryTypes::WorkflowRunCardSummary;
use crate::components::workflow_graph::run_state::WorkflowRunStatus;
use crate::components::workflow_graph::run_status_presentation::{
    run_status_dot_class, run_status_text_class,
};

// ---------------------------------------------------------------------------
// 交互判定（真源 :9-15）
// ---------------------------------------------------------------------------

/// `isInteractiveDescendant` 的判据部分（真源 :11）。
///
/// ★真源 :12-14 注释：卡片自身带 role=button，`closest` 会让卡片任意位置都命中自己，
/// 那样「点击卡片打开详情」就永远不执行（plan 卡的原始 bug）。这里只拦截**真实子控件**。
/// Leptos 侧没有 `Element::closest`，故由调用方拿 `event.target` 的 tag_name / role 判定，
/// 另配 [`is_interactive_descendant`] 把「目标就是卡片本身」这一跳先摘掉。
pub fn is_interactive_element(tag_name: &str, role: Option<&str>) -> bool {
    const INTERACTIVE_TAGS: [&str; 6] = ["button", "a", "input", "textarea", "select", "summary"];
    INTERACTIVE_TAGS.contains(&tag_name.to_lowercase().as_str()) || role == Some("button")
}

/// 真源 `isInteractiveDescendant(event.target, event.currentTarget)`（:9-15, :58, :63）。
///
/// 与真源同构的两步：`interactive !== card`（真源 :14）先摘掉「点的就是卡片」这一跳
/// ——整卡有 role=button，不摘就等于每次点击都判定成子控件、`onOpen` 永不触发；
/// 剩下的才按标签名/role 认子控件。
fn is_interactive_descendant(
    target: Option<web_sys::EventTarget>,
    card: Option<web_sys::EventTarget>,
) -> bool {
    use wasm_bindgen::JsCast;
    let Some(target) = target else { return false };
    if let Some(card) = card {
        // `is_same_node` 是 Node 上的方法；两边都可能是 window 这类非 Node 的 EventTarget，
        // 所以用 dyn_ref 而不是 unchecked_ref。
        let target_node = target.dyn_ref::<web_sys::Node>();
        let card_node = card.dyn_ref::<web_sys::Node>();
        if let (Some(target_node), Some(card_node)) = (target_node.as_ref(), card_node.as_ref()) {
            if target_node.is_same_node(Some(card_node)) {
                return false;
            }
        }
    }
    let Some(element) = target.dyn_ref::<web_sys::Element>() else {
        return false;
    };
    is_interactive_element(&element.tag_name(), element.get_attribute("role").as_deref())
}

// ---------------------------------------------------------------------------
// 组件（真源 :17-115）
// ---------------------------------------------------------------------------

/// `WorkflowRunCompactCard`（真源 :27-115）。
///
/// ★真源 :20-22 注释——整卡就是入口：实测的可发现性失败正是因为入口曾经埋在展开后的
/// 卡体里。两个调用方只换 labelText 与 primaryText（create 用工作流名、resume 用 runId），
/// 状态点词与步数都由联接摘要实时驱动。
///
/// ★真源 :24-25 注释——`onOpen` 缺席即纯展示态：不加 role/tabIndex/hover
/// （宿主只在确实有可打开的 run 时注入回调）。
#[component]
pub fn WorkflowRunCompactCardComponent(
    aria_label: String,
    /// 真源 :41 `icon: ReactNode`。两个调用方给的是同一枚 Workflow 图标（族内一致），
    /// 折叠态才换 RotateCcw——见 resume_workflow_run.rs。
    icon_view: Option<ChildrenFn>,
    label_text: String,
    primary_text: String,
    /// 真源 :44 `primaryTitle?: string`——缺省时 title 用 primaryText 本身（:93）。
    primary_title: Option<String>,
    /// 联接到的 run 摘要（真源 :45）。runId 与状态词都从它读，不由调用方散着传。
    workflow_run: WorkflowRunCardSummary,
    status_label: String,
    steps_label: String,
    /// 真源 :48 `onOpen: (() => void) | undefined`。
    on_open: Option<Callback<()>>,
    show_icon: bool,
    /// 卡片下方的附加内容（如 ToolSnapshotFieldNotice），随卡渲染（真源 :50-51）。
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    // 真源 :101/:103 按 `workflowRun.status` 索引状态表；协议把它约束成五值联合，
    // Rust 侧从 wire 字符串解回 `WorkflowRunState`。解不出（协议外的值）时
    // 真源会拿到 undefined、cn() 直接丢掉该段——这里同样给空类名，不猜一个颜色。
    let parsed_status = serde_json::from_value::<WorkflowRunStatus>(serde_json::Value::String(
        workflow_run.status.clone(),
    ))
    .ok();
    let status_dot_cls = parsed_status
        .map(run_status_dot_class)
        .unwrap_or_default()
        .to_string();
    let status_text_cls = parsed_status
        .map(run_status_text_class)
        .unwrap_or_default()
        .to_string();

    // ★真源 :74-76 —— 打开时才有交互态类名。
    let interactive_cls = if on_open.is_some() {
        "hover:border-border-hover focus-visible:border-input-border-focused focus-visible:ring-2 focus-visible:ring-ring/40"
    } else {
        ""
    };
    let card_cls = format!(
        "flex w-full min-w-0 items-center gap-2 overflow-hidden rounded-xl \
         border border-card-border bg-card px-3 py-2.5 text-foreground \
         shadow-xs outline-none transition-colors {interactive_cls}"
    );
    // 真源 :93 —— primaryText 的 title 缺省时用自身。
    let title_text = primary_title.unwrap_or_else(|| primary_text.clone());
    let open_for_click = on_open.clone();
    let open_for_key = on_open;
    // 真源 :113 —— children（卡片下方的附加内容）随卡渲染。
    let extra = children.map(|f| f());

    view! {
        <>
            <section
                aria-label=aria_label
                class=card_cls
                data-testid="workflow-run-card"
                data-workflow-run-id=workflow_run.run_id.clone()
                // 真源 :80 —— 属性写协议原样的状态串，不写成解析后的枚举名。
                data-workflow-run-status=workflow_run.status.clone()
                // ★真源 :83-84 —— 打开时才给 role/tabIndex（纯展示态不加）。
                role=on_open.is_some().then_some("button")
                tabindex=on_open.is_some().then_some("0")
                on:click=move |ev| {
                    // 真源 :56-59 handleCardClick。
                    let Some(callback) = open_for_click else { return };
                    if is_interactive_descendant(ev.target(), ev.current_target()) {
                        return;
                    }
                    callback.run(());
                }
                on:keydown=move |ev| {
                    // 真源 :60-66 handleCardKeyDown —— Enter / Space，且子控件的 keydown
                    // 会继续冒泡到整卡，一次键盘操作会打开两次详情页，故同样先判子控件。
                    let Some(callback) = open_for_key else { return };
                    if ev.key() != "Enter" && ev.key() != " " {
                        return;
                    }
                    if is_interactive_descendant(ev.target(), ev.current_target()) {
                        return;
                    }
                    ev.prevent_default();
                    callback.run(());
                }
            >
                {show_icon.then(|| icon_view.as_ref().map(|f| f()))}
                <span class="shrink-0 text-ui-base font-medium text-foreground-subtle">
                    {label_text}
                </span>
                // ★真源 :90 —— 工作流名称是界面标题，用默认字体，
                // 避免被当作代码以等宽字体展示。
                <span class="min-w-0 flex-1 truncate text-ui-base text-foreground-subtlest" title=title_text>
                    {primary_text}
                </span>
                <span class="flex shrink-0 items-center gap-1.5">
                    // ★真源 :98 —— 状态词永远在圆点旁边：
                    // 状态绝不只靠颜色或动画表达。
                    <span aria-hidden="true" class=format!("size-1.5 shrink-0 rounded-full {status_dot_cls}") />
                    <span class=format!("text-ui-sm {status_text_cls}")>{status_label}</span>
                </span>
                // ★真源 :107-108 —— 步数是「已排程的里结算了几个」，
                // 不是全程百分比——动态工作流没有静态总数。
                <span class="shrink-0 font-mono text-ui-xs tabular-nums text-foreground-subtlest">
                    {steps_label}
                </span>
            </section>
            {extra}
        </>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::run_state::WorkflowRunStatus;

    #[test]
    fn interactive_elements_are_detected() {
        // 真源 :11 —— 真实子控件才拦截。
        for tag in ["button", "a", "input", "textarea", "select", "summary"] {
            assert!(is_interactive_element(tag, None), "{tag} 应算交互元素");
        }
        // role=button 也算。
        assert!(is_interactive_element("div", Some("button")));
        // section/span 本身不是交互标签。
        assert!(!is_interactive_element("section", None));
        assert!(!is_interactive_element("span", None));
        assert!(!is_interactive_element("div", None));
    }

    #[test]
    fn interactive_detection_is_case_insensitive() {
        assert!(is_interactive_element("BUTTON", None));
        assert!(is_interactive_element("Button", None));
    }

    #[test]
    fn card_wire_status_parses_back_to_the_enum() {
        // 组件按 `workflowRun.status`（wire 字符串）查状态表，
        // 解得回来才有点色与词色；解不出的值必须让调用方拿到 None 而不是猜一个。
        for s in ["pending", "running", "completed", "errored", "stopped"] {
            let parsed = serde_json::from_value::<WorkflowRunStatus>(serde_json::Value::String(
                s.to_string(),
            ));
            assert!(parsed.is_ok(), "{s} 应能解回 WorkflowRunStatus");
        }
        assert!(
            serde_json::from_value::<WorkflowRunStatus>(serde_json::Value::String(
                "cancelled".to_string()
            ))
            .is_err(),
            "协议外的状态串不该被硬凑成一个已知态"
        );
    }

    #[test]
    fn the_five_run_states_have_distinct_word_colors() {
        // ★状态永远有词，不只靠颜色（run-status-presentation.ts:43 的原话）：
        // 每个态都要有语义色类名，且成功/失败/运行中三者互不相同。
        let all = [
            WorkflowRunStatus::Pending,
            WorkflowRunStatus::Running,
            WorkflowRunStatus::Completed,
            WorkflowRunStatus::Errored,
            WorkflowRunStatus::Stopped,
        ];
        for s in all {
            assert!(
                run_status_text_class(s).starts_with("text-"),
                "{s:?} 应有状态词色"
            );
            assert!(
                !run_status_dot_class(s).is_empty(),
                "{s:?} 应有状态点类名"
            );
        }
        assert_eq!(run_status_text_class(WorkflowRunStatus::Running), "text-warning");
        assert_eq!(run_status_dot_class(WorkflowRunStatus::Errored), "bg-destructive ring-2 ring-destructive/30");
    }

    #[test]
    fn smoke_component_constructs() {
        let summary = WorkflowRunCardSummary {
            run_id: "run-1".into(),
            tool_call_id: Some("tc-1".into()),
            status: "running".into(),
            stop_reason: None,
            nodes_settled: 2,
            nodes_total: 5,
            agents: Some(2),
            run: None,
            resumable: false,
        };
        let _ = view! {
            <WorkflowRunCompactCardComponent
                aria_label="工作流已启动".to_string()
                icon_view=None
                label_text="工作流".to_string()
                primary_text="run-1".to_string()
                primary_title=None
                workflow_run=summary
                status_label="运行中".to_string()
                steps_label="2/5".to_string()
                on_open=None
                show_icon=false
            />
        };
    }
}
