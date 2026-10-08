//! 1:1 翻译真源的 **sessions-index join**（真源无独立文件，见下）。
//!
//! ## 为什么需要这个文件
//!
//! 真源 `workspace-grouped-tasks/view.ts` 的重排算法要在节点之间**搬运完整
//! task 对象**，所以 `GroupedTaskViewNode` 的 `tasks` 是完整 `ZCodeTaskListItem[]`。
//!
//! 但服务端只提供分组结构，不提供 task 内容。真源注释原文
//! （`zcodeTaskListTypes.ts`，`ZCodeGroupedTaskViewStructureMember`上方）：
//! 「grouped 视图的任务数据源迁到 sessions-index 后，服务端只提供分组结构
//! （task_groups / task_group_members / task_group_view_node_orders），
//! **由客户端与 sessions-index 会话做 join**。」
//!
//! Rust 侧的后端 `taskgroup::GroupedNode` 忠实照抄了这个契约——只返回 `task_ids`。
//! 本文件补的就是客户端这一半：把 `task_ids` join 成 `TaskListItem`。
//!
//! ## join 的语义（不能简化）
//!
//! `taskKey` = `${resolveWorkspaceStateKey(workspacePath, workspaceIdentity)}\0${taskId}`，
//! 其中 workspaceKey = `identity?.trim() || path`。所以 join 时**必须拿到
//! workspaceIdentity**，否则：
//! - 本地 workspace：identity 为空 → key 用 path，尚可；
//! - 远程 workspace（ssh/wsl/docker）：identity 非空 → 真源 key 用 identity，
//!   而我们只会填path → **key 算错，跨窗口拖拽会串台**。
//!
//! 真源对此有明确警告（`zcodeSessionStoreSelectors.ts`）：
//! 「同一路径的不同 SSH/WSL/Docker 窗口会互相读到 task config、队列和错误态」。
//!
//! ## 与真源的一处**刻意差异**
//!
//! 真源 join 数据源是完整的 sessions-index（SQLite），可拿到每个 task 的
//! `workspacePath` / `workspaceIdentity`。Rust 侧当前只有内存中的
//! `SessionsStore`（agent 会话列表快照），且 `workspace` 字段是宽松 JSON。
//! 因此：
//! - 命中会话列表 → 用会话的 workspace 填，字段齐全；
//! - 未命中（会话不在当前列表里，例如归档/跨页）→ **不能凭空猜 workspace**，
//!   退化为「用分组视图所属的 workspace_path」，并标记 `joined: false`，
//!   由调用方决定是否渲染占位行。
//!
//! 绝不用空串冒充电workspacePath —— 那会让 taskKey 变成 `\0taskId`，
//! 看起来能用，但一旦与真实条目比较就会产生假相等。

use super::view::{GroupedTaskView, GroupedTaskViewNode, TaskGroup, TaskListItem};

/// join 后的任务条目：视图数据 + join 元信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedTask {
    pub item: TaskListItem,
    /// 是否成功 join 到真实会话。
    ///
    /// `false` 表示该 task 只存在于分组结构里（会话不在当前列表），
    /// 此时 `item.workspace_path` 是**所属 workspace 的兜底值**而非真源事实，
    /// 渲染时应作占位处理，不要拿它的 title/status 当权威值。
    pub joined: bool,
}

/// 后端分组结构（`taskgroup::GroupedNode` 的前端镜像，`task_ids` 形态）。
///
/// 与后端 serde 配置对应：`tag = "type"` + camelCase，tag 值转小写。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupedStructureNode {
    Group {
        group: TaskGroup,
        task_ids: Vec<String>,
    },
    Task {
        task_id: String,
    },
}

/// 后端分组结构（`taskgroup::GroupedView` 的前端镜像）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupedStructure {
    pub nodes: Vec<GroupedStructureNode>,
}

/// join 所需的会话侧数据（`SessionsStore::all()` 的投影）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    /// 会话的 workspacePath（真源 `workspace.workspacePath`）。
    pub workspace_path: String,
    /// 会话的 workspaceIdentity（真源 `workspace.workspaceIdentity`）。
    /// 远程 workspace 必须有，否则 taskKey 会算错（见模块注释）。
    pub workspace_identity: Option<String>,
}

/// 从会话列表建`task_id -> SessionRow` 索引。
pub fn index_sessions(rows: &[SessionRow]) -> HashMap<String, SessionRow> {
    rows.iter()
        .map(|r| (r.session_id.clone(), r.clone()))
        .collect()
}

/// 兜底构造：会话缺失时用所属 workspace 造一个未 join 的条目。
///
/// 兜底的 workspace_path 用**分组视图所属 workspace**（后端查询时的 ws_path），
/// 这是当前唯一已知为真的 workspace 事实；workspace_identity 传 `None`
/// 而非猜一个值——identity 猜错比没有更危险。
fn fallback_task(
    task_id: &str,
    workspace_path: &str,
    workspace_identity: Option<&str>,
) -> JoinedTask {
    JoinedTask {
        item: TaskListItem {
            task_id: task_id.to_string(),
            title: task_id.to_string(),
            workspace_path: workspace_path.to_string(),
            workspace_identity: workspace_identity.map(|s| s.to_string()),
            created_at: 0,
            updated_at: 0,
            status: String::new(),
        },
        joined: false,
    }
}

