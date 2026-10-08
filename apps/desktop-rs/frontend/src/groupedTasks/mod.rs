//! 1:1 翻译 `packages/ui/src/workspace-grouped-tasks/` 的**纯逻辑层**。
//!
//! 真源这两个文件不含 JSX，是可独立测试的视图变换算法：
//! - `ids.ts`（10 行）—— `taskKey`，视图内任务唯一键；
//! - `view.ts`（581 行）—— task / group 的查找、移除、插入、重排，
//!   以及拖拽与右键菜单移动产生的视图变换。
//!
//! 渲染层由同级`grouped.rs` 负责（已迁）；本模块补的是它明确留白的部分：
//! 「未做（依赖真源交互体系）：拖拽重排（view.ts 581 行 + dnd-kit）」。
//!
//! 模块名沿用PascalCase 里的下划线风格是刻意的（对齐真源文件名 view），
//! 故豁免 snake_case 警告。
#![allow(non_snake_case)]

pub mod view;
