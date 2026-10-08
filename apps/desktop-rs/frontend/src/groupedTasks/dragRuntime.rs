//! 拖拽**事件适配层**：把浏览器 HTML5 拖拽事件翻译成 [`dnd.rs`] 的语义输入。
//!
//! ## 为什么用 HTML5 原生拖拽，而不是自己实现指针事件
//!
//! 真源用 dnd-kit 的 `PointerSensor` + `activationConstraint: { distance: 6 }`
//! （`WorkspaceGroupedTasksSection.tsx:667-673`）——拖动超过 6px 才激活，
//! 避免点击被误判成拖拽。
//!
//! HTML5 原生 `draggable` **天然满足这个约束**：浏览器只在按住并移动一段距离后
//! 才触发 `dragstart`，点击不会触发。所以不需要自己写阈值检测、指针捕获、
//! `touch-action` 协商这一整套。
//!
//! 代价是失去自定义拖拽预览（原生 ghost image 不可控），但真源的
//! DragOverlay 只是「跟着鼠标的半透明卡片」，可以先用原生预览 + CSS 近似。
//! 若后续要做真源级预览，再换指针事件实现，业务逻辑（本文件以下部分）不变。
//!
//! ## 分层
//!
//! - 本文件：状态机 + 事件翻译（依赖 DOM）
//! - `dnd.rs`：落位决策（纯函数，已 1:1 迁移真源）
//! - `view.rs`：视图重排算法（纯函数，已 1:1 迁移真源）
//!
//! 事件翻译只负责「把 DOM 事件里的 taskKey/groupId 抽出来填进 `DragOverSpec`」，
//! 任何业务判断都不在这里——那些都在 dnd.rs 里，已被 23 个测试覆盖。

use super::dnd::{apply_drag_over_preview, DragDirection, DragOverSpec, DragSource, DropTarget};
use super::view::GroupedTaskView;

/// 拖拽运行时状态（对应真源的 `activeDragTaskKey` / `activeDragGroupId` 等 ref）。
#[derive(Debug, Clone, Default)]
pub struct DragRuntime {
    /// 当前拖拽源。真源用两个 ref（taskKey / groupId）分别存，这里合并成枚举——
    /// 两者互斥，且合并后能避免 dnd.rs 分发时的「双源」歧义。
    pub active: Option<DragSource>,
    /// 拖拽起始时的视图快照。真源 `dragOriginViewRef`：dragEnd 时用它回滚，
    /// 以及判定「有没有真的移动过」。
    pub origin_view: Option<GroupedTaskView>,
    /// 拖拽方向。真源 `dragDirectionRef`：由 dragMove 的 delta.y 符号维护。
    pub direction: DragDirection,
    /// 上一次的 dy 增量。真源 `lastDragDeltaYRef`（初值 0）。
    prev_delta_y: f64,
}

impl DragRuntime {
    /// 新拖拽开始：记录起点视图。
    ///
    /// 真源在 `handleGroupedTaskDragStart` 里设`dragOriginViewRef.current = view`，
    /// 并重置方向为 `after`（:1399）。
    pub fn begin(&mut self, source: DragSource, view: &GroupedTaskView) {
        self.active = Some(source);
        self.origin_view = Some(view.clone());
        self.direction = DragDirection::After;
        // 真源 resetGroupedTaskDrag（:1399）会把 lastDragDeltaYRef 归零。
        self.prev_delta_y = 0.0;
    }

    /// 拖拽中：按落位算出预览视图。
    ///
    /// 对应真源 `applyGroupedTaskDragOverPreview`（:1315-1360）——
    /// 它做三件事，本方法照搬：
    /// 1. 无 over（拖出所有投放区）→ 原样返回；
    /// 2. 调 `apply_drag_over_preview` 算新视图；
    /// 3. 签名未变 → 返回原视图（不触发重渲染）。
    pub fn preview(&self, view: &GroupedTaskView, over: Option<DropTarget>) -> GroupedTaskView {
        let Some(active) = self.active.clone() else {
            return view.clone();
        };
        let spec = DragOverSpec {
            active,
            over,
            direction: self.direction,
        };
        apply_drag_over_preview(view, &spec)
    }

