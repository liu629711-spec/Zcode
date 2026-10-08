//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/ToolCallBody.tsx`（204 行）。
//!
//! 工具卡「展开区」的公共渲染器：按 displayModel 组合
//! 预览（文本/图片）/ plan 结果 / Parameters / Result / kind 文本；
//! `childToolList` 有值时整体替换。
//!
//! **裁剪注明**：
//! - `InlineImageContent` 的 `fileService.readMediaPreview`（IPC 未迁）——
//!   维持真源首帧的「正在加载...」占位（:329-331 的 !imagePreview 分支），
//!   img 渲染待 IPC 迁入。
//! - `onOpenCodeViewer` / `onOpenFileLink` / `onOpenBrowserUrl`：
//!   Rust 侧 markdown 走 `render_markdown` 纯 HTML，无交互回调通道
//!   （各卡既有裁剪的延续）。

use leptos::prelude::*;
use serde_json::Value;

use super::renderers::codeBlock::CodeBlock;
use super::renderers::toolOutput::{ToolInputBlock, ToolOutputBlock};
use super::toolCallRowAdapter::LegacyToolCall;
use super::toolDisplay::{ToolDisplayModel, ToolInlinePreview};

/// `ToolCallBody`（真源 :28-104）。
///
/// 用 `#[component]` 的 inline 参数形式：宏会为 `ToolCallBody` 生成
/// `ToolCallBodyProps`，与手写同名 struct 冲突（ToolSnapshotFieldNotice
/// 同款坑）——inline 参数直接避开。
#[component]
pub fn ToolCallBody(
    /// `childToolList`：有值时整体替换 body（真源 :45-47 的 `??` 短路）。
    #[prop(optional)]
    child_tool_list: Option<ChildrenFn>,
    display_model: ToolDisplayModel,
    #[prop(optional)] inline_preview_override: Option<ToolInlinePreview>,
    tool_call: LegacyToolCall,
    workspace_path: String,
) -> AnyView {
    if let Some(child_tool_list) = child_tool_list {
        return child_tool_list();
    }

    let model = display_model;
    let tc = tool_call;
    let inline_preview = inline_preview_override.unwrap_or_else(|| model.inline_preview.clone());

    let preview_view: AnyView = match &inline_preview {
        ToolInlinePreview::Text(source) => {
            if source.language == "markdown" {
                let html = crate::app::render_markdown(&source.content);
                view! {
                    <div class="max-h-60 overflow-auto rounded-xl border border-border bg-muted/15 px-3 py-3">
                        <div class="min-w-0 break-words" inner_html=html></div>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="max-h-60 overflow-auto rounded-xl bg-muted/15">
                        <CodeBlock code=source.content.clone() language=source.language.clone() />
                    </div>
                }
                .into_any()
            }
        }
        ToolInlinePreview::Image(source) => {
            // 真源 :307-331 —— fileService.readMediaPreview（IPC 未迁）；
            // 维持首帧「正在加载...」占位。
            let _ = source;
            view! { <p>"正在加载..."</p> }.into_any()
        }
        _ => ().into_any(),
    };

    let plan_view: AnyView = match &model.plan_result {
        Some(plan) => {
            let html = crate::app::render_markdown(&plan.plan);
            let plan_file_path = plan.plan_file_path.clone();
            view! {
                <div class="space-y-3 rounded-xl border border-outline/60 bg-muted/15 p-3">
                    {plan_file_path.map(|path| view! {
                        <div class="flex flex-wrap items-center gap-2 text-ui-base text-on-surface-muted">
                            <span>"计划文件"</span>
                            <span class="cursor-default rounded-md bg-muted/70 px-2 py-1 font-mono text-on-surface">
                                {path}
                            </span>
                        </div>
                    })}
                    <div
                        class="size-full min-w-0 break-words whitespace-normal [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
                        inner_html=html
                    ></div>
                </div>
            }
            .into_any()
        }
        None => ().into_any(),
    };

    let input_view: AnyView = if model.show_input && tc.input.is_some() {
        view! {
            <ToolInputBlock input=tc.input.clone().unwrap_or(Value::Null) />
        }
        .into_any()
    } else {
        ().into_any()
    };

    let output_view: AnyView = if model.show_output {
        view! {
            <ToolOutputBlock
                error_text=tc.error.clone()
                output=tc.output.clone().map(Value::String)
            />
        }
        .into_any()
    } else {
        ().into_any()
    };

    let kind_view: AnyView = if model.show_kind {
        let kind = tc.kind.clone();
        view! { <p class="text-ui-base text-muted-foreground">{kind}</p> }.into_any()
    } else {
        ().into_any()
    };

    let _ = workspace_path; // markdown 渲染 v1 不带 workspace 上下文的回调通道。

    view! { {preview_view} {plan_view} {input_view} {output_view} {kind_view} }.into_any()
}
