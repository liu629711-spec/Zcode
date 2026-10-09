//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/get-workflow-run-situation.tsx`（227 行）。
//!
//! GetWorkflowRun 工具卡的**情势截面**：阶段轨 + 健康行（花名册在
//! `get_workflow_run_roster.rs`）。
//!
//! 两条纪律与模型面
//! （`apps/zcode-cli/packages/core/src/tool/handlers/get-workflow-run-format-roster.ts`）
//! 一字不差（真源 :5-11）：
//! 1. **缺席即不画**。没有时刻就没有年龄，没有读数就没有那一格；`0` 是一件事实，
//!    而「不知道」是另一件——绝不用 0 顶替后者。
//! 2. 所有年龄对**快照时刻** `generatedAt` 算，不对 `Date.now()` 算：一张三天前的卡
//!    重新打开时读数不能跟着今天漂。`generatedAt` 缺席就一个年龄都不画。
//!
//! 布局：一律换行行（flex-wrap），没有定宽表格——手机窄屏下要能折行而不是横向溢出。

use leptos::prelude::*;

use super::super::i18n;
use crate::ToolCallBlocks::toolResultDisplay::{
    WorkflowRunHealth, WorkflowRunPhaseState, WorkflowRunPhaseView,
};
use crate::app_shell::workflowRunThrottle::throttle_reason_label;
use crate::lib::workflowObservationFormat::{format_workflow_age, format_workflow_duration};

/// 真源 :23 的 `I18N_PREFIX`。
pub const I18N_PREFIX: &str = "chat.toolCall.workflow.getRun.";

/// 情势区块的容器（真源 :26-27）：与同一张卡里的日志面板同款低层容器，不是第二种卡面。
pub const SITUATION_BLOCK_CLASS: &str =
    "min-w-0 space-y-1 rounded-lg border border-border bg-surface px-2 py-1.5";

/// 情势区块里的一行（真源 :29-30）：窄屏折行，基线对齐
/// （数字与文字混排时才不会互相顶高）。
pub const SITUATION_ROW_CLASS: &str = "flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5";

/// `PHASE_STATE_TEXT`（真源 :36-41）：阶段状态词的语义色。
///
/// 与 run 整体状态同一套判断（run-status-presentation.ts）：在动的用活动色 warning，
/// 完成用 success，还没发生的最弱，终态没结算的居中。
pub fn phase_state_text_class(state: WorkflowRunPhaseState) -> &'static str {
    match state {
        WorkflowRunPhaseState::Done => "text-success",
        WorkflowRunPhaseState::Current => "text-warning",
        WorkflowRunPhaseState::Ahead => "text-foreground-subtlest",
        WorkflowRunPhaseState::Unfinished => "text-foreground-subtle",
    }
}

/// 真源 :61-87 —— 阶段行的计数格。
///
/// `ahead` 的行只有序号、名字和状态词——它还没发生过，没有轮次也没有步数可说
/// （三个数都是 0，于是下面三条 `> 0` 一条都不进）。
pub fn phase_cells(phase: &WorkflowRunPhaseView, terminal: bool) -> Vec<String> {
    let mut cells = Vec::new();
    // rounds 为 0 只可能是 `ahead`：没进过的阶段说不出「进过几次」（真源 :62 注释）。
    if phase.rounds > 0 {
        let id = if phase.rounds == 1 {
            "chat.toolCall.workflow.getRun.phase.roundsOne"
        } else {
            "chat.toolCall.workflow.getRun.phase.rounds"
        };
        cells.push(i18n::format(id, &[("count".to_string(), phase.rounds.to_string())]));
    }
    if phase.nodes_settled > 0 {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.phase.settled",
            &[("count".to_string(), phase.nodes_settled.to_string())],
        ));
    }
    if phase.nodes_running > 0 {
        // 真源 :80 —— 终态 run 里的「还在跑」是没结算，不是在动。
        let id = if terminal {
            "chat.toolCall.workflow.getRun.phase.unfinished"
        } else {
            "chat.toolCall.workflow.getRun.phase.running"
        };
        cells.push(i18n::format(id, &[("count".to_string(), phase.nodes_running.to_string())]));
    }
    cells
}

