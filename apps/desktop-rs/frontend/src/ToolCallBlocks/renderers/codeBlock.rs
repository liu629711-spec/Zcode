//! 1:1 翻译 `packages/ui/src/components/ai-elements/code-block.tsx`（550 行）
//! 的 **v1 简化版**。
//!
//! **裁剪注明**（对 550 行完整能力的差距表）：
//! - 语法高亮：真源用 shiki（JS 库，WASM 侧不可用）——Rust 侧输出
//!   `<pre><code class="language-{lang}">`，由 app.rs 的 hljs 后处理
//!   （`window.hljsLib` + `data-highlighted` 防重复）着色，与聊天区
//!   markdown 代码块同一机制。
//! - 未迁：Header（复制/换行切换/全屏按钮组）、行号、`focusedRange`
//!   行定位、Mermaid 渲染分支（`MermaidBlock`/`DiagramPreviewDialog`）、
//!   `CodeViewer` 集成、`enableSyntaxHighlighting`/`wrapLongLines`/
//!   `fontSizePx` 等 props——待码查看器链路整体迁移时补齐。
//! - hljs 后处理只在消息流变化时触发（app.rs Effect）；展开折叠等
//!   局部重渲的新 code 块 v1 可能未着色，待观察。

use leptos::prelude::*;

/// `CodeBlock`（真源 :43-）的 v1 等价：语言类名 + 纯文本代码。
#[component]
pub fn CodeBlock(
    /// 代码文本。
    code: String,
    /// 语言标识符（如 "json" / "rust"）。
    language: String,
) -> impl IntoView {
    let language_class = format!("language-{language}");
    view! {
        <pre class="overflow-x-auto p-3 text-ui-base leading-[1.5]">
            <code class=language_class>{code}</code>
        </pre>
    }
}
