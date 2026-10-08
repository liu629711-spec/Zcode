//! 1:1 翻译 `packages/ui/src/workspace-grouped-tasks/task-context-menu-content.tsx`
//! 中**可离线实现**的菜单动作。
//!
//! ## 迁移边界（重要）
//!
//! 真源菜单 13 项，按「是否依赖本模块能力」分三类：
//!
//! | 类别 | 菜单项 | 本文件|
//! |---|---|---|
//! | **纯视图变换**（view.rs 已有算法） | 移动到分组 / 移出分组 / 移到顶部 | ✅ 已迁 |
//! | 依赖外部系统 | 归档 / 标未读 / 复制路径 / 文件管理器 / 打开反馈 | ⏸ 后续 |
//! | 依赖尚未迁移的功能 | 重命名 / 写验收标准 / 查看调用轨迹 | ⏸ 后续 |
//!
//! 之所以按能力切而非按文件整迁：菜单项本身只是回调绑定，
//! 真价值在`view.rs` 的 `moveTaskByMenu` / `moveTaskToTopByMenu`——
//! 那些已在前几轮迁完。这里把菜单**结构与文案**对齐，执行走同一批函数。
//!
//! ## 文案照抄真源 zh-CN.ts
//!
//! 13 条 key 的中文文案逐条对照 packages/ui/src/i18n/locales/zh-CN.ts:
//! - `taskGroup.moveToGroup`:1890→ "移动到分组"
//! - `taskGroup.moveToTop`:1973→ "移动到顶部"
//! - `taskGroup.removeFromGroup`:1974→ "移出分组"
//! - `taskList.rename`:1890→ "重命名任务"
//! - `taskList.archive`:1891→ "归档任务"
//! - `taskList.markAsUnread`:1899→ "标记为未读"
//! - `taskList.feedback`:1942→ "反馈问题"
//! - `taskList.viewModelTrajectory`:1943→ "查看调用轨迹"
//! - `dispatchDesk.writeSpec`:1937→ "写验收标准"
//! - `appHeader.copyPath`/`copyTaskPath`/`copyLogPath`/`copySessionId`（:1413/1416/1421/1422）
//!
//! **不自造文案**——这与项目红线一致：标签显示用中文，语义与真源枚举一一对应。

/// 菜单项标识（对齐真源 13 个 `on*` 回调）。
///
/// 未实现的三类也保留在枚举里，附带 `implemented` 标记，
/// 这样 UI 可以显示成禁用项（真源的 `disabledReason` 机制）而非直接消失——
/// 消失会让用户以为功能不存在。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskMenuAction {
    // ── 纯视图变换（已实现）──
    /// 移出分组（移到顶层，游离节点）。
    RemoveFromGroup,
    /// 移动到指定分组。
    MoveToGroup,
    /// 移到顶部（顶层首位或组内首位，由 view.rs 按当前位置决定）。
    MoveToTop,

    // ── 依赖外部系统（待接）──
    ArchiveTask,
    MarkAsUnread,
    CopyPath,
    CopyTaskPath,
    CopyLogPath,
    CopySessionId,
    OpenTaskPathInFileManager,
    OpenTaskFeedback,

    // ── 依赖尚未迁移的功能（待接）──
    RenameTask,
    WriteTicketSpec,
    ViewModelTrajectory,
}

impl TaskMenuAction {
    /// 中文文案（照抄 zh-CN.ts，见模块头对照表）。
    pub fn label_zh(self) -> &'static str {
        match self {
            TaskMenuAction::MoveToGroup => "移动到分组",
            TaskMenuAction::RemoveFromGroup => "移出分组",
            TaskMenuAction::MoveToTop => "移动到顶部",
            TaskMenuAction::RenameTask => "重命名任务",
            TaskMenuAction::WriteTicketSpec => "写验收标准",
            TaskMenuAction::ArchiveTask => "归档任务",
            TaskMenuAction::MarkAsUnread => "标记为未读",
            TaskMenuAction::CopyPath => "复制路径",
            TaskMenuAction::CopyTaskPath => "复制任务路径",
            TaskMenuAction::CopyLogPath => "复制日志路径",
            TaskMenuAction::CopySessionId => "复制会话 ID",
            TaskMenuAction::OpenTaskFeedback => "反馈问题",
            TaskMenuAction::ViewModelTrajectory => "查看调用轨迹",
            // 文件管理器项的真源文案随平台变化（fileManagerLabel 入参），
            // 这里给通用占位——调用方应覆盖。
            TaskMenuAction::OpenTaskPathInFileManager => "在文件管理器中显示",
        }
    }

    /// 本模块是否已实现（未实现的应渲染为禁用项）。
    pub fn is_implemented(self) -> bool {
        matches!(
            self,
            TaskMenuAction::MoveToGroup
                | TaskMenuAction::RemoveFromGroup
                | TaskMenuAction::MoveToTop
        )
    }

    /// 无实现项的禁用原因（真源的 `disabledReason`，:47 / :62 传进 `title`）。
    pub fn disabled_reason(self) -> Option<&'static str> {
        if self.is_implemented() {
            None
        } else {
            Some("功能尚未迁移")
        }
    }
}

