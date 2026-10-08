//! 1:1 翻译 `packages/ui/src/workspace-grouped-tasks/view.ts`（581 行）+ `ids.ts`。
//!
//! 本文件是**分组任务视图的纯逻辑层**：不含任何渲染，负责 task / group 的
//! 查找、移除、插入、重排，以及拖拽与菜单移动产生的视图变换。
//!
//! 真源把这段逻辑集中在 view.ts 里，是刻意为之——文件头注释原文：
//!「grouped task 的纯 view helper 暂时集中维护 task/group 重排、菜单移动和乐观合并，
//! 后续按拖拽域继续拆分」。Rust 侧保持同样的集中，便于逐函数对照 diff。
//!
//! ## 必须保留的三条真源语义
//!
//! 1. **`taskKey` 的唯一性靠 workspace 前缀**（ids.ts:7）。真源是
//!    `${buildTaskWorkspaceKey(path, identity)}\0${taskId}`，其中
//!    `buildTaskWorkspaceKey` → `resolveWorkspaceStateKey` =
//!    `identity?.trim() || path`（zcodeSessionStoreSelectors.ts）。
//!    分隔符是 `\u0000`（NUL），保证拼接后无法产生歧义碰撞。
//!    跨窗口拖拽时，同一 taskId 可能存在于不同 workspace，必须靠这个 key 区分。
//!
//! 2. **重排一律"先移除再按目标重新定位"**（view.ts:335-336 注释）。
//!    真源原文：「先移除 active 再按 over task 重新定位，避免同组向下移动时
//!    旧index 把插入点推后一格」。这是最容易写错的地方——如果先算好 over 的
//!    index 再移除，同组内向下移动会偏移一格。Rust 侧严格照此顺序。
//!
//! 3. **`getWorkspaceHash` 之类的命名会骗人**。真源里 `workspaceIdentity`
//!    是远程/隔离 workspace 的身份（authority + canonicalPath），不是哈希；
//!    而 key 拼接优先用 identity、identity 为空才回退 path（见上）。
//!
//! 行号注释均指真源文件。

use std::collections::HashSet;

/// 任务条目（真源 `ZCodeTaskListItem` / `ZCodeTaskMeta` 的视图所需子集）。
///
/// 真源 `ZCodeTaskMeta` 有 30+ 字段，但 view.ts 全程只用
/// `workspacePath` / `workspaceIdentity` / `taskId` 三项定位（ids.ts:5），
/// 其余字段仅随任务一起搬运。Rust 侧保留搬运时用得到的展示字段，
/// 不搬与重排无关的运行时字段（model / effort / epoch 等）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListItem {
    pub task_id: String,
    pub title: String,
    pub workspace_path: String,
    pub workspace_identity: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    /// 任务运行态（真源 ZCodeTaskRuntimeStatus），渲染用。
    pub status: String,
}

/// 任务分组（真源 `ZCodeTaskGroup`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGroup {
    pub group_id: String,
    pub title: String,
    /// 真源 `ZCodeTaskGroupColor`，7 种固定色（types.ts:8-16）。
    pub color: String,
}

/// 分组视图的顶层节点（真源 `ZCodeGroupedTaskViewNode`）。
///
/// 真源是 discriminated union：`type: "group"` 带 `tasks` 数组，
/// `type: "task"` 带单个 `task`。
///
/// **与后端 `taskgroup::GroupedNode` 的差异（刻意）**：后端只返回
/// `task_ids`（真源注释：分组视图的任务数据源迁到 sessions-index 后，
/// 服务端只提供分组结构，由客户端 join）。而 view.ts 的重排算法需要
/// **完整 task 对象**才能在节点间搬运，所以本模块按真源建模，
/// 由调用方（后续的 join 逻辑）负责把 task_ids 补成完整任务。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupedTaskViewNode {
    Group {
        group: TaskGroup,
        tasks: Vec<TaskListItem>,
        sort_order: Option<i64>,
    },
    Task {
        task: TaskListItem,
        sort_order: Option<i64>,
    },
}

/// 分组视图（真源 `ZCodeGroupedTaskView`）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupedTaskView {
    pub nodes: Vec<GroupedTaskViewNode>,
}

/// 任务在分组视图里的定位（真源 `GroupedTaskLocation`，view.ts:201-213）。
///
/// 真源是 discriminated union：`parent` 区分挂在顶层还是在组内，
/// `nodeIndex` 是顶层数组下标，`taskIndex` 只有组内定位才有意义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskLocation {
    /// 游离在顶层：`nodes[node_index]`。
    Root { node_index: usize },
    /// 在组内：`nodes[node_index].tasks[task_index]`。
    Group {
        group_id: String,
        node_index: usize,
        task_index: usize,
    },
}

impl TaskLocation {
    /// 是否已在组内（真源各处`location?.parent === "group"` 的判定）。
    pub fn is_in_group(&self) -> bool {
        matches!(self, TaskLocation::Group { .. })
    }

    /// 取所在组 id；顶层任务返回 `None`。
    pub fn group_id(&self) -> Option<&str> {
        match self {
            TaskLocation::Group { group_id, .. } => Some(group_id.as_str()),
            TaskLocation::Root { .. } => None,
        }
    }
}

/// 拖拽落点的相对位置（真源 `GroupedTaskInsertPosition`，view.ts:215）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertPosition {
    Before,
    After,
}

impl InsertPosition {
    /// 转成「相对锚点下标的偏移量」：after 为 +1，before 为 +0。
    fn offset(self) -> usize {
        match self {
            InsertPosition::Before => 0,
            InsertPosition::After => 1,
        }
    }
}

/// 草稿任务最终应落在哪个分组（真源 `GroupedDraftTaskPlacement`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftPlacement {
    /// 顶层。
    Top,
    /// 指定组内（该组当前无成员时也归此类）。
    Group { group_id: String },
}

/// group 拖拽时的悬停目标（真源 view.ts:437-434 的 `over` 联合类型）。
///
/// 真源注释（view.ts:441-443）：group 拖到另一个 group 的 content 上时，
/// collision 可能返回组内 task；这里需要把组内 task 归一化为所属 group，
/// 避免 content 区域不触发 group over group。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragOver {
    /// 悬停在某个 group 节点上。
    Group { group_id: String },
    /// 悬停在某个 task 上。
    Task { task_key: String },
}

/// `resolveWorkspaceStateKey`（真源 zcodeSessionStoreSelectors.ts）。
/// `identity?.trim() || path` —— identity 优先，为空回退 path。
fn resolve_workspace_state_key(workspace_path: &str, workspace_identity: Option<&str>) -> String {
    match workspace_identity {
        Some(id) if !id.trim().is_empty() => id.trim().to_string(),
        _ => workspace_path.to_string(),
    }
}

/// `buildTaskWorkspaceKey`（真源 lib/taskQueryCache.ts:40-42）。
fn build_task_workspace_key(workspace_path: &str, workspace_identity: Option<&str>) -> String {
    resolve_workspace_state_key(workspace_path, workspace_identity)
}

