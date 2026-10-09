//! 1:1 翻译 `packages/ui/src/app-shell/` 下被工作流渲染层消费的部分。
//!
//! 目前只有 workflow-artifacts 的呈现层；WorkspaceShellLayout 等应用壳
//! 由 app.rs 按 1:1 骨架承担。

#![allow(non_snake_case)]

pub mod workflow_artifacts;
