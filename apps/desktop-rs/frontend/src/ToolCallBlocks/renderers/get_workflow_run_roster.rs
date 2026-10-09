//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/get-workflow-run-roster.tsx`（226 行）。
//!
//! GetWorkflowRun 工具卡的**子代理花名册**（情势截面的另一半，阶段轨与健康行在
//! `get_workflow_run_situation.rs`）。
//!
//! 一行的读法与模型面
//! （`apps/zcode-cli/packages/core/src/tool/handlers/get-workflow-run-format-roster.ts`）
//! 同一句话（真源 :5-10）：「谁 · 在哪 · 什么相位 · 在哪个阶段 · 正在做什么 · 花了多少
//! token」，其中「正在做什么」按相位分叉——在跑的说它的进度与最后一个工具，在等的说等什么、
//! 还要等多久，停驻的说等哪个问题。这正是这张卡存在的理由：**一眼看出谁卡住了**。
//!
//! 载荷是**扁平行**（没有 `currentAsk` 嵌套），所以「有没有一次在飞的 ask」由那几个读数
//! 是否在场推出来（真源 :11-13）；缺席一律不画，绝不用 0 顶替不知道。

use leptos::prelude::*;

use super::get_workflow_run_situation::{SITUATION_BLOCK_CLASS, SITUATION_ROW_CLASS};
use super::super::i18n;
use crate::ToolCallBlocks::toolResultDisplay::{
    WorkflowRunSubagentState, WorkflowRunSubagentView, WorkflowRunWaitCause,
};
use crate::lib::workflowObservationFormat::{
    format_workflow_age, format_workflow_duration, format_workflow_token_count,
};

/// 真源 :30 的 `I18N_PREFIX`。
pub const I18N_PREFIX: &str = "chat.toolCall.workflow.getRun.";

/// `SUBAGENT_STATE_TEXT`（真源 :37-45）：相位词的语义色。
///
/// 词永远在场，颜色只是第二通道（DESIGN：状态不能只靠颜色）：在动的用活动色 warning，
/// 等待与终态未完结居中，完成 success、失败 destructive。
/// ★`parked` 也用活动色——它在等一个人回答，是需要注意的状态，不是安静的空闲。
pub fn subagent_state_text_class(state: WorkflowRunSubagentState) -> &'static str {
    match state {
        WorkflowRunSubagentState::Idle => "text-foreground-subtlest",
        WorkflowRunSubagentState::Executing => "text-warning",
        WorkflowRunSubagentState::Waiting => "text-foreground-subtle",
        WorkflowRunSubagentState::Parked => "text-warning",
        WorkflowRunSubagentState::Done => "text-success",
        WorkflowRunSubagentState::Failed => "text-destructive",
        WorkflowRunSubagentState::Unfinished => "text-foreground-subtle",
    }
}

/// `hasCurrentAsk`（真源 :113-122）：那几个进度读数任意一个在场，
/// 就说明这一行背后有一次 ask（扁平载荷没有 `currentAsk` 标记）。
pub fn has_current_ask(subagent: &WorkflowRunSubagentView) -> bool {
    subagent.started_at.is_some()
        || subagent.turn.is_some()
        || subagent.tool_calls.is_some()
        || subagent.last_tool.is_some()
        || subagent.instructions_head.is_some()
}

/// `executingCells`（真源 :145-182）。
pub fn executing_cells(
    subagent: &WorkflowRunSubagentView,
    generated_at: Option<f64>,
) -> Vec<String> {
    let mut cells = Vec::new();
    if let Some(on_step) = format_workflow_age(generated_at, subagent.started_at) {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.subagent.onStep",
            &[("age".to_string(), on_step)],
        ));
    }
    if let Some(turn) = subagent.turn {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.subagent.turn",
            &[("count".to_string(), turn.to_string())],
        ));
    }
    if let Some(tool_calls) = subagent.tool_calls {
        let id = if tool_calls == 1 {
            "chat.toolCall.workflow.getRun.subagent.toolCallsOne"
        } else {
            "chat.toolCall.workflow.getRun.subagent.toolCalls"
        };
        cells.push(i18n::format(id, &[("count".to_string(), tool_calls.to_string())]));
    }
    if let Some(last_tool) = &subagent.last_tool {
        let age = format_workflow_age(generated_at, last_tool.at);
        // 三段拼一格：工具名 / 目标 / 「多久前」，空段丢掉（真源 :172-179）。
        let parts: Vec<String> = [
            Some(i18n::format(
                "chat.toolCall.workflow.getRun.subagent.lastTool",
                &[("name".to_string(), last_tool.name.clone())],
            )),
            last_tool.target.clone().filter(|t| !t.is_empty()),
            age.map(|age| {
                i18n::format("chat.toolCall.workflow.getRun.age", &[("age".to_string(), age)])
            }),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect();
        cells.push(parts.join(" "));
    }
    cells
}