/// join 单个 task_id。
fn join_one(
    task_id: &str,
    sessions: &HashMap<String, SessionRow>,
    workspace_path: &str,
    workspace_identity: Option<&str>,
) -> JoinedTask {
    match sessions.get(task_id) {
        Some(row) => JoinedTask {
            item: TaskListItem {
                task_id: task_id.to_string(),
                title: row.title.clone(),
                // 会话侧的 workspace 优先——它是 task 自己的事实，
                // 分组视图所属 workspace 只是查询范围，两者不一定相同
                // （跨 workspace 分组视图）。
                workspace_path: row.workspace_path.clone(),
                workspace_identity: row.workspace_identity.clone(),
                created_at: row.created_at,
                updated_at: row.updated_at,
                status: row.status.clone(),
            },
            joined: true,
        },
        None => fallback_task(task_id, workspace_path, workspace_identity),
    }
}

/// 把后端分组结构 join 成 `view.rs` 可直接消费的 `GroupedTaskView`。
///
/// `workspace_path` / `workspace_identity` 是**本次查询的 workspace 范围**，
/// 仅用于给未命中的 task 做兜底（见模块注释）。
///
/// 真源对应的 join 发生在 `useGroupedTaskView` 一类的hook 里，
/// 语义即「结构 × sessions-index →完整视图」。
pub fn join_grouped_structure(
    structure: &GroupedStructure,
    sessions: &HashMap<String, SessionRow>,
    workspace_path: &str,
    workspace_identity: Option<&str>,
) -> GroupedTaskView {
    GroupedTaskView {
        nodes: structure
            .nodes
            .iter()
            .map(|node| match node {
                GroupedStructureNode::Group { group, task_ids } => {
                    let tasks = task_ids
                        .iter()
                        .map(|id| join_one(id, sessions, workspace_path, workspace_identity).item)
                        .collect();
                    GroupedTaskViewNode::Group {
                        group: group.clone(),
                        tasks,
                        sort_order: None,
                    }
                }
                GroupedStructureNode::Task { task_id } => GroupedTaskViewNode::Task {
                    task: join_one(task_id, sessions, workspace_path, workspace_identity).item,
                    sort_order: None,
                },
            })
            .collect(),
    }
}

