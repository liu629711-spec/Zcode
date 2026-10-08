//! HTML5 原生拖拽事件的接线层：DOM事件 ↔ [`DragRuntime`] / [`dnd::DropTarget`]。
//!
//! ## 事件契约（与真源 dnd-kit 的对应关系）
//!
//! | 本实现 | 真源 dnd-kit | 说明 |
//! |---|---|---|
//! | `draggable="true"` + `on:dragstart` | `useDraggable` | 拖拽源 |
//! | `on:dragover`(preventDefault) | `useDroppable` + collision | 投放区命中 |
//! | `data-drop-type` 属性 | `data.current.type` | 投放区类型标识 |
//! | `on:drop` / `on:dragend` | `onDragEnd` | 落位提交 / 回滚 |
//!
//! ## 为什么用 dataTransfer 传类型
//!
//! dnd-kit 靠碰撞检测算出 `event.over.data.current.type`（投放区类型）。
//! 原生 DnD 没有碰撞检测，但我们可以**在每个投放区元素上放一个
//! `data-drop-type` 属性**，`dragover` 时直接从 `event.target` 往上找。
//! 这等价于「鼠标在哪个元素上」——正是 collision 检测想回答的问题。
//!
//! `dragover` 必须 `preventDefault()`，否则浏览器不会允许 drop——这是
//! HTML5 DnD 的硬性要求（不preventDefault 就不会触发 drop 事件）。
//!
//! 行号注释均指 `packages/ui/src/workspace-grouped-tasks/group-item.tsx`
//! 与 `task-row.tsx`。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::dnd::{DragSource, DropTarget};
use super::dragRuntime::DragRuntime;
use super::view::{task_key_of, GroupedTaskView};

/// `data-drop-type` 的属性名。投放区元素用它声明自己是哪种落点。
pub const DROP_TYPE_ATTR: &str = "data-drop-type";

/// `data-task-key` 的属性名：task 行用它声明自己的 taskKey。
pub const TASK_KEY_ATTR: &str = "data-task-key";

/// `data-group-id` 的属性名：组元素用它声明自己的 groupId。
pub const GROUP_ID_ATTR: &str = "data-group-id";

/// 拖拽源在 `dataTransfer` 里的类型标记（真源 dnd-kit 用 data.current.type，
/// 这里用自定义 MIME 串）。用 `application/x-zcode` 前缀避免与浏览器
/// 内部类型冲突，也让 `dragover` 能识别「这是我们的拖拽」。
pub const DRAG_MIME: &str = "application/x-zcode-grouped-drag";

/// 从元素读取 `data-drop-type`（投放区类型）。
///
/// 真源有 5 种（group-item.tsx:156-186+ task-row.tsx:380）：
/// `grouped-group-over` / `grouped-collapsed-group` / `grouped-expanded-group-header` /
/// `grouped-expanded-group-footer` / `grouped-task` / `grouped-empty-drop`。
/// 这里按同样语义映射到 `DropTarget`。
pub fn read_drop_target(node: &web_sys::Element) -> Option<DropTarget> {
    let drop_type = node.get_attribute(DROP_TYPE_ATTR)?;
    let group_id = || node.get_attribute(GROUP_ID_ATTR);
    let task_key = || node.get_attribute(TASK_KEY_ATTR);

    match drop_type.as_str() {
        // task 行（task-row.tsx:380 useDroppable，data.type = "grouped-task"）。
        "grouped-task" => task_key().map(|task_key| DropTarget::Task { task_key }),
        // 组头 / 折叠组头（group-item.tsx:163 collapsedGroup / :171 header）。
        // 真源用「是否折叠」区分两个 droppable，但落位效果都是进组首位，
        // 故 Rust 侧统一映射为 GroupHeader，由 dnd.rs 决定分支。
        "grouped-collapsed-group" | "grouped-expanded-group-header" => {
            group_id().map(|group_id| DropTarget::GroupHeader { group_id })
        }
        // 组尾（group-item.tsx:179 expandedGroupFooter）。
        "grouped-expanded-group-footer" => group_id().map(|group_id| DropTarget::GroupFooter { group_id }),
        // 空组内容区（真源 emptyDropZone）。
        "grouped-empty-drop" => group_id().map(|group_id| DropTarget::EmptyGroupBody { group_id }),
        // group 本体（group-item.tsx:156 groupOver），拖 group 时用。
        "grouped-group-over" => group_id().map(|group_id| DropTarget::Group { group_id }),
        _ => None,
    }
}

