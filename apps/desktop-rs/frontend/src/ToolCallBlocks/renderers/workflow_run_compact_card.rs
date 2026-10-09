//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/workflow-run-compact-card.tsx`
//! （115 行）+ `components/workflow-graph/run-status-presentation.ts` 的状态词表。
//!
//! workflow run 的紧凑可点卡——CreateWorkflow 与 ResumeWorkflowRun 工具卡
//! 共享的「run 态第三态」。

use leptos::prelude::*;

// ---------------------------------------------------------------------------
// 状态词表（真源 run-status-presentation.ts :12-50）
// ---------------------------------------------------------------------------

/// `STATUS_DOT`（真源 :12-17）——步骤级状态点。
pub const STATUS_DOT_DONE: &str = "bg-success";
pub const STATUS_DOT_FAILED: &str = "bg-destructive ring-2 ring-destructive/30";
pub const STATUS_DOT_PENDING: &str = "border-[1.5px] border-foreground-subtlest bg-transparent";
pub const STATUS_DOT_RUNNING: &str = "animate-pulse bg-warning motion-reduce:animate-none";

/// run 级状态（真源 `WorkflowRunState["status"]`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunState {
    Pending,
    Running,
    Completed,
    Errored,
    Stopped,
}

impl WorkflowRunState {
    /// 从协议字符串解析（`statusLabel` 等按此取值）。
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "errored" => Some(Self::Errored),
            "stopped" => Some(Self::Stopped),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Errored => "errored",
            Self::Stopped => "stopped",
        }
    }
}

/// `RUN_STATUS_DOT`（真源 :35-41）：run 状态 → 状态点类名。
pub fn run_status_dot(state: WorkflowRunState) -> &'static str {
    match state {
        WorkflowRunState::Pending => STATUS_DOT_PENDING,
        WorkflowRunState::Running => STATUS_DOT_RUNNING,
        WorkflowRunState::Completed => STATUS_DOT_DONE,
        WorkflowRunState::Errored => STATUS_DOT_FAILED,
        WorkflowRunState::Stopped => STATUS_DOT_PENDING,
    }
}

/// `RUN_STATUS_TEXT`（真源 :43-50）：run 状态 → 状态词语义色。
///
/// ★真源注释：与状态点同一套判断，只是换成文字通道
/// ——**状态永远有词，不只靠颜色**。
pub fn run_status_text(state: WorkflowRunState) -> &'static str {
    match state {
        WorkflowRunState::Pending => "text-foreground-subtle",
        WorkflowRunState::Running => "text-warning",
        WorkflowRunState::Completed => "text-success",
        WorkflowRunState::Errored => "text-destructive",
        WorkflowRunState::Stopped => "text-foreground-subtle",
    }
}

/// 空环灯两态（真源 :19-22）。
///
/// ★真源注释：形状与「compiled」、待启动的站同为空环：什么都没跑。
/// 颜色只说注意力是否还悬着——`open` 是该谱系最新的一稿（循环还在、
/// 或模型停在这里），`settled` 是后面已有更新的一稿。
/// **绝不用 destructive**：这个特性里红色只属于出错的 run。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftFeedbackDot {
    Open,
    Settled,
}

impl DraftFeedbackDot {
    pub fn class(self) -> &'static str {
        match self {
            Self::Open => "bg-destructive",
            Self::Settled => "bg-foreground-subtlest",
        }
    }

    pub fn data_attr(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Settled => "settled",
        }
    }
}

// ---------------------------------------------------------------------------
// 交互判定（真源 :9-15）
// ---------------------------------------------------------------------------

/// `isInteractiveDescendant`（真源 :9-15）。
///
/// ★真源注释：卡片自身带 role=button，`closest` 会让卡片任意位置都命中自己，
/// 那样「点击卡片打开详情」就永远不执行（plan 卡的原始 bug）。
/// 这里只拦截**真实子控件**。
///
/// Leptos 侧没有 `Element::closest`，故由调用方用 `event.target` 的
/// tag_name / role 属性判定；本函数只承载判据。
pub fn is_interactive_element(tag_name: &str, role: Option<&str>) -> bool {
    const INTERACTIVE_TAGS: [&str; 6] = ["button", "a", "input", "textarea", "select", "summary"];
    INTERACTIVE_TAGS.contains(&tag_name.to_lowercase().as_str()) || role == Some("button")
}