/// **`taskKey`（真源 ids.ts:4-8）** —— 视图内任务的唯一键。
///
/// 真源原文：
/// ```text
/// `${buildTaskWorkspaceKey(task.workspacePath, task.workspaceIdentity)}\u0000${task.taskId}`
/// ```
///
/// 形参对齐真源 `Pick<ZCodeTaskMeta, "workspacePath" | "workspaceIdentity" | "taskId">`。
/// 用 `&str` 而非整个 task 结构，因为真源也只需要这三个字段。
pub fn task_key(workspace_path: &str, workspace_identity: Option<&str>, task_id: &str) -> String {
    format!(
        "{}\u{0}{}",
        build_task_workspace_key(workspace_path, workspace_identity),
        task_id
    )
}

// ── 以下为 view.ts 的内部 helper（真源未export，但被导出函数复用）──

/// `cloneView`（真源 :7-13）：深拷贝——task 节点拷贝自身，group 节点额外拷贝 tasks 数组。
pub fn clone_view(view: &GroupedTaskView) -> GroupedTaskView {
    GroupedTaskView {
        nodes: view
            .nodes
            .iter()
            .map(|node| match node {
                GroupedTaskViewNode::Task { task, sort_order } => GroupedTaskViewNode::Task {
                    task: task.clone(),
                    sort_order: *sort_order,
                },
                GroupedTaskViewNode::Group {
                    group,
                    tasks,
                    sort_order,
                } => GroupedTaskViewNode::Group {
                    group: group.clone(),
                    tasks: tasks.clone(),
                    sort_order: *sort_order,
                },
            })
            .collect(),
    }
}

/// `cloneNodes`（真源 :15-17）：只拷贝顶层数组，节点本身共享。
///
/// 真源刻意用这个浅拷贝——重排时只有被改动的那个 group 会单独走
/// `cloneGroupNode`，其余节点保持引用以避免无谓拷贝。Rust 侧 `nodes` 是
/// 拥有所有权的 `Vec`，浅拷贝即为`Vec::clone()`（元素按�� clone）。
fn clone_nodes(view: &GroupedTaskView) -> Vec<GroupedTaskViewNode> {
    view.nodes.clone()
}

/// `cloneGroupNode`（真源 :19-23）：拷贝 group 节点并单独拷贝其 tasks 数组，
/// 使后续 splice 不污染入参。
fn clone_group_node(node: &GroupedTaskViewNode) -> GroupedTaskViewNode {
    match node {
        GroupedTaskViewNode::Group {
            group,
            tasks,
            sort_order,
        } => GroupedTaskViewNode::Group {
            group: group.clone(),
            tasks: tasks.clone(),
            sort_order: *sort_order,
        },
        GroupedTaskViewNode::Task { .. } => node.clone(),
    }
}

/// `removeTaskFromView`（真源 :25-53）：从顶层或任一组的 tasks 里摘掉目标任务，
/// 返回新视图与被摘出的 task。
///
/// 真源遍历顺序：先按 `nodes` 顺序找，组内用 `findIndex` 取首个匹配。
/// 因为 `task_group_members` 主键是 `(workspace_key, task_id)`，一个 task
/// 只会出现在一处，所以「首个匹配」即唯一匹配。
///
/// 返回 `None` 表示没找到（真源返回 `task: null`）。
pub fn remove_task_from_view(
    view: &GroupedTaskView,
    target_task_key: &str,
) -> (GroupedTaskView, Option<TaskListItem>) {
    let mut nodes = clone_nodes(view);
    for index in 0..nodes.len() {
        match &nodes[index] {
            GroupedTaskViewNode::Task { task, .. } => {
                if task_key_of(task) == target_task_key {
                    let removed = nodes.remove(index);
                    let GroupedTaskViewNode::Task { task, .. } = removed else {
                        unreachable!("按Task 分支匹配，移除后必为 Task");
                    };
                    return (GroupedTaskView { nodes }, Some(task));
                }
            }
            GroupedTaskViewNode::Group { tasks, .. } => {
                if let Some(task_index) =
                    tasks.iter().position(|t| task_key_of(t) == target_task_key)
                {
                    let mut next_node = clone_group_node(&nodes[index]);
                    if let GroupedTaskViewNode::Group { tasks, .. } = &mut next_node {
                        let removed = tasks.remove(task_index);
                        nodes[index] = next_node;
                        return (GroupedTaskView { nodes }, Some(removed));
                    }
                }
            }
        }
    }
    (GroupedTaskView { nodes }, None)
}

/// `insertTaskIntoGroup`（真源 :55-75）：把 task 插入指定组。
///
/// `before_task_key` 为 `None` → 追加到组末；否则插到该 key 之前，
/// 找不到（`findIndex` 返回 -1）时也追加到组末（真源 :72）。
pub fn insert_task_into_group(
    view: &GroupedTaskView,
    task: TaskListItem,
    group_id: &str,
    before_task_key: Option<&str>,
) -> GroupedTaskView {
    let mut nodes = clone_nodes(view);
    let Some(group_index) = nodes.iter().position(
        |node| matches!(node, GroupedTaskViewNode::Group { group, .. } if group.group_id == group_id),
    ) else {
        // 真源 :64-66：找不到组则原样返回（只带浅拷贝的 nodes）。
        return GroupedTaskView { nodes };
    };

    let mut next_node = clone_group_node(&nodes[group_index]);
    if let GroupedTaskViewNode::Group { tasks, .. } = &mut next_node {
        let insert_index = match before_task_key {
            None => tasks.len(),
            Some(key) => tasks
                .iter()
                .position(|t| task_key_of(t) == key)
                .unwrap_or(tasks.len()),
        };
        tasks.insert(insert_index, task);
    }
    nodes[group_index] = next_node;
    GroupedTaskView { nodes }
}

/// `removeTaskFromGroupedView`（真源 :77-100）：只做移除，丢弃被摘出的 task
/// （调用方不关心task 本体，例如删除场景）。
///
/// 与 `remove_task_from_view` 的区别：真源这里**先扫所有组**，再扫顶层；
/// 而 `removeTaskFromView` 是单趟按节点顺序扫。同一 task 不可能既在组又在顶层，
/// 所以结果一致，但遍历顺序不同——照抄真源以免后续diff 误判。
pub fn remove_task_from_grouped_view(
    view: &GroupedTaskView,
    target_task_key: &str,
) -> GroupedTaskView {
    let mut next_view = clone_view(view);
    for node in &mut next_view.nodes {
        if let GroupedTaskViewNode::Group { tasks, .. } = node {
            if let Some(task_index) = tasks.iter().position(|t| task_key_of(t) == target_task_key) {
                tasks.remove(task_index);
                return next_view;
            }
        }
    }
    if let Some(node_index) = next_view.nodes.iter().position(
        |node| matches!(node, GroupedTaskViewNode::Task { task, .. } if task_key_of(task) == target_task_key),
    ) {
        next_view.nodes.remove(node_index);
    }
    next_view
}

