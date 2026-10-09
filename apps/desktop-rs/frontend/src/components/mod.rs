//! 1:1 翻译 `packages/ui/src/components/` 下的共享组件与逻辑。
//!
//! 目前迁入的是 workflow-timeline 的设置轮措辞（被 toolCall 的多张卡共用）。

#![allow(non_snake_case)]

pub mod timeline_bands;
pub mod workflowSettingsChange;