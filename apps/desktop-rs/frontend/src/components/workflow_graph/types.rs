//! 1:1 翻译 `packages/ui/src/components/workflow-graph/types.ts`（91 行）。
//!
//! 真源这一层几乎全是**注释**——它在定义「渲染器消费的图就是 CreateWorkflow 工具
//! 那份有界 display 载荷」，即「从分析器到像素只有一套词汇」。
//! Rust 侧保留这份约定并落成结构。

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 因果图载荷（照抄 create-workflow-display.ts:20-133的 zod 约束）
// ---------------------------------------------------------------------------

/// `namePatternSchema`（真源 :20-25）：名字的**静态形状**。
///
/// ★真源 `name-pattern.ts` 注释——分析器给的是形状而不是名字：
/// `` agent(`研究员${i + 1}`) `` 折不成 8 个具体名字（那要求把 `map` 展开，
/// 而 `×N` 存在的意义正是拒绝展开），能拿到的只有模板两端的字面量。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamePattern {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<String>,
}

/// `workflowEdgeSchema`（真源 :29-35）：一条边 = runs after。
///
/// 交接边与阶段边同形。`back` 只标循环回边（布局排秩与 cycle 计数读），
/// 画法与其他边相同。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<bool>,
}

/// fan-out 家族的字面量基数（真源 :88-91）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantMember {
    /// `z.number().int().nonnegative()`
    pub index: i64,
    /// `z.number().int().positive()`
    pub of: i64,
}

/// 一个 step（真源 :41-60）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStepData {
    pub id: String,
    /// `z.enum(["ask", "world-read"])`
    pub kind: String,
    pub label: String,
    #[serde(
        rename = "labelPattern",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub label_pattern: Option<NamePattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    pub lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lanes: Option<Vec<String>>,
    /// 展开自的站点 id，只出现在 may-set 车道展开的拷贝上。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 作者用 `phase("…")` 标记划入的阶段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// `z.enum(["stack", "serial"])`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
}

/// 一条车道（真源 :65-75）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowLaneData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `name` 缺席而 agent() 首参是带洞的模板串时的静态形状；**与 name 互斥**。
    #[serde(
        rename = "namePattern",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub name_pattern: Option<NamePattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
}

/// 一张参与者卡（真源 :81-96）。
///
/// ★真源 :78-79 注释——参与者 = 每阶段一张子代理卡（工作区 / 未解析同形）；
/// 数组顺序就是交接序，第一张是开局者。fan-out 家族按字面量基数展开成 `member`，
/// 基数未知时一张 `many` 卡。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowParticipantData {
    pub id: String,
    pub phase: String,
    pub lane: String,
    pub steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<ParticipantMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub many: Option<bool>,
}

/// 一条交接边（真源 :98-104）：参与者之间的 runs after。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowHandoffData {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<bool>,
    /// 跨越它的产物类型（检视器素材，**不上箭头**）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<String>>,
}

/// 一个阶段（真源 :109-125）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPhaseData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// 进入本阶段时还在跑的其他阶段（它们的 strand 尚未 join）。
    ///
    /// ★真源 :118-121 注释——**是节点事实而不是边**：控制没有从那里转移过来，
    /// 所以不进 `phaseEdges`。时间轴据此把相邻阶段折成一条分叉的「带」，
    /// 侧栏迷你轨道画成双线段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alongside: Option<Vec<String>>,
}

