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
/// 假数据渲染画廊：只在 `?fixture` 入口挂载，产品路径不经过。
pub mod fixtureGallery;
pub mod grouped;
pub mod groupedTasks;
// 真源目录就叫 lib/（packages/ui/src/lib）；special_module_name 的豁免写在 lib/mod.rs 内。
pub mod lib;
pub mod pinned;
pub mod stream;
pub mod taskTitle;
pub mod toast;
pub mod workspace_file_tree;

fn main() {
    console_error_panic_hook::set_once();
    // `?fixture` 挂假数据画廊：1400+ 单测全是纯函数断言，没有一个能渲染，
    // 而正常入口没有 Tauri 后端就画不出工具卡，改动因此没有验证面。
    // 产品路径仍然走 app::App，不受这个开关影响。
    let query = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    if query.contains("fixture") {
        mount_to_body(fixtureGallery::FixtureGallery);
    } else {
        mount_to_body(app::App);
    }
}