/// 从 `dragstart` 的 `dataTransfer` 读取被拖对象（真源 event.active.data.current）。
///
/// 真源把 `{ type, taskKey|groupId }` 挂在 draggable 上；原生 DnD 我们用
/// dataTransfer 的 `setData` 传，drop 时读回。读不到就返回 None（外部拖入的
/// 文件等，不是我们的拖拽源）。
pub fn read_drag_source(data_transfer: &web_sys::DataTransfer) -> Option<DragSource> {
    let payload = data_transfer.get_data(DRAG_MIME).ok()?;
    if payload.is_empty() {
        return None;
    }
    // 格式："task:<taskKey>" 或 "group:<groupId>"。
    let (kind, value) = payload.split_once(':')?;
    match kind {
        "task" => Some(DragSource::Task {
            task_key: value.to_string(),
        }),
        "group" => Some(DragSource::Group {
            group_id: value.to_string(),
        }),
        _ => None,
    }
}

/// 构造 dataTransfer 的载荷串（`read_drag_source` 的逆操作）。
///
/// `taskKey` 里含 `\u0000` 分隔符（NUL），会破坏 `split_once(':')` 吗？
/// 不会——taskKey 是 `${workspaceKey}\0${taskId}`，NUL 不是冒号，
/// 且我们只 split **第一个**冒号（`split_once`），后面的原样保留。
pub fn drag_source_payload(source: &DragSource) -> String {
    match source {
        DragSource::Task { task_key } => format!("task:{task_key}"),
        DragSource::Group { group_id } => format!("group:{group_id}"),
    }
}

/// 在 `dragstart` 时从 DOM 元素推导拖拽源（真源 draggable 的 `data`）。
///
/// 优先读元素的 `data-task-key`（task 行）→ 拖 task；
/// 否则读 `data-group-id`（组头）→ 拖 group。
pub fn source_of_node(node: &web_sys::Element) -> Option<DragSource> {
    if let Some(task_key) = node.get_attribute(TASK_KEY_ATTR) {
        return Some(DragSource::Task { task_key });
    }
    node.get_attribute(GROUP_ID_ATTR)
        .map(|group_id| DragSource::Group { group_id })
}

/// 从 `dragover`/`drop` 的 event.target 链上找最近的投放区。
///
/// 为什么要往上找：拖拽时指针常落在子元素（如行内的文字 span）上，
/// 而 droppable 挂在容器上。真源靠 collision 检测处理这个层级关系。
fn closest_drop_target(event: &web_sys::Event) -> Option<DropTarget> {
    let target = event.target()?;
    let mut node = target.dyn_into::<web_sys::Element>().ok();
    while let Some(el) = node {
        if let Some(hit) = read_drop_target(&el) {
            return Some(hit);
        }
        node = el.parent_element();
    }
    None
}

/// 组装 `on:dragstart` 的处理闭包。
///
/// 真源 `useDraggable` 的 data 是 `{ type, taskKey|groupId }`（task-row.tsx:379）；
/// 原生DnD 对应「读元素属性 → 写 dataTransfer」。
pub fn on_drag_start(
    event: web_sys::DragEvent,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<GroupedTaskView>,
) {
    let Some(node) = event
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Some(source) = source_of_node(&node) else {
        return;
    };
    let Some(transfer) = event.data_transfer() else {
        return;
    };
    transfer.set_data(DRAG_MIME, &drag_source_payload(&source)).ok();
    // dnd-kit 默认行为也设effectAllowed，这里设 move 表示移动语义。
    transfer.set_effect_allowed("move");

    let current = view.get_untracked();
    runtime.update(|rt| rt.begin(source, &current));
}

/// 组装 `on:dragover` 的处理闭包。
///
/// 对应真源 `onDragOver` → `applyGroupedTaskDragOverPreview`（:1384-1390）：
/// 每次 over 到新投放区就算一次预览。
pub fn on_drag_over(
    event: web_sys::DragEvent,
    runtime: ReadSignal<DragRuntime>,
    view: RwSignal<GroupedTaskView>,
) {
    // 必须 preventDefault，否则 drop 不会触发。这是 HTML5 DnD 的硬性要求。
    event.prevent_default();
    let Some(transfer) = event.data_transfer() else {
        return;
    };
    transfer.set_drop_effect("move");

    // 不是我们的拖拽源（如外部文件拖入）→ 不处理。
    if read_drag_source(&transfer).is_none() {
        return;
    }

    let over = closest_drop_target(&event);
    let current = view.get_untracked();
    let rt = runtime.get_untracked();
    let next = rt.preview(&current, over);
    // 只在视图签名真的变了才 set，���免高频 dragover 引发无谓重渲染。
    // 判据用签名（真源 :1350-1354 同思路），而不是结构相等 —— 后者会把
    // 「仅title/status 变化」也当成结构变化。
    if !super::dnd::is_same_view(&next, &current) {
        view.set(next);
    }
}