/// `filterGroupedViewByTaskKeys`（真源 :102-132）：按 key 集合过滤任务，
/// 用于搜索时隐藏不匹配项。
///
/// 真源 :104-106：隐藏集合为空时**直接返回原引用**（不拷贝）。
/// Rust 侧无引用语义，但保留这个短路以避免无谓深拷贝——用是否变化来等价表达。
///
/// 真源用 `changed` 标志决定是否返回新对象：全都没被过滤掉就返回原 view。
pub fn filter_grouped_view_by_task_keys(
    view: &GroupedTaskView,
    hidden_task_keys: &HashSet<String>,
) -> GroupedTaskView {
    if hidden_task_keys.is_empty() {
        return view.clone();
    }

    let mut changed = false;
    let mut nodes: Vec<GroupedTaskViewNode> = Vec::with_capacity(view.nodes.len());
    for node in &view.nodes {
        match node {
            GroupedTaskViewNode::Task { task, .. } => {
                if hidden_task_keys.contains(&task_key_of(task)) {
                    changed = true;
                } else {
                    nodes.push(node.clone());
                }
            }
            GroupedTaskViewNode::Group {
                group,
                tasks,
                sort_order,
            } => {
                let filtered: Vec<TaskListItem> = tasks
                    .iter()
                    .filter(|t| !hidden_task_keys.contains(&task_key_of(t)))
                    .cloned()
                    .collect();
                if filtered.len() == tasks.len() {
                    nodes.push(node.clone());
                } else {
                    changed = true;
                    nodes.push(GroupedTaskViewNode::Group {
                        group: group.clone(),
                        tasks: filtered,
                        sort_order: *sort_order,
                    });
                }
            }
        }
    }

    if changed {
        GroupedTaskView { nodes }
    } else {
        view.clone()
    }
}

