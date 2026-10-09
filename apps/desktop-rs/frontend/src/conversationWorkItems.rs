//! 1:1 翻译 `packages/ui/src/v4/conversationAssistantWorkItems.ts`（428 行）。
//!
//! 把 assistant 工作行序列折叠成渲染项：连续同类工具 → 父分组卡
//! （exploreGroup / executeGroup / changesGroup），Agent 工具行 ↔ subagent 行
//! 配对，其余逐行。
//!
//! 模块名沿用真源文件名（camelCase）是刻意约定，故豁免 snake_case 警告。
//!
//! ## Rust 侧开关状态（对齐真源常量语义）
//!
//! | 分组 | 真源默认 | Rust 侧 | 原因 |
//! |---|---|---|---|
//! | explore | true | true | `explore.rs` 已迁（本轮） |
//! | terminal | true | true | `execute_group.rs` 已就绪 |
//! | changes | false | false | 照抄真源默认 |
//!
//! ## 裁剪注明
//!
//! - `prepareCuaGroups`（conversationCuaGroups.ts 281 行）：CUA 分组未迁，
//!   pass-through 替代——CUA 行按普通 toolCall 行渲染（不聚合），与
//!   分组关闭时的真源行为一致；
//! - reasoning 可见性过滤（`isConversationReasoningRowVisible`）：Rust 侧
//!   无 reasoning 折叠控制，恒可见，过滤省略；
//! - 未 claim 的 subagent 行：Rust 侧 subagent 卡未迁，主循环不渲染
//!   （与现状一致），配对逻辑完整迁移（claim 判定影响 Agent 行）。

#![allow(non_snake_case)]

use serde_json::{Value, json};

use super::ToolCallBlocks::toolCallRowAdapter::{
    LegacyToolCall, LegacyToolCallNode, tool_call_row_to_legacy_node,
};

/// 分组器输入行的字段子集（来自 `ConversationRowView` 的投影）。
#[derive(Debug, Clone)]
pub struct GroupingRow {
    pub row_id: i64,
    pub kind: String,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub status: String,
    pub input: Value,
    pub input_text: String,
    pub started_at: Option<f64>,
    pub turn_id: Option<String>,
    pub parent_tool_call_id: Option<String>,
    /// 助手响应 id（真源 assistantResponseId）—— CUA 分组的判据。
    pub assistant_response_id: Option<String>,
}

impl GroupingRow {
    /// 是否是 toolCall 行。
    pub fn is_tool_call(&self) -> bool {
        self.kind == "toolCall"
    }
}

/// 渲染项（真源 `ConversationAssistantWorkRenderItem`，:21-54）。
///
/// Rust 侧 Row 项携带**原行下标**（app.rs 持有原行数组），
/// 分组项携带聚合节点（与真源同构）。
#[derive(Debug, Clone)]
pub enum WorkRenderItem {
    /// 普通行（含未配对 subagent——渲染层自行决定是否显示）。
    Row { key: String, row_index: usize },
    /// Explore 分组（真源 :27-33）。
    ExploreGroup {
        key: String,
        row_id: i64,
        row_indices: Vec<usize>,
        node: LegacyToolCallNode,
    },
    /// 终端分组（真源 :35-41）。
    ExecuteGroup {
        key: String,
        row_id: i64,
        row_indices: Vec<usize>,
        node: LegacyToolCallNode,
    },
    /// 变更分组（真源 :42-48）。
    ChangesGroup {
        key: String,
        row_id: i64,
        row_indices: Vec<usize>,
        node: LegacyToolCallNode,
    },
    /// Agent 工具行 + 配对 subagent（真源 :49-54）。
    AgentToolCall {
        key: String,
        row_index: usize,
        /// 配对的 subagent 行下标（供后续 agent 卡迁移用）。
        subagent_row_index: usize,
    },
    /// CUA 工具聚合组（真源 `ConversationCuaGroupRenderItem`）。
    CuaGroup {
        key: String,
        row_id: i64,
        row_indices: Vec<usize>,
        node: LegacyToolCallNode,
        /// 完整的聚合项（事件序列、active、flowKind 等），
        /// 供`cua_group.rs` 渲染层消费。
        group: crate::cuaGroups::CuaGroupRenderItem,
    },
}

/// 分组开关（真源 `ConversationAssistantWorkRenderOptions`，:61-67）。
#[derive(Debug, Clone)]
pub struct WorkRenderOptions {
    /// 阶段尾部是否仍在运行（决定父分组状态）。
    pub stage_tail_is_running: bool,
    pub enable_explore_grouping: bool,
    pub enable_terminal_grouping: bool,
    pub enable_changes_grouping: bool,
    /// CUA 工具聚合分组（真源 ENABLE_CUA_TOOL_CALL_GROUPING = true）。
    pub enable_cua_grouping: bool,
}

