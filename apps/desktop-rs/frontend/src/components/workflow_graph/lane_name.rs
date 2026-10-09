//! 1:1 翻译 `packages/ui/src/components/workflow-graph/lane-name.ts`（123 行）。
//!
//! 车道显示名的唯一策略点。

use std::collections::HashMap;

use super::name_pattern::format_name_pattern;
use super::types::{LaneClass, NamePattern, WorkflowLaneData, lane_class_of};

/// 决定一条车道显示名所需的全部输入——不含 id：id 是身份，从不参与命名。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaneNaming {
    pub lane_class: Option<LaneClass>,
    /// 脚本里 `agent("planner")` 给出的名字；分析拿不到字面量时缺席。
    pub name: Option<String>,
    /// 名字是插值出来的时，模板两端的字面量。
    ///
    /// ★真源 :33-36 注释——排在 `name` 之后、匿名兜底之前：
    /// 它比「未命名智能体」多说了一件真事，又不是作者原样写下的那个词。
    pub name_pattern: Option<NamePattern>,
    /// 同一张图里并存多条匿名 agent 车道时的 1-based 序号；唯一一条时缺席。
    pub anonymous_index: Option<i64>,
}

/// `LaneNameFormatter`（真源 :47-52）：`formatMessage` 的最小形状。
///
/// helper 只要这个函数，不碰 React，于是能用假 formatter 单测策略本身。
pub type LaneNameFormatter<'a> = &'a dyn Fn(&str) -> String;

/// 对一条车道的引用：身份（`id`）加上做显示名所需的全部素材。
///
/// ★真源 :85-87 注释——**从不是拼好的显示串**：匿名兜底文案在渲染时才由
/// `lane_display_name` 成型（见文件头那段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneRef {
    pub id: String,
    pub naming: LaneNaming,
}

/// 合成车道的固定文案 id（真源 :56-59）。
///
/// ★真源 :55 注释——合成车道不靠名字识别：它们是「什么在这里跑」，文案固定。
fn name_id_by_class(lane_class: LaneClass) -> Option<&'static str> {
    match lane_class {
        LaneClass::Unresolved => Some("chat.toolCall.workflow.graph.lane.unresolved"),
        LaneClass::Workspace => Some("chat.toolCall.workflow.graph.lane.script"),
        // agent 车道没有固定文案（走作者原词 / pattern / 匿名兜底）。
        LaneClass::Agent => None,
    }
}

/// `laneDisplayName`（真源 :61-82）。
///
/// 四级：合成固定文案 > 作者原词 > pattern 形状 > 匿名兜底。
///
/// ★真源 :17-24 注释——站点 id（`actor#3`）是**身份**，显示名是**表现**。
/// 两者混在一起时界面上就会出现一行叫「actor#3」的车道——那是把 key 当名字用。
/// 名字的来源只有两个：脚本里写下的名字（原样显示，本地化它就是改作者的话），
/// 以及分析拿不到名字时的本地化兜底。
pub fn lane_display_name(lane: &LaneNaming, format_message: LaneNameFormatter<'_>) -> String {
    if let Some(class) = lane.lane_class {
        if let Some(id) = name_id_by_class(class) {
            return format_message(id);
        }
    }
    // 作者原词原样显示。
    if let Some(name) = &lane.name {
        return name.clone();
    }
    // 名字是插值出来的：形状仍然是作者的话，所以同样不本地化，只是补一个省略号。
    if let Some(patterned) = format_name_pattern(lane.name_pattern.as_ref()) {
        return patterned;
    }
    // ★真源 :78-81 —— 编号只在需要区分时才出现：
    // 图里只有一条匿名车道，「未命名智能体 1」里的 1 什么也没说。
    match lane.anonymous_index {
        None => format_message("chat.toolCall.workflow.graph.lane.anonymous"),
        Some(index) => format_message("chat.toolCall.workflow.graph.lane.anonymousIndexed")
            .replace("{index}", &index.to_string()),
    }
}