/// 组装 `on:drop` 的处理闭包：落位提交。
///
/// 对应真源 `onDragEnd`（:1429-1500）：next 与 origin 签名不同才 `applyOrder` 落库。
pub fn on_drop(
    event: web_sys::DragEvent,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<GroupedTaskView>,
    on_commit: impl Fn(GroupedTaskView),
) {
    event.prevent_default();
    let next = view.get_untracked();
    // Signal::update 返回 ()，不能拿闭包的返回值。先在本地副本上算出
    // finish 的结果，再把「已结束」的状态写回 signal。
    let mut rt = runtime.get_untracked();
    let committed = rt.finish(&next);
    runtime.set(rt);
    if let Some(committed) = committed {
        on_commit(committed.clone());
        view.set(committed);
    }
}

/// 组装 `on:dragend` 的处理闭包：拖拽结束（无论是否 drop）。
///
/// - 正常 drop 后触发：此时 `finish` 已被 `on_drop` 调过，此处无操作（幂等）。
/// - 未 drop（取消/Esc）触发：`cancel` 交还原点视图供回滚。
///
/// 对应真源 `handleGroupedTaskDragCancel`（:1403-1424）。
pub fn on_drag_end(
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<GroupedTaskView>,
) {
    // 若 on_drop 已处理（active 为空），这里直接返回。
    let mut rt = runtime.get_untracked();
    let restored = rt.cancel();
    runtime.set(rt);
    if let Some(restored) = restored {
        //没 drop 成功才需要回滚到原点。
        if !runtime.get_untracked().is_dragging() {
            view.set(restored);
        }
    }
}

/// 组装 `on:dragover` 的方向追踪：喂 dy 增量给 runtime。
///
/// 真源 `handleGroupedTaskDragMove`（:1362-1380）单独监听 dragMove 维护方向。
/// 原生 DnD 没有 dragMove 事件，但 `dragover` 是持续触发的，可用其
/// `movement_y`（本次事件的位移量）近似喂给方向判定。
pub fn track_direction(event: &web_sys::DragEvent, runtime: RwSignal<DragRuntime>) {
    let movement_y = event.movement_y() as f64;
    if movement_y != 0.0 {
        runtime.update(|rt| rt.feed_delta_y(movement_y));
    }
}

/// 给 task 行元素生成拖拽所需的属性（对齐真源 task-row.tsx:373-382）。
///
/// 真源 `dragDisabled = workspaceActionsDisabled || !dragId || contextMenuOpen || dragOverlay`，
/// Rust 侧当前只保留 `dragId` 缺失这一条（没有 contextMenu/dragOverlay 状态）。
pub fn task_drag_attrs(task: &super::view::TaskListItem) -> Vec<(&'static str, String)> {
    let task_key = task_key_of(task);
    vec![
        (TASK_KEY_ATTR, task_key.clone()),
        (DROP_TYPE_ATTR, "grouped-task".to_string()),
        ("draggable", "true".to_string()),
    ]
}

