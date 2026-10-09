//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/timeline-model.ts`（477 行）。
//!
//! 时间线模型。
//!
//! ★真源 :38-48 注释——一个纯函数把有界 display 与 `workflowRuns` 投影折成三样东西：
//! 站（阶段）、轨道段（相邻且有边的两站之间）、弧（非相邻的边）。聊天卡、确认窗与侧栏清单
//! 都从这里出发——不变式 1「一个模型，三处消费」。无 React、无 DOM、无时间。
//!
//! 顺序是**声明序**（载荷 `phases[]` = 首个 `phase()` 标记的顺序），不再分秩：两个只共享前驱的
//! 兄弟阶段照声明序排成一列，它们之间的边成为弧。回边按**方向**判定（目标站在左），
//! 不读载荷的 `back`——分析器把再入（第二个 `phase("plan")` 标记）标成前向边，
//! 画面上它仍然向左。
//!
//! ★关于 `sharedTimelineModel`（真源 timeline-cache.ts）——Rust 侧**不搬**：
//! 那层是 JS 的 `WeakMap` 按对象身份缓存，靠的是「协议对象不可变 + 内容一变必然换新对象」
//! 这条性质。Rust 侧模型是**借用输入构造出来的拥有值**，缓存的键得是身份而不是值；
//! 把它留给渲染层的 `Memo`/信号去管（Leptos 的 memo 键就是 `PartialEq` 值），
//! 语义等价而不多一层跨语言的缓存表。

use std::collections::{HashMap, HashSet};

use crate::components::station_observation::{
    StationUnlisted, observe_phase, phase_entry_for, site_ids_of, station_unlisted,
};
use crate::components::timeline_bands::{
    ArcSpec, RailSpec, TimelineRailKind, assign_air_lanes, band_of, fold_phase_bands,
    fold_phase_edges, track_of,
};
use crate::components::workflow_graph::instance_phases::phase_binder;
use crate::components::workflow_graph::lane_name::{LaneRef, lane_refs_by_id};
use crate::components::workflow_graph::participant_model::{
    live_participant_view, participant_status, participants_of_phase, with_implicit_phase,
};
use crate::components::workflow_graph::phase_model::phase_members;
use crate::components::workflow_graph::phase_name::{PhaseNaming, phase_name_matches};
use crate::components::workflow_graph::run_state::{WorkflowRunNode, WorkflowRunState};
use crate::components::workflow_graph::run_status::{collapse_statuses, workflow_run_overlay};
use crate::components::workflow_graph::types::{
    LaneClass, StepRunStatus, WorkflowCausalityGraphData, lane_class_of,
};

// 真源 :51-53 是再导出：弧的分道是纯下标的组合学，与带的折叠同住 timeline-bands.rs。
pub use crate::components::timeline_bands::{arc_lane_count, assign_arc_lanes};
pub use crate::components::timeline_bands::TimelineRailKind as RailKind;

/// `TimelineInk`（真源 :50）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineInk {
    Faint,
    Strong,
    March,
}

impl TimelineInk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Faint => "faint",
            Self::Strong => "strong",
            Self::March => "march",
        }
    }
}

/// 一枚药丸交出的实例抓手（真源 :67）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillInstance {
    pub site_id: String,
    pub ordinal: i64,
    pub session_id: Option<String>,
}

/// 药丸交出的**槽位**身份（真源 :73）。
///
/// ★真源 :68-72 注释——有实例就是实例的序号；没有实例就是它**将来**的序号
/// ——成员卡 `member.index + 1`，单卡与 `many` 卡 1（引擎按站点顺序发号）。
/// 只有活的 run 里的 agent 车道才有；有了它，还没启动的子代理也能开一个占位的
/// transcript tab。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillSlot {
    pub site_id: String,
    pub ordinal: i64,
}

/// `TimelinePill`（真源 :55-84）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelinePill {
    /// 拆分后的参与者 id（实例卡 `参与者@ordinal`）；React key 与打开实例的抓手。
    pub key: String,
    pub lane: LaneRef,
    pub lane_class: LaneClass,
    /// 引擎发出的运行时名（persona 可能改写脚本里的名字）；缺席时渲染车道显示名。
    pub runtime_name: Option<String>,
    /// workflow 内按代理身份分配的头像编号，跨阶段复用。
    pub avatar_index: Option<usize>,
    /// 无 run 时 `None`（静态药丸，与 pending 逐像素相同）。
    pub status: Option<StepRunStatus>,
    /// 该药丸在投影里对应的实例；合成车道（workspace / unknown）没有。
    pub instance: Option<PillInstance>,
    pub slot: Option<PillSlot>,
    /// 脚本药丸交出的抓手：它没有实例、没有会话，能开的是**整个 run 的脚本
    /// transcript**，落到这一站的第一张卡。与 `slot` 互斥：只有活的 run 里的
    /// workspace 车道才有；没有 run 就没有可开的东西。
    pub workspace: Option<PillWorkspace>,
    /// 该参与者在本阶段的 step id（侧栏行的活动与计数素材）。
    pub step_ids: Vec<String>,
    /// 该实例有待答的升级问题（`run.pendingQuestions`）；名册的钉位规则读它。
    pub asking: bool,
}

/// `workspace`（真源 :79）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillWorkspace {
    pub phase_id: String,
}

/// 站的分数（真源 :101）：`settled / observed`。
///
/// ★真源 :100 注释——一个节点都没观察到时缺席。**表外已结算的实例也在这两个数里**
/// （分子分母一起抬，见 `station_observation.rs`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationFraction {
    pub settled: i64,
    pub observed: i64,
}

/// `TimelineStation`（真源 :86-106）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineStation {
    pub id: String,
    pub naming: PhaseNaming,
    pub pills: Vec<TimelinePill>,
    /// 成员 step 状态的折叠；无 run 时 `None`。
    pub status: Option<StepRunStatus>,
    /// 投影里存在任一节点落在该站的站点上。
    pub visited: bool,
    /// 该站站点上节点的最大 ordinal；0 = 未到。
    pub rounds: i64,
    /// 是任一回边（向左的弧）的源或目标——只有这样的站才显示 `⟳ n`。
    pub on_loop: bool,
    /// 所在的轨道（`timeline_bands.rs`）；带外一律 0，也就是主线。
    pub track: i64,
    pub fraction: Option<StationFraction>,
    /// 界在这一站花掉的表外条目（`station_observation.rs`）；一条都没少时缺席。
    pub unlisted: Option<StationUnlisted>,
    /// 流式草稿里尚未闭合的最后一站。
    pub typing: bool,
}

