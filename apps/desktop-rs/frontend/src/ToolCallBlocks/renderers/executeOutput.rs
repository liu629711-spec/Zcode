//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/ExecuteOutput.tsx`
//! （50 行）。
//!
//! 终端输出视窗：**流式时吸底**，用户上滚即冻结（保留阅读位置），回到
//! 底部解冻。
//!
//! 真源注释（:36-37）：原预览与结果的高度上限不同且不吸底；共用五行上限，
//! 短内容自适应，结束时保留阅读状态。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

/// `ExecuteOutput` 的 props（真源 :4）。
#[component]
pub fn ExecuteOutputComponent(
    /// 输出正文（流式中的 fullText / 失败正文 / 结果文本）。
    text: String,
    /// 是否仍在流式输出。
    running: bool,
) -> impl IntoView {
    let scroll = NodeRef::<leptos::html::Div>::new();
    // 真源 :8-10 —— `frozen` 非 null 时不再跟随；`hasStreamed` 记流式是否开始过。
    let frozen: RwSignal<Option<String>> = RwSignal::new(None);
    let has_streamed = RwSignal::new(false);
    // 滚动 API（web-sys Element / leptos html::Div）都是 i32。
    let previous_top = RwSignal::new(0i32);

    // on_scroll 也要读原始文本（冻结时兜底）——先克隆再让 display 闭包捕获。
    let text_for_scroll = text.clone();
    let following = move || frozen.get().is_none();
    let display = move || frozen.get().unwrap_or_else(|| text.clone());

    // 真源 :11-19 —— 内容/跟随态变化时吸底，并记录浏览器实际滚动位置
    // （避免尾窗变短时把程序滚动误判为上滚）。
    Effect::new(move |_| {
        if running {
            has_streamed.set(true);
        }
        if !has_streamed.get_untracked() || !following() {
            return;
        }
        if let Some(el) = scroll.get() {
            el.set_scroll_top(el.scroll_height());
            previous_top.set(el.scroll_top());
        }
    });

    let on_scroll = move |ev: web_sys::Event| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(el) = target.dyn_into::<HtmlElement>() else {
            return;
        };
        let scroll_top = el.scroll_top();
        let at_bottom = el.scroll_height() - scroll_top - el.client_height() <= 8;
        if has_streamed.get_untracked()
            && following()
            && scroll_top < previous_top.get_untracked()
            && !at_bottom
        {
            // 用户上滚 → 冻结在当前内容（就地算，不捕获 display 闭包——
            // 它已被 view! 的 pre 用掉）。
            frozen.set(Some(
                frozen
                    .get_untracked()
                    .unwrap_or_else(|| text_for_scroll.clone()),
            ));
        } else if !following() && at_bottom {
            // 回到底部 → 恢复跟随。
            frozen.set(None);
        }
        previous_top.set(scroll_top);
    };

    view! {
        <div
            node_ref=scroll
            class="min-w-0 max-w-full max-h-[5lh] flex-none overflow-auto leading-5"
            data-testid="bash-output-scroll"
            data-following=following()
            tabindex="0"
            on:scroll=move |ev| on_scroll(ev)
        >
            <pre
                class="whitespace-pre-wrap break-words font-mono text-ui-base leading-5 text-foreground-subtle"
                data-testid=if running { "bash-output-preview-full" } else { "bash-result-output" }
            >
                {display()}
            </pre>
        </div>
    }
}

#[cfg(test)]
mod tests {
    /// 冻结/解冻判定下沉为纯函数，便于单测（组件本身需要 DOM 与 executor）。
    /// 真源 :20-31 的 onScroll 分支。
    pub(crate) fn resolve_following(
        was_following: bool,
        has_streamed: bool,
        scroll_top: i32,
        previous_top: i32,
        scroll_height: i32,
        client_height: i32,
    ) -> bool {
        let at_bottom = scroll_height - scroll_top - client_height <= 8;
        if has_streamed && was_following && scroll_top < previous_top && !at_bottom {
            return false; // 上滚 → 冻结
        }
        if !was_following && at_bottom {
            return true; // 回到底 → 解冻
        }
        was_following
    }

    #[test]
    fn scroll_freezes_on_user_scroll_up_and_resumes_at_bottom() {
        // 流式中上滚（未到底）→ 冻结。
        assert!(!resolve_following(true, true, 100, 200, 1000, 300));
        // 冻结状态下回到底部 → 恢复跟随。
        assert!(resolve_following(false, true, 700, 100, 1000, 300));
        // 未曾流式输出（快照直达）→ 始终跟随（真源 :14 的 hasStreamed 门）。
        assert!(resolve_following(true, false, 100, 200, 1000, 300));
    }
}
