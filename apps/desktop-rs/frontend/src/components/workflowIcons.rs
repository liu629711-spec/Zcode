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

// ── workflow 工具小卡（save / list / get-run / submit / list-models 一族）用的图标 ──

/// `MessageCircleReply`（resolve-workflow-question.tsx:1,9）。
pub fn icon_message_circle_reply() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M2.992 16.342a2 2 0 0 1 .094 1.167l-1.065 3.29a1 1 0 0 0 1.236 1.168l3.413-.998a2 2 0 0 1 1.099.092 10 10 0 1 0-4.777-4.719",
                "m10 15-3-3 3-3",
                "M7 12h8a2 2 0 0 1 2 2v1",
            ]
            circles=vec![]
        />
    }
}

/// `Save`（save-workflow.tsx）。
pub fn icon_save() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
                "M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7",
                "M7 3v4a1 1 0 0 0 1 1h7",
            ]
            circles=vec![]
        />
    }
}

/// `Library`（list-saved-workflows.tsx）。
pub fn icon_library() -> impl IntoView {
    view! {
        <Icon
            paths=vec!["m16 6 4 14", "M12 6v14", "M8 8v12", "M4 4v16"]
            circles=vec![]
        />
    }
}

/// `History`（list-workflow-runs.tsx）。
pub fn icon_history() -> impl IntoView {
    view! {
        <Icon
            paths=vec!["M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8", "M3 3v5h5", "M12 7v5l4 2"]
            circles=vec![]
        />
    }
}

/// `Cpu`（list-models.tsx）。
///
/// 真源两个 rect（外框 rx=2、内核 rx=1），`Icon` 的 rect 统一画 rx=1；
/// 与 `icon_workflow` 同一处既有偏离，不为一个图标改公共组件。
pub fn icon_cpu() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M12 20v2",
                "M12 2v2",
                "M17 20v2",
                "M17 2v2",
                "M2 12h2",
                "M2 17h2",
                "M2 7h2",
                "M20 12h2",
                "M20 17h2",
                "M20 7h2",
                "M7 20v2",
                "M7 2v2",
            ]
            circles=vec![]
            rects=vec![("4", "4", "16", "16"), ("8", "8", "8", "8")]
        />
    }
}

/// `ClipboardCheck`（submit-result.tsx）。
pub fn icon_clipboard_check() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2",
                "m9 14 2 2 4-4",
            ]
            circles=vec![]
            rects=vec![("8", "2", "8", "4")]
        />
    }
}

/// `BookOpenText`（read-session-context.tsx）。
pub fn icon_book_open_text() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M12 7v14",
                "M16 12h2",
                "M16 8h2",
                "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",
                "M6 12h2",
                "M6 8h2",
            ]
            circles=vec![]
        />
    }
}