/// `ToolCallCreateWorkflowCausalityGraph`（真源 :37-133）。
///
/// ★载荷是**camelCase**（display 的 `causalityGraph` 是原始 JSON），所以多词字段
/// 逐个显式 `rename`——漏一个不会报错，只会让那一项**静默变成 `None`**：
/// `phaseEdges` 读成 `phase_edges` 时整张图的边集合是空的，而所有断言
/// 「没有轨道段」的测试都会通过。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowCausalityGraphData {
    #[serde(rename = "steps")]
    pub steps: Vec<WorkflowStepData>,
    #[serde(rename = "lanes")]
    pub lanes: Vec<WorkflowLaneData>,
    #[serde(rename = "participants")]
    pub participants: Vec<WorkflowParticipantData>,
    #[serde(rename = "handoffs")]
    pub handoffs: Vec<WorkflowHandoffData>,
    /// 阶段词汇表：作者施加的分组结构。
    ///
    /// ★真源 :106-108 注释——与 `phaseEdges` / `exits` / `Step.phase`
    /// **全有或全无**：零标记脚本全缺席，UI 据此退回 step/车道视图。
    /// 零成员阶段也在表里。`unphased` 无 name，显示名由 UI 本地化。
    #[serde(
        rename = "phases",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub phases: Option<Vec<WorkflowPhaseData>>,
    #[serde(
        rename = "phaseEdges",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub phase_edges: Option<Vec<WorkflowEdge>>,
    /// 控制流可在其后正常完成的阶段（阶段视图的「阶段 → 返回物」箭头）。
    #[serde(
        rename = "exits",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exits: Option<Vec<String>>,
    #[serde(
        rename = "sink",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sink: Option<Vec<String>>,
    #[serde(
        rename = "truncated",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub truncated: Option<bool>,
}

// ---------------------------------------------------------------------------
// 运行状态（真源 :27-56）
// ---------------------------------------------------------------------------

/// `StepRunStatus`（真源 :27）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepRunStatus {
    Pending,
    Running,
    Done,
    Failed,
}

/// `StepStatusTable`（真源 :33-38）。
///
/// ★真源 :31-32 注释——**偏表**：没有观察到实例的 step 没有条目。
/// 缺席与 `pending` 是两件事：前者是「这里什么都没发生」，
/// 后者是「一个真实的实例在排队」。把两者写成同一个值会让控制流没走的分支
/// 站点把整站拖成 pending。
pub type StepStatusTable = std::collections::HashMap<String, StepRunStatus>;

/// 板面上可被选中的三类东西（真源 :44-50）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowGraphSelectionKind {
    Participant,
    Phase,
    Sink,
}

impl WorkflowGraphSelectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Participant => "participant",
            Self::Phase => "phase",
            Self::Sink => "sink",
        }
    }
}

/// `WorkflowCausalityGraphSelection`（真源 :46-50）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowCausalityGraphSelection {
    pub id: String,
    pub kind: WorkflowGraphSelectionKind,
    /// 宿主做脚本行定位的便利字段（权限块）。
    pub line: Option<i64>,
}

// ---------------------------------------------------------------------------
// id 常量（真源 :52-83）
// ---------------------------------------------------------------------------

/// `WORKSPACE_LANE_ID`（真源 :52）：所有 `files.*` 读操作跑在的那条车道。
pub const WORKSPACE_LANE_ID: &str = "workspace";

/// `UNKNOWN_LANE_ID`（真源 :54）：接收方无法定位到某个 actor 的询问所在车道。
pub const UNKNOWN_LANE_ID: &str = "unknown";

/// `SINK_NODE_ID`（真源 :56）：终端标记节点——工作流返回的产物。
pub const SINK_NODE_ID: &str = "sink";

/// `UNPHASED_PHASE_ID`（真源 :64）。
///
/// ★真源 :58-63 注释——兜底阶段：首个 `phase()` 标记之前发出的 step 的家，
/// 也是无标记脚本的隐式唯一阶段（分析器的参与者恒带 `phase: "unphased"`）。
/// 保留 id，且**没有 name**——显示名由 UI 本地化。
pub const UNPHASED_PHASE_ID: &str = "unphased";

/// `IMPLICIT_PHASE_ID`（真源 :72）。
///
/// ★真源 :66-71 注释——与 `unphased` **分开**：那个词在有标记的脚本里
/// 意味着「首个标记之前」（Ungrouped），而隐式模块就是整个工作流（Workflow）。
pub const IMPLICIT_PHASE_ID: &str = "workflow";

