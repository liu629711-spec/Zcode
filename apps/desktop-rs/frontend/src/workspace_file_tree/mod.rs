//! 1:1 翻译 `packages/ui/src/workspace-file-tree/`（git 状态 / watcher 剩余项批次）。
//!
//! 模块划分对齐真源目录：
//! - `model.rs` —— model.ts 的 git 状态纯层（树形平铺部分已在 file_tree.rs 落地）
//! - `statusStyles.rs` —— statusStyles.ts（状态字母与着色）
//! - `gitStatus.rs` —— gitStatus.ts（git_refresh 载荷 → statusByPath）
//! - `useWorkspaceFileTreeWatchers.rs` —— useWorkspaceFileTreeWatchers.ts（目录监听登记）
//!
//! 依赖的主进程协议：`git_refresh` / `fs_watch` / `fs_unwatch` + `fs-watcher-change`
//! 事件（本批在 src-tauri/lib.rs 落地）。

pub mod gitStatus;
pub mod model;
pub mod statusStyles;
pub mod useWorkspaceFileTreeWatchers;