/// `TimelineRail`（真源 :109-115）：一条轨道上前后相接的两站之间的一段轨道。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineRail {
    pub from: usize,
    pub to: usize,
    pub ink: TimelineInk,
    /// 缺席 = 一条轨道上的普通段；带的两端是 `fork` / `merge`，
    /// 带内跨轨道的相邻两站是 `twin`。
    pub kind: Option<TimelineRailKind>,
}

/// `TimelineArc`（真源 :121-128）。
///
/// ★真源 :117-120 注释——非相邻的边：`to < from` 是回边。`lane` 从 0 起，
/// 贴近轨道的是 0；只有在 x 上**相交**的弧才分道（见 `assign_arc_lanes`），
/// 互不相干的弧同高。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineArc {
    pub from: usize,
    pub to: usize,
    pub lane: i64,
    pub ink: TimelineInk,
    /// 画在哪条轨道的空中；分道按空各算各的，所以 `arc_lane_count` 要先按 `air` 筛。
    pub air: i64,
}

/// `TimelineTrack`（真源 :131-137）：带内的一条轨道；`stations` 是它的成员，声明序。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineTrack {
    pub stations: Vec<usize>,
    /// 分叉进入这条轨道的墨；没有前驱时按首站到没到过。
    pub entry: TimelineInk,
    /// 这条轨道汇合出去的墨；没有汇合站时按末站到没到过。
    pub exit: TimelineInk,
}

/// `TimelineBand`（真源 :140-148）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineBand {
    pub from: usize,
    pub to: usize,
    /// 分叉所在的前驱站；缺席时画面上只有一小截尾巴。
    pub pred: Option<usize>,
    /// 汇合所在的后继站；缺席时画面上只有一小截残段。
    pub join: Option<usize>,
    pub tracks: Vec<TimelineTrack>,
}

/// `WorkflowTimelineModel`（真源 :150-168）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTimelineModel {
    pub stations: Vec<TimelineStation>,
    pub rails: Vec<TimelineRail>,
    pub arcs: Vec<TimelineArc>,
    /// 并行阶段折成的带，按 `from` 升序；没有 `alongside` 的时间线是空的。
    pub bands: Vec<TimelineBand>,
    /// 正在运行的站（多个时取最右）；无则 `None`。
    pub running_index: Option<usize>,
    /// 是否有 run 投影参与（决定灯 / 墨迹是否有话说）。
    pub live: bool,
    /// ★真源 :160-165 —— run 级停滞：**只属于 run**——单个 actor 在等槽位是正常排队，
    /// 不是停滞；停滞色因此只染整条时间线，不染任何一枚药丸。
    pub stalled: bool,
    /// 流式草稿：站由笔逐字写出，子代理只计数不画（`draft-scan.ts`）。
    /// 分析器的模型没有它。
    pub draft: Option<DraftAgents>,
}

/// `draft`（真源 :167）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftAgents {
    pub agents: usize,
}

/// `isCurrentPhase`（真源 :171-173）：`currentPhase` 与一站的关联，
/// 与 `phase_entry_for` 同一条名字规则。
fn is_current_phase(run: Option<&WorkflowRunState>, name: Option<&str>) -> bool {
    phase_name_matches(name, run.and_then(|r| r.current_phase.as_deref()))
}

/// `stationStatus`（真源 :182-197）：站的灯。
///
/// ★真源 :175-181 注释——成员节点先说话：running / failed 是硬事实；
/// 之后才轮到控制流：这一站是当前阶段且 run 还在跑，就是 running（第一个 ask 派发之前、
/// 最后一个 ask 结算之后下一个标记到来之前，控制流都在这一站）；一个节点都没观察到的站
/// （零成员，或整站被跳过）只能靠进入记录点灯。`node_status` 是折叠的结果，
/// 缺席（`None`）就是「没有节点」——折叠不会为控制流没走的站点造一个 pending。
fn station_status(
    run: Option<&WorkflowRunState>,
    node_status: Option<StepRunStatus>,
    current: bool,
    entered: bool,
) -> Option<StepRunStatus> {
    let run = run?;
    if node_status == Some(StepRunStatus::Running) || node_status == Some(StepRunStatus::Failed) {
        return node_status;
    }
    let live = run.status.is_live();
    if current && live {
        return Some(StepRunStatus::Running);
    }
    // ★真源 :192-194 —— 当前阶段随 run 的终态收场：失败发生在这一站
    // （不管它有没有节点）；cancelled 与节点的画法一致，同样是 failed。
    if current {
        return Some(if run.status == super::workflow_graph::run_state::WorkflowRunStatus::Completed {
            StepRunStatus::Done
        } else {
            StepRunStatus::Failed
        });
    }
    if node_status.is_some() {
        return node_status;
    }
    Some(if entered {
        StepRunStatus::Done
    } else {
        StepRunStatus::Pending
    })
}

/// `buildWorkflowTimeline`（真源 :204-209）。
///
/// ★真源 :199-203 注释——一个模型，三处消费（不变式 1）：卡、详情页与侧栏清单
/// 在同一帧里拿到**同一个**模型对象。模型是只读的，没有消费者改它。
///
/// Rust 侧每次调用重算（缓存键是值，见文件头关于 `sharedTimelineModel` 的说明）；
/// 调用方（渲染层）按 `Memo` 复用。
pub fn build_workflow_timeline(
    input: &WorkflowCausalityGraphData,
    run: Option<&WorkflowRunState>,
) -> WorkflowTimelineModel {
    compute_workflow_timeline(input, run)
}

