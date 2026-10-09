//! 1:1 翻译 `packages/ui/src/components/ai-elements/code-block.tsx`（550 行）
//! 的 **v2 简化版**（v1 + Header/复制/换行，供 MCP / node-repl 卡消费）。
//!
//! **裁剪注明**（对 550 行完整能力的差距表）：
//! - 语法高亮：真源用 shiki（JS 库，WASM 侧不可用）——Rust 侧输出
//!   `<pre><code class="language-{lang}">`，由 app.rs 的 hljs 后处理
//!   （`window.hljsLib` + `data-highlighted` 防重复）着色，与聊天区
//!   markdown 代码块同一机制。
//! - 行号、`focusedRange` 行定位、Mermaid 渲染分支（`MermaidBlock`/
//!   `DiagramPreviewDialog`）、`CodeViewer`（@pierre/diffs）集成、
//!   `fontSizePx` / `showLineNumbers` 等 props——仍待码查看器链路整体迁移。
//! - v2 补齐：`CodeBlockContainer` 外壳类、`CodeBlockHeader` / `CodeBlockTitle` /
//!   `CodeBlockActions`、`CodeBlockWrapButton`（换行切换，aria-pressed + bg-muted）、
//!   `CodeBlockCopyButton`（clipboard 写入 + 已复制 2s 打勾）。复制走
//!   `navigator.clipboard`（web-sys Clipboard feature 已开）。
//! - hljs 后处理只在消息流变化时触发（app.rs Effect）；展开折叠等
//!   局部重渲的新 code 块 v1 可能未着色，待观察。
//!
//! ⚠ 实现注记：按钮经 `ControlHintTooltip` 的触发器闭包渲染，该闭包只求值一次
//! （ControlHintTooltip.rs `{children()}`）——按钮的动态状态（aria-pressed /
//! bg-muted / 复制打勾）必须用**响应式属性绑定**（`class=move ||`、
//! `{move || ...}` 子节点）承载，不能在闭包里 `.get()` 成静态值。

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

// ---------------------------------------------------------------------------
// v2：Container / Header / Title / Actions / WrapButton / CopyButton
// （真源 :147-243 与 :376-510；容器类 :152-165，正文包裹 :326-360）
// ---------------------------------------------------------------------------

/// Button 基础变体类（真源 `buttonVariants` 基座 + ghost + icon-md 的 tailwind-merge 结果）。
pub const CODE_BLOCK_BUTTON_CLASS: &str = "group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding font-medium whitespace-nowrap transition-colors outline-none select-none text-foreground hover:bg-hover hover:text-foreground size-7 [&_svg]:pointer-events-none [&_svg]:shrink-0";

/// `CodeBlockWrapButton` 的图标（真源 `WrapTextIcon size-3.5`，lucide wrap-text）。
fn wrap_text_icon() -> impl IntoView {
    view! {
        <crate::app::Icon
            circles=vec![]
            paths=vec![
                "M3 6h18",
                "M3 12h15a3 3 0 1 1 0 6h-4",
                "M16 16l-2-2 2-2",
            ]
        />
    }
}

/// 复制态图标（真源 `isCopied ? CheckIcon : CopyIcon`，lucide check / copy）。
fn copy_state_icon(copied: bool) -> AnyView {
    if copied {
        view! { <crate::app::Icon circles=vec![] paths=vec!["M20 6 9 17l-5-5"] /> }.into_any()
    } else {
        view! {
            <crate::app::Icon
                circles=vec![]
                paths=vec![
                    "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2",
                    "M22 18c0 1.1-.9 2-2 2H10c-1.1 0-2-.9-2-2V8c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2z",
                ]
            />
        }
        .into_any()
    }
}

