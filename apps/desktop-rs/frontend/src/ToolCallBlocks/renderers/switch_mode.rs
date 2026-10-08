//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/switch-mode.tsx`（160 行）。
//!
//! 计划卡：provider 给 markdown 正文时出「计划」卡（标题 + 文件 chip +
//! 复制按钮 + 截断渐变正文 + 查看完整按钮）；无正文时回退最小 ToolOutput。
//! 真源注释（:141-143）：switch_mode 的有效信息就是那段 markdown，
//! 不该再套通用工具卡片。
//!
//! ## 裁剪注明
//!
//! - `onOpenPlanDetail`（打开计划详情面板）：Rust 侧计划面板未迁，
//!   `open_detail` 无操作——卡片仍完整呈现（复制/渐变/按钮都在），
//!   仅「查看完整计划」按钮暂无跳转目标；
//! - `ToolSnapshotFieldNotice`：未迁不渲染；
//! - MessageResponse 的 theme/codePreview/链接回调：未迁，静态
//!   pulldown-cmark 渲染。

use leptos::prelude::*;
use serde_json::Value;

use super::planToolCall::{extract_plan_tool_call_content, get_plan_file_label};
use super::toolOutput::ToolOutputBlock;

/// 真源 :11 —— NotepadTextIcon 类名。
pub const SWITCH_MODE_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// 写剪贴板（真源 :66-72）：成功置「已复制」，失败保持未复制态。
fn write_clipboard(text: &str, copied: RwSignal<bool>) {
    let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) else {
        return;
    };
    let promise = clipboard.write_text(text);
    leptos::task::spawn_local(async move {
        // JsFuture 等 promise settle；成功才置位（真源 rejected → null）。
        if wasm_bindgen_futures::JsFuture::from(promise).await.is_ok() {
            copied.set(true);
        }
    });
}

/// 计划卡模型（真源 :25-40 的取值）。
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchModeModel {
    pub markdown: Option<String>,
    pub plan_file_path: Option<String>,
}

/// 组装模型。
pub fn build_switch_mode_model(
    input: &Value,
    input_text: &str,
    output: &Value,
    raw: &Value,
    workspace_path: &str,
) -> SwitchModeModel {
    let content = extract_plan_tool_call_content(input, input_text, output, raw, workspace_path);
    let has_markdown = content.markdown.as_deref().is_some_and(|m| !m.is_empty());
    SwitchModeModel {
        markdown: has_markdown.then_some(content.markdown).flatten(),
        plan_file_path: content.plan_file_path,
    }
}

/// `SwitchModeToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct SwitchModeBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub input_text: String,
    pub output: Value,
    pub raw: Value,
    pub error_text: Option<String>,
    pub workspace_path: String,
    pub show_icon: bool,
}