/// `computeWorkflowTimeline`（真源 :211-451）。
fn compute_workflow_timeline(
    input: &WorkflowCausalityGraphData,
    run: Option<&WorkflowRunState>,
) -> WorkflowTimelineModel {
    let graph = with_implicit_phase(input);
    let phases = graph.phases.clone().unwrap_or_default();
    let index: HashMap<&str, usize> = phases
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    let members = phase_members(&graph);
    let overlay = workflow_run_overlay(run, &graph);
    let live_view = live_participant_view(&graph, run);
    let mut binder = phase_binder(&graph, run);
    let lane_refs = lane_refs_by_id(&graph.lanes);
    let session_by_instance: HashMap<String, Option<String>> = run
        .map(|r| {
            r.actors
                .iter()
                .map(|a| (format!("{}@{}", a.site_id, a.ordinal), a.session_id.clone()))
                .collect()
        })
        .unwrap_or_default();
    let asking_instances: HashSet<String> = run
        .map(|r| {
            r.pending_questions
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|q| q.actor_site_id.is_some() && q.actor_ordinal.is_some())
                .map(|q| format!("{}@{}", q.actor_site_id.as_deref().unwrap_or(""), q.actor_ordinal.unwrap_or(0)))
                .collect()
        })
        .unwrap_or_default();

    // 边：先按下标去重（自环与指向未列出阶段的边在这一粒度上没有话说），
    // 再交给带的折叠——相邻的成轨道段，其余成弧，带把分叉与汇合那几条吃掉。
    let mut edges: Vec<(i64, i64)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for edge in graph.phase_edges.as_deref().unwrap_or_default() {
        let (Some(&from), Some(&to)) = (index.get(edge.from.as_str()), index.get(edge.to.as_str()))
        else {
            continue;
        };
        if from == to {
            continue;
        }
        let key = format!("{from}>{to}");
        if !seen.insert(key) {
            continue;
        }
        edges.push((from as i64, to as i64));
    }
    let alongside: Vec<Vec<i64>> = phases
        .iter()
        .map(|phase| {
            phase
                .alongside
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter_map(|id| index.get(id.as_str()).map(|&at| at as i64))
                .collect()
        })
        .collect();
    let folded = fold_phase_bands(phases.len() as i64, &alongside);
    let fold = fold_phase_edges(phases.len() as i64, &folded, &edges);
    // 弧按跨度从短到长排（真源 :257-259）。
    let mut arc_pairs: Vec<ArcSpec> = fold.arcs.clone();
    arc_pairs.sort_by_key(|a| (a.from - a.to).abs());

    // 回边的两端上环；端点在带里时整条带都上环——带是一个节点，再入的是整条带。
    let mut on_loop: HashSet<usize> = HashSet::new();
    for arc in &arc_pairs {
        if arc.to >= arc.from {
            continue;
        }
        for end in [arc.from, arc.to] {
            match band_of(&folded, end) {
                None => {
                    on_loop.insert(end as usize);
                }
                Some(band) => {
                    for i in band.from..=band.to {
                        on_loop.insert(i as usize);
                    }
                }
            }
        }
    }

    // 名称哈希会碰撞；以实例身份分配连续编号，同一代理跨阶段保持同一头像。
    let mut avatar_indexes: HashMap<String, usize> = HashMap::new();
    let mut stations: Vec<TimelineStation> = Vec::with_capacity(phases.len());
    for (i, phase) in phases.iter().enumerate() {
        let empty_members: Vec<crate::components::workflow_graph::types::WorkflowStepData> = Vec::new();
        let member_steps = members.get(&phase.id).unwrap_or(&empty_members);
        let unlisted = station_unlisted(run, &mut binder, &phase.id);
        let entry = phase_entry_for(run, phase.name.as_deref());
        let sites = site_ids_of(member_steps);
        let observed = {
            // 真源 :280 —— `belongs` 闭包在真源里捕获 binder（`&mut`）；
            // Rust 侧把判定写在这里，逐节点问 binder，语义一致。
            let phase_id = phase.id.clone();
            observe_phase(
                run,
                &sites,
                entry,
                |node: &WorkflowRunNode| binder.has(&phase_id, node.phase_name.as_deref()),
                unlisted,
            )
        };
        let pills: Vec<TimelinePill> = participants_of_phase(&live_view.graph, &phase.id)
            .into_iter()
            .map(|participant| {
                let instance = live_view.instances.get(&participant.id);
                let lane = lane_refs.get(&participant.lane).cloned().unwrap_or(LaneRef {
                    id: participant.lane.clone(),
                    naming: crate::components::workflow_graph::lane_name::LaneNaming {
                        lane_class: Some(lane_class_of(&participant.lane)),
                        name: None,
                        name_pattern: None,
                        anonymous_index: None,
                    },
                });
                // ★真源 :289-290 —— 折叠为空 = 这个子代理在这一站、这一次 run 里
                // 什么都没做：空心是诚实的。`Some(Pending)` 只留给无 run 的静态药丸。
                let status = if run.is_none() {
                    None
                } else {
                    Some(
                        participant_status(
                            &participant,
                            Some(&overlay.statuses),
                            Some(&live_view.participant_statuses),
                        )
                        .unwrap_or(StepRunStatus::Pending),
                    )
                };
                let session_id = instance.and_then(|i| {
                    session_by_instance
                        .get(&format!("{}@{}", participant.lane, i.ordinal))
                        .cloned()
                        .flatten()
                });
                let member_index = participant.member.as_ref().map(|m| m.index).unwrap_or(0);
                let lane_class = lane.naming.lane_class.unwrap_or(LaneClass::Agent);
                let slot = match run {
                    Some(_) if lane_class == LaneClass::Agent => Some(PillSlot {
                        ordinal: instance.map(|i| i.ordinal).unwrap_or(member_index + 1),
                        site_id: participant.lane.clone(),
                    }),
                    _ => None,
                };
                let avatar_key = format!(
                    "{}@{}",
                    participant.lane,
                    instance.map(|i| i.ordinal).unwrap_or(member_index + 1)
                );
                if lane_class == LaneClass::Agent && !avatar_indexes.contains_key(&avatar_key) {
                    let next = avatar_indexes.len();
                    avatar_indexes.insert(avatar_key.clone(), next);
                }
                let asking = instance
                    .map(|i| {
                        asking_instances.contains(&format!("{}@{}", participant.lane, i.ordinal))
                    })
                    .unwrap_or(false);
                TimelinePill {
                    key: participant.id.clone(),
                    lane_class: lane_class,
                    lane,
                    runtime_name: instance.and_then(|i| i.name.clone()),
                    avatar_index: if lane_class == LaneClass::Agent {
                        avatar_indexes.get(&avatar_key).copied()
                    } else {
                        None
                    },
                    status,
                    instance: instance.map(|i| PillInstance {
                        site_id: participant.lane.clone(),
                        ordinal: i.ordinal,
                        session_id: session_id.clone(),
                    }),
                    slot,
                    workspace: match run {
                        Some(_) if lane_class == LaneClass::Workspace => Some(PillWorkspace {
                            phase_id: phase.id.clone(),
                        }),
                        _ => None,
                    },
                    step_ids: participant.steps.clone(),
                    asking,
                }
            })
            .collect();
        stations.push(TimelineStation {
            id: phase.id.clone(),
            naming: PhaseNaming {
                id: phase.id.clone(),
                name: phase.name.clone(),
            },
            status: station_status(
                run,
                collapse_statuses(
                    &member_steps.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                    Some(&overlay.statuses),
                ),
                is_current_phase(run, phase.name.as_deref()),
                observed.entered,
            ),
            visited: observed.visited,
            rounds: observed.rounds,
            on_loop: on_loop.contains(&i),
            track: track_of(&folded, i as i64),
            fraction: if observed.observed == 0 {
                None
            } else {
                Some(StationFraction {
                    observed: observed.observed,
                    settled: observed.settled,
                })
            },
            unlisted,
            pills,
            typing: false,
        });
    }

    let visited = |i: usize| -> bool {
        stations
            .get(i)
            .map(|s| s.visited)
            .unwrap_or(false)
    };
    // 多个正在运行的站取**最右**那个（真源 :361-368 从右往左扫）。
    let mut running_index: Option<usize> = None;
    for i in (0..stations.len()).rev() {
        if stations[i].status == Some(StepRunStatus::Running) {
            running_index = Some(i);
            break;
        }
    }

    let mut rails: Vec<TimelineRail> = fold
        .rails
        .iter()
        .map(|rail: &RailSpec| TimelineRail {
            from: rail.from as usize,
            to: rail.to as usize,
            ink: if visited(rail.from as usize) && visited(rail.to as usize) {
                TimelineInk::Strong
            } else {
                TimelineInk::Faint
            },
            kind: rail.kind,
        })
        .collect();
    let lanes = assign_air_lanes(&arc_pairs);
    let mut arcs: Vec<TimelineArc> = arc_pairs
        .iter()
        .enumerate()
        .map(|(at, arc)| {
            let back = arc.to < arc.from;
            // ★真源 :378-381 —— 回边的墨看的是**对站的轮次**（再入一次才算走过一遍），
            // 前向边看两端是否都到过。
            let strong = if back {
                visited(arc.from as usize)
                    && stations
                        .get(arc.to as usize)
                        .map(|s| s.rounds)
                        .unwrap_or(0)
                        >= 2
            } else {
                visited(arc.from as usize) && visited(arc.to as usize)
            };
            TimelineArc {
                air: arc.air,
                from: arc.from as usize,
                ink: if strong { TimelineInk::Strong } else { TimelineInk::Faint },
                lane: lanes.get(at).copied().unwrap_or(0),
                to: arc.to as usize,
            }
        })
        .collect();

    // 行进边：从上一个已结算的阶段进入正在运行的阶段的那一条。投影没有时间戳，
    // 所以按三条规则取最诚实的一条：再入的回边 > 进入该站的轨道段 > 任一落在该站的弧。
    // 每个正在运行的站各走一遍——带里两条轨道可以同时在跑，它们各自的分叉都该亮。
    for r in 0..stations.len() {
        if stations[r].status != Some(StepRunStatus::Running) {
            continue;
        }
        let entry = band_of(&folded, r as i64).map(|b| b.from as usize).unwrap_or(r);
        let reentry = arcs.iter_mut().find(|arc| {
            arc.to == entry && arc.from > arc.to && visited(arc.from) && stations[r].rounds >= 2
        });
        if let Some(arc) = reentry {
            arc.ink = TimelineInk::March;
            continue;
        }
        // 双线段不是控制流走的路，它只说「这两站并行」，永不行进。
        let inbound: Vec<usize> = rails
            .iter()
            .enumerate()
            .filter(|(_, rail)| {
                rail.to == r && rail.kind != Some(TimelineRailKind::Twin) && visited(rail.from)
            })
            .map(|(i, _)| i)
            .collect();
        if !inbound.is_empty() {
            for i in inbound {
                rails[i].ink = TimelineInk::March;
            }
            continue;
        }
        if let Some(arc) = arcs.iter_mut().find(|arc| arc.to == r && visited(arc.from)) {
            arc.ink = TimelineInk::March;
        }
    }

    let rail_ink = |from: usize, to: usize| -> TimelineInk {
        rails
            .iter()
            .find(|rail| {
                rail.from == from && rail.to == to && rail.kind != Some(TimelineRailKind::Twin)
            })
            .map(|rail| rail.ink)
            .unwrap_or(TimelineInk::Faint)
    };
    let bands: Vec<TimelineBand> = fold
        .bands
        .iter()
        .map(|band| TimelineBand {
            from: band.from as usize,
            to: band.to as usize,
            pred: band.pred.map(|p| p as usize),
            join: band.join.map(|j| j as usize),
            tracks: band
                .tracks
                .iter()
                .map(|members| {
                    // ★真源 :426-427 —— 空轨道不可能出现（带内至少两个成员），
                    // 但真源用 `!` 断言了；这里退回首站= 末站而不是 panic。
                    let head = members.first().copied().unwrap_or(band.from) as usize;
                    let tail = members.last().copied().unwrap_or(band.to) as usize;
                    TimelineTrack {
                        entry: match band.pred {
                            None => {
                                if visited(head) {
                                    TimelineInk::Strong
                                } else {
                                    TimelineInk::Faint
                                }
                            }
                            Some(pred) => rail_ink(pred as usize, head),
                        },
                        exit: match band.join {
                            None => {
                                if visited(tail) {
                                    TimelineInk::Strong
                                } else {
                                    TimelineInk::Faint
                                }
                            }
                            Some(join) => rail_ink(tail, join as usize),
                        },
                        stations: members.iter().map(|m| *m as usize).collect(),
                    }
                })
                .collect(),
        })
        .collect();

    WorkflowTimelineModel {
        arcs,
        bands,
        live: run.is_some(),
        rails,
        running_index,
        stations,
        stalled: run.map(|r| r.stalled == Some(true)).unwrap_or(false),
        draft: None,
    }
}