/// 菜单项分组（真源用 `ContextMenuSeparator` 隔开，:89 / :103 / :133）。
///
/// 按真源顺序：
/// ```text
/// [ 移动到▸（含移出分组 + 各组） ]
/// [ 移动到顶部 ]
/// ────────
/// [ 重命名任务 ]
/// [ 写验收标准 ]
/// [ 归档任务 ]
/// [ 标记为未读 ]
/// [ 复制路径 / 复制任务路径 / 复制日志路径 / 复制会话 ID ]
/// ────────
/// [ 在文件管理器中显示 ]
/// [ 反馈问题 ]
/// [ 查看调用轨迹 ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSection {
    /// 「移动到分组」子菜单（含「移出分组」+各组）。
    MoveToGroup,
    MoveToTop,
    Rename,
    WriteSpec,
    Archive,
    MarkUnread,
    Copy,
    OpenFileManager,
    Feedback,
    ViewTrajectory,
}

/// 菜单项在渲染层的一条记录（已解析文案与禁用态）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub action: TaskMenuAction,
    pub label: String,
    pub disabled: bool,
    /// 禁用原因（真源透传给 `title`，悬停时可见）。
    pub disabled_reason: Option<String>,
    /// 组条目专有：移动目标组 id。
    ///
    /// 真源直接用 React key 传 `group.id`；Rust 侧菜单项必须显式携带，
    /// 否则渲染层只能按 label 反查 id——两个组同名时会移错。
    pub group_id: Option<String>,
    /// 子菜单项（非空表示这是分组子菜单）。
    pub submenu: Vec<MenuEntry>,
}

impl MenuEntry {
    fn new(action: TaskMenuAction) -> Self {
        Self {
            action,
            label: action.label_zh().to_string(),
            disabled: !action.is_implemented(),
            disabled_reason: action.disabled_reason().map(|s| s.to_string()),
            group_id: None,
            submenu: Vec::new(),
        }
    }
}

/// 一个菜单分节的完整内容（对应真源的一段 `ContextMenuItem` / `Separator`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSectionContent {
    pub section: MenuSection,
    /// 本分节的条目（`MoveToGroup` 分节含子菜单条目）。
    pub entries: Vec<MenuEntry>,
}

