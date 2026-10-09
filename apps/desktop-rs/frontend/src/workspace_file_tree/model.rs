//! 1:1 翻译 `packages/ui/src/workspace-file-tree/model.ts` 的 **git 状态纯层**
//! （:22-208 与 :324-381；树形平铺 / compact 压平已在 file_tree.rs 落地）。
//!
//! 状态合并的三条真源修复注释都随实现搬进来：
//! 1. staged added/deleted/renamed 不许被压成 M/U（按优先级保留）；
//! 2. 目录 descendant 用独立的 decoration 优先级（modified 高于 U/A/D，VS Code 同款）；
//! 3. deleted 文件已不在文件系统，readdir 造不出行——从 statusByPath 反向注入。

use std::collections::{HashMap, HashSet};

/// `WorkspaceFileGitStatus`（model.ts:22-28，闭集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceFileGitStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
}

impl WorkspaceFileGitStatus {
    /// 真源的 wire 字面量（statusByPath 的 key 语义仅用于展示，不落协议）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modified => "modified",
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::Untracked => "untracked",
            Self::Ignored => "ignored",
        }
    }

    /// `WORKSPACE_FILE_GIT_STATUS_PRIORITY`（model.ts:30-37）——文件状态合并优先级。
    pub fn priority(self) -> u8 {
        match self {
            Self::Ignored => 0,
            Self::Modified => 1,
            Self::Renamed => 2,
            Self::Deleted => 3,
            Self::Added => 4,
            Self::Untracked => 5,
        }
    }

    /// `WORKSPACE_FILE_GIT_DIRECTORY_STATUS_PRIORITY`（model.ts:39-46）
    /// ——目录圆点的 decoration 优先级（modified 最高，与文件合并优先级**不是**一张表）。
    pub fn directory_priority(self) -> u8 {
        match self {
            Self::Added | Self::Deleted | Self::Renamed | Self::Untracked => 1,
            Self::Modified => 2,
            Self::Ignored => 3,
        }
    }
}

/// `normalizeForRelativePath`（model.ts:48-50）。
pub fn normalize_for_relative_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        // 多个尾斜杠 `\/+$` 全吃掉：一次 trim 不够，循环到稳态。
        .trim_end_matches('/')
        .to_string()
}

/// `resolveWorkspaceFileGitStatus`（model.ts:56-64）。
fn resolve_git_status(change: &GitFileChangeInput) -> WorkspaceFileGitStatus {
    if change.is_untracked || change.section == "untracked" {
        return WorkspaceFileGitStatus::Untracked;
    }
    match change.kind.as_str() {
        "added" => WorkspaceFileGitStatus::Added,
        "deleted" => WorkspaceFileGitStatus::Deleted,
        "renamed" => WorkspaceFileGitStatus::Renamed,
        // 闭集外兜底 modified（真源 TS 类型层面排除了这种情况）。
        _ => WorkspaceFileGitStatus::Modified,
    }
}

/// `GitFileChange` 的读取子集（model.ts:66 的 `Pick<...>`；wire 字段见 packages/shared git.ts:58-71）。
#[derive(Debug, Clone, PartialEq)]
pub struct GitFileChangeInput {
    pub path: String,
    pub kind: String,
    pub section: String,
    pub is_untracked: bool,
}

/// `buildWorkspaceFileGitStatusByPath`（model.ts:66-89）。
pub fn build_workspace_file_git_status_by_path(
    changes: &[GitFileChangeInput],
) -> HashMap<String, WorkspaceFileGitStatus> {
    let mut status_by_path: HashMap<String, WorkspaceFileGitStatus> = HashMap::new();
    for change in changes {
        let path_key = normalize_for_relative_path(&change.path);
        let next_status = resolve_git_status(change);
        // 修复：文件树之前把 Git 改动统一压成 M/U，导致 staged added/deleted/renamed
        // 在树里丢失真实状态。这里保留优先级更高的状态，兼容同一文件同时存在
        // staged/unstaged 记录。
        if let Some(&existing) = status_by_path.get(&path_key) {
            if existing.priority() >= next_status.priority() {
                continue;
            }
        }
        status_by_path.insert(path_key, next_status);
    }
    status_by_path
}

/// summary 的可用性读取子集（model.ts:91-97 的 `Pick<GitRepositorySummary, ...>`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GitSummaryAvailability {
    pub is_git_available: bool,
    pub is_repository: bool,
}

/// `isWorkspaceFileTreeGitStatusAvailable`（model.ts:91-97）。
///
/// 修复：`getChanges()` 在非 Git workspace 里也会返回空数组，不能把「读取成功」
/// 当成 Git 状态可用。文件树只在 Git 可执行且当前 workspace 属于仓库时展示变更过滤。
pub fn is_workspace_file_tree_git_status_available(summary: &GitSummaryAvailability) -> bool {
    summary.is_git_available && summary.is_repository
}

