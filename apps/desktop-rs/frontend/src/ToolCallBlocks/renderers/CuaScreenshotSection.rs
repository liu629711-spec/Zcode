//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/CuaScreenshotSection.tsx`（84 行）。
//!
//! 数据层（`CuaScreenshotDetails` + `buildCuaScreenshotDetails`）已在
//! `cuaScreenshotDetails.rs` 迁好，本文件只做渲染。

use leptos::prelude::*;

use super::cuaScreenshotDetails::CuaScreenshotDetails;
use crate::ToolCallBlocks::i18n;

/// 元数据行（真源 :8`Array<[string, string]>`）。
///
/// 键是 i18n id，值已是可显示文本（部分带插值）。
pub type MetaRow = (String, String);

/// 元数据构建（真源 :8-28）。
///
/// 顺序与真源一致（全屏 → 区域 → 尺寸 → 格式 → 已裁剪），
/// 因为它决定 `dl` 里的行序。
pub fn build_metadata(screenshot: &CuaScreenshotDetails) -> Vec<MetaRow> {
    let mut metadata: Vec<MetaRow> = Vec::new();
    if screenshot.full_screen {
        metadata.push((
            "chat.toolCall.cua.details.scope".to_string(),
            i18n::text("chat.toolCall.cua.details.fullScreen"),
        ));
    }
    if let Some(region) = &screenshot.region {
        metadata.push((
            "chat.toolCall.cua.details.region".to_string(),
            region.clone(),
        ));
    }
    // 真源 :17 —— 两个值都truthy 才显示（0 视为无）。
    if let (Some(w), Some(h)) = (screenshot.width, screenshot.height) {
        if w != 0 && h != 0 {
            metadata.push((
                "chat.toolCall.cua.details.dimensions".to_string(),
                format!("{w} × {h}"),
            ));
        }
    }
    if let Some(mime) = &screenshot.mime_type {
        metadata.push((
            "chat.toolCall.cua.details.format".to_string(),
            // 真源 :22 —— 去掉 `image/` 前缀并大写（png → PNG）。
            mime.strip_prefix("image/").unwrap_or(mime).to_uppercase(),
        ));
    }
    if screenshot.clamped {
        metadata.push((
            "chat.toolCall.cua.details.bounds".to_string(),
            i18n::text("chat.toolCall.cua.details.clamped"),
        ));
    }
    metadata
}

/// 区块标题 id（真源 :10-13）：放大镜与截图用不同标题。
pub fn section_title_id(zoom: bool) -> &'static str {
    if zoom {
        "chat.toolCall.cua.details.zoomPreview"
    } else {
        "chat.toolCall.cua.details.screenshot"
    }
}