/// `anonymousLaneIndexes`（真源 :99-108）。
///
/// 1-based 序号，按车道顺序发给「没有名字的 agent 车道」，
/// 且**只在并存两条以上时才发**。
///
/// ★真源 :92-98 注释——**带 pattern 的车道算「有名字」**，不进这份计数：
/// 它头上写着「研究员…」，再挂一个匿名序号只会让序号与画面上看得见的
/// 匿名车道数量对不上。两条 pattern 恰好相同时会撞名——那和两条都字面叫
/// `"worker"` 的车道撞名是同一件事，这里不欠新的消歧。
///
/// 计数与语言无关，所以它属于这一层（并且可单测）。
fn anonymous_lane_indexes(lanes: &[WorkflowLaneData]) -> HashMap<String, i64> {
    let anonymous: Vec<&WorkflowLaneData> = lanes
        .iter()
        .filter(|lane| {
            lane.name.is_none()
                && format_name_pattern(lane.name_pattern.as_ref()).is_none()
                && lane_class_of(&lane.id) == LaneClass::Agent
        })
        .collect();
    if anonymous.len() < 2 {
        return HashMap::new();
    }
    anonymous
        .iter()
        .enumerate()
        .map(|(i, lane)| (lane.id.clone(), i as i64 + 1))
        .collect()
}