/// `SwitchModeToolCallBlock`（真源 :24-160）。
#[component]
pub fn SwitchModeToolCallBlock(props: SwitchModeBlockProps) -> impl IntoView {
    let model = build_switch_mode_model(
        &props.input,
        &props.input_text,
        &props.output,
        &props.raw,
        &props.workspace_path,
    );

    // 复制按钮（真源 :66-72）：真实写剪贴板，成功才置「已复制」——
    // 失败保持未复制态（真源 then 的 rejected 分支 setCopiedMarkdown(null)）。
    let copied = RwSignal::new(false);

    let has_markdown = model.markdown.is_some();
    if has_markdown {
        let markdown = model.markdown.clone().unwrap_or_default();
        let html = crate::app::render_markdown(&markdown);
        let file_label = get_plan_file_label(model.plan_file_path.as_deref());
        let file_path_title = model.plan_file_path.clone();
        let markdown_for_copy = markdown.clone();

        return view! {
            <section class="group w-full min-w-0 overflow-hidden rounded-xl border border-card-border bg-card text-foreground shadow-xs outline-none transition-colors hover:border-border-hover focus-visible:border-input-border-focused focus-visible:ring-2 focus-visible:ring-ring/40">
                <header class="flex h-10 min-w-0 items-start gap-2 px-4 pt-4">
                    <div class="flex flex-none items-center gap-2">
                        <span class=SWITCH_MODE_TOOL_ICON_CLASS>
                            // NotepadTextIcon（lucide）：便签 + 行。
                            <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M8 2v4"></path>
                                <path d="M12 2v4"></path>
                                <path d="M16 2v4"></path>
                                <rect width="16" height="18" x="4" y="4" rx="2"></rect>
                                <path d="M8 10h6"></path>
                                <path d="M8 14h8"></path>
                                <path d="M8 18h5"></path>
                            </svg>
                        </span>
                        <h3 class="text-ui-base font-medium text-foreground-subtle">"计划"</h3>
                    </div>
                    {file_label.map(|label| view! {
                        <code
                            class="min-w-0 truncate text-ui-sm text-foreground-subtlest"
                            title=file_path_title.clone().unwrap_or_default()
                        >
                            {label}
                        </code>
                    })}
                    <div class="ml-auto flex flex-none items-center gap-1">
                        <button
                            type="button"
                            class="flex size-7 items-center justify-center rounded-md text-foreground-subtle hover:bg-surface-hover"
                            aria-label=move || if copied.get() { "计划已复制" } else { "复制计划" }
                            on:click={
                                let markdown = markdown_for_copy.clone();
                                move |ev| {
                                    ev.stop_propagation();
                                    write_clipboard(&markdown, copied);
                                }
                            }
                        >
                            {move || copied.get().then(|| view! {
                                // CheckIcon。
                                <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M20 6 9 17l-5-5"></path>
                                </svg>
                            })}
                            {move || (!copied.get()).then(|| view! {
                                // CopyIcon。
                                <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <rect width="14" height="14" x="8" y="8" rx="2" ry="2"></rect>
                                    <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"></path>
                                </svg>
                            })}
                        </button>
                    </div>
                </header>
                <div class="relative overflow-hidden">
                    // 真源 :108-111 —— max-height 与 mask 必须在同一裁切节点
                    // （分层后长正文的渐变会按完整内容高度计算，可见区无渐隐）。
                    <div class="max-h-64 overflow-hidden px-4 pt-2 pb-12 [mask-image:linear-gradient(to_bottom,black_0%,black_30%,transparent_100%)]">
                        <div
                            class="min-w-0 text-ui-base break-words text-foreground [&_h1]:text-foreground [&_h2]:text-foreground [&_h3]:text-foreground [&_li]:text-foreground-subtle [&_p]:text-foreground-subtle"
                            inner_html=html
                        ></div>
                    </div>
                    <button
                        type="button"
                        class="absolute bottom-6 left-1/2 flex h-10 -translate-x-1/2 items-center gap-2 rounded-full bg-primary pl-6 pr-4.5 text-ui-base font-medium text-primary-foreground shadow-xs"
                        on:click=move |ev| ev.stop_propagation()
                    >
                        "查看完整计划"
                        // ArrowRightIcon。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M5 12h14"></path>
                            <path d="m12 5 7 7-7 7"></path>
                        </svg>
                    </button>
                </div>
            </section>
        }
        .into_any();
    }

    // 回退（真源 :146-151）：无 markdown 或失败态 → 最小输出块。
    if props.output != Value::Null || props.error_text.is_some() {
        let output = if props.error_text.is_some() {
            None
        } else {
            Some(props.output.clone())
        };
        return view! {
            <ToolOutputBlock output=output error_text=props.error_text.clone() />
        }
        .into_any();
    }
    ().into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn markdown_from_input_plan_field() {
        let model = build_switch_mode_model(
            &json!({"plan": "# 步骤\n- 一"}),
            "",
            &json!({}),
            &json!({}),
            "/ws",
        );
        assert!(model.markdown.is_some());
    }

    #[test]
    fn no_markdown_falls_back() {
        let model = build_switch_mode_model(&json!({}), "", &json!({}), &json!({}), "/ws");
        assert_eq!(model.markdown, None);
    }
}
