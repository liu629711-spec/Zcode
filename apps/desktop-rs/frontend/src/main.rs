// 模块名沿用camelCase 对齐真源文件名（conversationCuaGroups.ts）。
#![allow(non_snake_case)]
// 真源目录 packages/ui/src/lib 对应 src/lib；lib 与 crate 库目标同名是刻意的 1:1 对齐。
#![allow(special_module_name)]

use leptos::mount::mount_to_body;

pub mod ControlHintTooltip;
pub mod ToolCallBlocks;
pub mod app;
pub mod app_shell;
pub mod archived;
pub mod components;
pub mod conversationWorkItems;
pub mod cuaGroups;
pub mod file_icons;
pub mod file_tree;
pub mod grouped;
pub mod groupedTasks;
// 真源目录就叫 lib/（packages/ui/src/lib）；special_module_name 的豁免写在 lib/mod.rs 内。
pub mod lib;
pub mod pinned;
pub mod stream;
pub mod taskTitle;
pub mod toast;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}
