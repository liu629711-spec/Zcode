//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/`（12 文件）+ `renderers/`（58 文件）。
//!
//! **每个真源文件对应本目录下一个同名 .rs 文件**，目录结构与行号注释都照抄真源，
//! 便于逐文件对照 diff 与逐文件验收。
//!
//! 模块名沿用 PascalCase 是刻意的（对齐真源文件名），故豁免 snake_case 警告。
#![allow(non_snake_case)]

pub mod ToolCallBlock;
pub mod ToolCallBody;
pub mod ToolLayout;
pub mod ToolSnapshotFieldNotice;
pub mod ToolSummaryRow;
pub mod codePreviewPreferences;
pub mod codePreviewSettings;
pub mod codeViewer;
pub mod exploreToolCall;
pub mod fileDisplay;
pub mod fileSummaries;
pub mod fileSummaryHeuristics;
pub mod fileSummaryTypes;
pub mod i18n;
pub mod patchDiffPreview;
pub mod renderers;
pub mod resolveRenderer;
pub mod toolCallRowAdapter;
pub mod toolDiffPreview;
pub mod toolDisplay;
pub mod toolError;
pub mod toolIdentity;
pub mod toolResultDisplay;
pub mod toolStatus;
