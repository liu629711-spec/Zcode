//! 1:1 翻译 `packages/ui/src/components/workflow-run-line/TaskWorkflowRunLines.tsx`（276 行）。
//!
//! 侧栏任务行标题下的工作流运行行：Workflow 图标 + 迷你轨道灯 + 当前 phase 名。
//! 一眼看懂「这个会话在跑工作流、跑到哪一站」，别的都不画：没有光晕、没有行进虚线、
//! 没有问题 chip（升级问答由主代理作答，不是用户）、没有子代理数、没有箭头——
//! 那些进 hover tooltip。结束的 run 只剩一个中性词，颜色只留给灯。
//!
//! 打开会话 = 确认它此刻所有已结束的 run；会话保持打开时 run 结束也立即确认
//! （在读者眼前折叠）。

use leptos::prelude::*;

use crate::components::workflow_graph::run_status_presentation::status_dot_class;
use crate::components::workflow_graph::types::StepRunStatus;
use crate::lib::taskListItemPresentation::format_task_relative_time;
use crate::lib::workflowRunLine::{
    fold_workflow_run_rail, select_workflow_run_lines, settled_workflow_run_ids,
    workflow_run_parallel_phase_label, SessionWorkflowActivity, SessionWorkflowPhaseStatus,
    SessionWorkflowRunSummary, WorkflowRunRail,
};
use crate::ToolCallBlocks::i18n;

/// 行上的词（真源 `runStatusWord`，:114-116）：`chat.toolCall.workflow.run.status.{status}`。
fn run_status_word(run: &crate::lib::workflowRunLine::SessionWorkflowRunSummary) -> String {
    i18n::text(&format!(
        "chat.toolCall.workflow.run.status.{}",
        run.status
    ))
}

/// 行上的词（真源 `runLineText`，:118-139）：在跑 → 当前 phase 名
/// （无阶段词汇表 → 「Workflow」）；结束 → 中性词（+ phase / 原因）。
pub fn run_line_text(
    run: &crate::lib::workflowRunLine::SessionWorkflowRunSummary,
    rail: &WorkflowRunRail,
) -> String {
    let separator = " \u{b7} ";
    if crate::lib::workflowRunLine::is_session_workflow_run_live(&run.status) {
        if rail.implicit {
            return i18n::text("chat.toolCall.workflow.graph.phase.workflow");
        }
        if let Some(current_phase) = &run.current_phase {
            return current_phase.clone();
        }
        return run_status_word(run);
    }
    let word = run_status_word(run);
    if run.status == "errored" {
        if let Some(current_phase) = &run.current_phase {
            return format!("{word}{separator}{current_phase}");
        }
    }
    // stopped 的原因词（真源 :134-137）。
    if run.status == "stopped" {
        if let Some(reason) = &run.stop_reason {
            return format!(
                "{word}{separator}{}",
                i18n::text(&format!("chat.toolCall.workflow.run.stopReason.{reason}"))
            );
        }
    }
    word
}

/// run 的展示名（真源 `runName`，:141-143）：后台工作标题 > 「Workflow」。
pub fn run_name(run: &crate::lib::workflowRunLine::SessionWorkflowRunSummary) -> String {
    run.name
        .clone()
        .unwrap_or_else(|| i18n::text("chat.toolCall.workflow.graph.phase.workflow"))
}

/// tooltip 第二行（真源 `runTooltipDescription`，:146-165）：
/// `{phase} · {n agents working} · {elapsed}`，缺的段落省略。
/// `now` 由调用方注入（测试）；运行时传 `None` 走 `Date.now()`。
pub fn run_tooltip_description(
    run: &SessionWorkflowRunSummary,
    now: Option<i64>,
) -> Option<String> {
    let separator = " \u{b7} ";
    let mut parts: Vec<String> = Vec::new();
    // 并行时「当前阶段」不再是一个站：同时在跑的几站并排列出（真源 :151-154）。
    let parallel = workflow_run_parallel_phase_label(&run.phases);
    if let Some(parallel) = parallel {
        parts.push(parallel);
    } else if let Some(current_phase) = &run.current_phase {
        parts.push(current_phase.clone());
    }
    let live = crate::lib::workflowRunLine::is_session_workflow_run_live(&run.status);
    if live && run.agents_working > 0 {
        parts.push(i18n::format(
            "chat.toolCall.workflow.card.agentsWorking",
            &[("count".to_string(), run.agents_working.to_string())],
        ));
    }
    if let Some(started_at) = run.started_at {
        // 真源 `formatTaskRelativeTime(run.startedAt, intl)`（真源 :163）。
        let relative = match now {
            Some(now) => {
                let diff = now - started_at;
                let minutes = diff / 60_000;
                if minutes < 1 {
                    i18n::text("taskList.justNow")
                } else if minutes < 60 {
                    i18n::format(
                        "taskList.minutesAgo",
                        &[("minutes".to_string(), minutes.to_string())],
                    )
                } else {
                    let hours = minutes / 60;
                    if hours < 24 {
                        i18n::format(
                            "taskList.hoursAgo",
                            &[("hours".to_string(), hours.to_string())],
                        )
                    } else {
                        i18n::format(
                            "taskList.daysAgo",
                            &[("days".to_string(), (hours / 24).to_string())],
                        )
                    }
                }
            }
            // 无 now 注入时走 Date.now（运行时路径）。
            None => format_task_relative_time(started_at),
        };
        parts.push(relative);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(separator))
    }
}

