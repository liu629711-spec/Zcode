//! 1:1 翻译 `packages/ui/src/workspace-file-tree/statusStyles.ts`（84 行）。
//!
//! git 状态的视觉词汇：状态字母、文字色、目录圆点色。三张映射表都按真源闭集
//! 逐项对齐——同一个状态在树里的字母、行文字色与圆点色必须来自同一处判定。

use super::model::WorkspaceFileGitStatus;

/// `getWorkspaceFileGitStatusIndicator`（statusStyles.ts:3-18）。
pub fn get_workspace_file_git_status_indicator(status: WorkspaceFileGitStatus) -> Option<&'static str> {
    match status {
        WorkspaceFileGitStatus::Modified => Some("M"),
        WorkspaceFileGitStatus::Added => Some("A"),
        WorkspaceFileGitStatus::Deleted => Some("D"),
        WorkspaceFileGitStatus::Renamed => Some("R"),
        WorkspaceFileGitStatus::Untracked => Some("U"),
        WorkspaceFileGitStatus::Ignored => None,
    }
}

/// `getWorkspaceFileGitStatusTextClassName`（statusStyles.ts:20-39）。
pub fn get_workspace_file_git_status_text_class_name(
    status: Option<WorkspaceFileGitStatus>,
) -> Option<&'static str> {
    match status {
        Some(WorkspaceFileGitStatus::Added) => Some("text-git-added"),
        Some(WorkspaceFileGitStatus::Untracked) => Some("text-git-untracked"),
        Some(WorkspaceFileGitStatus::Deleted) => Some("text-git-deleted"),
        Some(WorkspaceFileGitStatus::Ignored) => Some("text-git-ignored"),
        Some(WorkspaceFileGitStatus::Renamed) => Some("text-git-renamed"),
        Some(WorkspaceFileGitStatus::Modified) => Some("text-git-modified"),
        None => None,
    }
}

/// `getWorkspaceFileTreeRowDisplayGitStatus`（statusStyles.ts:41-51）。
///
/// 修复：目录文字颜色之前硬编码优先使用 untracked，和 descendant 圆点的
/// 聚合状态优先级不一致，导致同一个目录文字和圆点颜色表达冲突。
pub fn get_workspace_file_tree_row_display_git_status(
    git_status: Option<WorkspaceFileGitStatus>,
    directory_git_statuses: &[WorkspaceFileGitStatus],
) -> Option<WorkspaceFileGitStatus> {
    git_status.or_else(|| directory_git_statuses.first().copied())
}

/// `getWorkspaceFileGitStatusIndicatorClassName`（statusStyles.ts:53-70）。
pub fn get_workspace_file_git_status_indicator_class_name(
    status: WorkspaceFileGitStatus,
) -> Option<&'static str> {
    match status {
        WorkspaceFileGitStatus::Added => Some("text-git-added/70"),
        WorkspaceFileGitStatus::Untracked => Some("text-git-untracked/70"),
        WorkspaceFileGitStatus::Deleted => Some("text-git-deleted/70"),
        WorkspaceFileGitStatus::Renamed => Some("text-git-renamed/70"),
        WorkspaceFileGitStatus::Modified => Some("text-git-modified/70"),
        WorkspaceFileGitStatus::Ignored => Some("text-git-ignored"),
    }
}

/// `getWorkspaceFileGitStatusDotClassName`（statusStyles.ts:72-89）——目录聚合圆点。
pub fn get_workspace_file_git_status_dot_class_name(
    status: Option<WorkspaceFileGitStatus>,
) -> &'static str {
    match status {
        Some(WorkspaceFileGitStatus::Added) => "bg-git-added/60",
        Some(WorkspaceFileGitStatus::Untracked) => "bg-git-untracked/60",
        Some(WorkspaceFileGitStatus::Deleted) => "bg-git-deleted/60",
        Some(WorkspaceFileGitStatus::Ignored) => "bg-git-ignored/60",
        Some(WorkspaceFileGitStatus::Renamed) => "bg-git-renamed/60",
        Some(WorkspaceFileGitStatus::Modified) => "bg-git-modified/60",
        // 无聚合状态（shouldn't happen，真源 default 分支）→ 中性 descendant 色。
        None => "bg-git-descendant/60",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicator_letters_match_git_vocabulary() {
        // statusStyles.ts:3-18 —— M/A/D/R/U，ignored 无字母。
        assert_eq!(
            get_workspace_file_git_status_indicator(WorkspaceFileGitStatus::Modified),
            Some("M")
        );
        assert_eq!(
            get_workspace_file_git_status_indicator(WorkspaceFileGitStatus::Added),
            Some("A")
        );
        assert_eq!(
            get_workspace_file_git_status_indicator(WorkspaceFileGitStatus::Deleted),
            Some("D")
        );
        assert_eq!(
            get_workspace_file_git_status_indicator(WorkspaceFileGitStatus::Renamed),
            Some("R")
        );
        assert_eq!(
            get_workspace_file_git_status_indicator(WorkspaceFileGitStatus::Untracked),
            Some("U")
        );
        assert_eq!(
            get_workspace_file_git_status_indicator(WorkspaceFileGitStatus::Ignored),
            None
        );
    }

    #[test]
    fn display_status_prefers_own_then_first_directory_status() {
        // statusStyles.ts:41-51 —— 自身状态优先，目录回聚合表首位。
        assert_eq!(
            get_workspace_file_tree_row_display_git_status(
                Some(WorkspaceFileGitStatus::Modified),
                &[WorkspaceFileGitStatus::Untracked]
            ),
            Some(WorkspaceFileGitStatus::Modified)
        );
        assert_eq!(
            get_workspace_file_tree_row_display_git_status(
                None,
                &[WorkspaceFileGitStatus::Modified, WorkspaceFileGitStatus::Untracked]
            ),
            Some(WorkspaceFileGitStatus::Modified)
        );
        assert_eq!(
            get_workspace_file_tree_row_display_git_status(None, &[]),
            None
        );
    }

    #[test]
    fn dot_class_defaults_to_descendant_neutral() {
        // statusStyles.ts:72-89 —— None 走 bg-git-descendant/60。
        assert_eq!(
            get_workspace_file_git_status_dot_class_name(None),
            "bg-git-descendant/60"
        );
        assert_eq!(
            get_workspace_file_git_status_dot_class_name(Some(WorkspaceFileGitStatus::Added)),
            "bg-git-added/60"
        );
    }
}