/// `phaseDuration`（真源 :113-128）。
///
/// 没有离开时刻时：活着的 run 说「到现在为止」；**终态 run 什么也不说**——它的离开时刻
/// 无人记录，拿快照时刻去减等于把进程死后的那几个小时算进这个阶段（真源 :121-122）。
pub fn phase_duration(
    phase: &WorkflowRunPhaseView,
    generated_at: Option<f64>,
    terminal: bool,
) -> Option<String> {
    let entered_at = phase.entered_at?;
    if let Some(exited_at) = phase.exited_at {
        return Some(format_workflow_duration(exited_at - entered_at));
    }
    if terminal {
        return None;
    }
    let so_far = format_workflow_age(generated_at, Some(entered_at))?;
    Some(i18n::format(
        "chat.toolCall.workflow.getRun.phase.soFar",
        &[("duration".to_string(), so_far)],
    ))
}

/// 真源 :150-203 —— 健康行的一排读数格。
///
/// `consecutiveFailures` / `cachedSteps` 只在大于 0 时出现：这两个读数为 0 时不是新闻，
/// 而卡面比模型面更吝惜行（真源 :137-138）。
pub fn health_cells(
    health: &WorkflowRunHealth,
    generated_at: Option<f64>,
    terminal: bool,
) -> Vec<String> {
    let mut cells = Vec::new();

    if let Some(last_progress) = format_workflow_age(generated_at, health.last_progress_at) {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.health.lastProgress",
            &[("age".to_string(), last_progress)],
        ));
    }

    if let Some(concurrency) = &health.concurrency {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.health.concurrency",
            &[
                ("effective".to_string(), concurrency.effective.to_string()),
                ("cap".to_string(), concurrency.cap.to_string()),
            ],
        ));
        // 原因与起始时刻各占一格，不塞进括号：括号的形状中英文不同，
        // 而这一行本来就是按格读的。reason 是开放字符串，认识的映射成短标签，
        // 不认识的原样显示（真源 :162-164）。
        if let Some(reason) = &concurrency.reason {
            cells.push(throttle_reason_label(reason));
        }
        if let Some(since_age) = format_workflow_age(generated_at, concurrency.since) {
            cells.push(i18n::format(
                "chat.toolCall.workflow.getRun.health.concurrencySince",
                &[("age".to_string(), since_age)],
            ));
        }
    }

    // `stalled` 只对活着的 run 有意义（终态 run 当然不动了，真源 :133）。
    if !terminal {
        let cell = match (health.stalled_since, format_workflow_age(generated_at, health.stalled_since)) {
            (None, _) => i18n::text("chat.toolCall.workflow.getRun.health.notStalled"),
            (Some(_), None) => i18n::text("chat.toolCall.workflow.getRun.health.stalledNoClock"),
            (Some(_), Some(age)) => i18n::format(
                "chat.toolCall.workflow.getRun.health.stalled",
                &[("age".to_string(), age)],
            ),
        };
        cells.push(cell);
    }

    if health.consecutive_failures > 0 {
        let id = if health.consecutive_failures == 1 {
            "chat.toolCall.workflow.getRun.health.failuresOne"
        } else {
            "chat.toolCall.workflow.getRun.health.failures"
        };
        cells.push(i18n::format(id, &[("count".to_string(), health.consecutive_failures.to_string())]));
    }
    if health.cached_steps > 0 {
        let id = if health.cached_steps == 1 {
            "chat.toolCall.workflow.getRun.health.cachedStepsOne"
        } else {
            "chat.toolCall.workflow.getRun.health.cachedSteps"
        };
        cells.push(i18n::format(id, &[("count".to_string(), health.cached_steps.to_string())]));
    }

    cells
}

/// 真源 :205 —— leftover 说明只在终态出现（且 `leftoverRunning` 在场）。
///
/// 这是本张卡上唯一一处「把不知道说出口」的地方：花名册里那些标着运行中的行是进程死在
/// 它们下面的残留，沉默会被读成「它们还在跑」（真源 :133-136）。
pub fn health_leftover_text(
    health: &WorkflowRunHealth,
    terminal: bool,
) -> Option<String> {
    let leftover = if terminal { health.leftover_running } else { None }?;
    let id = if leftover == 1 {
        "chat.toolCall.workflow.getRun.health.leftoverOne"
    } else {
        "chat.toolCall.workflow.getRun.health.leftover"
    };
    Some(i18n::format(
        id,
        &[("count".to_string(), leftover.to_string())],
    ))
}

/// 真源 :206 —— 一格读数都没有、也没有 leftover 时，整段不渲染。
pub fn health_line_renders(cells: &[String], leftover: &Option<String>) -> bool {
    !cells.is_empty() || leftover.is_some()
}