/// `laneRefsById`（真源 :118-138）：车道 id → 命名素材（**不是显示串**）。
///
/// ★真源 :110-117 注释——三个消费者共用这一份：图的名册、step 卡片的候选车道、
/// 以及 transcript 下钻的实例选择器。**必须同源**——匿名编号是
/// 「图里第几条匿名车道」，各算一次总有一天不相等，表现是选择器里的
/// 「未命名智能体 2」对着图上的「未命名智能体 1」。
pub fn lane_refs_by_id(lanes: &[WorkflowLaneData]) -> HashMap<String, LaneRef> {
    let anonymous_indexes = anonymous_lane_indexes(lanes);
    lanes
        .iter()
        .map(|lane| {
            let naming = LaneNaming {
                lane_class: Some(lane_class_of(&lane.id)),
                name: lane.name.clone(),
                name_pattern: lane.name_pattern.clone(),
                anonymous_index: anonymous_indexes.get(&lane.id).copied(),
            };
            (
                lane.id.clone(),
                LaneRef {
                    id: lane.id.clone(),
                    naming,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::types::{UNKNOWN_LANE_ID, WORKSPACE_LANE_ID};

    /// 假 formatter（真源 :47-52 的用意：能单测策略本身）。
    fn fake(id: &str) -> String {
        match id {
            "chat.toolCall.workflow.graph.lane.unresolved" => "未解析".to_string(),
            "chat.toolCall.workflow.graph.lane.script" => "脚本".to_string(),
            "chat.toolCall.workflow.graph.lane.anonymous" => "未命名智能体".to_string(),
            "chat.toolCall.workflow.graph.lane.anonymousIndexed" => {
                "未命名智能体 {index}".to_string()
            }
            other => format!("[{other}]"),
        }
    }

    fn naming(class: Option<LaneClass>) -> LaneNaming {
        LaneNaming {
            lane_class: class,
            ..Default::default()
        }
    }

    #[test]
    fn synthetic_lanes_use_fixed_wording() {
        // 真源 :62-63 —— workspace / unresolved 不靠名字识别，文案固定。
        assert_eq!(
            lane_display_name(&naming(Some(LaneClass::Workspace)), &fake),
            "脚本"
        );
        assert_eq!(
            lane_display_name(&naming(Some(LaneClass::Unresolved)), &fake),
            "未解析"
        );
    }

    #[test]
    fn synthetic_wording_overrides_author_name() {
        // 真源 :62 —— 合成车道的固定文案优先于作者原词。
        let mut lane = naming(Some(LaneClass::Workspace));
        lane.name = Some("我的脚本".into());
        assert_eq!(lane_display_name(&lane, &fake), "脚本");
    }

    #[test]
    fn author_name_wins_over_pattern() {
        // 真源 :65-66 —— 原词原样显示：本地化它就是改作者的话。
        let mut lane = naming(Some(LaneClass::Agent));
        lane.name = Some("planner".into());
        lane.name_pattern = Some(NamePattern {
            head: Some("x".into()),
            tail: None,
        });
        assert_eq!(lane_display_name(&lane, &fake), "planner");
    }

    #[test]
    fn pattern_used_when_no_literal_name() {
        // 真源 :68-70 —— 形状仍然是作者的话，同样不本地化。
        let mut lane = naming(Some(LaneClass::Agent));
        lane.name_pattern = Some(NamePattern {
            head: Some("研究员".into()),
            tail: None,
        });
        assert_eq!(lane_display_name(&lane, &fake), "研究员…");
    }

    #[test]
    fn anonymous_fallback_without_index() {
        let lane = naming(Some(LaneClass::Agent));
        assert_eq!(lane_display_name(&lane, &fake), "未命名智能体");
    }

    #[test]
    fn anonymous_fallback_with_index() {
        let mut lane = naming(Some(LaneClass::Agent));
        lane.anonymous_index = Some(2);
        assert_eq!(lane_display_name(&lane, &fake), "未命名智能体 2");
    }

    // ── 匿名编号 ──

    fn lane(id: &str, name: Option<&str>, pattern: Option<(&str, &str)>) -> WorkflowLaneData {
        WorkflowLaneData {
            id: id.into(),
            name: name.map(str::to_string),
            name_pattern: pattern.map(|(h, t)| NamePattern {
                head: Some(h.into()),
                tail: Some(t.into()),
            }),
            line: None,
            column: None,
        }
    }

    #[test]
    fn single_anonymous_lane_gets_no_index() {
        // ★真源 :93-94 —— 只有一条匿名车道时，
        // 「未命名智能体 1」里的 1 没有区分对象，只是噪声。
        let lanes = vec![
            lane("a", None, None),
            lane("b", Some("named"), None),
        ];
        let refs = lane_refs_by_id(&lanes);
        assert_eq!(refs.get("a").unwrap().naming.anonymous_index, None);
    }

    #[test]
    fn two_anonymous_lanes_get_1_based_indexes() {
        let lanes = vec![lane("a", None, None), lane("b", None, None)];
        let refs = lane_refs_by_id(&lanes);
        assert_eq!(refs.get("a").unwrap().naming.anonymous_index, Some(1));
        assert_eq!(refs.get("b").unwrap().naming.anonymous_index, Some(2));
    }

    #[test]
    fn patterned_lanes_count_as_named() {
        // ★真源 :92-95 —— 带 pattern 的车道算「有名字」，不进匿名计数。
        let lanes = vec![
            lane("a", None, Some(("研究员", ""))),
            lane("b", None, None),
            lane("c", None, None),
        ];
        let refs = lane_refs_by_id(&lanes);
        assert_eq!(
            refs.get("a").unwrap().naming.anonymous_index, None,
            "有 pattern 的车道不挂匿名序号"
        );
        // b、c 仍然编号，且从 1 起（pattern 那条不占号）。
        assert_eq!(refs.get("b").unwrap().naming.anonymous_index, Some(1));
        assert_eq!(refs.get("c").unwrap().naming.anonymous_index, Some(2));
    }

    #[test]
    fn synthetic_lanes_never_counted_as_anonymous() {
        let lanes = vec![
            lane(WORKSPACE_LANE_ID, None, None),
            lane(UNKNOWN_LANE_ID, None, None),
            lane("a", None, None),
            lane("b", None, None),
        ];
        let refs = lane_refs_by_id(&lanes);
        assert_eq!(refs.get(WORKSPACE_LANE_ID).unwrap().naming.anonymous_index, None);
        assert_eq!(refs.get(UNKNOWN_LANE_ID).unwrap().naming.anonymous_index, None);
        // 只有两条真 agent 车道编号。
        assert_eq!(refs.get("a").unwrap().naming.anonymous_index, Some(1));
        assert_eq!(refs.get("b").unwrap().naming.anonymous_index, Some(2));
    }

    #[test]
    fn lane_ref_carries_identity_and_class() {
        // 真源 :118-138 —— laneRef 是身份 + 命名素材，不是显示串。
        let lanes = vec![lane("agent-1", Some("研究员"), None)];
        let refs = lane_refs_by_id(&lanes);
        let r = &refs.get("agent-1").unwrap();
        assert_eq!(r.id, "agent-1");
        assert_eq!(r.naming.lane_class, Some(LaneClass::Agent));
        assert_eq!(r.naming.name.as_deref(), Some("研究员"));
    }
}