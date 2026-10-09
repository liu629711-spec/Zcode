//! workflow 渲染层共用的 lucide 图标（真源经 `lucide-react` 引入）。
//!
//! 这不是某个真源文件的 1:1 翻译——真源的图标来自共享包 `lucide-react`
//! （一个图标一个文件），Rust 侧对应为「一个共享图标模块」；路径数据逐字取自
//! `node_modules/lucide-react/dist/esm/icons/*.mjs` 的 `__iconNode`，与真源同源。

use crate::app::Icon;
use leptos::prelude::*;

pub fn icon_workflow() -> impl IntoView {
    view! {
        <Icon paths=vec!["M7 11v4a2 2 0 0 0 2 2h4"] circles=vec![]
            rects=vec![("3", "3", "8", "8"), ("13", "13", "8", "8")] />
    }
}

pub fn icon_repeat2() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "m2 9 3-3 3 3",
                "M13 18H7a2 2 0 0 1-2-2V6",
                "m22 15-3 3-3-3",
                "M11 6h6a2 2 0 0 1 2 2v10",
            ]
            circles=vec![]
        />
    }
}

pub fn icon_arrow_up_right() -> impl IntoView {
    view! { <Icon paths=vec!["M7 7h10v10", "M7 17 17 7"] circles=vec![] /> }
}

pub fn icon_chevron_down() -> impl IntoView {
    view! { <Icon paths=vec!["m6 9 6 6 6-6"] circles=vec![] /> }
}

pub fn icon_circle_x() -> impl IntoView {
    view! {
        <Icon paths=vec!["m15 9-6 6", "m9 9 6 6"] circles=vec![("12", "12", "10")] />
    }
}

pub fn icon_circle_check() -> impl IntoView {
    view! {
        <Icon paths=vec!["m9 12 2 2 4-4"] circles=vec![("12", "12", "10")] />
    }
}

pub fn icon_loader_circle() -> impl IntoView {
    view! { <Icon paths=vec!["M21 12a9 9 0 1 1-6.219-8.56"] circles=vec![] /> }
}

pub fn icon_terminal() -> impl IntoView {
    view! { <Icon paths=vec!["M12 19h8", "m4 17 6-6-6-6"] circles=vec![] /> }
}

pub fn icon_circle_help() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
                "M12 17h.01",
            ]
            circles=vec![("12", "12", "10")]
        />
    }
}

pub fn icon_message_circle_question() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M2.992 16.342a2 2 0 0 1 .094 1.167l-1.065 3.29a1 1 0 0 0 1.236 1.168l3.413-.998a2 2 0 0 1 1.099.092 10 10 0 1 0-4.777-4.719",
                "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
                "M12 17h.01",
            ]
            circles=vec![]
        />
    }
}

pub fn icon_rotate_ccw() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8",
                "M3 3v5h5",
            ]
            circles=vec![]
        />
    }
}

pub fn icon_sliders_horizontal() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M10 5H3",
                "M12 19H3",
                "M14 3v4",
                "M16 17v4",
                "M21 12h-9",
                "M21 19h-5",
                "M21 5h-7",
                "M8 10v4",
                "M8 12H3",
            ]
            circles=vec![]
        />
    }
}

pub fn icon_square_filled() -> impl IntoView {
    view! { <Icon paths=vec![] circles=vec![] rects=vec![("3", "3", "18", "18")] /> }
}

pub fn icon_maximize2() -> impl IntoView {
    view! {
        <Icon
            paths=vec!["M15 3h6v6", "m21 3-7 7", "m3 21 7-7", "M9 21H3v-6"]
            circles=vec![]
        />
    }
}

pub fn icon_chevron_right() -> impl IntoView {
    view! { <Icon paths=vec!["m9 18 6-6-6-6"] circles=vec![] /> }
}

pub fn icon_list() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M3 5h.01",
                "M3 12h.01",
                "M3 19h.01",
                "M8 5h13",
                "M8 12h13",
                "M8 19h13",
            ]
            circles=vec![]
        />
    }
}

pub fn icon_file() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z",
                "M14 2v5a1 1 0 0 0 1 1h5",
            ]
            circles=vec![]
        />
    }
}

pub fn icon_chart_line() -> impl IntoView {
    view! {
        <Icon paths=vec!["M3 3v16a2 2 0 0 0 2 2h16", "m19 9-5 5-4-4-3 3"] circles=vec![] />
    }
}

pub fn icon_gauge() -> impl IntoView {
    view! {
        <Icon paths=vec!["m12 14 4-4", "M3.34 19a10 10 0 1 1 17.32 0"] circles=vec![] />
    }
}

pub fn icon_square_kanban() -> impl IntoView {
    view! {
        <Icon
            paths=vec!["M8 7v7", "M12 7v4", "M16 7v9"]
            circles=vec![]
            rects=vec![("3", "3", "18", "18")]
        />
    }
}

pub fn icon_table() -> impl IntoView {
    view! {
        <Icon
            paths=vec!["M12 3v18", "M3 9h18", "M3 15h18"]
            circles=vec![]
            rects=vec![("3", "3", "18", "18")]
        />
    }
}

pub fn icon_ellipsis() -> impl IntoView {
    view! {
        <Icon paths=vec![] circles=vec![("12", "12", "1"), ("19", "12", "1"), ("5", "12", "1")] />
    }
}

pub fn icon_minus() -> impl IntoView {
    view! { <Icon paths=vec!["M5 12h14"] circles=vec![] /> }
}

pub fn icon_plus() -> impl IntoView {
    view! { <Icon paths=vec!["M5 12h14", "M12 5v14"] circles=vec![] /> }
}