/// `pillActivity`（真源 :454-477）：一枚药丸「正在做什么」。
///
/// 优先正在跑的 step 的 label，其次最后一个已结算的，再次第一个。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillActivity {
    pub label: String,
    pub asks: i64,
    pub reads: i64,
}

pub fn pill_activity(
    graph: &WorkflowCausalityGraphData,
    run: Option<&WorkflowRunState>,
    pill: &TimelinePill,
) -> PillActivity {
    let steps_by_id: HashMap<&str, &crate::components::workflow_graph::types::WorkflowStepData> =
        graph.steps.iter().map(|s| (s.id.as_str(), s)).collect();
    let overlay_statuses = run.map(|r| workflow_run_overlay(Some(r), graph).statuses);
    let mut asks = 0;
    let mut reads = 0;
    let mut running: Option<String> = None;
    let mut done: Option<String> = None;
    let mut first: Option<String> = None;
    for id in &pill.step_ids {
        let Some(step) = steps_by_id.get(id.as_str()) else {
            continue;
        };
        if step.kind == "world-read" {
            reads += 1;
        } else {
            asks += 1;
        }
        if first.is_none() {
            first = Some(step.label.clone());
        }
        match overlay_statuses
            .as_ref()
            .and_then(|s| s.get(id.as_str()).copied())
        {
            Some(StepRunStatus::Running) => {
                if running.is_none() {
                    running = Some(step.label.clone());
                }
            }
            Some(StepRunStatus::Done) | Some(StepRunStatus::Failed) => {
                done = Some(step.label.clone());
            }
            _ => {}
        }
    }
    PillActivity {
        label: running.or(done).or(first).unwrap_or_default(),
        asks,
        reads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_from_str(raw: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(raw).unwrap()
    }

    fn run_from_str(raw: &str) -> WorkflowRunState {
        serde_json::from_str(raw).unwrap()
    }

    /// 两个具名阶段 + 一条前向边。
    fn two_phase_graph() -> WorkflowCausalityGraphData {
        graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "问A", "lane": "l1", "phase": "p1"},
                {"id": "s2", "kind": "ask", "label": "问B", "lane": "l1", "phase": "p2"}
            ], "lanes": [{"id": "l1", "name": "研究员"}], "participants": [
                {"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]},
                {"id": "b", "phase": "p2", "lane": "l1", "steps": ["s2"]}
            ], "handoffs": [{"from": "a", "to": "b"}],
            "phases": [{"id": "p1", "name": "阶段一"}, {"id": "p2", "name": "阶段二"}],
            "phaseEdges": [{"from": "p1", "to": "p2"}]}"#,
        )
    }

    // ── stationStatus ──

    fn run_status(status: &str) -> WorkflowRunState {
        run_from_str(&format!(r#"{{"runId": "r", "status": "{status}", "actors": [], "nodes": []}}"#))
    }

    #[test]
    fn station_status_absent_without_run() {
        // 真源 :188 —— 无 run 直接 None（静态渲染不点灯）。
        assert_eq!(station_status(None, Some(StepRunStatus::Done), true, true), None);
    }

    #[test]
    fn station_status_node_facts_win() {
        // ★真源 :189 —— running / failed 是硬事实，先说话。
        let run = run_status("running");
        assert_eq!(
            station_status(Some(&run), Some(StepRunStatus::Running), false, false),
            Some(StepRunStatus::Running)
        );
        assert_eq!(
            station_status(Some(&run), Some(StepRunStatus::Failed), true, true),
            Some(StepRunStatus::Failed),
            "failed 胜过「当前阶段且在跑」"
        );
    }

    #[test]
    fn current_phase_is_running_while_run_lives() {
        // ★真源 :190-191 —— 第一个 ask 派发之前、最后一个结算之后，
        // 控制流都在这一站。
        for status in ["running", "pending"] {
            let run = run_status(status);
            assert_eq!(
                station_status(Some(&run), None, true, false),
                Some(StepRunStatus::Running),
                "{status} 是活的"
            );
        }
    }

    #[test]
    fn current_phase_settles_with_run_terminal_state() {
        // ★真源 :192-194 —— 失败发生在这一站（不管它有没有节点）；
        // cancelled 与节点的画法一致，同样是 failed。
        let done = run_status("completed");
        assert_eq!(
            station_status(Some(&done), None, true, false),
            Some(StepRunStatus::Done)
        );
        for status in ["errored", "stopped"] {
            let run = run_status(status);
            assert_eq!(
                station_status(Some(&run), None, true, false),
                Some(StepRunStatus::Failed),
                "{status} → failed"
            );
        }
    }

    #[test]
    fn station_without_nodes_relies_on_entry_record() {
        // ★真源 :194-196 —— 折叠对自己缺席的站点只说 undefined，
        // 「解缺席」在这里：到过 = done，没到 = pending。
        let run = run_status("completed");
        assert_eq!(
            station_status(Some(&run), None, false, true),
            Some(StepRunStatus::Done),
            "进入记录点灯"
        );
        assert_eq!(
            station_status(Some(&run), None, false, false),
            Some(StepRunStatus::Pending),
            "没观察到也没进入"
        );
        // 折叠给了值就用它。
        assert_eq!(
            station_status(Some(&run), Some(StepRunStatus::Done), false, false),
            Some(StepRunStatus::Done)
        );
    }

    // ── isCurrentPhase ──

    #[test]
    fn current_phase_matches_by_name() {
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [],
                "currentPhase": "阶段二"}"#,
        );
        assert!(is_current_phase(Some(&run), Some("阶段二")));
        assert!(!is_current_phase(Some(&run), Some("阶段一")));
        // 无名阶段与有戳的 currentPhase 关联不上（关联需要两个名字）。
        assert!(!is_current_phase(Some(&run), None));
        assert!(!is_current_phase(None, Some("阶段二")));
    }

    // ── buildWorkflowTimeline：骨架 ──

    #[test]
    fn static_model_without_run() {
        let graph = two_phase_graph();
        let model = build_workflow_timeline(&graph, None);
        assert!(!model.live, "无 run → live=false");
        assert_eq!(model.stations.len(), 2);
        assert!(model.stations.iter().all(|s| s.status.is_none()), "静态不点灯");
        assert!(model.running_index.is_none());
        assert!(!model.stalled);
        // 相邻且有边 → 一段轨道。
        assert_eq!(model.rails.len(), 1);
        assert_eq!(model.rails[0].from, 0);
        assert_eq!(model.rails[0].to, 1);
        assert_eq!(model.rails[0].ink, TimelineInk::Faint, "没到过 → faint");
        assert!(model.rails[0].kind.is_none(), "带外普通段");
        assert!(model.arcs.is_empty());
        assert!(model.bands.is_empty(), "没有 alongside → 没有带");
    }

    #[test]
    fn implicit_phase_gives_a_single_station() {
        // 无标记脚本 → 合成一个隐式阶段，站表只有一项。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [{"id": "a", "phase": "unphased", "lane": "l1", "steps": ["s1"]}]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert_eq!(model.stations.len(), 1);
        assert_eq!(model.stations[0].id, crate::components::workflow_graph::types::IMPLICIT_PHASE_ID);
    }

    #[test]
    fn back_edge_marks_both_ends_on_loop() {
        // ★真源 :260-269 —— 回边的两端上环；端点在带里时整条带都上环。
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "问A", "lane": "l1", "phase": "p1"},
                {"id": "s2", "kind": "ask", "label": "问B", "lane": "l1", "phase": "p2"}
            ], "lanes": [], "participants": [
                {"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]},
                {"id": "b", "phase": "p2", "lane": "l1", "steps": ["s2"]}
            ], "handoffs": [],
            "phases": [{"id": "p1", "name": "一"}, {"id": "p2", "name": "二"}],
            "phaseEdges": [{"from": "p2", "to": "p1"}]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert!(model.stations[0].on_loop, "回边目标");
        assert!(model.stations[1].on_loop, "回边源");
        assert!(model.arcs.len() >= 1, "非相邻/反向边成弧");
    }

    #[test]
    fn self_loops_and_unknown_phases_drop_out() {
        // 真源 :238-245 —— 自环与指向未列出阶段的边在这一粒度上没有话说。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                "phases": [{"id": "p1", "name": "一"}],
                "phaseEdges": [
                  {"from": "p1", "to": "p1"},
                  {"from": "p1", "to": "ghost"}
                ]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert!(model.rails.is_empty());
        assert!(model.arcs.is_empty());
    }

    #[test]
    fn duplicate_edges_are_deduped_by_index() {
        // 真源 :242 —— 按下标去重。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [], "handoffs": [],
                "phases": [{"id": "p1", "name": "一"}, {"id": "p2", "name": "二"}],
                "phaseEdges": [
                  {"from": "p1", "to": "p2"},
                  {"from": "p1", "to": "p2"}
                ]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert_eq!(model.rails.len(), 1, "重复边只算一条");
    }

    // ── 墨迹 ──

    fn two_nodes_run() -> WorkflowRunState {
        // ★actor 的 `siteId` 是**车道 id**（l1），节点的 `siteId` 才是站点 id（s1/s2）
        // ——两者不同源正是 run-status.ts 用 `source ?? id` 收窄的原因。
        run_from_str(
            r#"{"runId": "r", "status": "completed", "actors": [
                {"siteId": "l1", "ordinal": 0, "status": "completed", "phaseName": "阶段一"}
              ], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段一"},
                {"siteId": "s2", "ordinal": 0, "phase": "settled", "outcome": "ok",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段二"}
              ], "phases": [{"name": "阶段一", "rounds": 1}, {"name": "阶段二", "rounds": 1}],
              "currentPhase": "阶段二"}"#,
        )
    }

    #[test]
    fn visited_rail_is_strong() {
        // 真源 :372 —— 两端都到过 → strong。
        let model = build_workflow_timeline(&two_phase_graph(), Some(&two_nodes_run()));
        assert!(model.live);
        assert_eq!(model.rails[0].ink, TimelineInk::Strong);
        assert!(model.stations.iter().all(|s| s.visited));
    }

    #[test]
    fn fraction_absent_when_nothing_observed() {
        // ★真源 :100 —— 一个节点都没观察到时缺席。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1", "phase": "p1"}],
                "lanes": [], "participants": [{"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]}],
                "handoffs": [], "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let model = build_workflow_timeline(&graph, Some(&run_status("running")));
        assert!(model.stations[0].fraction.is_none());
        // 没有观测也没有进入记录 → pending（不是「零分之零」）。
        assert_eq!(model.stations[0].status, Some(StepRunStatus::Pending));
    }

    #[test]
    fn fraction_counts_observed_and_settled() {
        let model = build_workflow_timeline(&two_phase_graph(), Some(&two_nodes_run()));
        let f = model.stations[0].fraction.unwrap();
        assert_eq!(f.observed, 1);
        assert_eq!(f.settled, 1);
    }

    #[test]
    fn running_index_takes_the_rightmost_running_station() {
        // 真源 :362-368 —— 从右往左扫，取最右那个。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "executing",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段一"},
                {"siteId": "s2", "ordinal": 0, "phase": "executing",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段二"}
              ], "currentPhase": "阶段二"}"#,
        );
        let model = build_workflow_timeline(&two_phase_graph(), Some(&run));
        assert_eq!(model.stations[0].status, Some(StepRunStatus::Running));
        assert_eq!(model.stations[1].status, Some(StepRunStatus::Running));
        assert_eq!(model.running_index, Some(1), "取最右");
    }

    #[test]
    fn marching_ink_lights_the_inbound_rail() {
        // ★真源 :391-415 —— 从上一个已结算的阶段进入正在运行的阶段的那一条。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [], "nodes": [
                {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段一"},
                {"siteId": "s2", "ordinal": 0, "phase": "executing",
                 "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段二"}
              ], "currentPhase": "阶段二"}"#,
        );
        let model = build_workflow_timeline(&two_phase_graph(), Some(&run));
        assert_eq!(model.running_index, Some(1));
        assert_eq!(model.rails[0].ink, TimelineInk::March, "进入该站的轨道段行进");
    }

    #[test]
    fn stalled_flag_comes_from_run_only() {
        // ★真源 :160-165 —— 停滞只属于 run，不染任何一枚药丸。
        let mut run = two_nodes_run();
        assert!(!build_workflow_timeline(&two_phase_graph(), Some(&run)).stalled);
        run.stalled = Some(true);
        let model = build_workflow_timeline(&two_phase_graph(), Some(&run));
        assert!(model.stalled);
        assert!(
            model.stations.iter().all(|s| s.pills.iter().all(|p| p.status != Some(StepRunStatus::Failed))),
            "停滞不落到药丸上"
        );
    }

    // ── 药丸 ──

    #[test]
    fn pill_has_slot_and_avatar_for_agent_lane_in_live_run() {
        // ★真源 :300-306 —— 只有活的 run 里的 agent 车道才有 slot，
        // 还没启动的子代理也能开一个占位的 transcript tab。
        let model = build_workflow_timeline(&two_phase_graph(), Some(&two_nodes_run()));
        let pill = &model.stations[0].pills[0];
        assert_eq!(pill.lane_class, LaneClass::Agent);
        assert_eq!(pill.slot, Some(PillSlot { site_id: "l1".into(), ordinal: 0 }));
        assert_eq!(pill.avatar_index, Some(0));
        assert!(pill.workspace.is_none(), "agent 车道没有 workspace 抓手");
        assert_eq!(pill.instance.as_ref().unwrap().site_id, "l1");
        assert_eq!(pill.instance.as_ref().unwrap().ordinal, 0);
    }

    #[test]
    fn pill_avatar_is_reused_across_phases_for_same_agent() {
        // ★真源 :271 —— 名称哈希会碰撞；以实例身份分配连续编号，
        // 同一代理跨阶段保持同一头像。
        let model = build_workflow_timeline(&two_phase_graph(), Some(&two_nodes_run()));
        assert_eq!(model.stations[0].pills[0].avatar_index, Some(0));
        assert_eq!(
            model.stations[1].pills[0].avatar_index,
            Some(0),
            "同一个 actor 跨阶段同一头像"
        );
    }

    #[test]
    fn synthetic_lane_pill_gets_workspace_handle_not_slot() {
        // ★真源 :74-79 —— workspace 药丸没有实例、没有会话，
        // 能开的是整个 run 的脚本 transcript。与 `slot` 互斥。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "r1", "kind": "world-read", "label": "读", "lane": "workspace", "phase": "p1"}],
                "lanes": [], "participants": [{"id": "w", "phase": "p1", "lane": "workspace", "steps": ["r1"]}],
                "handoffs": [], "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let model = build_workflow_timeline(&graph, Some(&run_status("running")));
        let pill = &model.stations[0].pills[0];
        assert_eq!(pill.lane_class, LaneClass::Workspace);
        assert!(pill.slot.is_none(), "互斥");
        assert_eq!(
            pill.workspace,
            Some(PillWorkspace { phase_id: "p1".into() }),
            "脚本 transcript 抓手"
        );
        assert!(pill.avatar_index.is_none(), "合成车道没有头像");
    }

    #[test]
    fn workspace_handle_absent_without_run() {
        // 真源 :328 —— 没有 run 就没有可开的东西。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "r1", "kind": "world-read", "label": "读", "lane": "workspace", "phase": "p1"}],
                "lanes": [], "participants": [{"id": "w", "phase": "p1", "lane": "workspace", "steps": ["r1"]}],
                "handoffs": [], "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert!(model.stations[0].pills[0].workspace.is_none());
        assert!(model.stations[0].pills[0].slot.is_none());
    }

    #[test]
    fn pill_status_is_none_without_run_and_pending_with_run() {
        // ★真源 :289-295 —— 无 run 的静态药丸是 undefined；
        // 折叠为空（有 run）时是 pending：空心是诚实的。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1", "phase": "p1"}],
                "lanes": [], "participants": [{"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]}],
                "handoffs": [], "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let static_model = build_workflow_timeline(&graph, None);
        assert!(static_model.stations[0].pills[0].status.is_none());
        let live_model = build_workflow_timeline(&graph, Some(&run_status("running")));
        assert_eq!(live_model.stations[0].pills[0].status, Some(StepRunStatus::Pending));
    }

    #[test]
    fn many_cards_split_into_one_pill_per_instance() {
        // 真源 :283 —— 药丸建在实时视图拆开的图上。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1", "phase": "p1"}],
                "lanes": [], "handoffs": [],
                "participants": [{"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"], "many": true}],
                "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [
                  {"siteId": "l1", "ordinal": 0, "status": "running", "phaseName": "一"},
                  {"siteId": "l1", "ordinal": 1, "status": "running", "phaseName": "一"}
                ],
                "nodes": [
                  {"siteId": "s1", "ordinal": 0, "phase": "executing",
                   "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "一"},
                  {"siteId": "s1", "ordinal": 1, "phase": "settled", "outcome": "ok",
                   "actorSiteId": "l1", "actorOrdinal": 1, "phaseName": "一"}
                ]}"#,
        );
        let model = build_workflow_timeline(&graph, Some(&run));
        let keys: Vec<&str> = model.stations[0]
            .pills
            .iter()
            .map(|p| p.key.as_str())
            .collect();
        assert_eq!(keys, vec!["a@0", "a@1"], "实例卡各出一枚药丸");
        assert_eq!(model.stations[0].pills[0].status, Some(StepRunStatus::Running));
        assert_eq!(model.stations[0].pills[1].status, Some(StepRunStatus::Done));
        // 头像按实例身份分配，两个实例两个号。
        assert_eq!(model.stations[0].pills[0].avatar_index, Some(0));
        assert_eq!(model.stations[0].pills[1].avatar_index, Some(1));
    }

    #[test]
    fn pending_question_marks_the_asking_pill() {
        // 真源 :332 —— 该实例有待答的升级问题。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "waiting",
                            "phaseName": "阶段一"}],
                "nodes": [{"siteId": "s1", "ordinal": 0, "phase": "waiting",
                           "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段一"}],
                "pendingQuestions": [
                  {"qid": "q1", "actorSiteId": "l1", "actorOrdinal": 0, "question": "?"}
                ]}"#,
        );
        let model = build_workflow_timeline(&two_phase_graph(), Some(&run));
        assert!(model.stations[0].pills[0].asking, "阶段一那枚药丸有待答问题");
        assert!(!model.stations[1].pills[0].asking, "别的药丸不受影响");
    }

    #[test]
    fn question_without_actor_binding_is_ignored() {
        // 真源 :226-232 —— 两项都要有才算绑定到某个实例。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "waiting",
                            "phaseName": "阶段一"}],
                "nodes": [],
                "pendingQuestions": [{"qid": "q1", "question": "?"}]}"#,
        );
        let model = build_workflow_timeline(&two_phase_graph(), Some(&run));
        assert!(!model.stations[0].pills[0].asking);
    }

    #[test]
    fn member_card_pill_uses_index_plus_one_as_future_slot() {
        // ★真源 :70-71 —— 没有实例就是它**将来**的序号：成员卡 `member.index + 1`。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1", "phase": "p1"}],
                "lanes": [], "handoffs": [],
                "participants": [{"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"],
                                  "member": {"index": 1, "of": 3}}],
                "phases": [{"id": "p1", "name": "一"}]}"#,
        );
        let model = build_workflow_timeline(&graph, Some(&run_status("running")));
        let pill = &model.stations[0].pills[0];
        assert_eq!(pill.slot, Some(PillSlot { site_id: "l1".into(), ordinal: 2 }));
        assert!(pill.instance.is_none(), "实例未出现 → 没有 instance");
        assert_eq!(pill.status, Some(StepRunStatus::Pending));
    }

    // ── 带 ──

    #[test]
    fn alongside_forms_a_band_with_tracks() {
        // 真源 :139-148 —— 并行阶段折成带，轨道 0 是主线。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [],
                "participants": [
                  {"id": "a", "phase": "p1", "lane": "l1", "steps": []},
                  {"id": "b", "phase": "p2", "lane": "l2", "steps": []},
                  {"id": "c", "phase": "p3", "lane": "l1", "steps": []}
                ],
                "phases": [
                  {"id": "p1", "name": "一", "alongside": ["p2"]},
                  {"id": "p2", "name": "二"},
                  {"id": "p3", "name": "三"}
                ],
                "phaseEdges": [{"from": "p1", "to": "p3"}]}"#,
        );
        let model = build_workflow_timeline(&graph, None);
        assert_eq!(model.bands.len(), 1);
        let band = &model.bands[0];
        assert_eq!(band.from, 0);
        assert_eq!(band.to, 1);
        assert_eq!(band.tracks.len(), 2, "两站并行 → 两条轨道");
        assert_eq!(band.tracks[0].stations, vec![0]);
        assert_eq!(band.tracks[1].stations, vec![1]);
        // 带内成员各自在自己那一轨，带外一律 0（主线）。
        assert_eq!(model.stations[0].track, 0);
        assert_eq!(model.stations[1].track, 1);
        assert_eq!(model.stations[2].track, 0);
    }

    // ── pillActivity ──

    #[test]
    fn pill_activity_prefers_running_label() {
        // 真源 :454-477 —— 优先正在跑的 step 的 label。
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "第一个", "lane": "l1"},
                {"id": "s2", "kind": "ask", "label": "正在跑", "lane": "l1"},
                {"id": "s3", "kind": "world-read", "label": "读文件", "lane": "workspace"}
            ], "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running"},
                           {"siteId": "workspace", "ordinal": 0, "status": "completed"}],
                "nodes": [
                  {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok",
                   "actorSiteId": "l1", "actorOrdinal": 0},
                  {"siteId": "s2", "ordinal": 0, "phase": "executing",
                   "actorSiteId": "l1", "actorOrdinal": 0},
                  {"siteId": "s3", "ordinal": 0, "phase": "settled", "outcome": "ok",
                   "actorSiteId": "workspace", "actorOrdinal": 0}
                ]}"#,
        );
        let pill = TimelinePill {
            key: "a".into(),
            lane: LaneRef {
                id: "l1".into(),
                naming: crate::components::workflow_graph::lane_name::LaneNaming {
                    lane_class: Some(LaneClass::Agent),
                    name: None,
                    name_pattern: None,
                    anonymous_index: None,
                },
            },
            lane_class: LaneClass::Agent,
            runtime_name: None,
            avatar_index: None,
            status: None,
            instance: None,
            slot: None,
            workspace: None,
            step_ids: vec!["s1".into(), "s2".into(), "s3".into()],
            asking: false,
        };
        let activity = pill_activity(&graph, Some(&run), &pill);
        assert_eq!(activity.label, "正在跑");
        assert_eq!(activity.asks, 2);
        assert_eq!(activity.reads, 1);
    }

    #[test]
    fn pill_activity_falls_back_to_last_settled_then_first() {
        let graph = graph_from_str(
            r#"{"steps": [
                {"id": "s1", "kind": "ask", "label": "第一个", "lane": "l1"},
                {"id": "s2", "kind": "ask", "label": "最后结算", "lane": "l1"}
            ], "lanes": [], "participants": [], "handoffs": []}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "completed",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "completed"},
                           {"siteId": "l1", "ordinal": 1, "status": "completed"}],
                "nodes": [
                  {"siteId": "s1", "ordinal": 0, "phase": "settled", "outcome": "ok",
                   "actorSiteId": "l1", "actorOrdinal": 0},
                  {"siteId": "s2", "ordinal": 1, "phase": "settled", "outcome": "failed",
                   "actorSiteId": "l1", "actorOrdinal": 1}
                ]}"#,
        );
        let base = TimelinePill {
            key: "a".into(),
            lane: LaneRef {
                id: "l1".into(),
                naming: crate::components::workflow_graph::lane_name::LaneNaming {
                    lane_class: Some(LaneClass::Agent),
                    name: None,
                    name_pattern: None,
                    anonymous_index: None,
                },
            },
            lane_class: LaneClass::Agent,
            runtime_name: None,
            avatar_index: None,
            status: None,
            instance: None,
            slot: None,
            workspace: None,
            step_ids: vec!["s1".into(), "s2".into()],
            asking: false,
        };
        assert_eq!(
            pill_activity(&graph, Some(&run), &base).label,
            "最后结算",
            "failed 也算已结算，取最后一个"
        );
        // 无 run → 走第一个。
        assert_eq!(pill_activity(&graph, None, &base).label, "第一个");
        // 一个 step 都没有 → 空串。
        let empty = TimelinePill {
            step_ids: Vec::new(),
            ..base
        };
        let activity = pill_activity(&graph, None, &empty);
        assert_eq!(activity.label, "");
        assert_eq!(activity.asks, 0);
        assert_eq!(activity.reads, 0);
    }

    #[test]
    fn ink_and_rail_kind_as_str() {
        assert_eq!(TimelineInk::March.as_str(), "march");
        assert_eq!(TimelineInk::Faint.as_str(), "faint");
        assert_eq!(TimelineInk::Strong.as_str(), "strong");
    }
}