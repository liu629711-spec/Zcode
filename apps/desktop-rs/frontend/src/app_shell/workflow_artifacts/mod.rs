//! 1:1 翻译 `packages/ui/src/app-shell/workflow-artifacts/` 下的呈现层。
//!
//! 当前迁入：
//! - `presets/parts.ts` 的 `PresetLabels`（喂给预置渲染器的三句译文）；
//! - `artifactPresentation.tsx`（248 行）的呈现规则。
//!
//! 其余成员（`WorkflowArtifactTilePreview`、`ArtifactPresetBody`、四个预置渲染器）等
//! run 侧板 / 产物 tab 迁到时再进。

#![allow(non_snake_case)]

pub mod artifactPresentation;
pub mod presets;