/// `WorkflowRunPhaseTrack`（真源 :47-109）：一行一个阶段，声明序。
#[component]
pub fn WorkflowRunPhaseTrack(
    phases: Vec<WorkflowRunPhaseView>,
    generated_at: Option<f64>,
    terminal: bool,
) -> impl IntoView {
    // 真源 :57 —— 零个阶段整段不画（缺席即不画）。
    if phases.is_empty() {
        return ().into_any();
    }

    // ★真源 :88 起的那些值全部在 view! 之前算好（children 先于属性求值）。
    let rows = phases
        .into_iter()
        .enumerate()
        .map(|(index, phase)| {
            let number = format!("{}.", index + 1);
            let name = phase.name.clone();
            let state = phase_state_text_class(phase.state);
            let state_word = i18n::text(&format!(
                "{I18N_PREFIX}phase.state.{}",
                phase.state.as_str()
            ));
            let cells = phase_cells(&phase, terminal);
            let duration = phase_duration(&phase, generated_at, terminal);
            // 真源 :90 的 React key（`{name}-{index}`）在 Leptos 侧由非键控渲染承担，
            // 阶段列表是快照内静态的，不随交互重排。
            let cell_views = cells
                .into_iter()
                .map(|cell| view! { <span class="text-foreground-subtle">{cell}</span> })
                .collect_view();
            view! {
                <div class=SITUATION_ROW_CLASS>
                    <span class="shrink-0 tabular-nums text-foreground-subtlest">{number}</span>
                    <span class="min-w-0 break-words text-foreground">{name}</span>
                    <span class=format!("shrink-0 {state}")>{state_word}</span>
                    {cell_views}
                    {duration.map(|text| view! {
                        <span class="text-foreground-subtlest">{text}</span>
                    })}
                </div>
            }
        })
        .collect_view();

    view! {
        <div class=SITUATION_BLOCK_CLASS data-testid="workflow-run-phases">{rows}</div>
    }
    .into_any()
}

