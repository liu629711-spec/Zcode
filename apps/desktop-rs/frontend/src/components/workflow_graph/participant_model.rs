//! 1:1 翻译 `packages/ui/src/components/workflow-graph/participant-model.ts`
//! （369 行）。
//!
//! ★真源 :14-24 注释——参与者层的纯选择器。载荷的 `participants` 已经是
//! 分析器排好的**交接序**（第一张是开局者），`handoffs` 已经归约过；
//! 这里只做分桶、状态折叠、计数，以及 UI 自己的两件事：
//! - 无标记脚本合成一个隐式阶段（`with_implicit_phase`）；
//! - 运行中把 `many` 卡按真实实例拆开、成员卡按 ordinal 收窄、
//!   单卡绑定它唯一的实例或在多实例时同样拆开（`live_participant_view`）。
//!
//! 无 React、无 DOM：投影层与组件都消费它，测试直接调。

use std::collections::{HashMap, HashSet};

use super::instance_phases::{PhaseBinder, phase_binder};
use super::run_state::{WorkflowRunActor, WorkflowRunNode, WorkflowRunState};
use super::run_status::{aggregate_run_statuses, collapse_statuses, status_of_run_node};
use super::types::{
    IMPLICIT_PHASE_ID, StepRunStatus, StepStatusTable, WorkflowCausalityGraphData,
    WorkflowHandoffData, WorkflowParticipantData, WorkflowPhaseData,
};

pub use super::run_status::has_phase_vocabulary;

/// `withImplicitPhase`（真源 :34-47）。
///
/// 无 `phase()` 标记的脚本 → 一个隐式模块：阶段表只有 `workflow`
/// （无 name，UI 本地化为「Workflow」），没有阶段边，
/// 控制流到达正常完成**当且仅当**脚本有返回物；
/// 每个 step 与每张卡归入它（分析器给它们的 `unphased` 只是
/// 「没有阶段」的占位，**卡 id 不变**）。
///
/// 有词汇表的图**原样返回**（引用相等，memo 友好）。
pub fn with_implicit_phase(graph: &WorkflowCausalityGraphData) -> WorkflowCausalityGraphData {
    if has_phase_vocabulary(graph) {
        return graph.clone();
    }
    let mut out = graph.clone();
    // 真源 :39 —— exits：有返回物才有一个可正常完成的阶段。
    out.exits = Some(
        if graph.sink.as_ref().is_some_and(|s| !s.is_empty()) {
            vec![IMPLICIT_PHASE_ID.to_string()]
        } else {
            Vec::new()
        },
    );
    out.participants = graph
        .participants
        .iter()
        .map(|p| {
            let mut p = p.clone();
            p.phase = IMPLICIT_PHASE_ID.to_string();
            p
        })
        .collect();
    out.phase_edges = Some(Vec::new());
    out.phases = Some(vec![WorkflowPhaseData {
        id: IMPLICIT_PHASE_ID.to_string(),
        name: None,
        line: None,
        column: None,
        alongside: None,
    }]);
    out.steps = graph
        .steps
        .iter()
        .map(|s| {
            let mut s = s.clone();
            s.phase = Some(IMPLICIT_PHASE_ID.to_string());
            s
        })
        .collect();
    out
}

/// `participantsOfPhase`（真源 :50-56）：一个阶段的参与者，保持载荷顺序（= 交接序）。
pub fn participants_of_phase(
    graph: &WorkflowCausalityGraphData,
    phase_id: &str,
) -> Vec<WorkflowParticipantData> {
    graph
        .participants
        .iter()
        .filter(|p| p.phase == phase_id)
        .cloned()
        .collect()
}

/// `participantById`（真源 :58-63）。
pub fn participant_by_id<'a>(
    graph: &'a WorkflowCausalityGraphData,
    id: &str,
) -> Option<&'a WorkflowParticipantData> {
    graph.participants.iter().find(|p| p.id == id)
}

/// `handoffsWithin`（真源 :65-71）：两端都在给定集合里的交接边（阶段内部的边）。
pub fn handoffs_within(
    graph: &WorkflowCausalityGraphData,
    ids: &std::collections::HashSet<String>,
) -> Vec<WorkflowHandoffData> {
    graph
        .handoffs
        .iter()
        .filter(|e| ids.contains(&e.from) && ids.contains(&e.to))
        .cloned()
        .collect()
}

/// `handoffsAround`（真源 :73-85）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffsAround {
    pub incoming: Vec<WorkflowHandoffData>,
    pub outgoing: Vec<WorkflowHandoffData>,
}

pub fn handoffs_around(
    graph: &WorkflowCausalityGraphData,
    id: &str,
) -> HandoffsAround {
    HandoffsAround {
        incoming: graph.handoffs.iter().filter(|e| e.to == id).cloned().collect(),
        outgoing: graph.handoffs.iter().filter(|e| e.from == id).cloned().collect(),
    }
}

/// `ParticipantCounts`（真源 :87-91）：卡片次行的计数素材。
///
/// ★真源 :87 ——ask 数与工作区读取数（**一张卡只会有其中一种非零**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParticipantCounts {
    pub asks: i64,
    pub reads: i64,
}

/// `participantCounts`（真源 :93-105）。
pub fn participant_counts(
    graph: &WorkflowCausalityGraphData,
    participant: &WorkflowParticipantData,
) -> ParticipantCounts {
    let kinds: HashMap<&str, &str> = graph
        .steps
        .iter()
        .map(|s| (s.id.as_str(), s.kind.as_str()))
        .collect();
    let mut asks = 0;
    let mut reads = 0;
    for id in &participant.steps {
        if kinds.get(id.as_str()) == Some(&"world-read") {
            reads += 1;
        } else {
            asks += 1;
        }
    }
    ParticipantCounts { asks, reads }
}