/// `findTaskInGroupedView`（真源 :134-150）：按 key 找 task 本体，找不到返回 `None`。
pub fn find_task_in_grouped_view<'a>(
    view: &'a GroupedTaskView,
    target_task_key: &str,
) -> Option<&'a TaskListItem> {
    for node in &view.nodes {
        match node {
            GroupedTaskViewNode::Task { task, .. } => {
                if task_key_of(task) == target_task_key {
                    return Some(task);
                }
            }
            GroupedTaskViewNode::Group { tasks, .. } => {
                if let Some(found) = tasks.iter().find(|t| task_key_of(t) == target_task_key) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// `replaceTaskInGroupedView`（真源 :152-168）：用新 task 替换同 key 的旧 task
/// （乐观更新：状态/标题变化后回填，不改结构）。
pub fn replace_task_in_grouped_view(
    view: &GroupedTaskView,
    next_task: TaskListItem,
) -> GroupedTaskView {
    let target_task_key = task_key_of(&next_task);
    GroupedTaskView {
        nodes: view
            .nodes
            .iter()
            .map(|node| match node {
                GroupedTaskViewNode::Task { task, sort_order } => {
                    if task_key_of(task) == target_task_key {
                        GroupedTaskViewNode::Task {
                            task: next_task.clone(),
                            sort_order: *sort_order,
                        }
                    } else {
                        node.clone()
                    }
                }
                GroupedTaskViewNode::Group {
                    group,
                    tasks,
                    sort_order,
                } => GroupedTaskViewNode::Group {
                    group: group.clone(),
                    tasks: tasks
                        .iter()
                        .map(|t| {
                            if task_key_of(t) == target_task_key {
                                next_task.clone()
                            } else {
                                t.clone()
                            }
                        })
                        .collect(),
                    sort_order: *sort_order,
                },
            })
            .collect(),
    }
}

/// `getGroupedTaskGroupIds`（真源 :170-172）：按顶层顺序列出所有 group id。
pub fn get_grouped_task_group_ids(view: &GroupedTaskView) -> Vec<String> {
    view.nodes
        .iter()
        .filter_map(|node| match node {
            GroupedTaskViewNode::Group { group, .. } => Some(group.group_id.clone()),
            GroupedTaskViewNode::Task { .. } => None,
        })
        .collect()
}

/// `areAllGroupedTaskGroupsExpanded`（真源 :174-179）。
///
/// 注意真源 :178 的 `groupIds.length > 0`：一个组都没有时返回 **false**
/// （不是 vacuous true）。这条语义要保留——它被用来决定是否显示「全部折叠」
/// 之类的动作。
pub fn are_all_grouped_task_groups_expanded(
    group_ids: &[String],
    collapsed_group_ids: &HashSet<String>,
) -> bool {
    !group_ids.is_empty() && group_ids.iter().all(|id| !collapsed_group_ids.contains(id))
}

/// `pruneCollapsedGroupedTaskGroupIds`（真源 :181-187）：清理折叠态里已不存在的组 id
/// （组被删除后不留脏key）。
pub fn prune_collapsed_grouped_task_group_ids(
    collapsed_group_ids: &HashSet<String>,
    group_ids: &[String],
) -> HashSet<String> {
    let known: HashSet<&str> = group_ids.iter().map(|s| s.as_str()).collect();
    collapsed_group_ids
        .iter()
        .filter(|id| known.contains(id.as_str()))
        .cloned()
        .collect()
}

/// `getTaskGroupIdInView`（真源 :189-199）：查 task 所在组 id，顶层返回 `None`。
pub fn get_task_group_id_in_view(view: &GroupedTaskView, target_task_key: &str) -> Option<String> {
    for node in &view.nodes {
        if let GroupedTaskViewNode::Group { group, tasks, .. } = node {
            if tasks.iter().any(|t| task_key_of(t) == target_task_key) {
                return Some(group.group_id.clone());
            }
        }
    }
    None
}

/// `findTopLevelTaskIndex`（真源 :217-221）。
fn find_top_level_task_index(view: &GroupedTaskView, target_task_key: &str) -> Option<usize> {
    view.nodes.iter().position(
        |node| matches!(node, GroupedTaskViewNode::Task { task, .. } if task_key_of(task) == target_task_key),
    )
}

/// `findGroupIndex`（真源 :223-225）。
fn find_group_index(view: &GroupedTaskView, group_id: &str) -> Option<usize> {
    view.nodes.iter().position(
        |node| matches!(node, GroupedTaskViewNode::Group { group, .. } if group.group_id == group_id),
    )
}

/// `findGroupIdByTaskKey`（真源 :227-237）。
fn find_group_id_by_task_key(view: &GroupedTaskView, target_task_key: &str) -> Option<String> {
    for node in &view.nodes {
        if let GroupedTaskViewNode::Group { group, tasks, .. } = node {
            if tasks.iter().any(|t| task_key_of(t) == target_task_key) {
                return Some(group.group_id.clone());
            }
        }
    }
    None
}

/// `findTaskLocation`（真源 :239-269）：定位 task 的层级与下标。
///
/// 真源 :254-257 有个细节：组内命中但取出的 task 为 undefined 时**直接返回 null**
/// （而不是继续找），Rust 侧对应 `Option` 的 `None` 早退。
pub fn find_task_location(view: &GroupedTaskView, target_task_key: &str) -> Option<TaskLocation> {
    for (node_index, node) in view.nodes.iter().enumerate() {
        match node {
            GroupedTaskViewNode::Task { task, .. } => {
                if task_key_of(task) == target_task_key {
                    return Some(TaskLocation::Root { node_index });
                }
            }
            GroupedTaskViewNode::Group { group, tasks, .. } => {
                if let Some(task_index) =
                    tasks.iter().position(|t| task_key_of(t) == target_task_key)
                {
                    return Some(TaskLocation::Group {
                        group_id: group.group_id.clone(),
                        node_index,
                        task_index,
                    });
                }
            }
        }
    }
    None
}

/// `resolveGroupedDraftTaskPlacementForTask`（真源 :271-282）：新草稿任务的落位。
///
/// 无参照 task → `Top`；参照 task 在组内 → 跟到那个组；参照在顶层或不存在 → `Top`。
pub fn resolve_grouped_draft_task_placement_for_task(
    view: &GroupedTaskView,
    target_task_key: Option<&str>,
) -> DraftPlacement {
    let Some(key) = target_task_key.filter(|k| !k.is_empty()) else {
        return DraftPlacement::Top;
    };
    match find_task_location(view, key) {
        Some(location) if location.is_in_group() => DraftPlacement::Group {
            group_id: location.group_id().unwrap_or_default().to_string(),
        },
        _ => DraftPlacement::Top,
    }
}

/// `insertTaskNearTask`（真源 :284-311）：在参照 task 的前/后插入。
///
/// 父级跟随参照 task 所在层级：参照在顶层 → 插顶层；在组内 → 插该组。
pub fn insert_task_near_task(
    view: &GroupedTaskView,
    task: TaskListItem,
    over_task_key: &str,
    position: InsertPosition,
) -> GroupedTaskView {
    let mut nodes = clone_nodes(view);
    let Some(over_location) = find_task_location(
        &GroupedTaskView {
            nodes: nodes.clone(),
        },
        over_task_key,
    ) else {
        return GroupedTaskView { nodes };
    };

    match over_location {
        TaskLocation::Root { node_index } => {
            let insert_index = node_index + position.offset();
            // 真源 splice 越界时钳到末尾；Rust 的 Vec::insert 会 panic，需显式钳位。
            let insert_index = insert_index.min(nodes.len());
            nodes.insert(
                insert_index,
                GroupedTaskViewNode::Task {
                    task,
                    sort_order: None,
                },
            );
        }
        TaskLocation::Group {
            node_index,
            task_index,
            ..
        } => {
            let mut next_node = clone_group_node(&nodes[node_index]);
            if let GroupedTaskViewNode::Group { tasks, .. } = &mut next_node {
                let insert_index = (task_index + position.offset()).min(tasks.len());
                tasks.insert(insert_index, task);
            }
            nodes[node_index] = next_node;
        }
    }
    GroupedTaskView { nodes }
}

/// `moveTaskOverTask`（真源 :313-338）：task 拖到另一个 task 的前/后。
///
/// **顺序是关键**（真源 :335-336 注释）：先`removeTaskFromView` 再
/// `insertTaskNearTask`。若先按 over 算好插入下标再移除，同组内向下移动时
/// 旧下标会把插入点推后一格。
pub fn move_task_over_task(
    view: &GroupedTaskView,
    active_task_key: &str,
    over_task_key: &str,
    position: InsertPosition,
) -> GroupedTaskView {
    if active_task_key == over_task_key {
        return view.clone();
    }
    // 真源 :324-328 先确认两端都存在，任一缺失即原样返回。
    if find_task_location(view, active_task_key).is_none()
        || find_task_location(view, over_task_key).is_none()
    {
        return view.clone();
    }

    let (next_view, task) = remove_task_from_view(view, active_task_key);
    let Some(task) = task else {
        return view.clone();
    };
    insert_task_near_task(&next_view, task, over_task_key, position)
}

/// `insertTaskAroundGroup`（真源 :340-353）：在某个组的前/后插入顶层 task。
fn insert_task_around_group(
    view: &GroupedTaskView,
    task: TaskListItem,
    group_id: &str,
    position: InsertPosition,
) -> GroupedTaskView {
    let mut nodes = clone_nodes(view);
    let Some(group_index) = find_group_index(
        &GroupedTaskView {
            nodes: nodes.clone(),
        },
        group_id,
    ) else {
        return GroupedTaskView { nodes };
    };
    let insert_index = (group_index + position.offset()).min(nodes.len());
    nodes.insert(
        insert_index,
        GroupedTaskViewNode::Task {
            task,
            sort_order: None,
        },
    );
    GroupedTaskView { nodes }
}

/// `moveTaskToRootAroundGroup`（真源 :355-376）：把组内 task 拖到组外的组前/后。
pub fn move_task_to_root_around_group(
    view: &GroupedTaskView,
    active_task_key: &str,
    group_id: &str,
    position: InsertPosition,
) -> GroupedTaskView {
    if find_group_index(view, group_id).is_none() {
        return view.clone();
    }

    let (next_view, task) = remove_task_from_view(view, active_task_key);
    let Some(task) = task else {
        return view.clone();
    };
    insert_task_around_group(&next_view, task, group_id, position)
}

/// `moveTaskToGroupStart`（真源 :378-397）：把 task 移到组内首位。
pub fn move_task_to_group_start(
    view: &GroupedTaskView,
    active_task_key: &str,
    group_id: &str,
) -> GroupedTaskView {
    let (next_view, task) = remove_task_from_view(view, active_task_key);
    let Some(task) = task else {
        return view.clone();
    };
    let Some(node) = next_view.nodes.iter().find(
        |node| matches!(node, GroupedTaskViewNode::Group { group, .. } if group.group_id == group_id),
    ) else {
        // 真源 :392-394：移除后找不到组（组本身不存在）→ 返回**原始** view。
        return view.clone();
    };
    let first_task_key = match node {
        GroupedTaskViewNode::Group { tasks, .. } => tasks.first().map(task_key_of),
        _ => None,
    };
    insert_task_into_group(&next_view, task, group_id, first_task_key.as_deref())
}

/// `moveTaskToGroupEnd`（真源 :399-420）：把 task 追加到组末。
pub fn move_task_to_group_end(
    view: &GroupedTaskView,
    active_task_key: &str,
    group_id: &str,
) -> GroupedTaskView {
    let (next_view, task) = remove_task_from_view(view, active_task_key);
    let Some(task) = task else {
        return view.clone();
    };
    let Some(group_index) = find_group_index(&next_view, group_id) else {
        return view.clone();
    };
    let mut next_node = clone_group_node(&next_view.nodes[group_index]);
    if let GroupedTaskViewNode::Group { tasks, .. } = &mut next_node {
        tasks.push(task);
    }
    let mut nodes = clone_nodes(&next_view);
    nodes[group_index] = next_node;
    GroupedTaskView { nodes }
}

/// `moveGroupAroundTopLevelNode`（真源 :422-471）：group 拖到另一个顶层节点前/后。
///
/// 两处真源注释必须保留：
/// - :441-443 悬停目标是组内 task 时要归一化为所属 group，否则 content 区域
///   不触发 group over group；
/// - 真源 :456 命中 active 自身/同组时直接返回原 view。
pub fn move_group_around_top_level_node(
    view: &GroupedTaskView,
    active_group_id: &str,
    over: &DragOver,
    position: InsertPosition,
) -> GroupedTaskView {
    // 真源 :438-440：over 就是自己 → no-op。
    if let DragOver::Group { group_id } = over {
        if active_group_id == group_id {
            return view.clone();
        }
    }
    // 真源 :443-447：over 是组内 task → 归一化成所属 group；若就是本组则 no-op。
    let over_group_id = match over {
        DragOver::Group { .. } => None,
        DragOver::Task { task_key } => find_group_id_by_task_key(view, task_key),
    };
    if over_group_id.as_deref() == Some(active_group_id) {
        return view.clone();
    }

    let mut nodes = clone_nodes(view);
    let Some(active_index) = nodes.iter().position(
        |node| matches!(node, GroupedTaskViewNode::Group { group, .. } if group.group_id == active_group_id),
    ) else {
        return view.clone();
    };
    let active_node = nodes.remove(active_index);
    debug_assert!(matches!(active_node, GroupedTaskViewNode::Group { .. }));

    let next_view = GroupedTaskView { nodes };
    // 真源 :460-465：overIndex 按over 类型分三路求。
    let over_index = match over {
        DragOver::Group { group_id } => find_group_index(&next_view, group_id),
        DragOver::Task { task_key } => match &over_group_id {
            Some(gid) => find_group_index(&next_view, gid),
            None => find_top_level_task_index(&next_view, task_key),
        },
    };
    let Some(over_index) = over_index else {
        return view.clone();
    };

    let mut nodes = next_view.nodes;
    let insert_index = (over_index + position.offset()).min(nodes.len());
    nodes.insert(insert_index, active_node);
    GroupedTaskView { nodes }
}

/// `getNodeIdAfterGroup`（真源 :473-481）：取组后面那个顶层节点。
fn get_node_after_group(view: &GroupedTaskView, group_id: &str) -> Option<GroupedTaskViewNode> {
    let group_index = find_group_index(view, group_id)?;
    view.nodes.get(group_index + 1).cloned()
}

/// `insertTaskBeforeTopLevelNode`（真源 :483-502）：在指定顶层节点前插入 task。
///
/// `before_node` 为 `None` → 追加到末尾；节点已不存在 → 也追加到末尾
/// （真源 :497 的 `insertIndex < 0 ? nodes.length : insertIndex`）。
fn insert_task_before_top_level_node(
    view: &GroupedTaskView,
    task: TaskListItem,
    before_node: Option<&GroupedTaskViewNode>,
) -> GroupedTaskView {
    let mut nodes = clone_nodes(view);
    let insert_index = match before_node {
        None => nodes.len(),
        Some(before) => {
            let found = nodes.iter().position(|node| match (node, before) {
                (
                    GroupedTaskViewNode::Group { group, .. },
                    GroupedTaskViewNode::Group {
                        group: before_group,
                        ..
                    },
                ) => group.group_id == before_group.group_id,
                (
                    GroupedTaskViewNode::Task { task, .. },
                    GroupedTaskViewNode::Task {
                        task: before_task, ..
                    },
                ) => task_key_of(task) == task_key_of(before_task),
                _ => false,
            });
            found.unwrap_or(nodes.len())
        }
    };
    nodes.insert(
        insert_index,
        GroupedTaskViewNode::Task {
            task,
            sort_order: None,
        },
    );
    GroupedTaskView { nodes }
}

/// `moveTaskByMenu`（真源 :504-526）：右键菜单「移动到分组」。
///
/// 真源 :523-524 注释说明了移出分组时的落位策略：
/// 「菜单移出分组时把 task 放到原 group 后面，避免用户刚操作完就丢失空间上下文。」
/// 这是个体验细节，不是随手写的。
pub fn move_task_by_menu(
    view: &GroupedTaskView,
    target_task_key: &str,
    target_group_id: Option<&str>,
) -> GroupedTaskView {
    let current_group_id = get_task_group_id_in_view(view, target_task_key);
    if current_group_id.as_deref() == target_group_id {
        return view.clone();
    }

    let (next_view, task) = remove_task_from_view(view, target_task_key);
    let Some(task) = task else {
        return view.clone();
    };
    match target_group_id {
        Some(group_id) => insert_task_into_group(&next_view, task, group_id, None),
        None => {
            let before_node = current_group_id
                .as_deref()
                .and_then(|gid| get_node_after_group(view, gid));
            insert_task_before_top_level_node(&next_view, task, before_node.as_ref())
        }
    }
}

/// `moveTaskToTopByMenu`（真源 :528-561）：右键菜单「移到顶部」。
///
/// 真源 :533-540 的短路条件：已经在顶层首位、或已在组内首位 → no-op。
pub fn move_task_to_top_by_menu(view: &GroupedTaskView, target_task_key: &str) -> GroupedTaskView {
    let Some(location) = find_task_location(view, target_task_key) else {
        return view.clone();
    };
    let already_top = match &location {
        TaskLocation::Root { node_index } => *node_index == 0,
        TaskLocation::Group { task_index, .. } => *task_index == 0,
    };
    if already_top {
        return view.clone();
    }

    let group_id = location.group_id().map(|s| s.to_string());
    let (next_view, task) = remove_task_from_view(view, target_task_key);
    let Some(task) = task else {
        return view.clone();
    };

    if let Some(group_id) = group_id {
        let first_task_key = next_view
            .nodes
            .iter()
            .find(
                |node| matches!(node, GroupedTaskViewNode::Group { group, .. } if group.group_id == group_id),
            )
            .and_then(|node| match node {
                GroupedTaskViewNode::Group { tasks, .. } => tasks.first().map(task_key_of),
                _ => None,
            });
        return insert_task_into_group(&next_view, task, &group_id, first_task_key.as_deref());
    }

    // 真源 :558-560：顶层任务直接插到整个 nodes 头部。
    let mut nodes = vec![GroupedTaskViewNode::Task {
        task,
        sort_order: None,
    }];
    nodes.extend(next_view.nodes);
    GroupedTaskView { nodes }
}

/// `taskKey` 的便捷版：从 `TaskListItem` 直接取 key。
///
/// 视图内所有查找/比较都走这个函数，保证 key 生成口径唯一。
pub fn task_key_of(task: &TaskListItem) -> String {
    task_key(
        &task.workspace_path,
        task.workspace_identity.as_deref(),
        &task.task_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 测试夹具 ──

    fn task(workspace_path: &str, identity: Option<&str>, task_id: &str) -> TaskListItem {
        TaskListItem {
            task_id: task_id.into(),
            title: format!("task {task_id}"),
            workspace_path: workspace_path.into(),
            workspace_identity: identity.map(|s| s.to_string()),
            created_at: 0,
            updated_at: 0,
            status: String::new(),
        }
    }

    fn group_node(id: &str, tasks: Vec<TaskListItem>) -> GroupedTaskViewNode {
        GroupedTaskViewNode::Group {
            group: TaskGroup {
                group_id: id.into(),
                title: id.into(),
                color: "blue".into(),
            },
            tasks,
            sort_order: None,
        }
    }

    fn task_node(t: TaskListItem) -> GroupedTaskViewNode {
        GroupedTaskViewNode::Task {
            task: t,
            sort_order: None,
        }
    }

    /// 视图形状：`[t1, t2, g1[t3, t4], t5]`
    fn sample_view() -> GroupedTaskView {
        GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", None, "t1")),
                task_node(task("/ws", None, "t2")),
                group_node("g1", vec![task("/ws", None, "t3"), task("/ws", None, "t4")]),
                task_node(task("/ws", None, "t5")),
            ],
        }
    }

    /// 把视图压成可断言的形状字符串：`t1,t2,g1[t3,t4],t5`
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

    fn key(id: &str) -> String {
        task_key("/ws", None, id)
    }

    // ── taskKey / ids.ts ──

    #[test]
    fn task_key_uses_nul_separator_and_path_workspace() {
        // 真源 ids.ts：workspace 段 + \0 + taskId。
        assert_eq!(key("t1"), "/ws\u{0}t1");
    }

    #[test]
    fn task_key_prefers_trimmed_identity_over_path() {
        // 真源 resolveWorkspaceStateKey：identity?.trim() || path。
        assert_eq!(
            task_key("/ws", Some("  ssh://box/p  "), "t1"),
            "ssh://box/p\u{0}t1"
        );
    }

    #[test]
    fn task_key_falls_back_to_path_when_identity_blank() {
        // 空串 /纯空白 /None 三种情况都回退 path。
        assert_eq!(task_key("/ws", Some(""), "t1"), key("t1"));
        assert_eq!(task_key("/ws", Some("   \t "), "t1"), key("t1"));
        assert_eq!(task_key("/ws", None, "t1"), key("t1"));
    }

    #[test]
    fn same_task_id_in_different_workspaces_yields_different_keys() {
        // 这是 taskKey 必须带 workspace 前缀的根本原因（跨窗口同taskId）。
        let a = task_key("/ws-a", None, "t1");
        let b = task_key("/ws-b", None, "t1");
        assert_ne!(a, b);
        // 同路径但不同 identity 也必须区分（远程 workspace 隔离）。
        let c = task_key("/ws", Some("ssh://box-a/p"), "t1");
        let d = task_key("/ws", Some("ssh://box-b/p"), "t1");
        assert_ne!(c, d);
    }

    // ── removeTaskFromView ──

    #[test]
    fn remove_task_from_top_level() {
        let view = sample_view();
        let (next, removed) = remove_task_from_view(&view, &key("t2"));
        assert_eq!(shape(&next), "t1,g1[t3,t4],t5");
        assert_eq!(removed.map(|t| t.task_id), Some("t2".into()));
    }

    #[test]
    fn remove_task_from_inside_group_keeps_others() {
        let view = sample_view();
        let (next, removed) = remove_task_from_view(&view, &key("t3"));
        assert_eq!(shape(&next), "t1,t2,g1[t4],t5");
        assert_eq!(removed.map(|t| t.task_id), Some("t3".into()));
    }

    #[test]
    fn remove_missing_task_returns_none_and_keeps_shape() {
        let view = sample_view();
        let (next, removed) = remove_task_from_view(&view, &key("nope"));
        assert!(removed.is_none());
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn remove_does_not_mutate_input_view() {
        let view = sample_view();
        let before = shape(&view);
        let _ = remove_task_from_view(&view, &key("t3"));
        assert_eq!(shape(&view), before, "入参视图不应被 splice 污染");
    }

    // ── insertTaskIntoGroup ──

    #[test]
    fn insert_into_group_appends_when_no_anchor() {
        let view = sample_view();
        let next = insert_task_into_group(&view, task("/ws", None, "t9"), "g1", None);
        assert_eq!(shape(&next), "t1,t2,g1[t3,t4,t9],t5");
    }

    #[test]
    fn insert_into_group_before_anchor() {
        let view = sample_view();
        let next = insert_task_into_group(&view, task("/ws", None, "t9"), "g1", Some(&key("t4")));
        assert_eq!(shape(&next), "t1,t2,g1[t3,t9,t4],t5");
    }

    #[test]
    fn insert_into_group_with_unknown_anchor_appends() {
        // 真源 :72：findIndex 返回 -1 时追加到组末，不是插到头部。
        let view = sample_view();
        let next =
            insert_task_into_group(&view, task("/ws", None, "t9"), "g1", Some(&key("ghost")));
        assert_eq!(shape(&next), "t1,t2,g1[t3,t4,t9],t5");
    }

    #[test]
    fn insert_into_unknown_group_is_noop() {
        let view = sample_view();
        let next = insert_task_into_group(&view, task("/ws", None, "t9"), "ghost", None);
        assert_eq!(shape(&next), shape(&view));
    }

    // ── 同组向下移动：验证真源注释里那个"偏移一格"的坑 ──

    #[test]
    fn move_task_down_within_same_group_does_not_skip() {
        // g1[t3,t4] 把 t3 拖到 t4 之后，应得 g1[t4,t3]，而不是 g1[t4] 后面插到组外。
        let view = GroupedTaskView {
            nodes: vec![group_node(
                "g1",
                vec![task("/ws", None, "t3"), task("/ws", None, "t4")],
            )],
        };
        let next = move_task_over_task(&view, &key("t3"), &key("t4"), InsertPosition::After);
        assert_eq!(shape(&next), "g1[t4,t3]");
    }

    #[test]
    fn move_task_up_within_same_group() {
        let view = GroupedTaskView {
            nodes: vec![group_node(
                "g1",
                vec![task("/ws", None, "t3"), task("/ws", None, "t4")],
            )],
        };
        let next = move_task_over_task(&view, &key("t4"), &key("t3"), InsertPosition::Before);
        assert_eq!(shape(&next), "g1[t4,t3]");
    }

    #[test]
    fn move_top_level_task_before_top_level_task() {
        let view = sample_view();
        let next = move_task_over_task(&view, &key("t1"), &key("t5"), InsertPosition::Before);
        assert_eq!(shape(&next), "t2,g1[t3,t4],t1,t5");
    }

    #[test]
    fn move_group_task_to_top_level_promotes_it() {
        // 拖到顶层 task 之前 → task 升为顶层游离节点。
        let view = sample_view();
        let next = move_task_over_task(&view, &key("t3"), &key("t1"), InsertPosition::Before);
        assert_eq!(shape(&next), "t3,t1,t2,g1[t4],t5");
    }

    #[test]
    fn move_onto_itself_is_noop() {
        let view = sample_view();
        let next = move_task_over_task(&view, &key("t3"), &key("t3"), InsertPosition::After);
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn move_with_unknown_anchor_is_noop() {
        let view = sample_view();
        let next = move_task_over_task(&view, &key("t1"), &key("ghost"), InsertPosition::After);
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn move_from_group_to_its_own_end_keeps_membership() {
        let view = sample_view();
        let next = move_task_to_group_end(&view, &key("t3"), "g1");
        assert_eq!(shape(&next), "t1,t2,g1[t4,t3],t5");
    }

    #[test]
    fn move_to_group_start() {
        let view = sample_view();
        let next = move_task_to_group_start(&view, &key("t5"), "g1");
        assert_eq!(shape(&next), "t1,t2,g1[t5,t3,t4]");
    }

    #[test]
    fn move_to_unknown_group_returns_original_view() {
        let view = sample_view();
        let next = move_task_to_group_start(&view, &key("t5"), "ghost");
        assert_eq!(shape(&next), shape(&view));
    }

    // ── moveTaskToRootAroundGroup ──

    #[test]
    fn move_task_out_of_group_after_that_group() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", None, "t3")]),
                task_node(task("/ws", None, "t9")),
            ],
        };
        let next = move_task_to_root_around_group(&view, &key("t3"), "g1", InsertPosition::After);
        assert_eq!(shape(&next), "g1[],t3,t9");
    }

    #[test]
    fn move_task_out_of_group_before_that_group() {
        let view = GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", None, "t9")),
                group_node("g1", vec![task("/ws", None, "t3")]),
            ],
        };
        let next = move_task_to_root_around_group(&view, &key("t3"), "g1", InsertPosition::Before);
        // 真源 :347 的 groupIndex 是在**移除后**的 nextView 里找的：此时 g1 已移到下标 0，
        // 所以 Before 插到下标 0 → t3 落在 t9 之后，而不是被顶到最前面。
        // 这个顺序与 moveTaskOverTask 里的注释（:335）同源：先移除再定位。
        assert_eq!(shape(&next), "t9,t3,g1[]");
    }

    // ── moveGroupAroundTopLevelNode ──

    #[test]
    fn move_group_after_another_group() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", None, "t3")]),
                group_node("g2", vec![task("/ws", None, "t4")]),
            ],
        };
        let next = move_group_around_top_level_node(
            &view,
            "g1",
            &DragOver::Group {
                group_id: "g2".into(),
            },
            InsertPosition::After,
        );
        assert_eq!(shape(&next), "g2[t4],g1[t3]");
    }

    #[test]
    fn group_dropped_onto_own_member_task_is_noop() {
        // 真源 :441-447：over 是组内 task 且属于本组 → 归一化为本组 → no-op。
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", None, "t3")]),
                group_node("g2", vec![task("/ws", None, "t4")]),
            ],
        };
        let next = move_group_around_top_level_node(
            &view,
            "g1",
            &DragOver::Task {
                task_key: key("t3"),
            },
            InsertPosition::After,
        );
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn group_dropped_on_other_groups_member_targets_that_group() {
        // over 是 g2 的成员 task → 归一化为 g2，g1 排到 g2 之后。
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", None, "t3")]),
                group_node("g2", vec![task("/ws", None, "t4")]),
            ],
        };
        let next = move_group_around_top_level_node(
            &view,
            "g1",
            &DragOver::Task {
                task_key: key("t4"),
            },
            InsertPosition::After,
        );
        assert_eq!(shape(&next), "g2[t4],g1[t3]");
    }

    #[test]
    fn group_dropped_onto_itself_is_noop() {
        let view = sample_view();
        let next = move_group_around_top_level_node(
            &view,
            "g1",
            &DragOver::Group {
                group_id: "g1".into(),
            },
            InsertPosition::After,
        );
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn group_moved_before_top_level_task() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", None, "t3")]),
                task_node(task("/ws", None, "t9")),
            ],
        };
        let next = move_group_around_top_level_node(
            &view,
            "g1",
            &DragOver::Task {
                task_key: key("t9"),
            },
            InsertPosition::Before,
        );
        assert_eq!(shape(&next), "g1[t3],t9");
    }

    // ── 菜单移动 ──

    #[test]
    fn menu_move_into_group_appends() {
        let view = sample_view();
        let next = move_task_by_menu(&view, &key("t1"), Some("g1"));
        assert_eq!(shape(&next), "t2,g1[t3,t4,t1],t5");
    }

    #[test]
    fn menu_move_to_same_group_is_noop() {
        let view = sample_view();
        let next = move_task_by_menu(&view, &key("t3"), Some("g1"));
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn menu_move_out_of_group_places_task_after_its_group() {
        // 真源 :523-524：移出分组落到原组之后，保住空间上下文。
        let view = GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", None, "t1")),
                group_node("g1", vec![task("/ws", None, "t3")]),
                task_node(task("/ws", None, "t5")),
            ],
        };
        let next = move_task_by_menu(&view, &key("t3"), None);
        assert_eq!(shape(&next), "t1,g1[],t3,t5");
    }

    #[test]
    fn menu_move_out_of_last_group_lands_after_that_group() {
        // 原组后面还有一个游离 task：真源 :524 的 beforeNode 取自**原 view**
        //（getNodeIdAfterGroup(view, ...)），拿到 t1，再插到 t1 之前 → 紧跟原组。
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", None, "t3")]),
                task_node(task("/ws", None, "t1")),
            ],
        };
        let next = move_task_by_menu(&view, &key("t3"), None);
        assert_eq!(shape(&next), "g1[],t3,t1");
    }

    #[test]
    fn menu_move_out_of_trailing_group_appends_to_end() {
        // 原组是最后一个节点 → getNodeIdAfterGroup 返回 null → 追加到末尾
        // （真源 :497 的 insertIndex < 0 ? nodes.length 分支）。
        let view = GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", None, "t1")),
                group_node("g1", vec![task("/ws", None, "t3")]),
            ],
        };
        let next = move_task_by_menu(&view, &key("t3"), None);
        assert_eq!(shape(&next), "t1,g1[],t3");
    }

    #[test]
    fn menu_move_top_of_top_level_is_noop() {
        let view = sample_view();
        let next = move_task_to_top_by_menu(&view, &key("t1"));
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn menu_move_top_of_lower_top_level_goes_to_global_front() {
        // t5 在顶层且不在首位 → 插到整个 nodes 头部（真源 :558-560）。
        let view = sample_view();
        let next = move_task_to_top_by_menu(&view, &key("t5"));
        assert_eq!(shape(&next), "t5,t1,t2,g1[t3,t4]");
    }

    #[test]
    fn menu_move_top_of_grouped_to_group_start() {
        // t4 是组内第二位 → 「移到顶部」= 组内升到首位（真源 :547-555），
        // 而不是把它拽出组放到全局最前。
        let view = sample_view();
        let next = move_task_to_top_by_menu(&view, &key("t4"));
        assert_eq!(shape(&next), "t1,t2,g1[t4,t3],t5");
    }

    #[test]
    fn menu_move_to_top_when_already_first_in_group_is_noop() {
        let view = sample_view();
        let next = move_task_to_top_by_menu(&view, &key("t3"));
        // t3 已是组内首位 → 组视角无变化；这是 no-op 分支（node_index!=0 但 task_index==0）。
        assert_eq!(shape(&next), "t1,t2,g1[t3,t4],t5");
    }

    // ── 查找类 ──

    #[test]
    fn find_task_across_levels() {
        let view = sample_view();
        assert_eq!(
            find_task_in_grouped_view(&view, &key("t4")).map(|t| t.task_id.clone()),
            Some("t4".into())
        );
        assert!(find_task_in_grouped_view(&view, &key("ghost")).is_none());
    }

    #[test]
    fn find_location_distinguishes_root_and_group() {
        let view = sample_view();
        assert_eq!(
            find_task_location(&view, &key("t1")),
            Some(TaskLocation::Root { node_index: 0 })
        );
        assert_eq!(
            find_task_location(&view, &key("t3")),
            Some(TaskLocation::Group {
                group_id: "g1".into(),
                node_index: 2,
                task_index: 0,
            })
        );
        assert_eq!(find_task_location(&view, &key("ghost")), None);
    }

    #[test]
    fn task_group_id_lookup_by_task() {
        let view = sample_view();
        assert_eq!(
            get_task_group_id_in_view(&view, &key("t3")),
            Some("g1".into())
        );
        assert_eq!(get_task_group_id_in_view(&view, &key("t1")), None);
    }

    #[test]
    fn get_grouped_task_group_ids_in_order() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![]),
                task_node(task("/ws", None, "t1")),
                group_node("g2", vec![]),
            ],
        };
        assert_eq!(
            get_grouped_task_group_ids(&view),
            vec!["g1".to_string(), "g2".to_string()]
        );
    }

    // ── 过滤 / 替换 / 移除 ──

    #[test]
    fn filter_removes_tasks_at_both_levels() {
        let view = sample_view();
        let hidden: HashSet<String> = [key("t1"), key("t3")].into_iter().collect();
        let next = filter_grouped_view_by_task_keys(&view, &hidden);
        assert_eq!(shape(&next), "t2,g1[t4],t5");
    }

    #[test]
    fn filter_with_unmatched_keys_keeps_everything() {
        let view = sample_view();
        let hidden: HashSet<String> = [key("ghost")].into_iter().collect();
        let next = filter_grouped_view_by_task_keys(&view, &hidden);
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn filter_with_empty_hidden_set_is_identity() {
        let view = sample_view();
        let next = filter_grouped_view_by_task_keys(&view, &HashSet::new());
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn replace_task_updates_payload_but_not_structure() {
        let view = sample_view();
        let mut updated = task("/ws", None, "t3");
        updated.title = "改过的标题".into();
        let next = replace_task_in_grouped_view(&view, updated);
        assert_eq!(shape(&next), shape(&view), "替换不应改变节点结构");
        assert_eq!(
            find_task_in_grouped_view(&next, &key("t3")).map(|t| t.title.clone()),
            Some("改过的标题".into())
        );
    }

    #[test]
    fn remove_task_from_grouped_view_at_both_levels() {
        let view = sample_view();
        assert_eq!(
            shape(&remove_task_from_grouped_view(&view, &key("t4"))),
            "t1,t2,g1[t3],t5"
        );
        assert_eq!(
            shape(&remove_task_from_grouped_view(&view, &key("t2"))),
            "t1,g1[t3,t4],t5"
        );
        assert_eq!(
            shape(&remove_task_from_grouped_view(&view, &key("zz"))),
            shape(&view)
        );
    }

    // ── 折叠态 ──

    #[test]
    fn all_expanded_requires_non_empty_group_list() {
        // 真源 :178：空组列表返回 false，不是 vacuous true。
        assert!(!are_all_grouped_task_groups_expanded(&[], &HashSet::new()));
        let ids = vec!["g1".to_string(), "g2".to_string()];
        assert!(are_all_grouped_task_groups_expanded(&ids, &HashSet::new()));
        let collapsed: HashSet<String> = ["g1".into()].into_iter().collect();
        assert!(!are_all_grouped_task_groups_expanded(&ids, &collapsed));
    }

    #[test]
    fn prune_drops_unknown_collapsed_ids() {
        let collapsed: HashSet<String> = ["g1".into(), "ghost".into()].into_iter().collect();
        let pruned = prune_collapsed_grouped_task_group_ids(&collapsed, &["g1".to_string()]);
        assert_eq!(pruned.len(), 1);
        assert!(pruned.contains("g1"));
    }

    // ── 草稿落位 ──

    #[test]
    fn draft_placement_follows_anchor_task() {
        let view = sample_view();
        assert_eq!(
            resolve_grouped_draft_task_placement_for_task(&view, Some(&key("t3"))),
            DraftPlacement::Group {
                group_id: "g1".into()
            }
        );
        assert_eq!(
            resolve_grouped_draft_task_placement_for_task(&view, Some(&key("t1"))),
            DraftPlacement::Top
        );
    }

    #[test]
    fn draft_placement_without_anchor_is_top() {
        let view = sample_view();
        assert_eq!(
            resolve_grouped_draft_task_placement_for_task(&view, None),
            DraftPlacement::Top
        );
        assert_eq!(
            resolve_grouped_draft_task_placement_for_task(&view, Some("")),
            DraftPlacement::Top
        );
    }

    // ── 空视图边界 ──

    #[test]
    fn empty_view_operations_are_safe() {
        let view = GroupedTaskView { nodes: vec![] };
        assert!(get_grouped_task_group_ids(&view).is_empty());
        assert!(find_task_in_grouped_view(&view, &key("x")).is_none());
        assert_eq!(find_task_location(&view, &key("x")), None);
        assert!(
            move_task_over_task(&view, &key("x"), &key("y"), InsertPosition::After)
                .nodes
                .is_empty()
        );
        assert!(
            remove_task_from_grouped_view(&view, &key("x"))
                .nodes
                .is_empty()
        );
    }
}
