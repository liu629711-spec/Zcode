use leptos::mount::mount_to_body;

pub mod ToolCallBlocks;
pub mod app;
pub mod archived;
pub mod file_icons;
pub mod file_tree;
pub mod grouped;
pub mod groupedTasks;
pub mod pinned;
pub mod stream;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}
