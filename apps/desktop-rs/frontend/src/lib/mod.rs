//! 1:1 翻译 `packages/ui/src/lib/` 下的纯模型与存储。
//!
//! 目前迁入的是 workflow 渲染层直接消费的三个库；其余（文件展示、模型清单、
//! 会话投影等）等各自的消费面迁到时再进。

#![allow(non_snake_case)]
#![allow(special_module_name)]

pub mod nodeReplToolDisplay;
pub mod taskListItemPresentation;
pub mod workDuration;
pub mod workflowRunAckStore;
pub mod workflowRunLine;
