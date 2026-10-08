//! 1:1 翻译 `packages/ui/src/WorkspaceGroupedTasksSection.tsx` 的拖拽**纯逻辑**部分。
//!
//! ## 迁移边界（重要）
//!
//! 真源的拖拽分两层，本文件只迁**可脱离 React/DOM 测试的那层**：
//!
//! | 层 | 真源位置 | 本文件 |
//! |---|---|---|
//! | 落位决策（over 落在哪 → 视图怎么变） | :240-345 的 6 个 preview 函数 + `getGroupedTaskViewSignature` | ✅ 已迁 |
//! | 碰撞检测 / 拖拽源注册 / DragOverlay | dnd-kit `DndContext` + `useDraggable/useDroppable` | ❌ 未迁 |
//!
//! 之所以能这样切：dnd-kit 的 `event.active.data.current` / `event.over.data.current`
//! 只是 `{ type, taskKey|groupId }` 这样的**纯数据**，真正决定「视图怎么变」的是
//! 本文件这层。真源 :1315-1360 `applyGroupedTaskDragOverPreview` 的 7 层嵌套分发，
//! 每一层都只做「解包 over 标识 → 调view.rs 的某个 move 函数」。
//!
//! 事件适配层（把 dnd-kit / DOM 事件转成本文件的 [`DragOverSpec`]）留待后续，
//! 需要先在 Leptos 侧选定拖拽方案（原生 HTML5 DnD 还是自实现指针事件）。
//!
//! ## 真源为什么用「多层套娃」而不是一个大 switch
//!
//! :1324-1349 的结构是：
//! ```text
//! activeGroupId ?  group over group → group over task
//!                : task over 折叠组 → 组头 → 组尾 → 空投放区 → task over task
//! ```
//! 每一层都是「不适用就原样返回 view」。这个早退链**有顺序语义**：例如 task
//! 拖到组头时，`getGroupedTaskOverCollapsedGroupPreviewView` 先跑，未折叠的组
//! 不命中，落到组头分支。本文件用同样的链式顺序，不用 match 改写——
//! 顺序改了就可能命中不同的分支。

use super::view::{
    move_group_around_top_level_node, move_task_over_task, move_task_to_group_end,
    move_task_to_group_start, move_task_to_root_around_group, task_key_of, DragOver,
    GroupedTaskView, GroupedTaskViewNode, InsertPosition,
};

/// 拖拽方向（真源 `GroupedTaskDragDirectionPosition`）。
///
/// 真源由 `handleGroupedTaskDragMove`（:1362-1380）根据 `event.delta.y` 的
/// 符号变化维护：向下拖→ `after`，向上拖 → `before`。注意它比的是
/// `nextDeltaY` 与 `lastDragDeltaY`（增量而非绝对位置），所以是
/// 「本次移动方向」而非「指针在目标上方还是下方」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragDirection {
    Before,
    After,
}

impl DragDirection {
    fn to_position(self) -> InsertPosition {
        match self {
            DragDirection::Before => InsertPosition::Before,
            DragDirection::After => InsertPosition::After,
        }
    }
}

/// 被拖拽对象（真源 `event.active.data.current`，`:1317-1318`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragSource {
    Task { task_key: String },
    Group { group_id: String },
}

/// 悬停目标的**语义类型**（真源靠 `event.over.data.current.type` 区分）。
///
/// 真源有 5 种投放区，每种对应一个 preview 函数：
/// - `grouped-task` → task over task
/// - 组头 / 折叠组头 → 组头分支
/// - 组尾 → 组尾分支
/// - 空投放区 → 组内首位
/// - group 自身 / 组内容区 → group over group / group over task
///
/// 这里用枚举显式建模，替代真源里散落在各处的
/// `getGroupedTaskHeaderOverGroupId` 一类取值函数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropTarget {
    /// task 行本身（真源 grouped-task）。
    Task { task_key: String },
    /// 组头 / 折叠组头（真源 collapsedGroup / expandedGroupHeader）。
    GroupHeader { group_id: String },
    /// 组尾（真源 expandedGroupFooter）。
    GroupFooter { group_id: String },
    /// 空组内容区（真源 emptyDropZone）。
    EmptyGroupBody { group_id: String },
    /// group 节点本体（拖 group 时用）。
    Group { group_id: String },
}

/// 一次 dragOver 事件的完整语义输入（等价真源的 `DragOverEvent` 关键字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragOverSpec {
    pub active: DragSource,
    pub over: Option<DropTarget>,
    pub direction: DragDirection,
}

