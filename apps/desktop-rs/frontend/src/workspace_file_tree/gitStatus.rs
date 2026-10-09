//! 1:1 翻译 `packages/ui/src/workspace-file-tree/gitStatus.ts`（28 行）。
//!
//! git 状态装载：调主进程 `git_refresh`（真源 `gitService.refresh`），
//! 经 model 层纯函数折叠成 statusByPath。可用性判定同样走 model 层——
//! 「读取成功」不等于「Git 可用」。

use std::collections::HashMap;

use serde_json::Value;

use super::model::{
    build_workspace_file_git_status_by_path, is_workspace_file_tree_git_status_available,
    GitFileChangeInput, GitSummaryAvailability, WorkspaceFileGitStatus,
};

/// `WorkspaceFileTreeGitStatusState`（gitStatus.ts:8-11）。
#[derive(Debug, Clone, Default)]
pub struct WorkspaceFileTreeGitStatusState {
    pub available: bool,
    pub status_by_path: HashMap<String, WorkspaceFileGitStatus>,
}

/// wire 载荷 → `GitFileChangeInput`（真源直接传 `GitFileChange` 对象；
/// Rust 侧经 serde_json 防御性读取，字段缺失按真源 Pick 的必填语义丢弃该项）。
fn parse_change(value: &Value) -> Option<GitFileChangeInput> {
    Some(GitFileChangeInput {
        path: value.get("path")?.as_str()?.to_string(),
        kind: value
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("modified")
            .to_string(),
        section: value
            .get("section")
            .and_then(|v| v.as_str())
            .unwrap_or("unstaged")
            .to_string(),
        is_untracked: value
            .get("isUntracked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// `loadWorkspaceFileTreeGitStatus`（gitStatus.ts:13-30）。
pub async fn load_workspace_file_tree_git_status(
    workspace_path: &str,
) -> Result<WorkspaceFileTreeGitStatusState, String> {
    let payload = crate::app::invoke_json(
        "git_refresh",
        serde_json::json!({ "workspacePath": workspace_path }),
    )
    .await?;

    let summary = payload.get("summary").cloned().unwrap_or(Value::Null);
    let available = is_workspace_file_tree_git_status_available(&GitSummaryAvailability {
        is_git_available: summary
            .get("isGitAvailable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        is_repository: summary
            .get("isRepository")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    });
    if !available {
        return Ok(WorkspaceFileTreeGitStatusState::default());
    }

    let mut changes: Vec<GitFileChangeInput> = Vec::new();
    for group in ["unstagedChanges", "stagedChanges"] {
        for item in payload
            .get(group)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            if let Some(change) = parse_change(item) {
                changes.push(change);
            }
        }
    }
    Ok(WorkspaceFileTreeGitStatusState {
        available: true,
        status_by_path: build_workspace_file_git_status_by_path(&changes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unavailable_summary_yields_empty_state() {
        // gitStatus.ts:24-29 —— 不可用 → statusByPath 恒空 Map。
        let summary = json!({ "isGitAvailable": true, "isRepository": false });
        let available = is_workspace_file_tree_git_status_available(&GitSummaryAvailability {
            is_git_available: summary["isGitAvailable"].as_bool().unwrap(),
            is_repository: summary["isRepository"].as_bool().unwrap(),
        });
        assert!(!available);
        assert!(WorkspaceFileTreeGitStatusState::default().status_by_path.is_empty());
    }

    #[test]
    fn parse_change_requires_path() {
        // 真源 Pick 语义：path 是必填标识；kind/section 缺席有兜底。
        assert!(parse_change(&json!({ "kind": "modified" })).is_none());
        let parsed = parse_change(&json!({ "path": "a.rs" })).unwrap();
        assert_eq!(parsed.kind, "modified");
        assert_eq!(parsed.section, "unstaged");
        assert!(!parsed.is_untracked);
    }
}