/// 把视图转成后端 `task_group_apply_order` 的入参（真源 `viewToOrderInput`）。
///
/// 真源（useGroupedTaskView.ts:539-554）提交两份数据：
/// - `topLevelNodes`：顶层节点顺序，组只传 id、任务只传 taskId；
/// - `groups`：各组**内部**成员顺序。
///
/// Rust 侧字段名与后端 `TopLevelNodeInput` / `GroupOrderInput` 对齐
/// （serde tag = "type"，值小写 group/task）。
///
/// 注意：顶层节点里组的 `taskIds` 为空——成员顺序由 groups 单独提交，
/// 与真源 `nodeToTopLevelRef` 只取节点身份一致。
pub fn view_to_order_input(view: &GroupedTaskView) -> serde_json::Value {
    let mut top_level: Vec<serde_json::Value> = Vec::with_capacity(view.nodes.len());
    let mut groups: Vec<serde_json::Value> = Vec::new();

    for node in &view.nodes {
        match node {
            super::view::GroupedTaskViewNode::Group { group, tasks, .. } => {
                top_level.push(serde_json::json!({
                    "type": "group",
                    "groupId": group.group_id,
                }));
                groups.push(serde_json::json!({
                    "groupId": group.group_id,
                    "taskIds": tasks.iter().map(|t| t.task_id.clone()).collect::<Vec<_>>(),
                }));
            }
            super::view::GroupedTaskViewNode::Task { task, .. } => {
                top_level.push(serde_json::json!({
                    "type": "task",
                    "task": task.task_id,
                }));
            }
        }
    }

    serde_json::json!({ "topLevel": top_level, "groups": groups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groupedTasks::view::TaskListItem;

    #[test]
    fn payload_roundtrip_for_task() {
        let src = DragSource::Task {
            task_key: "/ws\u{0}t1".into(),
        };
        let payload = drag_source_payload(&src);
        // NUL 分隔符不能破坏 split_once(':')。
        assert_eq!(
            payload.split_once(':').map(|(_, v)| v),
            Some("/ws\u{0}t1")
        );
    }

    #[test]
    fn payload_roundtrip_for_group() {
        let src = DragSource::Group {
            group_id: "g1".into(),
        };
        assert_eq!(drag_source_payload(&src), "group:g1");
    }

    #[test]
    fn payload_with_colon_in_group_id_is_preserved() {
        // groupId 若含冒号，split_once 只切第一个，剩余部分完整保留。
        // 这正是用 split_once 而非 split 的原因。
        let src = DragSource::Group {
            group_id: "a:b:c".into(),
        };
        let payload = drag_source_payload(&src);
        assert_eq!(payload.split_once(':').map(|(_, v)| v), Some("a:b:c"));
    }

    #[test]
    fn task_drag_attrs_contains_key_and_drop_type() {
        let task = TaskListItem {
            task_id: "t1".into(),
            title: "t1".into(),
            workspace_path: "/ws".into(),
            workspace_identity: None,
            created_at: 0,
            updated_at: 0,
            status: String::new(),
        };
        let attrs = task_drag_attrs(&task);
        let keys: Vec<&str> = attrs.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&TASK_KEY_ATTR));
        assert!(keys.contains(&DROP_TYPE_ATTR));
        assert!(keys.contains(&"draggable"));
        // drop-type 必须是 grouped-task（task-row.tsx:380）。
        let drop_type = attrs
            .iter()
            .find(|(k, _)| *k == DROP_TYPE_ATTR)
            .map(|(_, v)| v.as_str());
        assert_eq!(drop_type, Some("grouped-task"));
    }

    // ── viewToOrderInput ──

    fn t(id: &str) -> super::super::view::TaskListItem {
        super::super::view::TaskListItem {
            task_id: id.into(),
            title: id.into(),
            workspace_path: "/ws".into(),
            workspace_identity: None,
            created_at: 0,
            updated_at: 0,
            status: String::new(),
        }
    }

    #[test]
    fn order_input_splits_top_level_and_groups() {
        use super::super::view::{GroupedTaskViewNode, TaskGroup};
        let view = GroupedTaskView {
            nodes: vec![
                GroupedTaskViewNode::Task {
                    task: t("t1"),
                    sort_order: None,
                },
                GroupedTaskViewNode::Group {
                    group: TaskGroup {
                        group_id: "g1".into(),
                        title: "g1".into(),
                        color: "blue".into(),
                    },
                    tasks: vec![t("t2"), t("t3")],
                    sort_order: None,
                },
            ],
        };
        let payload = view_to_order_input(&view);
        let top = payload["topLevel"].as_array().unwrap();
        assert_eq!(top.len(), 2);
        // 顶层 task 节点只传 taskId（真源 nodeToTopLevelRef）。
        assert_eq!(top[0]["type"], "task");
        assert_eq!(top[0]["task"], "t1");
        // 顶层 group 节点只传 groupId，不带成员。
        assert_eq!(top[1]["type"], "group");
        assert_eq!(top[1]["groupId"], "g1");
        assert!(top[1].get("taskIds").is_none());

        // 组成员顺序单独提交。
        let groups = payload["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["groupId"], "g1");
        assert_eq!(groups[0]["taskIds"], serde_json::json!(["t2", "t3"]));
    }

    #[test]
    fn order_input_preserves_top_level_order() {
        use super::super::view::GroupedTaskViewNode;
        let view = GroupedTaskView {
            nodes: vec![
                GroupedTaskViewNode::Task {
                    task: t("t2"),
                    sort_order: None,
                },
                GroupedTaskViewNode::Task {
                    task: t("t1"),
                    sort_order: None,
                },
            ],
        };
        let payload = view_to_order_input(&view);
        let top = payload["topLevel"].as_array().unwrap();
        // 顺序即数组下标——后端按 index 写 sort_order，顺序不能乱。
        assert_eq!(top[0]["task"], "t2");
        assert_eq!(top[1]["task"], "t1");
    }

    #[test]
    fn order_input_with_no_groups_is_empty_array_not_null() {
        let view = GroupedTaskView::default();
        let payload = view_to_order_input(&view);
        assert_eq!(payload["topLevel"].as_array().unwrap().len(), 0);
        assert_eq!(payload["groups"].as_array().unwrap().len(), 0);
    }
}