impl Default for WorkRenderOptions {
    fn default() -> Self {
        Self {
            stage_tail_is_running: false,
            // 真源 ENABLE_EXPLORE_TOOL_CALL_GROUPING = true（explore.rs 已迁）。
            enable_explore_grouping: true,
            // 真源 ENABLE_CUA_TOOL_CALL_GROUPING = true（cuaGroups.rs 已迁）。
            enable_cua_grouping: true,
            // 真源 ENABLE_TERMINAL_TOOL_CALL_GROUPING = true。
            enable_terminal_grouping: true,
            // 真源 ENABLE_CHANGES_TOOL_CALL_GROUPING = false。
            enable_changes_grouping: false,
        }
    }
}

/// `SUBAGENT_TOOL_NAMES`（真源 :69）。
const SUBAGENT_TOOL_NAMES: [&str; 3] = ["Agent", "Task", "subagent"];

/// `isAgentToolCallRow`（真源 :73-74）。
fn is_agent_tool_call_row(row: &GroupingRow) -> bool {
    row.is_tool_call()
        && row
            .tool_name
            .as_deref()
            .is_some_and(|n| SUBAGENT_TOOL_NAMES.contains(&n))
}

/// `isExploreToolCallRow`（真源 :76-85）。
fn is_explore_tool_call_row(row: &GroupingRow) -> bool {
    if !row.is_tool_call() {
        return false;
    }
    let node = legacy_of(row);
    super::ToolCallBlocks::exploreToolCall::is_explore_tool_call(
        &node.tool_call.kind,
        node.tool_call.input.as_ref().unwrap_or(&Value::Null),
    )
}

/// `isExecuteToolCallRow`（真源 :87-96）。
fn is_execute_tool_call_row(row: &GroupingRow) -> bool {
    if !row.is_tool_call() {
        return false;
    }
    let node = legacy_of(row);
    super::ToolCallBlocks::exploreToolCall::is_execute_tool_call(
        &node.tool_call.kind,
        node.tool_call.input.as_ref().unwrap_or(&Value::Null),
    )
}

/// `isChangesToolCallRow`（真源 :98-101）：
/// `resolveToolCallIdentity(...).family === "file-write"`。
fn is_changes_tool_call_row(row: &GroupingRow) -> bool {
    if !row.is_tool_call() {
        return false;
    }
    let node = legacy_of(row);
    super::ToolCallBlocks::resolveRenderer::family_by_lower(&node.tool_call.kind)
        == super::ToolCallBlocks::resolveRenderer::ToolFamily::FileWrite
}

/// `shouldDeferUnclassifiedShellToolCall`（真源 :103-112）：
/// 运行中的 shell 工具但命令还没流出来 → 暂不参与分组与阶段判定。
fn should_defer_unclassified_shell_tool_call(row: &GroupingRow) -> bool {
    if !row.is_tool_call() || (row.status != "inputStreaming" && row.status != "running") {
        return false;
    }
    let node = legacy_of(row);
    super::ToolCallBlocks::exploreToolCall::is_shell_tool_call_awaiting_command(
        &node.tool_call.kind,
        node.tool_call.input.as_ref().unwrap_or(&Value::Null),
    )
}

/// `resolveGroupStageStatus`（真源 :114-123）。
///
/// 注释照抄（:118-120）：父节点表达的是**当前工作阶段**，不是子工具状态汇总——
/// 子工具可能已全部完成，但只要当前运行段未出现下一条可见边界，父阶段仍在继续；
/// 反之后续内容已出现时，迟到的 running 也不影响父阶段结束。
fn resolve_group_stage_status(rows: &[&GroupingRow], stage_tail_is_running: bool) -> &'static str {
    if stage_tail_is_running {
        return "in_progress";
    }
    if rows.iter().any(|r| r.status == "cancelled") {
        "stopped"
    } else {
        "completed"
    }
}

/// 从 GroupingRow 造 legacy node（分组判定与子节点都要）。
///
/// CUA 分组器（`cuaGroups.rs`）同样需要把 `GroupingRow` 转成 legacy 子节点，
/// 故开放为 `pub` —— 转换口径必须唯一，两处各造一份会漂移。
pub fn legacy_of(row: &GroupingRow) -> LegacyToolCallNode {
    tool_call_row_to_legacy_node(&json!({
        "kind": "toolCall",
        "rowId": row.row_id,
        "toolCallId": row.tool_call_id.clone().unwrap_or_default(),
        "toolName": row.tool_name.clone(),
        "status": row.status.clone(),
        "inputText": row.input_text,
        "input": row.input.clone(),
        "startedAt": row.started_at,
    }))
}

/// 分组聚合节点里的 toolCall 骨架（真源 `buildExploreGroup` 等三处的公共形状）。
fn group_tool_call(
    tool_id: String,
    tool_name: &str,
    kind: &str,
    title: &str,
    status: &str,
    started_at: Option<f64>,
) -> LegacyToolCall {
    LegacyToolCall {
        tool_id,
        tool_name: Some(tool_name.to_string()),
        kind: kind.to_string(),
        title: Some(title.to_string()),
        input: Some(json!({})),
        status: status.to_string(),
        v4_status: status.to_string(),
        output: None,
        content: None,
        error: None,
        raw: Value::Null,
        started_at,
        snapshot_refs: Vec::new(),
        thought: None,
    }
}