/// `WorkflowRunHealthLine`（真源 :140-226）：run 整体还在不在动，一行读数。
#[component]
pub fn WorkflowRunHealthLine(
    health: WorkflowRunHealth,
    generated_at: Option<f64>,
    terminal: bool,
) -> impl IntoView {
    let cells = health_cells(&health, generated_at, terminal);
    let leftover = health_leftover_text(&health, terminal);
    if !health_line_renders(&cells, &leftover) {
        return ().into_any();
    }

    let cell_views = cells
        .into_iter()
        .map(|cell| view! { <span>{cell}</span> })
        .collect_view();
    let has_cells = !cell_views.is_empty();
    let row_class = format!("{SITUATION_ROW_CLASS} text-ui-sm text-foreground-subtlest");

    view! {
        <div class="min-w-0 space-y-1" data-testid="workflow-run-health">
            {has_cells.then(|| view! {
                <div class=row_class>{cell_views}</div>
            })}
            {leftover.map(|text| view! {
                // 唯一一处「把不知道说出口」的话用活动色，不用 destructive：
                // 红色在这条特性里只属于出错的 run。
                <p class="break-words text-ui-sm text-warning" data-testid="workflow-run-leftover">
                    {text}
                </p>
            })}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolResultDisplay::{
        WorkflowRunConcurrencyHealth, WorkflowRunLastTool, WorkflowRunSubagentView,
    };

    fn phase(name: &str, state: WorkflowRunPhaseState, rounds: i64, settled: i64, running: i64) -> WorkflowRunPhaseView {
        WorkflowRunPhaseView {
            name: name.into(),
            state,
            rounds,
            nodes_settled: settled,
            nodes_running: running,
            entered_at: None,
            exited_at: None,
        }
    }

    fn health() -> WorkflowRunHealth {
        WorkflowRunHealth {
            last_progress_at: None,
            stalled_since: None,
            concurrency: None,
            consecutive_failures: 0,
            cached_steps: 0,
            leftover_running: None,
            pending_questions_known: true,
        }
    }

    #[test]
    fn class_constants_match_source() {
        assert_eq!(
            SITUATION_BLOCK_CLASS,
            "min-w-0 space-y-1 rounded-lg border border-border bg-surface px-2 py-1.5"
        );
        assert_eq!(
            SITUATION_ROW_CLASS,
            "flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5"
        );
        assert_eq!(I18N_PREFIX, "chat.toolCall.workflow.getRun.");
    }

    #[test]
    fn phase_state_colors_follow_the_run_palette() {
        // 真源 :36-41 —— 在动的用 warning、完成用 success、还没发生的最弱。
        assert_eq!(phase_state_text_class(WorkflowRunPhaseState::Done), "text-success");
        assert_eq!(phase_state_text_class(WorkflowRunPhaseState::Current), "text-warning");
        assert_eq!(
            phase_state_text_class(WorkflowRunPhaseState::Ahead),
            "text-foreground-subtlest"
        );
        assert_eq!(
            phase_state_text_class(WorkflowRunPhaseState::Unfinished),
            "text-foreground-subtle"
        );
    }

    #[test]
    fn ahead_phase_says_nothing_but_its_state() {
        // 真源 :44-46 + :63-87 —— rounds/settled/running 全 0 时一格计数都不出。
        let ahead = phase("收尾", WorkflowRunPhaseState::Ahead, 0, 0, 0);
        assert_eq!(phase_cells(&ahead, false), Vec::<String>::new());
    }

    #[test]
    fn phase_cells_read_their_three_counters() {
        let p = phase("编译", WorkflowRunPhaseState::Current, 2, 3, 1);
        let cells = phase_cells(&p, false);
        assert_eq!(cells, vec!["2 轮".to_string(), "3 步已结算".to_string(), "1 步运行中".to_string()]);
        // rounds 为 1 走单数 key（这里两条文案同字，仍是两条 key）。
        let one = phase("编译", WorkflowRunPhaseState::Current, 1, 0, 0);
        assert_eq!(phase_cells(&one, false), vec!["1 轮".to_string()]);
    }

    #[test]
    fn terminal_phase_running_cell_relabels_as_unfinished() {
        // ★真源 :80 —— 终态 run 里的「还在跑」是没结算，不是在动。
        let p = phase("编译", WorkflowRunPhaseState::Unfinished, 0, 0, 4);
        assert_eq!(phase_cells(&p, false), vec!["4 步运行中".to_string()]);
        assert_eq!(phase_cells(&p, true), vec!["4 步未完结".to_string()]);
    }

    #[test]
    fn phase_duration_never_borrows_the_clock_from_a_dead_run() {
        // 真源 :119-127。
        let mut p = phase("编译", WorkflowRunPhaseState::Done, 1, 1, 0);
        // 没有 enteredAt → 没有时长格（缺席即不画）。
        assert_eq!(phase_duration(&p, Some(1_000.0), false), None);
        // 两端都有 → 直接相减。
        p.entered_at = Some(0.0);
        p.exited_at = Some(310_000.0);
        assert_eq!(phase_duration(&p, Some(9_999_999.0), false).as_deref(), Some("5m 10s"));
        // 只有进入时刻 + 活着的 run → 「已进行 X」，年龄对 generatedAt 算。
        p.exited_at = None;
        assert_eq!(
            phase_duration(&p, Some(40_000.0), false).as_deref(),
            Some("已进行 40s")
        );
        // ★只有进入时刻 + 终态 run → 什么也不说（拿快照去减会把进程死后的几小时算进来）。
        assert_eq!(phase_duration(&p, Some(40_000.0), true), None);
        // 活着的 run 但没有快照时刻 → 同样不画（绝不用现在的时间顶替）。
        assert_eq!(phase_duration(&p, None, false), None);
    }

    #[test]
    fn ages_are_measured_against_the_snapshot_not_now() {
        // 纪律 2（真源 :9-11）：没有 generatedAt 就没有任何**年龄**格。
        let mut h = health();
        h.last_progress_at = Some(0.0);
        // ★「未停滞」不是年龄格——它是「stalledSince 缺席」这一件事实，
        // 没有快照时刻也照样要说（真源 :176-177 的 notStalled 分支不看 generatedAt）。
        let cells = health_cells(&h, None, false);
        assert_eq!(cells, vec!["未停滞".to_string()], "最近进展格因缺快照而缺席");
        let cells = health_cells(&h, Some(60_000.0), false);
        assert_eq!(cells[0], "最近进展 1m 00s 前");
        // 终态 run 连「未停滞」都不说。
        assert_eq!(health_cells(&h, Some(60_000.0), true).len(), 1);
    }

    #[test]
    fn stalled_has_three_readings_and_only_for_live_runs() {
        // 真源 :173-182 —— 未停滞 / 已停滞但没钟点 / 已停滞多久。
        let h = health();
        let cells = health_cells(&h, Some(1_000.0), false);
        assert_eq!(cells, vec!["未停滞".to_string()]);

        let mut stalled_no_clock = health();
        stalled_no_clock.stalled_since = Some(500.0);
        // generatedAt 缺席 → 有停滞事实、没有停滞时长。
        let cells = health_cells(&stalled_no_clock, None, false);
        assert_eq!(cells, vec!["已停滞".to_string()]);

        let cells = health_cells(&stalled_no_clock, Some(60_500.0), false);
        // 时长走同一套 formatWorkflowDuration：整分钟写作「1m 00s」，不是「1m」。
        assert_eq!(cells, vec!["已停滞，1m 00s 前起".to_string()]);

        // 终态 run 不画停滞格。
        let cells = health_cells(&stalled_no_clock, Some(60_500.0), true);
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn zero_readings_are_not_news() {
        // 真源 :137-138 —— consecutiveFailures / cachedSteps 为 0 时不出现。
        let mut h = health();
        h.consecutive_failures = 0;
        h.cached_steps = 0;
        let cells = health_cells(&h, None, true);
        assert_eq!(cells, Vec::<String>::new());
        h.consecutive_failures = 1;
        h.cached_steps = 2;
        let cells = health_cells(&h, None, true);
        assert_eq!(cells, vec!["连续失败 1 次".to_string(), "2 步命中缓存".to_string()]);
    }

    #[test]
    fn concurrency_row_is_split_into_cells_not_parentheses() {
        // 真源 :159-171 —— 读数 / 原因 / 起始时刻各占一格。
        let mut h = health();
        h.concurrency = Some(WorkflowRunConcurrencyHealth {
            effective: 2,
            cap: 4,
            reason: Some("rate_limited".into()),
            since: Some(0.0),
        });
        let cells = health_cells(&h, Some(40_000.0), false);
        assert_eq!(
            cells,
            vec![
                "并发 2/4".to_string(),
                "限流".to_string(),
                "40s 前起".to_string(),
                "未停滞".to_string(),
            ]
        );
        // 陌生的原因值原样显示——不认识不等于不显示。
        h.concurrency.as_mut().unwrap().reason = Some("brand_new".into());
        let cells = health_cells(&h, Some(40_000.0), true);
        assert_eq!(cells[1], "brand_new");
    }

    #[test]
    fn leftover_line_only_for_terminal_runs() {
        // 真源 :205 + :133-136。
        let mut h = health();
        h.leftover_running = Some(3);
        assert_eq!(health_leftover_text(&h, false), None, "活着的 run 没有残留可言");
        assert_eq!(
            health_leftover_text(&h, true).as_deref(),
            Some("下面标着运行中的 3 步是已退出进程的残留，不是还在进行的工作。")
        );
        h.leftover_running = Some(1);
        assert_eq!(
            health_leftover_text(&h, true).as_deref(),
            Some("下面标着运行中的那一步是已退出进程的残留，不是还在进行的工作。"),
            "1 走单数 key"
        );
        h.leftover_running = None;
        assert_eq!(health_leftover_text(&h, true), None);
    }

    #[test]
    fn empty_health_block_is_not_rendered() {
        // 真源 :206 —— cells 与 leftover 都空时整段 null。
        let h = health();
        let cells = health_cells(&h, None, true);
        let leftover = health_leftover_text(&h, true);
        assert!(!health_line_renders(&cells, &leftover));
        // 只要还有一格读数就渲染。
        let with_one_cell = vec!["未停滞".to_string()];
        assert!(health_line_renders(&with_one_cell, &None));
        // 只有 leftover 说明也渲染。
        let mut with_leftover = health();
        with_leftover.leftover_running = Some(2);
        let leftover_text = health_leftover_text(&with_leftover, true);
        assert!(health_line_renders(&health_cells(&with_leftover, None, true), &leftover_text));
    }

    #[test]
    fn subagent_view_types_are_reachable_from_the_display() {
        // 花名册那件（get_workflow_run_roster.rs）复用同一批 struct，
        // 这里只做一次类型连通性检查，避免两张卡各自定义一份。
        let view = WorkflowRunSubagentView {
            site_id: "a1".into(),
            ordinal: 0,
            name: None,
            state: crate::ToolCallBlocks::toolResultDisplay::WorkflowRunSubagentState::Executing,
            phase_name: None,
            instructions_head: None,
            started_at: None,
            turn: None,
            tool_calls: None,
            last_tool: Some(WorkflowRunLastTool { name: "Bash".into(), target: None, at: None }),
            wait_cause: None,
            retry_after_ms: None,
            wait_since: None,
            parked_on: None,
            steps_settled: 0,
            steps_failed: 0,
            tokens: 0,
            last_progress_at: None,
        };
        assert_eq!(view.last_tool.as_ref().unwrap().name, "Bash");
    }
}
