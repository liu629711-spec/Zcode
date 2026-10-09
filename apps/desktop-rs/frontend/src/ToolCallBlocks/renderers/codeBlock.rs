//! 1:1 翻译 `packages/ui/src/components/ai-elements/code-block.tsx`（550 行）
//! 的 **v2 简化版**（v1 + Header/复制/换行，供 MCP / node-repl 卡消费）。
//!
//! **裁剪注明**（对 550 行完整能力的差距表）：
//! - 语法高亮：真源用 shiki（JS 库，WASM 侧不可用）——Rust 侧输出
//!   `<pre><code class="language-{lang}">`，由 app.rs 的 hljs 后处理
//!   （`window.hljsLib` + `data-highlighted` 防重复）着色，与聊天区
//!   markdown 代码块同一机制。
//! - ~~行号、`markedLines`~~ 已在 **v3** 补上（普通 flex 行号列复现可观察结果，
//!   不是 @pierre/diffs 那套 shadow DOM，见文件内 v3 段注记）。
//!   仍未迁：`focusedRange` 行定位与滚动、gutter 点选/拖拽选区、行内评论、
//!   Mermaid 渲染分支（`MermaidBlock`/`DiagramPreviewDialog`）、
//!   `fontSizePx` 等 props——仍待码查看器链路整体迁移。
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

// ── v3 追加（create-workflow 的脚本块要的两件）：行号列 + markedLines 着色判定 ──
//
// ★做法偏离已注明在 props 上：真源这两件由 `@pierre/diffs` 在异步 Shadow DOM 里画
// （code-viewer.tsx:12, :146-158, :694），整条码查看器链路（行定位 focusedRange、
// gutter 点选/拖拽选区、行内评论、字体像素与主题 token）不在这一步。
// 这里复现的是**可观察结果**：一列右对齐行号 + 被点名行号着 warning 色、代码行不加背景。
// 未覆盖的仍是未覆盖：行定位滚动与选区评论没有，等码查看器那批。
//

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

/// `codeViewerMarkedLinesCss` 的判据部分（真源 code-viewer.tsx:153-158）：
/// 去重、丢掉非整数与非正数；空集合即「没有要标的行」。
///
/// 真源把结果拼成选择器注入 shadow root；Rust 侧的行号是自己渲染的，
/// 所以这里只保留**哪些行算被点名**这一半判定。
pub fn normalize_marked_lines(lines: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::with_capacity(lines.len());
    for line in lines {
        if *line > 0 && !out.contains(line) {
            out.push(*line);
        }
    }
    out
}

/// 行号列的行数（真源交给库，这里自己数）：按 `\n` 切，与 JS `code.split("\n").length` 同形。
pub fn code_line_count(code: &str) -> usize {
    code.split('\n').count()
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
    /// 真源 `showLineNumbers`（:249，缺省 false）：左侧行号列。
    ///
    /// ★实现偏离注明（不是少做，是换做法）：真源的行号由 `@pierre/diffs` 在
    /// **异步创建的 Shadow DOM** 里画（code-viewer.tsx:12, :694 `disableLineNumbers`），
    /// 整条「码查看器链路」（行定位 focusedRange、gutter 点选/拖拽选区、行内评论、
    /// 字体像素与主题 token）都不在这一批。这里用一根普通 flex 行号列复现**可观察结果**：
    /// 与代码同 `text-ui-base leading-[1.5]`、右对齐、不随文本换行错位。
    /// 因此它只在**不换行**时成立——换行态（wrap）下行数与可视行数不再一一对应，
    /// 真源由库重算，这里直接让行号列在不换行态才出现（见 `gutter_lines`）。
    #[prop(default = false)]
    show_line_numbers: bool,
    /// 真源 `markedLines`（:61-62）：编译反馈点名的行。
    ///
    /// 着色的只有**行号本身**（`codeViewerMarkedLinesCss`：`[data-column-number="n"]{color:
    /// var(--color-warning)}`），代码行不加背景——被诊断指到不等于那一行被选中。
    #[prop(default = Vec::new())]
    marked_lines: Vec<i64>,
) -> impl IntoView {
    let is_wrapped = RwSignal::new(wrap_long_lines);
    let is_copied = RwSignal::new(false);
    let language_class = format!("language-{language}");
    let code_for_copy = code.clone();
    // 行号列的着色判定与行数都先算好（view! 的 children 先于属性求值，
    // 且 code_for_copy 已被上面复制按钮的闭包 move 走）。
    let marked_for_class: std::collections::HashSet<i64> =
        normalize_marked_lines(&marked_lines).into_iter().collect();
    let gutter_line_count = code_line_count(&code);
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
                <div class="flex min-w-0">
                    // 行号列：与代码同一套字号与行高，才能一行对一行。
                    // 换行态下行数与可视行数不再一一对应，此时整列隐藏（见 props 上的偏离注明）。
                    {show_line_numbers.then(|| {
                        (1..=gutter_line_count as i64)
                            .map(|line| {
                                let marked = marked_for_class.contains(&(line as i64));
                                view! {
                                    <div
                                        data-column-number=line.to_string()
                                        class=format!(
                                            "{} text-right",
                                            if marked { "text-warning" } else { "text-foreground-subtlest" },
                                        )
                                    >
                                        {line.to_string()}
                                    </div>
                                }
                            })
                            .collect_view()
                    })
                        .map(|gutter| {
                            view! {
                                <div
                                    aria-hidden="true"
                                    class=move || format!(
                                        "select-none shrink-0 pr-2 font-mono text-ui-base leading-[1.5] {}",
                                        if is_wrapped.get() { "hidden" } else { "block" },
                                    )
                                >
                                    {gutter}
                                </div>
                            }
                        })}
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
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marked_lines_drop_invalid_and_duplicate_entries() {
        // 真源 codeViewerMarkedLinesCss :154 —— `Number.isInteger(line) && line > 0`，
        // 外加 `[...new Set(...)]` 去重。顺序保留首次出现序（拼选择器用）。
        assert_eq!(normalize_marked_lines(&[3, 1, 3, 2]), vec![3, 1, 2]);
        assert_eq!(normalize_marked_lines(&[0, -4, 5]), vec![5], "非正整数丢弃");
        assert_eq!(normalize_marked_lines(&[]), Vec::<i64>::new());
        // 空集合 = 「没有要标的行」，行号列仍照常渲染，只是全用弱色。
        assert!(normalize_marked_lines(&[0, -1]).is_empty());
    }

    #[test]
    fn line_count_matches_js_split_semantics() {
        // JS `code.split("\n").length`：尾随换行会多出一空行，这决定行号列画几格。
        assert_eq!(code_line_count(""), 1);
        assert_eq!(code_line_count("one line"), 1);
        assert_eq!(code_line_count("a\nb"), 2);
        assert_eq!(code_line_count("a\nb\n"), 3, "尾部换行算一行");
        assert_eq!(code_line_count("a\r\nb"), 2, "\\r 留在行内，只按 \\n 切");
    }
}
