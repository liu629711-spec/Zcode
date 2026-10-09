//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowRunDigest.tsx`（363 行）。
//!
//! 轮尾摘要运行卡：下方默认展开，无箭头但仍可收起；状态由标题表达。
//! 阶段线只在有活投影且有图时画：没有投影的图全是 pending 灯，会把一条已完成的
//! run 画成没跑过。
//!
//! 去掉箭头不代表取消折叠。仅空白区域切换，子控件继续执行各自的操作
//! （`hitsControl`：button / a / input / textarea / select / role=button 之外的空白才算）。
//!
//! 与真源的偏离（Rust 侧）：「配置」弹层（`WorkflowRunSettingsPopover`）依赖
//! `ModelConfigSelect` / `ThoughtLevelCycleControl` / `useModelSelectionView` 子系统，
//! 尚未迁移——Configure 槽位等该子系统迁入后接入；Stop / Resume 槽位完整。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::timeline_model::build_workflow_timeline;
use crate::components::timeline_summary::workflow_card_detail;
use crate::components::workflow_graph::run_state::WorkflowRunState;
use crate::components::workflow_graph::types::WorkflowCausalityGraphData;
use crate::ToolCallBlocks::i18n;

use super::WorkflowCardChrome::{workflow_run_kind_message_id, WorkflowCardHeader};
use super::WorkflowArtifactStrip::WorkflowArtifactStrip;
use super::WorkflowTimeline::{timeline_height, WorkflowTimeline};
use super::WorkflowTruncatedNotice::WorkflowTruncatedNotice;

/// `WorkflowRunDigestData`（真源 `WorkflowRunDigestProps`，:33-75）。
pub struct WorkflowRunDigestData {
    pub name: String,
    pub run_id: String,
    /// 该 run 的发起图（按发起 toolCallId 查到）；缺席即画不出阶段线。
    pub graph: Option<WorkflowCausalityGraphData>,
    /// 活投影的联接摘要。缺席 = run 不在投影里：卡退成中性单行。
    pub summary: Option<crate::ToolCallBlocks::fileSummaryTypes::WorkflowRunCardSummary>,
    /// 该 run 停驻的待答问题数；> 0 时表头出现警示色芯片。
    pub pending_questions: i64,
    /// 打开 run 详情；缺席即无 ⤢、芯片不可点、「还有 n 个」那一行是静态的。
    pub on_open_run: Option<Callback<Option<String>>>,
    /// 恢复 run；只在 `summary.resumable` 且回调在场时渲染 Resume。
    pub on_resume: Option<Callback<()>>,
    /// 停止 run；只在 running 且回调在场时渲染 Stop，与 Resume 占同一个位置。
    pub on_cancel: Option<Callback<()>>,
    /// 点一枚药丸开那个子代理的 transcript；缺席即药丸不可点。
    pub on_open_pill: Option<Callback<crate::components::timeline_model::TimelinePill>>,
    /// 点脚本药丸开脚本 transcript、落到那一站；缺席即脚本药丸不可点。
    pub on_open_workspace: Option<Callback<crate::components::timeline_model::TimelinePill>>,
    /// 点一枚产物药丸开产物 tab；缺席即产物药丸禁用。
    pub on_open_artifact: Option<Callback<String>>,
    /// testid 后缀（unit.key + toolCallId）。
    pub test_id_key: String,
}

/// 子控件谓词（真源 `hitsControl`，:117-123）：点到控件（button / a / input /
/// textarea / select / role=button）且不是卡身本身 → 这一下不属于整块开关。
pub fn hits_control(target: Option<web_sys::EventTarget>, root: &web_sys::Element) -> bool {
    let Some(target) = target else {
        return false;
    };
    let Ok(element) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };
    element
        .closest("button, a, input, textarea, select, [role='button']")
        .ok()
        .flatten()
        .map(|control| {
            !control.is_same_node(Some(root.unchecked_ref::<web_sys::Node>()))
        })
        .unwrap_or(false)
}

/// 摘要里的 run 投影（真源 `summary?.run`）：活投影 run 直接是 WorkflowRunState。
fn summary_run(
    summary: &crate::ToolCallBlocks::fileSummaryTypes::WorkflowRunCardSummary,
) -> Option<WorkflowRunState> {
    summary
        .run
        .as_ref()
        .and_then(|raw| serde_json::from_value(raw.clone()).ok())
}

