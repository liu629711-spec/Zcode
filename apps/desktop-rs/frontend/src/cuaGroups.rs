//! 1:1 翻译 `packages/ui/src/v4/conversationCuaGroups.ts`（281 行）。
//!
//! CUA 工具调用分组器：把连续的官方 CUA 工具 + 它们的助手正文/思考
//! 收敛成一张「Computer Use」聚合卡（`CuaGroup`）。
//!
//! 与 `conversationWorkItems.rs`（explore/terminal/changes 分组）同层，
//! 但本组的判据更复杂——需要**先按 responseId 预分类**才能决定
//! 某段正文是否属于 CUA 组。

use crate::ToolCallBlocks::toolCallRowAdapter::{LegacyToolCall, LegacyToolCallNode};
use crate::conversationWorkItems::GroupingRow;

// ---------------------------------------------------------------------------
// 常量（真源 :22-27）
// ---------------------------------------------------------------------------

/// `OFFICIAL_CUA_TOOL_PREFIXES`（真源 :22-25）。
///
/// 注意这里是**连字符** `computer-use`，与 `cua.rs` 的 `computer_use`（下划线）
/// 不同——真源两处命名空间写法确实不一致（前者给 CUA server，
/// 后者给 plugin namespace）。**不做归一化**，照抄。
pub const OFFICIAL_CUA_TOOL_PREFIXES: [&str; 2] = [
    "mcp__computer-use__",
    "mcp__plugin_zcode-cua_computer-use__",
];

/// `ENABLE_CUA_TOOL_CALL_GROUPING`（真源 :27）——真源硬编码 true。
pub const ENABLE_CUA_TOOL_CALL_GROUPING: bool = true;

/// `isOfficialCuaToolCallRow`（真源 :33-38）。
pub fn is_official_cua_tool_call_row(row: &GroupingRow) -> bool {
    row.is_tool_call()
        && row
            .tool_name
            .as_deref()
            .is_some_and(|n| OFFICIAL_CUA_TOOL_PREFIXES.iter().any(|p| n.starts_with(p)))
}

// ---------------------------------------------------------------------------
// 分组事件与渲染项（真源 :7-30）
// ---------------------------------------------------------------------------

/// 分组内的事件（真源 :7-10）。
#[derive(Debug, Clone, PartialEq)]
pub enum CuaGroupEvent {
    Tool {
        row_index: usize,
        node: LegacyToolCallNode,
    },
    /// 助手正文行（真源 :8）。Rust 侧携带原行下标。
    AssistantMessage { row_index: usize },
    /// 思考行（真源 :9）。
    Reasoning { row_index: usize },
}

/// 流程段类别（真源 :26 `flowKind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuaFlowKind {
    AssistantHistory,
    AssistantWork,
}

impl CuaFlowKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::AssistantHistory => "assistantHistory",
            Self::AssistantWork => "assistantWork",
        }
    }
}

/// `ConversationCuaGroupRenderItem`（真源 :12-27）。
#[derive(Debug, Clone, PartialEq)]
pub struct CuaGroupRenderItem {
    pub key: String,
    pub row_id: i64,
    /// 组内 ToolCall 行在原数组中的下标（真源 `rows: ToolCallRow[]`）。
    pub row_indices: Vec<usize>,
    pub events: Vec<CuaGroupEvent>,
    pub node: LegacyToolCallNode,
    pub active: bool,
    pub flow_kind: CuaFlowKind,
    pub assistant_response_ids: Vec<String>,
}

