//! 1:1 翻译 `packages/ui/src/workspace-grouped-tasks/` 的**纯逻辑层**。
//!
//! 真源这两个文件不含 JSX，是可独立测试的视图变换算法：
//! - `ids.ts`（10 行）—— `taskKey`，视图内任务唯一键；
//! - `view.ts`（581 行）—— task / group 的查找、移除、插入、重排，
//!   以及拖拽与右键菜单移动产生的视图变换。
//!
//! `join.rs` 对应真源里没有独立文件、但必须存在的那一步：把后端只返回
//! `task_ids` 的分组结构 join 成带完整 task 的视图（真源注释称之为
//! 「由客户端与 sessions-index 会话做 join」）。
//!
//! `dnd.rs` 对应 `WorkspaceGroupedTasksSection.tsx` 的拖拽**纯逻辑**部分
//! （6 个 preview 函数 + 视图签名 + 分发链）；dnd-kit 的碰撞检测与拖拽源注册
//! 属事件适配层，尚未迁移，原因见该文件模块头的边界说明。
//!
//! 渲染层由同级`grouped.rs` 负责（已迁）；本模块补的是它明确留白的部分：
//! 「未做（依赖真源交互体系）：拖拽重排（view.ts 581 行 + dnd-kit）」。
//!
//! 模块名沿用PascalCase 里的下划线风格是刻意的（对齐真源文件名 view），
//! 故豁免 snake_case 警告。
#![allow(non_snake_case)]

pub mod dnd;
pub mod join;
pub mod view;