#[component]
pub fn WorkflowRunDigest(input: WorkflowRunDigestData) -> impl IntoView {
    let expanded = RwSignal::new(true);

    // 阶段线只在有活投影且有图时画（真源 :97-104）。
    let run = input.summary.as_ref().and_then(summary_run);
    let model = input
        .graph
        .as_ref()
        .filter(|graph| !graph.steps.is_empty())
        .zip(run.as_ref())
        .map(|(graph, run)| build_workflow_timeline(graph, Some(run)));
    let has_rail = model
        .as_ref()
        .map(|model| !model.stations.is_empty())
        .unwrap_or(false);
    // 收起时只隐藏代理，保留阶段线作为运行进度概览（真源 :106-115）。
    let shown: Option<crate::components::timeline_model::WorkflowTimelineModel> =
        model.clone().map(|model| {
            if expanded.get_untracked() {
                model
            } else {
                let mut clipped = model;
                for station in clipped.stations.iter_mut() {
                    station.pills = Vec::new();
                }
                clipped
            }
        });

    // 卡上刻意**不**画 lineage（真源 :125-129）：「调整自 / 已被替代」只在详情页与
    // 确认窗说；卡只换种类词。细节串的最后一段是子代理模型名。
    let card_detail = workflow_card_detail(
        model.as_ref(),
        input.graph.as_ref(),
        run.as_ref(),
        input
            .summary
            .as_ref()
            .and_then(|summary| summary.run.as_ref())
            .and_then(|raw| raw.get("subagentModel").and_then(|value| value.as_str())),
        None,
    );
    let live = input
        .summary
        .as_ref()
        .map(|summary| summary.status == "running")
        .unwrap_or(false);
    let kind = i18n::text(match &input.summary {
        None => super::WorkflowCardChrome::WORKFLOW_RUN_ENDED_KIND_ID,
        Some(summary) => workflow_run_kind_message_id(
            summary
                .status
                .parse::<WorkflowRunStatusWord>()
                .ok()
                .map(|word| word.0)
                .unwrap_or(crate::components::workflow_graph::run_state::WorkflowRunStatus::Pending),
            summary.stop_reason.as_deref(),
        ),
    });
    let questions_label = (input.pending_questions > 0).then(|| {
        i18n::format(
            if input.pending_questions == 1 {
                "chat.toolCall.workflow.digest.question"
            } else {
                "chat.toolCall.workflow.digest.questions"
            },
            &[("count".to_string(), input.pending_questions.to_string())],
        )
    });
    // Resume 与 Stop 互斥（真源 :166-187），共用表头右侧同一个位置。
    let resume = (input
        .summary
        .as_ref()
        .map(|summary| summary.resumable)
        .unwrap_or(false)
        && input.on_resume.is_some())
    .then(|| {
        let on_resume = input.on_resume.clone().unwrap();
        view! {
            <button
                data-testid="workflow-digest-resume"
                on:click=move |_| on_resume.run(())
                class="inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-md border border-border bg-transparent px-3 text-ui-sm font-medium hover:bg-surface-hover"
                type="button"
            >
                <span class="inline-flex size-3.5">
                    {crate::components::workflowIcons::icon_rotate_ccw()}
                </span>
                {i18n::text("chat.toolCall.workflow.run.resume")}
            </button>
        }
        .into_any()
    });
    let cancel = (live && input.on_cancel.is_some()).then(|| {
        // 「正在停止」是它自己的局部状态；宿主按 run 状态给它 key，状态一变即重挂、
        // 自然复位（真源 :323-330）。
        let on_cancel = input.on_cancel.clone().unwrap();
        view! {
            <CancelRunButton on_cancel=on_cancel />
        }
        .into_any()
    });
    let trailing = match (resume, cancel) {
        (Some(resume), Some(cancel)) => Some(
            view! { <>{resume}{cancel}</> }.into_any(),
        ),
        (Some(view), None) | (None, Some(view)) => Some(view),
        (None, None) => None,
    };

    // 待答问题的警示色芯片（真源 :134-165）：可点开整个 run，不可点即静态。
    let questions = questions_label.map(|questions_label| {
        let on_open_run = input.on_open_run.clone();
        match on_open_run {
            Some(on_open_run) => view! {
                <button
                    class="wf-arrive flex shrink-0 cursor-pointer items-center gap-1 rounded-full bg-[color-mix(in_oklab,var(--color-warning)_12%,transparent)] py-0.5 pl-1.5 pr-2 text-ui-xs font-medium text-warning outline-none transition-colors hover:bg-[color-mix(in_oklab,var(--color-warning)_20%,transparent)] focus-visible:ring-2 focus-visible:ring-ring/40"
                    data-testid="workflow-digest-questions"
                    on:click=move |_| on_open_run.run(None)
                    type="button"
                >
                    <span class="inline-flex size-3">
                        {crate::components::workflowIcons::icon_message_circle_question()}
                    </span>
                    {questions_label.clone()}
                </button>
            }
            .into_any(),
            None => view! {
                <span class="wf-arrive flex shrink-0 items-center gap-1 rounded-full bg-[color-mix(in_oklab,var(--color-warning)_12%,transparent)] py-0.5 pl-1.5 pr-2 text-ui-xs font-medium text-warning"
                    data-testid="workflow-digest-questions"
                >
                    <span class="inline-flex size-3">
                        {crate::components::workflowIcons::icon_message_circle_question()}
                    </span>
                    {questions_label.clone()}
                </span>
            }
            .into_any(),
        }
    });

    let status_word = match &input.summary {
        Some(summary) => summary.status.clone(),
        None => "absent".to_string(),
    };
    let has_rail_signal = RwSignal::new(has_rail);
    let expanded_read = expanded;
    let artifacts: Vec<super::WorkflowArtifactPill::ArtifactPillData> = run
        .as_ref()
        .and_then(|run| run.artifacts.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|artifact| super::WorkflowArtifactPill::ArtifactPillData {
            id: artifact.id,
            kind: Some(artifact.kind),
            title: artifact.title,
            version: Some(artifact.version),
        })
        .collect();

    view! {
        <section
            aria-label=kind.clone()
            class=format!(
                "wf-motion wf-arrive flex w-full min-w-0 flex-col gap-1 rounded-xl border border-border/70 bg-card/70 px-3.5 pb-2 pt-1.5 outline-none {}",
                if has_rail {
                    "cursor-pointer focus-visible:ring-2 focus-visible:ring-ring/40"
                } else {
                    ""
                },
            )
            aria-expanded=(has_rail_signal.get()).then(|| expanded_read.get_untracked())
            data-testid=format!("workflow-run-digest-{}", input.test_id_key)
            data-workflow-run-digest="true"
            data-workflow-run-id=input.run_id.clone()
            data-workflow-run-status=status_word.clone()
            role=has_rail.then_some("button")
            tabindex=has_rail.then_some("0")
            on:click=move |event| {
                // 仅空白区域切换（真源 :209-212）。
                if has_rail_signal.get_untracked()
                    && !hits_control(Some(event.target().unwrap()), event.current_target().unwrap().unchecked_ref())
                {
                    expanded.update(|value| *value = !*value);
                }
            }
            on:keydown=move |event: leptos::ev::KeyboardEvent| {
                if !has_rail_signal.get_untracked()
                    || hits_control(Some(event.target().unwrap()), event.current_target().unwrap().unchecked_ref())
                    || (event.key() != "Enter" && event.key() != " ")
                {
                    return;
                }
                event.prevent_default();
                expanded.update(|value| *value = !*value);
            }
        >
            <WorkflowCardHeader
                detail=card_detail.as_ref().map(|detail| detail.detail.clone())
                detail_title=card_detail.as_ref().and_then(|detail| detail.title.clone())
                expanded=expanded_read.get_untracked()
                kind=kind.clone()
                leading=questions
                live=live
                name=input.name.clone()
                status=None
                trailing=trailing
                on_open_details=input.on_open_run.clone().map(|cb| Callback::new(move |_| cb.run(None)))
                on_toggle=None
                toggle_label=None
            />
            {(shown.is_some() && has_rail).then(|| {
                let shown = shown.clone().unwrap();
                // 收起时只隐藏代理，保留阶段线作为运行进度概览（真源 :244-261）。
                // 「还有 n 个」落站用的站序（先取，model 马上要被 view 吃掉）。
                let stations_for_more = shown.stations.clone();
                // 高度也要先算（view! 的 children 先于属性求值）。
                let plot_height = timeline_height(&shown) + 8.0;
                view! {
                    <div
                        class="wf-digest-plot overflow-hidden"
                        data-testid="workflow-digest-plot"
                        style=format!("height: {plot_height}px;")
                    >
                        <WorkflowTimeline
                            class="py-1".to_string()
                            model=shown
                            on_select_station=None
                            on_open_more=input.on_open_run.clone().map(|cb| {
                                // 「还有 n 个」→ 开 run 详情并落到这一站
                                //（真源 :256-258：onOpenRun({ phaseId: station.id })）。
                                let stations = stations_for_more;
                                Callback::new(move |station_index: usize| {
                                    cb.run(stations
                                        .get(station_index)
                                        .map(|station| station.id.clone()));
                                })
                            })
                            on_open_pill=input.on_open_pill.clone()
                            on_open_workspace=input.on_open_workspace.clone()
                        />
                    </div>
                }
                .into_any()
            })}
            // 时间线下的「仅展示 n/m 步的详情」：表头的计数已是真实步数，这一行只说
            // 停在界上的是**详情**（真源 :262-266）。没有轨道的卡按规范是单行，一并缺席。
            {has_rail.then(|| {
                view! {
                    <WorkflowTruncatedNotice
                        run=run.clone()
                        test_id="workflow-digest-truncated".to_string()
                    />
                }
                .into_any()
            })}
            // 产物条（真源 :267-277）：run 的交付物，收起与展开态都在。
            {(!artifacts.is_empty()).then(|| {
                view! {
                    <WorkflowArtifactStrip
                        artifacts=artifacts
                        class="pb-0.5 pt-0.5".to_string()
                        more_test_id=Some("workflow-digest-artifacts-more".to_string())
                        on_open_artifact=input.on_open_artifact.clone()
                        pill_test_id=Some("workflow-digest-artifact".to_string())
                        test_id=Some("workflow-digest-artifacts".to_string())
                    />
                }
                .into_any()
            })}
        </section>
    }
    .into_any()
}