/// `buildCuaGroup`（真源 :42-80）。
pub fn build_cua_group(
    events: Vec<CuaGroupEvent>,
    row_indices: Vec<usize>,
    active: bool,
    flow_kind: CuaFlowKind,
    assistant_response_id: Option<&str>,
    anchor: Option<(&GroupingRow, usize)>,
) -> CuaGroupRenderItem {
    // 真源 :45-47 —— 行与虚拟节点的身份都优先用 responseId。
    let first_row_id = anchor.map(|(r, _)| r.row_id).unwrap_or(0);
    let first_tool_call_id = anchor
        .map(|(r, _)| r.tool_call_id.clone().unwrap_or_default())
        .unwrap_or_default();

    let identity = match assistant_response_id {
        Some(id) => format!("cua-response:{id}"),
        None => format!("cua:{first_row_id}"),
    };
    // ★真源 :62 —— 有 responseId 时虚拟 toolId 就是 identity 本身。
    let virtual_tool_id = match assistant_response_id {
        Some(_) => identity.clone(),
        None => format!("cua:{first_tool_call_id}"),
    };

    let children: Vec<LegacyToolCallNode> = events
        .iter()
        .filter_map(|e| match e {
            CuaGroupEvent::Tool { node, .. } => Some(node.clone()),
            _ => None,
        })
        .collect();

    CuaGroupRenderItem {
        key: identity,
        row_id: first_row_id,
        row_indices,
        node: LegacyToolCallNode {
            tool_call: LegacyToolCall {
                tool_id: virtual_tool_id,
                tool_name: Some("CuaGroup".to_string()),
                kind: "cuaGroup".to_string(),
                title: Some("Computer Use".to_string()),
                input: Some(serde_json::json!({})),
                status: if active { "in_progress" } else { "completed" }.to_string(),
                v4_status: if active { "running" } else { "success" }.to_string(),
                output: None,
                content: None,
                error: None,
                raw: serde_json::json!({}),
                started_at: anchor.and_then(|(r, _)| r.started_at),
                snapshot_refs: Vec::new(),
                thought: None,
            },
            child_tool_calls: children,
        },
        events,
        active,
        flow_kind,
        assistant_response_ids: assistant_response_id
            .map(|id| vec![id.to_string()])
            .unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// 预分类（真源 :82-111）
// ---------------------------------------------------------------------------

/// `ResponseClassification`（真源 :82-86）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseClassification {
    pub has_official_cua: bool,
    pub has_non_cua_tool: bool,
    ///首个官方 CUA 工具行在原数组中的下标。
    pub first_cua_row_index: Option<usize>,
}

/// `classifyResponses`（真源 :88-111）。
///
/// 按 `assistant_response_id` 归类：这次响应里有没有官方 CUA 工具、
/// 有没有别的工具。只有「**只有** CUA、没有其他工具」的响应，
/// 它的正文与思考才收进 CUA 组（真源 :200/:218）。
pub fn classify_responses(
    rows: &[GroupingRow],
) -> std::collections::HashMap<String, ResponseClassification> {
    let mut classifications: std::collections::HashMap<String, ResponseClassification> =
        std::collections::HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        if !row.is_tool_call() {
            continue;
        }
        let Some(response_id) = row.assistant_response_id.as_deref() else {
            continue;
        };
        let current = classifications
            .entry(response_id.to_string())
            .or_default();
        if is_official_cua_tool_call_row(row) {
            current.has_official_cua = true;
            if current.first_cua_row_index.is_none() {
                current.first_cua_row_index = Some(index);
            }
        } else {
            current.has_non_cua_tool = true;
        }
    }
    classifications
}

// ---------------------------------------------------------------------------
// 主流程（真源 :121-248）
// ---------------------------------------------------------------------------

/// 分组开关（真源 :122）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CuaGroupOptions {
    pub enabled: bool,
    /// 阶段尾部是否仍在运行（决定末尾的 activeGroup 是否收尾）。
    pub stage_tail_is_running: bool,
}

/// 分组器内部可变状态（对应真源闭包里的 `activeGroup`）。
struct GroupState {
    group: Option<CuaGroupRenderItem>,
    /// 组在 `prepared` 中的位置（用于回改active）。
    slot: usize,
}

/// `prepareCuaGroupFlowItems`（真源 :121-248）。
///
/// 返回 `prepared` 的元素序列：`(usize, FlowEntry)`，其中 `usize` 是
/// 该段的首行在原数组中的下标（用于渲染层取原行）。
/// `FlowEntry` 与真源 `ConversationTurnFlowItem` 同构的三种形态简化成
/// 「一组行下标 + 类别」。
#[derive(Debug, Clone, PartialEq)]
pub enum PreparedEntry {
    /// 一段同类的行（真源 `assistantHistory` / `assistantWork` 的 `rows`）。
    Rows {
        flow_kind: CuaFlowKind,
        row_indices: Vec<usize>,
    },
    /// 一个 CUA 聚合卡（真源 `cuaGroup`）。
    CuaGroup(CuaGroupRenderItem),
    /// 独立正文行（真源 `assistantText`，旧路径兼容用）。
    AssistantText { row_index: usize },
}