    /// 拖拽结束：产出应落库的视图，或 `None` 表示无变化、无需落库。
    ///
    /// 对应真源 `handleGroupedTaskDragEnd`（:1429-1500）的核心判据（:1476-1484）：
    /// 没有 originView、或 next 与 origin 的视图签名相同，就不落库。
    /// 注意真源还会在 group 拖拽分支（:1439-1447）做同样的签名比较，
    /// 只是路径不同、额外调了 `restoreGroupDragCollapsedState`。
    pub fn finish(&mut self, next_view: &GroupedTaskView) -> Option<GroupedTaskView> {
        let origin = self.origin_view.take()?;
        self.active = None;
        if super::dnd::is_same_view(&origin, next_view) {
            None
        } else {
            Some(next_view.clone())
        }
    }

    /// 拖拽取消：丢弃一切，回到原点。
    ///
    /// 对应真源 `handleGroupedTaskDragCancel`（:1403-1424）：恢复原点视图 + reset。
    pub fn cancel(&mut self) -> Option<GroupedTaskView> {
        self.active = None;
        self.origin_view.take()
    }

    /// 是否正在拖拽（真源多处用它控制 body cursor 与禁用 hover action）。
    pub fn is_dragging(&self) -> bool {
        self.active.is_some()
    }

    /// 更新拖拽方向。
    ///
    /// 真源 `handleGroupedTaskDragMove`（:1362-1380）比较的是**增量**
    /// `event.delta.y` 与 `lastDragDeltaY`，不是绝对位置：
    /// 增量变大→ `after`，变小 → `before`。所以这个方法是「喂本次 dy 增量」，
    /// 内部自己存上一次的增量。
    ///
    /// 真源首次拖动时 `lastDragDeltaYRef` 初值为 0，故 dy>0 时首帧就是 `after`。
    pub fn feed_delta_y(&mut self, delta_y: f64) {
        // 真源 :1371-1376 的顺序很关键：先拿本次 nextDeltaY 与上次
        // lastDragDeltaYRef 比较，**再**更新 lastDragDeltaYRef。
        // 若先赋值就变成自己跟自己比，方向永远不会翻转。
        if delta_y > self.prev_delta_y {
            self.direction = DragDirection::After;
        } else if delta_y < self.prev_delta_y {
            self.direction = DragDirection::Before;
        }
        self.prev_delta_y = delta_y;
    }