/// `buildExploreGroup`（真源 :125-152）。
///
/// key 锚定**首个子工具**（真源 :133-135 注释）：旧 key 含末尾 row 和数量时，
/// 每新增一个子工具都会重建组件、丢失用户保存的展开状态。
fn build_explore_group(rows: &[&GroupingRow], stage_tail_is_running: bool) -> WorkRenderItem {
    let first = rows[0];
    let status = resolve_group_stage_status(rows, stage_tail_is_running);
    let child_tool_calls = rows.iter().map(|r| legacy_of(r)).collect();
    WorkRenderItem::ExploreGroup {
        key: format!("explore:{}", first.row_id),
        row_id: first.row_id,
        row_indices: Vec::new(), // 由调用方填充（保持函数形状对齐真源）。
        node: LegacyToolCallNode {
            tool_call: group_tool_call(
                format!("explore:{}", first.tool_call_id.clone().unwrap_or_default()),
                "Explore",
                "Explore",
                "Explore",
                status,
                first.started_at,
            ),
            child_tool_calls,
        },
    }
}

/// `buildExecuteGroup`（真源 :154-176）。key 锚定首个子工具（:158-159 同注释）。
fn build_execute_group(rows: &[&GroupingRow], stage_tail_is_running: bool) -> WorkRenderItem {
    let first = rows[0];
    let status = resolve_group_stage_status(rows, stage_tail_is_running);
    let child_tool_calls = rows.iter().map(|r| legacy_of(r)).collect();
    WorkRenderItem::ExecuteGroup {
        key: format!("execute:{}", first.row_id),
        row_id: first.row_id,
        row_indices: Vec::new(),
        node: LegacyToolCallNode {
            tool_call: group_tool_call(
                format!("execute:{}", first.tool_call_id.clone().unwrap_or_default()),
                "ExecuteGroup",
                "executeGroup",
                "Execute",
                status,
                first.started_at,
            ),
            child_tool_calls,
        },
    }
}

/// `buildChangesGroup`（真源 :178-200）。
///
/// 注意状态判定与 Explore/Execute 不同（真源 :193-195）：Changes 是纯 UI 阶段
/// 容器，子项失败/取消只留在各自明细，父级只表达「是否仍在可见运行段尾部」。
fn build_changes_group(rows: &[&GroupingRow], stage_tail_is_running: bool) -> WorkRenderItem {
    let first = rows[0];
    let status = if stage_tail_is_running {
        "in_progress"
    } else {
        "completed"
    };
    let child_tool_calls = rows.iter().map(|r| legacy_of(r)).collect();
    WorkRenderItem::ChangesGroup {
        key: format!("changes:{}", first.row_id),
        row_id: first.row_id,
        row_indices: Vec::new(),
        node: LegacyToolCallNode {
            tool_call: group_tool_call(
                format!("changes:{}", first.tool_call_id.clone().unwrap_or_default()),
                "ChangesGroup",
                "changesGroup",
                "Changes",
                status,
                first.started_at,
            ),
            child_tool_calls,
        },
    }
}

/// subagent 配对结果（真源 `pairSubagentRows` 返回，:209-270）。
struct SubagentPairing {
    subagent_by_agent_tool_row_id: std::collections::HashMap<usize, usize>, // agent row idx -> subagent row idx
    claimed_subagent_row_ids: std::collections::HashSet<usize>,
}