fn phase_status_dot(status: SessionWorkflowPhaseStatus) -> &'static str {
    match status {
        SessionWorkflowPhaseStatus::Pending => status_dot_class(StepRunStatus::Pending),
        SessionWorkflowPhaseStatus::Running => status_dot_class(StepRunStatus::Running),
        SessionWorkflowPhaseStatus::Done => status_dot_class(StepRunStatus::Done),
        SessionWorkflowPhaseStatus::Failed => status_dot_class(StepRunStatus::Failed),
    }
}

/// 迷你轨道（真源 `RunRail`，:50-112）。
#[component]
fn RunRail(rail: WorkflowRunRail) -> impl IntoView {
    if rail.implicit {
        return view! {
            <span data-workflow-run-rail="true" data-implicit="true" class="flex shrink-0 items-center">
                <span
                    aria-hidden="true"
                    class=format!("size-1.5 rounded-full {}", phase_status_dot(SessionWorkflowPhaseStatus::Running))
                />
            </span>
        }
        .into_any();
    }
    view! {
        <span data-workflow-run-rail="true" class="flex shrink-0 items-center">
            {rail.stations
                .iter()
                .enumerate()
                .map(|(index, station)| {
                    let station_word = match station.status {
                        SessionWorkflowPhaseStatus::Pending => "pending",
                        SessionWorkflowPhaseStatus::Running => "running",
                        SessionWorkflowPhaseStatus::Done => "done",
                        SessionWorkflowPhaseStatus::Failed => "failed",
                    };
                    let reached_color = if station.reached {
                        "var(--color-workflow-trace-strong)"
                    } else {
                        "var(--color-workflow-trace)"
                    };
                    // 双线段：本站与前一站并行，控制流没有从那站走到这站（真源 :67-85）。
                    // 两条 1px 线相距 2px，宽度与墨色规则与普通段完全相同。
                    let segment = (index > 0).then(|| {
                        if station.twin {
                            view! {
                                <span
                                    aria-hidden="true"
                                    data-rail-segment=if station.reached { "strong" } else { "faint" }
                                    data-rail-twin="true"
                                    class="flex h-1 w-1.5 flex-col justify-between"
                                >
                                    <span class="h-px w-full" style=format!("background-color: {reached_color};") />
                                    <span class="h-px w-full" style=format!("background-color: {reached_color};") />
                                </span>
                            }
                            .into_any()
                        } else {
                            view! {
                                <span
                                    aria-hidden="true"
                                    data-rail-segment=if station.reached { "strong" } else { "faint" }
                                    class="h-px w-1.5"
                                    style=format!("background-color: {reached_color};")
                                />
                            }
                            .into_any()
                        }
                    });
                    view! {
                        <span class="flex items-center">
                            {segment}
                            <span
                                aria-hidden="true"
                                data-rail-station=station_word
                                title=station.name.clone()
                                class=format!("size-1.5 rounded-full {}", phase_status_dot(station.status))
                            />
                        </span>
                    }
                    .into_any()
                })
                .collect_view()}
            {(rail.hidden > 0).then(|| {
                view! {
                    <span class="ml-1 text-ui-xs leading-none text-foreground-subtlest">
                        {i18n::format(
                            "taskList.workflowRun.moreStations",
                            &[("count".to_string(), rail.hidden.to_string())],
                        )}
                    </span>
                }
            })}
        </span>
    }
    .into_any()
}