    /// 是否正在拖拽（供 UI 禁用 hover action /显示 grabbing 光标）。
    pub fn cursor_class(&self) -> &'static str {
        if self.is_dragging() {
            "cursor-grabbing"
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groupedTasks::view::{GroupedTaskViewNode, TaskGroup, TaskListItem};

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

    fn sample() -> GroupedTaskView {
        GroupedTaskView {
            nodes: vec![
                GroupedTaskViewNode::Task {
                    task: task("/ws", "t1"),
                    sort_order: None,
                },
                GroupedTaskViewNode::Group {
                    group: TaskGroup {
                        group_id: "g1".into(),
                        title: "g1".into(),
                        color: "blue".into(),
                    },
                    tasks: vec![task("/ws", "t2")],
                    sort_order: None,
                },
            ],
        }
    }

    fn key(id: &str) -> String {
        super::super::view::task_key("/ws", None, id)
    }

    fn shape(view: &GroupedTaskView) -> String {
        view.nodes
            .iter()
            .map(|n| match n {
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

    // ── 状态机：begin / preview / finish ──

    #[test]
    fn begin_records_origin_and_resets_direction() {
        let mut rt = DragRuntime::default();
        rt.begin(
            DragSource::Task {
                task_key: key("t1"),
            },
            &sample(),
        );
        assert!(rt.is_dragging());
        assert_eq!(rt.direction, DragDirection::After, "真源初始方向是 after");
        assert!(rt.origin_view.is_some());
    }

    #[test]
    fn preview_without_active_source_returns_input() {
        let rt = DragRuntime::default();
        let view = sample();
        assert_eq!(
            shape(&rt.preview(&view, Some(DropTarget::Task { task_key: key("t1") }))),
            shape(&view)
        );
    }

    #[test]
    fn preview_applies_drop_decision() {
        let mut rt = DragRuntime::default();
        rt.begin(
            DragSource::Task {
                task_key: key("t2"),
            },
            &sample(),
        );
        // 把组内 t2 拖到 t1 之前（向上拖）→ 升为顶层且排在 t1 前。
        rt.direction = DragDirection::Before;
        let next = rt.preview(
            &sample(),
            Some(DropTarget::Task {
                task_key: key("t1"),
            }),
        );
        assert_eq!(shape(&next), "t2,t1,g1[]");
    }

    #[test]
    fn preview_with_no_target_returns_input() {
        let mut rt = DragRuntime::default();
        rt.begin(
            DragSource::Task {
                task_key: key("t2"),
            },
            &sample(),
        );
        assert_eq!(shape(&rt.preview(&sample(), None)), shape(&sample()));
    }

    #[test]
    fn finish_returns_none_when_view_unchanged() {
        // 真源 :1476-1484：签名相同 → 不落库（省一次 RPC + 避免无谓动画）。
        let view = sample();
        let mut rt = DragRuntime::default();
        let key1 = key("t1");
        rt.begin(
            DragSource::Task {
                task_key: key1.clone(),
            },
            &view,
        );
        let next = rt.preview(
            &view,
            Some(DropTarget::Task {
                task_key: key1.clone(),
            }),
        );
        assert!(rt.finish(&next).is_none(), "没实际移动就不该落库");
    }

    #[test]
    fn finish_returns_view_when_moved() {
        let view = sample();
        let mut rt = DragRuntime::default();
        rt.begin(
            DragSource::Task {
                task_key: key("t2"),
            },
            &view,
        );
        rt.direction = DragDirection::Before;
        let next = rt.preview(
            &view,
            Some(DropTarget::Task {
                task_key: key("t1"),
            }),
        );
        let committed = rt.finish(&next).expect("实际移动了应返回待落库视图");
        assert_eq!(shape(&committed), "t2,t1,g1[]");
        assert!(!rt.is_dragging(), "finish 后应清空 active");
    }

    #[test]
    fn finish_without_begin_returns_none() {
        let mut rt = DragRuntime::default();
        assert!(rt.finish(&sample()).is_none());
    }

    #[test]
    fn cancel_restores_origin_and_clears_state() {
        let view = sample();
        let mut rt = DragRuntime::default();
        rt.begin(
            DragSource::Task {
                task_key: key("t2"),
            },
            &view,
        );
        let restored = rt.cancel().expect("cancel 应交还原点视图供回滚");
        assert_eq!(shape(&restored), shape(&view));
        assert!(!rt.is_dragging());
        assert!(rt.cancel().is_none(), "重复 cancel 不应再交还视图");
    }

    // ── 方向增量语义（真源 dragMove）──

    #[test]
    fn direction_follows_delta_y_sign_change() {
        let mut rt = DragRuntime::default();
        // 真源 lastDragDeltaY 初值 0：首个正增量即判为 after（:1371-1373）。
        rt.feed_delta_y(5.0);
        assert_eq!(rt.direction, DragDirection::After);
        // 增量继续变大 → 维持 after
        rt.feed_delta_y(8.0);
        assert_eq!(rt.direction, DragDirection::After);
        // 增量变小（向上拖）→ before
        rt.feed_delta_y(3.0);
        assert_eq!(rt.direction, DragDirection::Before);
        // 增量继续变小 → 维持 before
        rt.feed_delta_y(1.0);
        assert_eq!(rt.direction, DragDirection::Before);
    }

    #[test]
    fn zero_delta_keeps_direction() {
        let mut rt = DragRuntime::default();
        rt.feed_delta_y(5.0);
        rt.feed_delta_y(5.0); // 相等 → 两个分支都不进
        assert_eq!(rt.direction, DragDirection::After);
    }

    #[test]
    fn negative_first_delta_is_before() {
        let mut rt = DragRuntime::default();
        rt.feed_delta_y(-3.0);
        assert_eq!(rt.direction, DragDirection::Before);
    }

    #[test]
    fn cursor_class_reflects_dragging() {
        let mut rt = DragRuntime::default();
        assert_eq!(rt.cursor_class(), "");
        rt.begin(
            DragSource::Task {
                task_key: key("t1"),
            },
            &sample(),
        );
        assert_eq!(rt.cursor_class(), "cursor-grabbing");
    }
}