/// join 结果中未命中的 task_id 列表。
///
/// 用途：UI 决定是否提示「有 N 个任务未加载」；也便于测试断言 join 覆盖率。
pub fn unjoined_task_ids(structure: &GroupedStructure, sessions: &HashMap<String, SessionRow>) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for node in &structure.nodes {
        match node {
            GroupedStructureNode::Group { task_ids, .. } => {
                for id in task_ids {
                    if !sessions.contains_key(id) && !missing.contains(id) {
                        missing.push(id.clone());
                    }
                }
            }
            GroupedStructureNode::Task { task_id } => {
                if !sessions.contains_key(task_id) && !missing.contains(task_id) {
                    missing.push(task_id.clone());
                }
            }
        }
    }
    missing
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn session(
        id: &str,
        title: &str,
        ws_path: &str,
        ws_identity: Option<&str>,
    ) -> SessionRow {
        SessionRow {
            session_id: id.into(),
            title: title.into(),
            status: "idle".into(),
            created_at: 100,
            updated_at: 200,
            workspace_path: ws_path.into(),
            workspace_identity: ws_identity.map(|s| s.into()),
        }
    }

    fn structure() -> GroupedStructure {
        GroupedStructure {
            nodes: vec![
                GroupedStructureNode::Task {
                    task_id: "t1".into(),
                },
                GroupedStructureNode::Group {
                    group: TaskGroup {
                        group_id: "g1".into(),
                        title: "工作".into(),
                        color: "blue".into(),
                    },
                    task_ids: vec!["t2".into(), "t3".into()],
                },
            ],
        }
    }

    fn sessions() -> HashMap<String, SessionRow> {
        index_sessions(&[
            session("t1", "任务一", "/ws", None),
            session("t2", "任务二", "/ws", None),
            // t3 缺失：会话不在当前列表。
        ])
    }

    fn shape(view: &GroupedTaskView) -> String {
        view.nodes
            .iter()
            .map(|node| match node {
                GroupedTaskViewNode::Task { task, .. } => task.task_id.clone(),
                GroupedTaskViewNode::Group { group, tasks, .. } => format!(
                    "{}[{}]",
                    group.group_id,
                    tasks
                        .iter()
                        .map(|t| t.task_id.clone())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    #[test]
    fn join_preserves_structure_shape() {
        let view = join_grouped_structure(&structure(), &sessions(), "/ws", None);
        assert_eq!(shape(&view), "t1,g1[t2,t3]");
    }

    #[test]
    fn join_fills_title_status_from_sessions() {
        let view = join_grouped_structure(&structure(), &sessions(), "/ws", None);
        let task = match &view.nodes[0] {
            GroupedTaskViewNode::Task { task, .. } => task,
            other => panic!("期望 task 节点，实际 {other:?}"),
        };
        assert_eq!(task.title, "任务一");
        assert_eq!(task.status, "idle");
        assert_eq!(task.updated_at, 200);
    }

    #[test]
    fn missing_session_falls_back_to_query_workspace() {
        // t3 不在会话列表 → 用分组视图所属 workspace 兜底，title 退化为 taskId。
        let view = join_grouped_structure(&structure(), &sessions(), "/ws", None);
        let GroupedTaskViewNode::Group { tasks, .. } = &view.nodes[1] else {
            panic!("期望 group 节点");
        };
        let t3 = &tasks[1];
        assert_eq!(t3.task_id, "t3");
        assert_eq!(t3.title, "t3", "未 join 的条目 title 应退化为 taskId");
        assert_eq!(t3.workspace_path, "/ws");
        assert_eq!(t3.status, "");
        assert_eq!(t3.updated_at, 0);
    }

    #[test]
    fn unjoined_task_ids_reports_missing() {
        assert_eq!(unjoined_task_ids(&structure(), &sessions()), vec!["t3".to_string()]);
    }

    #[test]
    fn unjoined_is_empty_when_all_hit() {
        let s = index_sessions(&[
            session("t1", "a", "/ws", None),
            session("t2", "b", "/ws", None),
            session("t3", "c", "/ws", None),
        ]);
        assert!(unjoined_task_ids(&structure(), &s).is_empty());
    }

    // ── 远程 workspace：identity 必须透传，否则 taskKey 算错 ──

    #[test]
    fn remote_identity_is_preserved_into_task_key() {
        // 真源 remote:ssh:<host>:<port>:<user>:<path> 格式（remote-workspace-identity.ts:50）。
        let ident = "remote:ssh:box.local:22:root:/srv/app";
        let s = index_sessions(&[session("t1", "远程任务", "/srv/app", Some(ident))]);
        let view = join_grouped_structure(
            &GroupedStructure {
                nodes: vec![GroupedStructureNode::Task {
                    task_id: "t1".into(),
                }],
            },
            &s,
            "/ws",
            None,
        );
        let GroupedTaskViewNode::Task { task, .. } = &view.nodes[0] else {
            panic!("期望 task 节点");
        };
        assert_eq!(task.workspace_identity.as_deref(), Some(ident));
        // taskKey 必须以 identity 打头，而不是 path —— 这是跨窗口隔离的根据。
        assert_eq!(
            super::super::view::task_key_of(task),
            format!("{ident}\u{0}t1")
        );
    }

    #[test]
    fn fallback_does_not_fabricate_identity() {
        // 未命中时 identity 传 None（用查询范围的），绝不猜一个值。
        let view = join_grouped_structure(&structure(), &sessions(), "/ws", None);
        let GroupedTaskViewNode::Group { tasks, .. } = &view.nodes[1] else {
            panic!("期望 group 节点");
        };
        assert_eq!(tasks[1].workspace_identity, None);
    }

    #[test]
    fn session_workspace_wins_over_query_scope() {
        // 跨 workspace 分组视图：task 自己的 workspace 优先于查询范围。
        let s = index_sessions(&[session("t1", "别的 ws", "/other", None)]);
        let view = join_grouped_structure(
            &GroupedStructure {
                nodes: vec![GroupedStructureNode::Task {
                    task_id: "t1".into(),
                }],
            },
            &s,
            "/ws",
            None,
        );
        let GroupedTaskViewNode::Task { task, .. } = &view.nodes[0] else {
            panic!("期望 task 节点");
        };
        assert_eq!(task.workspace_path, "/other");
    }

    #[test]
    fn joined_view_is_reorderable() {
        // join 产物必须能直接喂给 view.rs 的重排算法——这正是本文件的目的。
        let view = join_grouped_structure(&structure(), &sessions(), "/ws", None);
        let key_t3 = super::super::view::task_key("/ws", None, "t3");
        let moved = super::super::view::move_task_over_task(
            &view,
            &key_t3,
            &super::super::view::task_key("/ws", None, "t1"),
            super::super::view::InsertPosition::Before,
        );
        assert_eq!(shape(&moved), "t3,t1,g1[t2]");
    }

    #[test]
    fn empty_structure_joins_to_empty_view() {
        let view = join_grouped_structure(&GroupedStructure::default(), &sessions(), "/ws", None);
        assert!(view.nodes.is_empty());
        assert!(unjoined_task_ids(&GroupedStructure::default(), &sessions()).is_empty());
    }
}