/// `waitCells`（真源 :184-209）。
///
/// ★真源 :195 —— 「等了多久」贴着原因，「还要等多久」收尾：
/// 两个时长挨在一起时读者分不清哪个是哪个。
pub fn wait_cells(
    subagent: &WorkflowRunSubagentView,
    generated_at: Option<f64>,
) -> Vec<String> {
    let cause = match subagent.wait_cause {
        None => return Vec::new(),
        Some(cause) => cause,
    };
    let id = if cause == WorkflowRunWaitCause::Slot {
        "chat.toolCall.workflow.getRun.subagent.waitingSlot"
    } else {
        "chat.toolCall.workflow.getRun.subagent.waitingBackoff"
    };
    let mut cells = vec![i18n::text(id)];
    if let Some(waited) = format_workflow_age(generated_at, subagent.wait_since) {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.subagent.waitedFor",
            &[("age".to_string(), waited)],
        ));
    }
    if let Some(retry_after_ms) = subagent.retry_after_ms {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.subagent.retryIn",
            &[("duration".to_string(), format_workflow_duration(retry_after_ms))],
        ));
    }
    cells
}

/// `settledCells`（真源 :211-225）。
pub fn settled_cells(subagent: &WorkflowRunSubagentView) -> Vec<String> {
    if subagent.steps_settled == 0 && subagent.steps_failed == 0 {
        return Vec::new();
    }
    let id = if subagent.steps_settled == 1 {
        "chat.toolCall.workflow.getRun.subagent.stepsOne"
    } else {
        "chat.toolCall.workflow.getRun.subagent.steps"
    };
    let mut cells = vec![i18n::format(id, &[("count".to_string(), subagent.steps_settled.to_string())])];
    if subagent.steps_failed > 0 {
        cells.push(i18n::format(
            "chat.toolCall.workflow.getRun.subagent.stepsFailed",
            &[("count".to_string(), subagent.steps_failed.to_string())],
        ));
    }
    cells
}

/// `subagentActivity`（真源 :124-143）——「正在做什么」按相位分叉。
pub fn subagent_activity(
    subagent: &WorkflowRunSubagentView,
    generated_at: Option<f64>,
) -> Vec<String> {
    // 只有 qid，没有提问时刻：卡面载荷不带 pendingQuestions，
    // 所以这一行说不出「等了多久」（真源 :130）。
    if subagent.state == WorkflowRunSubagentState::Parked {
        if let Some(qid) = &subagent.parked_on {
            return vec![i18n::format(
                "chat.toolCall.workflow.getRun.subagent.parkedOn",
                &[("qid".to_string(), qid.clone())],
            )];
        }
    }
    if subagent.state == WorkflowRunSubagentState::Waiting {
        return wait_cells(subagent, generated_at);
    }
    if subagent.state == WorkflowRunSubagentState::Unfinished && has_current_ask(subagent) {
        return vec![i18n::text("chat.toolCall.workflow.getRun.subagent.inFlightAtStop")];
    }
    if has_current_ask(subagent) {
        let cells = executing_cells(subagent, generated_at);
        // 一次 ask 在飞但一个读数也没有（老 journal）：退回已结算步数，
        // 别留下一行只有相位词（真源 :139-140）。
        if !cells.is_empty() {
            return cells;
        }
        return settled_cells(subagent);
    }
    settled_cells(subagent)
}

/// 一行花名册的全部渲染数据（真源 :59-105 里散在 JSX 中的每个判定，先算好后消费）。
#[derive(Debug, Clone, PartialEq)]
pub struct RosterRowView {
    pub name: Option<String>,
    pub address: String,
    pub state_class: &'static str,
    pub state_word: String,
    pub phase_cell: Option<String>,
    pub activity: Vec<String>,
    pub tokens_cell: Option<String>,
    /// 任务行：从属于上一行，不是新的一行事实（真源 :98）。
    pub task_cell: Option<String>,
}