// ---------------------------------------------------------------------------
// 组件（真源 :17-115）
// ---------------------------------------------------------------------------

/// `WorkflowRunCompactCard`（真源 :34-115）。
///
/// ★真源 :36-41 注释——整卡就是入口（DESIGN.md 的语义色 + 现有 run 状态词汇表）：
/// 实测的可发现性失败正是因为入口曾经埋在展开后的卡体里。
/// 两个调用方只换 labelText 与 primaryText（create 用工作流名、resume 用 runId），
/// 状态点词与步数都由联接摘要实时驱动。
///
/// ★真源 :22-25 注释——`onOpen` 缺席即纯展示态：不加 role/tabIndex/hover
/// （宿主只在确实有可打开的 run 时注入回调）。
#[component]
pub fn WorkflowRunCompactCardComponent(
    aria_label: String,
    label_text: String,
    primary_text: String,
    run_id: String,
    status: WorkflowRunState,
    status_label: String,
    steps_label: String,
    /// 宿主是否注入了打开回调（缺席 = 纯展示态）。
    #[prop(optional)] has_on_open: bool,
    show_icon: bool,
    #[prop(optional, into)] primary_title: String,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    // ★真源 :95-97 —— 打开时才有交互态类名。
    let interactive_cls = if has_on_open {
        "hover:border-border-hover focus-visible:border-input-border-focused focus-visible:ring-2 focus-visible:ring-ring/40"
    } else {
        ""
    };
    let card_cls = format!(
        "flex w-full min-w-0 items-center gap-2 overflow-hidden rounded-xl \
         border border-card-border bg-card px-3 py-2.5 text-foreground \
         shadow-xs outline-none transition-colors {interactive_cls}"
    );
    let status_dot_cls = run_status_dot(status);
    let status_text_cls = run_status_text(status);
    let role_attr = if has_on_open { "button" } else { "" };
    let title_text = if primary_title.is_empty() {
        primary_text.clone()
    } else {
        primary_title
    };

    // 真源 :103 —— primaryText 的 title 缺省时用自身。
    // 真源 :113 —— children（卡片下方的附加内容，如快照提示）随卡渲染。
    let extra = children.map(|f| f());

    view! {
        <>
            <section
                aria-label=aria_label
                class=card_cls
                data-testid="workflow-run-card"
                data-workflow-run-id=run_id
                data-workflow-run-status=status.as_str()
                // ★真源 :75-77 —— 打开时才给 role/tabindex（纯展示态不加）。
                role=role_attr
                tabindex=if has_on_open { "0" } else { "-1" }
            >
                {show_icon.then(|| {
                    view! {
                        <span class="shrink-0 size-4 flex items-center justify-center">
                            <crate::app::Icon
                                paths=vec!["m21 21-4.34-4.34"]
                                circles=vec![("11", "11", "8")]
                            />
                        </span>
                    }
                })}
                <span class="shrink-0 text-ui-base font-medium text-foreground-subtle">
                    {label_text}
                </span>
                // ★真源 :101 —— 工作流名称是界面标题，用默认字体，
                // 避免被当作代码以等宽字体展示。
                <span class="min-w-0 flex-1 truncate text-ui-base text-foreground-subtlest" title=title_text>
                    {primary_text}
                </span>
                <span class="flex shrink-0 items-center gap-1.5">
                    // ★真源 :105 —— 状态词永远在圆点旁边：
                    // 状态绝不只靠颜色或动画表达。
                    <span aria-hidden="true" class=format!("size-1.5 shrink-0 rounded-full {status_dot_cls}") />
                    <span class=format!("text-ui-sm {status_text_cls}")>{status_label}</span>
                </span>
                // ★真源 :111-113 —— 步数是「已排程的里结算了几个」，
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

    #[test]
    fn run_status_dot_maps_five_states() {
        // 真源 :35-41 —— completed 走 done、errored 走 failed、
        // stopped 复用 pending（与 pending 同色）。
        assert_eq!(run_status_dot(WorkflowRunState::Completed), STATUS_DOT_DONE);
        assert_eq!(run_status_dot(WorkflowRunState::Errored), STATUS_DOT_FAILED);
        assert_eq!(run_status_dot(WorkflowRunState::Pending), STATUS_DOT_PENDING);
        assert_eq!(
            run_status_dot(WorkflowRunState::Stopped),
            run_status_dot(WorkflowRunState::Pending),
            "stopped 与 pending 同点"
        );
        assert_eq!(run_status_dot(WorkflowRunState::Running), STATUS_DOT_RUNNING);
    }

    #[test]
    fn run_status_text_never_relies_on_color_alone() {
        // ★真源 :43 注释：状态永远有词，不只靠颜色。
        // 每个状态都要有自己的语义色类名。
        let all = [
            WorkflowRunState::Pending,
            WorkflowRunState::Running,
            WorkflowRunState::Completed,
            WorkflowRunState::Errored,
            WorkflowRunState::Stopped,
        ];
        for s in all {
            assert!(
                run_status_text(s).starts_with("text-"),
                "{s:?} 的状态词色应存在：{}",
                run_status_text(s)
            );
        }
        // 成功是success、失败是 destructive、运行中是 warning。
        assert_eq!(
            run_status_text(WorkflowRunState::Completed),
            "text-success"
        );
        assert_eq!(
            run_status_text(WorkflowRunState::Errored),
            "text-destructive"
        );
        assert_eq!(run_status_text(WorkflowRunState::Running), "text-warning");
    }

    #[test]
    fn pending_dot_is_hollow_ring() {
        // 真源 :15 —— 待启动是空环（1.5px 边框 + 透明底）。
        assert!(STATUS_DOT_PENDING.contains("border-[1.5px]"));
        assert!(STATUS_DOT_PENDING.contains("bg-transparent"));
    }

    #[test]
    fn running_dot_pulses_but_respects_reduced_motion() {
        // 真源 :16 —— animate-pulse + motion-reduce:animate-none。
        assert!(STATUS_DOT_RUNNING.contains("animate-pulse"));
        assert!(STATUS_DOT_RUNNING.contains("motion-reduce:animate-none"));
    }

    #[test]
    fn draft_feedback_never_uses_destructive_for_settled() {
        // ★真源 :22 —— 绝不用 destructive（这个特性里红色只属于出错的 run）。
        assert_eq!(DraftFeedbackDot::Settled.class(), "bg-foreground-subtlest");
        assert_ne!(DraftFeedbackDot::Settled.class(), DraftFeedbackDot::Open.class());
        assert_eq!(DraftFeedbackDot::Open.data_attr(), "open");
        assert_eq!(DraftFeedbackDot::Settled.data_attr(), "settled");
    }

    #[test]
    fn state_parse_and_as_str_round_trip() {
        for s in [
            WorkflowRunState::Pending,
            WorkflowRunState::Running,
            WorkflowRunState::Completed,
            WorkflowRunState::Errored,
            WorkflowRunState::Stopped,
        ] {
            assert_eq!(WorkflowRunState::parse(s.as_str()), Some(s));
        }
        assert_eq!(WorkflowRunState::parse("unknown"), None);
    }

    #[test]
    fn interactive_elements_are_detected() {
        // 真源 :13 —— 真实子控件才拦截。
        for tag in ["button", "a", "input", "textarea", "select", "summary"] {
            assert!(is_interactive_element(tag, None), "{tag} 应算交互元素");
        }
        // role=button 也算。
        assert!(is_interactive_element("div", Some("button")));
        // 卡片自身（section/span）不算——否则「点卡片打开详情」永不执行。
        assert!(!is_interactive_element("section", None));
        assert!(!is_interactive_element("span", None));
        assert!(!is_interactive_element("div", None));
    }

    #[test]
    fn interactive_detection_is_case_insensitive() {
        assert!(is_interactive_element("BUTTON", None));
        assert!(is_interactive_element("Button", None));
    }
}