/// `participantRepeats`（真源 :107-118）。
///
/// ★真源 :107 注释——任一 step 带 `repeat` ⇒ 卡片显示重复图元
/// （自环在参与者粒度上消失，这是它留下的线索）。
pub fn participant_repeats(
    graph: &WorkflowCausalityGraphData,
    participant: &WorkflowParticipantData,
) -> bool {
    let repeating: std::collections::HashSet<&str> = graph
        .steps
        .iter()
        .filter(|s| s.repeat.is_some())
        .map(|s| s.id.as_str())
        .collect();
    participant.steps.iter().any(|id| repeating.contains(id.as_str()))
}

/// `participantStatus`（真源 :128-134）。
///
/// ★真源 :121-122 注释——实时视图给出的按实例收窄的覆盖值优先
/// （成员卡 / 拆分出的实例卡），否则由它的 step 折叠。
pub fn participant_status(
    participant: &WorkflowParticipantData,
    statuses: Option<&StepStatusTable>,
    participant_statuses: Option<&HashMap<String, StepRunStatus>>,
) -> Option<StepRunStatus> {
    participant_statuses
        .and_then(|m| m.get(&participant.id).copied())
        .or_else(|| collapse_statuses(&participant.steps, statuses))
}

/// `participantAlsoIn`（真源 :136-149）。
///
/// 同一条车道还出现在哪些其他阶段（检视器「also in」一行）。
pub fn participant_also_in(
    graph: &WorkflowCausalityGraphData,
    participant: &WorkflowParticipantData,
) -> Vec<WorkflowPhaseData> {
    let phases: std::collections::HashSet<&str> = graph
        .participants
        .iter()
        .filter(|o| o.lane == participant.lane && o.phase != participant.phase)
        .map(|o| o.phase.as_str())
        .collect();
    match graph.phases.as_ref() {
        Some(ps) => ps
            .iter()
            .filter(|p| phases.contains(p.id.as_str()))
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}

/// `instanceCardId`（真源 :151-154）：拆分出的实例卡 id。
pub fn instance_card_id(participant_id: &str, ordinal: i64) -> String {
    format!("{participant_id}@{ordinal}")
}

/// `ParticipantInstance`（真源 :156-181）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantInstance {
    /// 拆分前的参与者 id（绑定时就是这张卡自己的 id）。
    pub participant: String,
    pub ordinal: i64,
    /// 名字就是引擎发出的有效名，卡面优先念它。
    pub name: Option<String>,
    /// ★真源 :172-176 ——原卡绑定而非拆分：卡 id 没变，标签规则也不变
    /// （成员卡仍 `#i`，单卡无标签）——运行时名不做视觉标记。
    /// `None` = 拆分出的实例卡，标签念 `#ordinal`。
    pub bound: bool,
}

// ---------------------------------------------------------------------------
// 实时视图（真源 :161-369）
// ---------------------------------------------------------------------------

/// `LiveParticipantView`（真源 :183-190）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveParticipantView {
    /// 参与者与交接已按实例拆分的图；无 run 时就是输入图（值相等）。
    pub graph: WorkflowCausalityGraphData,
    /// 按实例收窄后的卡片状态（成员卡、实例卡）；其余卡由 step 折叠。
    pub participant_statuses: HashMap<String, StepRunStatus>,
    /// 实例卡 id → 身份。
    pub instances: HashMap<String, ParticipantInstance>,
}

/// `boundInstance`（真源 :359-369）。
fn bound_instance(participant_id: &str, actor: &WorkflowRunActor) -> ParticipantInstance {
    ParticipantInstance {
        // ★真源 :296-297 —— 名字缺席就不带这个键（Rust 侧 None）。
        name: actor.name.clone(),
        ordinal: actor.ordinal,
        participant: participant_id.to_string(),
        bound: true,
    }
}

/// `RunIndex`（真源 :325-330）：一次视图建一遍的两张索引。
#[derive(Debug, Clone, Default)]
struct RunIndex {
    /// `actorKey` → 该实例名下的节点，保持 `run.nodes` 的顺序。
    nodes_by_actor: HashMap<String, Vec<WorkflowRunNode>>,
    /// 车道 → 该车道上的 actor，已按 ordinal 升序（认领顺序）。
    actors_by_site: HashMap<String, Vec<WorkflowRunActor>>,
}

/// `actorKey`（真源 :321-323）。
///
/// ★键用 `\0` 连接，与 shared 的 `workflow-runs-actor-status.ts` 同一条理由：
/// `siteId` 是引擎给的**任意字符串**，用 `-` 之类可打印分隔符会让
/// `("a-1", 2)` 与 `("a", "1-2")` 撞车。
fn actor_key(site_id: &str, ordinal: i64) -> String {
    format!("{site_id}\0{ordinal}")
}