/// `getWorkspaceFileGitStatus`（model.ts:99-104）。
pub fn get_workspace_file_git_status(
    status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
    file_path: &str,
) -> Option<WorkspaceFileGitStatus> {
    status_by_path.get(&normalize_for_relative_path(file_path)).copied()
}

/// `getWorkspaceDirectoryGitStatuses`（model.ts:132-156）。
///
/// 修复：目录 descendant 之前沿用了文件状态合并优先级，导致 U/A/D 会盖过 M。
/// VS Code 的 Git resource priority 是 modified 高于新增/删除/重命名/未跟踪，
/// 目录聚合只决定文件夹圆点样式，因此单独使用这套 decoration 优先级。
pub fn get_workspace_directory_git_statuses(
    status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
    directory_path: &str,
) -> Vec<WorkspaceFileGitStatus> {
    let normalized = normalize_for_relative_path(directory_path);
    let prefix = format!("{normalized}/");
    let mut statuses: HashSet<WorkspaceFileGitStatus> = HashSet::new();
    for (path, status) in status_by_path {
        if normalize_for_relative_path(path).starts_with(&prefix) {
            statuses.insert(*status);
        }
    }
    let mut sorted: Vec<WorkspaceFileGitStatus> = statuses.into_iter().collect();
    sorted.sort_by(|left, right| {
        right
            .directory_priority()
            .cmp(&left.directory_priority())
            .then_with(|| left.as_str().cmp(right.as_str()))
    });
    sorted
}

/// `isWorkspaceFileTreeDeletedFile`（model.ts:117-122）。
pub fn is_workspace_file_tree_deleted_file(
    is_directory: bool,
    git_status: Option<WorkspaceFileGitStatus>,
) -> bool {
    !is_directory && git_status == Some(WorkspaceFileGitStatus::Deleted)
}

/// 树节点注入用输入（model.ts 的 `WorkspaceFileTreeNode` 子集；与 file_tree.rs 的
/// TreeNode 同构，这里只保留注入函数用到的字段）。
#[derive(Debug, Clone, PartialEq)]
pub struct GitDeletedNode {
    pub path: String,
    pub name: String,
    pub depth: usize,
    /// 软链接目录不参与自动压平（isWorkspaceFileTreeAutoFlattenableDirectory）。
    pub is_symlink: bool,
}

/// `addDeletedGitStatusRowsToWorkspaceFileTree`（model.ts:158-208）。
///
/// 修复：deleted 文件已不在文件系统里，单靠 readdir 无法生成行，导致文件树只能在
/// 目录上显示聚合点，看不到具体文件的 D 状态。children 表达为
/// `Vec<(节点, 是否目录)>` 以保留真源 `type: "file" | "directory"` 的排序语义
/// （目录在前，同类型 localeCompare）。
pub fn add_deleted_git_status_rows_to_tree(
    root_path: &str,
    children_by_directory: &HashMap<String, Vec<(GitDeletedNode, bool)>>,
    status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
) -> HashMap<String, Vec<(GitDeletedNode, bool)>> {
    let mut next: Option<HashMap<String, Vec<(GitDeletedNode, bool)>>> = None;

    for (path, status) in status_by_path {
        if *status != WorkspaceFileGitStatus::Deleted {
            continue;
        }
        let Some(parent_directory) = get_workspace_file_parent_directory(root_path, path) else {
            continue;
        };
        let current_map = next.as_ref().unwrap_or(children_by_directory);
        let Some(current_children) = current_map.get(&parent_directory) else {
            continue;
        };
        let normalized_path = normalize_for_relative_path(path);
        if current_children
            .iter()
            .any(|(child, _)| normalize_for_relative_path(&child.path) == normalized_path)
        {
            continue;
        }

        let mut appended = current_children.clone();
        appended.push((
            GitDeletedNode {
                path: path.clone(),
                name: path_leaf(path).to_string(),
                depth: get_workspace_file_directory_child_depth(root_path, &parent_directory),
                is_symlink: false,
            },
            false,
        ));
        // 目录在前，同类型按名称 localeCompare（model.ts:193-199）。
        appended.sort_by(|(left, left_dir), (right, right_dir)| {
            right_dir.cmp(left_dir).then_with(|| left.name.cmp(&right.name))
        });

        next
            .get_or_insert_with(|| children_by_directory.clone())
            .insert(parent_directory, appended);
    }

    next.unwrap_or_else(|| children_by_directory.clone())
}

/// `getWorkspaceFileDirectoryChildDepth`（model.ts:274-290）。
pub fn get_workspace_file_directory_child_depth(
    workspace_path: &str,
    directory_path: &str,
) -> usize {
    let ws = normalize_for_relative_path(workspace_path);
    let dir = normalize_for_relative_path(directory_path);
    if dir == ws || !dir.starts_with(&ws) {
        return 0;
    }
    dir[ws.len() + 1..]
        .split('/')
        .filter(|s| !s.is_empty())
        .count()
}