/// `LaneClass`（真源 :78）。
///
/// ★真源 :74-77 注释——这条是**颜色唯一编码的东西**。
/// `workspace` 不是 actor（没有邮箱，故没有 `fifo`），
/// `unresolved` 是接收方无法定位的询问——两者都是真区别。
/// per-actor 的身份由卡上的名字承载，不由色相承载。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneClass {
    Agent,
    Workspace,
    Unresolved,
}

/// `laneClassOf`（真源 :80-84）。
pub fn lane_class_of(lane_id: &str) -> LaneClass {
    if lane_id == WORKSPACE_LANE_ID {
        LaneClass::Workspace
    } else if lane_id == UNKNOWN_LANE_ID {
        LaneClass::Unresolved
    } else {
        LaneClass::Agent
    }
}

/// `isSyntheticLaneId`（真源 :86-88）。
pub fn is_synthetic_lane_id(id: &str) -> bool {
    id == WORKSPACE_LANE_ID || id == UNKNOWN_LANE_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lane_class_distinguishes_three_cases() {
        // 真源 :80-84 —— workspace / unresolved / agent。
        assert_eq!(lane_class_of(WORKSPACE_LANE_ID), LaneClass::Workspace);
        assert_eq!(lane_class_of(UNKNOWN_LANE_ID), LaneClass::Unresolved);
        assert_eq!(lane_class_of("研究员1"), LaneClass::Agent);
        // 未知 id 默认是 agent（真源 :83）。
        assert_eq!(lane_class_of(""), LaneClass::Agent);
    }

    #[test]
    fn synthetic_lanes_are_workspace_and_unknown_only() {
        assert!(is_synthetic_lane_id(WORKSPACE_LANE_ID));
        assert!(is_synthetic_lane_id(UNKNOWN_LANE_ID));
        assert!(!is_synthetic_lane_id("agent-1"));
        // sink 是终端标记节点，不是车道。
        assert!(!is_synthetic_lane_id(SINK_NODE_ID));
    }

    #[test]
    fn unphased_and_implicit_phase_ids_are_distinct() {
        // ★真源 :66-71 —— unphased 是「首个标记之前」，
        // implicit（workflow）是「整个工作流」。两者不能混。
        assert_ne!(UNPHASED_PHASE_ID, IMPLICIT_PHASE_ID);
        assert_eq!(UNPHASED_PHASE_ID, "unphased");
        assert_eq!(IMPLICIT_PHASE_ID, "workflow");
    }

    #[test]
    fn selection_kind_as_str() {
        assert_eq!(
            WorkflowGraphSelectionKind::Participant.as_str(),
            "participant"
        );
        assert_eq!(WorkflowGraphSelectionKind::Phase.as_str(), "phase");
        assert_eq!(WorkflowGraphSelectionKind::Sink.as_str(), "sink");
    }

    #[test]
    fn multiword_fields_use_camel_case_on_the_wire() {
        // ★回归——`phaseEdges` / `labelPattern` / `namePattern` 曾经漏了 rename，
        // serde 不报错，只是把那一项读成 `None`：整张图的边集合静默变空，
        // 所有「没有轨道段」的断言照样通过。这里逐项钉住。
        let raw = serde_json::json!({
            "steps": [{
                "id": "s1", "kind": "ask", "label": "问", "lane": "l1",
                "labelPattern": { "head": "研究员", "tail": "号" }
            }],
            "lanes": [{ "id": "l1", "namePattern": { "head": "研究员" } }],
            "participants": [],
            "handoffs": [],
            "phaseEdges": [{ "from": "p1", "to": "p2" }]
        });
        let g: WorkflowCausalityGraphData = serde_json::from_value(raw).unwrap();
        assert_eq!(
            g.phase_edges.as_ref().unwrap().len(),
            1,
            "★phaseEdges 必须读进来"
        );
        assert_eq!(
            g.steps[0].label_pattern.as_ref().unwrap().head.as_deref(),
            Some("研究员")
        );
        assert_eq!(
            g.lanes[0].name_pattern.as_ref().unwrap().head.as_deref(),
            Some("研究员")
        );
    }

    #[test]
    fn graph_payload_round_trips_through_json() {
        // 因果图走 serde：display 的 causalityGraph 是原始 JSON，
        // 这里确认字段名与 zod 一致（camelCase + strict）。
        let raw = serde_json::json!({
            "steps": [{
                "id": "s1", "kind": "ask", "label": "问研究员",
                "lane": "agent-1", "phase": "preflight"
            }],
            "lanes": [{ "id": "agent-1", "name": "研究员" }],
            "participants": [{
                "id": "p1", "phase": "preflight", "lane": "agent-1",
                "steps": ["s1"]
            }],
            "handoffs": [{ "from": "p1", "to": "p2" }],
            "phases": [{ "id": "preflight", "name": "预备", "alongside": ["main"] }],
            "phaseEdges": [{ "from": "preflight", "to": "main" }],
            "exits": ["main"],
            "sink": ["s1"]
        });
        let graph: WorkflowCausalityGraphData = serde_json::from_value(raw).unwrap();
        assert_eq!(graph.steps.len(), 1);
        assert_eq!(graph.steps[0].kind, "ask");
        assert_eq!(graph.steps[0].phase.as_deref(), Some("preflight"));
        assert_eq!(graph.lanes[0].name.as_deref(), Some("研究员"));
        assert_eq!(graph.participants[0].steps, vec!["s1"]);
        assert_eq!(graph.handoffs[0].to, "p2");
        // phases / phaseEdges / exits / sink 全有或全无。
        assert_eq!(graph.phases.as_ref().unwrap().len(), 1);
        assert_eq!(graph.exits.as_deref(), Some(&["main".to_string()][..]));
    }

    #[test]
    fn graph_payload_allows_absent_optional_blocks() {
        // ★真源 :106-108 —— 零标记脚本 phases/phaseEdges/exits 全缺席。
        let raw = serde_json::json!({
            "steps": [],
            "lanes": [],
            "participants": [],
            "handoffs": []
        });
        let graph: WorkflowCausalityGraphData = serde_json::from_value(raw).unwrap();
        assert!(graph.phases.is_none());
        assert!(graph.phase_edges.is_none());
        assert!(graph.exits.is_none());
        assert!(graph.sink.is_none());
        assert!(graph.truncated.is_none());
    }

    #[test]
    fn name_pattern_is_optional_and_camel_free() {
        // 真源 :22-24 —— head / tail 都是可选。
        let raw = serde_json::json!({ "head": "研究员" });
        let p: NamePattern = serde_json::from_value(raw).unwrap();
        assert_eq!(p.head.as_deref(), Some("研究员"));
        assert_eq!(p.tail, None);
        // 空对象也合法（契约上 `{}` 能过 .strict()）。
        let empty: NamePattern = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(empty.head, None);
    }

    #[test]
    fn participant_member_and_many_are_optional() {
        let with_member = serde_json::json!({
            "id": "p1", "phase": "ph", "lane": "l",
            "steps": ["s1"], "member": { "index": 0, "of": 3 }
        });
        let p: WorkflowParticipantData = serde_json::from_value(with_member).unwrap();
        assert_eq!(p.member.unwrap().of, 3);

        let many = serde_json::json!({
            "id": "p2", "phase": "ph", "lane": "l",
            "steps": ["s1"], "many": true
        });
        let p2: WorkflowParticipantData = serde_json::from_value(many).unwrap();
        assert_eq!(p2.many, Some(true));
    }

    #[test]
    fn handoff_types_are_inspection_only() {
        // 真源 :99 —— types 是跨越交接的产物类型（检视器素材，不上箭头）。
        let raw = serde_json::json!({
            "from": "p1", "to": "p2", "types": ["artifact"]
        });
        let h: WorkflowHandoffData = serde_json::from_value(raw).unwrap();
        assert_eq!(h.types.as_deref(), Some(&["artifact".to_string()][..]));
        // 阶段边没有 types。
        let e: WorkflowEdge = serde_json::from_value(serde_json::json!({ "from": "a", "to": "b" })).unwrap();
        assert_eq!(e.back, None);
    }
}