/// `runIndex`（真源 :340-357）。
///
/// ★真源 :332-339 注释——没有它们，每张实例卡都要重扫一遍 `run.nodes` 才能收自己的状态
/// ——表界是 1024 个实例 × 1024 个节点，也就是每帧一百万次比较。
/// 索引之后建模按 actors + nodes 线性。
///
/// 没有 actor 的节点（world-read）与不带 ordinal 的旧载荷不进索引：原来的筛选条件
/// `node.actorSiteId === lane && node.actorOrdinal === ordinal` 对它们恒假，丢掉等价。
fn run_index(run: &WorkflowRunState) -> RunIndex {
    let mut nodes_by_actor: HashMap<String, Vec<WorkflowRunNode>> = HashMap::new();
    for node in &run.nodes {
        let (Some(site), Some(ordinal)) = (node.actor_site_id.as_deref(), node.actor_ordinal)
        else {
            continue;
        };
        nodes_by_actor
            .entry(actor_key(site, ordinal))
            .or_default()
            .push(node.clone());
    }
    let mut actors_by_site: HashMap<String, Vec<WorkflowRunActor>> = HashMap::new();
    for actor in &run.actors {
        actors_by_site
            .entry(actor.site_id.clone())
            .or_default()
            .push(actor.clone());
    }
    // 整条车道只排一次序，与原来「先筛后排」同序——排序稳定，筛选保序。
    for actors in actors_by_site.values_mut() {
        actors.sort_by_key(|a| a.ordinal);
    }
    RunIndex {
        nodes_by_actor,
        actors_by_site,
    }
}