/// `getWorkspaceFileParentDirectory`（model.ts:292-318）。
pub fn get_workspace_file_parent_directory(
    workspace_path: &str,
    directory_path: &str,
) -> Option<String> {
    let ws = normalize_for_relative_path(workspace_path);
    let dir = normalize_for_relative_path(directory_path);
    if dir == ws || !dir.starts_with(&ws) {
        return None;
    }
    let relative_segments: Vec<&str> = dir[ws.len() + 1..]
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let parent_segments = &relative_segments[..relative_segments.len().saturating_sub(1)];
    if parent_segments.is_empty() {
        // 真源 :313 —— 父级就是 workspace 根本身（剥掉尾部分隔符）。
        return Some(workspace_path.trim_end_matches(['\\', '/']).to_string());
    }
    // 真源分隔符判定：原路径只含反斜杠才用 `\`，其余用 `/`。
    let separator = if directory_path.contains('\\') && !directory_path.contains('/') {
        "\\"
    } else {
        "/"
    };
    let base = workspace_path.trim_end_matches(['\\', '/']);
    Some(format!("{base}{separator}{}", parent_segments.join(separator)))
}

/// `getPathLeaf`（真源 lib/path.js；model.ts:189 的注入行名来源）。
fn path_leaf(path: &str) -> &str {
    path.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

/// `filterWorkspaceFileTreeRows` 的行输入（与 file_tree.rs 的 TreeRow 对齐的字段子集）。
#[derive(Debug, Clone, PartialEq)]
pub struct FilterRow {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

/// `isWorkspaceFileTreeRowChanged`（model.ts:324-336）。
fn is_row_changed(
    row: &FilterRow,
    status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
) -> bool {
    let git_status = get_workspace_file_git_status(status_by_path, &row.path);
    if git_status.is_some_and(|status| status != WorkspaceFileGitStatus::Ignored) {
        return true;
    }
    row.is_dir && !get_workspace_directory_git_statuses(status_by_path, &row.path).is_empty()
}

/// `filterWorkspaceFileTreeRows`（model.ts:338-381）。
pub fn filter_rows(
    rows: &[FilterRow],
    search_query: &str,
    changed_only: bool,
    status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
) -> Vec<FilterRow> {
    let changed_filtered: Vec<FilterRow> = if changed_only {
        rows.iter()
            .filter(|row| is_row_changed(row, status_by_path))
            .cloned()
            .collect()
    } else {
        rows.to_vec()
    };
    let query = search_query.trim().to_lowercase();
    if query.is_empty() {
        return changed_filtered;
    }

    let matched: Vec<&FilterRow> = changed_filtered
        .iter()
        .filter(|row| row.name.to_lowercase().contains(&query))
        .collect();
    if matched.is_empty() {
        return Vec::new();
    }

    let matched_paths: HashSet<String> = matched.iter().map(|row| row.path.clone()).collect();
    let matched_directories: Vec<String> = matched
        .iter()
        .filter(|row| row.is_dir)
        .map(|row| row.path.clone())
        .collect();

    changed_filtered
        .into_iter()
        .filter(|row| {
            if matched_paths.contains(&row.path) {
                return true;
            }
            if row.is_dir
                && matched_paths
                    .iter()
                    .any(|matched_path| is_workspace_file_path_inside(&row.path, matched_path))
            {
                return true;
            }
            matched_directories
                .iter()
                .any(|directory| is_workspace_file_path_inside(directory, &row.path))
        })
        .collect()
}

/// `isWorkspaceFilePathInside`（model.ts:230-237）。
pub fn is_workspace_file_path_inside(workspace_path: &str, file_path: &str) -> bool {
    let ws = normalize_for_relative_path(workspace_path);
    let path = normalize_for_relative_path(file_path);
    path == ws || path.starts_with(&format!("{ws}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(path: &str, kind: &str, section: &str, untracked: bool) -> GitFileChangeInput {
        GitFileChangeInput {
            path: path.into(),
            kind: kind.into(),
            section: section.into(),
            is_untracked: untracked,
        }
    }

    #[test]
    fn status_merge_keeps_higher_priority() {
        // model.ts:66-89 —— 同文件 staged M + unstaged D：D(3) > M(1) 保留 D。
        let by_path = build_workspace_file_git_status_by_path(&[
            change("a.rs", "modified", "staged", false),
            change("a.rs", "deleted", "unstaged", false),
        ]);
        assert_eq!(by_path.get("a.rs"), Some(&WorkspaceFileGitStatus::Deleted));

        // 后到的低优先级不覆盖高优先级。
        let by_path = build_workspace_file_git_status_by_path(&[
            change("b.rs", "deleted", "unstaged", false),
            change("b.rs", "modified", "staged", false),
        ]);
        assert_eq!(by_path.get("b.rs"), Some(&WorkspaceFileGitStatus::Deleted));

        // untracked 判定优先于 kind。
        let by_path = build_workspace_file_git_status_by_path(&[change("c.rs", "modified", "untracked", true)]);
        assert_eq!(by_path.get("c.rs"), Some(&WorkspaceFileGitStatus::Untracked));
    }

    #[test]
    fn availability_requires_git_and_repo() {
        // model.ts:91-97 —— 空数组不能当「可用」。
        assert!(is_workspace_file_tree_git_status_available(&GitSummaryAvailability {
            is_git_available: true,
            is_repository: true
        }));
        assert!(!is_workspace_file_tree_git_status_available(&GitSummaryAvailability {
            is_git_available: true,
            is_repository: false
        }));
        assert!(!is_workspace_file_tree_git_status_available(&GitSummaryAvailability {
            is_git_available: false,
            is_repository: true
        }));
    }

    #[test]
    fn directory_statuses_sort_by_decoration_priority() {
        // model.ts:132-156 —— 目录聚合 modified 优先于 untracked/added。
        let by_path = build_workspace_file_git_status_by_path(&[
            change("dir/new.txt", "added", "untracked", true),
            change("dir/sub/m.rs", "modified", "unstaged", false),
        ]);
        let statuses = get_workspace_directory_git_statuses(&by_path, "dir");
        assert_eq!(statuses.first(), Some(&WorkspaceFileGitStatus::Modified));
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn deleted_rows_are_injected_and_sorted() {
        // model.ts:158-208 —— deleted 文件不在 readdir 结果里，按 statusByPath 注入。
        let mut children: HashMap<String, Vec<(GitDeletedNode, bool)>> = HashMap::new();
        children.insert(
            "ws".into(),
            vec![
                (GitDeletedNode { path: "ws/sub".into(), name: "sub".into(), depth: 0, is_symlink: false }, true),
                (GitDeletedNode { path: "ws/a.txt".into(), name: "a.txt".into(), depth: 0, is_symlink: false }, false),
            ],
        );
        let by_path = HashMap::from([( "ws/gone.md".to_string(), WorkspaceFileGitStatus::Deleted)]);
        let next = add_deleted_git_status_rows_to_tree("ws", &children, &by_path);
        let rows = &next["ws"];
        let names: Vec<&str> = rows.iter().map(|(node, _)| node.name.as_str()).collect();
        // 目录在前，文件段 a.txt / gone.md 按名排序。
        assert_eq!(names, vec!["sub", "a.txt", "gone.md"]);

        // 已在树里的同名文件不重复注入。
        let by_path = HashMap::from([("ws/a.txt".to_string(), WorkspaceFileGitStatus::Deleted)]);
        let next = add_deleted_git_status_rows_to_tree("ws", &children, &by_path);
        assert_eq!(next["ws"].len(), 2);
    }

    #[test]
    fn parent_directory_respects_workspace_root() {
        // model.ts:292-318 —— workspace 本身没有父目录概念之外的路径回 None；
        // 子目录回其直接父级。
        assert_eq!(
            get_workspace_file_parent_directory("D:/ws", "D:/ws/a/b"),
            Some("D:/ws/a".to_string())
        );
        assert_eq!(
            get_workspace_file_parent_directory("D:/ws", "D:/ws/a"),
            Some("D:/ws".to_string())
        );
        assert_eq!(get_workspace_file_parent_directory("D:/ws", "D:/other"), None);
    }

    #[test]
    fn changed_filter_matches_files_and_dir_descendants() {
        // model.ts:324-381 —— changedOnly：文件看自身状态，目录看 descendant 聚合。
        let rows = vec![
            FilterRow { path: "ws/m.rs".into(), name: "m.rs".into(), is_dir: false },
            FilterRow { path: "ws/sub".into(), name: "sub".into(), is_dir: true },
            FilterRow { path: "ws/clean".into(), name: "clean".into(), is_dir: true },
            FilterRow { path: "ws/clean/x.txt".into(), name: "x.txt".into(), is_dir: false },
        ];
        let by_path = build_workspace_file_git_status_by_path(&[
            change("ws/m.rs", "modified", "unstaged", false),
            change("ws/sub/inner.txt", "added", "untracked", true),
        ]);
        let filtered = filter_rows(&rows, "", true, &by_path);
        let paths: Vec<&str> = filtered.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["ws/m.rs", "ws/sub"]);

        // 搜索 + changedOnly 叠加：先 changed 再按名过滤，命中目录保留其子树。
        let filtered = filter_rows(&rows, "sub", true, &by_path);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, "ws/sub");
    }
}