/// 状态词的轻量解析（真源 status 是字面量联合；Rust 摘要存 wire 字符串）。
struct WorkflowRunStatusWord(crate::components::workflow_graph::run_state::WorkflowRunStatus);

impl std::str::FromStr for WorkflowRunStatusWord {
    type Err = ();
    fn from_str(word: &str) -> Result<Self, Self::Err> {
        Ok(WorkflowRunStatusWord(match word {
            "pending" => crate::components::workflow_graph::run_state::WorkflowRunStatus::Pending,
            "running" => crate::components::workflow_graph::run_state::WorkflowRunStatus::Running,
            "completed" => {
                crate::components::workflow_graph::run_state::WorkflowRunStatus::Completed
            }
            "errored" => crate::components::workflow_graph::run_state::WorkflowRunStatus::Errored,
            "stopped" => crate::components::workflow_graph::run_state::WorkflowRunStatus::Stopped,
            _ => return Err(()),
        }))
    }
}

/// 表头的 Stop 钮（真源 `CancelRunButton`，:331-363）。
/// 「正在停止」是它自己的局部状态；提示带第二行「停止后可以随时恢复，已完成的步骤
/// 会保留。」正在停止时那句话撤走。
#[component]
fn CancelRunButton(on_cancel: Callback<()>) -> impl IntoView {
    let cancelling = RwSignal::new(false);
    let label = move || {
        i18n::text(if cancelling.get() {
            "chat.toolCall.workflow.run.cancelling"
        } else {
            "chat.toolCall.workflow.run.cancel"
        })
    };
    let description = move || {
        (!cancelling.get()).then(|| i18n::text("chat.toolCall.workflow.run.stopHint"))
    };
    view! {
        <div class="relative inline-flex">
            <button
                aria-label=label
                data-testid="workflow-digest-cancel"
                disabled=cancelling.get()
                on:click=move |_| {
                    cancelling.set(true);
                    on_cancel.run(());
                }
                class="inline-flex h-6 w-6 shrink-0 items-center justify-center gap-1.5 rounded-md text-foreground-subtle hover:bg-surface-hover hover:text-foreground disabled:opacity-50"
                type="button"
            >
                <span class="inline-flex size-3.5 fill-current">
                    {crate::components::workflowIcons::icon_square_filled()}
                </span>
            </button>
            <span class="hidden">{description}</span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use crate::components::workflow_graph::run_state::WorkflowRunState;

    // 组件内部的折叠 / 键盘切换依赖 Leptos executor 与 DOM，宿主测试跑不动
    // （上一批 ToolLayout 同一条纪律）：判定逻辑 `hits_control` 的纯规则已由
    // 真源 :117-123 的谓词逐字落实现；数据面的 `summary_run` 解析走 serde，失败即
    // None（卡退中性单行），与真源「不在投影里」同一分支。
    #[test]
    fn digest_has_no_host_testable_pure_layer_beyondserde() {
        // 纯层只有 run 投影解析：坏 JSON 必须静默退化而不是 panic。
        let bad: Result<WorkflowRunState, _> = serde_json::from_value(serde_json::json!({
            "runId": "r1", "status": "unknown-status", "actors": [], "nodes": []
        }));
        assert!(bad.is_err());
    }
}
