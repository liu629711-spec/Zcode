//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/agentPromptSection.tsx`（51 行）。
//!
//! Agent 展开区的「提示词」区块：标题（uppercase 小标）+ markdown 正文
//! （max-h-50 滚动）。
//!
//! **裁剪注明**：真源正文走 `MessageResponse`（streamdown markdown 渲染器，
//! 含代码查看/文件链接回调）；Rust 侧用 `app::render_markdown`（pulldown-cmark）
//! 输出 HTML——结构等价，交互回调（onOpenCodeViewer 等）未迁不挂。

use leptos::prelude::*;

/// `AgentPromptSection`（真源 :7-51）。
#[component]
pub fn AgentPromptSection(prompt: String) -> impl IntoView {
    let label = super::agentHelpers::agent_message("chat.toolCall.agent.prompt", "Prompt");
    let html = crate::app::render_markdown(&prompt);
    view! {
        <section class="space-y-2">
            <div class="flex flex-col rounded-lg border border-border">
                <div class="flex min-w-0 items-center p-3">
                    <h4 class="min-w-0 text-ui-base font-medium tracking-wide text-foreground-subtlest uppercase">
                        {label}
                    </h4>
                </div>
                <div
                    class="max-h-50 overflow-auto px-3 py-2 text-ui-base break-words"
                    data-markdown-table-sticky-scrollbar="disabled"
                    inner_html=html
                ></div>
            </div>
        </section>
    }
}
