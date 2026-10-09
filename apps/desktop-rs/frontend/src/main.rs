// 模块名沿用camelCase 对齐真源文件名（conversationCuaGroups.ts）。
#![allow(non_snake_case)]

use leptos::mount::mount_to_body;

pub mod ToolCallBlocks;
pub mod app;
pub mod archived;
pub mod components;
pub mod conversationWorkItems;
pub mod cuaGroups;
pub mod file_icons;
pub mod file_tree;
pub mod grouped;
pub mod groupedTasks;
pub mod pinned;
pub mod stream;
pub mod taskTitle;
pub mod toast;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}
