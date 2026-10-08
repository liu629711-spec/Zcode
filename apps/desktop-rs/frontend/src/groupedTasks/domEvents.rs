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
use web_sys::DataTransfer;

use super::dnd::{DragSource, DropTarget};
use super::dragRuntime::DragRuntime;
use super::view::{GroupedTaskView, task_key_of};

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
        "grouped-expanded-group-footer" => {
            group_id().map(|group_id| DropTarget::GroupFooter { group_id })
        }
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

/// 自定义拖拽预览的标记属性（dragend 时按它找到并移除）。
pub const PREVIEW_MARK_ATTR: &str = "data-drag-preview";

/// 构建自定义拖拽预览元素并 `setDragImage`。
///
/// HTML5 DnD 的默认 ghost image 是**整行元素的截图**，含选中态背景与
/// hover 按钮，观感差且不可控。真源用 dnd-kit `DragOverlay` 自绘卡片
/// （group-drag-overlay.tsx / task-row.tsx 的 dragOverlay 分支）。
///
/// 原生等价物：`dataTransfer.setDragImage(el, x, y)`——el 必须已插入文档
/// 且可见（不能 display:none），故挂 body 后定位到屏幕外（top:-9999px）。
/// `dragend` 时按 [`PREVIEW_MARK_ATTR`] 清理（`remove_drag_preview`）。
///
/// 预览样式对齐真源 overlay：
/// - task：行纯展示版（状态点 + 标题 + 时间），task-row.tsx:287 注释
///   「DragOverlay 高频渲染时只返回纯展示节点」；
/// - group：组头复刻（颜色标 + 标题 + 数量徽章），group-drag-overlay.tsx:25-34，
///   `cursor-grabbing` + `shadow-lg`。
fn attach_drag_preview(
    transfer: &web_sys::DataTransfer,
    source: &DragSource,
    view: &GroupedTaskView,
) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let preview = match source {
        DragSource::Task { task_key } => {
            // 从视图里找任务数据（标题 / 状态 / 时间）。
            let task = find_task_in_view(view, task_key);
            build_task_preview(&document, task)
        }
        DragSource::Group { group_id } => {
            let group = find_group_in_view(view, group_id);
            build_group_preview(&document, group)
        }
    };
    preview.set_attribute(PREVIEW_MARK_ATTR, "true").ok();
    // 屏幕外但保持渲染（display:none 的元素 setDragImage 无效）。
    // 用 set_attribute 一次写入而非 style() 对象——leptos prelude 的
    // trait 与 web_sys::Element::style 同名，会产生方法解析歧义。
    preview
        .set_attribute(
            "style",
            "position:fixed;top:-9999px;left:-9999px;pointer-events:none",
        )
        .ok();
    document.body().unwrap().append_child(&preview).ok();
    // 指针居中（偏移取近似半宽半高；精确值需量元素尺寸，不值得）。
    DataTransfer::set_drag_image(&transfer, &preview, 60, 14);
}

/// dragend 时移除预览元素（drop 成功与取消都要清理）。
pub fn remove_drag_preview() {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    // 完整路径调用：leptos prelude 的 trait 会遮蔽 Document 的同名方法。
    if let Ok(nodes) =
        web_sys::Document::query_selector_all(&document, &format!("[{PREVIEW_MARK_ATTR}]"))
    {
        for i in 0..nodes.length() {
            if let Some(node) = nodes.get(i) {
                let _ = node.parent_element().map(|p| p.remove_child(&node));
            }
        }
    }
}

/// 在视图中查找任务（预览渲染用）。
fn find_task_in_view<'a>(
    view: &'a GroupedTaskView,
    task_key: &str,
) -> Option<&'a super::view::TaskListItem> {
    for node in &view.nodes {
        match node {
            super::view::GroupedTaskViewNode::Task { task, .. } => {
                if super::view::task_key_of(task) == task_key {
                    return Some(task);
                }
            }
            super::view::GroupedTaskViewNode::Group { tasks, .. } => {
                for task in tasks {
                    if super::view::task_key_of(task) == task_key {
                        return Some(task);
                    }
                }
            }
        }
    }
    None
}