/// 构建完整的菜单结构。
///
/// `groups` 是可选分组列表（真源 `groups: TaskGroupMenuItem[]`，:33），
/// 每项是 `(group_id, title, color)`。
///
/// **当前组也要列在子菜单里但标为禁用**（真源 :79 `group.id === currentGroupId`
/// → disabled），这样用户能看到自己在哪个组，但不能重复移动。
///
/// `current_group_id` 为 `None` 表示任务在顶层，此时「移出分组」应禁用
/// （真源 :65 `!currentGroupId` → disabled）。
pub fn build_menu(
    groups: &[(String, String, String)],
    current_group_id: Option<&str>,
) -> Vec<MenuSectionContent> {
    let mut submenu = Vec::new();

    // 移出分组：仅当任务确实在某个组内才可用（真源 :65）。
    let remove = TaskMenuAction::RemoveFromGroup;
    submenu.push(MenuEntry {
        disabled: current_group_id.is_none(),
        ..MenuEntry::new(remove)
    });

    // 各组条目：当前组禁用（真源 :79），标题前带颜色标记（:81）。
    // group_id 显式携带——渲染层按它移动，不做 label 反查。
    for (group_id, title, _color) in groups {
        let is_current = current_group_id == Some(group_id.as_str());
        submenu.push(MenuEntry {
            action: TaskMenuAction::MoveToGroup,
            label: title.clone(),
            disabled: is_current,
            disabled_reason: if is_current {
                Some("已在该分组".into())
            } else {
                None
            },
            group_id: Some(group_id.clone()),
            submenu: Vec::new(),
        });
    }

    // 真源顺序：移动到分组▸ → 移到顶部 → 重命名 → 写验收 → 归档 → 未读
    // → 复制路径 → 文件管理器 → 反馈 → 轨迹（:56-215）。
    [
        (MenuSection::MoveToGroup, submenu),
        (
            MenuSection::MoveToTop,
            vec![MenuEntry::new(TaskMenuAction::MoveToTop)],
        ),
        (
            MenuSection::Rename,
            vec![MenuEntry::new(TaskMenuAction::RenameTask)],
        ),
        (
            MenuSection::WriteSpec,
            vec![MenuEntry::new(TaskMenuAction::WriteTicketSpec)],
        ),
        (
            MenuSection::Archive,
            vec![MenuEntry::new(TaskMenuAction::ArchiveTask)],
        ),
        (
            MenuSection::MarkUnread,
            vec![MenuEntry::new(TaskMenuAction::MarkAsUnread)],
        ),
        (
            MenuSection::Copy,
            vec![
                MenuEntry::new(TaskMenuAction::CopyPath),
                MenuEntry::new(TaskMenuAction::CopyTaskPath),
                MenuEntry::new(TaskMenuAction::CopyLogPath),
                MenuEntry::new(TaskMenuAction::CopySessionId),
            ],
        ),
        (
            MenuSection::OpenFileManager,
            vec![MenuEntry::new(TaskMenuAction::OpenTaskPathInFileManager)],
        ),
        (
            MenuSection::Feedback,
            vec![MenuEntry::new(TaskMenuAction::OpenTaskFeedback)],
        ),
        (
            MenuSection::ViewTrajectory,
            vec![MenuEntry::new(TaskMenuAction::ViewModelTrajectory)],
        ),
    ]
    .into_iter()
    .map(|(section, entries)| MenuSectionContent { section, entries })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_zh_cn_source() {
        // 对照 packages/ui/src/i18n/locales/zh-CN.ts 的实际文案。
        assert_eq!(TaskMenuAction::MoveToGroup.label_zh(), "移动到分组");
        assert_eq!(TaskMenuAction::RemoveFromGroup.label_zh(), "移出分组");
        assert_eq!(TaskMenuAction::MoveToTop.label_zh(), "移动到顶部");
        assert_eq!(TaskMenuAction::RenameTask.label_zh(), "重命名任务");
        assert_eq!(TaskMenuAction::WriteTicketSpec.label_zh(), "写验收标准");
        assert_eq!(TaskMenuAction::ArchiveTask.label_zh(), "归档任务");
        assert_eq!(TaskMenuAction::MarkAsUnread.label_zh(), "标记为未读");
        assert_eq!(TaskMenuAction::CopyTaskPath.label_zh(), "复制任务路径");
        assert_eq!(TaskMenuAction::CopyLogPath.label_zh(), "复制日志路径");
        assert_eq!(TaskMenuAction::CopySessionId.label_zh(), "复制会话 ID");
        assert_eq!(TaskMenuAction::OpenTaskFeedback.label_zh(), "反馈问题");
        assert_eq!(
            TaskMenuAction::ViewModelTrajectory.label_zh(),
            "查看调用轨迹"
        );
    }

    #[test]
    fn implemented_actions_are_exactly_the_three_view_ones() {
        // 只有走 view.rs 的三个已实现；其余应禁用而非隐藏
        // （隐藏会让用户以为功能不存在）。
        assert!(TaskMenuAction::MoveToGroup.is_implemented());
        assert!(TaskMenuAction::RemoveFromGroup.is_implemented());
        assert!(TaskMenuAction::MoveToTop.is_implemented());
        assert!(!TaskMenuAction::ArchiveTask.is_implemented());
        assert!(!TaskMenuAction::RenameTask.is_implemented());
        assert!(!TaskMenuAction::CopyPath.is_implemented());
    }

    #[test]
    fn unimplemented_has_disabled_reason() {
        assert_eq!(
            TaskMenuAction::ArchiveTask.disabled_reason(),
            Some("功能尚未迁移")
        );
        assert_eq!(TaskMenuAction::MoveToTop.disabled_reason(), None);
    }

    #[test]
    fn menu_entry_defaults_to_disabled_for_unimplemented() {
        let entry = MenuEntry::new(TaskMenuAction::ArchiveTask);
        assert!(entry.disabled);
        assert_eq!(entry.disabled_reason.as_deref(), Some("功能尚未迁移"));
        assert_eq!(entry.label, "归档任务");

        let entry = MenuEntry::new(TaskMenuAction::MoveToTop);
        assert!(!entry.disabled);
        assert_eq!(entry.disabled_reason, None);
    }

    #[test]
    fn menu_sections_match_source_order() {
        let groups = vec![
            ("g1".to_string(), "工作".to_string(), "blue".to_string()),
            ("g2".to_string(), "个人".to_string(), "red".to_string()),
        ];
        let sections = build_menu(&groups, Some("g1"));
        // 真源顺序：移动到分组 → 移到顶部 → 重命名 → 写验收 → 归档 → 未读
        // → 复制 → 文件管理器 → 反馈 → 轨迹。
        assert_eq!(sections.len(), 10);
        assert_eq!(sections[0].section, MenuSection::MoveToGroup);
        assert_eq!(sections[1].section, MenuSection::MoveToTop);
        assert_eq!(sections[2].section, MenuSection::Rename);
        assert_eq!(sections[8].section, MenuSection::Feedback);
        assert_eq!(sections[9].section, MenuSection::ViewTrajectory);
    }

    #[test]
    fn current_group_is_listed_but_disabled() {
        // 真源 :79：当前组仍在子菜单里，只是 disabled + title 说明原因。
        let groups = vec![
            ("g1".to_string(), "工作".to_string(), "blue".to_string()),
            ("g2".to_string(), "个人".to_string(), "red".to_string()),
        ];
        let sections = build_menu(&groups, Some("g1"));
        let submenu = &sections[0].entries;
        // 第 0 项是「移出分组」，第 1 项是分隔符占位，之后才是各组。
        let g1 = submenu.iter().find(|e| e.label == "工作").unwrap();
        assert!(g1.disabled, "当前组应禁用");
        assert_eq!(g1.disabled_reason.as_deref(), Some("已在该分组"));
        let g2 = submenu.iter().find(|e| e.label == "个人").unwrap();
        assert!(!g2.disabled, "非当前组应可用");
    }

    #[test]
    fn remove_from_group_disabled_when_task_is_top_level() {
        // 真源 :65：!currentGroupId → disabled。
        let sections = build_menu(&[], None);
        let remove = &sections[0].entries[0];
        assert_eq!(remove.action, TaskMenuAction::RemoveFromGroup);
        assert!(remove.disabled, "顶层任务的「移出分组」应禁用");
    }

    #[test]
    fn remove_from_group_enabled_when_task_is_in_group() {
        let groups = vec![("g1".to_string(), "工作".to_string(), "blue".to_string())];
        let sections = build_menu(&groups, Some("g1"));
        let remove = &sections[0].entries[0];
        assert!(!remove.disabled);
    }

    #[test]
    fn group_entries_carry_explicit_group_id() {
        // 渲染层按 group_id 移动，不做 label 反查——两个组同名时
        // 反查会移到先出现的那个（真源用 React key 传 id 无此问题）。
        let groups = vec![
            ("g1".to_string(), "同名".to_string(), "blue".to_string()),
            ("g2".to_string(), "同名".to_string(), "red".to_string()),
        ];
        let sections = build_menu(&groups, None);
        let group_entries: Vec<&MenuEntry> = sections[0]
            .entries
            .iter()
            .filter(|e| e.action == TaskMenuAction::MoveToGroup && !e.label.is_empty())
            .collect();
        assert_eq!(group_entries.len(), 2);
        assert_eq!(group_entries[0].group_id.as_deref(), Some("g1"));
        assert_eq!(group_entries[1].group_id.as_deref(), Some("g2"));
        // 同名但 id 不同——这是显式携带 id 的意义。
        assert_ne!(group_entries[0].group_id, group_entries[1].group_id);
    }
}