/// `pairSubagentRows`（真源 :209-270）。
///
/// 注释照抄（:202-208）：Agent 工具行与 subagent 行必须按 parentToolCallId
/// 精确配对——同一轮并发 Agent 的 tool call 行按模型输出顺序出现，而
/// SubagentSpawned 事件按异步调度顺序到达；旧 FIFO 会把一个 Agent 的标题
/// 与另一个 childSessionId 拼在一起。仅对缺字段的历史数据保留
/// 「同 turn 唯一剩余一对」的无歧义兼容。
fn pair_subagent_rows(rows: &[&GroupingRow]) -> SubagentPairing {
    use std::collections::{HashMap, HashSet};

    let mut subagent_by_agent_tool_row_id: HashMap<usize, usize> = HashMap::new();
    let mut claimed_subagent_row_ids: HashSet<usize> = HashSet::new();
    let mut agent_tool_by_turn_and_call_id: HashMap<String, usize> = HashMap::new();
    let mut agent_tool_rows: Vec<usize> = Vec::new();
    let mut subagent_rows: Vec<usize> = Vec::new();
    let mut legacy_subagent_rows: Vec<usize> = Vec::new();

    for (index, row) in rows.iter().enumerate() {
        if is_agent_tool_call_row(row) {
            agent_tool_rows.push(index);
            agent_tool_by_turn_and_call_id.insert(
                format!(
                    "{}\u{0}{}",
                    row.turn_id.clone().unwrap_or_default(),
                    row.tool_call_id.clone().unwrap_or_default()
                ),
                index,
            );
        } else if row.kind == "subagent" {
            subagent_rows.push(index);
        }
    }

    for &index in &subagent_rows {
        let row = rows[index];
        let Some(parent) = row.parent_tool_call_id.as_ref().filter(|p| !p.is_empty()) else {
            legacy_subagent_rows.push(index);
            continue;
        };
        let key = format!("{}\u{0}{}", row.turn_id.clone().unwrap_or_default(), parent);
        if let Some(&host_index) = agent_tool_by_turn_and_call_id.get(&key) {
            let host = rows[host_index];
            let same_turn = host.turn_id == row.turn_id;
            if same_turn && !subagent_by_agent_tool_row_id.contains_key(&host_index) {
                subagent_by_agent_tool_row_id.insert(host_index, index);
                claimed_subagent_row_ids.insert(index);
            }
        }
    }

    // 无歧义兼容（真源 :240-267）：同 turn 恰好一对未配对的，补配。
    let mut remaining_agent_tools_by_turn: HashMap<String, Vec<usize>> = HashMap::new();
    for &index in &agent_tool_rows {
        if subagent_by_agent_tool_row_id.contains_key(&index) {
            continue;
        }
        remaining_agent_tools_by_turn
            .entry(rows[index].turn_id.clone().unwrap_or_default())
            .or_default()
            .push(index);
    }
    let mut legacy_subagents_by_turn: HashMap<String, Vec<usize>> = HashMap::new();
    for &index in &legacy_subagent_rows {
        legacy_subagents_by_turn
            .entry(rows[index].turn_id.clone().unwrap_or_default())
            .or_default()
            .push(index);
    }
    for (turn_id, legacy_subagents) in &legacy_subagents_by_turn {
        let Some(tool_rows) = remaining_agent_tools_by_turn.get(turn_id) else {
            continue;
        };
        if tool_rows.len() != 1 || legacy_subagents.len() != 1 {
            continue;
        }
        subagent_by_agent_tool_row_id.insert(tool_rows[0], legacy_subagents[0]);
        claimed_subagent_row_ids.insert(legacy_subagents[0]);
    }

    SubagentPairing {
        subagent_by_agent_tool_row_id,
        claimed_subagent_row_ids,
    }
}

