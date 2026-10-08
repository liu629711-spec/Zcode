use leptos::mount::mount_to_body;

pub mod app;
pub mod file_icons;
pub mod file_tree;
pub mod pinned;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}