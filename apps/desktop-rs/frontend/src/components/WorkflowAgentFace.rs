//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowAgentFace.tsx`（125 行）。
//!
//! 瓦片脸：应用图标的圆角方块去掉 Z、加上两只眼。子代理的头像就是它——药丸、
//! 名册格、侧栏折叠头像串共用这一张脸。
//!
//! 身份是机身色：九个固定 HEX 颜色按 `avatarIndex` 取，第十个起循环；没有编号退回
//! 名字散列。状态定义基础表情，独立随机动作仅更新 SVG 属性（`workflow_face_motion`）。

use leptos::prelude::*;

use crate::components::workflow_face_motion::{base_expression, face_state, start_face_motion};
use crate::components::workflow_graph::types::StepRunStatus;

/// `FACE_COLORS`（真源 :18-28）：九个固定 HEX 颜色。
pub const FACE_COLORS: [&str; 9] = [
    "#54B9A6", "#F19D38", "#6464EF", "#885CF5", "#3C82F6", "#ED712E", "#EB4699", "#5BC67A",
    "#EA4045",
];

/// `avatarColor`（真源 :31-35）：名字散列选色（31 进制取模 360 后映射到九色板）；
/// 只在没有 `avatarIndex` 时兜底。
pub fn avatar_color(name: &str) -> &'static str {
    let mut hash: u64 = 0;
    for char in name.chars() {
        hash = (hash.wrapping_mul(31).wrapping_add(char as u64)) % 360;
    }
    FACE_COLORS[(hash % FACE_COLORS.len() as u64) as usize]
}

/// `agentColor`（真源 :38-43）：代理的颜色。编号优先（九色环循环），缺席时退回名字散列。
/// 药丸悬停描边与脸共用它。
pub fn agent_color(avatar_index: Option<usize>, name: &str) -> &'static str {
    let Some(index) = avatar_index else {
        return avatar_color(name);
    };
    let n = FACE_COLORS.len() as i64;
    FACE_COLORS[(((index as i64 % n) + n) % n) as usize]
}

/// 眼形路径（真源 :67-76）：共享左右起点（x=7 / x=13），换脸时保持底部和视线位置。
fn eye_path(expression: &str, i: usize) -> &'static str {
    match expression {
        "happy" => "M0 11 V8 A2 2 0 0 1 4 8 V11 Z",
        "sleepy" => {
            if i == 0 {
                "M0 8 L4 7 V9 A2 2 0 0 1 0 9 Z"
            } else {
                "M0 7 L4 8 V9 A2 2 0 0 1 0 9 Z"
            }
        }
        "focused" => {
            if i == 0 {
                "M0 6 L4 8 V10 A2 2 0 0 1 0 10 Z"
            } else {
                "M0 8 L4 6 V10 A2 2 0 0 1 0 10 Z"
            }
        }
        // sad
        _ => {
            if i == 0 {
                "M0 8 L4 6 V10 A2 2 0 0 1 0 10 Z"
            } else {
                "M0 6 L4 8 V10 A2 2 0 0 1 0 10 Z"
            }
        }
    }
}