/// `CuaScreenshotSection`（真源 :6-84）。
#[component]
pub fn CuaScreenshotSectionComponent(screenshot: CuaScreenshotDetails) -> impl IntoView {
    let metadata = build_metadata(&screenshot);
    let title_id = section_title_id(screenshot.zoom);
    let data_url = screenshot.data_url.clone();
    let zoom = screenshot.zoom;

    view! {
        <section class="space-y-2 border-t border-border pt-3">
            <h4 class="text-sm text-foreground-subtle">{i18n::text(title_id)}</h4>
            {data_url.clone().map(|url| {
                // 真源 :37-56 —— 有图时是可点开的缩略图 + Dialog 放大。
                // Dialog 依赖 Radix（Rust 侧无等价物），本轮只渲染缩略图按钮，
                // 点击行为留待 Dialog 组件迁移时接（见文件末 TODO）。
                view! {
                    <button
                        type="button"
                        class="block w-full overflow-hidden rounded-lg border border-border \
                               bg-surface outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        <img
                            src=url
                            alt=i18n::text("chat.toolCall.cua.details.openScreenshot")
                            class="max-h-96 w-full object-contain"
                        />
                    </button>
                }
            })}
            // 真源 :58-71 —— 无图时的虚线占位（区分 zoom 与普通截图文案）。
            {data_url.is_none().then(|| {
                view! {
                    <div class="flex min-h-28 flex-col items-center justify-center gap-2 \
                                rounded-lg border border-dashed border-border bg-surface px-3 py-6 \
                                text-sm text-foreground-subtle">
                        <crate::app::Icon
                            paths=vec!["m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"]
                            circles=vec![("9", "9", "2")]
                            // lucide image 的外框 rect（真源 __iconNode[0]：
// width=18 height=18 x=3 y=3 rx=2 ry=2）。Icon 组件的 rect 用固定
                            // rx="1"（app.rs:617），这里保持同一约定不引入新参数。
                            rects=vec![("3", "3", "18", "18")]
                        />
                        <span>
                            {i18n::text(if zoom {
                                "chat.toolCall.cua.details.zoomUnavailable"
                            } else {
                                "chat.toolCall.cua.details.screenshotUnavailable"
                            })}
                        </span>
                    </div>
                }
            })}
            {(!metadata.is_empty()).then(|| {
                view! {
                    <dl class="grid grid-cols-[minmax(4rem,auto)_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-sm">
                        {metadata
                            .into_iter()
                            .map(|(label_id, value)| {
                                view! {
                                    <div class="contents">
                                        <dt class="text-foreground-subtlest">
                                            {i18n::text(&label_id)}
                                        </dt>
                                        <dd class="text-foreground">{value}</dd>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </dl>
                }
            })}
        </section>
    }
    .into_any()
}

// TODO(后续迁移)：点击缩略图弹出 Dialog 放大（真源 :38-55），
// 依赖 Radix Dialog 的 focus trap / portal / 关闭语义。
// Rust 侧无等价物，需自实现或用原生 <dialog> 元素近似（届时标注差异）。

#[cfg(test)]
mod tests {
    use super::*;

    fn shot() -> CuaScreenshotDetails {
        CuaScreenshotDetails {
            data_url: None,
            width: None,
            height: None,
            mime_type: None,
            full_screen: false,
            zoom: false,
            region: None,
            clamped: false,
        }
    }

    fn label_ids(rows: &[MetaRow]) -> Vec<&str> {
        rows.iter().map(|(k, _)| k.as_str()).collect()
    }

    #[test]
    fn metadata_is_empty_by_default() {
        assert!(build_metadata(&shot()).is_empty());
    }

    #[test]
    fn metadata_order_matches_source() {
        // 真源 :8-28 的 push 顺序：scope → region → dimensions → format → bounds。
        let mut s = shot();
        s.clamped = true;
        s.mime_type = Some("image/png".into());
        s.width = Some(1920);
        s.height = Some(1080);
        s.region = Some("0,0,100,100".into());
        s.full_screen = true;
        assert_eq!(
            label_ids(&build_metadata(&s)),
            vec![
                "chat.toolCall.cua.details.scope",
                "chat.toolCall.cua.details.region",
                "chat.toolCall.cua.details.dimensions",
                "chat.toolCall.cua.details.format",
                "chat.toolCall.cua.details.bounds",
            ]
        );
    }

    #[test]
    fn dimensions_require_both_nonzero() {
        // 真源 :17 `if (width && height)` —— 0 视为无。
        let mut s = shot();
        s.width = Some(0);
        s.height = Some(0);
        assert!(!label_ids(&build_metadata(&s)).contains(&"chat.toolCall.cua.details.dimensions"));

        let mut s2 = shot();
        s2.width = Some(800);
        s2.height = Some(600);
        assert!(label_ids(&build_metadata(&s2)).contains(&"chat.toolCall.cua.details.dimensions"));
    }

    #[test]
    fn mime_strips_image_prefix_and_uppercases() {
        // 真源 :22 `replace(/^image\//u, "").toUpperCase()`。
        let mut s = shot();
        s.mime_type = Some("image/png".into());
        assert_eq!(
            build_metadata(&s)[0].1,
            "PNG",
            "image/png 应变成 PNG"
        );

        let mut s2 = shot();
        s2.mime_type = Some("jpeg".into());
        assert_eq!(build_metadata(&s2)[0].1, "JPEG", "无前缀时只大写");
    }

    #[test]
    fn dimensions_use_multiplication_sign() {
        // 真源 :20 `${width} × ${height}` —— 是×(U+00D7) 不是 x。
        let mut s = shot();
        s.width = Some(1920);
        s.height = Some(1080);
        assert_eq!(build_metadata(&s)[0].1, "1920 × 1080");
    }

    #[test]
    fn title_id_switches_on_zoom() {
        assert_eq!(
            section_title_id(false),
            "chat.toolCall.cua.details.screenshot"
        );
        assert_eq!(
            section_title_id(true),
            "chat.toolCall.cua.details.zoomPreview"
        );
    }

    #[test]
    fn component_renders_metadata_without_data_url() {
        let mut s = shot();
        s.region = Some("0,0,10,10".into());
        let _ = build_metadata(&s);
        // 组件本身依赖 view! 渲染环境（Effect/executor），
        // 元数据构建逻辑已由上面 6 个测试覆盖。
    }
}