/// `buildAssistantWorkRenderItems`（真源 :272-428）：分组主入口。
///
/// 输入应为 assistant 工作行序列（assistantText / reasoning / toolCall /
/// subagent 等）。userInput / turnHeader / artifact 行不参与分组——调用方
/// 传入时它们会以 Row 出现并自然打断连续工具链（与真源把 turn 结构拆分的
/// 效果一致）。
pub fn build_assistant_work_render_items(
    rows: &[GroupingRow],
    options: &WorkRenderOptions,
) -> Vec<WorkRenderItem> {
    let mut items: Vec<WorkRenderItem> = Vec::new();

    // visibleRows（真源 :283-294）：剔除暂不可见行，保证阶段边界与尾部状态
    // 基于用户实际可见的行序。（reasoning 过滤裁剪见模块头注释。）
    let visible_indices: Vec<usize> = (0..rows.len())
        .filter(|&i| !should_defer_unclassified_shell_tool_call(&rows[i]))
        .collect();
    let visible_rows: Vec<&GroupingRow> = visible_indices.iter().map(|&i| &rows[i]).collect();

    let pairing = pair_subagent_rows(&visible_rows);

    // prepareCuaGroups（真源 conversationCuaGroups.ts:253-280）：
    // 先算出「哪些连续行被收敛成 CUA 聚合组」，主循环再按这份映射产出渲染项。
    // 这样行序仍由主循环独家决定，不会出现两段 items 拼接导致的乱序。
    let cua_groups: Vec<(Vec<usize>, crate::cuaGroups::CuaGroupRenderItem)> = if options
        .enable_cua_grouping
    {
        crate::cuaGroups::prepare_cua_groups(
            rows,
            crate::cuaGroups::CuaGroupOptions {
                enabled: true,
                stage_tail_is_running: options.stage_tail_is_running,
            },
        )
        .into_iter()
        .filter_map(|entry| match entry {
            crate::cuaGroups::PreparedEntry::CuaGroup(g) if !g.row_indices.is_empty() => {
                Some((g.row_indices.clone(), g))
            }
            _ => None,
        })
        .collect()
    } else {
        Vec::new()
    };
    // 行下标 → 所属 CUA 组（CUA 组内任一行的下标都能查到组）。
    let cua_group_of: std::collections::HashMap<usize, crate::cuaGroups::CuaGroupRenderItem> =
        cua_groups
            .iter()
            .flat_map(|(indices, group)| {
                indices
                    .iter()
                    .map(move |i| (*i, group.clone()))
            })
            .collect();
    // 已消费的组（避免组内后续行重复产出）。
    let mut cua_groups_emitted: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // 主循环的输入仍是可见行（CUA 分组只提供映射，不改行序）。
    let prepared_rows = visible_rows;


    let mut index = 0usize;
    while index < prepared_rows.len() {
        let row = prepared_rows[index];
        let row_index = visible_indices[index];

        // CUA 聚合组（真源 conversationCuaGroups）：整组只在其**首个可见行**
        // 处产出一次，后续组内行跳过。
        if let Some(group) = cua_group_of.get(&row_index) {
            if !cua_groups_emitted.contains(&group.key) {
                cua_groups_emitted.insert(group.key.clone());
                items.push(WorkRenderItem::CuaGroup {
                    key: group.key.clone(),
                    row_id: group.row_id,
                    row_indices: group.row_indices.clone(),
                    node: group.node.clone(),
                    group: group.clone(),
                });
            }
            index += 1;
            continue;
        }

        // 已配对进 Agent 块的 subagent 行：不再单独渲染（真源 :316-320）。
        if row.kind == "subagent" && pairing.claimed_subagent_row_ids.contains(&row_index) {
            index += 1;
            continue;
        }
        if row.is_tool_call() {
            if let Some(&subagent_index) = pairing.subagent_by_agent_tool_row_id.get(&row_index) {
                items.push(WorkRenderItem::AgentToolCall {
                    key: format!("agent:{}", row.row_id),
                    row_index,
                    subagent_row_index: subagent_index,
                });
                index += 1;
                continue;
            }
        }

        let is_explore_row = is_explore_tool_call_row(row);
        if !is_explore_row {
            // Changes 分组（真源 :337-359）。
            if options.enable_changes_grouping && is_changes_tool_call_row(row) {
                let mut group_indices = vec![row_index];
                let mut group_refs = vec![row];
                index += 1;
                while index < prepared_rows.len() {
                    let next = prepared_rows[index];
                    if next.kind == "cuaGroup" || !is_changes_tool_call_row(next) {
                        break;
                    }
                    group_indices.push(visible_indices[index]);
                    group_refs.push(next);
                    index += 1;
                }
                if group_refs.len() == 1 {
                    // 单个工具不建组（真源 :346-351）：等第二个连续同类工具到达再升级。
                    items.push(WorkRenderItem::Row {
                        key: format!("row:{}", row.row_id),
                        row_index,
                    });
                    continue;
                }
                let stage_tail = options.stage_tail_is_running && index == prepared_rows.len();
                let mut item = build_changes_group(&group_refs, stage_tail);
                set_row_indices(&mut item, group_indices);
                items.push(item);
                continue;
            }
            // 终端分组（真源 :360-384）。
            if options.enable_terminal_grouping && is_execute_tool_call_row(row) {
                let mut group_indices = vec![row_index];
                let mut group_refs = vec![row];
                index += 1;
                while index < prepared_rows.len() {
                    let next = prepared_rows[index];
                    if next.kind == "cuaGroup" || !is_execute_tool_call_row(next) {
                        break;
                    }
                    group_indices.push(visible_indices[index]);
                    group_refs.push(next);
                    index += 1;
                }
                if group_refs.len() == 1 {
                    items.push(WorkRenderItem::Row {
                        key: format!("row:{}", row.row_id),
                        row_index,
                    });
                    continue;
                }
                let stage_tail = options.stage_tail_is_running && index == prepared_rows.len();
                let mut item = build_execute_group(&group_refs, stage_tail);
                set_row_indices(&mut item, group_indices);
                items.push(item);
                continue;
            }
            items.push(WorkRenderItem::Row {
                key: format!("row:{}", row.row_id),
                row_index,
            });
            index += 1;
            continue;
        }

        // Explore 分组（真源 :394-424）。
        if !options.enable_explore_grouping {
            items.push(WorkRenderItem::Row {
                key: format!("row:{}", row.row_id),
                row_index,
            });
            index += 1;
            continue;
        }

        let mut group_indices = vec![row_index];
        let mut group_refs = vec![row];
        index += 1;
        while index < prepared_rows.len() {
            let next = prepared_rows[index];
            if next.kind == "cuaGroup" || !is_explore_tool_call_row(next) {
                break;
            }
            group_indices.push(visible_indices[index]);
            group_refs.push(next);
            index += 1;
        }
        // Explore 只有在出现第二个连续只读工具后才成立（真源 :414-418）。
        if group_refs.len() == 1 {
            items.push(WorkRenderItem::Row {
                key: format!("row:{}", row.row_id),
                row_index,
            });
            continue;
        }
        let stage_tail = options.stage_tail_is_running && index == prepared_rows.len();
        let mut item = build_explore_group(&group_refs, stage_tail);
        set_row_indices(&mut item, group_indices);
        items.push(item);
    }

    items
}