/// 眼睛（真源 `Eyes`，:48-90）：直接在 20 格内绘制参考眼型。
/// 闭眼单独画横胶囊，避免纵向缩放把端部圆角压成细线。
fn eyes() -> impl IntoView {
    const EXPRESSIONS: [&str; 6] = ["pill", "happy", "sleepy", "focused", "sad", "confused"];
    let dots: Vec<f64> = vec![5.0, 10.0, 15.0];
    let closed: Vec<(f64, f64, f64, f64)> = vec![(7.0, 8.0, 4.0, 2.0), (13.0, 8.0, 4.0, 2.0)];
    view! {
        <g class="wf-face-eyes">
            <g class="wf-face-bounce">
                <g class="wf-face-lids">
                    <g data-eye-expression="dots" fill="var(--wf-face-eye)">
                        {dots.into_iter().map(|cx| view! { <circle cx=cx cy=10.0 r=1.5 /> }).collect_view()}
                    </g>
                    {EXPRESSIONS
                        .iter()
                        .map(|expression| {
                            let expr = *expression;
                            let is_rect = expr == "pill" || expr == "confused";
                            let y = if expr == "confused" { 8.0 } else { 6.0 };
                            let height = if expr == "confused" { 4.0 } else { 6.0 };
                            view! {
                                <g data-eye-expression=expr fill="var(--wf-face-eye)">
                                    {if is_rect {
                                        vec![7.0, 13.0]
                                            .into_iter()
                                            .map(|x| {
                                                view! { <rect x=x y=y width=4.0 height=height rx=2.0 /> }
                                            })
                                            .collect_view()
                                            .into_any()
                                    } else {
                                        vec![(7.0, eye_path(expr, 0)), (13.0, eye_path(expr, 1))]
                                            .into_iter()
                                            .map(|(x, d)| {
                                                view! {
                                                    <g transform=format!("translate({x} 0)")><path d=d /></g>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }}
                                </g>
                            }
                        })
                        .collect_view()}
                </g>
                <g class="wf-face-closed" fill="var(--wf-face-eye)">
                    {closed
                        .into_iter()
                        .map(|(x, y, w, h)| view! { <rect x=x y=y width=w height=h rx=1.0 /> })
                        .collect_view()}
                </g>
            </g>
        </g>
    }
}

/// `WorkflowAgentFace`（真源 :92-125）。
#[component]
pub fn WorkflowAgentFace(
    avatar_index: Option<usize>,
    #[prop(optional, into)] class: String,
    name: String,
    status: Option<StepRunStatus>,
) -> impl IntoView {
    let state = face_state(status);
    // 动作引擎挂在 SVG 节点上（真源 :103-107 的 ref + useEffect）；换状态即重启。
    // Leptos 的 NodeRef 对 <svg> 需要 svg 命名空间的元素类型，这里把 ref 挂在包装
    // span 上、用 query_selector 取到 SVG 子节点——与真源 ref 指向 SVG 等价。
    let face_ref: NodeRef<leptos::html::Span> = NodeRef::new();
    let state_stored = StoredValue::new(state);
    Effect::new(move |_| {
        let target_state = state_stored.get_value();
        let Some(span) = face_ref.get() else {
            return;
        };
        let Some(svg) = span
            .query_selector("svg")
            .ok()
            .flatten()
        else {
            return;
        };
        let cleanup = start_face_motion(&svg, target_state);
        let cleanup = send_wrapper::SendWrapper::new(cleanup);
        on_cleanup(move || cleanup.take()());
    });
    let body_color = agent_color(avatar_index, &name);
    view! {
        <span node_ref=face_ref class="contents">
            <svg
                aria-hidden="true"
                class=format!("wf-face overflow-visible {class}")
                data-face-state=state.as_str()
                data-expression=base_expression(state).as_str()
                data-motion="idle"
                data-subagent-avatar=""
                style=format!("--wf-face-body: {body_color}")
                viewBox="0 0 20 20"
            >
                <rect class="wf-face-body" fill="var(--wf-face-body)" height="20" rx="7" width="20" />
                {eyes()}
            </svg>
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_colors_are_nine_fixed_hex_values() {
        // 真源 :18-28 —— 九色按序。
        assert_eq!(FACE_COLORS.len(), 9);
        assert_eq!(FACE_COLORS[0], "#54B9A6");
        assert_eq!(FACE_COLORS[8], "#EA4045");
    }

    #[test]
    fn avatar_color_hashes_name_into_palette() {
        // 真源 :31-35 —— 31 进制取模 360 再落到九色板；同一名字稳定。
        let a = avatar_color("子代理 A");
        assert!(FACE_COLORS.contains(&a));
        assert_eq!(a, avatar_color("子代理 A"));
    }

    #[test]
    fn agent_color_prefers_index_and_wraps() {
        // 真源 :38-43 —— 编号优先、第十个起循环。
        assert_eq!(agent_color(Some(0), "x"), FACE_COLORS[0]);
        assert_eq!(agent_color(Some(9), "x"), FACE_COLORS[0]);
        assert_eq!(agent_color(Some(10), "x"), FACE_COLORS[1]);
        assert_eq!(agent_color(None, "x"), avatar_color("x"));
    }

    #[test]
    fn empty_name_hashes_to_a_stable_color() {
        // 空名散列稳定且落在九色板内。
        assert_eq!(avatar_color(""), avatar_color(""));
        assert!(FACE_COLORS.contains(&avatar_color("")));
    }

    #[test]
    fn eye_paths_are_distinct_per_side_for_asymmetric_shapes() {
        // sleepy / focused / sad 的两只眼形状不同（镜像）。
        assert_ne!(eye_path("sleepy", 0), eye_path("sleepy", 1));
        assert_ne!(eye_path("focused", 0), eye_path("focused", 1));
        assert_ne!(eye_path("sad", 0), eye_path("sad", 1));
        // happy 两只眼同形。
        assert_eq!(eye_path("happy", 0), eye_path("happy", 1));
    }
}
