//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowMarchLight.tsx`（47 行）。
//!
//! 行进边的光：控制流进入正在运行的站所走的那条边，在 SVG 里叠一条圆头路径，用一道
//! `userSpaceOnUse` 的渐变描边——从路径起点的全透明到 80% 处的满警示色，在灯那一头最亮、
//! 朝控制流来的方向淡去。它不动：行进边说的是过去，动作属于正在运行的灯。
//!
//! 渐变沿 x 铺（路径首尾两点的 x）；首尾 x 相同的路径改沿 y 铺。`id` 由调用方给。

use leptos::prelude::*;

/// `MarchLightProps` 的 Rust 视图（真源 :11-19）。
#[derive(Debug, Clone, PartialEq)]
pub struct MarchLightPoint {
    pub x: f64,
    pub y: f64,
}

fn fmt(v: f64) -> String {
    // JS 模板串的数字直接落字符串；Rust 侧去掉多余的小数点。
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[component]
pub fn MarchLight(
    /// 要点亮的路径；弧给的是截短 6px、停在箭头根部的那条。
    d: String,
    /// 路径起点（控制流来的那一头）。
    from: MarchLightPoint,
    /// 路径终点（正在运行的灯那一头）。
    to: MarchLightPoint,
    id: String,
) -> impl IntoView {
    let along_y = from.x == to.x;
    view! {
        <>
            <defs>
                <linearGradient
                    gradientUnits="userSpaceOnUse"
                    id=id.clone()
                    x1=if along_y { "0".to_string() } else { fmt(from.x) }
                    x2=if along_y { "0".to_string() } else { fmt(to.x) }
                    y1=if along_y { fmt(from.y) } else { "0".to_string() }
                    y2=if along_y { fmt(to.y) } else { "0".to_string() }
                >
                    <stop offset="0" stop-color="var(--color-warning)" stop-opacity="0" />
                    <stop offset="0.8" stop-color="var(--color-warning)" />
                </linearGradient>
            </defs>
            <path class="wf-lit" d=d data-testid="workflow-march-light" fill="none" stroke=format!("url(#{id})") />
        </>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting_drops_trailing_zeros() {
        // SVG 路径的数字写法：整数不带小数。
        assert_eq!(fmt(120.0), "120");
        assert_eq!(fmt(0.0), "0");
        assert_eq!(fmt(12.5), "12.5");
    }
}