/// 把分组行下标写回分组项（分组构造对齐真源形状，下标由主循环补）。
fn set_row_indices(item: &mut WorkRenderItem, indices: Vec<usize>) {
    match item {
        WorkRenderItem::ExploreGroup { row_indices, .. }
        | WorkRenderItem::ExecuteGroup { row_indices, .. }
        | WorkRenderItem::ChangesGroup { row_indices, .. }
        | WorkRenderItem::CuaGroup { row_indices, .. } => *row_indices = indices,
        WorkRenderItem::Row { .. } | WorkRenderItem::AgentToolCall { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 官方 CUA 工具（连字符命名空间，真源 conversationCuaGroups.ts:22-25）。
    fn cua_row(id: i64, name: &str, response_id: &str) -> GroupingRow {
        let mut row = tool_row(id, name, "completed", Value::Null);
        row.assistant_response_id = Some(response_id.into());
        row
    }

    #[test]
    fn cua_group_merges_official_cua_tools() {
        // 两个官方 CUA 工具 + 同响应正文 → 一张CUA 卡。
        let rows = vec![
            cua_row(0, "mcp__computer-use__left_click", "r1"),
            text_row(1, "assistantText"),
            cua_row(2, "mcp__computer-use__type", "r1"),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        let group_count = items
            .iter()
            .filter(|i| matches!(i, WorkRenderItem::CuaGroup { .. }))
            .count();
        assert_eq!(group_count, 1, "应合成一个 CUA 组：{items:?}");
    }

    #[test]
    fn cua_group_not_emitted_twice() {
        // 组内后续行必须跳过，否则同一张卡会渲染多次。
        let rows = vec![
            cua_row(0, "mcp__computer-use__left_click", "r1"),
            cua_row(1, "mcp__computer-use__type", "r1"),
            cua_row(2, "mcp__computer-use__key", "r1"),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(
            items
                .iter()
                .filter(|i| matches!(i, WorkRenderItem::CuaGroup { .. }))
                .count(),
            1,
            "三个连续 CUA 工具只出一张卡"
        );
    }

    #[test]
    fn cua_grouping_can_be_disabled() {
        let rows = vec![
            cua_row(0, "mcp__computer-use__left_click", "r1"),
            cua_row(1, "mcp__computer-use__type", "r1"),
        ];
        let items = build_assistant_work_render_items(
            &rows,
            &WorkRenderOptions {
                enable_cua_grouping: false,
                ..Default::default()
            },
        );
        assert!(
            !items
                .iter()
                .any(|i| matches!(i, WorkRenderItem::CuaGroup { .. })),
            "关闭后不产出 CUA 组：{items:?}"
        );
    }

    #[test]
    fn underscore_cua_tools_stay_ungrouped() {
        // ★下划线命名空间不属于官方 CUA 前缀（真源 :22-25用连字符），
        // 不该被聚合。
        let rows = vec![
            cua_row(0, "mcp__computer_use__left_click", "r1"),
            cua_row(1, "mcp__computer_use__type", "r1"),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert!(
            !items
                .iter()
                .any(|i| matches!(i, WorkRenderItem::CuaGroup { .. })),
            "下划线形式不聚合：{items:?}"
        );
    }

    #[test]
    fn non_cua_rows_keep_their_render_items() {
        // 普通行仍走既有路径，CUA 分组不影响它们。
        let rows = vec![text_row(0, "assistantText"), text_row(1, "reasoning")];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert!(
            items.iter().all(|i| matches!(i, WorkRenderItem::Row { .. })),
            "无 CUA 工具时全部为普通行：{items:?}"
        );
    }

    fn tool_row(id: i64, name: &str, status: &str, input: Value) -> GroupingRow {
        GroupingRow {
            row_id: id,
            kind: "toolCall".into(),
            tool_name: Some(name.into()),
            tool_call_id: Some(format!("tc-{id}")),
            status: status.into(),
            input,
            input_text: String::new(),
            started_at: None,
            turn_id: Some("t1".into()),
            parent_tool_call_id: None,
            assistant_response_id: None,
        }
    }

    fn text_row(id: i64, kind: &str) -> GroupingRow {
        GroupingRow {
            row_id: id,
            kind: kind.into(),
            tool_name: None,
            tool_call_id: None,
            status: "completed".into(),
            input: Value::Null,
            input_text: String::new(),
            started_at: None,
            turn_id: Some("t1".into()),
            parent_tool_call_id: None,
            assistant_response_id: None,
        }
    }

    fn bash(cmd: &str) -> Value {
        json!({ "command": cmd })
    }

    #[test]
    fn consecutive_execute_rows_form_group() {
        let rows = vec![
            tool_row(1, "Bash", "success", bash("cargo build")),
            tool_row(2, "Bash", "success", bash("cargo test")),
        ];
        let opts = WorkRenderOptions::default();
        let items = build_assistant_work_render_items(&rows, &opts);
        assert_eq!(items.len(), 1);
        match &items[0] {
            WorkRenderItem::ExecuteGroup {
                key,
                node,
                row_indices,
                ..
            } => {
                assert_eq!(key, "execute:1");
                assert_eq!(node.tool_call.title.as_deref(), Some("Execute"));
                assert_eq!(node.child_tool_calls.len(), 2);
                assert_eq!(row_indices, &vec![0, 1]);
                // 完成态：无 cancelled → completed。
                assert_eq!(node.tool_call.status, "completed");
            }
            other => panic!("期望 ExecuteGroup，得 {other:?}"),
        }
    }

    #[test]
    fn single_execute_row_stays_row() {
        // 真源 :371-376：单项不建组。
        let rows = vec![tool_row(1, "Bash", "success", bash("ls"))];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], WorkRenderItem::Row { .. }));
    }

    #[test]
    fn read_only_shell_interrupts_execute_group() {
        // 真源 :157-159：只读命令是 explore 行（当前 Rust 侧 explore 分组关闭 →
        // 逐行），它会打断 execute 链。
        let rows = vec![
            tool_row(1, "Bash", "success", bash("rm -rf build")),
            tool_row(2, "Bash", "success", bash("rg -n foo")),
            tool_row(3, "Bash", "success", bash("cargo build")),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        // 1 与 3 之间隔着 explore 行 → 三段都是单项 → 3 个 Row。
        assert_eq!(items.len(), 3);
        assert!(
            items
                .iter()
                .all(|i| matches!(i, WorkRenderItem::Row { .. }))
        );
    }

    #[test]
    fn stage_tail_running_marks_in_progress() {
        let rows = vec![
            tool_row(1, "Bash", "success", bash("a")),
            tool_row(2, "Bash", "running", bash("b")),
        ];
        let opts = WorkRenderOptions {
            stage_tail_is_running: true,
            ..Default::default()
        };
        let items = build_assistant_work_render_items(&rows, &opts);
        match &items[0] {
            WorkRenderItem::ExecuteGroup { node, .. } => {
                assert_eq!(node.tool_call.status, "in_progress");
            }
            other => panic!("期望 ExecuteGroup，得 {other:?}"),
        }
    }

    #[test]
    fn cancelled_child_marks_stopped() {
        // 真源 :122：无 running 尾部时，有 cancelled → stopped。
        let rows = vec![
            tool_row(1, "Bash", "success", bash("a")),
            tool_row(2, "Bash", "cancelled", bash("b")),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        match &items[0] {
            WorkRenderItem::ExecuteGroup { node, .. } => {
                assert_eq!(node.tool_call.status, "stopped");
            }
            other => panic!("期望 ExecuteGroup，得 {other:?}"),
        }
    }

    #[test]
    fn text_row_interrupts_group() {
        // assistantText 在两条 Bash 之间：打断分组（真源里 assistantText 是 work row，
        // nextRow.kind !== toolCall → break）。
        let rows = vec![
            tool_row(1, "Bash", "success", bash("a")),
            text_row(2, "assistantText"),
            tool_row(3, "Bash", "success", bash("b")),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn awaiting_command_shell_is_filtered_from_visibility() {
        // 真源 :103-112 + :286-294：运行中但命令未到的 shell 行从可见序列剔除，
        // 它不打断 Execute 链。
        let rows = vec![
            tool_row(1, "Bash", "success", bash("a")),
            tool_row(2, "Bash", "running", json!({})), // 命令还没流出来
            tool_row(3, "Bash", "success", bash("b")),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        // 行 2 被剔除后 1 与 3 连续 → 成组；行 2 不进组。
        assert_eq!(items.len(), 1);
        match &items[0] {
            WorkRenderItem::ExecuteGroup {
                row_indices, node, ..
            } => {
                assert_eq!(row_indices, &vec![0, 2]);
                assert_eq!(node.child_tool_calls.len(), 2);
            }
            other => panic!("期望 ExecuteGroup，得 {other:?}"),
        }
    }

    #[test]
    fn agent_tool_pairs_subagent_by_parent_tool_call_id() {
        // 真源 :209-238：按 parentToolCallId 精确配对（不看顺序）。
        let mut agent = tool_row(10, "Agent", "success", json!({}));
        agent.tool_call_id = Some("call-A".into());
        let mut sub = tool_row(11, "subagent", "success", Value::Null);
        sub.kind = "subagent".into();
        sub.parent_tool_call_id = Some("call-A".into());
        let rows = vec![agent, sub];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        // Agent 行出 AgentToolCall；subagent 行被 claim 不单独出现。
        assert_eq!(items.len(), 1);
        match &items[0] {
            WorkRenderItem::AgentToolCall {
                key,
                row_index,
                subagent_row_index,
            } => {
                assert_eq!(key, "agent:10");
                assert_eq!(*row_index, 0);
                assert_eq!(*subagent_row_index, 1);
            }
            other => panic!("期望 AgentToolCall，得 {other:?}"),
        }
    }

    #[test]
    fn mismatched_parent_call_id_does_not_pair() {
        // parentToolCallId 对不上 → 不配对；subagent 行落 Row。
        let mut agent = tool_row(10, "Agent", "success", json!({}));
        agent.tool_call_id = Some("call-A".into());
        let mut sub = tool_row(11, "subagent", "success", Value::Null);
        sub.kind = "subagent".into();
        sub.parent_tool_call_id = Some("call-OTHER".into());
        let rows = vec![agent, sub];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|i| matches!(i, WorkRenderItem::Row { .. }))
        );
    }

    #[test]
    fn legacy_subagent_pairs_only_when_unambiguous() {
        // 真源 :259-267：无 parentToolCallId 的历史数据，同 turn 恰好一对才补配。
        let mut agent = tool_row(10, "Agent", "success", json!({}));
        agent.tool_call_id = Some("call-A".into());
        let mut sub = tool_row(11, "subagent", "success", Value::Null);
        sub.kind = "subagent".into();
        sub.parent_tool_call_id = None; // 历史数据
        let rows = vec![agent.clone(), sub.clone()];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], WorkRenderItem::AgentToolCall { .. }));

        // 两个 Agent 对两个历史 subagent：不唯一 → 不配对。
        let mut agent2 = tool_row(20, "Agent", "success", json!({}));
        agent2.tool_call_id = Some("call-B".into());
        let mut sub2 = tool_row(21, "subagent", "success", Value::Null);
        sub2.kind = "subagent".into();
        let rows = vec![agent.clone(), sub.clone(), agent2, sub2];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 4, "不唯一时全部落 Row");
    }

    #[test]
    fn explore_grouping_disabled_stays_rows() {
        // 显式关闭 explore 分组：连续 Read 逐行。
        let rows = vec![
            tool_row(1, "Read", "success", json!({"file_path": "a.rs"})),
            tool_row(2, "Read", "success", json!({"file_path": "b.rs"})),
        ];
        let opts = WorkRenderOptions {
            enable_explore_grouping: false,
            ..Default::default()
        };
        let items = build_assistant_work_render_items(&rows, &opts);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn explore_grouping_defaults_to_on_and_forms_group() {
        // 真源 ENABLE_EXPLORE_TOOL_CALL_GROUPING = true：默认开启，连续 Read 成组。
        let rows = vec![
            tool_row(1, "Read", "success", json!({"file_path": "a.rs"})),
            tool_row(2, "Read", "success", json!({"file_path": "b.rs"})),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 1);
        match &items[0] {
            WorkRenderItem::ExploreGroup { key, node, .. } => {
                assert_eq!(key, "explore:1");
                assert_eq!(node.tool_call.title.as_deref(), Some("Explore"));
                assert_eq!(node.child_tool_calls.len(), 2);
            }
            other => panic!("期望 ExploreGroup，得 {other:?}"),
        }
    }

    #[test]
    fn changes_grouping_enabled_groups_writes() {
        // changes 开关打开时 Write/Edit 成组；父状态无 cancelled 分支（真源 :195）。
        let mut rows = vec![
            tool_row(1, "Write", "success", json!({"file_path": "a.rs"})),
            tool_row(2, "Edit", "cancelled", json!({"file_path": "b.rs"})),
        ];
        let opts = WorkRenderOptions {
            enable_changes_grouping: true,
            ..Default::default()
        };
        let items = build_assistant_work_render_items(&rows, &opts);
        assert_eq!(items.len(), 1);
        match &items[0] {
            WorkRenderItem::ChangesGroup { node, .. } => {
                // cancelled 子项不影响父状态（非 stage tail → completed）。
                assert_eq!(node.tool_call.status, "completed");
            }
            other => panic!("期望 ChangesGroup，得 {other:?}"),
        }
        // 开关关（默认）：逐行。
        let _ = &mut rows;
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn child_nodes_carry_full_legacy_fields() {
        // 子节点必须携带完整（含 input/raw）——execute_group 的 children 递归渲染依赖它。
        let rows = vec![
            tool_row(1, "Bash", "success", bash("cargo build")),
            tool_row(2, "Bash", "success", bash("cargo test")),
        ];
        let items = build_assistant_work_render_items(&rows, &WorkRenderOptions::default());
        match &items[0] {
            WorkRenderItem::ExecuteGroup { node, .. } => {
                let child = &node.child_tool_calls[0];
                assert_eq!(
                    child
                        .tool_call
                        .input
                        .as_ref()
                        .and_then(|i| i.get("command"))
                        .and_then(|c| c.as_str()),
                    Some("cargo build")
                );
                assert!(child.tool_call.raw.is_object());
            }
            other => panic!("期望 ExecuteGroup，得 {other:?}"),
        }
    }
}