/// 运行行（真源 `TaskWorkflowRunLines`，:167-276）。
#[component]
pub fn TaskWorkflowRunLines(
    /// sessions-index 下发的会话工作流活动；缺席即什么都不画。
    activity: Option<SessionWorkflowActivity>,
    /// 会话是否正被打开：为真时确认它所有已结束的 run（结束的行随即折叠）。
    #[prop(default = false)] is_active: bool,
    /// 点击打开 run pane 所需的会话地址；缺席（手机首页）时运行行不是按钮。
    #[prop(optional)] session: Option<(String, Option<String>, String)>,
    /// `compact = 手机远控行（24px、更小字号）`。
    #[prop(default = false)] compact: bool,
    #[prop(optional, into)] class: String,
    /// 打开 run pane 的回调（真源 useWorkflowRunOpen 的 Rust 对应物）。
    #[prop(optional)] on_open_run: Option<Callback<crate::lib::workflowRunLine::SessionWorkflowRunSummary>>,
) -> impl IntoView {
    // 已确认集合（真源 useWorkflowRunAcknowledged，:175）：订阅版本驱动的谓词。
    let ack_version = crate::lib::workflowRunAckStore::use_workflow_run_acknowledged();
    // 打开会话 = 确认它此刻所有已结束的 run（真源 :178-182）。
    let activity_for_effect = activity.clone();
    Effect::new(move |_| {
        let _ = is_active;
        let settled = settled_workflow_run_ids(activity_for_effect.as_ref());
        if is_active && !settled.is_empty() {
            crate::lib::workflowRunAckStore::get_workflow_run_ack_store().acknowledge(&settled);
        }
    });
    let selection = Memo::new(move |_| {
        let _ = ack_version.get();
        select_workflow_run_lines(activity.as_ref(), &|run_id: &str| {
            crate::lib::workflowRunAckStore::get_workflow_run_ack_store().is_acknowledged(run_id)
        })
    });

    view! {
        <div
            data-workflow-run-lines="true"
            class=format!("flex min-w-0 max-w-full flex-col items-start {class}")
        >
            {move || {
                let selection = selection.get();
                if selection.lines.is_empty() {
                    return view! { <span class="hidden" /> }.into_any();
                }
                let interactive = on_open_run.is_some() && session.is_some();
                let interactive = move || interactive;
                let _ = interactive;
                let line_views: Vec<AnyView> = selection
                    .lines
                    .iter()
                    .map(|run| {
                        let rail = fold_workflow_run_rail(&run.phases);
                        let text = run_line_text(run, &rail);
                        let name = run_name(run);
                        let status_word = run_status_word(run);
                        let aria_label = i18n::format(
                            "taskList.workflowRun.ariaLabel",
                            &[
                                ("name".to_string(), name.clone()),
                                ("status".to_string(), status_word.clone()),
                            ],
                        );
                        let interactive = on_open_run.is_some() && session.is_some();
                        let run_id = run.run_id.clone();
                        let line_class = format!(
                            "flex min-w-0 max-w-full items-center gap-1.5 rounded-md text-foreground-subtle {} {}",
                            if compact { "h-6 text-ui-sm" } else { "h-5 text-ui-sm" },
                            if interactive {
                                "-ml-1 px-1 hover:bg-surface-hover hover:text-foreground"
                            } else {
                                ""
                            },
                        );
                        // 行内容（真源 :205-211）：Workflow 图标 + 迷你轨道 + 文本。
                        let content = move || -> AnyView {
                            view! {
                                <>
                                    <span class="inline-flex size-3 shrink-0 text-foreground-subtle">
                                        {crate::components::workflowIcons::icon_workflow()}
                                    </span>
                                    <RunRail rail=rail.clone() />
                                    <span class="min-w-0 truncate">{text.clone()}</span>
                                </>
                            }
                            .into_any()
                        };
                        let tooltip_title = format!("{name} \u{b7} {status_word}");
                        let description = run_tooltip_description(run, None);
                        let on_open = on_open_run.clone();
                        let session_for_click = session.clone();
                        let run_for_click = run.clone();
                        let line: AnyView = if interactive {
                            view! {
                                <button
                                    type="button"
                                    data-workflow-run-line="true"
                                    data-run-id=run_id.clone()
                                    data-run-status=run.status.clone()
                                    class=line_class.clone()
                                    aria-label=aria_label.clone()
                                    on:click=move |event: leptos::ev::MouseEvent| {
                                        // 行本身也可点（选中会话）；运行行是更具体的落点，
                                        // 不让点击再冒泡成一次普通选中（真源 :229-241）。
                                        event.prevent_default();
                                        event.stop_propagation();
                                        if let Some(cb) = on_open.clone() {
                                            cb.run(run_for_click.clone());
                                        }
                                        let _ = &session_for_click;
                                    }
                                >
                                    {content()}
                                </button>
                            }
                            .into_any()
                        } else {
                            view! {
                                <span
                                    data-workflow-run-line="true"
                                    data-run-id=run_id.clone()
                                    data-run-status=run.status.clone()
                                    class=line_class.clone()
                                    aria-label=aria_label.clone()
                                >
                                    {content()}
                                </span>
                            }
                            .into_any()
                        };
                        // tooltip（真源 ControlHintTooltip 包装，:251-261）：行 + 右侧提示。
                        view! {
                            <span class="inline-flex max-w-full flex-col" title=tooltip_title data-description=description.unwrap_or_default()>
                                {line}
                            </span>
                        }
                        .into_any()
                    })
                    .collect();
                let overflow_view = (selection.overflow > 0).then(|| {
                    view! {
                        <span
                            data-workflow-run-overflow=selection.overflow.to_string()
                            class=format!(
                                "text-foreground-subtlest {}",
                                if compact { "h-6 text-ui-sm" } else { "h-5 text-ui-xs" },
                            )
                        >
                            {i18n::format(
                                "taskList.workflowRun.moreRuns",
                                &[("count".to_string(), selection.overflow.to_string())],
                            )}
                        </span>
                    }
                });
                view! {
                    <>
                        {line_views.into_iter().collect_view()}
                        {overflow_view}
                    </>
                }
                .into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::workflowRunLine::{SessionWorkflowPhaseSummary, SessionWorkflowRunSummary};

    fn summary(status: &str, current_phase: Option<&str>, stop_reason: Option<&str>) -> SessionWorkflowRunSummary {
        SessionWorkflowRunSummary {
            run_id: "r1".into(),
            tool_call_id: None,
            name: None,
            status: status.into(),
            stop_reason: stop_reason.map(str::to_string),
            started_at: None,
            phases: vec![SessionWorkflowPhaseSummary {
                name: "计划".into(),
                status: SessionWorkflowPhaseStatus::Done,
                alongside: None,
            }],
            current_phase: current_phase.map(str::to_string),
            agents_working: 0,
        }
    }

    #[test]
    fn run_line_text_live_prefers_current_phase() {
        // 真源 :124-128 —— 在跑且有词汇表：currentPhase > 状态词。
        // （implicit rail 时「Workflow」先于 currentPhase，由下一条测试覆盖。）
        let phases = vec![SessionWorkflowPhaseSummary {
            name: "计划".into(),
            status: SessionWorkflowPhaseStatus::Done,
            alongside: None,
        }];
        let rail = fold_workflow_run_rail(&phases);
        assert!(!rail.implicit);
        let run = summary("running", Some("主体"), None);
        assert_eq!(run_line_text(&run, &rail), "主体");
        // 无 currentPhase：退回状态词。
        let run = summary("running", None, None);
        assert_eq!(run_line_text(&run, &rail), run_status_word(&run));
    }

    #[test]
    fn run_line_text_implicit_rail_says_workflow() {
        // 真源 :125-127 —— 脚本没有阶段词汇表：「Workflow」。
        let rail = fold_workflow_run_rail(&[]);
        assert!(rail.implicit);
        let run = summary("running", Some("主体"), None);
        assert_eq!(run_line_text(&run, &rail), i18n::text("chat.toolCall.workflow.graph.phase.workflow"));
    }

    #[test]
    fn run_line_text_errored_appends_phase_and_stopped_appends_reason() {
        // 真源 :130-137 —— errored + phase；stopped + 原因词。
        let phases = vec![SessionWorkflowPhaseSummary {
            name: "计划".into(),
            status: SessionWorkflowPhaseStatus::Done,
            alongside: None,
        }];
        let rail = fold_workflow_run_rail(&phases);
        assert!(!rail.implicit);
        let run = summary("errored", Some("主体"), None);
        let word = run_status_word(&run);
        assert_eq!(run_line_text(&run, &rail), format!("{word} · 主体"));
        let run = summary("stopped", None, Some("user"));
        let word = run_status_word(&run);
        assert_eq!(
            run_line_text(&run, &rail),
            format!("{word} · {}", i18n::text("chat.toolCall.workflow.run.stopReason.user"))
        );
    }

    #[test]
    fn run_name_falls_back_to_workflow() {
        // 真源 :141-143。
        let run = summary("running", None, None);
        assert_eq!(run_name(&run), i18n::text("chat.toolCall.workflow.graph.phase.workflow"));
        let mut named = run.clone();
        named.name = Some("重构登录".into());
        assert_eq!(run_name(&named), "重构登录");
    }

    #[test]
    fn tooltip_description_joins_present_parts() {
        // 真源 :146-165 —— phase · agents · elapsed，缺的省略。
        let mut run = summary("running", Some("主体"), None);
        run.agents_working = 2;
        run.started_at = Some(1_000);
        let description = run_tooltip_description(&run, Some(1_000 + 5 * 60_000));
        let text = description.unwrap();
        assert!(text.contains("主体"));
        assert!(text.contains('2'));
        assert!(text.contains('5'));
        // 零 agents、无 startedAt：只剩 phase 一段。
        let mut bare = summary("running", Some("主体"), None);
        bare.agents_working = 0;
        let description = run_tooltip_description(&bare, Some(1_000));
        assert!(description.unwrap().contains("主体"));
    }
}