/// `getGroupedTaskViewSignature`（真源 :336-344）：把视图压成可比较的字符串。
///
/// 真源用法有两处，都是**幂等判据**：
/// - :1350-1354 dragOver 预览：next 与 current 签名相同 → 不触发重渲染；
/// - :1441 / :1478 dragEnd：next 与 origin 签名相同 → 认为没动，回滚不落库。
///
/// 因此这个函数的稳定性是正确性前提：任何重排都必须改变签名，
/// 任何 no-op 都必须保持签名不变。`sort_order` **不进签名**——真源就没放它，
/// 因为排序只影响落库顺序，不影响展示结构。
pub fn view_signature(view: &GroupedTaskView) -> String {
    view.nodes
        .iter()
        .map(|node| match node {
            GroupedTaskViewNode::Task { task, .. } => format!("t:{}", task_key_of(task)),
            GroupedTaskViewNode::Group { group, tasks, .. } => {
                let keys: Vec<String> = tasks.iter().map(task_key_of).collect();
                format!("g:{}[{}]", group.group_id, keys.join(","))
            }
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// 拖拽前后视图是否等价（真源两处签名比较的收敛形式）。
pub fn is_same_view(left: &GroupedTaskView, right: &GroupedTaskView) -> bool {
    view_signature(left) == view_signature(right)
}

// ── 6 个 preview 函数（真源 :240-334）──
// 每个都遵循同一形状：解包 active/over 标识 → 不适用则原样返回 → 调 view.rs。

/// `getGroupedTaskOverCollapsedGroupPreviewView`（真源 :196-214）。
///
/// 拖到**折叠组**上：展开该组并放进首位。真源注释说明这是折叠组的专属行为——
/// 折叠组没有展开的 content 区可落，只能借这次拖拽顺手展开。
pub fn preview_task_over_collapsed_group(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Task { task_key }, Some(DropTarget::GroupHeader { group_id })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    move_task_to_group_start(view, task_key, group_id)
}

/// `getGroupedTaskOverGroupHeaderPreviewView`（真源 :240-261）。
///
/// 拖到组头：向上拖 → 放到组**前面**（仍是顶层）；向下拖 → 放进组**首位**。
/// 这个非对称是真源刻意设计：向下表示「我要进这个组」。
pub fn preview_task_over_group_header(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Task { task_key }, Some(DropTarget::GroupHeader { group_id })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    match spec.direction {
        DragDirection::Before => move_task_to_root_around_group(
            view,
            task_key,
            group_id,
            InsertPosition::Before,
        ),
        DragDirection::After => move_task_to_group_start(view, task_key, group_id),
    }
}

/// `getGroupedTaskOverGroupFooterPreviewView`（真源 :263-286）。
///
/// 拖到组尾：向上拖 → 放进组**末尾**；向下拖 → 放到组**后面**（出组）。
/// 与组头分支严格镜像。
pub fn preview_task_over_group_footer(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Task { task_key }, Some(DropTarget::GroupFooter { group_id })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    match spec.direction {
        DragDirection::Before => move_task_to_group_end(view, task_key, group_id),
        DragDirection::After => move_task_to_root_around_group(
            view,
            task_key,
            group_id,
            InsertPosition::After,
        ),
    }
}

/// `getGroupedTaskOverEmptyDropZonePreviewView`（真源 :288-300）。
///
/// 拖到空组的内容区 → 放进首位。与组头「向下」分支同结果，但走的是不同投放区，
/// 故仍单列一个函数以对齐真源结构。
pub fn preview_task_over_empty_drop_zone(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Task { task_key }, Some(DropTarget::EmptyGroupBody { group_id })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    move_task_to_group_start(view, task_key, group_id)
}

/// `getGroupedGroupOverGroupPreviewView`（真源 :302-318）：group 拖到另一个 group。
///
/// 真源 :307的 `activeGroupId === overGroupId` 短路是必需的——view.rs 的
/// `moveGroupAroundTopLevelNode` 也做了同样判断，但真源在分发层就挡住，
/// 避免无谓进入重排。
pub fn preview_group_over_group(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Group { group_id: active_group_id }, Some(DropTarget::Group { group_id: over_group_id })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    if active_group_id == over_group_id {
        return view.clone();
    }
    move_group_around_top_level_node(
        view,
        active_group_id,
        &DragOver::Group {
            group_id: over_group_id.clone(),
        },
        spec.direction.to_position(),
    )
}

/// `getGroupedGroupOverTaskPreviewView`（真源 :320-334）：group 拖到 task 上。
///
/// 落位由 view.rs 内部决定：over 是组内 task 时归一化为所属 group，
/// 避免 content 区域不触发 group over group（view.rs :441-443 注释）。
pub fn preview_group_over_task(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Group { group_id: active_group_id }, Some(DropTarget::Task { task_key })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    move_group_around_top_level_node(
        view,
        active_group_id,
        &DragOver::Task {
            task_key: task_key.clone(),
        },
        spec.direction.to_position(),
    )
}

/// `getGroupedTaskOverTaskPreviewView`（真源 :227-238，task over task）。
///
/// 复用 view.rs 的 `moveTaskOverTask`——这也是 view.rs :335 那条
/// 「先移除再重新定位」注释真正生效的地方。
pub fn preview_task_over_task(
    view: &GroupedTaskView,
    spec: &DragOverSpec,
) -> GroupedTaskView {
    let (DragSource::Task { task_key: active }, Some(DropTarget::Task { task_key: over })) =
        (&spec.active, &spec.over)
    else {
        return view.clone();
    };
    move_task_over_task(view, active, over, spec.direction.to_position())
}

/// `applyGroupedTaskDragOverPreview`（真源 :1315-1360）：完整分发链。
///
/// **链式顺序照抄真源**（见模块头说明）：task 分支是
/// 折叠组 → 组头 → 组尾 → 空投放区 → task 五层依次套用，
/// group 分支只有两层。任一层命中就返回，未命中则原样穿透。
///
/// 真源 :1319 的守卫也照抄：`active` 既无 task 也无 group、或 `over` 为空
/// （拖出所有投放区）时直接返回。
pub fn apply_drag_over_preview(view: &GroupedTaskView, spec: &DragOverSpec) -> GroupedTaskView {
    if spec.over.is_none() {
        return view.clone();
    }

    let next_view = match &spec.active {
        DragSource::Group { .. } => preview_group_over_task(
            &preview_group_over_group(view, spec),
            spec,
        ),
        DragSource::Task { .. } => preview_task_over_task(
            &preview_task_over_empty_drop_zone(
                &preview_task_over_group_footer(
                    &preview_task_over_group_header(
                        &preview_task_over_collapsed_group(view, spec),
                        spec,
                    ),
                    spec,
                ),
                spec,
            ),
            spec,
        ),
    };

    // 真源 :1350-1354：签名未变则不产生新视图（避免无谓重渲染）。
    // Rust 侧无引用可比较，用签名判等后返回入参语义上等价于真源返回 current。
    if is_same_view(&next_view, view) {
        return view.clone();
    }
    next_view
}

/// dragOver 预览是否产生了实质变化（真源 :1350 的判断，提取为可测函数）。
pub fn preview_changes_view(view: &GroupedTaskView, spec: &DragOverSpec) -> bool {
    !is_same_view(view, &apply_drag_over_preview(view, spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groupedTasks::view::{TaskGroup, TaskListItem};

    fn task(workspace_path: &str, task_id: &str) -> TaskListItem {
        TaskListItem {
            task_id: task_id.into(),
            title: task_id.into(),
            workspace_path: workspace_path.into(),
            workspace_identity: None,
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

    /// `[t1, t2, g1[t3,t4], t5]`
    fn sample() -> GroupedTaskView {
        GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", "t1")),
                task_node(task("/ws", "t2")),
                group_node("g1", vec![task("/ws", "t3"), task("/ws", "t4")]),
                task_node(task("/ws", "t5")),
            ],
        }
    }

    fn shape(view: &GroupedTaskView) -> String {
        view.nodes
            .iter()
            .map(|node| match node {
                GroupedTaskViewNode::Task { task, .. } => task.task_id.clone(),
                GroupedTaskViewNode::Group { group, tasks, .. } => format!(
                    "{}[{}]",
                    group.group_id,
                    tasks.iter().map(|t| t.task_id.clone()).collect::<Vec<_>>().join(",")
                ),
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn key(id: &str) -> String {
        super::super::view::task_key("/ws", None, id)
    }

    fn task_source(id: &str) -> DragSource {
        DragSource::Task {
            task_key: key(id),
        }
    }

    fn group_source(id: &str) -> DragSource {
        DragSource::Group {
            group_id: id.into(),
        }
    }

    fn spec(active: DragSource, over: Option<DropTarget>, direction: DragDirection) -> DragOverSpec {
        DragOverSpec {
            active,
            over,
            direction,
        }
    }

    // ── viewSignature ──

    #[test]
    fn signature_uses_task_key_and_group_membership() {
        // 真源 :336-344 的格式：t:<key> 与 g:<id>[<keys>]，| 连接。
        let sig = view_signature(&sample());
        assert!(sig.starts_with("t:/ws\u{0}t1|"));
        assert!(sig.contains("g:g1[/ws\u{0}t3,/ws\u{0}t4]"));
        assert!(sig.ends_with("|t:/ws\u{0}t5"));
    }

    #[test]
    fn signature_changes_on_reorder() {
        let view = sample();
        let moved = preview_task_over_task(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::Task {
                    task_key: key("t5"),
                }),
                DragDirection::Before,
            ),
        );
        assert_ne!(view_signature(&view), view_signature(&moved));
        assert!(!is_same_view(&view, &moved));
    }

    #[test]
    fn signature_ignores_task_payload_changes() {
        // 签名只含结构，不含 title/status —— 乐观更新不该触发重排落库。
        let view = sample();
        let GroupedTaskViewNode::Task { task, .. } = &view.nodes[0] else {
            panic!("期望 task 节点");
        };
        let mut renamed = task.clone();
        renamed.title = "新标题".into();
        let updated = super::super::view::replace_task_in_grouped_view(&view, renamed);
        assert!(is_same_view(&view, &updated));
    }

    #[test]
    fn signature_ignores_sort_order() {
        // 真源签名不含 sort_order：它只影响落库顺序，不影响展示结构。
        let view = sample();
        let mut with_sort = view.clone();
        if let GroupedTaskViewNode::Task { sort_order, .. } = &mut with_sort.nodes[0] {
            *sort_order = Some(42);
        }
        assert_eq!(view_signature(&view), view_signature(&with_sort));
    }

    #[test]
    fn different_workspaces_produce_different_signatures() {
        // taskKey 带 workspace 前缀，跨 workspace 同 taskId 不会误判为同一视图。
        let a = GroupedTaskView {
            nodes: vec![task_node(task("/ws-a", "t1"))],
        };
        let b = GroupedTaskView {
            nodes: vec![task_node(task("/ws-b", "t1"))],
        };
        assert!(!is_same_view(&a, &b));
    }

    // ── task over task ──

    #[test]
    fn task_over_task_before_and_after() {
        let view = sample();
        let before = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::Task {
                    task_key: key("t2"),
                }),
                DragDirection::Before,
            ),
        );
        assert_eq!(shape(&before), "t1,t2,g1[t3,t4],t5", "t1 已在 t2 前，no-op");

        let after = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::Task {
                    task_key: key("t2"),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&after), "t2,t1,g1[t3,t4],t5");
    }

    #[test]
    fn task_over_task_moves_group_member_to_top_level() {
        // 组内 task 拖到顶层 task 前 → 升为顶层游离节点。
        let view = sample();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t3"),
                Some(DropTarget::Task {
                    task_key: key("t1"),
                }),
                DragDirection::Before,
            ),
        );
        assert_eq!(shape(&next), "t3,t1,t2,g1[t4],t5");
    }

    // ── 组头/ 组尾/ 空投放区 ──

    #[test]
    fn task_over_group_header_before_goes_above_group() {
        let view = sample();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::GroupHeader {
                    group_id: "g1".into(),
                }),
                DragDirection::Before,
            ),
        );
        // 向上拖 → 放在组前（顶层），不是进组。
        assert_eq!(shape(&next), "t2,t1,g1[t3,t4],t5");
    }

    #[test]
    fn task_over_group_header_after_enters_group_start() {
        let view = sample();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::GroupHeader {
                    group_id: "g1".into(),
                }),
                DragDirection::After,
            ),
        );
        // 向下拖 → 进组首位。这是组头分支的非对称设计。
        assert_eq!(shape(&next), "t2,g1[t1,t3,t4],t5");
    }

    #[test]
    fn task_over_group_footer_before_enters_group_end() {
        let view = sample();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::GroupFooter {
                    group_id: "g1".into(),
                }),
                DragDirection::Before,
            ),
        );
        assert_eq!(shape(&next), "t2,g1[t3,t4,t1],t5");
    }

    #[test]
    fn task_over_group_footer_after_leaves_group() {
        let view = sample();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t3"),
                Some(DropTarget::GroupFooter {
                    group_id: "g1".into(),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), "t1,t2,g1[t4],t3,t5");
    }

    #[test]
    fn task_over_empty_group_body_enters_group() {
        let view = GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", "t1")),
                group_node("g1", vec![]),
            ],
        };
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::EmptyGroupBody {
                    group_id: "g1".into(),
                }),
                DragDirection::Before,
            ),
        );
        assert_eq!(shape(&next), "g1[t1]");
    }

    #[test]
    fn task_over_collapsed_group_enters_group_start() {
        // 折叠组分支先于组头分支命中（链式顺序），效果同样是进组首位。
        let view = GroupedTaskView {
            nodes: vec![
                task_node(task("/ws", "t1")),
                group_node("g1", vec![task("/ws", "t3")]),
            ],
        };
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::GroupHeader {
                    group_id: "g1".into(),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), "g1[t1,t3]");
    }

    // ── group over group / group over task ──

    #[test]
    fn group_over_group_reorders() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", "t3")]),
                group_node("g2", vec![task("/ws", "t4")]),
            ],
        };
        let next = apply_drag_over_preview(
            &view,
            &spec(
                group_source("g1"),
                Some(DropTarget::Group {
                    group_id: "g2".into(),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), "g2[t4],g1[t3]");
    }

    #[test]
    fn group_over_itself_is_noop() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![]),
                group_node("g2", vec![]),
            ],
        };
        let next = apply_drag_over_preview(
            &view,
            &spec(
                group_source("g1"),
                Some(DropTarget::Group {
                    group_id: "g1".into(),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn group_over_member_task_targets_owning_group() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", "t3")]),
                group_node("g2", vec![task("/ws", "t4")]),
            ],
        };
        // over 是 g2 的成员 → 归一化为 g2，g1 排到其后。
        let next = apply_drag_over_preview(
            &view,
            &spec(
                group_source("g1"),
                Some(DropTarget::Task {
                    task_key: key("t4"),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), "g2[t4],g1[t3]");
    }

    #[test]
    fn group_over_own_member_is_noop() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", "t3")]),
                group_node("g2", vec![task("/ws", "t4")]),
            ],
        };
        let next = apply_drag_over_preview(
            &view,
            &spec(
                group_source("g1"),
                Some(DropTarget::Task {
                    task_key: key("t3"),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn group_over_top_level_task_reorders_group() {
        let view = GroupedTaskView {
            nodes: vec![
                group_node("g1", vec![task("/ws", "t3")]),
                task_node(task("/ws", "t9")),
            ],
        };
        let next = apply_drag_over_preview(
            &view,
            &spec(
                group_source("g1"),
                Some(DropTarget::Task {
                    task_key: key("t9"),
                }),
                DragDirection::Before,
            ),
        );
        assert_eq!(shape(&next), "g1[t3],t9");
    }

    // ── 分发链守卫 ──

    #[test]
    fn no_over_target_is_noop() {
        // 真源 :1319：拖出所有投放区 → 直接返回。
        let view = sample();
        let next = apply_drag_over_preview(&view, &spec(task_source("t1"), None, DragDirection::After));
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn mismatched_source_and_target_is_noop() {
        // 拖 group 时悬停在组头投放区 → group 分支只认 Group/Task，no-op。
        let view = sample();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                group_source("g1"),
                Some(DropTarget::GroupFooter {
                    group_id: "g1".into(),
                }),
                DragDirection::After,
            ),
        );
        assert_eq!(shape(&next), shape(&view));
    }

    #[test]
    fn preview_changes_view_detects_real_moves_only() {
        let view = sample();
        // 真实移动 → true
        assert!(preview_changes_view(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::Task {
                    task_key: key("t2"),
                }),
                DragDirection::After,
            )
        ));
        // no-op → false（真源据此跳过重渲染）
        assert!(!preview_changes_view(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::Task {
                    task_key: key("t1"),
                }),
                DragDirection::After,
            )
        ));
    }

    #[test]
    fn dragging_into_group_is_idempotent_across_repeated_events() {
        // dragOver 会高频重复触发；同一落点重复预览必须稳定收敛，
        // 否则会累积出「越拖越偏」的状态。
        let view = sample();
        let s = spec(
            task_source("t1"),
            Some(DropTarget::GroupHeader {
                group_id: "g1".into(),
            }),
            DragDirection::After,
        );
        let once = apply_drag_over_preview(&view, &s);
        let twice = apply_drag_over_preview(&once, &s);
        let thrice = apply_drag_over_preview(&twice, &s);
        assert_eq!(shape(&once), "t2,g1[t1,t3,t4],t5");
        assert_eq!(shape(&twice), shape(&once));
        assert_eq!(shape(&thrice), shape(&once));
    }

    #[test]
    fn empty_view_preview_is_safe() {
        let view = GroupedTaskView::default();
        let next = apply_drag_over_preview(
            &view,
            &spec(
                task_source("t1"),
                Some(DropTarget::Task {
                    task_key: key("t1"),
                }),
                DragDirection::After,
            ),
        );
        assert!(next.nodes.is_empty());
    }
}