/// 在视图中查找组（预览渲染用）。
fn find_group_in_view<'a>(
    view: &'a GroupedTaskView,
    group_id: &str,
) -> Option<(&'a super::view::TaskGroup, usize)> {
    for node in &view.nodes {
        if let super::view::GroupedTaskViewNode::Group { group, tasks, .. } = node {
            if group.group_id == group_id {
                return Some((group, tasks.len()));
            }
        }
    }
    None
}

/// 构建 task 行预览（纯展示版：状态点 + 标题 + 时间）。
fn build_task_preview(
    document: &web_sys::Document,
    task: Option<&super::view::TaskListItem>,
) -> web_sys::Element {
    let el = document.create_element("div").expect("create preview div");
    let (dot_class, title, time) = match task {
        Some(t) => (
            crate::app::status_dot_class(&t.status).to_string(),
            t.title.clone(),
            crate::app::relative_time(t.updated_at),
        ),
        // 视图未命中（理论上不该发生）给占位，不让预览凭空消失。
        None => (
            "bg-foreground-subtle".to_string(),
            "…".into(),
            String::new(),
        ),
    };
    el.set_class_name(
        "flex h-8 w-64 items-center gap-2 rounded-lg border border-border bg-background pl-2.5 pr-1 text-sm text-foreground shadow-lg",
    );
    el.set_inner_html(&format!(
        "<span class=\"size-1.5 flex-none rounded-full {dot_class}\"></span>\
         <span class=\"min-w-0 flex-1 truncate\">{}</span>\
         <span class=\"flex-none text-xs text-foreground-subtlest\">{}</span>",
        escape_html(&title),
        escape_html(&time),
    ));
    el
}

/// 构建组头预览（真源 GroupDragOverlay：颜色标 + 标题 + 数量徽章）。
fn build_group_preview(
    document: &web_sys::Document,
    group: Option<(&super::view::TaskGroup, usize)>,
) -> web_sys::Element {
    let el = document.create_element("div").expect("create preview div");
    let (color, title, count) = match group {
        Some((g, n)) => (g.color.clone(), g.title.clone(), n),
        None => ("gray".to_string(), "…".into(), 0),
    };
    el.set_class_name(
        "flex h-8 w-48 items-center gap-1 rounded-lg border border-border bg-background pl-1.5 pr-1 text-ui-base text-foreground shadow-lg",
    );
    el.set_inner_html(&format!(
        "<span class=\"size-2.5 flex-none rounded-full {}\"></span>\
         <span class=\"min-w-0 flex-1 truncate px-1\">{}</span>\
         <span class=\"inline-flex min-w-5 flex-none items-center justify-center rounded-full bg-tag/50 px-1.5 py-0.5 text-xs font-medium leading-none text-foreground-subtle\">{count}</span>",
        crate::groupedTasks::view::color_class(&color),
        escape_html(&title),
    ));
    el
}

/// 简单 HTML 转义（标题是用户可控文本，直接 innerHTML 会注入）。
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 组装 `on:dragstart` 的处理闭包。
///
/// 真源 `useDraggable` 的 data 是 `{ type, taskKey|groupId }`（task-row.tsx:379）；
/// 原生DnD 对应「读元素属性 → 写 dataTransfer」。
pub fn on_drag_start(
    event: web_sys::DragEvent,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<Option<GroupedTaskView>>,
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
    // `None` = 视图尚未 join 就绪，此时没有可回滚的起点，不启动拖拽
    // （真源此时同样没有 dragOriginView）。
    let Some(current) = view.get_untracked() else {
        return;
    };
    let Some(transfer) = event.data_transfer() else {
        return;
    };
    transfer
        .set_data(DRAG_MIME, &drag_source_payload(&source))
        .ok();
    // dnd-kit 默认行为也设effectAllowed，这里设 move 表示移动语义。
    transfer.set_effect_allowed("move");

    // 自定义拖拽预览（真源 DragOverlay 自绘卡片的原生等价物）。
    attach_drag_preview(&transfer, &source, &current);

    runtime.update(|rt| rt.begin(source, &current));
}