/// `liveParticipantView`（真源 :212-315）。
///
/// ★真源 :193-211 注释——
/// - `many` 卡：该车道上每个已出现的 actor 实例各出一张卡，交接边按原卡复制到
///   每张实例卡；实例尚未出现时保留原来那一张（状态由 step 折叠）。
/// - 单卡：车道上恰有一个实例 → 原卡绑定它（id 不变、状态仍由 step 折叠，
///   只是名字有了）；两个以上 → 与 `many` 同一条拆分路径——运行时基数胜过静态基数。
/// - 成员卡（字面量基数展开）：第 i 个成员对应该车道上按 ordinal 排序的第 i 个实例，
///   绑定它，状态只看那个实例的节点；实例未出现 → pending、不绑定。
///   多出 `of` 的实例不出卡。
/// - 工作区等合成车道上没有实例，卡不动。
///
/// 节点按 `(siteId ∈ 卡的站点, actorSiteId === lane, actorOrdinal === ordinal,
/// 卡的阶段 ∈ phasesOf(节点))` 收窄；站点取 `step.source ?? step.id`（may-set 拷贝报的是
/// 站点 id，与 run-status.ts 的关联键同源）。实例在场而它的站点上一个节点都没有 → pending：
/// 实例是观察到的事实，只是还没在这一站动（折叠自己对空集只说 undefined，解缺席是这里的事）。
///
/// ★「实例绑定」（真源 :207-210）——一张卡认领的实例不再是**整条车道**上的实例：
/// 同一个站点被 k 个阶段再入时，k 张卡共享一条车道，按车道认领就是一次广播
/// （每站都列全部 100 个）。见 `instances_of_card`。
pub fn live_participant_view(
    graph: &WorkflowCausalityGraphData,
    run: Option<&WorkflowRunState>,
) -> LiveParticipantView {
    let Some(run) = run else {
        // ★真源 :216 —— 无 run 时就是输入图（引用相等）。
        return LiveParticipantView {
            graph: graph.clone(),
            instances: HashMap::new(),
            participant_statuses: HashMap::new(),
        };
    };
    // 站点取 `source ?? id`（真源 :217）——may-set 拷贝报的是站点 id。
    let site_of: HashMap<&str, &str> = graph
        .steps
        .iter()
        .map(|step| {
            (
                step.id.as_str(),
                step.source.as_deref().unwrap_or(step.id.as_str()),
            )
        })
        .collect();
    let mut binder = phase_binder(graph, Some(run));
    let index = run_index(run);

    // 真源 :219-220 —— 一张卡的站点集合。
    let sites_of = |participant: &WorkflowParticipantData| -> HashSet<String> {
        participant
            .steps
            .iter()
            .map(|id| site_of.get(id.as_str()).copied().unwrap_or(id.as_str()).to_string())
            .collect()
    };

    /// 真源 :231-247 —— 这张卡名下的实例：车道上在**这张卡的站点**留下过节点、
    /// 且该节点的戳落在这张卡的阶段的 actor；在这些站点上还一个节点都没有的 actor
    /// （建了还没被 ask，或还没走到这一站）则按它**自己的出生戳**归位。
    /// 无戳的 run 里 `phasesOf` 恒是全部阶段，两条合起来正是今天的「按车道」
    /// ——旧 run 逐字节不变。
    fn instances_of_card(
        participant: &WorkflowParticipantData,
        sites: &HashSet<String>,
        index: &RunIndex,
        binder: &mut PhaseBinder,
    ) -> Vec<WorkflowRunActor> {
        let mut claimed: Vec<WorkflowRunActor> = Vec::new();
        let Some(actors) = index.actors_by_site.get(&participant.lane) else {
            return claimed;
        };
        for actor in actors {
            let mut seen = false;
            let mut here = false;
            if let Some(nodes) = index
                .nodes_by_actor
                .get(&actor_key(&participant.lane, actor.ordinal))
            {
                for node in nodes {
                    if !sites.contains(&node.site_id) {
                        continue;
                    }
                    seen = true;
                    if binder.has(&participant.phase, node.phase_name.as_deref()) {
                        here = true;
                        break;
                    }
                }
            }
            if here || (!seen && binder.has(&participant.phase, actor.phase_name.as_deref())) {
                claimed.push(actor.clone());
            }
        }
        claimed
    }

    /// 真源 :248-259 —— 按实例收状态。
    fn status_for(
        participant: &WorkflowParticipantData,
        ordinal: i64,
        sites: &HashSet<String>,
        index: &RunIndex,
        binder: &mut PhaseBinder,
    ) -> StepRunStatus {
        let mut values: Vec<StepRunStatus> = Vec::new();
        if let Some(nodes) = index.nodes_by_actor.get(&actor_key(&participant.lane, ordinal)) {
            for node in nodes {
                if !sites.contains(&node.site_id)
                    || !binder.has(&participant.phase, node.phase_name.as_deref())
                {
                    continue;
                }
                values.push(status_of_run_node(node));
            }
        }
        // ★真源 :258 —— 空集落pending：实例在场而它的站点上一个节点都没有，
        // 实例是观察到的事实，只是还没在这一站动。
        aggregate_run_statuses(&values).unwrap_or(StepRunStatus::Pending)
    }

    let mut participants: Vec<WorkflowParticipantData> = Vec::new();
    let mut replacements: HashMap<String, Vec<String>> = HashMap::new();
    let mut participant_statuses: HashMap<String, StepRunStatus> = HashMap::new();
    let mut instances: HashMap<String, ParticipantInstance> = HashMap::new();
    let mut changed = false;
    for participant in &graph.participants {
        let sites = sites_of(participant);
        let actors = instances_of_card(participant, &sites, &index, &mut binder);
        // 成员卡（真源 :269-276）：字面量基数展开的那一张。
        if let Some(member) = &participant.member {
            let actor = actors.get(member.index.max(0) as usize);
            participant_statuses.insert(
                participant.id.clone(),
                match actor {
                    None => StepRunStatus::Pending,
                    Some(actor) => status_for(participant, actor.ordinal, &sites, &index, &mut binder),
                },
            );
            if let Some(actor) = actor {
                instances.insert(
                    participant.id.clone(),
                    bound_instance(&participant.id, actor),
                );
            }
            participants.push(participant.clone());
            continue;
        }
        // 单卡恰有一个实例（真源 :277-281）：原卡绑定它，id 与标签规则都不变。
        if participant.many != Some(true) && actors.len() == 1 {
            instances.insert(
                participant.id.clone(),
                bound_instance(&participant.id, &actors[0]),
            );
            participants.push(participant.clone());
            continue;
        }
        // `many` 卡或多个实例（真源 :282-303）：按真实实例拆开。
        if participant.many == Some(true) || actors.len() > 1 {
            if actors.is_empty() {
                // ★真源 :283-286 —— 实例尚未出现时保留原来那一张
                // （状态由 step 折叠）。
                participants.push(participant.clone());
                continue;
            }
            changed = true;
            let mut ids: Vec<String> = Vec::new();
            for actor in &actors {
                let id = instance_card_id(&participant.id, actor.ordinal);
                let mut split = participant.clone();
                split.many = None;
                split.id = id.clone();
                participants.push(split);
                participant_statuses.insert(
                    id.clone(),
                    status_for(participant, actor.ordinal, &sites, &index, &mut binder),
                );
                instances.insert(
                    id.clone(),
                    ParticipantInstance {
                        ordinal: actor.ordinal,
                        participant: participant.id.clone(),
                        name: actor.name.clone(),
                        // 缺席 = 拆分出的实例卡（真源 :176-180）。
                        bound: false,
                    },
                );
                ids.push(id);
            }
            replacements.insert(participant.id.clone(), ids);
            continue;
        }
        participants.push(participant.clone());
    }
    // ★真源 :306 —— 没拆过就不重建图（引用相等，memo友好）。
    if !changed {
        return LiveParticipantView {
            graph: graph.clone(),
            instances,
            participant_statuses,
        };
    }

    // 交接边按原卡复制到每张实例卡（真源 :308-313）。
    let mut handoffs: Vec<WorkflowHandoffData> = Vec::new();
    for edge in &graph.handoffs {
        let empty = Vec::new();
        let froms = replacements.get(&edge.from).unwrap_or(&empty);
        let tos = replacements.get(&edge.to).unwrap_or(&empty);
        let froms: Vec<&String> = if froms.is_empty() {
            vec![&edge.from]
        } else {
            froms.iter().collect()
        };
        let tos: Vec<&String> = if tos.is_empty() { vec![&edge.to] } else { tos.iter().collect() };
        for from in &froms {
            for to in &tos {
                let mut split = edge.clone();
                split.from = (*from).clone();
                split.to = (*to).clone();
                handoffs.push(split);
            }
        }
    }
    let mut split_graph = graph.clone();
    split_graph.handoffs = handoffs;
    split_graph.participants = participants;
    LiveParticipantView {
        graph: split_graph,
        instances,
        participant_statuses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::run_status::aggregate_run_statuses;

    fn graph_from_str(raw: &str) -> WorkflowCausalityGraphData {
        serde_json::from_str(raw).unwrap()
    }

    fn run_from_str(raw: &str) -> WorkflowRunState {
        serde_json::from_str(raw).unwrap()
    }

    fn bare_graph() -> WorkflowCausalityGraphData {
        graph_from_str(
            r#"{"steps": [
                { "id": "s1", "kind": "ask", "label": "问A", "lane": "agent-1" },
                { "id": "s2", "kind": "world-read", "label": "读文件", "lane": "agent-1" },
                { "id": "s3", "kind": "ask", "label": "问B", "lane": "agent-2", "repeat": "serial" }
            ], "lanes": [], "handoffs": [],
            "participants": [
                { "id": "p1", "phase": "unphased", "lane": "agent-1", "steps": ["s1", "s2"] },
                { "id": "p2", "phase": "unphased", "lane": "agent-2", "steps": ["s3"] }
            ],
            "sink": ["s1"]}"#,
        )
    }

    // ── 隐式阶段 ──

    #[test]
    fn implicit_phase_created_when_no_vocabulary() {
        // ★真源 :38-47 —— 无标记脚本合成一个隐式模块 workflow。
        let graph = bare_graph();
        assert!(!has_phase_vocabulary(&graph));
        let out = with_implicit_phase(&graph);
        assert!(has_phase_vocabulary(&out));
        assert_eq!(out.phases.as_ref().unwrap().len(), 1);
        assert_eq!(out.phases.as_ref().unwrap()[0].id, IMPLICIT_PHASE_ID);
        // ★真源 :39 —— 有返回物才有一个可正常完成的阶段。
        assert_eq!(out.exits.as_deref(), Some(&[IMPLICIT_PHASE_ID.to_string()][..]));
    }

    #[test]
    fn implicit_phase_exits_empty_without_sink() {
        let mut graph = bare_graph();
        graph.sink = None;
        let out = with_implicit_phase(&graph);
        assert_eq!(
            out.exits.as_deref(),
            Some(&[][..]),
            "无返回物 → 没有可正常完成的阶段"
        );
    }

    #[test]
    fn implicit_phase_reassigns_steps_and_cards_but_keeps_ids() {
        // ★真源 :41-46 —— 分析器给的 unphased 只是占位，**卡 id 不变**。
        let out = with_implicit_phase(&bare_graph());
        assert!(out.steps.iter().all(|s| s.phase.as_deref() == Some(IMPLICIT_PHASE_ID)));
        assert!(out
            .participants
            .iter()
            .all(|p| p.phase == IMPLICIT_PHASE_ID));
        assert_eq!(out.participants[0].id, "p1", "卡 id 不变");
        assert_eq!(out.participants[1].id, "p2");
        assert_eq!(out.steps[0].id, "s1", "step id 不变");
    }

    #[test]
    fn implicit_phase_no_phase_edges() {
        let out = with_implicit_phase(&bare_graph());
        assert_eq!(
            out.phase_edges.as_deref(),
            Some(&[][..]),
            "隐式模块没有阶段边"
        );
    }

    #[test]
    fn graph_with_vocabulary_returned_unchanged() {
        // 真源 :47 —— 有词汇表的图原样返回（引用相等，memo 友好）。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [], "participants": [],
                "phases": [{"id": "preflight", "name": "预备"}]}"#,
        );
        let out = with_implicit_phase(&graph);
        assert_eq!(out, graph, "有词汇表时不该改");
    }

    // ── 选择器 ──

    #[test]
    fn participants_of_phase_keep_payload_order() {
        // 真源 :52-55 —— 保持载荷顺序（= 交接序）。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [],
                "participants": [
                    { "id": "p1", "phase": "main", "lane": "a", "steps": ["s1"] },
                    { "id": "p2", "phase": "pre", "lane": "a", "steps": ["s2"] },
                    { "id": "p3", "phase": "main", "lane": "a", "steps": ["s3"] }
                ],
                "phases": [{"id": "main"}, {"id": "pre"}]}"#,
        );
        let main = participants_of_phase(&graph, "main");
        assert_eq!(main.len(), 2);
        assert_eq!(main[0].id, "p1");
        assert_eq!(main[1].id, "p3", "保持声明序");
    }

    #[test]
    fn participant_by_id_finds_or_none() {
        let graph = bare_graph();
        assert_eq!(participant_by_id(&graph, "p1").unwrap().lane, "agent-1");
        assert!(participant_by_id(&graph, "nope").is_none());
    }

    #[test]
    fn handoffs_within_filters_both_ends() {
        // 真源 :67-69 —— 两端都在集合里（阶段内部的边）。
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [],
                "handoffs": [
                    { "from": "p1", "to": "p2" },
                    { "from": "p2", "to": "p1" },
                    { "from": "p1", "to": "p9" }
                ]}"#,
        );
        let ids: std::collections::HashSet<String> =
            ["p1".to_string(), "p2".to_string()].into_iter().collect();
        let within = handoffs_within(&graph, &ids);
        assert_eq!(within.len(), 2, "p1→p9 有一端在外");
    }

    #[test]
    fn handoffs_around_splits_in_out() {
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "participants": [],
                "handoffs": [
                    { "from": "p1", "to": "p2" },
                    { "from": "p2", "to": "p3" },
                    { "from": "p1", "to": "p3" }
                ]}"#,
        );
        let around = handoffs_around(&graph, "p2");
        assert_eq!(around.incoming.len(), 1);
        assert_eq!(around.incoming[0].from, "p1");
        assert_eq!(around.outgoing.len(), 1);
        assert_eq!(around.outgoing[0].to, "p3");
    }

    // ── 计数 ──

    #[test]
    fn counts_split_asks_and_reads() {
        // ★真源 :87 —— 一张卡只会有其中一种非零。
        let graph = bare_graph();
        let p1 = participant_by_id(&graph, "p1").unwrap();
        let counts = participant_counts(&graph, p1);
        assert_eq!(counts.asks, 1, "s1 是 ask");
        assert_eq!(counts.reads, 1, "s2 是 world-read");
    }

    #[test]
    fn counts_treat_unknown_kind_as_ask() {
        // 真源 :102 —— 只有 world-read 记reads，其余都记 asks。
        let graph = graph_from_str(
            r#"{"steps": [
                { "id": "s1", "kind": "mystery", "label": "?", "lane": "a" }
            ], "lanes": [], "handoffs": [],
            "participants": [{ "id": "p1", "phase": "x", "lane": "a", "steps": ["s1"] }]}"#,
        );
        let p = participant_by_id(&graph, "p1").unwrap();
        let counts = participant_counts(&graph, p);
        assert_eq!(counts.asks, 1);
        assert_eq!(counts.reads, 0);
    }

    #[test]
    fn repeat_flag_marks_card() {
        // ★真源 :107 —— 任一 step 带 repeat ⇒ 卡片显示重复图元。
        let graph = bare_graph();
        assert!(!participant_repeats(&graph, participant_by_id(&graph, "p1").unwrap()));
        assert!(participant_repeats(&graph, participant_by_id(&graph, "p2").unwrap()));
    }

    // ── 状态 ──

    #[test]
    fn participant_status_prefers_instance_override() {
        // ★真源 :121-122 —— 实时视图的按实例收窄覆盖值优先。
        let graph = bare_graph();
        let p = participant_by_id(&graph, "p1").unwrap();
        let statuses: StepStatusTable = [("s1".to_string(), StepRunStatus::Done)]
            .into_iter()
            .collect();
        let overrides: HashMap<String, StepRunStatus> =
            [("p1".to_string(), StepRunStatus::Running)].into_iter().collect();
        assert_eq!(
            participant_status(p, Some(&statuses), Some(&overrides)),
            Some(StepRunStatus::Running),
            "覆盖值优先于step 折叠"
        );
        // 没有覆盖时回落 step 折叠。
        assert_eq!(
            participant_status(p, Some(&statuses), None),
            Some(StepRunStatus::Done)
        );
        // 都没有 → None。
        assert_eq!(participant_status(p, None, None), None);
    }

    #[test]
    fn participant_also_in_lists_other_phases_of_same_lane() {
        let graph = graph_from_str(
            r#"{"steps": [], "lanes": [], "handoffs": [],
                "participants": [
                    { "id": "p1", "phase": "main", "lane": "agent-1", "steps": ["s1"] },
                    { "id": "p2", "phase": "pre", "lane": "agent-1", "steps": ["s2"] },
                    { "id": "p3", "phase": "post", "lane": "agent-2", "steps": ["s3"] }
                ],
                "phases": [{"id": "pre"}, {"id": "main"}, {"id": "post"}]}"#,
        );
        let p1 = participant_by_id(&graph, "p1").unwrap();
        let also = participant_also_in(&graph, p1);
        assert_eq!(also.len(), 1);
        assert_eq!(also[0].id, "pre", "同车道另一个阶段");
        // 不同车道不进列表。
        assert_eq!(also[0].id, "pre");
    }

    #[test]
    fn instance_card_id_format() {
        // 真源 :153 —— `${participant.id}@${ordinal}`。
        assert_eq!(instance_card_id("p1", 3), "p1@3");
    }

    #[test]
    fn aggregate_statuses_still_reachable_from_here() {
        // 再导出检查：participant-model 的 collapseStatuses 已搬到 run_status.rs，
        // 这里确认两条路径给出同一答案。
        let ids = vec!["a".to_string(), "b".to_string()];
        let statuses: StepStatusTable = [
            ("a".to_string(), StepRunStatus::Pending),
            ("b".to_string(), StepRunStatus::Done),
        ]
        .into_iter()
        .collect();
        assert_eq!(collapse_statuses(&ids, Some(&statuses)), Some(StepRunStatus::Running));
        assert_eq!(
            aggregate_run_statuses(&[StepRunStatus::Pending, StepRunStatus::Done]),
            Some(StepRunStatus::Running),
            "两条路径一致"
        );
    }

    // ── 实时视图 ──

    /// 一张 `many` 卡在 lane `l1` 上，站点 `s1`。
    fn many_graph() -> WorkflowCausalityGraphData {
        graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [
                    {"id": "p1", "phase": "main", "lane": "l1", "steps": ["s1"], "many": true}
                ]}"#,
        )
    }

    fn many_run(ordinals: &str) -> WorkflowRunState {
        run_from_str(&format!(
            r#"{{"runId": "r", "status": "running",
                "actors": [{ordinals}],
                "nodes": [
                  {{"siteId": "s1", "ordinal": 0, "phase": "executing",
                    "actorSiteId": "l1", "actorOrdinal": 0}},
                  {{"siteId": "s1", "ordinal": 1, "phase": "settled", "outcome": "ok",
                    "actorSiteId": "l1", "actorOrdinal": 1}}
                ]}}"#
        ))
    }

    #[test]
    fn live_view_without_run_returns_input_graph() {
        // ★真源 :216 —— 无 run 时就是输入图（引用相等）。
        let graph = many_graph();
        let view = live_participant_view(&graph, None);
        assert_eq!(view.graph, graph);
        assert!(view.instances.is_empty());
        assert!(view.participant_statuses.is_empty());
    }

    #[test]
    fn many_card_splits_into_one_card_per_instance() {
        // ★真源 :195 —— `many` 卡：每个已出现的 actor 实例各出一张卡。
        let run = many_run(
            r#"{"siteId": "l1", "ordinal": 0, "status": "running", "name": "研究员1"},
               {"siteId": "l1", "ordinal": 1, "status": "completed", "name": "研究员2"}"#,
        );
        let view = live_participant_view(&many_graph(), Some(&run));
        let ids: Vec<&str> = view.graph.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["p1@0", "p1@1"], "实例卡 id 是参与者 id + @ + ordinal");
        // 状态按实例收窄，不是一张卡一个状态。
        assert_eq!(
            view.participant_statuses.get("p1@0"),
            Some(&StepRunStatus::Running)
        );
        assert_eq!(
            view.participant_statuses.get("p1@1"),
            Some(&StepRunStatus::Done)
        );
        // 身份表：名字是引擎发出的有效名，卡面优先念它。
        assert_eq!(view.instances["p1@0"].name.as_deref(), Some("研究员1"));
        assert_eq!(view.instances["p1@0"].participant, "p1");
        assert!(!view.instances["p1@0"].bound, "拆分出的实例卡不bound");
    }

    #[test]
    fn split_cards_lose_the_many_flag() {
        // 真源 :291 —— `{...rest}` 去掉 `many`，实例卡自己不再声称「多」。
        let run = many_run(r#"{"siteId": "l1", "ordinal": 0, "status": "running"}"#);
        let view = live_participant_view(&many_graph(), Some(&run));
        for card in &view.graph.participants {
            assert_eq!(card.many, None);
        }
    }

    #[test]
    fn many_card_without_instances_stays_as_is() {
        // ★真源 :196 —— 实例尚未出现时保留原来那一张
        // （状态由 step 折叠，所以不进 participant_statuses）。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [],
                "nodes": []}"#,
        );
        let view = live_participant_view(&many_graph(), Some(&run));
        assert_eq!(view.graph.participants.len(), 1);
        assert_eq!(view.graph.participants[0].id, "p1");
        assert!(view.participant_statuses.is_empty());
    }

    #[test]
    fn handoffs_are_copied_to_every_split_card() {
        // ★真源 :196 /308-313 —— 交接边按原卡复制到每张实例卡。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [{"from": "p1", "to": "p2"}],
                "participants": [
                    {"id": "p1", "phase": "main", "lane": "l1", "steps": ["s1"], "many": true},
                    {"id": "p2", "phase": "main", "lane": "l2", "steps": []}
                ]}"#,
        );
        let run = many_run(
            r#"{"siteId": "l1", "ordinal": 0, "status": "running"},
               {"siteId": "l1", "ordinal": 1, "status": "completed"}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        let mut edges: Vec<(String, String)> = view
            .graph
            .handoffs
            .iter()
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect();
        edges.sort();
        assert_eq!(
            edges,
            vec![
                ("p1@0".to_string(), "p2".to_string()),
                ("p1@1".to_string(), "p2".to_string()),
            ],
            "每张实例卡各拿一份边"
        );
    }

    #[test]
    fn single_card_with_one_instance_is_bound_not_split() {
        // ★真源 :197 —— 车道上恰有一个实例 → 原卡绑定它
        // （id 不变、状态仍由 step 折叠，只是名字有了）。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [{"id": "p1", "phase": "main", "lane": "l1", "steps": ["s1"]}]}"#,
        );
        let run = many_run(r#"{"siteId": "l1", "ordinal": 0, "status": "running", "name": "研究员1"}"#);
        let view = live_participant_view(&graph, Some(&run));
        assert_eq!(view.graph.participants.len(), 1);
        assert_eq!(view.graph.participants[0].id, "p1", "卡 id 不变");
        assert!(
            view.participant_statuses.is_empty(),
            "单卡状态仍由 step 折叠，不进覆盖表"
        );
        let bound = &view.instances["p1"];
        assert!(bound.bound, "原卡绑定");
        assert_eq!(bound.name.as_deref(), Some("研究员1"));
    }

    #[test]
    fn runtime_cardinality_beats_static_cardinality() {
        // ★真源 :198 —— 两个以上 → 与 `many` 同一条拆分路径：
        // 运行时基数胜过静态基数（这张卡静态上没写 many）。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [{"id": "p1", "phase": "main", "lane": "l1", "steps": ["s1"]}]}"#,
        );
        let run = many_run(
            r#"{"siteId": "l1", "ordinal": 0, "status": "running"},
               {"siteId": "l1", "ordinal": 1, "status": "completed"}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        let ids: Vec<&str> = view.graph.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["p1@0", "p1@1"]);
    }

    #[test]
    fn member_card_binds_the_nth_instance_by_ordinal() {
        // ★真源 :199-200 —— 成员卡：第 i 个成员对应该车道上按 ordinal 排序的第 i 个实例。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [
                    {"id": "p0", "phase": "main", "lane": "l1", "steps": ["s1"],
                     "member": {"index": 0, "of": 2}},
                    {"id": "p1", "phase": "main", "lane": "l1", "steps": ["s1"],
                     "member": {"index": 1, "of": 2}}
                ]}"#,
        );
        let run = many_run(
            r#"{"siteId": "l1", "ordinal": 0, "status": "running", "name": "甲"},
               {"siteId": "l1", "ordinal": 1, "status": "completed", "name": "乙"}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        assert_eq!(view.graph.participants.len(), 2, "成员卡不拆开");
        assert_eq!(view.participant_statuses["p0"], StepRunStatus::Running);
        assert_eq!(view.participant_statuses["p1"], StepRunStatus::Done);
        assert_eq!(view.instances["p0"].name.as_deref(), Some("甲"));
        assert_eq!(view.instances["p1"].name.as_deref(), Some("乙"));
        assert!(view.instances["p0"].bound, "成员卡是绑定而非拆分");
    }

    #[test]
    fn member_card_without_its_instance_is_pending_and_unbound() {
        // ★真源 :200 —— 实例未出现 → pending、不绑定。
        // 声明 of=3 但只出现了 ordinal 0 那一个：index 1 的成员卡没实例可认。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [
                    {"id": "p0", "phase": "main", "lane": "l1", "steps": ["s1"],
                     "member": {"index": 0, "of": 3}},
                    {"id": "p1", "phase": "main", "lane": "l1", "steps": ["s1"],
                     "member": {"index": 1, "of": 3}}
                ]}"#,
        );
        let run = many_run(r#"{"siteId": "l1", "ordinal": 0, "status": "running"}"#);
        let view = live_participant_view(&graph, Some(&run));
        assert_eq!(view.participant_statuses["p0"], StepRunStatus::Running);
        assert_eq!(
            view.participant_statuses["p1"],
            StepRunStatus::Pending,
            "第1 号成员还没有对应实例"
        );
        assert!(!view.instances.contains_key("p1"), "缺席 = 不绑定");
    }

    #[test]
    fn instance_present_but_idle_on_this_site_is_pending() {
        // ★真源 :204-206 —— 实例在场而它的站点上一个节点都没有 → pending：
        // 实例是观察到的事实，只是还没在这一站动。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [
                    {"id": "p0", "phase": "main", "lane": "l1", "steps": ["s1"],
                     "member": {"index": 0, "of": 1}}
                ]}"#,
        );
        // actor 在别的站点留下了节点 —— 它在l1 上还没动。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "elsewhere", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "l1", "actorOrdinal": 0}]}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        assert!(view.instances.contains_key("p0"), "建了还没被ask → 按出生戳归位");
        assert_eq!(view.participant_statuses["p0"], StepRunStatus::Pending);
    }

    #[test]
    fn instance_binding_is_a_partition_not_a_broadcast() {
        // ★真源 :207-210 —— 同一个站点被 k 个阶段再入时，k 张卡共享一条车道；
        // 按车道认领就是一次广播（每站都列全部 100 个）。
        let graph = graph_from_str(
            r#"{"steps": [
                  {"id": "s1", "kind": "ask", "label": "问", "lane": "l1", "source": "site1"}
                ], "lanes": [], "handoffs": [],
                "participants": [
                    {"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]},
                    {"id": "b", "phase": "p2", "lane": "l1", "steps": ["s1"]}
                ],
                "phases": [{"id": "p1", "name": "阶段一"}, {"id": "p2", "name": "阶段二"}]}"#,
        );
        // 只有一个 actor，它出生在阶段一。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running",
                            "phaseName": "阶段一"}],
                "nodes": [{"siteId": "site1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "l1", "actorOrdinal": 0, "phaseName": "阶段一"}]}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        assert!(
            view.instances.contains_key("a"),
            "阶段一那张卡认领它"
        );
        assert!(
            !view.instances.contains_key("b"),
            "★阶段二那张卡不认领——一次划分，不是一次广播"
        );
    }

    #[test]
    fn unstamped_run_falls_back_to_claiming_by_lane() {
        // ★真源 :222-229 —— 无戳的 run 里 `phasesOf` 恒是全部阶段，
        // 两条合起来正是今天的「按车道」——旧 run 逐字节不变。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "s1", "kind": "ask", "label": "问", "lane": "l1"}],
                "lanes": [], "handoffs": [],
                "participants": [
                    {"id": "a", "phase": "p1", "lane": "l1", "steps": ["s1"]},
                    {"id": "b", "phase": "p2", "lane": "l1", "steps": ["s1"]}
                ],
                "phases": [{"id": "p1", "name": "阶段一"}, {"id": "p2", "name": "阶段二"}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "s1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "l1", "actorOrdinal": 0}]}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        assert!(
            view.instances.contains_key("a") && view.instances.contains_key("b"),
            "旧 run按车道认领，两张卡都绑定（逐字节不变）"
        );
    }

    #[test]
    fn synthetic_lane_cards_never_move() {
        // ★真源 :201 —— 工作区等合成车道上没有实例，卡不动。
        let graph = graph_from_str(
            r#"{"steps": [{"id": "r1", "kind": "world-read", "label": "读", "lane": "workspace"}],
                "lanes": [], "handoffs": [],
                "participants": [{"id": "w", "phase": "main", "lane": "workspace",
                                  "steps": ["r1"]}]}"#,
        );
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "r1", "ordinal": 0, "phase": "executing",
                           "actorSiteId": "l1", "actorOrdinal": 0}]}"#,
        );
        let view = live_participant_view(&graph, Some(&run));
        assert_eq!(view.graph, graph, "没有卡被拆 → 图原样返回");
        assert!(view.instances.is_empty());
    }

    #[test]
    fn actor_key_uses_nul_separator() {
        // ★真源 :317-323 —— 用 `\0` 连接：siteId 是引擎给的任意字符串，
        // 用可打印分隔符会让 ("a-1", 2) 与 ("a", "1-2") 撞车。
        assert_eq!(actor_key("a-1", 2), "a-1\0 2".replace(' ', ""));
        assert_ne!(actor_key("a-1", 2), actor_key("a", 12));
    }

    #[test]
    fn run_index_skips_nodes_without_actor() {
        // ★真源 :337-338 —— 没有 actor 的节点（world-read）与不带ordinal
        // 的旧载荷不进索引：原筛选条件对它们恒假，丢掉等价。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running",
                "actors": [{"siteId": "l1", "ordinal": 0, "status": "running"}],
                "nodes": [{"siteId": "r1", "ordinal": 0, "phase": "executing"}]}"#,
        );
        let index = run_index(&run);
        assert_eq!(index.actors_by_site["l1"].len(), 1);
        assert!(index.nodes_by_actor.is_empty());
    }

    #[test]
    fn run_index_sorts_actors_by_ordinal() {
        // 真源 :355 —— 车道上的 actor 按 ordinal 升序（认领顺序）。
        let run = run_from_str(
            r#"{"runId": "r", "status": "running", "actors": [
                {"siteId": "l1", "ordinal": 5, "status": "running"},
                {"siteId": "l1", "ordinal": 2, "status": "running"},
                {"siteId": "l1", "ordinal": 9, "status": "running"}
              ], "nodes": []}"#,
        );
        let index = run_index(&run);
        let ords: Vec<i64> = index.actors_by_site["l1"].iter().map(|a| a.ordinal).collect();
        assert_eq!(ords, vec![2, 5, 9]);
    }
}