/// v2 代码块（真源 `CodeBlock` + `CodeBlockHeader` 组合的 Rust 等价）。
///
/// 真源 header 由 children 槽组装；Rust 侧收拢为结构化 props——
/// `label`（标题词）、`copy_label` / `wrap_label`（按钮的无障碍名）。
/// 三者全缺席时退化为无 header 的纯代码块（真源不传 children 的形态）。
#[component]
pub fn RichCodeBlock(
    /// 代码文本。
    code: String,
    /// 语言标识符（"json" / "log" / "javascript"）。
    language: String,
    /// header 标题（真源 CodeBlockTitle 的 span 文案）。
    label: Option<String>,
    /// 复制按钮的无障碍名；缺席 = 不渲染复制钮。
    copy_label: Option<String>,
    /// 换行按钮的无障碍名；缺席 = 不渲染换行钮。
    wrap_label: Option<String>,
    /// 初始换行态（真源 `wrapLongLines`，:252 缺省 false）。
    #[prop(default = false)]
    wrap_long_lines: bool,
    /// 容器底色（真源 className：MCP/node-repl 传 "bg-card"）。
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let is_wrapped = RwSignal::new(wrap_long_lines);
    let is_copied = RwSignal::new(false);
    let language_class = format!("language-{language}");
    let code_for_copy = code.clone();
    let has_header = label.is_some() || copy_label.is_some() || wrap_label.is_some();

    view! {
        <div
            class=format!(
                "group relative w-full overflow-hidden rounded-xl bg-background text-foreground {class}",
            )
            data-language=language
        >
            {has_header.then(|| {
                let label = label.clone();
                view! {
                    <div class="flex items-center justify-between gap-3 pl-3 pr-2 py-2 text-ui-base text-muted-foreground">
                        <div class="flex items-center gap-2">
                            {label.map(|label| {
                                view! {
                                    <span class="text-ui-base font-medium text-foreground-subtle">
                                        {label}
                                    </span>
                                }
                            })}
                        </div>
                        <div class="-my-1 -mr-1 flex items-center gap-1">
                            {wrap_label.map(|wrap_label| {
                                view! {
                                    <crate::ControlHintTooltip::ControlHintTooltip
                                        children={{
                                            let wrap_label = wrap_label.clone();
                                            std::sync::Arc::new(move || {
                                                let wrap_label = wrap_label.clone();
                                                view! {
                                                    <button
                                                        aria-label=wrap_label.clone()
                                                        aria-pressed=move || is_wrapped.get()
                                                        class=move || {
                                                            format!(
                                                                "{} {}",
                                                                CODE_BLOCK_BUTTON_CLASS,
                                                                is_wrapped.get().then_some("bg-muted").unwrap_or_default(),
                                                            )
                                                        }
                                                        type="button"
                                                        on:click=move |_| is_wrapped.update(|v| *v = !*v)
                                                    >
                                                        <span class="inline-flex size-3.5 items-center justify-center [&>svg]:size-3.5">
                                                            {wrap_text_icon()}
                                                        </span>
                                                    </button>
                                                }
                                                .into_any()
                                            })
                                                as std::sync::Arc<
                                                    dyn Fn() -> AnyView + Send + Sync + 'static,
                                                >
                                        }}
                                        data=crate::ControlHintTooltip::ControlHintTooltipData {
                                            title: wrap_label.clone(),
                                            ..Default::default()
                                        }
                                    />
                                }
                            })}
                            {copy_label.map(|copy_label| {
                                view! {
                                    <crate::ControlHintTooltip::ControlHintTooltip
                                        children={{
                                            let copy_label = copy_label.clone();
                                            std::sync::Arc::new(move || {
                                                let copy_label = copy_label.clone();
                                                // 先克隆进 handler（在 Fn 闭包体内 move 被捕获的
                                                // String 会把整个闭包降级成 FnMut，Arc<dyn Fn> 转换即炸）。
                                                let code_for_click = code_for_copy.clone();
                                                view! {
                                                    <button
                                                        aria-label=copy_label.clone()
                                                        class=CODE_BLOCK_BUTTON_CLASS
                                                        type="button"
                                                        on:click=move |_| {
                                                            // 真源 :470-486 —— clipboard 写入，
                                                            // 成功后打勾 2s（真源 timeout=2000）。
                                                            if let Some(window) = web_sys::window() {
                                                                let navigator = window.navigator();
                                                                let clipboard = navigator.clipboard();
                                                                let _ = clipboard.write_text(&code_for_click);
                                                                is_copied.set(true);
                                                                set_timeout(
                                                                    move || is_copied.set(false),
                                                                    std::time::Duration::from_millis(2000),
                                                                );
                                                            }
                                                        }
                                                    >
                                                        <span class="inline-flex size-3.5 items-center justify-center [&>svg]:size-3.5">
                                                            {move || copy_state_icon(is_copied.get())}
                                                        </span>
                                                    </button>
                                                }
                                                .into_any()
                                            })
                                                as std::sync::Arc<
                                                    dyn Fn() -> AnyView + Send + Sync + 'static,
                                                >
                                        }}
                                        data=crate::ControlHintTooltip::ControlHintTooltipData {
                                            title: copy_label.clone(),
                                            ..Default::default()
                                        }
                                    />
                                }
                            })}
                        </div>
                    </div>
                }
            })}
            <div class="p-2 pt-0 pb-3">
                <pre class=move || {
                    format!(
                        "{} text-ui-base leading-[1.5]",
                        if is_wrapped.get() {
                            "whitespace-pre-wrap break-words"
                        } else {
                            "overflow-x-auto"
                        },
                    )
                }>
                    <code class=language_class>{code}</code>
                </pre>
            </div>
        </div>
    }
}