/// 组装 `on:dragover` 的处理闭包。
///
/// 对应真源 `onDragOver` → `applyGroupedTaskDragOverPreview`（:1384-1390）：
/// 每次 over 到新投放区就算一次预览。
///
/// `runtime` 用 `ReadSignal`：本函数只读拖拽状态（方向 + active），
/// 方向更新交给 `track_direction`（dragover 每次触发都会先调它）。
pub fn on_drag_over(
    event: web_sys::DragEvent,
    runtime: ReadSignal<DragRuntime>,
    view: RwSignal<Option<GroupedTaskView>>,
) {
    // 必须 prevent_default，否则 drop 不会触发。这是 HTML5 DnD 的硬性要求。
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
    let Some(current) = view.get_untracked() else {
        return;
    };
    let rt = runtime.get_untracked();
    let next = rt.preview(&current, over);
    // 只在视图签名真的变了才 set，���免高频 dragover 引发无谓重渲染。
    // 判据用签名（真源 :1350-1354 同思路），而不是结构相等 —— 后者会把
    // 「仅title/status 变化」也当成结构变化。
    if !super::dnd::is_same_view(&next, &current) {
        view.set(Some(next));
    }
}

/// 组装 `on:drop` 的处理闭包：落位提交。
///
/// 对应真源 `onDragEnd`（:1429-1500）：next 与 origin 签名不同才 `applyOrder` 落库。
pub fn on_drop(
    event: web_sys::DragEvent,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<Option<GroupedTaskView>>,
    on_commit: impl Fn(GroupedTaskView),
) {
    event.prevent_default();
    let Some(next) = view.get_untracked() else {
        return;
    };
    // Signal::update 返回 ()，不能拿闭包的返回值。先在本地副本上算出
    // finish 的结果，再把「已结束」的状态写回 signal。
    let mut rt = runtime.get_untracked();
    let committed = rt.finish(&next);
    runtime.set(rt);
    if let Some(committed) = committed {
        on_commit(committed.clone());
        view.set(Some(committed));
    }
}

/// 组装 `on:dragend` 的处理闭包：拖拽结束（无论是否 drop）。
///
/// - 正常 drop 后触发：此时 `finish` 已被 `on_drop` 调过，此处无操作（幂等）。
/// - 未 drop（取消/Esc）触发：`cancel` 交还原点视图供回滚。
///
/// 对应真源 `handleGroupedTaskDragCancel`（:1403-1424）。
pub fn on_drag_end(runtime: RwSignal<DragRuntime>, view: RwSignal<Option<GroupedTaskView>>) {
    // 预览元素无论 drop 成功与否都要清理（挂 body 屏幕外，泄漏会堆积）。
    remove_drag_preview();
    // 若 on_drop 已处理（active 为空），这里直接返回。
    let mut rt = runtime.get_untracked();
    let restored = rt.cancel();
    runtime.set(rt);
    // `cancel()` 只在「拖拽已开始但未被 drop 结束」时返回原点视图：
    // - 正常 drop：on_drop 已调 finish()，origin_view 被 take 走 → restored 为 None，
    //   这里什么都不做（幂等）；
    // - 取消 / Esc / 拖出窗口：origin_view 还在 → restored 为 Some，回滚预览。
    if let Some(restored) = restored {
        view.set(Some(restored));
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
        assert_eq!(payload.split_once(':').map(|(_, v)| v), Some("/ws\u{0}t1"));
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