/// `appendFlowRow`（真源 :113-118）：同类相邻则并入上一段，否则新起一段。
fn append_flow_row(prepared: &mut Vec<PreparedEntry>, kind: CuaFlowKind, row_index: usize) {
    if let Some(PreparedEntry::Rows {
        flow_kind,
        row_indices,
    }) = prepared.last_mut()
    {
        if *flow_kind == kind {
            row_indices.push(row_index);
            return;
        }
    }
    prepared.push(PreparedEntry::Rows {
        flow_kind: kind,
        row_indices: vec![row_index],
    });
}

/// `prepareCuaGroups`（真源 :253-280）：旧路径——单一 assistantWork 段的扁平版本。
///
/// 输出按行下标排序的 `PreparedEntry` 列表。
pub fn prepare_cua_groups(
    rows: &[GroupingRow],
    options: CuaGroupOptions,
) -> Vec<PreparedEntry> {
    prepare_cua_group_flow_items(rows, options)
}

/// `prepareCuaGroupFlowItems`（真源 :121-248）的 Rust 版。
///
/// Rust 侧输入是扁平行数组（真源的 `sourceItems` 是多段结构），
/// 按 `assistantWork` 一段处理——与真源 `prepareCuaGroups` 兼容路径
/// （真源 :254-256）语义一致。
pub fn prepare_cua_group_flow_items(
    rows: &[GroupingRow],
    options: CuaGroupOptions,
) -> Vec<PreparedEntry> {
    if !options.enabled {
        return vec![PreparedEntry::Rows {
            flow_kind: CuaFlowKind::AssistantWork,
            row_indices: (0..rows.len()).collect(),
        }];
    }

    let prepared: Vec<PreparedEntry> = Vec::new();
    let classifications = classify_responses(rows);
    let mut state = GroupState {
        group: None,
        slot: 0,
    };
    let mut active_response_ids: Vec<String> = Vec::new();
    let mut active_row_indices: Vec<usize> = Vec::new();
    let mut active_events: Vec<CuaGroupEvent> = Vec::new();
    let mut active_node_children: Vec<LegacyToolCallNode> = Vec::new();
    let mut out = prepared;

    /// `closeGroup`（真源 :130-135）：收尾当前组（真源靠改对象引用，
    /// Rust 侧因为值语义改成「取出 + 写回slot」）。
    fn close_group(
        state: &mut GroupState,
        out: &mut Vec<PreparedEntry>,
        active_row_indices: &mut Vec<usize>,
        active_events: &mut Vec<CuaGroupEvent>,
        active_node_children: &mut Vec<LegacyToolCallNode>,
        active_response_ids: &mut Vec<String>,
    ) {
        if state.group.is_none() {
            return;
        }
        let mut group = state.group.take().unwrap();
        group.active = false;
        group.node.tool_call.status = "completed".to_string();
        group.row_indices = std::mem::take(active_row_indices);
        group.events = std::mem::take(active_events);
        group.assistant_response_ids = std::mem::take(active_response_ids);
        group.node.child_tool_calls = std::mem::take(active_node_children);
        if let Some(slot) = out.get_mut(state.slot) {
            *slot = PreparedEntry::CuaGroup(group);
        }
    }

    /// `ensureGroup`（真源 :136-152）：有活动组就复用，否则新建。
    #[allow(clippy::too_many_arguments)]
    fn ensure_group(
        state: &mut GroupState,
        out: &mut Vec<PreparedEntry>,
        flow_kind: CuaFlowKind,
        assistant_response_id: Option<&str>,
        anchor: Option<(&GroupingRow, usize)>,
        active_response_ids: &mut Vec<String>,
    ) {
        if state.group.is_some() {
            // 真源 :137-143 ——复用已有活动组，只补记新的 responseId。
            if let Some(id) = assistant_response_id {
                let already = state
                    .group
                    .as_ref()
                    .is_some_and(|g| g.assistant_response_ids.iter().any(|x| x == id));
                if !already {
                    active_response_ids.push(id.to_string());
                }
            }
            return;
        }
        let group = build_cua_group(
            Vec::new(),
            Vec::new(),
            true,
            flow_kind,
            assistant_response_id,
            anchor,
        );
        state.slot = out.len();
        if let Some(id) = assistant_response_id {
            active_response_ids.push(id.to_string());
        }
        state.group = Some(group);
        out.push(PreparedEntry::CuaGroup(
            state.group.clone().expect("刚建组"),
        ));
    }

    for (index, row) in rows.iter().enumerate() {
        if row.is_tool_call() {
            if is_official_cua_tool_call_row(row) {
                // `appendCua`（真源 :153-162）。
                let node = crate::conversationWorkItems::legacy_of(row);
                ensure_group(
                    &mut state,
                    &mut out,
                    CuaFlowKind::AssistantWork,
                    row.assistant_response_id.as_deref(),
                    Some((row, index)),
                    &mut active_response_ids,
                );
                active_events.push(CuaGroupEvent::Tool {
                    row_index: index,
                    node: node.clone(),
                });
                active_row_indices.push(index);
                active_node_children.push(node);
            } else {
                // 非 CUA 工具：关闭活动组，原位展示（真源 :242-244）。
                close_group(
                    &mut state,
                    &mut out,
                    &mut active_row_indices,
                    &mut active_events,
                    &mut active_node_children,
                    &mut active_response_ids,
                );
                append_flow_row(&mut out, CuaFlowKind::AssistantWork, index);
            }
            continue;
        }

        match row.kind.as_str() {
            "assistantText" => {
                let classification = row
                    .assistant_response_id
                    .as_deref()
                    .and_then(|id| classifications.get(id));
                let belongs = classification
                    .is_some_and(|c| c.has_official_cua && !c.has_non_cua_tool);
                if belongs {
                    // `appendAssistantMessage`（真源 :163-169）。
                    let anchor_index = classification
                        .and_then(|c| c.first_cua_row_index)
                        .and_then(|i| rows.get(i));
                    let anchor = anchor_index.map(|r| (r, classification.unwrap().first_cua_row_index.unwrap()));
                    ensure_group(
                        &mut state,
                        &mut out,
                        CuaFlowKind::AssistantWork,
                        row.assistant_response_id.as_deref(),
                        anchor,
                        &mut active_response_ids,
                    );
                    active_events.push(CuaGroupEvent::AssistantMessage { row_index: index });
                } else {
                    // ★真源 :203-210 —— streaming 且未分类时**不关闭**活动组：
                    // 没有 tool 事实前提前收纳，会让正文在滚动摘要出现后被移回外部。
                    let streaming_unclassified = row.status == "streaming"
                        && row.assistant_response_id.is_some()
                        && classification.is_none();
                    if !streaming_unclassified {
                        close_group(
                            &mut state,
                            &mut out,
                            &mut active_row_indices,
                            &mut active_events,
                            &mut active_node_children,
                            &mut active_response_ids,
                        );
                    }
                    append_flow_row(&mut out, CuaFlowKind::AssistantWork, index);
                }
            }
            "reasoning" => {
                let classification = row
                    .assistant_response_id
                    .as_deref()
                    .and_then(|id| classifications.get(id));
                let belongs = classification
                    .is_some_and(|c| c.has_official_cua && !c.has_non_cua_tool);
                if belongs {
                    let first = classification.and_then(|c| c.first_cua_row_index);
                    let anchor = first.and_then(|i| rows.get(i).map(|r| (r, i)));
                    ensure_group(
                        &mut state,
                        &mut out,
                        CuaFlowKind::AssistantWork,
                        row.assistant_response_id.as_deref(),
                        anchor,
                        &mut active_response_ids,
                    );
                    active_events.push(CuaGroupEvent::Reasoning { row_index: index });
                } else {
                    // ★真源 :221-229 —— reasoning 往往先于正文和工具完成；
                    // 仅凭「当前没有 tool」就关组会把随后确认的纯 CUA response 错切。
                    // 未分类阶段与旧无 ID 数据都只原位展示。
                    close_group(
                        &mut state,
                        &mut out,
                        &mut active_row_indices,
                        &mut active_events,
                        &mut active_node_children,
                        &mut active_response_ids,
                    );
                    append_flow_row(&mut out, CuaFlowKind::AssistantWork, index);
                }
            }
            _ => {
                // marker / todo / status 只在组外展示，不构成 response 工具边界
                // （真源 :250-252）。
                close_group(
                    &mut state,
                    &mut out,
                    &mut active_row_indices,
                    &mut active_events,
                    &mut active_node_children,
                    &mut active_response_ids,
                );
                append_flow_row(&mut out, CuaFlowKind::AssistantWork, index);
            }
        }
    }

    // 真源 :245 —— 尾部仍在运行则保留 active，否则收尾。
    if !options.stage_tail_is_running {
        close_group(
            &mut state,
            &mut out,
            &mut active_row_indices,
            &mut active_events,
            &mut active_node_children,
            &mut active_response_ids,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(index: i64, name: &str, response_id: Option<&str>) -> GroupingRow {
        GroupingRow {
            row_id: index,
            kind: "toolCall".into(),
            tool_name: Some(name.into()),
            tool_call_id: Some(format!("tc-{index}")),
            status: "completed".into(),
            input: json!({}),
            input_text: String::new(),
            started_at: None,
            turn_id: None,
            parent_tool_call_id: None,
            assistant_response_id: response_id.map(str::to_string),
        }
    }

    fn assistant_text(index: i64, response_id: Option<&str>, status: &str) -> GroupingRow {
        GroupingRow {
            row_id: index,
            kind: "assistantText".into(),
            tool_name: None,
            tool_call_id: None,
            status: status.into(),
            input: json!({}),
            input_text: "正文".into(),
            started_at: None,
            turn_id: None,
            parent_tool_call_id: None,
            assistant_response_id: response_id.map(str::to_string),
        }
    }

    fn reasoning(index: i64, response_id: Option<&str>) -> GroupingRow {
        GroupingRow {
            row_id: index,
            kind: "reasoning".into(),
            tool_name: None,
            tool_call_id: None,
            status: "completed".into(),
            input: json!({}),
            input_text: String::new(),
            started_at: None,
            turn_id: None,
            parent_tool_call_id: None,
            assistant_response_id: response_id.map(str::to_string),
        }
    }

    const ON: CuaGroupOptions = CuaGroupOptions {
        enabled: true,
        stage_tail_is_running: false,
    };

    fn group_of<'a>(items: &'a [PreparedEntry], idx: usize) -> &'a CuaGroupRenderItem {
        match &items[idx] {
            PreparedEntry::CuaGroup(g) => g,
            other => panic!("第 {idx} 项不是 CUA 组：{other:?}"),
        }
    }

    // ── 前缀判定 ──

    #[test]
    fn official_prefix_uses_hyphen_not_underscore() {
        // ★真源 :22-25 用连字符 computer-use，与 cua.rs 的下划线命名空间不同。
        assert!(is_official_cua_tool_call_row(&tool(
            0,
            "mcp__computer-use__left_click",
            None
        )));
        assert!(is_official_cua_tool_call_row(&tool(
            0,
            "mcp__plugin_zcode-cua_computer-use__type",
            None
        )));
    }

    #[test]
    fn underscore_variant_is_not_official() {
        // 真源不认下划线形式——那是 cua.rs 的另一套命名空间，不做归一化。
        assert!(!is_official_cua_tool_call_row(&tool(
            0,
            "mcp__computer_use__left_click",
            None
        )));
    }

    #[test]
    fn non_tool_rows_are_never_official() {
        let mut row = tool(0, "mcp__computer-use__x", None);
        row.kind = "assistantText".into();
        assert!(!is_official_cua_tool_call_row(&row));
    }

    #[test]
    fn grouping_enabled_constant_matches_source() {
        assert!(ENABLE_CUA_TOOL_CALL_GROUPING);
    }

    // ── 预分类 ──

    #[test]
    fn classify_splits_cua_and_non_cua() {
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            tool(1, "Read", Some("r1")),
        ];
        let map = classify_responses(&rows);
        let c1 = &map["r1"];
        assert!(c1.has_official_cua);
        assert!(c1.has_non_cua_tool);
        assert_eq!(c1.first_cua_row_index, Some(0), "记住首个 CUA 行下标");
    }

    #[test]
    fn classify_ignores_rows_without_response_id() {
        let rows = vec![tool(0, "mcp__computer-use__left_click", None)];
        assert!(classify_responses(&rows).is_empty(), "无 responseId 不分类");
    }

    #[test]
    fn classify_records_first_cua_row_only() {
        let rows = vec![
            tool(0, "Read", Some("r1")),
            tool(1, "mcp__computer-use__type", Some("r1")),
            tool(2, "mcp__computer-use__key", Some("r1")),
        ];
        let map = classify_responses(&rows);
        assert_eq!(map["r1"].first_cua_row_index, Some(1), "只记首个");
    }

    // ── 主流程 ──

    #[test]
    fn disabled_returns_single_block() {
        let rows = vec![tool(0, "mcp__computer-use__left_click", Some("r1"))];
        let items = prepare_cua_groups(
            &rows,
            CuaGroupOptions {
                enabled: false,
                stage_tail_is_running: false,
            },
        );
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], PreparedEntry::Rows { .. }));
    }

    #[test]
    fn consecutive_cua_tools_merge_into_one_group() {
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            tool(1, "mcp__computer-use__type", Some("r1")),
        ];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(items.len(), 1, "连续 CUA 工具合成一个组");
        let g = group_of(&items, 0);
        assert_eq!(g.row_indices, vec![0, 1]);
        assert_eq!(g.assistant_response_ids, vec!["r1".to_string()]);
        assert_eq!(g.node.child_tool_calls.len(), 2);
    }

    #[test]
    fn group_key_uses_response_id_when_present() {
        // 真源 :61-64 —— 有 responseId 时 key 为 cua-response:<id>。
        let rows = vec![tool(0, "mcp__computer-use__left_click", Some("resp-9"))];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(group_of(&items, 0).key, "cua-response:resp-9");
        assert_eq!(
            group_of(&items, 0).node.tool_call.tool_id,
            "cua-response:resp-9",
            "虚拟 toolId 也用 responseId"
        );
    }

    #[test]
    fn group_key_falls_back_to_row_id_without_response() {
        let rows = vec![tool(7, "mcp__computer-use__left_click", None)];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(group_of(&items, 0).key, "cua:7");
        assert_eq!(
            group_of(&items, 0).node.tool_call.tool_id,
            "cua:tc-7",
            "无 responseId 时用 toolCallId"
        );
    }

    #[test]
    fn non_cua_tool_closes_group() {
        // 真源 :242-244 —— 非 CUA 工具关闭活动组并原位展示。
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            tool(1, "Read", Some("r2")),
        ];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(items.len(), 2, "组 + 独立行");
        assert!(!group_of(&items, 0).active, "被新工具关闭");
        assert_eq!(
            group_of(&items, 0).node.tool_call.status, "completed",
            "关闭时状态转completed"
        );
        assert!(matches!(items[1], PreparedEntry::Rows { .. }));
    }

    #[test]
    fn pure_cua_response_absorbs_its_assistant_text() {
        // 真源 :200 —— 响应只有 CUA 工具时，正文收进组。
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            assistant_text(1, Some("r1"), "complete"),
        ];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(items.len(), 1, "正文被收进组，不额外成段");
        let g = group_of(&items, 0);
        assert!(matches!(
            g.events.last().unwrap(),
            CuaGroupEvent::AssistantMessage { row_index: 1 }
        ));
    }

    #[test]
    fn mixed_response_keeps_text_outside() {
        // 真源 :200 —— 混了其他工具的响应，正文留在组外。
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            tool(1, "Read", Some("r1")),
            assistant_text(2, Some("r1"), "complete"),
        ];
        let items = prepare_cua_groups(&rows, ON);
        // 组（仅含 CUA 工具） + 组外的工具行与正文段
        assert!(items.len() >= 2);
        assert_eq!(group_of(&items, 0).row_indices, vec![0]);
    }

    #[test]
    fn streaming_unclassified_text_does_not_close_group() {
        // ★真源 :203-210 —— streaming 且未分类时不能关闭活动组，
        // 否则正文在滚动摘要出现后会被移回组外。
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            assistant_text(1, Some("r2"), "streaming"),
        ];
        let items = prepare_cua_groups(
            &rows,
            CuaGroupOptions {
                enabled: true,
                stage_tail_is_running: true,
            },
        );
        let g = group_of(&items, 0);
        assert!(g.active, "未分类的 streaming 正文不该关闭活动组");
    }

    #[test]
    fn reasoning_before_tools_does_not_close_group() {
        // ★真源 :221-229 —— reasoning 常先于工具完成，
        // 不能仅凭「当前没 tool」就关组。
        let rows = vec![
            reasoning(0, Some("r1")),
            tool(1, "mcp__computer-use__left_click", Some("r1")),
        ];
        let items = prepare_cua_groups(&rows, ON);
        let g = group_of(&items, 0);
        assert!(matches!(
            g.events.first().unwrap(),
            CuaGroupEvent::Reasoning { row_index: 0 }
        ));
        assert_eq!(g.row_indices, vec![1], "工具仍在该组内");
    }

    #[test]
    fn reasoning_absorbed_when_response_is_pure_cua() {
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            reasoning(1, Some("r1")),
        ];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(items.len(), 1);
        assert!(matches!(
            group_of(&items, 0).events.last().unwrap(),
            CuaGroupEvent::Reasoning { .. }
        ));
    }

    #[test]
    fn stage_tail_running_keeps_group_active() {
        let rows = vec![tool(0, "mcp__computer-use__left_click", Some("r1"))];
        let items = prepare_cua_groups(
            &rows,
            CuaGroupOptions {
                enabled: true,
                stage_tail_is_running: true,
            },
        );
        let g = group_of(&items, 0);
        assert!(g.active, "尾部仍在运行时组保持活动");
        assert_eq!(g.node.tool_call.status, "in_progress");
    }

    #[test]
    fn stage_tail_stopped_closes_group() {
        let rows = vec![tool(0, "mcp__computer-use__left_click", Some("r1"))];
        let items = prepare_cua_groups(&rows, ON);
        assert!(!group_of(&items, 0).active);
    }

    #[test]
    fn marker_rows_stay_outside_and_close_group() {
        // 真源 :250-252 —— marker/todo/status 只在组外，不构成边界。
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            tool(1, "TodoWrite", None),
        ];
        let items = prepare_cua_groups(&rows, ON);
        assert!(!group_of(&items, 0).active, "其他工具行也应关闭组");
    }

    #[test]
    fn non_cua_rows_after_group_are_appended_as_block() {
        let rows = vec![
            tool(0, "mcp__computer-use__left_click", Some("r1")),
            assistant_text(1, None, "complete"),
        ];
        let items = prepare_cua_groups(&rows, ON);
        assert_eq!(items.len(), 2);
        match &items[1] {
            PreparedEntry::Rows { row_indices, .. } => assert_eq!(row_indices, &vec![1]),
            other => panic!("应是行段：{other:?}"),
        }
    }

    #[test]
    fn group_node_metadata_matches_source() {
        // 真源 :69-77 —— 虚拟节点的字段值。
        let rows = vec![tool(5, "mcp__computer-use__left_click", Some("r1"))];
        let items = prepare_cua_groups(&rows, ON);
        let g = group_of(&items, 0);
        assert_eq!(g.node.tool_call.tool_name.as_deref(), Some("CuaGroup"));
        assert_eq!(g.node.tool_call.kind, "cuaGroup");
        assert_eq!(g.node.tool_call.title.as_deref(), Some("Computer Use"));
        assert_eq!(g.row_id, 5);
        assert_eq!(g.flow_kind, CuaFlowKind::AssistantWork);
    }

    #[test]
    fn flow_kind_as_str_matches_source() {
        assert_eq!(CuaFlowKind::AssistantHistory.as_str(), "assistantHistory");
        assert_eq!(CuaFlowKind::AssistantWork.as_str(), "assistantWork");
    }

    #[test]
    fn empty_input_yields_empty_output() {
        // 真源：`for (const item of sourceItems)` 不进循环 → 返回空数组。
        // 不造空行段（那会让渲染层多出一个空气泡）。
        let items = prepare_cua_groups(&[], ON);
        assert!(items.is_empty(), "空输入应产出空结果，实际 {:?}", items);
    }

    #[test]
    fn disabled_mode_keeps_single_block_even_when_empty() {
        // 关闭分组时真源 `return [...sourceItems]` —— 单段原样透传，
        // 空输入下就是「一个空段」。
        let items = prepare_cua_groups(
            &[],
            CuaGroupOptions {
                enabled: false,
                stage_tail_is_running: false,
            },
        );
        assert_eq!(items.len(), 1);
        match &items[0] {
            PreparedEntry::Rows { row_indices, .. } => assert!(row_indices.is_empty()),
            other => panic!("应是行段：{other:?}"),
        }
    }
}