/// 组装一行的读法（真源 :59-105）。
pub fn roster_row_view(
    subagent: &WorkflowRunSubagentView,
    generated_at: Option<f64>,
) -> RosterRowView {
    let activity = subagent_activity(subagent, generated_at);
    RosterRowView {
        // 匿名子代理不合成兜底名：留空，地址仍然把它认出来（真源 :64）。
        name: subagent.name.clone().filter(|n| !n.is_empty()),
        address: format!("{}@{}", subagent.site_id, subagent.ordinal),
        state_class: subagent_state_text_class(subagent.state),
        state_word: i18n::text(&format!(
            "{I18N_PREFIX}subagent.state.{}",
            subagent_state_token(subagent.state)
        )),
        phase_cell: subagent.phase_name.as_ref().map(|name| i18n::format(
            "chat.toolCall.workflow.getRun.subagent.phase",
            &[("name".to_string(), name.clone())],
        )),
        activity,
        tokens_cell: (subagent.tokens > 0).then(|| {
            i18n::format(
                "chat.toolCall.workflow.run.usage.tokens",
                &[("tokens".to_string(), format_workflow_token_count(subagent.tokens as f64))],
            )
        }),
        task_cell: subagent
            .instructions_head
            .clone()
            .filter(|head| !head.is_empty())
            .map(|task| {
                i18n::format(
                    "chat.toolCall.workflow.getRun.subagent.task",
                    &[("task".to_string(), task)],
                )
            }),
    }
}

/// `subagent.state` 的 i18n 词表后缀（真源 :72 直接内插枚举字面量）。
fn subagent_state_token(state: WorkflowRunSubagentState) -> &'static str {
    match state {
        WorkflowRunSubagentState::Idle => "idle",
        WorkflowRunSubagentState::Executing => "executing",
        WorkflowRunSubagentState::Waiting => "waiting",
        WorkflowRunSubagentState::Parked => "parked",
        WorkflowRunSubagentState::Done => "done",
        WorkflowRunSubagentState::Failed => "failed",
        WorkflowRunSubagentState::Unfinished => "unfinished",
    }
}

