//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/nodeReplImageGrid.tsx`（77 行）
//! + 依赖的 `ai-elements/image-thumbnail-gallery.tsx`（31 行，纯类名常量）。
//!
//! 结果图片网格：缩略图按序排布，两张以上走分组网格（小屏单列/两列，桌面流式）。
//!
//! **裁剪注明**：点击放大的 lightbox（真源 `ImagePreviewDialog`，Radix Dialog +
//! focus trap + 关闭后焦点回位）未迁——与 CuaScreenshotSection 的点击放大同一条
//! 依赖链（Rust 无 Radix 等价物）。缩略图按钮保留（可聚焦、有 aria 名），
//! 点击暂为 no-op；Portal 弹层待 Dialog 子系统迁移后补。

use leptos::prelude::*;

use crate::lib::nodeReplToolDisplay::NodeReplDisplayImage;
use crate::ToolCallBlocks::{i18n};

/// `imageThumbnailTriggerClassName`（image-thumbnail-gallery.tsx :6-7）。
pub const IMAGE_THUMBNAIL_TRIGGER_CLASS: &str = "my-4 block w-fit max-w-1/2 cursor-zoom-in overflow-hidden rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

/// `imageThumbnailClassName`（image-thumbnail-gallery.tsx :9-10）。
pub const IMAGE_THUMBNAIL_CLASS: &str =
    "h-auto max-h-90 max-w-full rounded-xl border border-border bg-background object-cover";

/// 分组网格类（image-thumbnail-gallery.tsx :22 —— grouped=true 分支）。
pub const IMAGE_THUMBNAIL_GALLERY_GROUPED_CLASS: &str = "my-4 grid grid-cols-1 gap-2 [&>[data-image-thumbnail-trigger]]:my-0 [&>[data-image-thumbnail-trigger]]:h-32 [&>[data-image-thumbnail-trigger]]:w-full [&>[data-image-thumbnail-trigger]]:max-w-none [&>[data-image-thumbnail-trigger]>img]:size-full [&>[data-image-thumbnail-trigger]>img]:max-h-none sm:grid-cols-2 md:flex md:flex-wrap md:[&>[data-image-thumbnail-trigger]]:h-44 md:[&>[data-image-thumbnail-trigger]]:w-auto md:[&>[data-image-thumbnail-trigger]]:shrink-0 md:[&>[data-image-thumbnail-trigger]>img]:h-44 md:[&>[data-image-thumbnail-trigger]>img]:w-auto md:[&>[data-image-thumbnail-trigger]>img]:max-w-none";

/// 预览项（真源 `previewItems`，:22-30 的映射产物）。
pub struct ImagePreviewItem {
    pub alt: String,
    pub filename: String,
    /// `data:{mime};base64,{data}` 的 data URI。
    pub src: String,
}

/// `previewItems` 映射（真源 :22-30 的纯函数拆出，供测试与 lightbox 复用）。
pub fn build_preview_items(
    images: &[NodeReplDisplayImage],
    result_image_label: &str,
) -> Vec<ImagePreviewItem> {
    images
        .iter()
        .enumerate()
        .map(|(index, image)| ImagePreviewItem {
            alt: format!("{result_image_label} {}", index + 1),
            filename: format!("result-image-{}", index + 1),
            src: format!("data:{};base64,{}", image.mime_type, image.base64),
        })
        .collect()
}

/// `NodeReplImageGrid`（真源 :11-80）。
#[component]
pub fn NodeReplImageGrid(
    images: Vec<NodeReplDisplayImage>,
    result_image_label: String,
) -> impl IntoView {
    let preview_items = build_preview_items(&images, &result_image_label);
    let grouped = preview_items.len() >= 2;
    let open_image_label = i18n::text("chat.attachments.preview.open");

    view! {
        <div
            class=if grouped {
                IMAGE_THUMBNAIL_GALLERY_GROUPED_CLASS.to_string()
            } else {
                "contents".to_string()
            }
            data-node-repl-image-gallery=""
        >
            {preview_items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    // 真源 :54 的 key：`{mime}:{base64.length}:{index}`。
                    let key = format!(
                        "{}:{}:{}",
                        images.get(index).map(|i| i.mime_type.as_str()).unwrap_or_default(),
                        item.src.len(),
                        index,
                    );
                    let aria_label = format!("{open_image_label} {}", index + 1);
                    view! {
                        <button
                            aria-label=aria_label
                            class=IMAGE_THUMBNAIL_TRIGGER_CLASS
                            data-image-thumbnail-trigger=""
                            data-key=key
                            type="button"
                        >
                            <img
                                alt=item.alt.clone()
                                class=IMAGE_THUMBNAIL_CLASS
                                draggable="false"
                                loading="lazy"
                                src=item.src.clone()
                            />
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_items_are_numbered_and_data_uris() {
        // 真源 :22-30 —— alt/filename 从 1 起编；src 是 data URI。
        let images = vec![
            NodeReplDisplayImage {
                base64: "AAAA".into(),
                mime_type: "image/png".into(),
            },
            NodeReplDisplayImage {
                base64: "BBBB".into(),
                mime_type: "image/jpeg".into(),
            },
        ];
        let items = build_preview_items(&images, "结果图片");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].alt, "结果图片 1");
        assert_eq!(items[0].filename, "result-image-1");
        assert_eq!(items[0].src, "data:image/png;base64,AAAA");
        assert_eq!(items[1].alt, "结果图片 2");
        assert_eq!(items[1].src, "data:image/jpeg;base64,BBBB");
    }

    #[test]
    fn gallery_groups_only_two_or_more() {
        // 真源 :44 —— grouped={previewItems.length >= 2}。
        let one = vec![NodeReplDisplayImage {
            base64: "A".into(),
            mime_type: "image/png".into(),
        }];
        assert!(!build_preview_items(&one, "图").len() >= 2);
        assert_eq!(build_preview_items(&one, "图").len(), 1);
        let two = vec![one[0].clone(), one[0].clone()];
        assert!(build_preview_items(&two, "图").len() >= 2);
    }
}