/// `WorkflowRunSubagentRoster`（真源 :47-111）。
#[component]
pub fn WorkflowRunSubagentRoster(
    subagents: Vec<WorkflowRunSubagentView>,
    generated_at: Option<f64>,
) -> impl IntoView {
    // 空花名册什么也不画：还没造出子代理是一件不需要一整块区域来说的事（真源 :55-56）。
    if subagents.is_empty() {
        return ().into_any();
    }

    let rows = subagents
        .into_iter()
        .map(|subagent| {
            let row = roster_row_view(&subagent, generated_at);
            let activity_cells = row
                .activity
                .into_iter()
                .map(|cell| view! {
                    <span class="min-w-0 break-words text-foreground-subtle">{cell}</span>
                })
                .collect_view();
            let row_class = format!("{SITUATION_ROW_CLASS} text-ui-sm");

            view! {
                <div class="min-w-0 space-y-0.5">
                    <div class=row_class>
                        {row.name.map(|name| view! {
                            <span class="min-w-0 break-words text-foreground">{name}</span>
                        })}
                        <span class="break-all font-mono text-ui-xs text-foreground-subtlest">
                            {row.address}
                        </span>
                        <span class=format!("shrink-0 {}", row.state_class)>{row.state_word}</span>
                        {row.phase_cell.map(|cell| view! {
                            <span class="min-w-0 break-words text-foreground-subtle">{cell}</span>
                        })}
                        {activity_cells}
                        {row.tokens_cell.map(|cell| view! {
                            <span class="shrink-0 tabular-nums text-foreground-subtlest">{cell}</span>
                        })}
                    </div>
                    {row.task_cell.map(|cell| view! {
                        // 任务行从属于上一行，不是新的一行事实：缩进而不是另起一格。
                        <p class="min-w-0 break-words pl-3 text-ui-sm text-foreground-subtle">{cell}</p>
                    })}
                </div>
            }
        })
        .collect_view();

    view! {
        <div class=SITUATION_BLOCK_CLASS data-testid="workflow-run-subagents">{rows}</div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolResultDisplay::WorkflowRunLastTool;

    fn subagent(state: WorkflowRunSubagentState) -> WorkflowRunSubagentView {
        WorkflowRunSubagentView {
            site_id: "a1".into(),
            ordinal: 2,
            name: None,
            state,
            phase_name: None,
            instructions_head: None,
            started_at: None,
            turn: None,
            tool_calls: None,
            last_tool: None,
            wait_cause: None,
            retry_after_ms: None,
            wait_since: None,
            parked_on: None,
            steps_settled: 0,
            steps_failed: 0,
            tokens: 0,
            last_progress_at: None,
        }
    }

    #[test]
    fn every_state_word_resolves() {
        // ★真源 :72 把枚举值直接内插进 message id——漏一个词，卡上就显示原始 id。
        for state in [
            WorkflowRunSubagentState::Idle,
            WorkflowRunSubagentState::Executing,
            WorkflowRunSubagentState::Waiting,
            WorkflowRunSubagentState::Parked,
            WorkflowRunSubagentState::Done,
            WorkflowRunSubagentState::Failed,
            WorkflowRunSubagentState::Unfinished,
        ] {
            let row = roster_row_view(&subagent(state), None);
            // 未收录的 id 会原样回显 id 本身（i18n::text 的缺省行为），所以查「还带着
            // subagent.state. 前缀」就是在查漏词条。
            assert!(
                !row.state_word.contains("subagent.state."),
                "{state:?} 的状态词应在文案表里，实际拿到 {:?}",
                row.state_word
            );
        }
    }

    #[test]
    fn parked_and_executing_share_the_activity_color() {
        // 真源 :33-36 —— parked 也用活动色：它在等一个人回答，是需要注意的状态。
        assert_eq!(
            subagent_state_text_class(WorkflowRunSubagentState::Parked),
            subagent_state_text_class(WorkflowRunSubagentState::Executing)
        );
        assert_eq!(subagent_state_text_class(WorkflowRunSubagentState::Done), "text-success");
        assert_eq!(
            subagent_state_text_class(WorkflowRunSubagentState::Failed),
            "text-destructive"
        );
        assert_eq!(
            subagent_state_text_class(WorkflowRunSubagentState::Idle),
            "text-foreground-subtlest"
        );
    }

    #[test]
    fn any_progress_reading_implies_an_in_flight_ask() {
        // 真源 :113-122 —— 五个读数任一在场即算。
        assert!(!has_current_ask(&subagent(WorkflowRunSubagentState::Executing)));
        for fill in [
            |s: &mut WorkflowRunSubagentView| s.started_at = Some(0.0),
            |s: &mut WorkflowRunSubagentView| s.turn = Some(1),
            |s: &mut WorkflowRunSubagentView| s.tool_calls = Some(1),
            |s: &mut WorkflowRunSubagentView| s.last_tool = Some(WorkflowRunLastTool { name: "Bash".into(), target: None, at: None }),
            |s: &mut WorkflowRunSubagentView| s.instructions_head = Some("看下 diff".into()),
        ] {
            let mut s = subagent(WorkflowRunSubagentState::Executing);
            fill(&mut s);
            assert!(has_current_ask(&s));
        }
        // stepsSettled 不在名单里：它是终态读数，不代表有 ask 在飞。
        let mut settled = subagent(WorkflowRunSubagentState::Done);
        settled.steps_settled = 5;
        assert!(!has_current_ask(&settled));
    }

    #[test]
    fn parked_row_says_which_question_without_an_age() {
        // 真源 :129-132。
        let mut s = subagent(WorkflowRunSubagentState::Parked);
        s.parked_on = Some("q-9".into());
        s.wait_since = Some(0.0);
        // 有 waitSince 也不画年龄——载荷不带 pendingQuestions。
        assert_eq!(
            subagent_activity(&s, Some(60_000.0)),
            vec!["等待问题 q-9 的答复".to_string()]
        );
        // 没有 qid 时继续往下走（不整行消失）。
        let no_qid = subagent(WorkflowRunSubagentState::Parked);
        assert_eq!(subagent_activity(&no_qid, None), Vec::<String>::new());
    }

    #[test]
    fn waiting_row_keeps_the_two_durations_apart() {
        // ★真源 :195 —— 原因 → 已等多久 → 还要等多久，两个时长不挨在一起。
        let mut s = subagent(WorkflowRunSubagentState::Waiting);
        s.wait_cause = Some(WorkflowRunWaitCause::Slot);
        s.wait_since = Some(0.0);
        s.retry_after_ms = Some(40_000.0);
        assert_eq!(
            subagent_activity(&s, Some(20_000.0)),
            vec!["等待槽位".to_string(), "已等 20s".to_string(), "40s 后重试".to_string()]
        );
        s.wait_cause = Some(WorkflowRunWaitCause::Backoff);
        let cells = subagent_activity(&s, None);
        // 「已等多久」要快照才算得出；「还要等多久」是载荷自带的读数，没快照也照说。
        assert_eq!(cells, vec!["退避中".to_string(), "40s 后重试".to_string()]);
        // 没有 waitCause 的 waiting 行不给活动格（真源 :189）。
        let causeless = subagent(WorkflowRunSubagentState::Waiting);
        assert_eq!(subagent_activity(&causeless, Some(1.0)), Vec::<String>::new());
    }

    #[test]
    fn unfinished_with_progress_says_it_was_in_flight_at_stop() {
        // 真源 :134-136。
        let mut s = subagent(WorkflowRunSubagentState::Unfinished);
        s.turn = Some(3);
        assert_eq!(
            subagent_activity(&s, None),
            vec!["停止时仍在执行".to_string()]
        );
        // 没有任何进度读数的 unfinished 行回到步数（真源 :142）。
        let bare = subagent(WorkflowRunSubagentState::Unfinished);
        assert_eq!(subagent_activity(&bare, None), Vec::<String>::new());
        let mut with_steps = bare.clone();
        with_steps.steps_settled = 2;
        assert_eq!(subagent_activity(&with_steps, None), vec!["2 步".to_string()]);
    }

    #[test]
    fn in_flight_ask_without_readings_falls_back_to_settled_steps() {
        // ★真源 :137-140 —— 老 journal：有 ask 在飞但一个读数也没有，
        // 别留下一行只有相位词。instructionsHead 算「有 ask」但不算读数。
        let mut s = subagent(WorkflowRunSubagentState::Executing);
        s.instructions_head = Some("把测试跑绿".into());
        s.steps_settled = 4;
        s.steps_failed = 1;
        assert_eq!(
            subagent_activity(&s, None),
            vec!["4 步".to_string(), "1 步失败".to_string()]
        );
    }

    #[test]
    fn executing_row_reads_progress_then_last_tool() {
        let mut s = subagent(WorkflowRunSubagentState::Executing);
        s.started_at = Some(0.0);
        s.turn = Some(2);
        s.tool_calls = Some(1);
        s.last_tool = Some(WorkflowRunLastTool {
            name: "Bash".into(),
            target: Some("cargo test".into()),
            at: Some(10_000.0),
        });
        assert_eq!(
            executing_cells(&s, Some(50_000.0)),
            vec![
                "这一步已进行 50s".to_string(),
                "第 2 轮".to_string(),
                "1 次工具调用".to_string(),
                "最后 Bash cargo test 40s 前".to_string(),
            ]
        );
        // 无 target 时那一格只有工具名（真源 :177 的空段过滤）。
        s.last_tool.as_mut().unwrap().target = None;
        let cells = executing_cells(&s, None);
        assert_eq!(cells.last().unwrap(), "最后 Bash");
    }

    #[test]
    fn zero_step_row_has_no_cells() {
        // 真源 :212 —— 两个步数都是 0 时不给格（缺席即不画）。
        assert_eq!(settled_cells(&subagent(WorkflowRunSubagentState::Done)), Vec::<String>::new());
        let mut one = subagent(WorkflowRunSubagentState::Done);
        one.steps_settled = 1;
        assert_eq!(settled_cells(&one), vec!["1 步".to_string()]);
    }

    #[test]
    fn anonymous_row_has_no_invented_name() {
        // 真源 :64-67 —— 匿名子代理留空，地址仍然把它认出来。
        let row = roster_row_view(&subagent(WorkflowRunSubagentState::Idle), None);
        assert_eq!(row.name, None);
        assert_eq!(row.address, "a1@2");
        let mut named = subagent(WorkflowRunSubagentState::Idle);
        named.name = Some("审查者".into());
        assert_eq!(roster_row_view(&named, None).name.as_deref(), Some("审查者"));
    }

    #[test]
    fn tokens_cell_only_when_nonzero() {
        // 真源 :87-94。
        assert!(roster_row_view(&subagent(WorkflowRunSubagentState::Done), None).tokens_cell.is_none());
        let mut s = subagent(WorkflowRunSubagentState::Done);
        s.tokens = 1_250;
        // 与列表行同一套 token 格式化（平局远离 0 → 1.3k）。
        assert_eq!(
            roster_row_view(&s, None).tokens_cell.as_deref(),
            Some("1.3k tokens")
        );
    }

    #[test]
    fn phase_and_task_cells_are_optional() {
        let mut s = subagent(WorkflowRunSubagentState::Executing);
        s.phase_name = Some("编译".into());
        s.instructions_head = Some("把测试跑绿".into());
        let row = roster_row_view(&s, None);
        assert_eq!(row.phase_cell.as_deref(), Some("阶段 编译"));
        assert_eq!(row.task_cell.as_deref(), Some("任务：把测试跑绿"));
        // 空串不算在场（真源 :96-97 的 `.length === 0` 判定）。
        s.instructions_head = Some(String::new());
        assert_eq!(roster_row_view(&s, None).task_cell, None);
    }

    #[test]
    fn prefix_matches_source() {
        assert_eq!(I18N_PREFIX, "chat.toolCall.workflow.getRun.");
    }
}
